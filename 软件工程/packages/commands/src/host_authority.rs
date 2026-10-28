use super::*;

impl ProjectHost {
    pub(super) fn require_registered_ui(&self, authorizer: &ObjectId) -> Result<(), ApiError> {
        if !self
            .sessions
            .get(authorizer)
            .is_some_and(|caller| matches!(caller.origin, CallerOrigin::InteractiveUi))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Authority changes require the actual registered UI session",
                None,
            ));
        }
        Ok(())
    }

    /// Trusted native administration only. The request wire never carries this
    /// context, and a replacement cannot supply its own creation ledger.
    pub fn update_external_session_authority(
        &mut self,
        authorizer_ui: &ObjectId,
        mut next: HostCallContext,
    ) -> Result<(), ApiError> {
        self.require_registered_ui(authorizer_ui)?;
        let current = self
            .sessions
            .get(&next.session_id)
            .filter(|caller| matches!(caller.origin, CallerOrigin::ExternalAgent))
            .ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Only an existing external session can be updated",
                    None,
                )
            })?;
        if !matches!(next.origin, CallerOrigin::ExternalAgent) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "An external session cannot become a UI session",
                None,
            ));
        }
        let mut grant_ids = BTreeSet::new();
        for grant in &mut next.edit_grants {
            if !grant_ids.insert(grant.id.clone()) {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Duplicate authority grant ID",
                    None,
                ));
            }
            let ledger = current
                .edit_grants
                .iter()
                .find(|old| old.id == grant.id)
                .map(|old| old.created_objects.clone())
                .unwrap_or_default();
            if !grant.created_objects.is_empty() && !exact_data(&grant.created_objects, &ledger) {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Creation ledgers are Host-owned and cannot be supplied or changed",
                    None,
                ));
            }
            grant.created_objects = ledger;
        }
        if next.selected_file_tokens.iter().any(|token| {
            self.selections.get(token).is_none_or(|selection| {
                selection.session != next.session_id
                    || selection.project != self.project.as_ref().map(|project| project.id.clone())
            })
        }) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "File authority must name this session's actual current Host selection",
                None,
            ));
        }
        self.sessions.insert(next.session_id.clone(), next);
        self.refresh_render_controls();
        Ok(())
    }

    /// A disconnected peer loses future dispatch rights. Already-admitted work
    /// retains its original owner, input and FIFO position and is not cancelled.
    pub fn revoke_external_session(&mut self, session_id: &ObjectId) -> Result<(), ApiError> {
        if !self
            .sessions
            .get(session_id)
            .is_some_and(|caller| matches!(caller.origin, CallerOrigin::ExternalAgent))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Only a registered external session can be revoked",
                None,
            ));
        }
        self.sessions.remove(session_id);
        self.revoked_external_sessions.insert(session_id.clone());
        self.selections
            .retain(|_, selection| &selection.session != session_id);
        self.external_task_access
            .retain(|(session, _), _| session != session_id);
        self.refresh_render_controls();
        Ok(())
    }

    fn external_task_scope_authorized(&self, caller: &HostCallContext, task: &HostIoTask) -> bool {
        match &task.state.task.scope {
            TaskScope::Workspace(_) => caller.granted_project.as_ref().is_none_or(|id| {
                self.project
                    .as_ref()
                    .is_some_and(|project| &project.id == id)
            }),
            TaskScope::Edit(EditScope::Project(id)) => {
                caller.granted_project.as_ref() == Some(id)
                    && self
                        .project
                        .as_ref()
                        .is_some_and(|project| &project.id == id)
            }
            TaskScope::Edit(EditScope::DefinitionDraft(identity)) => {
                self.project
                    .as_ref()
                    .is_some_and(|project| caller.granted_project.as_ref() == Some(&project.id))
                    && caller.allowed_definition_drafts.contains(&identity.id)
                    && self.definition_drafts.contains_key(&identity.id)
            }
        }
    }

    /// Reconnect grants name one original TaskId. They never restore a previous
    /// session, file selection, task owner or caller-supplied context.
    pub fn grant_external_task_access(
        &mut self,
        authorizer_ui: &ObjectId,
        target_session: &ObjectId,
        task_id: &TaskId,
        allow_controls: bool,
    ) -> Result<(), ApiError> {
        self.require_registered_ui(authorizer_ui)?;
        let caller = self
            .sessions
            .get(target_session)
            .filter(|caller| matches!(caller.origin, CallerOrigin::ExternalAgent))
            .ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Task access requires a registered external target session",
                    None,
                )
            })?;
        let task = self.tasks.get(task_id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "The original TaskId does not exist",
                None,
            )
        })?;
        if task.epoch != self.epoch || !self.external_task_scope_authorized(caller, task) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Task is outside the target session's current project or definition scope",
                None,
            ));
        }
        self.external_task_access
            .insert((target_session.clone(), task_id.clone()), allow_controls);
        Ok(())
    }

    pub(super) fn has_external_task_access(
        &self,
        caller: &HostCallContext,
        task: &HostIoTask,
        controls: bool,
    ) -> bool {
        matches!(caller.origin, CallerOrigin::ExternalAgent)
            && task.epoch == self.epoch
            && self.external_task_scope_authorized(caller, task)
            && self
                .external_task_access
                .get(&(caller.session_id.clone(), task.state.task.id.clone()))
                .is_some_and(|allow| !controls || *allow)
    }
}
