    pub(super) fn render_scope_available(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
    ) -> bool {
        let Some(backend) = &self.render_state.backend else {
            return false;
        };
        self.admit(
            caller,
            &self.project.as_ref().map(|p| p.id.clone()),
            scope,
            &OperationName::RenderView,
        )
        .is_ok()
            && backend.supported_kinds().iter().any(|kind| {
                !matches!(scope, ApiScope::DefinitionDraft(_))
                    || !matches!(
                        kind,
                        RenderViewKind::LineCanvas
                            | RenderViewKind::MultiHallCanvas
                            | RenderViewKind::Comparison
                    )
            })
    }

    pub(super) fn render_view(
