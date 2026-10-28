use super::*;

impl ProjectHost {
    pub(super) fn compare_runs(
        &self,
        _: &HostCallContext,
        _: &ApiScope,
        params: CompareRunsParams,
    ) -> Result<RunComparison, ApiError> {
        let inputs = self.comparison_inputs(&params.run_ids)?;
        sf_reporting::compare_runs(
            &inputs,
            &params.metric_scope.keys,
            &params.metric_scope.objects,
            &params.metric_scope.acceptance,
        )
        .map_err(|issues| {
            let mut failure = engine_failure(issues);
            failure.message =
                "Frozen Run comparison rejected its selected inputs or metric scope".into();
            failure
        })
    }

    fn actual_asset_content(
        &self,
        asset: &AssetId,
    ) -> Result<(Value, BTreeMap<String, Vec<u8>>), ApiError> {
        let packages: Vec<_> = self
            .modules
            .values()
            .filter(|package| {
                package
                    .assets
                    .iter()
                    .any(|reference| &reference.id == asset)
            })
            .collect();
        let entries: Vec<_> = self
            .backing
            .iter()
            .flat_map(|archive| &archive.manifest.documents)
            .chain(self.additions.iter().map(|addition| &addition.entry))
            .filter(|entry| {
                matches!(entry.object_ref.kind, ObjectKind::Asset)
                    && entry.object_ref.id.as_str() == asset.as_str()
            })
            .collect();
        if packages.is_empty() && entries.is_empty() {
            return Err(error(
                ErrorCode::MissingReference,
                "Asset has no actual source in the current project",
                None,
            ));
        }
        let mut files = BTreeMap::new();
        let paths = packages
            .iter()
            .flat_map(|package| package.assets.iter())
            .filter(|reference| &reference.id == asset)
            .map(|reference| &reference.path)
            .chain(entries.iter().map(|entry| &entry.file_ref.path));
        for path in paths {
            let bytes = self
                .additions
                .iter()
                .find(|addition| &addition.entry.file_ref.path == path)
                .map(|addition| &addition.bytes)
                .or_else(|| self.backing.as_ref()?.files.get(path.as_str()))
                .ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Asset bytes are absent; an imported permission label cannot confirm them",
                        None,
                    )
                })?;
            files.insert(path.as_str().to_owned(), bytes.clone());
        }
        let rights: Vec<_> = self
            .backing
            .iter()
            .flat_map(|archive| &archive.manifest.assets)
            .filter(|right| &right.asset_ref == asset)
            .collect();
        Ok((
            serde_json::json!({"modules":packages,"entries":entries,"rights":rights}),
            files,
        ))
    }

    /// Trusted UI permission action, never inferred from an imported rights flag
    /// or exposed as a caller-supplied export request field.
    pub fn confirm_asset_redistribution(
        &mut self,
        session: &ObjectId,
        asset: &AssetId,
    ) -> Result<(), ApiError> {
        let caller = self.sessions.get(session).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Asset redistribution requires an actual UI permission action",
                None,
            ));
        }
        if self.project.is_none() {
            return Err(error(
                ErrorCode::NoActiveProject,
                "Asset confirmation requires the active source project",
                None,
            ));
        }
        let content = self.actual_asset_content(asset)?;
        self.asset_confirmations
            .insert(asset.as_str().to_owned(), content);
        Ok(())
    }

    fn confirmed_asset_ids(&self) -> BTreeSet<String> {
        self.asset_confirmations
            .iter()
            .flat_map(|(id, (metadata, files))| {
                let valid = (|| {
                    let asset = AssetId::try_from(id.clone()).ok()?;
                    let current = self.actual_asset_content(&asset).ok()?;
                    (current.0 == *metadata && current.1 == *files).then_some(())
                })()
                .is_some();
                if valid { vec![id.clone()] } else { vec![] }
            })
            .collect()
    }

    pub(super) fn export_package(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: ExportPackageParams,
    ) -> Result<TaskHandle, ApiError> {
        self.export_package_candidate(caller, scope, params, None)
    }

    pub(super) fn export_package_candidate(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: ExportPackageParams,
        declarations: Option<&[ManufacturingFileDeclaration]>,
    ) -> Result<TaskHandle, ApiError> {
        let mut draft = self.current_draft();
        let (task_scope, revision) = match scope {
            ApiScope::Project => (
                EditScope::Project(draft.project.id.clone()),
                draft.project.revision.clone(),
            ),
            ApiScope::DefinitionDraft(id) => {
                let definition = &self.definition_drafts[id];
                if definition.composition.is_some() {
                    self.composition_project_read(caller)?;
                }
                check_revision(&params.revision, &definition.revision, "/params/revision")?;
                if definition.dirty {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Commit the definition version before exporting; an uncommitted session draft is not a saved module",
                        Some("/params/revision"),
                    ));
                }
                if !matches!(
                    params.package_scope.purpose,
                    PackagePurpose::PrivateModule | PackagePurpose::CommunityModule
                ) || !params.package_scope.scenarios.is_empty()
                    || !params.package_scope.run_ids.is_empty()
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Definition scope exports only committed modules and professional inputs, without Project scenarios or Runs",
                        Some("/params/package_scope"),
                    ));
                }
                let reference = ModuleRef {
                    id: definition.definition.id.clone(),
                    version: definition.definition.version.clone(),
                };
                if !params
                    .package_scope
                    .module_refs
                    .iter()
                    .any(|selected| module_key(selected) == module_key(&reference))
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Definition export must explicitly select its committed exact module version",
                        Some("/params/package_scope/module_refs"),
                    ));
                }
                if self
                    .modules
                    .get(&module_key(&reference))
                    .is_none_or(|package| !exact_data(&package.definition, &definition.definition))
                {
                    return Err(error(
                        ErrorCode::MissingReference,
                        "The exact committed definition source is absent",
                        None,
                    ));
                }
                for artifact in &params.package_scope.artifact_ids {
                    if !definition.professional.artifacts.contains_key(artifact) {
                        return Err(error(
                            if self.professional.artifacts.contains_key(artifact) {
                                ErrorCode::PermissionDenied
                            } else {
                                ErrorCode::MissingReference
                            },
                            "Selected artifact is not in this definition scope",
                            Some("/params/package_scope/artifact_ids"),
                        ));
                    }
                }
                draft.professional = definition.professional.clone();
                draft.project.acknowledgments.clear();
                draft.project.bindings.clear();
                draft.project.artifact_refs =
                    draft.professional.artifacts.keys().cloned().collect();
                (
                    EditScope::DefinitionDraft(DefinitionScopeIdentity {
                        id: id.clone(),
                        revision: definition.revision.clone(),
                    }),
                    definition.revision.clone(),
                )
            }
            ApiScope::Workspace => unreachable!("export admission excludes Workspace"),
        };
        check_revision(&params.revision, &revision, "/params/revision")?;
        // Scope failures precede missing-object or file errors for a mixed selection.
        for run in params
            .package_scope
            .run_ids
            .iter()
            .filter_map(|id| self.runs.get(id))
        {
            if !exact_data(&run.scope, &task_scope) {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Selected export Run is outside the actual edit scope",
                    None,
                ));
            }
        }
        let (destination, kind) = self.selected_path(
            caller,
            &params.selected_destination_ref,
            &FilePurpose::ExportPackage,
        )?;
        let package = match kind {
            SelectedPathKind::Directory => file_ref("project.json", DocumentKind::Project),
            SelectedPathKind::File => {
                let name = destination
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        error(
                            ErrorCode::InvalidInput,
                            "Selected export file name cannot be represented in the package result",
                            None,
                        )
                    })?;
                FileRef {
                    path: RelativeFilePath::try_from(name.to_owned())
                        .map_err(|message| error(ErrorCode::InvalidInput, &message, None))?,
                    kind: DocumentKind::Project,
                }
            }
        };
        let mut confirmed_assets = self.confirmed_asset_ids();
        let mut backing = self.backing.clone();
        let mut additions = self.additions.clone();
        if let Some(declarations) = declarations {
            let candidate = self.manufacturing_export_candidate(
                caller,
                scope,
                &draft,
                &params.package_scope,
                declarations,
                confirmed_assets,
            )?;
            backing = Some(candidate.archive);
            additions.clear();
            confirmed_assets = candidate.confirmed_assets;
        }
        let execution_captures=if matches!(scope,ApiScope::Project)&&matches!(params.package_scope.purpose,PackagePurpose::PrivateProject|PackagePurpose::HandoffProject) {
            self.capture_execution_archives()?
        } else {vec![]};
        Ok(self.enqueue_io(
            caller,
            TaskScope::Edit(task_scope),
            OperationName::ExportPackage,
            revision,
            Some(params.selected_destination_ref.clone()),
            IoInput::Export {
                destination,
                kind,
                selection: params.selected_destination_ref,
                package,
                draft,
                backing,
                additions,
                package_scope: params.package_scope,
                confirmed_assets,
                execution_captures,
            },
        ))
    }

    pub(super) fn frozen_document<T: serde::de::DeserializeOwned>(
        &self,
        reference: &FileRef,
    ) -> Result<T, ApiError> {
        let bytes = self
            .additions
            .iter()
            .find(|addition| addition.entry.file_ref.path == reference.path)
            .map(|addition| &addition.bytes)
            .or_else(|| self.backing.as_ref()?.files.get(reference.path.as_str()))
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Actual frozen Run input file is absent",
                    Some(reference.path.as_str()),
                )
            })?;
        serde_json::from_slice::<Versioned<T>>(bytes)
            .map(|document| document.data)
            .map_err(|failure| {
                error(
                    ErrorCode::InvalidInput,
                    &format!("Invalid frozen Run input: {failure}"),
                    Some(reference.path.as_str()),
                )
            })
    }

    pub(super) fn comparison_inputs(
        &self,
        run_ids: &[RunId],
    ) -> Result<
        Vec<(
            ProjectState,
            ScenarioInput,
            Vec<ModuleDefinition>,
            RunResult,
        )>,
        ApiError,
    > {
        if run_ids.len() < 2 || run_ids.iter().collect::<BTreeSet<_>>().len() != run_ids.len() {
            return Err(error(
                ErrorCode::InvalidInput,
                "Comparison requires at least two distinct actual Run IDs",
                Some("/params/run_ids"),
            ));
        }
        // Check every resolved scope before reporting absent IDs or input errors.
        for run in run_ids.iter().filter_map(|id| self.runs.get(id)) {
            if !matches!(&run.scope, EditScope::Project(id) if self.project.as_ref().is_some_and(|project| &project.id == id && &run.result.input_ref.project_id == id))
            {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Compared Run belongs to a different edit scope",
                    None,
                ));
            }
        }
        run_ids.iter().map(|id| {
            let run = self.runs.get(id).ok_or_else(|| error(ErrorCode::MissingReference, "Compared Run is absent from the active project", Some("/params/run_ids")))?;
            if !matches!(run.result.evidence_origin, EvidenceOrigin::ActualSoftware) {
                return Err(error(ErrorCode::InvalidInput, "Comparison requires actual software Run results", Some("/params/run_ids")));
            }
            let project: ProjectState = self.frozen_document(&run.result.input_ref.frozen_project_file)?;
            let scenario: ScenarioInput = self.frozen_document(&run.result.input_ref.frozen_scenario_file)?;
            let input = &run.result.input_ref;
            if project.id != input.project_id || project.revision.0.value() != input.project_revision.0.value()
                || scenario.id != input.scenario_id || scenario.revision.0.value() != input.scenario_revision.0.value()
                || scenario.project_revision.0.value() != input.project_revision.0.value()
            {
                return Err(error(ErrorCode::InvalidInput, "Frozen documents do not match the selected Run input identity", Some("/params/run_ids")));
            }
            let mut keys = BTreeSet::new();
            let definitions = project.module_refs.iter()
                .chain(project.devices.iter().flat_map(|device| &device.route_operation_refs))
                .filter(|reference| keys.insert(module_key(reference)))
                .map(|reference| self.modules.get(&module_key(reference)).map(|package| package.definition.clone())
                    .ok_or_else(|| error(ErrorCode::MissingReference, "Exact frozen Run module version is absent; current/latest definitions cannot replace it", Some("/params/run_ids"))))
                .collect::<Result<Vec<_>, _>>()?;
            Ok((project, scenario, definitions, run.result.clone()))
        }).collect()
    }
}
