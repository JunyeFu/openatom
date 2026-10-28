//! Private UI byte projection over the same admitted Host and exact source.
use super::*;

fn media_missing(message: &str) -> ApiError {
    error(ErrorCode::MissingReference, message, Some("/asset"))
}
fn declared_asset(package: &ModulePackage, asset: &AssetRef) -> Result<(), ApiError> {
    if !package
        .assets
        .iter()
        .any(|declared| exact_data(declared, asset))
    {
        return Err(media_missing(
            "Exact AssetRef id/path/kind is not declared by this source",
        ));
    }
    Ok(())
}

impl ProjectHost {
    pub fn read_media_asset(
        &self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
        source: &ObjectSource,
        asset: &AssetRef,
    ) -> Result<Vec<u8>, ApiError> {
        let caller = self.sessions.get(session_id).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Media bytes require the actual UI session",
                None,
            ));
        }
        self.admit(
            caller,
            &Some(project_id.clone()),
            scope,
            &OperationName::GetObject,
        )?;
        let actual = match scope {
            ApiScope::Project => &self.project.as_ref().unwrap().revision,
            ApiScope::DefinitionDraft(id) => {
                self.admit(
                    caller,
                    &Some(project_id.clone()),
                    &ApiScope::Project,
                    &OperationName::GetObject,
                )?;
                &self.definition_drafts[id].revision
            }
            ApiScope::Workspace => {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Media reads require Project or DefinitionDraft scope",
                    Some("/scope"),
                ));
            }
        };
        check_revision(scope_revision, actual, "/scope_revision")?;
        match source {
            ObjectSource::ModuleVersion { module_ref } => {
                let (package, draft_self) = self.scoped_media_package(scope, module_ref)?;
                declared_asset(&package, asset)?;
                if !draft_self
                    && !package.files.iter().any(|file| {
                        file.path == asset.path && matches!(file.kind, DocumentKind::Asset)
                    })
                    && self.archive_media_file(&asset.path).is_none()
                {
                    return Err(media_missing(
                        "此精确来源未提供可读取的实际素材文件；请核对素材引用与包内文件。",
                    ));
                }
                if let ApiScope::DefinitionDraft(id) = scope {
                    if draft_self {
                        if let Some(composition) = &self.definition_drafts[id].composition {
                            if let Some(file) = composition.files.iter().find(|file| {
                                file.entry.file_ref.path == asset.path
                                    && matches!(file.entry.file_ref.kind, DocumentKind::Asset)
                            }) {
                                return Ok(file.bytes.clone());
                            }
                        }
                        let draft = &self.definition_drafts[id];
                        if package.files.iter().any(|file| {
                            file.path == asset.path && matches!(file.kind, DocumentKind::Asset)
                        }) {
                            if let Some(bytes) = draft.professional.files.get(asset.path.as_str()) {
                                return Ok(bytes.clone());
                            }
                        }
                        return self
                            .archive_media_file(&asset.path)
                            .map(|(_, bytes)| bytes)
                            .ok_or_else(|| {
                                media_missing("Actual draft-listed media bytes are absent")
                            });
                    }
                }
                self.media_file_bytes(module_ref, asset)
                    .ok_or_else(|| media_missing("Actual listed media bytes are absent"))
            }
            ObjectSource::RunModuleVersion {
                run_id, module_ref, ..
            } => {
                let object = self.object(
                    caller,
                    scope,
                    GetObjectParams {
                        object_id: ObjectRef {
                            kind: ObjectKind::Module,
                            id: ObjectId::try_from(module_ref.id.as_str().to_owned()).unwrap(),
                        },
                        source: source.clone(),
                        node_index: None,
                    },
                )?;
                let DomainObject::Module(package) = object else {
                    unreachable!()
                };
                declared_asset(&package, asset)?;
                if !package
                    .files
                    .iter()
                    .any(|file| file.path == asset.path && matches!(file.kind, DocumentKind::Asset))
                    && self.archive_media_file(&asset.path).is_none()
                {
                    return Err(media_missing("此冻结版本未提供可读取的实际素材文件。"));
                }
                if matches!(scope, ApiScope::DefinitionDraft(_)) {
                    return self.frozen_cycle_media_bytes(run_id,module_ref,asset).ok_or_else(||media_missing("Definition sample has no actual frozen media bytes; current draft files are not historical input"));
                }
                self.archive_media_file(&asset.path)
                    .map(|(_, bytes)| bytes)
                    .ok_or_else(|| media_missing("Actual frozen module media bytes are absent"))
            }
            _ => Err(error(
                ErrorCode::InvalidInput,
                "Media reads require an explicit exact ModuleVersion or RunModuleVersion",
                Some("/source"),
            )),
        }
    }

    /// Same exact declared source eligibility for public metadata and UI bytes.
    /// Both entry points admit their actual session/Scope before this lookup.
    pub(super) fn scoped_media_package(
        &self,
        scope: &ApiScope,
        reference: &ModuleRef,
    ) -> Result<(ModulePackage, bool), ApiError> {
        match scope {
            ApiScope::Project => Ok((self.readable_project_module(reference)?, false)),
            ApiScope::DefinitionDraft(id) => {
                let draft = &self.definition_drafts[id];
                if draft.definition.id == reference.id
                    && draft.definition.version == reference.version
                {
                    return Ok((self.draft_media_package(id)?, true));
                }
                let mut pending = host_definitions::definition_dependencies(&draft.definition);
                if let Some(composition) = &draft.composition {
                    pending.extend(composition.dependencies.clone());
                }
                let mut seen = BTreeSet::new();
                while let Some(dependency) = pending.pop() {
                    if !seen.insert(module_key(&dependency)) {
                        continue;
                    }
                    let package = self
                        .adoption_source(&dependency)
                        .ok_or_else(|| media_missing("Actual draft dependency is not loaded"))?;
                    if module_key(&dependency) == module_key(reference) {
                        return Ok((package.clone(), false));
                    }
                    pending.extend(package.dependencies.clone());
                }
                Err(media_missing(
                    "Exact module is outside this draft's explicit dependency closure",
                ))
            }
            ApiScope::Workspace => Err(error(
                ErrorCode::InvalidInput,
                "Exact module reads require Project or DefinitionDraft scope",
                Some("/params/source"),
            )),
        }
    }

    pub(super) fn archive_media_file(&self, path: &RelativeFilePath) -> Option<(FileRef, Vec<u8>)> {
        self.additions
            .iter()
            .find(|file| {
                &file.entry.file_ref.path == path
                    && matches!(file.entry.file_ref.kind, DocumentKind::Asset)
            })
            .map(|file| (file.entry.file_ref.clone(), file.bytes.clone()))
            .or_else(|| {
                let archive = self.backing.as_ref()?;
                let entry = archive.manifest.documents.iter().find(|entry| {
                    &entry.file_ref.path == path
                        && matches!(entry.file_ref.kind, DocumentKind::Asset)
                })?;
                Some((
                    entry.file_ref.clone(),
                    archive.files.get(path.as_str())?.clone(),
                ))
            })
    }
    pub(super) fn media_file_bytes(
        &self,
        reference: &ModuleRef,
        asset: &AssetRef,
    ) -> Option<Vec<u8>> {
        if self.modules.contains_key(&module_key(reference)) {
            return self.archive_media_file(&asset.path).map(|(_, bytes)| bytes);
        }
        {
            self.bundled_modules
                .get(&module_key(reference))?
                .additions
                .iter()
                .find(|file| file.entry.file_ref.path == asset.path)
                .filter(|file| matches!(file.entry.file_ref.kind, DocumentKind::Asset))
                .map(|file| file.bytes.clone())
        }
    }
    pub(super) fn draft_media_package(&self, id: &ModuleId) -> Result<ModulePackage, ApiError> {
        let draft = &self.definition_drafts[id];
        let mut package = host_definitions::definition_package(&draft.definition)?;
        if let Some(composition) = &draft.composition {
            for asset in &composition.assets {
                if !package
                    .assets
                    .iter()
                    .any(|current| exact_data(current, asset))
                {
                    package.assets.push(asset.clone());
                }
            }
            for file in &composition.files {
                if !package
                    .files
                    .iter()
                    .any(|current| exact_data(current, &file.entry.file_ref))
                {
                    package.files.push(file.entry.file_ref.clone());
                }
            }
        }
        Ok(package)
    }
}
