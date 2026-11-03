use super::host_definitions::validate_definition;
use super::*;
use std::path::Path;

/// Immutable bytes loaded by trusted native startup. They are not Project state.
pub(super) struct BundledModule {
    original: BundledMetadata,
    pub package: BundledMetadata,
    pub additions: Vec<BundledFile>,
}

// Validation constructs both exact versions. Afterwards the catalogue can recreate either
// from its own immutable bytes without keeping all unused templates and definitions resident.
pub(super) struct BundledMetadata {
    file: BundledFile,
    parsed: std::sync::OnceLock<ModulePackage>,
}
impl BundledMetadata {
    fn new(file: BundledFile, package: ModulePackage) -> Self {
        Self { file, parsed: std::sync::OnceLock::from(package) }
    }
}
impl std::ops::Deref for BundledMetadata {
    type Target = ModulePackage;
    fn deref(&self) -> &Self::Target {
        self.parsed.get_or_init(|| serde_json::from_slice::<Versioned<ModulePackage>>(&self.file.bytes())
            .expect("catalogue metadata encoded during registration").data)
    }
}

// The installed catalogue owns an immutable copy even if its source directory is moved.
// Keep large media packed while idle; opening a project need not expand the whole catalogue.
#[derive(Clone)]
pub(super) struct BundledFile {
    pub entry: DocumentEntry,
    data: Arc<Vec<u8>>,
    expanded_len: Option<usize>,
}
impl BundledFile {
    fn new(file: ArchiveAddition) -> Self {
        use std::io::Write;
        let mut data = file.bytes;
        let mut expanded_len = None;
        if !matches!(file.entry.file_ref.kind, DocumentKind::GenerationProfile) && data.len() >= 64 * 1024 {
            let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
            encoder.write_all(&data).expect("in-memory catalogue compression");
            let packed = encoder.finish().expect("complete in-memory catalogue stream");
            if packed.len() < data.len() {
                expanded_len = Some(data.len());
                data = packed;
            }
        }
        Self { entry: file.entry, data: Arc::new(data), expanded_len }
    }
    pub(super) fn bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        use std::io::Read;
        match self.expanded_len {
            None => std::borrow::Cow::Borrowed(self.data.as_slice()),
            Some(len) => {
                let mut bytes = Vec::with_capacity(len);
                flate2::read::DeflateDecoder::new(self.data.as_slice())
                    .read_to_end(&mut bytes).expect("catalogue stream produced in memory");
                std::borrow::Cow::Owned(bytes)
            }
        }
    }
    pub(super) fn profile_bytes(&self) -> Option<&[u8]> {
        matches!(self.entry.file_ref.kind, DocumentKind::GenerationProfile).then(|| self.data.as_slice())
    }
}
#[derive(Default)]
pub(super) struct LibraryImports {
    pub modules: BTreeMap<(String, String), ModulePackage>,
    pub additions: BTreeMap<String, ArchiveAddition>,
}

fn library_error(message: impl Into<String>) -> ApiError {
    error(
        ErrorCode::InvalidInput,
        &message.into(),
        Some("/module_ref"),
    )
}

fn package_prefix(reference: &ModuleRef) -> String {
    let hex = |value: &str| {
        value
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    format!(
        "bundled/{}/{}/",
        hex(reference.id.as_str()),
        hex(&String::from(reference.version.clone()))
    )
}

pub(super) fn relocate_template_subset(
    source: &ModulePackage,
    subset: &TemplateSubset,
) -> TemplateSubset {
    let reference = ModuleRef {
        id: source.definition.id.clone(),
        version: source.definition.version.clone(),
    };
    let prefix = package_prefix(&reference);
    let ModulePayload::Templates(template) = &source.definition.payload else {
        return subset.clone();
    };
    if !template.project.as_str().starts_with(&prefix) {
        return subset.clone();
    }
    let mut subset = subset.clone();
    if let Some(adoption) = &mut subset.adoption {
        let path = format!("{prefix}{}", adoption.source_archive.path.as_str());
        if source.files.iter().any(|file| file.path.as_str() == path) {
            adoption.source_archive.path = RelativeFilePath::try_from(path).unwrap();
        }
    }
    subset
}

fn read_library_file(root: &Path, relative: &RelativeFilePath) -> Result<Vec<u8>, ApiError> {
    let path = root
        .join(relative.as_str())
        .canonicalize()
        .map_err(|failure| {
            error(
                ErrorCode::MissingReference,
                &format!("Bundled file {}: {failure}", relative.as_str()),
                None,
            )
        })?;
    if !path.starts_with(root) {
        return Err(library_error(
            "Bundled file escapes the trusted candidate directory",
        ));
    }
    std::fs::read(path).map_err(|failure| error(ErrorCode::IoFailure, &failure.to_string(), None))
}

fn read_package(root: &Path) -> Result<ModulePackage, ApiError> {
    let bytes = read_library_file(
        root,
        &RelativeFilePath::try_from("module.json".to_owned()).unwrap(),
    )?;
    let document: Versioned<ModulePackage> = serde_json::from_slice(&bytes)
        .map_err(|failure| library_error(format!("Bundled ModulePackage: {failure}")))?;
    validate_definition(&document.data.definition)?;
    Ok(document.data)
}

fn relocate_package(original: &ModulePackage, root: &Path) -> Result<BundledModule, ApiError> {
    let reference = ModuleRef {
        id: original.definition.id.clone(),
        version: original.definition.version.clone(),
    };
    let prefix = package_prefix(&reference);
    let relocated_path = |path: &RelativeFilePath| {
        RelativeFilePath::try_from(format!("{prefix}{}", path.as_str())).unwrap()
    };
    let asset_ids: BTreeMap<_, _> = original
        .assets
        .iter()
        .map(|asset| {
            (
                asset.id.clone(),
                AssetId::try_from(format!("{prefix}{}", asset.id.as_str())).unwrap(),
            )
        })
        .collect();
    let mut package = original.clone();
    for file in &mut package.files {
        file.path = relocated_path(&file.path);
    }
    for asset in &mut package.assets {
        asset.path = relocated_path(&asset.path);
        asset.id = asset_ids[&asset.id].clone();
    }
    fn relocate_payload_assets(value: &mut Value, original: &[AssetRef], relocated: &[AssetRef]) {
        if let Ok(asset) = serde_json::from_value::<AssetRef>(value.clone()) {
            if let Some(index) = original
                .iter()
                .position(|declared| exact_data(declared, &asset))
            {
                *value = serde_json::to_value(&relocated[index]).unwrap();
            }
            return;
        }
        match value {
            Value::Object(fields) => {
                for child in fields.values_mut() {
                    relocate_payload_assets(child, original, relocated);
                }
            }
            Value::Array(values) => {
                for child in values {
                    relocate_payload_assets(child, original, relocated);
                }
            }
            _ => {}
        }
    }
    let mut payload = serde_json::to_value(&package.definition.payload).unwrap();
    relocate_payload_assets(&mut payload, &original.assets, &package.assets);
    package.definition.payload = serde_json::from_value(payload)
        .expect("AssetRef-only relocation preserves the typed payload");
    for right in package
        .rights
        .iter_mut()
        .chain(&mut package.definition.contribution.assets)
    {
        if let Some(id) = asset_ids.get(&right.asset_ref) {
            right.asset_ref = id.clone();
        }
    }
    if let ModulePayload::Templates(template) = &mut package.definition.payload {
        template.project = relocated_path(&template.project);
        if let Some(path) = &mut template.generation_profile {
            *path = relocated_path(path);
        }
    }
    let target = ObjectRef {
        kind: ObjectKind::Module,
        id: ObjectId::try_from(reference.id.as_str().to_owned()).unwrap(),
    };
    let mut additions = BTreeMap::new();
    for file in &original.files {
        let original_bytes = read_library_file(root, &file.path)?;
        let mut source_project = if matches!(file.kind, DocumentKind::Asset) {
            serde_json::from_slice::<Versioned<ProjectState>>(&original_bytes).ok()
        } else {
            None
        };
        let bytes = if matches!(file.kind, DocumentKind::Template) || source_project.is_some() {
            let relocated = if let Some(project) = &mut source_project {
                for adoption in &mut project.data.template_adoptions {
                    let path = relocated_path(&adoption.source_archive.path);
                    if package.files.iter().any(|file| file.path == path) {
                        adoption.source_archive.path = path;
                    }
                }
                encode_document(&project.data)
            } else {
                let subset: Versioned<TemplateSubset> = serde_json::from_slice(&original_bytes)
                    .map_err(|failure| {
                        library_error(format!("Bundled TemplateSubset: {failure}"))
                    })?;
                encode_document(&relocate_template_subset(&package, &subset.data))
            };
            let raw_path = RelativeFilePath::try_from(format!(
                "{prefix}original-files/{}",
                file.path.as_str()
            ))
            .unwrap();
            additions.insert(
                raw_path.as_str().to_owned(),
                ArchiveAddition {
                    entry: DocumentEntry {
                        role: DocumentRole::SupportingFile,
                        module_ref: None,
                        object_ref: target.clone(),
                        revision: None,
                        file_ref: FileRef {
                            path: raw_path,
                            kind: DocumentKind::Asset,
                        },
                        required: true,
                    },
                    bytes: original_bytes,
                },
            );
            relocated
        } else {
            original_bytes
        };
        let path = relocated_path(&file.path);
        let asset = original.assets.iter().find(|asset| asset.path == file.path);
        let (object_ref, revision) = if matches!(file.kind, DocumentKind::Scenario) {
            let scenario: Versioned<ScenarioInput> = serde_json::from_slice(&bytes)
                .map_err(|failure| library_error(format!("Bundled Scenario source: {failure}")))?;
            (ObjectRef { kind: ObjectKind::Scenario, id: ObjectId::try_from(scenario.data.id.as_str().to_owned()).unwrap() }, Some(scenario.data.revision))
        } else {
            (asset.map(|asset| ObjectRef {
                kind: ObjectKind::Asset,
                id: ObjectId::try_from(asset_ids[&asset.id].as_str().to_owned()).unwrap(),
            }).unwrap_or_else(|| target.clone()), None)
        };
        let entry = DocumentEntry {
            role: if asset.is_some() {
                DocumentRole::Asset
            } else {
                DocumentRole::SupportingFile
            },
            module_ref: None,
            object_ref,
            revision,
            file_ref: FileRef {
                path: path.clone(),
                kind: file.kind.clone(),
            },
            required: true,
        };
        if additions
            .insert(path.as_str().to_owned(), ArchiveAddition { entry, bytes })
            .is_some()
        {
            return Err(library_error(
                "Bundled package declares duplicate file paths",
            ));
        }
    }
    for asset in &original.assets {
        if !original
            .files
            .iter()
            .any(|file| file.path == asset.path && matches!(file.kind, DocumentKind::Asset))
        {
            return Err(library_error(
                "Bundled asset requires its actual declared Asset file",
            ));
        }
        if !original
            .rights
            .iter()
            .any(|right| right.asset_ref == asset.id)
            || !original.definition.contribution.assets.iter().any(|right| {
                original
                    .rights
                    .iter()
                    .any(|actual| actual.asset_ref == asset.id && exact_data(right, actual))
            })
        {
            return Err(library_error(
                "Bundled asset rights must match the actual contribution declaration",
            ));
        }
    }
    let path = RelativeFilePath::try_from(format!("{prefix}module.json")).unwrap();
    let original_path =
        RelativeFilePath::try_from(format!("{prefix}original-module.json")).unwrap();
    additions.insert(
        original_path.as_str().to_owned(),
        ArchiveAddition {
            entry: DocumentEntry {
                role: DocumentRole::SupportingFile,
                module_ref: None,
                object_ref: target.clone(),
                revision: None,
                file_ref: FileRef {
                    path: original_path,
                    kind: DocumentKind::Asset,
                },
                required: true,
            },
            bytes: read_library_file(
                root,
                &RelativeFilePath::try_from("module.json".to_owned()).unwrap(),
            )?,
        },
    );
    additions.insert(
        path.as_str().to_owned(),
        ArchiveAddition {
            entry: DocumentEntry {
                role: DocumentRole::ModuleDefinition,
                module_ref: Some(reference),
                object_ref: target,
                revision: None,
                file_ref: FileRef {
                    path,
                    kind: DocumentKind::Module,
                },
                required: true,
            },
            bytes: encode_document(&package),
        },
    );
    let additions: BTreeMap<_, _> = additions.into_iter().map(|(p, a)| (p, BundledFile::new(a))).collect();
    Ok(BundledModule {
        original: BundledMetadata::new(additions[&format!("{prefix}original-module.json")].clone(), original.clone()),
        package: BundledMetadata::new(additions[&format!("{prefix}module.json")].clone(), package),
        additions: additions.into_values().collect(),
    })
}

impl ProjectHost {
    /// Read only an exact current-Project selection, an actual package declared
    /// by its loaded archive/additions, or a trusted registered library source.
    pub(super) fn readable_project_module(
        &self,
        reference: &ModuleRef,
    ) -> Result<ModulePackage, ApiError> {
        let key = module_key(reference);
        let selected = self.project.as_ref().is_some_and(|project| {
            project
                .module_refs
                .iter()
                .any(|member| module_key(member) == key)
        });
        let declared = |entry: &DocumentEntry| {
            matches!(entry.file_ref.kind, DocumentKind::Module)
                && matches!(entry.object_ref.kind, ObjectKind::Module)
                && entry.object_ref.id.as_str() == reference.id.as_str()
                && entry
                    .module_ref
                    .as_ref()
                    .is_some_and(|member| module_key(member) == key)
        };
        let archived = self.additions.iter().any(|file| declared(&file.entry))
            || self.backing.as_ref().is_some_and(|archive| {
                archive.manifest.documents.iter().any(|entry| {
                    declared(entry) && archive.files.contains_key(entry.file_ref.path.as_str())
                })
            });
        if selected || archived {
            return self.modules.get(&key).cloned().ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "The declared exact module package is not loaded",
                    Some("/params/source/data/module_ref"),
                )
            });
        }
        let bundled=self.bundled_modules.get(&key).ok_or_else(|| {
            error(ErrorCode::MissingReference,"Exact module is not selected, declared by the current archive, or registered in the trusted library",Some("/params/source/data/module_ref"))
        })?;
        if self
            .modules
            .get(&key)
            .is_some_and(|current| !exact_data(current, &bundled.package))
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Current exact module conflicts with the registered library contents",
                Some("/params/source/data/module_ref"),
            ));
        }
        Ok(bundled.package.clone())
    }
    /// Trusted native startup registration; never accepts a wire path or edits a Project.
    /// The directory contains canonical module.json and its real declared files.
    pub fn register_bundled_template_directory(
        &mut self,
        directory: &Path,
    ) -> Result<(), ApiError> {
        self.register_bundled_directory(directory, true)
    }

    /// Registers one canonical category package and its declared exact dependencies.
    /// Startup registration is read-only with respect to the active Project.
    pub fn register_bundled_module_directory(&mut self, directory: &Path) -> Result<(), ApiError> {
        self.register_bundled_directory(directory, false)
    }

    fn register_bundled_directory(
        &mut self,
        directory: &Path,
        template_only: bool,
    ) -> Result<(), ApiError> {
        let root = directory
            .canonicalize()
            .map_err(|failure| error(ErrorCode::IoFailure, &failure.to_string(), None))?;
        let template = read_package(&root)?;
        if template_only && !matches!(template.definition.payload, ModulePayload::Templates(_)) {
            return Err(library_error(
                "Bundled candidate root must be a Templates ModulePackage",
            ));
        }
        let mut packages = BTreeMap::new();
        for file in &template.files {
            // Actual canonical dependencies are declared as supporting files in
            // the template, with each ModulePackage's own directory as its root.
            if !file.path.as_str().ends_with("/module.json") {
                continue;
            }
            let bytes = read_library_file(&root, &file.path)?;
            let document: Versioned<ModulePackage> = serde_json::from_slice(&bytes)
                .map_err(|failure| library_error(format!("Bundled dependency: {failure}")))?;
            validate_definition(&document.data.definition)?;
            let module_root = root
                .join(file.path.as_str())
                .parent()
                .unwrap()
                .canonicalize()
                .map_err(|failure| error(ErrorCode::IoFailure, &failure.to_string(), None))?;
            let package = relocate_package(&document.data, &module_root)?;
            let key = (
                package.package.definition.id.as_str().to_owned(),
                String::from(package.package.definition.version.clone()),
            );
            if packages.insert(key, package).is_some() {
                return Err(library_error("Duplicate bundled dependency ID/version"));
            }
        }
        let declared_definitions =
            RelativeFilePath::try_from("definitions.json".to_owned()).unwrap();
        if template
            .files
            .iter()
            .any(|file| file.path == declared_definitions)
        {
            let definitions: Vec<ModuleDefinition> =
                serde_json::from_slice(&read_library_file(&root, &declared_definitions)?).map_err(
                    |failure| library_error(format!("Bundled exact definitions: {failure}")),
                )?;
            let mut seen = BTreeSet::new();
            for definition in &definitions {
                let key = (
                    definition.id.as_str().to_owned(),
                    String::from(definition.version.clone()),
                );
                if !seen.insert(key.clone())
                    || !packages
                        .get(&key)
                        .is_some_and(|package| exact_data(&package.original.definition, definition))
                {
                    return Err(library_error(
                        "Bundled definition list differs from the actual exact dependency ModulePackages",
                    ));
                }
            }
            if seen.len() != packages.len() {
                return Err(library_error(
                    "Bundled dependency ModulePackage is absent from definitions.json",
                ));
            }
        }
        let package = relocate_package(&template, &root)?;
        let key = (
            template.definition.id.as_str().to_owned(),
            String::from(template.definition.version.clone()),
        );
        packages.insert(key, package);
        for package in packages.values() {
            for dependency in &package.package.dependencies {
                if !packages.contains_key(&module_key(dependency))
                    && !self.bundled_modules.contains_key(&module_key(dependency))
                {
                    return Err(error(
                        ErrorCode::MissingReference,
                        "Bundled exact dependency is absent",
                        Some("/dependencies"),
                    ));
                }
            }
            if let Some(existing) = self.bundled_modules.get(&(
                package.package.definition.id.as_str().to_owned(),
                String::from(package.package.definition.version.clone()),
            )) {
                if !exact_data(&*existing.original, &*package.original)
                    || existing.additions.len() != package.additions.len()
                    || existing
                        .additions
                        .iter()
                        .zip(&package.additions)
                        .any(|(a, b)| !exact_data(&a.entry, &b.entry) || a.expanded_len != b.expanded_len || a.data != b.data)
                {
                    return Err(library_error(
                        "Registered exact bundled version is immutable",
                    ));
                }
            }
        }
        for package in packages.values_mut() {
            package.original.parsed.take();
            package.package.parsed.take();
        }
        self.bundled_modules.extend(packages);
        Ok(())
    }

    pub(super) fn adoption_source(&self, reference: &ModuleRef) -> Option<&ModulePackage> {
        self.modules.get(&module_key(reference)).or_else(|| {
            self.bundled_modules
                .get(&module_key(reference))
                .map(|entry| &*entry.package)
        })
    }

    pub(super) fn bundled_source_path(
        &self,
        reference: &ModuleRef,
        path: &RelativeFilePath,
    ) -> RelativeFilePath {
        if let Some(source) = self.adoption_source(reference) {
            let relocated = format!("{}{}", package_prefix(reference), path.as_str());
            if source
                .files
                .iter()
                .any(|file| file.path.as_str() == relocated)
            {
                return RelativeFilePath::try_from(relocated).unwrap();
            }
        }
        path.clone()
    }

    pub(super) fn project_file_bytes(&self, path: &str) -> Option<&[u8]> {
        self.additions.iter().rev().find(|a| a.entry.file_ref.path.as_str() == path)
            .map(|a| a.bytes.as_slice())
            .or_else(|| self.backing.as_ref()?.files.get(path).map(Vec::as_slice))
    }

    pub(super) fn module_source_files(&self, reference: &ModuleRef) -> BTreeMap<String, Vec<u8>> {
        let mut files = self
            .backing
            .as_ref()
            .map(|archive| archive.files.as_ref().clone())
            .unwrap_or_default();
        for addition in &self.additions {
            files.insert(
                addition.entry.file_ref.path.as_str().to_owned(),
                addition.bytes.clone(),
            );
        }
        // Current archived bytes always win over the independent library.
        if let Some(package) = self
            .bundled_modules
            .get(&module_key(reference))
            .filter(|_| !self.modules.contains_key(&module_key(reference)))
        {
            for addition in &package.additions {
                files
                    .entry(addition.entry.file_ref.path.as_str().to_owned())
                    .or_insert_with(|| addition.bytes().into_owned());
            }
        }
        files
    }

    pub(super) fn stage_library_import(
        &self,
        reference: &ModuleRef,
        candidate: &mut ProjectDraft,
        imports: &mut LibraryImports,
    ) -> Result<(), ApiError> {
        let Some(bundled) = self.bundled_modules.get(&module_key(reference)) else {
            return Ok(());
        };
        let key = module_key(reference);
        if let Some(existing) = self.modules.get(&key) {
            if !exact_data(existing, &bundled.package) {
                return Err(library_error(
                    "Current Project exact version conflicts with bundled source",
                ));
            }
        }
        if imports.modules.contains_key(&key) {
            return Ok(());
        }
        imports.modules.insert(key, bundled.package.clone());
        if !candidate
            .project
            .module_refs
            .iter()
            .any(|actual| module_key(actual) == module_key(reference))
        {
            candidate.project.module_refs.push(reference.clone());
        }
        for dependency in &bundled.package.dependencies {
            self.stage_library_import(dependency, candidate, imports)?;
        }
        for addition in &bundled.additions {
            let path = addition.entry.file_ref.path.as_str();
            let existing = self
                .additions
                .iter()
                .chain(imports.additions.values())
                .find(|actual| {
                    actual
                        .entry
                        .file_ref
                        .path
                        .as_str()
                        .eq_ignore_ascii_case(path)
                });
            if let Some(existing) = existing {
                if !exact_data(&existing.entry, &addition.entry) || existing.bytes.as_slice() != addition.bytes().as_ref()
                {
                    return Err(library_error(
                        "Bundled file conflicts with immutable Project content",
                    ));
                }
                continue;
            }
            if let Some(archive) = &self.backing {
                if let Some(entry) = archive
                    .manifest
                    .documents
                    .iter()
                    .find(|entry| entry.file_ref.path.as_str().eq_ignore_ascii_case(path))
                {
                    if !exact_data(entry, &addition.entry)
                        || archive.files.get(path).map(Vec::as_slice) != Some(addition.bytes().as_ref())
                    {
                        return Err(library_error(
                            "Bundled file conflicts with archived Project content",
                        ));
                    }
                    continue;
                }
            }
            imports.additions.insert(path.to_owned(), ArchiveAddition { entry: addition.entry.clone(), bytes: addition.bytes().into_owned() });
        }
        Ok(())
    }
}


impl ProjectHost {
    /// Shared trusted startup consumer. An explicit delivery profile replaces the legacy library only for that delivery.
    /// Absence keeps the original startup; malformed or missing required profile resources fail without fallback.
    pub fn register_bundled_library(&mut self, root: &Path) -> Result<(), String> {
        let profile = root.join("startup-library.json");
        if !profile.exists() { return self.register_legacy_bundled_library(root); }
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        let profile: serde_json::Value = serde_json::from_slice(&std::fs::read(profile).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if profile["schema_version"] != "1.0" || profile["profile"] != "paper-packaging-runtime" {
            return Err("Unsupported bundled startup profile".into());
        }
        let entries = profile["register_order"].as_array().ok_or("Startup profile lacks register_order")?;
        if entries.is_empty() { return Err("Startup profile is empty".into()); }
        let mut seen = BTreeSet::new();
        for entry in entries {
            let expected: ModuleRef = serde_json::from_value(entry["module_ref"].clone()).map_err(|e| e.to_string())?;
            let relative = RelativeFilePath::try_from(entry["directory"].as_str().ok_or("Startup entry lacks directory")?.to_owned())
                .map_err(|e| format!("Invalid startup directory: {e:?}"))?;
            let directory = root.join(relative.as_str()).canonicalize().map_err(|e| format!("Required startup {}: {e}", relative.as_str()))?;
            if !directory.starts_with(&root) { return Err("Startup directory escapes module root".into()); }
            let package: Versioned<ModulePackage> = serde_json::from_slice(&std::fs::read(directory.join("module.json")).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if package.data.definition.id != expected.id || package.data.definition.version != expected.version {
                return Err(format!("Required startup module differs from exact reference: {}", relative.as_str()));
            }
            if !seen.insert((expected.id.as_str().to_owned(), String::from(expected.version.clone()))) { return Err("Duplicate exact startup module".into()); }
            match entry["kind"].as_str() {
                Some("module") => self.register_bundled_module_directory(&directory),
                Some("template") => self.register_bundled_template_directory(&directory),
                _ => return Err("Unknown startup entry kind".into()),
            }.map_err(|e| format!("Required startup {}: {}", relative.as_str(), e.message))?;
        }
        Ok(())
    }

    fn register_legacy_bundled_library(&mut self, root: &Path) -> Result<(), String> {
        let host = self;
        for name in [
            "purchased-assembly",
            "machining-delivery",
            "parallel-electromechanical",
            "carton-factory",
        ] {
            host.register_bundled_template_directory(
                &root.join("templates").join(name).join("candidate"),
            )
            .map_err(|failure| format!("Bundled template {name}: {}", failure.message))?;
        }
        for category in [
            "items",
            "devices",
            "resources",
            "tasks",
            "operations",
            "transport",
            "professional",
            "media",
        ] {
            host.register_bundled_module_directory(
                &root.join("devices/template-support").join(category),
            )
            .map_err(|failure| format!("Bundled {category} sample: {}", failure.message))?;
        }
        for name in ["image-simple", "model-simple", "image", "gif", "model"] {
            host.register_bundled_module_directory(
                &root.join("media/p302-controlled/installable").join(name),
            )
            .map_err(|failure| format!("Bundled controlled media {name}: {}", failure.message))?;
        }
        for relative in [
            "devices/manual-assembly/presentation/p303-owned",
            "devices/manual-assembly/presentation/p303-owned/phase",
        ] {
            host.register_bundled_module_directory(&root.join(relative))
                .map_err(|failure| {
                    format!("Bundled image timeline {relative}: {}", failure.message)
                })?;
        }
        // Dependency order is fixed by the delivered standard packages. Fit
        // inspection packages are deliberately outside the production library.
        for name in [
            "assembly-simple",
            "rework-simple",
            "packing-simple",
            "robot-simple",
            "conveyor-simple",
            "inspection-simple",
            "sealer-simple",
            "pallet-simple",
            "control-box-simple",
            "carton-simple",
            "assembly",
            "rework",
            "packing",
            "robot",
            "conveyor",
            "inspection",
            "sealer",
            "pallet",
            "control-box",
            "carton",
            "assembly-image",
            "rework-image",
            "packing-image",
            "robot-image",
            "conveyor-image",
            "inspection-image",
            "sealer-image",
            "pallet-image",
        ] {
            host.register_bundled_module_directory(
                &root.join("media/p302-line-grade/installable").join(name),
            )
            .map_err(|failure| format!("Bundled line-grade media {name}: {}", failure.message))?;
        }
        // Keep older exact versions available; new media still require explicit adoption.
        for suffix in ["-simple", ""] {
            for family in ["assembly", "carton", "control-box", "conveyor", "fastener", "inspection", "packing", "pallet", "pcb", "rework", "robot", "sealer", "sealing-tape", "shell"] {
                let name = format!("{family}{suffix}");
                host.register_bundled_module_directory(&root.join("media/equipment-quality-v04/installable").join(&name))
                    .map_err(|failure| format!("Bundled equipment-quality0.4.0 {name}: {}", failure.message))?;
            }
        }
        for suffix in ["-simple", ""] {
            for family in ["assembly", "inspection", "packing", "rework"] {
                let name = format!("{family}{suffix}");
                host.register_bundled_module_directory(
                    &root.join("media/equipment-quality-v041/installable").join(&name),
                )
                .map_err(|failure| {
                    format!("Bundled equipment-quality0.4.1 {name}: {}", failure.message)
                })?;
            }
        }
        // Only individually accepted batches enter the ordinary Native library.
        let accepted = root.join("media/native-equipment-500/accepted-registration.json");
        if accepted.is_file() {
            let registry: Value = serde_json::from_slice(&std::fs::read(&accepted).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            for entry in registry["register_order"].as_array().ok_or("Accepted registry lacks register_order")? {
                let directory = entry["directory"].as_str().ok_or("Accepted registry lacks directory")?;
                host.register_bundled_module_directory(&root.join("media/native-equipment-500").join(directory))
                    .map_err(|failure| format!("Accepted Native500 {directory}: {}", failure.message))?;
            }
        }
        let factory_root = root.join("templates/paper-packaging-factory");
        let factory: Value = serde_json::from_slice(
            &std::fs::read(factory_root.join("registration.json")).map_err(|e| e.to_string())?,
        ).map_err(|e| e.to_string())?;
        for entry in factory["additional_register_order"].as_array().ok_or("Factory registry lacks additional_register_order")? {
            let directory = entry["directory"].as_str().ok_or("Factory registry lacks directory")?;
            host.register_bundled_module_directory(&root.join(directory))
                .map_err(|failure| format!("Paper factory dependency {directory}: {}", failure.message))?;
        }
        host.register_bundled_template_directory(&factory_root.join("candidate"))
            .map_err(|failure| format!("Paper factory template: {}", failure.message))?;
        Ok(())
    }
}
