use super::*;

impl ProjectHost {
    pub(super) fn frozen_run_summary(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        id: &RunId,
        at: &SimTimeMs,
    ) -> Result<RunSummary, ApiError> {
        if !caller
            .allowed_operations
            .iter()
            .any(|operation| matches!(operation, OperationName::GetRun))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Run input reads require current GetRun authority",
                None,
            ));
        }
        if !matches!(scope, ApiScope::Project) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Frozen Run inputs require Project scope",
                Some("/params/source"),
            ));
        }
        let RunView::Summary(summary) = self.get_run(
            caller,
            scope,
            GetRunParams {
                run_id: id.clone(),
                query: RunQuery::Summary,
            },
        )?
        else {
            unreachable!()
        };
        if self
            .project
            .as_ref()
            .is_none_or(|project| project.id != summary.input.project_id)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Frozen Run belongs to a different Project",
                None,
            ));
        }
        if at.0.value() < summary.window.start_ms.0.value()
            || summary
                .computed_until_ms
                .as_ref()
                .is_none_or(|frontier| at.0.value() > frontier.0.value())
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Requested time is outside the computed Run frontier",
                Some("/params/source/data/at_ms"),
            ));
        }
        Ok(summary)
    }

    pub(super) fn frozen_run_project(
        &self,
        summary: &RunSummary,
    ) -> Result<ProjectState, ApiError> {
        let project: ProjectState = self.frozen_document(&summary.input.frozen_project_file)?;
        if project.id != summary.input.project_id
            || project.revision.0.value() != summary.input.project_revision.0.value()
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Frozen Project does not match the selected Run input identity",
                Some("/params/source"),
            ));
        }
        Ok(project)
    }

    pub(super) fn frozen_run_scenario(
        &self,
        summary: &RunSummary,
    ) -> Result<ScenarioInput, ApiError> {
        let scenario: ScenarioInput = self.frozen_document(&summary.input.frozen_scenario_file)?;
        if scenario.id != summary.input.scenario_id
            || scenario.revision.0.value() != summary.input.scenario_revision.0.value()
            || scenario.project_revision.0.value() != summary.input.project_revision.0.value()
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Frozen Scenario does not match the selected Run input identity or revisions",
                Some("/params/source"),
            ));
        }
        Ok(scenario)
    }

    pub(super) fn frozen_run_module(
        &self,
        summary: &RunSummary,
        reference: &ModuleRef,
    ) -> Result<ModulePackage, ApiError> {
        let project = self.frozen_run_project(summary)?;
        if !project
            .module_refs
            .iter()
            .chain(
                project
                    .devices
                    .iter()
                    .flat_map(|device| &device.route_operation_refs),
            )
            .any(|member| module_key(member) == module_key(reference))
        {
            return Err(error(
                ErrorCode::MissingReference,
                "Exact module version is not a member of this Run's frozen Project",
                Some("/params/source/data/module_ref"),
            ));
        }
        let selected = |entry: &DocumentEntry| {
            matches!(entry.object_ref.kind, ObjectKind::Module)
                && matches!(entry.file_ref.kind, DocumentKind::Module)
                && entry.object_ref.id.as_str() == reference.id.as_str()
                && entry
                    .module_ref
                    .as_ref()
                    .is_some_and(|actual| module_key(actual) == module_key(reference))
        };
        // The same immutable exact-version package is retained by Save/Open.
        // Read its actual file bytes, never the current in-memory module registry.
        let bytes = self
            .additions
            .iter()
            .find(|addition| selected(&addition.entry))
            .map(|addition| &addition.bytes)
            .or_else(|| {
                self.backing
                    .as_ref()?
                    .manifest
                    .documents
                    .iter()
                    .filter(|entry| selected(entry))
                    .find_map(|entry| {
                        self.backing
                            .as_ref()?
                            .files
                            .get(entry.file_ref.path.as_str())
                    })
            })
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Actual historical module package bytes are absent",
                    Some("/params/source/data/module_ref"),
                )
            })?;
        let package: Versioned<ModulePackage> =
            serde_json::from_slice(bytes).map_err(|failure| {
                error(
                    ErrorCode::InvalidInput,
                    &format!("Invalid frozen module package: {failure}"),
                    Some("/params/source/data/module_ref"),
                )
            })?;
        if package.data.definition.id != reference.id
            || package.data.definition.version != reference.version
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Historical module package identity differs from the frozen exact reference",
                Some("/params/source/data/module_ref"),
            ));
        }
        Ok(package.data)
    }
}
