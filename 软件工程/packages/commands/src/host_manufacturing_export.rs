//! Explicit UI declarations for one immutable manufacturing export candidate.
//! No declaration or confirmation is written to the source Project or Host ledger.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManufacturingFileDeclaration {
    pub artifact_id: ArtifactId,
    pub file: FileRef,
    pub source: String,
    pub license: String,
}

pub(super) struct ManufacturingExportCandidate {
    pub archive: ProjectArchive,
    pub confirmed_assets: BTreeSet<String>,
}

impl ProjectHost {
    /// Main-window in-process entry. Native supplies the actual UI session.
    /// The public API remains export_package; this cannot grant an Agent rights.
    pub fn export_manufacturing_package(
        &mut self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
        params: ExportPackageParams,
        declarations: Vec<ManufacturingFileDeclaration>,
        confirm_redistribution: bool,
    ) -> Result<TaskHandle, ApiError> {
        let caller = self.sessions.get(session_id).cloned().ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Manufacturing redistribution declarations require the actual UI session",
                None,
            ));
        }
        // Resolve Scope/operations/revision before examining Artifact or file input.
        self.admit(
            &caller,
            &Some(project_id.clone()),
            scope,
            &OperationName::ExportPackage,
        )?;
        self.admit(
            &caller,
            &Some(project_id.clone()),
            scope,
            &OperationName::GetObject,
        )?;
        let revision = match scope {
            ApiScope::Project => &self.project.as_ref().unwrap().revision,
            ApiScope::DefinitionDraft(id) => &self.definition_drafts[id].revision,
            ApiScope::Workspace => {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Manufacturing exports require an edit scope",
                    Some("/scope"),
                ));
            }
        };
        check_revision(scope_revision, revision, "/scope_revision")?;
        check_revision(&params.revision, revision, "/params/revision")?;
        if !confirm_redistribution {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Explicit UI confirmation is required for this export",
                Some("/confirm_redistribution"),
            ));
        }
        if !params.package_scope.include_redistributable_assets {
            return Err(error(
                ErrorCode::InvalidInput,
                "Explicit manufacturing declarations require include_redistributable_assets=true",
                Some("/params/package_scope/include_redistributable_assets"),
            ));
        }
        if declarations.is_empty() {
            return Err(error(
                ErrorCode::InvalidInput,
                "Select actual manufacturing files and declare their source and license",
                Some("/declarations"),
            ));
        }
        self.export_package_candidate(&caller, scope, params, Some(&declarations))
    }

    /// Called only after the ordinary export helper's existing Scope/revision,
    /// exact committed definition, Run scope and selected-token checks.
    pub(super) fn manufacturing_export_candidate(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        draft: &ProjectDraft,
        selection: &PackageSelection,
        declarations: &[ManufacturingFileDeclaration],
        mut confirmed_assets: BTreeSet<String>,
    ) -> Result<ManufacturingExportCandidate, ApiError> {
        let project_id = &self.project.as_ref().unwrap().id;
        let revision = match scope {
            ApiScope::Project => &draft.project.revision,
            ApiScope::DefinitionDraft(id) => &self.definition_drafts[id].revision,
            ApiScope::Workspace => {
                unreachable!("private manufacturing admission excludes Workspace")
            }
        };
        // Validate the complete supplied selection before preparing any candidate.
        let mut selected = BTreeMap::<String, (&ManufacturingFileDeclaration, Vec<u8>)>::new();
        for (index, declaration) in declarations.iter().enumerate() {
            let pointer = format!("/declarations/{index}");
            if !selection.artifact_ids.contains(&declaration.artifact_id) {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "A declared manufacturing Artifact must be explicitly selected for this export",
                    Some(&format!("{pointer}/artifact_id")),
                ));
            }
            let (_, result, _) = self.read_stored_packaging_artifact(
                &caller.session_id,
                project_id,
                scope,
                revision,
                &declaration.artifact_id,
            )?;
            if !matches!(declaration.file.kind, DocumentKind::Asset)
                || !result
                    .manufacturing_files
                    .iter()
                    .any(|file| exact_data(&file.file, &declaration.file))
            {
                return Err(error(
                    ErrorCode::MissingReference,
                    "File is not an exact manufacturing FileRef of this scoped Artifact result",
                    Some(&format!("{pointer}/file")),
                ));
            }
            if declaration.source.trim().is_empty() || declaration.license.trim().is_empty() {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Manufacturing source and license must be explicitly supplied",
                    Some(&pointer),
                ));
            }
            let path = declaration.file.path.as_str();
            if selected.contains_key(path) {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "The same manufacturing file was declared more than once",
                    Some(&format!("{pointer}/file")),
                ));
            }
            // These are actual Scope-owned professional bytes, not a caller path,
            // arbitrary attachment, another Draft or an unlisted archive file.
            let bytes = draft
                .professional
                .files
                .get(path)
                .cloned()
                .or_else(|| {
                    if !matches!(scope, ApiScope::Project) {
                        return None;
                    }
                    self.additions
                        .iter()
                        .find(|file| exact_data(&file.entry.file_ref, &declaration.file))
                        .map(|file| file.bytes.clone())
                        .or_else(|| {
                            let archive = self.backing.as_ref()?;
                            archive
                                .manifest
                                .documents
                                .iter()
                                .find(|file| exact_data(&file.file_ref, &declaration.file))?;
                            archive.files.get(path).cloned()
                        })
                })
                .ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Actual manufacturing bytes are absent from the selected scope",
                        Some(&format!("{pointer}/file")),
                    )
                })?;
            selected.insert(path.to_owned(), (declaration, bytes));
        }
        let mut archive = crate::project_store::snapshot_project_archive(
            draft,
            self.backing.as_ref(),
            &self.additions,
        )?;
        for (path, (declaration, bytes)) in selected {
            let entry = archive
                .manifest
                .documents
                .iter_mut()
                .find(|entry| exact_data(&entry.file_ref, &declaration.file))
                .ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Frozen export has no exact manufacturing file entry",
                        Some("/declarations/file"),
                    )
                })?;
            if !matches!(entry.object_ref.kind, ObjectKind::Asset)
                || archive.files.get(&path) != Some(&bytes)
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Frozen manufacturing identity or bytes changed while preparing the export",
                    Some("/declarations/file"),
                ));
            }
            let asset_id =
                AssetId::try_from(entry.object_ref.id.as_str().to_owned()).map_err(|message| {
                    error(
                        ErrorCode::InvalidInput,
                        &message,
                        Some("/declarations/file"),
                    )
                })?;
            entry.role = DocumentRole::Asset;
            entry.required = true;
            archive
                .manifest
                .assets
                .retain(|right| right.asset_ref != asset_id);
            archive.manifest.assets.push(AssetRights {
                asset_ref: asset_id.clone(),
                source: Some(declaration.source.clone()),
                license: Some(declaration.license.clone()),
                redistribution_confirmed: true,
                missing_reason: None,
            });
            // The caller confirmed this exact candidate, not the source ledger.
            // Existing unrelated confirmations retain their ordinary filter gates.
            confirmed_assets.insert(asset_id.as_str().to_owned());
        }
        Ok(ManufacturingExportCandidate {
            archive,
            confirmed_assets,
        })
    }
}
