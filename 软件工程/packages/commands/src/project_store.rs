//! Directory packages. Only the root manifest replacement commits a generation.
use crate::{ApiError, ApiFieldError, ErrorCode, ProjectDraft};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sf_domain::*;
#[cfg(windows)]
use std::os::windows::{
    ffi::{OsStrExt, OsStringExt},
    fs::{MetadataExt, OpenOptionsExt},
    io::AsRawHandle,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};
#[cfg(windows)]
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
    FILE_SHARE_READ, FILE_SHARE_WRITE, GetFinalPathNameByHandleW, MoveFileW, ReplaceFileW,
};

#[derive(Clone)]
pub struct ProjectArchive {
    pub project: ProjectState,
    /// Editable ScenarioInput documents only; retained snapshots are not current.
    pub scenarios: BTreeMap<ScenarioId, ScenarioInput>,
    pub professional: crate::ProfessionalDocuments,
    pub manifest: PackageManifest,
    /// Exact included file bytes, keyed by package-relative paths; excludes project.json.
    pub files: BTreeMap<String, Vec<u8>>,
    pub(crate) execution_sources: crate::execution_archive::ArchivedExecutions,
}

/// Host-owned immutable files produced from an actual frozen execution or
/// committed module. They join the same root-manifest transaction as the draft.
#[derive(Clone)]
pub(crate) struct ArchiveAddition {
    pub entry: DocumentEntry,
    pub bytes: Vec<u8>,
}

impl ProjectArchive {
    pub(crate) fn modules(&self) -> Result<Vec<ModulePackage>, ApiError> {
        self.manifest
            .documents
            .iter()
            .filter(|entry| matches!(entry.file_ref.kind, DocumentKind::Module))
            .filter_map(|entry| {
                self.files
                    .get(entry.file_ref.path.as_str())
                    .map(|bytes| decode(bytes, entry.file_ref.path.as_str()))
            })
            .collect()
    }

    pub(crate) fn run_results(&self) -> Result<Vec<RunResult>, ApiError> {
        self.manifest
            .documents
            .iter()
            .filter(|entry| matches!(entry.file_ref.kind, DocumentKind::RunResult))
            .filter_map(|entry| {
                self.files
                    .get(entry.file_ref.path.as_str())
                    .map(|bytes| decode(bytes, entry.file_ref.path.as_str()))
            })
            .collect()
    }
}

pub(crate) fn encode_document<T: Serialize + Clone>(value: &T) -> Vec<u8> {
    encode(value)
}

fn professional_additions(
    draft: &ProjectDraft,
    backing: Option<&ProjectArchive>,
    generation: u64,
) -> Result<Vec<ArchiveAddition>, ApiError> {
    let mut additions = BTreeMap::<String, ArchiveAddition>::new();
    let documents = &draft.professional;
    let existing = |kind: &DocumentKind, id: &str, revision: Option<&Revision>| {
        backing
            .and_then(|archive| {
                archive.manifest.documents.iter().find(|entry| {
                    same(&entry.file_ref.kind, kind)
                        && entry.object_ref.id.as_str() == id
                        && revision.is_none_or(|revision| {
                            entry
                                .revision
                                .as_ref()
                                .is_some_and(|actual| same(actual, revision))
                        })
                })
            })
            .cloned()
    };
    let make = |role,
                kind: DocumentKind,
                object_kind,
                id: &str,
                revision: Option<Revision>,
                path: String| {
        existing(&kind, id, revision.as_ref()).unwrap_or(DocumentEntry {
            role,
            module_ref: None,
            object_ref: ObjectRef {
                kind: object_kind,
                id: ObjectId::try_from(id.to_owned()).unwrap(),
            },
            revision,
            file_ref: FileRef {
                path: RelativeFilePath::try_from(path).unwrap(),
                kind,
            },
            required: true,
        })
    };
    let mut insert = |entry: DocumentEntry, encoded: Vec<u8>| -> Result<(), ApiError> {
        let path = entry.file_ref.path.as_str().to_owned();
        let bytes = documents
            .files
            .get(&path)
            .or_else(|| backing.and_then(|archive| archive.files.get(&path)))
            .cloned()
            .unwrap_or(encoded);
        parse_document(&entry, &bytes)?;
        if let Some(prior) = additions.get(&path) {
            if prior.bytes != bytes || !same(&prior.entry, &entry) {
                return Err(invalid(
                    &path,
                    "conflicting professional document declarations",
                ));
            }
        } else {
            additions.insert(path, ArchiveAddition { entry, bytes });
        }
        Ok(())
    };
    for (index, job) in documents.jobs.values().enumerate() {
        let path = documents
            .artifacts
            .values()
            .find(|artifact| {
                documents.results.get(&artifact.id).is_some_and(|result| {
                    result.job_ref.id == job.id && same(&result.job_ref.revision, &job.revision)
                })
            })
            .map(|artifact| artifact.input_ref.path.as_str().to_owned())
            .unwrap_or_else(|| format!("data/g{generation:06}/job-{:04}.json", index + 1));
        let mut entry = make(
            DocumentRole::ProfessionalInput,
            DocumentKind::PackagingJob,
            ObjectKind::PackagingJob,
            job.id.as_str(),
            Some(job.revision.clone()),
            path.clone(),
        );
        // Artifact FileRefs choose the canonical input path for this exact Job.
        if documents
            .artifacts
            .values()
            .any(|artifact| artifact.input_ref.path.as_str() == path)
        {
            entry.file_ref.path = RelativeFilePath::try_from(path).unwrap();
        }
        insert(entry, encode(job))?;
    }
    for (index, artifact) in documents.artifacts.values().enumerate() {
        insert(
            make(
                DocumentRole::ArtifactMetadata,
                DocumentKind::ArtifactRecord,
                ObjectKind::Artifact,
                artifact.id.as_str(),
                None,
                format!("data/g{generation:06}/artifact-{:04}.json", index + 1),
            ),
            encode(artifact),
        )?;
        if let Some(result) = documents.results.get(&artifact.id) {
            let mut entry = make(
                DocumentRole::ProfessionalResult,
                DocumentKind::PackagingResult,
                ObjectKind::Artifact,
                artifact.id.as_str(),
                None,
                artifact.result_ref.path.as_str().to_owned(),
            );
            entry.file_ref = artifact.result_ref.clone();
            insert(entry, encode(result))?;
        }
    }
    let mut refs = vec![];
    for value in documents
        .jobs
        .values()
        .map(|value| serde_json::to_value(value).unwrap())
        .chain(
            documents
                .results
                .values()
                .map(|value| serde_json::to_value(value).unwrap()),
        )
        .chain(
            documents
                .artifacts
                .values()
                .map(|value| serde_json::to_value(value).unwrap()),
        )
    {
        file_references(&value, &mut refs);
    }
    for (index, (path, kind)) in refs.into_iter().enumerate() {
        if additions.contains_key(&path)
            || backing.is_some_and(|archive| archive.files.contains_key(&path))
        {
            continue;
        }
        let Some(bytes) = documents.files.get(&path) else {
            continue;
        };
        let kind = kind.unwrap_or(DocumentKind::Asset);
        if !matches!(kind, DocumentKind::Asset) {
            return Err(invalid(
                &path,
                "related typed professional file has no exact document identity",
            ));
        }
        let entry = DocumentEntry {
            role: DocumentRole::SupportingFile,
            module_ref: None,
            object_ref: ObjectRef {
                kind: ObjectKind::Asset,
                id: ObjectId::try_from(format!("professional-file:{generation}:{}", index + 1))
                    .unwrap(),
            },
            revision: None,
            file_ref: FileRef {
                path: RelativeFilePath::try_from(path.clone())
                    .map_err(|message| invalid(&path, message))?,
                kind,
            },
            required: true,
        };
        additions.insert(
            path,
            ArchiveAddition {
                entry,
                bytes: bytes.clone(),
            },
        );
    }
    Ok(additions.into_values().collect())
}

fn failure(code: ErrorCode, message: impl Into<String>, pointer: &str) -> ApiError {
    let message = message.into();
    ApiError {
        code,
        message: message.clone(),
        object_refs: vec![],
        field_errors: vec![ApiFieldError {
            pointer: pointer.into(),
            reason: message,
        }],
        expected_revision: None,
        actual_revision: None,
    }
}

fn io_failure(path: &str, error: std::io::Error) -> ApiError {
    let code = if error.kind() == std::io::ErrorKind::NotFound {
        ErrorCode::MissingReference
    } else {
        ErrorCode::IoFailure
    };
    failure(code, format!("{path}: {error}"), path)
}

fn invalid(path: &str, message: impl Into<String>) -> ApiError {
    failure(ErrorCode::InvalidInput, message, path)
}

fn missing(path: &str, message: impl Into<String>) -> ApiError {
    failure(ErrorCode::MissingReference, message, path)
}

fn same<T: Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).expect("typed serialization")
        == serde_json::to_value(b).expect("typed serialization")
}

fn folded(path: &str) -> String {
    path.to_lowercase().to_uppercase()
}

fn checked_path(path: &str) -> Result<(), ApiError> {
    if path.is_empty()
        || path.contains(['\\', ':', '\0', '<', '>', '"', '|', '?', '*'])
        || path.chars().any(|c| c < ' ')
    {
        return Err(invalid(
            path,
            "package-relative forward-slash Windows file path required",
        ));
    }
    for part in path.split('/') {
        let stem = folded(part.split('.').next().unwrap_or_default());
        let reserved = matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|p| {
            stem.strip_prefix(p).is_some_and(|n| {
                matches!(
                    n,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
        if part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' ']) || reserved
        {
            return Err(invalid(path, "unsafe or Windows-reserved path component"));
        }
    }
    Ok(())
}

fn check_paths<'a>(paths: impl IntoIterator<Item = &'a str>) -> Result<(), ApiError> {
    let mut names = BTreeSet::new();
    for path in paths {
        checked_path(path)?;
        let key = folded(path);
        if !names.insert(key.clone()) {
            return Err(invalid(path, "casefold path collision"));
        }
        for parent in key.match_indices('/').map(|(i, _)| &key[..i]) {
            if names.contains(parent) {
                return Err(invalid(path, "file/directory path collision"));
            }
        }
        if names.iter().any(|n| n.starts_with(&(key.clone() + "/"))) {
            return Err(invalid(path, "file/directory path collision"));
        }
    }
    Ok(())
}

/// Directory handles disallow deletion/renaming while descendant paths are used.
struct Directory {
    root: PathBuf,
    _handle: File,
}

impl Directory {
    fn open(path: &Path) -> Result<Self, ApiError> {
        let root = fs::canonicalize(path).map_err(|e| io_failure("selected directory", e))?;
        let handle = open_directory(&root).map_err(|e| io_failure("selected directory", e))?;
        Ok(Self {
            root,
            _handle: handle,
        })
    }

    fn parents(&self, relative: &str, create: bool) -> Result<Vec<File>, ApiError> {
        checked_path(relative)?;
        let mut path = self.root.clone();
        let mut held = vec![];
        let parts: Vec<_> = relative.split('/').collect();
        for part in &parts[..parts.len() - 1] {
            path.push(part);
            if create {
                match fs::create_dir(&path) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(io_failure(relative, e)),
                }
            }
            let handle = open_directory(&path).map_err(|e| io_failure(relative, e))?;
            self.check_handle(&handle, relative)?;
            held.push(handle);
        }
        Ok(held)
    }

    fn check_handle(&self, file: &File, relative: &str) -> Result<(), ApiError> {
        #[cfg(windows)]
        {
            if file
                .metadata()
                .map_err(|e| io_failure(relative, e))?
                .file_attributes()
                & FILE_ATTRIBUTE_REPARSE_POINT
                != 0
            {
                return Err(invalid(
                    relative,
                    "symbolic links/reparse points are not package files",
                ));
            }
            let actual = final_name(file).map_err(|e| io_failure(relative, e))?;
            let root = folded(&self.root.to_string_lossy());
            let actual = folded(&actual.to_string_lossy());
            if actual != root && !actual.starts_with(&(root + "\\")) {
                return Err(invalid(
                    relative,
                    "opened file escapes the selected actual directory",
                ));
            }
        }
        #[cfg(not(windows))]
        if file
            .metadata()
            .map_err(|e| io_failure(relative, e))?
            .file_type()
            .is_symlink()
        {
            return Err(invalid(relative, "symbolic links are not package files"));
        }
        Ok(())
    }

    fn read(&self, relative: &str) -> Result<Vec<u8>, ApiError> {
        let _parents = self.parents(relative, false)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        options
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .share_mode(FILE_SHARE_READ);
        let mut file = options
            .open(self.root.join(relative))
            .map_err(|e| io_failure(relative, e))?;
        self.check_handle(&file, relative)?;
        if !file
            .metadata()
            .map_err(|e| io_failure(relative, e))?
            .is_file()
        {
            return Err(invalid(relative, "regular package file required"));
        }
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)
            .map_err(|e| io_failure(relative, e))?;
        Ok(bytes)
    }

    fn write_new(&self, relative: &str, bytes: &[u8]) -> Result<(), ApiError> {
        let _parents = self.parents(relative, true)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(windows)]
        options
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
            .share_mode(FILE_SHARE_READ);
        let mut file = options
            .open(self.root.join(relative))
            .map_err(|e| io_failure(relative, e))?;
        self.check_handle(&file, relative)?;
        file.write_all(bytes)
            .and_then(|_| file.flush())
            .and_then(|_| file.sync_all())
            .map_err(|e| io_failure(relative, e))?;
        // file and all directory handles close before the caller may commit the root.
        Ok(())
    }
}

fn open_directory(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    options
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    #[cfg(windows)]
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "directory reparse point rejected",
        ));
    }
    if !metadata.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "directory required",
        ));
    }
    Ok(file)
}

#[cfg(windows)]
fn final_name(file: &File) -> std::io::Result<PathBuf> {
    let handle = file.as_raw_handle() as _;
    let mut buffer = vec![0u16; 512];
    loop {
        let count = unsafe {
            GetFinalPathNameByHandleW(handle, buffer.as_mut_ptr(), buffer.len() as u32, 0)
        };
        if count == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if (count as usize) < buffer.len() {
            return Ok(PathBuf::from(std::ffi::OsString::from_wide(
                &buffer[..count as usize],
            )));
        }
        buffer.resize(count as usize + 1, 0);
    }
}

fn decode<T: DeserializeOwned>(bytes: &[u8], path: &str) -> Result<T, ApiError> {
    // Valid documents need one typed pass, without materializing the whole Run as Value.
    // The existing error path below preserves schema/error classification.
    if let Ok(value) = serde_json::from_slice::<Versioned<T>>(bytes) {
        return Ok(value.data);
    }
    let envelope: Value = serde_json::from_slice(bytes)
        .map_err(|e| invalid(path, format!("invalid Versioned JSON: {e}")))?;
    let version = envelope
        .get("schema_version")
        .ok_or_else(|| invalid(path, "schema_version is required"))?;
    if version != "1.0" {
        return Err(failure(
            ErrorCode::UnsupportedSchema,
            format!("{path}: 不支持 schema_version={version}；当前支持 schema_version=\"1.0\"。"),
            path,
        ));
    }
    serde_json::from_value::<Versioned<T>>(envelope)
        .map(|v| v.data)
        .map_err(|e| invalid(path, format!("invalid typed document: {e}")))
}

fn encode<T: Serialize + Clone>(value: &T) -> Vec<u8> {
    serde_json::to_vec_pretty(&Versioned {
        schema_version: SchemaVersion::V1,
        data: value.clone(),
    })
    .expect("typed document serialization")
}

enum Document {
    Project(ProjectState),
    Scenario(ScenarioInput),
    Module(ModulePackage),
    Template(TemplateSubset),
    Profile(TemplateGenerationProfile),
    Job(PackageStructureJob),
    Result(PackagingResult),
    Artifact(ArtifactRecord),
    Run(RunResult),
    Evaluation(EvaluationReport),
    ExecutionArchive(crate::StoredExecutionArchive),
    Binary,
}

impl Document {
    fn reference_json(&self) -> Option<Value> {
        Some(
            match self {
                Self::Project(v) => serde_json::to_value(v),
                Self::Scenario(v) => serde_json::to_value(v),
                Self::Module(v) => serde_json::to_value(v),
                Self::Template(v) => serde_json::to_value(v),
                Self::Profile(v) => serde_json::to_value(v),
                Self::Job(v) => serde_json::to_value(v),
                Self::Result(v) => serde_json::to_value(v),
                Self::Artifact(v) => serde_json::to_value(v),
                Self::Run(v) => serde_json::to_value(run_references(v)),
                Self::Evaluation(v) => serde_json::to_value(v),
                // Frozen original paths/modules belong to this isolated source namespace.
                Self::ExecutionArchive(v) => serde_json::to_value(crate::execution_archive::file_refs(v)),
                Self::Binary => return None,
            }
            .expect("typed document serialization"),
        )
    }
}


// Run event payloads are closed typed data. Only these fields contain FileRef/ModuleRef;
// references are inspected without allocating JSON for the complete event history.
#[derive(Serialize)]
struct RunReferences<'a> {
    input_ref: &'a RunInputRef,
    module_refs: Vec<&'a ModuleRef>,
}

fn run_references(run: &RunResult) -> RunReferences<'_> {
    fn execution<'a>(value: &'a JobExecutionRef, refs: &mut Vec<&'a ModuleRef>) {
        match value {
            JobExecutionRef::DefinitionCycle { module_ref } => refs.push(module_ref),
            JobExecutionRef::ProjectDevice { .. } => {}
        }
    }
    let mut module_refs = Vec::new();
    for job in &run.jobs {
        module_refs.push(&job.operation_ref);
        execution(&job.execution_ref, &mut module_refs);
    }
    for wait in &run.waits {
        if let Some(value) = &wait.execution_ref { execution(value, &mut module_refs); }
    }
    for event in &run.events {
        match &event.payload {
            SimulationEventPayload::QualityChanged(decision) => module_refs.push(&decision.inspection_ref),
            SimulationEventPayload::WaitChanged(wait) => {
                if let Some(value) = &wait.execution_ref { execution(value, &mut module_refs); }
            }
            SimulationEventPayload::JobStarted(_) | SimulationEventPayload::PhaseCompleted(_)
            | SimulationEventPayload::ItemTransferred(_) | SimulationEventPayload::ArrivalAccepted(_)
            | SimulationEventPayload::ResourceReleased(_) | SimulationEventPayload::OrderAccepted(_)
            | SimulationEventPayload::TransportChanged(_) => {}
        }
    }
    RunReferences { input_ref: &run.input_ref, module_refs }
}

fn parse_document(entry: &DocumentEntry, bytes: &[u8]) -> Result<Document, ApiError> {
    let path = entry.file_ref.path.as_str();
    use DocumentKind as K;
    use DocumentRole as R;
    let valid_role = matches!(
        (&entry.role, &entry.file_ref.kind),
        (R::ActiveProject | R::FrozenRunProject, K::Project)
            | (R::ScenarioInput | R::FrozenRunScenario, K::Scenario)
            | (R::ModuleDefinition, K::Module)
            | (R::TemplateData, K::Template)
            | (R::GenerationProfile, K::GenerationProfile)
            | (R::ProfessionalInput, K::PackagingJob)
            | (R::ProfessionalResult, K::PackagingResult)
            | (R::ArtifactMetadata, K::ArtifactRecord)
            | (R::RunResult, K::RunResult)
            | (R::Evaluation, K::Evaluation)
            | (R::Asset, K::Asset)
            | (R::SupportingFile, _)
    );
    if !valid_role {
        return Err(invalid(
            path,
            "document role does not match its declared kind",
        ));
    }
    let document = match entry.file_ref.kind {
        K::Project => Document::Project(decode(bytes, path)?),
        K::Scenario => Document::Scenario(decode(bytes, path)?),
        K::Module => Document::Module(decode(bytes, path)?),
        K::Template => Document::Template(decode(bytes, path)?),
        K::GenerationProfile => Document::Profile(decode(bytes, path)?),
        K::PackagingJob => Document::Job(decode(bytes, path)?),
        K::PackagingResult => Document::Result(decode(bytes, path)?),
        K::ArtifactRecord => Document::Artifact(decode(bytes, path)?),
        K::RunResult => Document::Run(decode(bytes, path)?),
        K::Evaluation => Document::Evaluation(decode(bytes, path)?),
        K::ExecutionArchive => Document::ExecutionArchive(decode(bytes, path)?),
        K::Asset => Document::Binary,
        _ => {
            return Err(failure(
                ErrorCode::UnsupportedSchema,
                "no frozen directory document type for this kind",
                path,
            ));
        }
    };
    let identity = match &document {
        Document::Project(v) => Some((ObjectKind::Project, v.id.as_str(), Some(&v.revision))),
        Document::Scenario(v) => Some((ObjectKind::Scenario, v.id.as_str(), Some(&v.revision))),
        Document::Module(v) => Some((ObjectKind::Module, v.definition.id.as_str(), None)),
        Document::Job(v) => Some((ObjectKind::PackagingJob, v.id.as_str(), Some(&v.revision))),
        Document::Result(v) => Some((ObjectKind::Artifact, v.artifact_id.as_str(), None)),
        Document::Artifact(v) => Some((ObjectKind::Artifact, v.id.as_str(), None)),
        Document::Run(v) => Some((ObjectKind::Run, v.id.as_str(), None)),
        Document::Evaluation(v) => Some((ObjectKind::Report, v.id.as_str(), None)),
        Document::ExecutionArchive(v) => Some((ObjectKind::Project, v.frozen_inputs.identity.project_id.as_str(), None)),
        _ => None,
    };
    if let Some((kind, id, revision)) = identity {
        if !same(&entry.object_ref.kind, &kind)
            || entry.object_ref.id.as_str() != id
            || revision.is_some_and(|r| !entry.revision.as_ref().is_some_and(|e| same(e, r)))
        {
            return Err(invalid(
                path,
                "document identity/revision differs from its manifest entry",
            ));
        }
    }
    if let Document::Module(module) = &document {
        let expected = ModuleRef {
            id: module.definition.id.clone(),
            version: module.definition.version.clone(),
        };
        if !entry
            .module_ref
            .as_ref()
            .is_some_and(|m| same(m, &expected))
        {
            return Err(invalid(
                path,
                "module document must declare its exact ID and SemVer",
            ));
        }
    }
    Ok(document)
}

fn file_references(value: &Value, refs: &mut Vec<(String, Option<DocumentKind>)>) {
    match value {
        Value::Object(object) => {
            if let Some(path) = object.get("path").and_then(Value::as_str) {
                if object.contains_key("kind") {
                    let kind = serde_json::from_value::<FileRef>(value.clone())
                        .ok()
                        .map(|r| r.kind);
                    refs.push((path.into(), kind));
                }
            }
            for child in object.values() {
                file_references(child, refs);
            }
        }
        Value::Array(values) => {
            for child in values {
                file_references(child, refs);
            }
        }
        _ => {}
    }
}

fn module_key(reference: &ModuleRef) -> (String, String) {
    (
        reference.id.as_str().into(),
        String::from(reference.version.clone()),
    )
}

fn module_references(value: &Value, refs: &mut Vec<ModuleRef>) {
    match value {
        Value::Object(object) => {
            if object.len() == 2 && object.contains_key("id") && object.contains_key("version") {
                if let Ok(reference) = serde_json::from_value(value.clone()) {
                    refs.push(reference);
                }
            }
            for child in object.values() {
                module_references(child, refs);
            }
        }
        Value::Array(values) => {
            for child in values {
                module_references(child, refs);
            }
        }
        _ => {}
    }
}

pub fn read_project_archive(directory: &Path) -> Result<ProjectArchive, ApiError> {
    read_root(&Directory::open(directory)?, "project.json")
}

fn read_root(directory: &Directory, root_name: &str) -> Result<ProjectArchive, ApiError> {
    read_root_with_documents(directory, root_name).map(|(archive, _)| archive)
}

fn read_root_with_documents(
    directory: &Directory,
    root_name: &str,
) -> Result<(ProjectArchive, BTreeMap<String, Document>), ApiError> {
    let manifest: PackageManifest = decode(&directory.read(root_name)?, root_name)?;
    let mut files = BTreeMap::new();
    for entry in &manifest.documents {
        let path = entry.file_ref.path.as_str();
        match directory.read(path) {
            Ok(bytes) => {
                files.insert(path.into(), bytes);
            }
            Err(e) if matches!(e.code, ErrorCode::MissingReference) && !entry.required => {}
            Err(e) => return Err(e),
        }
    }
    validate_archive_with_documents(manifest, files)
}

fn validate_archive(
    manifest: PackageManifest,
    files: BTreeMap<String, Vec<u8>>,
) -> Result<ProjectArchive, ApiError> {
    validate_archive_with_documents(manifest, files).map(|(archive, _)| archive)
}

fn validate_archive_with_documents(
    manifest: PackageManifest,
    files: BTreeMap<String, Vec<u8>>,
) -> Result<(ProjectArchive, BTreeMap<String, Document>), ApiError> {
    validate_archive_reusing_documents(manifest, files, None)
}

fn validate_archive_reusing_documents(
    manifest: PackageManifest,
    files: BTreeMap<String, Vec<u8>>,
    mut reusable: Option<(&ProjectArchive, &mut BTreeMap<String, Document>)>,
) -> Result<(ProjectArchive, BTreeMap<String, Document>), ApiError> {
    check_paths(manifest.documents.iter().map(|e| e.file_ref.path.as_str()))?;
    check_paths(files.keys().map(String::as_str))?;
    if files.keys().any(|path| {
        !manifest
            .documents
            .iter()
            .any(|e| e.file_ref.path.as_str() == path)
    }) {
        return Err(invalid(
            "project.json",
            "included bytes require explicit manifest entries",
        ));
    }
    if manifest
        .documents
        .iter()
        .any(|e| folded(e.file_ref.path.as_str()) == "PROJECT.JSON")
    {
        return Err(invalid(
            "project.json",
            "root manifest cannot also be a document payload",
        ));
    }
    let active: Vec<_> = manifest
        .documents
        .iter()
        .filter(|e| matches!(e.role, DocumentRole::ActiveProject))
        .collect();
    if active.len() != 1 || !active[0].required {
        return Err(invalid(
            "project.json",
            "exactly one required active_project document is required",
        ));
    }
    let mut documents = BTreeMap::new();
    for entry in &manifest.documents {
        let path = entry.file_ref.path.as_str();
        match files.get(path) {
            Some(bytes) => {
                let reused = reusable.as_mut().and_then(|(archive, parsed)| {
                    let prior = archive.manifest.documents.iter()
                        .find(|prior| prior.file_ref.path.as_str() == path)?;
                    if same(prior, entry) && archive.files.get(path) == Some(bytes) {
                        parsed.remove(path)
                    } else {
                        None
                    }
                });
                let document = match reused {
                    Some(document) => document,
                    None => parse_document(entry, bytes)?,
                };
                documents.insert(path.to_owned(), document);
            }
            None if !entry.required => {}
            None => return Err(missing(path, "required document bytes are missing")),
        }
    }
    let Some(Document::Project(project)) = documents.get(active[0].file_ref.path.as_str()) else {
        return Err(invalid(
            "project.json",
            "active_project must contain a typed ProjectState",
        ));
    };
    if project.id != manifest.project_id || !same(&project.revision, &manifest.project_revision) {
        return Err(invalid(
            "project.json",
            "active project ID/revision differs from root manifest",
        ));
    }
    let mut scenarios = BTreeMap::new();
    for entry in &manifest.documents {
        if matches!(entry.role, DocumentRole::ScenarioInput) {
            if let Some(Document::Scenario(scenario)) = documents.get(entry.file_ref.path.as_str())
            {
                if !project.scenario_refs.contains(&scenario.id) {
                    return Err(invalid(
                        entry.file_ref.path.as_str(),
                        "current scenario is absent from the active project index",
                    ));
                }
                if !same(&scenario.project_revision, &project.revision) {
                    return Err(invalid(
                        entry.file_ref.path.as_str(),
                        "current scenario is not bound to the active project revision",
                    ));
                }
                if scenarios
                    .insert(scenario.id.clone(), scenario.clone())
                    .is_some()
                {
                    return Err(invalid(
                        "project.json",
                        "duplicate current ScenarioInput identity",
                    ));
                }
            }
        }
    }
    let mut indexed = BTreeSet::new();
    for id in &project.scenario_refs {
        if !indexed.insert(id) {
            return Err(invalid(
                "project.json",
                "duplicate current scenario index identity",
            ));
        }
        if !scenarios.contains_key(id)
            && !reported_missing(&manifest, project, &ObjectKind::Scenario, id.as_str())
        {
            return Err(missing(
                "project.json",
                "current ScenarioInput is missing; retained snapshots cannot satisfy the active index",
            ));
        }
    }
    validate_closure(&manifest, &documents, &files, project, &scenarios)?;
    let mut professional = crate::ProfessionalDocuments::default();
    let mut professional_paths = vec![];
    for (path, document) in &documents {
        match document {
            Document::Job(job) => {
                let key = (job.id.clone(), job.revision.0.value());
                if let Some(existing) = professional.jobs.get(&key) {
                    if !same(existing, job) {
                        return Err(invalid(
                            path,
                            "conflicting immutable PackagingJob identity across document paths",
                        ));
                    }
                } else {
                    professional.jobs.insert(key, job.clone());
                }
            }
            Document::Artifact(artifact) => {
                if let Some(existing) = professional.artifacts.get(&artifact.id) {
                    if !same(existing, artifact) {
                        return Err(invalid(
                            path,
                            "conflicting immutable ArtifactRecord identity across document paths",
                        ));
                    }
                } else {
                    professional
                        .artifacts
                        .insert(artifact.id.clone(), artifact.clone());
                }
            }
            Document::Result(result) => {
                if let Some(existing) = professional.results.get(&result.artifact_id) {
                    if !same(existing, result) {
                        return Err(invalid(
                            path,
                            "conflicting immutable PackagingResult identity across document paths",
                        ));
                    }
                } else {
                    professional
                        .results
                        .insert(result.artifact_id.clone(), result.clone());
                }
            }
            _ => continue,
        }
        if let Some(value) = document.reference_json() {
            file_references(&value, &mut professional_paths);
        }
    }
    for (path, _) in professional_paths {
        if let Some(bytes) = files.get(&path) {
            professional.files.insert(path, bytes.clone());
        }
    }
    let mut archive = ProjectArchive {
        project: project.clone(),
        scenarios,
        professional,
        manifest,
        files,
        execution_sources: BTreeMap::new(),
    };
    archive.execution_sources = crate::execution_archive::load_sources(&archive)?;
    Ok((archive, documents))
}

fn rights_index(rights: &[AssetRights], path: &str) -> Result<BTreeMap<String, Value>, ApiError> {
    let mut result = BTreeMap::new();
    for right in rights {
        if result
            .insert(
                right.asset_ref.as_str().into(),
                serde_json::to_value(right).expect("typed rights"),
            )
            .is_some()
        {
            return Err(invalid(path, "duplicate asset-rights identity"));
        }
    }
    Ok(result)
}

const MATERIAL_MISSING_TRANSPORT: &str = "business_inputs.material_missing_transport_module";

fn reported_missing(
    manifest: &PackageManifest,
    project: &ProjectState,
    kind: &ObjectKind,
    id: &str,
) -> bool {
    manifest.missing.iter().chain(&project.issues).any(|issue| {
        // This scoped Issue requires a complete ModuleRef and validated current
        // Scenario context; it cannot use the older ID-only direct-target rule.
        if issue.code == MATERIAL_MISSING_TRANSPORT {
            return false;
        }
        let direct = issue
            .object_ref
            .as_ref()
            .is_some_and(|r| same(&r.kind, kind) && r.id.as_str() == id)
            && !issue.message.is_empty();
        if direct {
            return true;
        }
        let Some(owner) = &issue.object_ref else {
            return false;
        };
        let Some(pointer) = issue.field_path.as_deref() else {
            return false;
        };
        if matches!(kind, ObjectKind::Scenario) {
            if issue.code != "business_inputs.missing_scenario"
                || !matches!(owner.kind, ObjectKind::Project)
                || owner.id.as_str() != project.id.as_str()
            {
                return false;
            }
            let parts: Vec<_> = pointer.split('/').collect();
            if parts.len() != 3 || parts[0] != "" || parts[1] != "scenario_refs" {
                return false;
            }
            return serde_json::to_value(project)
                .expect("typed Project")
                .pointer(pointer)
                .and_then(|value| serde_json::from_value::<ScenarioId>(value.clone()).ok())
                .is_some_and(|reference| reference.as_str() == id);
        }
        if !matches!(kind, ObjectKind::Module) {
            return false;
        }
        match (&owner.kind, issue.code.as_str(), pointer) {
            (ObjectKind::Route, "business_inputs.missing_module", _) => {
                let Some(route) = project.routes.iter().find(|route| route.id == owner.id) else {
                    return false;
                };
                let parts: Vec<_> = pointer.split('/').collect();
                if parts.len() != 4
                    || parts[0] != ""
                    || parts[1] != "steps"
                    || parts[3] != "operation_ref"
                {
                    return false;
                }
                serde_json::to_value(route)
                    .expect("typed Route")
                    .pointer(pointer)
                    .and_then(|value| serde_json::from_value::<ModuleRef>(value.clone()).ok())
                    .is_some_and(|reference| reference.id.as_str() == id)
            }
            (ObjectKind::Device, "business_inputs.missing_device_module", "/module_ref") => project
                .devices
                .iter()
                .find(|device| device.id == owner.id)
                .is_some_and(|device| device.module_ref.id.as_str() == id),
            _ => false,
        }
    })
}

fn reported_current_transport(
    manifest: &PackageManifest,
    project: &ProjectState,
    scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
    reference: &ModuleRef,
) -> bool {
    // Built by validate_archive from ScenarioInput roles after identity/index/
    // project-revision checks; FrozenRunScenario/SupportingFile are absent.
    manifest.missing.iter().chain(&project.issues).any(|issue| {
        if issue.code != MATERIAL_MISSING_TRANSPORT {
            return false;
        }
        let Some(owner) = issue
            .object_ref
            .as_ref()
            .filter(|r| matches!(r.kind, ObjectKind::Scenario))
        else {
            return false;
        };
        let Some(scenario) = scenarios
            .values()
            .find(|s| s.id.as_str() == owner.id.as_str())
        else {
            return false;
        };
        let Some(pointer) = issue.field_path.as_deref() else {
            return false;
        };
        let parts: Vec<_> = pointer.split('/').collect();
        if parts.len() != 4
            || parts[0] != ""
            || parts[1] != "supplies"
            || parts[3] != "transport_ref"
        {
            return false;
        }
        serde_json::to_value(scenario)
            .expect("typed current Scenario")
            .pointer(pointer)
            .and_then(|value| serde_json::from_value::<ModuleRef>(value.clone()).ok())
            .is_some_and(|actual| same(&actual, reference))
    })
}

fn validate_closure(
    manifest: &PackageManifest,
    documents: &BTreeMap<String, Document>,
    files: &BTreeMap<String, Vec<u8>>,
    project: &ProjectState,
    scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
) -> Result<(), ApiError> {
    let entries: BTreeMap<_, _> = manifest
        .documents
        .iter()
        .map(|e| (e.file_ref.path.as_str(), e))
        .collect();
    let mut modules = BTreeMap::new();
    let mut assets: BTreeMap<String, String> = BTreeMap::new();
    let mut canonical_rights = BTreeMap::new();
    for entry in &manifest.documents {
        if matches!(entry.role, DocumentRole::Asset) {
            if !matches!(entry.object_ref.kind, ObjectKind::Asset) {
                return Err(invalid(
                    entry.file_ref.path.as_str(),
                    "asset entry must have Asset identity",
                ));
            }
            if assets
                .insert(
                    entry.object_ref.id.as_str().into(),
                    entry.file_ref.path.as_str().into(),
                )
                .is_some()
            {
                return Err(invalid("project.json", "duplicate asset index identity"));
            }
        }
    }
    for (path, document) in documents {
        if let Document::Module(module) = document {
            let key = (
                module.definition.id.as_str().to_owned(),
                String::from(module.definition.version.clone()),
            );
            if modules.insert(key, module).is_some() {
                return Err(invalid(path, "duplicate module ID/version"));
            }
            let declared = rights_index(&module.definition.contribution.assets, path)?;
            let included: BTreeSet<_> = module.assets.iter().map(|a| a.id.as_str()).collect();
            let selected: BTreeMap<_, _> = declared
                .into_iter()
                .filter(|(id, _)| included.contains(id.as_str()))
                .collect();
            if selected.len() != included.len() || selected != rights_index(&module.rights, path)? {
                return Err(invalid(
                    path,
                    "module rights differ from canonical contribution declarations",
                ));
            }
            for (id, right) in selected {
                if canonical_rights.get(&id).is_some_and(|old| old != &right) {
                    return Err(invalid(path, "contradictory same-ID asset rights"));
                }
                canonical_rights.insert(id, right);
            }
            let mut local_assets = BTreeSet::new();
            for asset in &module.assets {
                if !local_assets.insert(asset.id.as_str()) {
                    return Err(invalid(path, "duplicate module asset ID"));
                }
                let target = asset.path.as_str();
                if assets.get(asset.id.as_str()).is_some_and(|p| p != target) {
                    return Err(invalid(path, "asset ID maps to conflicting included paths"));
                }
                assets.insert(asset.id.as_str().into(), target.into());
                if !module.rights.iter().any(|r| r.asset_ref == asset.id) {
                    return Err(missing(
                        path,
                        "included module asset lacks its declared rights record",
                    ));
                }
            }
        }
    }
    let declared_modules: BTreeSet<_> = manifest.module_refs.iter().map(module_key).collect();
    if declared_modules.len() != manifest.module_refs.len()
        || manifest.module_refs.iter().any(|reference| {
            !modules.contains_key(&module_key(reference))
                && !reported_missing(
                    manifest,
                    project,
                    &ObjectKind::Module,
                    reference.id.as_str(),
                )
                && !reported_current_transport(manifest, project, scenarios, reference)
        })
    {
        return Err(missing(
            "project.json",
            "manifest module index must contain unique exact included ID/versions",
        ));
    }
    if modules.keys().any(|key| !declared_modules.contains(key)) {
        return Err(invalid(
            "project.json",
            "included module missing from manifest module index",
        ));
    }
    let root_rights = rights_index(&manifest.assets, "project.json")?;
    for (id, right) in &canonical_rights {
        if root_rights.get(id) != Some(right) {
            return Err(invalid(
                "project.json",
                "selected asset rights disagree with canonical declarations",
            ));
        }
    }
    if root_rights.keys().any(|id| !assets.contains_key(id)) {
        return Err(missing(
            "project.json",
            "asset rights reference an absent included asset index",
        ));
    }
    for (path, document) in documents {
        if let Some(value) = document.reference_json() {
            let mut refs = vec![];
            file_references(&value, &mut refs);
            for (target, kind) in refs {
                checked_path(&target)?;
                if target == "project.json" && matches!(document, Document::Evaluation(_)) {
                    continue;
                }
                let entry = entries.get(target.as_str()).ok_or_else(|| {
                    missing(
                        path,
                        format!("referenced file {target} is absent from manifest"),
                    )
                })?;
                if !files.contains_key(&target)
                    && (entry.required
                        || !reported_missing(
                            manifest,
                            project,
                            &entry.object_ref.kind,
                            entry.object_ref.id.as_str(),
                        ))
                {
                    return Err(missing(
                        path,
                        format!("referenced file {target} has no included bytes"),
                    ));
                }
                if kind
                    .as_ref()
                    .is_some_and(|k| !same(k, &entry.file_ref.kind))
                {
                    return Err(invalid(
                        path,
                        format!("referenced file {target} kind differs from manifest"),
                    ));
                }
            }
            let mut refs = vec![];
            module_references(&value, &mut refs);
            for reference in refs {
                if !modules.contains_key(&module_key(&reference))
                    && !reported_missing(
                        manifest,
                        project,
                        &ObjectKind::Module,
                        reference.id.as_str(),
                    )
                    && !reported_current_transport(manifest, project, scenarios, &reference)
                {
                    return Err(missing(
                        path,
                        "exact referenced module ID/version is absent",
                    ));
                }
            }
        }
        match document {
            Document::Project(value) => validate_project_indices(value, documents, manifest, path)?,
            Document::Run(run) => {
                let input = &run.input_ref;
                let Some(Document::Project(frozen)) =
                    documents.get(input.frozen_project_file.path.as_str())
                else {
                    return Err(missing(path, "Run frozen project document is missing"));
                };
                let Some(Document::Scenario(scenario)) =
                    documents.get(input.frozen_scenario_file.path.as_str())
                else {
                    return Err(missing(path, "Run frozen scenario document is missing"));
                };
                if frozen.id != input.project_id
                    || !same(&frozen.revision, &input.project_revision)
                    || scenario.id != input.scenario_id
                    || !same(&scenario.revision, &input.scenario_revision)
                    || !same(&scenario.project_revision, &input.project_revision)
                {
                    return Err(invalid(
                        path,
                        "Run input identity/revisions differ from frozen documents",
                    ));
                }
            }
            Document::Artifact(artifact) => {
                if let (Some(Document::Job(job)), Some(Document::Result(result))) = (
                    documents.get(artifact.input_ref.path.as_str()),
                    documents.get(artifact.result_ref.path.as_str()),
                ) {
                    if result.artifact_id != artifact.id
                        || result.job_ref.id != job.id
                        || !same(&result.job_ref.revision, &job.revision)
                        || !same(&result.subject_ref, &artifact.subject_ref)
                        || !same(&job.subject_ref, &artifact.subject_ref)
                    {
                        return Err(invalid(
                            path,
                            "artifact input/result/subject identities do not match",
                        ));
                    }
                } else {
                    return Err(invalid(
                        path,
                        "artifact input/result kinds do not match the frozen professional pair",
                    ));
                }
            }
            Document::Evaluation(report) => {
                for run in &report.run_refs {
                    if !documents
                        .values()
                        .any(|d| matches!(d, Document::Run(v) if &v.id == run))
                    {
                        return Err(missing(path, "evaluation references an absent Run"));
                    }
                }
                for scenario in &report.scenario_refs {
                    if !documents.values().any(|d| matches!(d, Document::Scenario(v) if v.id == scenario.id && same(&v.revision, &scenario.revision))) {
                        return Err(missing(path, "evaluation references an absent scenario revision"));
                    }
                }
            }
            _ => {}
        }
    }
    // The active document is also checked above; keep its identity tied to the root.
    if project.id != manifest.project_id {
        return Err(invalid("project.json", "project identity mismatch"));
    }
    Ok(())
}

fn validate_project_indices(
    project: &ProjectState,
    documents: &BTreeMap<String, Document>,
    manifest: &PackageManifest,
    path: &str,
) -> Result<(), ApiError> {
    let json = serde_json::to_value(project).expect("typed project");
    for key in [
        "item_types",
        "locations",
        "halls",
        "lines",
        "devices",
        "resources",
        "calendars",
        "products",
        "boms",
        "routes",
        "acknowledgments",
        "bindings",
    ] {
        let mut ids = BTreeSet::new();
        for object in json[key].as_array().expect("typed project collection") {
            if !ids.insert(object["id"].as_str().expect("typed object identity")) {
                return Err(invalid(path, format!("duplicate identity in {key}")));
            }
        }
    }
    let mut connections = BTreeSet::new();
    for connection in &project.connections {
        let id = match connection {
            Connection::Material(value) => &value.id,
            Connection::Signal(value) => &value.id,
            Connection::Artifact(value) => &value.id,
        };
        if !connections.insert(id) {
            return Err(invalid(path, "duplicate identity in connections"));
        }
    }
    for id in &project.scenario_refs {
        if !documents
            .values()
            .any(|d| matches!(d, Document::Scenario(v) if &v.id == id))
            && !reported_missing(manifest, project, &ObjectKind::Scenario, id.as_str())
        {
            return Err(missing(
                path,
                "project references an absent scenario document",
            ));
        }
    }
    for id in &project.artifact_refs {
        if !documents
            .values()
            .any(|d| matches!(d, Document::Artifact(v) if &v.id == id))
            && !reported_missing(manifest, project, &ObjectKind::Artifact, id.as_str())
        {
            return Err(missing(path, "project references absent artifact metadata"));
        }
    }
    Ok(())
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<(), ApiError> {
    if cancelled.load(Ordering::Acquire) {
        Err(failure(
            ErrorCode::Cancelled,
            "save cancelled before root commit",
            "project.json",
        ))
    } else {
        Ok(())
    }
}

static INTERNAL_SEQUENCE: AtomicU64 = AtomicU64::new(1);

// Keep all non-current documents. Retired editable files enter the new root
// only when retained consumers (or the new draft) still refer to their bytes.
fn retained_paths(
    draft: &ProjectDraft,
    source: &ProjectArchive,
) -> Result<BTreeSet<String>, ApiError> {
    let mut parsed = BTreeMap::new();
    for entry in &source.manifest.documents {
        let path = entry.file_ref.path.as_str();
        if let Some(bytes) = source.files.get(path) {
            parsed.insert(path.to_owned(), parse_document(entry, bytes)?);
        }
    }
    retained_paths_from_documents(draft, source, &parsed)
}

fn retained_paths_from_documents(
    draft: &ProjectDraft,
    source: &ProjectArchive,
    parsed: &BTreeMap<String, Document>,
) -> Result<BTreeSet<String>, ApiError> {
    let mut retained = BTreeSet::new();
    for entry in &source.manifest.documents {
        let path = entry.file_ref.path.as_str();
        if !matches!(
            entry.role,
            DocumentRole::ActiveProject | DocumentRole::ScenarioInput
        ) {
            retained.insert(path.to_owned());
        }
    }
    let mut draft_files = vec![];
    file_references(
        &serde_json::to_value(&draft.project).expect("typed draft"),
        &mut draft_files,
    );
    for scenario in draft.scenarios.values() {
        file_references(
            &serde_json::to_value(scenario).expect("typed scenario"),
            &mut draft_files,
        );
    }
    loop {
        let mut files = draft_files.clone();
        let mut scenarios: Vec<(ScenarioId, Option<Revision>, Option<Revision>)> = vec![];
        for path in &retained {
            if let Some(document) = parsed.get(path) {
                if let Some(value) = document.reference_json() {
                    file_references(&value, &mut files);
                }
                match document {
                    Document::Project(project) => {
                        for id in &project.scenario_refs {
                            scenarios.push((id.clone(), None, Some(project.revision.clone())));
                        }
                    }
                    Document::Run(run) => scenarios.push((
                        run.input_ref.scenario_id.clone(),
                        Some(run.input_ref.scenario_revision.clone()),
                        Some(run.input_ref.project_revision.clone()),
                    )),
                    Document::Evaluation(report) => {
                        for reference in &report.scenario_refs {
                            scenarios.push((
                                reference.id.clone(),
                                Some(reference.revision.clone()),
                                None,
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
        let before = retained.len();
        for entry in &source.manifest.documents {
            let path = entry.file_ref.path.as_str();
            if matches!(
                entry.role,
                DocumentRole::ActiveProject | DocumentRole::ScenarioInput
            ) {
                let by_file = files.iter().any(|(target, _)| target == path);
                let by_scenario = parsed.get(path).is_some_and(|document| {
                    if let Document::Scenario(scenario) = document {
                        scenarios.iter().any(|(id, revision, project_revision)| {
                            id == &scenario.id
                                && revision
                                    .as_ref()
                                    .is_none_or(|r| same(r, &scenario.revision))
                                && project_revision
                                    .as_ref()
                                    .is_none_or(|r| same(r, &scenario.project_revision))
                        })
                    } else {
                        false
                    }
                });
                if by_file || by_scenario {
                    retained.insert(path.to_owned());
                }
            }
        }
        if before == retained.len() {
            return Ok(retained);
        }
    }
}

fn internal_name(directory: &Directory, purpose: &str) -> Result<String, ApiError> {
    loop {
        let sequence = INTERNAL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(".sf-{purpose}-{}-{sequence}.json", std::process::id());
        match fs::symlink_metadata(directory.root.join(&name)) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(name),
            Ok(_) => {}
            Err(e) => return Err(io_failure(&name, e)),
        }
    }
}

/// The Host's single per-target FIFO calls this with its frozen project revision.
pub fn write_project_archive(
    directory: &Path,
    draft: &ProjectDraft,
    backing: Option<&ProjectArchive>,
    cancelled: &AtomicBool,
) -> Result<ProjectArchive, ApiError> {
    write_project_archive_with_additions(directory, draft, backing, &[], cancelled)
}

/// Freeze an editable copy without changing the current backing or its disk root.
pub(crate) fn snapshot_project_archive(
    draft: &ProjectDraft,
    backing: Option<&ProjectArchive>,
    additions: &[ArchiveAddition],
) -> Result<ProjectArchive, ApiError> {
    if let Some(source) = backing {
        if source.project.id != draft.project.id {
            return Err(invalid(
                "project.json",
                "backing archive belongs to another project",
            ));
        }
        validate_archive(source.manifest.clone(), source.files.clone())?;
    }
    let mut generation = backing.map_or(0, |archive| archive.manifest.generation.value());
    loop {
        generation = generation
            .checked_add(1)
            .filter(|g| *g <= MAX_SAFE_UINT)
            .ok_or_else(|| invalid("project.json", "generation counter exhausted"))?;
        let prefix = format!("DATA/G{generation:06}/");
        if !backing.is_some_and(|archive| {
            archive
                .files
                .keys()
                .any(|path| folded(path).starts_with(&prefix))
        }) && !additions
            .iter()
            .any(|file| folded(file.entry.file_ref.path.as_str()).starts_with(&prefix))
        {
            break;
        }
    }
    assemble_archive(draft, backing, additions, generation)
}

/// A worker-owned new directory. Writes finish outside Host; only publication runs
/// after the Host's final caller/token/epoch/cancellation admission.
pub(crate) struct StagedExportDirectory {
    parent: Directory,
    staging: Option<Directory>,
    stage: PathBuf,
    name: String,
}
impl StagedExportDirectory {
    pub(crate) fn create(parent: &Path, name: &str) -> Result<Self, ApiError> {
        checked_path(name)?;
        if name.contains('/') { return Err(invalid(name, "one export directory name required")); }
        let parent = Directory::open(parent)?;
        match fs::symlink_metadata(parent.root.join(name)) {
            Ok(_) => return Err(invalid(name, "export destination already exists")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
            Err(e) => return Err(io_failure(name, e)),
        }
        let stage = loop {
            let sequence = INTERNAL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let candidate = parent.root.join(format!(".sf-execution-export-{}-{sequence}", std::process::id()));
            match fs::create_dir(&candidate) {
                Ok(()) => break candidate,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {},
                Err(e) => return Err(io_failure(name, e)),
            }
        };
        let staging = Some(Directory::open(&stage)?);
        Ok(Self { parent, staging, stage, name: name.into() })
    }
    pub(crate) fn parent(&self) -> &Path { &self.parent.root }
    pub(crate) fn write(&self, path: &str, bytes: &[u8]) -> Result<(), ApiError> {
        self.staging.as_ref().expect("uncommitted stage").write_new(path, bytes)
    }
    pub(crate) fn create_file(&self, path: &str) -> Result<File, ApiError> {
        let directory = self.staging.as_ref().expect("uncommitted stage");
        let _parents = directory.parents(path, true)?;
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(windows)]
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT).share_mode(FILE_SHARE_READ);
        let file = options.open(directory.root.join(path)).map_err(|e|io_failure(path,e))?;
        directory.check_handle(&file,path)?;
        Ok(file)
    }
    pub(crate) fn commit(mut self, cancelled: &AtomicBool) -> Result<(), ApiError> {
        // All stream handles are closed by the worker. Close the held stage root,
        // while retaining the parent handle, before Windows no-replace rename.
        self.staging.take();
        check_cancelled(cancelled)?;
        #[cfg(windows)]
        {
            let wide = |path: &Path| path.as_os_str().encode_wide().chain(Some(0)).collect::<Vec<_>>();
            let source = wide(&self.stage);
            let target = wide(&self.parent.root.join(&self.name));
            if unsafe { MoveFileW(source.as_ptr(),target.as_ptr()) } == 0 {
                return Err(failure(ErrorCode::IoFailure,
                    "Export directory publication failed; hidden uncommitted stage retained", &self.name));
            }
            Ok(())
        }
        #[cfg(not(windows))]
        Err(failure(ErrorCode::IoFailure,"atomic directory publication requires the Windows desktop filesystem",&self.name))
    }
}

/// Publish report bytes and their editable project in one new, same-volume folder.
pub(crate) fn write_evaluation_directory(
    destination_parent: &Path,
    report_directory: &str,
    project: &ProjectArchive,
    files: &[(RelativeFilePath, Vec<u8>)],
    cancelled: &AtomicBool,
) -> Result<(), ApiError> {
    checked_path(report_directory)?;
    if report_directory.contains('/') {
        return Err(invalid(
            report_directory,
            "one report directory name required",
        ));
    }
    validate_archive(project.manifest.clone(), project.files.clone())?;
    let project_root = encode(&project.manifest);
    let entries = std::iter::once(("project/project.json".to_owned(), &project_root))
        .chain(
            project
                .files
                .iter()
                .map(|(path, bytes)| (format!("project/{path}"), bytes)),
        )
        .chain(
            files
                .iter()
                .map(|(path, bytes)| (path.as_str().to_owned(), bytes)),
        )
        .collect::<Vec<_>>();
    check_paths(entries.iter().map(|(path, _)| path.as_str()))?;
    check_cancelled(cancelled)?;
    let parent = Directory::open(destination_parent)?;
    let destination = parent.root.join(report_directory);
    match fs::symlink_metadata(&destination) {
        Ok(_) => {
            return Err(invalid(
                report_directory,
                "report destination already exists",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_failure(report_directory, error)),
    }
    let stage = loop {
        let sequence = INTERNAL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = parent
            .root
            .join(format!(".sf-evaluation-{}-{sequence}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => break path,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(io_failure(report_directory, error)),
        }
    };
    {
        let staging = Directory::open(&stage)?;
        for (path, bytes) in entries {
            check_cancelled(cancelled)?;
            staging.write_new(&path, bytes)?;
        }
    }
    // Close staging handles before rename. Failed candidates remain hidden and
    // recoverable; no previously effective report directory is ever modified.
    #[cfg(windows)]
    {
        let wide = |path: &Path| {
            path.as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        };
        let source = wide(&stage);
        let target = wide(&destination);
        check_cancelled(cancelled)?;
        // MoveFileW refuses an existing destination, including an empty directory.
        if unsafe { MoveFileW(source.as_ptr(), target.as_ptr()) } == 0 {
            let cause = std::io::Error::last_os_error();
            return Err(failure(
                ErrorCode::IoFailure,
                &format!(
                    "Evaluation publication MoveFileW failed: {} -> {}: {cause}; uncommitted candidate retained at {}",
                    stage.display(), destination.display(), stage.display()
                ),
                report_directory,
            ));
        }
    }
    #[cfg(not(windows))]
    {
        return Err(failure(
            ErrorCode::IoFailure,
            "atomic directory publication requires the Windows desktop filesystem",
            report_directory,
        ));
    }
    // Rename is the commit point. A racing cancellation cannot undo success.
    Ok(())
}

pub(crate) fn write_project_archive_with_additions(
    directory: &Path,
    draft: &ProjectDraft,
    backing: Option<&ProjectArchive>,
    additions: &[ArchiveAddition],
    cancelled: &AtomicBool,
) -> Result<ProjectArchive, ApiError> {
    let project = &draft.project;
    for (id, scenario) in &draft.scenarios {
        if id != &scenario.id {
            return Err(invalid(
                "project.json",
                "current scenario map key differs from ScenarioInput.id",
            ));
        }
    }
    check_cancelled(cancelled)?;
    let directory = Directory::open(directory)?;
    // Resolve the actual committed root at this queue turn, never backing.generation.
    let mut old_with_documents = match directory.read("project.json") {
        Ok(_) => Some(read_root_with_documents(&directory, "project.json")?),
        Err(e) if matches!(e.code, ErrorCode::MissingReference) => None,
        Err(e) => return Err(e),
    };
    let retained = if let Some(source) = backing {
        if source.project.id != project.id || source.manifest.project_id != project.id {
            return Err(invalid(
                "project.json",
                "backing archive belongs to another project",
            ));
        }
        // This call already validated the target. Reuse only exact entries and
        // bytes; backing metadata and its reference closure still need validation.
        let reusable = old_with_documents.as_ref().filter(|(old, _)| {
            same(&old.manifest, &source.manifest) && old.files == source.files
        });
        if let Some((_, parsed)) = reusable {
            retained_paths_from_documents(draft, source, parsed)?
        } else {
            let reusable = old_with_documents.as_mut()
                .map(|(archive, parsed)| (&*archive, parsed));
            let (_validated, parsed) = validate_archive_reusing_documents(
                source.manifest.clone(), source.files.clone(), reusable,
            )?;
            retained_paths_from_documents(draft, source, &parsed)?
        }
    } else {
        BTreeSet::new()
    };
    // Release the parsed Run documents before assembling the new archive.
    let old = old_with_documents.map(|(archive, _)| archive);
    let mut generation = old.as_ref().map_or(0, |a| a.manifest.generation.value());
    loop {
        generation = generation
            .checked_add(1)
            .filter(|g| *g <= MAX_SAFE_UINT)
            .ok_or_else(|| invalid("project.json", "generation counter exhausted"))?;
        let relative = format!("data/g{generation:06}");
        // An empty destination can still receive an old snapshot at this path.
        // Do not reuse its generation directory for changed current documents.
        if retained
            .iter()
            .any(|p| folded(p).starts_with(&(folded(&relative) + "/")))
        {
            continue;
        }
        let marker = format!("{relative}/project-state.json");
        let _parents = directory.parents("data/placeholder", true)?;
        match fs::create_dir(directory.root.join(&relative)) {
            Ok(()) => {
                directory.parents(&marker, false)?;
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_failure(&relative, e)),
        }
    }
    let archive = assemble_archive_with_retained(draft, backing, additions, generation, retained)?;
    for (path, bytes) in &archive.files {
        check_cancelled(cancelled)?;
        match directory.read(path) {
            Ok(existing) if &existing == bytes => {}
            Ok(_) => {
                return Err(failure(
                    ErrorCode::IoFailure,
                    "immutable included path already contains different bytes; choose a new directory",
                    path,
                ));
            }
            Err(e) if matches!(e.code, ErrorCode::MissingReference) => {
                directory.write_new(path, bytes)?
            }
            Err(e) => return Err(e),
        }
    }
    check_cancelled(cancelled)?;
    let replacement = internal_name(&directory, "new-root")?;
    directory.write_new(&replacement, &encode(&archive.manifest))?;
    check_cancelled(cancelled)?;
    if let Some(old) = &old {
        let retained = internal_name(&directory, "retained-root")?;
        #[cfg(feature = "nfr06-save-faults")]
        let replaced = nfr06_before_replace(&directory, old, &retained)
            .and_then(|()| replace_root(&directory, &replacement, &retained));
        #[cfg(not(feature = "nfr06-save-faults"))]
        let replaced = replace_root(&directory, &replacement, &retained);
        if let Err(error) = replaced {
            return Err(recover_old_root(
                &directory,
                old,
                &replacement,
                &retained,
                error,
            ));
        }
        // Success is the commit point. A racing cancellation cannot undo it.
        // The retained root is no longer needed after a successful replacement.
        let _ = fs::remove_file(directory.root.join(retained));
    } else {
        fs::rename(directory.root.join(&replacement), directory.root.join("project.json")).map_err(|e| {
            failure(ErrorCode::IoFailure, format!("initial root rename failed: {e}; new root retained at {replacement}; no previous valid root was present"), "project.json")
        })?;
    }
    Ok(archive)
}

// NFR-06 explicitly requires prearranged old-root loss/recovery cases. This
// test-only feature moves real files, then still calls the real ReplaceFileW
// and normal recovery path. It never manufactures an OS error code.
#[cfg(feature = "nfr06-save-faults")]
fn nfr06_before_replace(
    directory: &Directory,
    old: &ProjectArchive,
    retained: &str,
) -> std::io::Result<()> {
    let plan_path = std::env::current_exe()?.with_file_name("nfr06-save-fault.json");
    arrange_nfr06_fault(directory, old, retained, &plan_path)
}

#[cfg(feature = "nfr06-save-faults")]
fn arrange_nfr06_fault(
    directory: &Directory,
    old: &ProjectArchive,
    retained: &str,
    plan_path: &Path,
) -> std::io::Result<()> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Plan {
        directory: PathBuf,
        branch: Branch,
        expected_project_id: String,
        expected_revision: u64,
        expected_generation: u64,
    }
    #[derive(serde::Deserialize)]
    enum Branch { F03, F04 }
    let bytes = match fs::read(plan_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let plan: Plan = serde_json::from_slice(&bytes)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
    if fs::canonicalize(&plan.directory)? != fs::canonicalize(&directory.root)? {
        return Ok(());
    }
    if plan.expected_project_id != old.manifest.project_id.as_str()
        || plan.expected_revision != old.manifest.project_revision.0.value()
        || plan.expected_generation != old.manifest.generation.value()
    {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput,
            "NFR06 fault plan does not match the frozen old project identity"));
    }
    // Consume before changing the target. A retry or saving elsewhere must not
    // repeat the fault; both files stay beside the dedicated test executable.
    fs::rename(plan_path, plan_path.with_file_name("nfr06-save-fault.consumed.json"))?;
    fs::rename(directory.root.join("project.json"), directory.root.join(retained))?;
    if matches!(plan.branch, Branch::F04) {
        fs::create_dir(directory.root.join("project.json"))?;
    }
    Ok(())
}

#[cfg(windows)]
fn replace_root(directory: &Directory, replacement: &str, retained: &str) -> std::io::Result<()> {
    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }
    let root = wide(&directory.root.join("project.json"));
    let replacement = wide(&directory.root.join(replacement));
    let retained = wide(&directory.root.join(retained));
    // Flags=0: do not ignore ACL or metadata-merge errors.
    let result = unsafe {
        ReplaceFileW(
            root.as_ptr(),
            replacement.as_ptr(),
            retained.as_ptr(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_root(
    _directory: &Directory,
    _replacement: &str,
    _retained: &str,
) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "existing package commit requires Windows ReplaceFileW",
    ))
}

fn old_identity(archive: &ProjectArchive, old: &ProjectArchive) -> bool {
    archive.manifest.project_id == old.manifest.project_id
        && same(&archive.manifest.generation, &old.manifest.generation)
        && same(
            &archive.manifest.project_revision,
            &old.manifest.project_revision,
        )
}

fn recover_old_root(
    directory: &Directory,
    old: &ProjectArchive,
    replacement: &str,
    retained: &str,
    native_error: std::io::Error,
) -> ApiError {
    let root_is_old = || read_root(directory, "project.json").is_ok_and(|a| old_identity(&a, old));
    if root_is_old() {
        return failure(
            ErrorCode::IoFailure,
            format!(
                "ReplaceFileW failed: {native_error}; parsed old project ID/generation/revision verified at project.json; new root retained at {replacement}"
            ),
            "project.json",
        );
    }
    if read_root(directory, retained).is_ok_and(|a| old_identity(&a, old)) {
        let restore = match fs::symlink_metadata(directory.root.join("project.json")) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => fs::rename(
                directory.root.join(retained),
                directory.root.join("project.json"),
            ),
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                match internal_name(directory, "uncommitted-root") {
                    Ok(displaced) => replace_root(directory, retained, &displaced),
                    Err(e) => {
                        return failure(
                            ErrorCode::IoFailure,
                            format!(
                                "ReplaceFileW failed: {native_error}; restoration could not reserve an internal name: {}; recoverable old root retained at {retained}; candidate at {replacement}; old root is NOT verified at project.json",
                                e.message
                            ),
                            "project.json",
                        );
                    }
                }
            }
            Ok(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "root is not a regular file",
            )),
            Err(e) => Err(e),
        };
        if root_is_old() {
            return failure(
                ErrorCode::IoFailure,
                format!(
                    "ReplaceFileW failed: {native_error}; old root restored and parsed project ID/generation/revision verified at project.json; candidate retained at {replacement}"
                ),
                "project.json",
            );
        }
        return failure(
            ErrorCode::IoFailure,
            format!(
                "ReplaceFileW failed: {native_error}; restore failed: {restore:?}; recoverable old root retained at {retained}; candidate at {replacement}; old root is NOT verified at project.json"
            ),
            "project.json",
        );
    }
    failure(
        ErrorCode::IoFailure,
        format!(
            "ReplaceFileW failed: {native_error}; old root is NOT verified at project.json or {retained}; all recoverable files retained, candidate at {replacement}"
        ),
        "project.json",
    )
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::{ApiRequest, ProjectHost};
    use serde_json::json;
    use std::os::windows::process::CommandExt;

    struct TempDirectory {
        path: PathBuf,
        parent: PathBuf,
    }

    impl TempDirectory {
        fn new() -> Self {
            let parent = fs::canonicalize(std::env::temp_dir()).unwrap();
            loop {
                let n = INTERNAL_SEQUENCE.fetch_add(1, Ordering::Relaxed);
                let path = parent.join(format!("sf-project-store-{}-{n}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Self { path, parent },
                    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(e) => panic!("temporary directory: {e}"),
                }
            }
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let actual = fs::canonicalize(&self.path).unwrap();
            assert_eq!(actual.parent(), Some(self.parent.as_path()));
            assert!(
                actual
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("sf-project-store-")
            );
            fs::remove_dir_all(&actual).unwrap();
        }
    }

    fn project() -> ProjectState {
        let ui = ObjectId::try_from("store-ui".to_owned()).unwrap();
        let mut host = ProjectHost::new(
            ObjectId::try_from("store-host".to_owned()).unwrap(),
            ui.clone(),
        );
        let request: ApiRequest = serde_json::from_value(json!({
            "api_version":"0.1", "request_id":"store-create", "project_id":null,
            "scope":{"kind":"workspace"}, "operation":"apply_commands",
            "params":{"base_revision":null,"commands":[{"kind":"create_project","data":{
                "title":"actual Host storage input", "time_reference":{"label":"relative milliseconds","origin_iso8601":null,"timezone":null}, "departure":"stay"
            }}]}
        })).unwrap();
        let created = host.dispatch(&ui, request);
        assert_eq!(created["outcome"]["status"], "ok", "{created:#}");
        let id = &created["outcome"]["data"]["project_id"];
        let read = host.dispatch(&ui, serde_json::from_value(json!({
            "api_version":"0.1", "request_id":"store-read", "project_id":id,
            "scope":{"kind":"project"}, "operation":"get_object",
            "params":{"object_id":{"kind":"project","id":id},"source":{"kind":"draft"},"node_index":null}
        })).unwrap());
        assert_eq!(read["outcome"]["status"], "ok", "{read:#}");
        serde_json::from_value(read["outcome"]["data"]["data"].clone()).unwrap()
    }

    #[test]
    fn evaluation_directory_publishes_report_with_movable_editable_snapshot() {
        let directory = TempDirectory::new();
        let original = project();
        let archive =
            super::snapshot_project_archive(&draft_for(&original, None), None, &[]).unwrap();
        let files = [(
            RelativeFilePath::try_from("report.json".to_owned()).unwrap(),
            b"{\"report\":true}".to_vec(),
        )];
        super::write_evaluation_directory(
            &directory.path,
            "evaluation-00000001",
            &archive,
            &files,
            &AtomicBool::new(false),
        )
        .unwrap();
        let moved = directory.path.join("moved-report");
        fs::rename(directory.path.join("evaluation-00000001"), &moved).unwrap();
        let reopened = read_project_archive(&moved.join("project")).unwrap();
        assert!(same(&original, &reopened.project));
        assert_eq!(fs::read(moved.join("report.json")).unwrap(), files[0].1);
    }

    #[test]
    fn evaluation_directory_cancellation_and_collision_preserve_previous_folder() {
        let directory = TempDirectory::new();
        let archive =
            super::snapshot_project_archive(&draft_for(&project(), None), None, &[]).unwrap();
        let old = directory.path.join("evaluation-00000001");
        fs::create_dir(&old).unwrap();
        fs::write(old.join("old.txt"), b"effective report").unwrap();
        assert!(
            super::write_evaluation_directory(
                &directory.path,
                "evaluation-00000001",
                &archive,
                &[],
                &AtomicBool::new(false)
            )
            .is_err()
        );
        assert!(
            super::write_evaluation_directory(
                &directory.path,
                "evaluation-00000002",
                &archive,
                &[],
                &AtomicBool::new(true)
            )
            .is_err()
        );
        assert_eq!(fs::read(old.join("old.txt")).unwrap(), b"effective report");
        assert!(!directory.path.join("evaluation-00000002").exists());
    }

    #[test]
    fn evaluation_directory_rejects_report_project_path_collision() {
        let directory = TempDirectory::new();
        let archive =
            super::snapshot_project_archive(&draft_for(&project(), None), None, &[]).unwrap();
        let files = [(
            RelativeFilePath::try_from("PROJECT/project.json".to_owned()).unwrap(),
            b"replaced".to_vec(),
        )];
        assert!(
            super::write_evaluation_directory(
                &directory.path,
                "evaluation-00000001",
                &archive,
                &files,
                &AtomicBool::new(false)
            )
            .is_err()
        );
        assert!(!directory.path.join("evaluation-00000001").exists());
    }

    fn set_revision(project: &mut ProjectState, revision: u64) {
        project.revision = Revision(SafeUint::try_from(revision).unwrap());
    }

    // P0-01 tests keep the same business inputs/expectations. Adapt only the
    // private snapshot carrier and Host-owned binding metadata for that input.
    fn draft_for(project: &ProjectState, source: Option<&ProjectArchive>) -> ProjectDraft {
        let mut scenarios = source.map_or_else(BTreeMap::new, |a| a.scenarios.clone());
        for scenario in scenarios.values_mut() {
            if !same(&scenario.project_revision, &project.revision) {
                scenario.project_revision = project.revision.clone();
                scenario.revision =
                    Revision(SafeUint::try_from(scenario.revision.0.value() + 1).unwrap());
            }
        }
        ProjectDraft {
            project: project.clone(),
            scenarios,
            professional: source.map_or_else(crate::ProfessionalDocuments::default, |a| {
                a.professional.clone()
            }),
        }
    }

    fn write_for_project(
        path: &Path,
        project: &ProjectState,
        source: Option<&ProjectArchive>,
        cancelled: &AtomicBool,
    ) -> Result<ProjectArchive, ApiError> {
        super::write_project_archive(path, &draft_for(project, source), source, cancelled)
    }

    fn save(
        path: &Path,
        project: &ProjectState,
        source: Option<&ProjectArchive>,
    ) -> ProjectArchive {
        write_for_project(path, project, source, &AtomicBool::new(false))
            .unwrap_or_else(|e| panic!("{:?}: {}", e.code, e.message))
    }

    fn error<T>(result: Result<T, ApiError>, code: ErrorCode) -> ApiError {
        let Err(error) = result else {
            panic!("expected error")
        };
        assert!(
            same(&error.code, &code),
            "{:?}: {}",
            error.code,
            error.message
        );
        error
    }

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tools/type-contract-proof/fixtures")
                .join(format!("{name}.json")),
        )
        .unwrap()
    }

    fn entry(
        role: DocumentRole,
        kind: DocumentKind,
        object: ObjectKind,
        id: &str,
        path: &str,
        revision: Option<Revision>,
    ) -> DocumentEntry {
        DocumentEntry {
            role,
            module_ref: None,
            object_ref: ObjectRef {
                kind: object,
                id: ObjectId::try_from(id.to_owned()).unwrap(),
            },
            revision,
            file_ref: FileRef {
                path: RelativeFilePath::try_from(path.to_owned()).unwrap(),
                kind,
            },
            required: true,
        }
    }

    /// Frozen fixtures are inputs only. Every assertion below runs production
    /// directory parsing/writes/ReplaceFileW against real temporary directories.
    fn complete_archive() -> ProjectArchive {
        let mut manifest: PackageManifest =
            decode(&fixture("manifest"), "fixture-manifest").unwrap();
        let mut files = BTreeMap::new();
        for document in &mut manifest.documents {
            if matches!(document.role, DocumentRole::ActiveProject) {
                document.file_ref.path =
                    RelativeFilePath::try_from("payload/project-state.json".to_owned()).unwrap();
                files.insert("payload/project-state.json".into(), fixture("project"));
            } else {
                let path = document.file_ref.path.as_str();
                files.insert(path.into(), fixture(path.trim_end_matches(".json")));
            }
        }
        let mut run: RunResult = decode(&fixture("run"), "run.json").unwrap();
        run.input_ref.frozen_project_file.path =
            RelativeFilePath::try_from("snapshots/project-state.json".to_owned()).unwrap();
        run.input_ref.frozen_scenario_file.path =
            RelativeFilePath::try_from("snapshots/scenario.json".to_owned()).unwrap();
        files.insert("snapshots/project-state.json".into(), fixture("project"));
        files.insert("snapshots/scenario.json".into(), fixture("scenario"));
        manifest.documents.push(entry(
            DocumentRole::FrozenRunProject,
            DocumentKind::Project,
            ObjectKind::Project,
            "P1",
            "snapshots/project-state.json",
            Some(Revision(SafeUint::try_from(1).unwrap())),
        ));
        manifest.documents.push(entry(
            DocumentRole::FrozenRunScenario,
            DocumentKind::Scenario,
            ObjectKind::Scenario,
            "SC1",
            "snapshots/scenario.json",
            Some(Revision(SafeUint::try_from(1).unwrap())),
        ));
        files.insert("run.json".into(), encode(&run));
        manifest.documents.push(entry(
            DocumentRole::RunResult,
            DocumentKind::RunResult,
            ObjectKind::Run,
            "R1",
            "run.json",
            None,
        ));
        for (name, role, kind, object, id, revision) in [
            (
                "packaging-job",
                DocumentRole::ProfessionalInput,
                DocumentKind::PackagingJob,
                ObjectKind::PackagingJob,
                "PACKJOB1",
                Some(Revision(SafeUint::try_from(1).unwrap())),
            ),
            (
                "packaging-result",
                DocumentRole::ProfessionalResult,
                DocumentKind::PackagingResult,
                ObjectKind::Artifact,
                "A1",
                None,
            ),
            (
                "artifact",
                DocumentRole::ArtifactMetadata,
                DocumentKind::ArtifactRecord,
                ObjectKind::Artifact,
                "A1",
                None,
            ),
            (
                "evaluation",
                DocumentRole::Evaluation,
                DocumentKind::Evaluation,
                ObjectKind::Report,
                "E1",
                None,
            ),
        ] {
            let path = format!("{name}.json");
            files.insert(path.clone(), fixture(name));
            manifest
                .documents
                .push(entry(role, kind, object, id, &path, revision));
        }
        let asset_id = AssetId::try_from("included-asset".to_owned()).unwrap();
        let asset_path = "assets/included.svg";
        let right = AssetRights {
            asset_ref: asset_id.clone(),
            source: Some("self-built storage test input".into()),
            license: None,
            redistribution_confirmed: false,
            missing_reason: Some("permission not granted by storage".into()),
        };
        let mut module: ModulePackage =
            decode(files.get("module-items.json").unwrap(), "module-items.json").unwrap();
        module.assets.push(AssetRef {
            id: asset_id,
            path: RelativeFilePath::try_from(asset_path.to_owned()).unwrap(),
            kind: AssetKind::Svg,
        });
        module.definition.contribution.assets.push(right.clone());
        module.rights.push(right.clone());
        files.insert("module-items.json".into(), encode(&module));
        files.insert(
            asset_path.into(),
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M0 0L1 1\"/></svg>".to_vec(),
        );
        manifest.assets.push(right);
        manifest.documents.push(entry(
            DocumentRole::Asset,
            DocumentKind::Asset,
            ObjectKind::Asset,
            "included-asset",
            asset_path,
            None,
        ));
        files.insert("support/notes.bin".into(), vec![0, 255, 17, 10, 0]);
        manifest.documents.push(entry(
            DocumentRole::SupportingFile,
            DocumentKind::Asset,
            ObjectKind::Asset,
            "support-notes",
            "support/notes.bin",
            None,
        ));
        validate_archive(manifest, files)
            .unwrap_or_else(|e| panic!("complete input {:?}: {}", e.code, e.message))
    }

    #[test]
    fn standalone_module_export_keeps_only_selected_dependency_gaps() {
        let mut source = complete_archive();
        let asset_gap: Issue = serde_json::from_value(json!({
            "code":"handoff.asset_not_redistributed",
            "object_ref":{"kind":"asset","id":"included-asset"},
            "field_path":"assets/included.svg",
            "message":"Retained source permission is unresolved",
            "blocks":["preview"]
        })).unwrap();
        source.manifest.missing.push(asset_gap.clone());
        let mut draft = draft_for(&source.project, Some(&source));
        draft.project.issues.push(asset_gap);
        for (module_id, expected_gap) in [("demo.operations", false), ("demo.items", true)] {
            let selection = serde_json::from_value(json!({
                "purpose":"community_module", "scenarios":[], "run_ids":[],
                "artifact_ids":[], "module_refs":[{"id":module_id,"version":"0.1.0"}],
                "include_redistributable_assets":false
            })).unwrap();
            let exported = prepare_handoff_archive(&draft, Some(&source), &[], &selection,
                &BTreeSet::new(), &AtomicBool::new(false)).unwrap();
            let has_gap = exported.manifest.missing.iter().any(|issue|
                issue.object_ref.as_ref().is_some_and(|object| object.id.as_str() == "included-asset"));
            assert_eq!(has_gap, expected_gap, "{module_id}: {:#?}", exported.manifest.missing);
            assert!(exported.project.issues.is_empty(), "Stripped Project issues must not enter a module-only package");
            if expected_gap {
                assert!(exported.manifest.assets.iter().any(|right|
                    right.asset_ref.as_str() == "included-asset" && right.license.is_none() && !right.redistribution_confirmed));
                assert!(!exported.files.contains_key("assets/included.svg"));
            }
        }
        assert_eq!(draft.project.issues.len(), 1, "The source remains unchanged");
    }

    #[test]
    fn conflicting_professional_identities_in_distinct_files_fail_read_and_leave_host_unchanged() {
        fn call(
            host: &mut ProjectHost,
            project: Option<&ProjectId>,
            operation: &str,
            params: Value,
        ) -> Value {
            host.dispatch(&ObjectId::try_from("collision-ui".to_owned()).unwrap(), serde_json::from_value(json!({"api_version":"0.1","request_id":"collision-test","project_id":project,"scope":{"kind":if project.is_some(){"project"}else{"workspace"}},"operation":operation,"params":params})).unwrap())
        }
        let ui = ObjectId::try_from("collision-ui".to_owned()).unwrap();
        let mut host = ProjectHost::new(
            ObjectId::try_from("collision-host".to_owned()).unwrap(),
            ui.clone(),
        );
        let response = call(
            &mut host,
            None,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"unchanged active project","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let project: ProjectId =
            serde_json::from_value(response["outcome"]["data"]["project_id"].clone()).unwrap();
        let read_params = json!({"object_id":{"kind":"project","id":project},"source":{"kind":"draft"},"node_index":null});
        let before = call(&mut host, Some(&project), "get_object", read_params.clone())["outcome"]
            ["data"]
            .clone();
        for (kind, expected) in [
            (DocumentKind::PackagingJob, "PackagingJob"),
            (DocumentKind::ArtifactRecord, "ArtifactRecord"),
            (DocumentKind::PackagingResult, "PackagingResult"),
        ] {
            let mut archive = complete_archive();
            let original = archive
                .manifest
                .documents
                .iter()
                .find(|entry| same(&entry.file_ref.kind, &kind))
                .unwrap()
                .clone();
            let original_path = original.file_ref.path.as_str();
            let changed = match kind {
                DocumentKind::PackagingJob => {
                    let mut job: PackageStructureJob =
                        decode(&archive.files[original_path], original_path).unwrap();
                    job.panels[0].thickness_mm =
                        FiniteNumber::try_from(job.panels[0].thickness_mm.value() + 1.0).unwrap();
                    encode(&job)
                }
                DocumentKind::ArtifactRecord => {
                    let mut artifact: ArtifactRecord =
                        decode(&archive.files[original_path], original_path).unwrap();
                    artifact
                        .artifact_type
                        .push_str("; conflicting same-ID claim");
                    encode(&artifact)
                }
                DocumentKind::PackagingResult => {
                    let mut result: PackagingResult =
                        decode(&archive.files[original_path], original_path).unwrap();
                    result.limitations.push(Issue {
                        code: "changed_same_identity".into(),
                        object_ref: None,
                        field_path: None,
                        message: "different immutable result content".into(),
                        blocks: vec![],
                    });
                    encode(&result)
                }
                _ => unreachable!(),
            };
            let mut duplicate = original.clone();
            let duplicate_path = format!("conflicts/{expected}.json");
            duplicate.file_ref.path = RelativeFilePath::try_from(duplicate_path.clone()).unwrap();
            archive.manifest.documents.push(duplicate);
            archive.files.insert(duplicate_path, changed);
            let directory = TempDirectory::new();
            let opened = Directory::open(&directory.path).unwrap();
            for (path, bytes) in &archive.files {
                opened.write_new(path, bytes).unwrap();
            }
            opened
                .write_new("project.json", &encode(&archive.manifest))
                .unwrap();
            drop(opened);
            let failure = error(
                read_project_archive(&directory.path),
                ErrorCode::InvalidInput,
            );
            assert!(
                failure
                    .message
                    .contains(&format!("conflicting immutable {expected} identity")),
                "{failure:?}"
            );

            let selected = host
                .select_directory(
                    &ui,
                    crate::FilePurpose::ImportPackage,
                    directory.path.clone(),
                )
                .unwrap();
            let response = call(
                &mut host,
                None,
                "import_package",
                json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            let task = response["outcome"]["data"]["id"].clone();
            let mut work = host.drain_io_work();
            assert_eq!(work.len(), 1);
            host.finish_io(work.pop().unwrap().run());
            let state = call(&mut host, None, "get_task", json!({"task_id":task}));
            assert_eq!(
                state["outcome"]["data"]["task"]["state"], "failed",
                "{state}"
            );
            assert_eq!(
                state["outcome"]["data"]["failure"]["code"], "InvalidInput",
                "{state}"
            );
            assert_eq!(
                call(&mut host, Some(&project), "get_object", read_params.clone())["outcome"]["data"],
                before
            );
        }
    }

    #[test]
    fn exact_professional_copies_and_distinct_job_versions_remain_valid_archives() {
        let mut archive = complete_archive();
        let original_job = archive.professional.jobs.values().next().unwrap().clone();
        let originals: Vec<_> = archive
            .manifest
            .documents
            .iter()
            .filter(|entry| {
                matches!(
                    entry.file_ref.kind,
                    DocumentKind::PackagingJob
                        | DocumentKind::ArtifactRecord
                        | DocumentKind::PackagingResult
                )
            })
            .cloned()
            .collect();
        for (index, original) in originals.iter().enumerate() {
            let path = format!("copies/document-{:04}.json", index + 1);
            archive.files.insert(
                path.clone(),
                archive.files[original.file_ref.path.as_str()].clone(),
            );
            let mut duplicate = original.clone();
            duplicate.file_ref.path = RelativeFilePath::try_from(path).unwrap();
            archive.manifest.documents.push(duplicate);
        }
        let mut next = original_job.clone();
        next.revision = Revision(SafeUint::try_from(original_job.revision.0.value() + 1).unwrap());
        let mut next_entry = originals
            .iter()
            .find(|entry| matches!(entry.file_ref.kind, DocumentKind::PackagingJob))
            .unwrap()
            .clone();
        next_entry.revision = Some(next.revision.clone());
        next_entry.file_ref.path =
            RelativeFilePath::try_from("copies/next-job-version.json".to_owned()).unwrap();
        archive
            .files
            .insert(next_entry.file_ref.path.as_str().into(), encode(&next));
        archive.manifest.documents.push(next_entry);
        let directory = TempDirectory::new();
        let opened = Directory::open(&directory.path).unwrap();
        for (path, bytes) in &archive.files {
            opened.write_new(path, bytes).unwrap();
        }
        opened
            .write_new("project.json", &encode(&archive.manifest))
            .unwrap();
        drop(opened);
        let actual = read_project_archive(&directory.path).unwrap();
        assert_eq!(actual.professional.jobs.len(), 2);
        assert_eq!(actual.professional.artifacts.len(), 1);
        assert_eq!(actual.professional.results.len(), 1);
        assert!(same(
            &actual.professional.jobs[&(original_job.id.clone(), original_job.revision.0.value())],
            &original_job
        ));
        assert!(same(
            &actual.professional.jobs[&(next.id.clone(), next.revision.0.value())],
            &next
        ));
    }

    #[test]
    fn initial_and_second_generation_reopen_without_overwriting_old_data() {
        let directory = TempDirectory::new();
        let mut project = project();
        // Opaque IDs remain data even when they would be unsafe as file names.
        project.id = ProjectId::try_from("opaque:/../CON id".to_owned()).unwrap();
        let first = save(&directory.path, &project, None);
        let old_path = first.manifest.documents[0].file_ref.path.as_str();
        let old_bytes = fs::read(directory.path.join(old_path)).unwrap();
        set_revision(&mut project, 2);
        project.title = "second generation".into();
        let second = save(&directory.path, &project, Some(&first));
        assert_eq!(second.manifest.generation.value(), 2);
        assert_eq!(fs::read(directory.path.join(old_path)).unwrap(), old_bytes);
        let reopened = read_project_archive(&directory.path).unwrap();
        assert!(same(&reopened.project, &project));
        assert!(same(&reopened.manifest, &second.manifest));
        assert!(
            decode::<PackageManifest>(
                &fs::read(directory.path.join("project.json")).unwrap(),
                "project.json"
            )
            .is_ok()
        );
    }

    #[test]
    fn stale_backing_does_not_reuse_a_committed_or_orphan_generation() {
        let directory = TempDirectory::new();
        let mut project = project();
        let first = save(&directory.path, &project, None);
        set_revision(&mut project, 2);
        save(&directory.path, &project, Some(&first));
        fs::create_dir(directory.path.join("data/g000003")).unwrap();
        set_revision(&mut project, 3);
        let third = save(&directory.path, &project, Some(&first));
        assert_eq!(third.manifest.generation.value(), 4);
        assert_eq!(
            read_project_archive(&directory.path)
                .unwrap()
                .project
                .revision
                .0
                .value(),
            3
        );
    }

    #[test]
    fn target_validation_reuse_does_not_excuse_changed_backing_bytes() {
        let directory = TempDirectory::new();
        let first = save(&directory.path, &project(), None);
        let old_root = fs::read(directory.path.join("project.json")).unwrap();
        let mut changed = first.clone();
        let active = changed.manifest.documents.iter()
            .find(|entry| matches!(entry.role, DocumentRole::ActiveProject)).unwrap()
            .file_ref.path.as_str().to_owned();
        changed.files.insert(active, b"not JSON".to_vec());
        error(write_for_project(&directory.path, &first.project, Some(&changed),
            &AtomicBool::new(false)), ErrorCode::InvalidInput);
        assert_eq!(fs::read(directory.path.join("project.json")).unwrap(), old_root);
        assert!(same(&read_project_archive(&directory.path).unwrap().project, &first.project));
    }

    #[test]
    fn target_validation_reuse_does_not_excuse_changed_backing_manifest() {
        let directory = TempDirectory::new();
        let first = save(&directory.path, &project(), None);
        let old_root = fs::read(directory.path.join("project.json")).unwrap();
        let mut changed = first.clone();
        let active = changed.manifest.documents.iter_mut()
            .find(|entry| matches!(entry.role, DocumentRole::ActiveProject)).unwrap();
        active.revision = Some(Revision(SafeUint::try_from(2).unwrap()));
        error(write_for_project(&directory.path, &first.project, Some(&changed),
            &AtomicBool::new(false)), ErrorCode::InvalidInput);
        assert_eq!(fs::read(directory.path.join("project.json")).unwrap(), old_root);
        assert!(same(&read_project_archive(&directory.path).unwrap().project, &first.project));
    }

    #[test]
    fn target_validation_reuse_still_checks_backing_reference_closure() {
        let directory = TempDirectory::new();
        let source = complete_archive();
        let first = save(&directory.path, &source.project, Some(&source));
        let old_root = fs::read(directory.path.join("project.json")).unwrap();
        let mut changed = first.clone();
        changed.manifest.module_refs.clear();
        error(write_for_project(&directory.path, &first.project, Some(&changed),
            &AtomicBool::new(false)), ErrorCode::InvalidInput);
        assert_eq!(fs::read(directory.path.join("project.json")).unwrap(), old_root);
        assert!(same(&read_project_archive(&directory.path).unwrap().project, &first.project));
    }

    #[test]
    fn all_roles_nine_modules_and_related_bytes_survive_another_directory() {
        let source = complete_archive();
        let original = TempDirectory::new();
        let first = save(&original.path, &source.project, Some(&source));
        let reopened = read_project_archive(&original.path).unwrap();
        let destination = TempDirectory::new();
        let mut changed = reopened.project.clone();
        changed.title = "edited after reopening".into();
        changed.item_types[0].title = "renamed item; unit/revision preserved".into();
        set_revision(&mut changed, 2);
        let moved = save(&destination.path, &changed, Some(&reopened));
        let active = first
            .manifest
            .documents
            .iter()
            .find(|e| matches!(e.role, DocumentRole::ActiveProject))
            .unwrap()
            .file_ref
            .path
            .as_str();
        for (path, bytes) in &first.files {
            if path != active {
                assert_eq!(moved.files.get(path), Some(bytes), "lost/changed {path}");
            }
        }
        let actual = read_project_archive(&destination.path).unwrap();
        assert!(same(&actual.project, &changed));
        assert_eq!(actual.manifest.module_refs.len(), 9);
        assert!(same(&actual.manifest.assets, &source.manifest.assets));
        assert_eq!(
            actual.files["assets/included.svg"],
            source.files["assets/included.svg"]
        );
        assert_eq!(actual.files["support/notes.bin"], vec![0, 255, 17, 10, 0]);
        assert!(same(
            &actual.project.connections,
            &source.project.connections
        ));
        assert!(same(
            &actual.project.item_types[0].unit,
            &source.project.item_types[0].unit
        ));
    }

    #[test]
    fn cancellation_before_commit_keeps_old_root_and_old_generation() {
        let directory = TempDirectory::new();
        let mut project = project();
        let first = save(&directory.path, &project, None);
        let root = fs::read(directory.path.join("project.json")).unwrap();
        set_revision(&mut project, 2);
        error(
            write_for_project(
                &directory.path,
                &project,
                Some(&first),
                &AtomicBool::new(true),
            ),
            ErrorCode::Cancelled,
        );
        assert_eq!(fs::read(directory.path.join("project.json")).unwrap(), root);
        assert_eq!(
            read_project_archive(&directory.path)
                .unwrap()
                .manifest
                .generation
                .value(),
            1
        );
    }

    #[test]
    fn unsupported_schema_identifies_document_actual_and_supported_version() {
        let bytes = serde_json::to_vec(&json!({
            "schema_version": "99.0", "data": {}
        })).unwrap();
        for path in ["project.json", "module-devices.json"] {
            let failure = error(decode::<Value>(&bytes, path), ErrorCode::UnsupportedSchema);
            assert!(failure.message.contains(path));
            assert!(failure.message.contains("99.0"));
            assert!(failure.message.contains("当前支持 schema_version=\"1.0\""));
        }
    }

    #[test]
    fn typed_root_required_fields_versions_roles_and_revision_are_enforced() {
        let directory = TempDirectory::new();
        let archive = save(&directory.path, &project(), None);
        let original: Value =
            serde_json::from_slice(&fs::read(directory.path.join("project.json")).unwrap())
                .unwrap();
        for (case, code) in [
            ("version", ErrorCode::UnsupportedSchema),
            ("required", ErrorCode::InvalidInput),
            ("role", ErrorCode::InvalidInput),
            ("revision", ErrorCode::InvalidInput),
            ("missing", ErrorCode::MissingReference),
        ] {
            let mut value = original.clone();
            match case {
                "version" => value["schema_version"] = json!("9.0"),
                "required" => {
                    value["data"].as_object_mut().unwrap().remove("documents");
                }
                "role" => value["data"]["documents"][0]["role"] = json!("scenario_input"),
                "revision" => value["data"]["project_revision"] = json!(99),
                "missing" => {
                    value["data"]["documents"][0]["file_ref"]["path"] = json!("data/absent.json")
                }
                _ => unreachable!(),
            }
            fs::write(
                directory.path.join("project.json"),
                serde_json::to_vec(&value).unwrap(),
            )
            .unwrap();
            error(read_project_archive(&directory.path), code);
        }
        fs::write(
            directory.path.join("project.json"),
            encode(&archive.manifest),
        )
        .unwrap();
        let path = archive.manifest.documents[0].file_ref.path.as_str();
        let mut value: Value = serde_json::from_slice(&archive.files[path]).unwrap();
        value["data"].as_object_mut().unwrap().remove("item_types");
        fs::write(
            directory.path.join(path),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        error(
            read_project_archive(&directory.path),
            ErrorCode::InvalidInput,
        );
    }

    #[test]
    fn windows_path_components_casefold_and_file_directory_collisions_are_rejected() {
        for path in [
            "/absolute",
            "C:/drive",
            "../escape",
            "a/./file",
            "a\\file",
            "NUL.json",
            "dir/COM1.txt",
            "dir/LPT²",
            "a/file.",
            "a/file ",
            "a/name:stream",
            "a/*.json",
        ] {
            error(checked_path(path), ErrorCode::InvalidInput);
        }
        for paths in [
            ["A/file.json", "a/FILE.json"],
            ["data", "data/file.json"],
            ["data/file.json", "DATA"],
        ] {
            error(check_paths(paths), ErrorCode::InvalidInput);
        }
        let directory = TempDirectory::new();
        let archive = save(&directory.path, &project(), None);
        let mut root = archive.manifest.clone();
        root.documents[0].file_ref.path =
            RelativeFilePath::try_from("CON.json".to_owned()).unwrap();
        fs::write(directory.path.join("project.json"), encode(&root)).unwrap();
        error(
            read_project_archive(&directory.path),
            ErrorCode::InvalidInput,
        );
    }

    #[test]
    fn junction_escape_is_rejected_by_actual_directory_handles() {
        let directory = TempDirectory::new();
        let outside = TempDirectory::new();
        let archive = save(&directory.path, &project(), None);
        fs::write(
            outside.path.join("project-state.json"),
            encode(&archive.project),
        )
        .unwrap();
        let link = directory.path.join("escape");
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&outside.path)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut root = archive.manifest.clone();
        root.documents[0].file_ref.path =
            RelativeFilePath::try_from("escape/project-state.json".to_owned()).unwrap();
        fs::write(directory.path.join("project.json"), encode(&root)).unwrap();
        let result = read_project_archive(&directory.path);
        // Remove only our junction, without recursively traversing the outside directory.
        fs::remove_dir(&link).unwrap();
        let e = match result {
            Err(e) => e,
            Ok(_) => panic!("junction escape accepted"),
        };
        assert!(matches!(
            e.code,
            ErrorCode::InvalidInput | ErrorCode::IoFailure
        ));
        assert_eq!(
            fs::read(outside.path.join("project-state.json")).unwrap(),
            encode(&archive.project)
        );
    }

    #[test]
    fn included_bytes_cannot_overwrite_an_old_root_referenced_path() {
        let directory = TempDirectory::new();
        let source = complete_archive();
        let first = save(&directory.path, &source.project, Some(&source));
        let old_root = fs::read(directory.path.join("project.json")).unwrap();
        let mut changed = first.clone();
        changed
            .files
            .insert("support/notes.bin".into(), vec![9, 9, 9]);
        let mut project = first.project.clone();
        set_revision(&mut project, 2);
        error(
            write_for_project(
                &directory.path,
                &project,
                Some(&changed),
                &AtomicBool::new(false),
            ),
            ErrorCode::IoFailure,
        );
        assert_eq!(
            fs::read(directory.path.join("project.json")).unwrap(),
            old_root
        );
        assert_eq!(
            read_project_archive(&directory.path).unwrap().files["support/notes.bin"],
            vec![0, 255, 17, 10, 0]
        );
    }

    #[test]
    fn actual_replacefile_sharing_failure_parses_and_preserves_the_old_root() {
        let directory = TempDirectory::new();
        let mut project = project();
        let old = save(&directory.path, &project, None);
        let bytes = fs::read(directory.path.join("project.json")).unwrap();
        let held = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(directory.path.join("project.json"))
            .unwrap();
        set_revision(&mut project, 2);
        let e = error(
            write_for_project(
                &directory.path,
                &project,
                Some(&old),
                &AtomicBool::new(false),
            ),
            ErrorCode::IoFailure,
        );
        assert!(
            e.message
                .contains("parsed old project ID/generation/revision verified"),
            "{}",
            e.message
        );
        assert!(e.message.contains("os error 32"), "{}", e.message);
        drop(held);
        assert_eq!(
            fs::read(directory.path.join("project.json")).unwrap(),
            bytes
        );
        assert!(old_identity(
            &read_project_archive(&directory.path).unwrap(),
            &old
        ));
    }

    #[test]
    #[ignore = "Explicit Windows storage diagnostic using owned temporary files and ACL"]
    fn p203_real_1176_preserves_complete_old_archive() {
        let directory = TempDirectory::new();
        let source = complete_archive();
        let old = save(&directory.path, &source.project, Some(&source));
        let original = fs::read(directory.path.join("project.json")).unwrap();
        fs::write(directory.path.join("candidate.json"), b"candidate base").unwrap();
        fs::write(directory.path.join("candidate.json:replacement"), &original).unwrap();
        let opened = Directory::open(&directory.path).unwrap();
        let actual = replace_root(&opened, "candidate.json:replacement", "retained.json").unwrap_err();
        assert_eq!(actual.raw_os_error(), Some(1176));
        assert_eq!(fs::read(directory.path.join("project.json")).unwrap(), original);
        let failure = recover_old_root(&opened, &old, "candidate.json:replacement", "retained.json", actual);
        assert!(matches!(failure.code, ErrorCode::IoFailure));
        assert!(failure.message.contains("parsed old project ID/generation/revision verified at project.json"));
        let reopened = read_project_archive(&directory.path).unwrap();
        assert!(old_identity(&reopened, &old));
        assert_eq!(reopened.files, old.files);
        // ADS is not a legal public package path; exercise the OS/recovery
        // boundary directly without weakening the normal path validator.
        assert!(checked_path("candidate.json:replacement").is_err());
        let evidence = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/cu-closeout-20261023/root-P203-reception/real-1176-storage.json");
        fs::create_dir_all(evidence.parent().unwrap()).unwrap();
        fs::write(evidence, serde_json::to_vec_pretty(&json!({"actual_api":"ReplaceFileW",
            "actual_win32_error":1176,"old_root_remained":true,"old_archive_readback":true,
            "failure":failure,"old_manifest":old.manifest,
            "scope":"actual OS/recovery boundary; ADS rejected by public path validator; not ordinary Host commit or Native"})).unwrap()).unwrap();
    }

    #[test]
    #[ignore = "Explicit Windows storage diagnostic using owned temporary files and ACL"]
    fn p203_real_1177_retained_archive_recovery_and_blocked_recovery() {
        struct DenyAdd(PathBuf);
        impl Drop for DenyAdd {
            fn drop(&mut self) {
                let result = std::process::Command::new("icacls")
                    .arg(&self.0).args(["/remove:d", "*S-1-1-0"]).output().unwrap();
                assert!(result.status.success());
            }
        }
        let mut records = Vec::new();
        for block_recovery in [false, true] {
            let directory = TempDirectory::new();
            let source = complete_archive();
            let old = save(&directory.path, &source.project, Some(&source));
            let original = fs::read(directory.path.join("project.json")).unwrap();
            fs::create_dir(directory.path.join("permitted")).unwrap();
            fs::write(directory.path.join("permitted/candidate.json"), &original).unwrap();
            let opened = Directory::open(&directory.path).unwrap();
            // This ACL is non-inheriting: a retained root can move into the
            // preexisting child, but its replacement cannot enter the root.
            let setup = std::process::Command::new("icacls").arg(&directory.path)
                .args(["/deny", "*S-1-1-0:(WD)"]).output().unwrap();
            assert!(setup.status.success());
            let guard = DenyAdd(directory.path.clone());
            let actual = replace_root(&opened, "permitted/candidate.json", "permitted/retained.json").unwrap_err();
            assert_eq!(actual.raw_os_error(), Some(1177));
            assert!(!directory.path.join("project.json").exists());
            assert_eq!(fs::read(directory.path.join("permitted/retained.json")).unwrap(), original);
            assert!(old_identity(&read_root(&opened, "permitted/retained.json").unwrap(), &old));
            let guard = if block_recovery { Some(guard) } else { drop(guard); None };
            let failure = recover_old_root(&opened, &old, "permitted/candidate.json", "permitted/retained.json", actual);
            assert!(matches!(failure.code, ErrorCode::IoFailure));
            if block_recovery {
                assert!(failure.message.contains("old root is NOT verified at project.json"), "{}", failure.message);
                assert!(!directory.path.join("project.json").exists());
                assert!(old_identity(&read_root(&opened, "permitted/retained.json").unwrap(), &old));
            } else {
                assert!(failure.message.contains("old root restored and parsed"), "{}", failure.message);
                let reopened = read_project_archive(&directory.path).unwrap();
                assert!(old_identity(&reopened, &old));
                assert_eq!(reopened.files, old.files);
                assert_eq!(fs::read(directory.path.join("project.json")).unwrap(), original);
            }
            records.push(json!({"actual_api":"ReplaceFileW", "actual_win32_error":1177,
                "backup":"permitted/retained.json", "replacement":"permitted/candidate.json",
                "root_absent_immediately_after_failure":true, "old_archive_readback":true,
                "recovery_blocked":block_recovery, "root_healthy_after_recovery":!block_recovery,
                "failure":failure, "old_manifest":old.manifest,
                "scope":"actual storage API and production recovery; nested backup diagnostic, not ordinary Host same-directory commit or Native"}));
            drop(guard);
        }
        let evidence = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/cu-closeout-20261023/root-P203-reception/real-1177-storage.json");
        fs::create_dir_all(evidence.parent().unwrap()).unwrap();
        fs::write(evidence, serde_json::to_vec_pretty(&records).unwrap()).unwrap();
    }

    #[test]
    fn actual_retained_root_is_restored_when_root_was_moved() {
        let directory = TempDirectory::new();
        let old = save(&directory.path, &project(), None);
        fs::rename(
            directory.path.join("project.json"),
            directory.path.join("retained.json"),
        )
        .unwrap();
        let opened = Directory::open(&directory.path).unwrap();
        let e = recover_old_root(
            &opened,
            &old,
            "candidate.json",
            "retained.json",
            std::io::Error::from_raw_os_error(1177),
        );
        assert!(matches!(e.code, ErrorCode::IoFailure));
        assert!(
            e.message.contains("old root restored and parsed"),
            "{}",
            e.message
        );
        assert!(old_identity(
            &read_project_archive(&directory.path).unwrap(),
            &old
        ));
        // The 1177 aftermath is arranged by real file moves, not claimed as an OS-produced error.
    }

    #[cfg(feature = "nfr06-save-faults")]
    #[test]
    fn nfr06_controlled_candidate_uses_real_replace_failure_and_original_recovery() {
        for branch in ["F03", "F04"] {
            let directory = TempDirectory::new();
            let old = save(&directory.path, &project(), None);
            let opened = Directory::open(&directory.path).unwrap();
            let plan = directory.path.join("nfr06-save-fault.json");
            fs::write(&plan, serde_json::to_vec(&json!({
                "directory": directory.path,
                "branch": branch,
                "expected_project_id": old.manifest.project_id,
                "expected_revision": old.manifest.project_revision,
                "expected_generation": old.manifest.generation,
            })).unwrap()).unwrap();
            fs::write(directory.path.join("candidate.json"), encode(&old.manifest)).unwrap();
            arrange_nfr06_fault(&opened, &old, "retained.json", &plan).unwrap();
            assert!(!plan.exists());
            assert!(directory.path.join("nfr06-save-fault.consumed.json").is_file());
            let actual_error = replace_root(&opened, "candidate.json", "retained.json").unwrap_err();
            assert!(actual_error.raw_os_error().is_some());
            let failure = recover_old_root(&opened, &old, "candidate.json", "retained.json", actual_error);
            assert!(matches!(failure.code, ErrorCode::IoFailure));
            assert!(directory.path.join("candidate.json").is_file());
            if branch == "F03" {
                assert!(failure.message.contains("old root restored and parsed"));
                assert!(old_identity(&read_project_archive(&directory.path).unwrap(), &old));
                // A second call cannot recreate the consumed one-shot fault.
                arrange_nfr06_fault(&opened, &old, "unused.json", &plan).unwrap();
                assert!(directory.path.join("project.json").is_file());
            } else {
                assert!(failure.message.contains("old root is NOT verified at project.json"));
                assert!(directory.path.join("project.json").is_dir());
                assert!(old_identity(&read_root(&opened, "retained.json").unwrap(), &old));
            }
        }
    }

    #[cfg(feature = "nfr06-save-faults")]
    #[test]
    fn nfr06_controlled_candidate_requires_exact_target_and_frozen_identity() {
        let directory = TempDirectory::new();
        let other = TempDirectory::new();
        let old = save(&directory.path, &project(), None);
        let opened = Directory::open(&directory.path).unwrap();
        let plan = directory.path.join("nfr06-save-fault.json");
        let mut value = json!({
            "directory": other.path,
            "branch": "F03",
            "expected_project_id": old.manifest.project_id,
            "expected_revision": old.manifest.project_revision,
            "expected_generation": old.manifest.generation,
        });
        fs::write(&plan, serde_json::to_vec(&value).unwrap()).unwrap();
        arrange_nfr06_fault(&opened, &old, "retained.json", &plan).unwrap();
        assert!(plan.is_file());
        value["directory"] = json!(directory.path);
        value["expected_generation"] = json!(old.manifest.generation.value() + 1);
        fs::write(&plan, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(arrange_nfr06_fault(&opened, &old, "retained.json", &plan).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput);
        assert!(plan.is_file());
        assert!(!directory.path.join("retained.json").exists());
        assert!(old_identity(&read_project_archive(&directory.path).unwrap(), &old));
    }

    #[test]
    fn restoration_failure_retains_recoverable_files_and_does_not_claim_intact_root() {
        let directory = TempDirectory::new();
        let old = save(&directory.path, &project(), None);
        fs::rename(
            directory.path.join("project.json"),
            directory.path.join("retained.json"),
        )
        .unwrap();
        fs::create_dir(directory.path.join("project.json")).unwrap();
        let opened = Directory::open(&directory.path).unwrap();
        let e = recover_old_root(
            &opened,
            &old,
            "candidate.json",
            "retained.json",
            std::io::Error::from_raw_os_error(1177),
        );
        assert!(matches!(e.code, ErrorCode::IoFailure));
        assert!(
            e.message
                .contains("old root is NOT verified at project.json"),
            "{}",
            e.message
        );
        assert!(e.message.contains("retained.json"));
        assert!(old_identity(
            &read_root(&opened, "retained.json").unwrap(),
            &old
        ));
    }

    #[test]
    fn a_retained_filename_without_the_old_identity_is_not_a_recovery_source() {
        let directory = TempDirectory::new();
        let old = save(&directory.path, &project(), None);
        let mut forged = old.manifest.clone();
        forged.generation = SafeUint::try_from(99).unwrap();
        fs::write(directory.path.join("retained.json"), encode(&forged)).unwrap();
        fs::remove_file(directory.path.join("project.json")).unwrap();
        let opened = Directory::open(&directory.path).unwrap();
        let e = recover_old_root(
            &opened,
            &old,
            "candidate.json",
            "retained.json",
            std::io::Error::from_raw_os_error(1177),
        );
        assert!(
            e.message.contains("old root is NOT verified"),
            "{}",
            e.message
        );
        assert!(!directory.path.join("project.json").exists());
        assert!(directory.path.join("retained.json").exists());
    }

    #[test]
    fn explicitly_reported_draft_gaps_do_not_excuse_missing_required_documents() {
        let directory = TempDirectory::new();
        let mut project = project();
        project
            .scenario_refs
            .push(ScenarioId::try_from("missing-scenario".to_owned()).unwrap());
        project.issues.push(Issue {
            code: "missing_scenario".into(),
            object_ref: Some(ObjectRef {
                kind: ObjectKind::Scenario,
                id: ObjectId::try_from("missing-scenario".to_owned()).unwrap(),
            }),
            field_path: Some("/scenario_refs/0".into()),
            message: "scenario input has not been supplied".into(),
            blocks: vec![],
        });
        let saved = save(&directory.path, &project, None);
        assert!(same(
            &read_project_archive(&directory.path)
                .unwrap()
                .project
                .issues,
            &project.issues
        ));
        let mut manifest = saved.manifest;
        manifest.documents.push(entry(
            DocumentRole::ScenarioInput,
            DocumentKind::Scenario,
            ObjectKind::Scenario,
            "missing-scenario",
            "not-supplied.json",
            Some(Revision(SafeUint::try_from(1).unwrap())),
        ));
        fs::write(directory.path.join("project.json"), encode(&manifest)).unwrap();
        error(
            read_project_archive(&directory.path),
            ErrorCode::MissingReference,
        );
        manifest.documents.last_mut().unwrap().required = false;
        fs::write(directory.path.join("project.json"), encode(&manifest)).unwrap();
        let draft = read_project_archive(&directory.path).unwrap();
        assert!(same(&draft.project.scenario_refs, &project.scenario_refs));
        assert!(!draft.files.contains_key("not-supplied.json"));
    }

    fn empty_scenario(project: &ProjectState, id: &str) -> ScenarioInput {
        ScenarioInput {
            acceptance_goal: None,
            quality_presets: vec![],
            id: ScenarioId::try_from(id.to_owned()).unwrap(),
            revision: Revision(SafeUint::try_from(1).unwrap()),
            project_revision: project.revision.clone(),
            title: "current scenario storage input".into(),
            time_reference: project.time_reference.clone(),
            window: WindowMs {
                start_ms: SimTimeMs(SafeUint::try_from(0).unwrap()),
                end_ms: SimTimeMs(SafeUint::try_from(1000).unwrap()),
            },
            initial_conditions: InitialConditions {
                items: vec![],
                initial_setup_families: BTreeMap::new(),
                declared_quality_decisions: vec![],
                material_allocations: vec![],
            },
            orders: vec![],
            production_tasks: vec![],
            arrivals: vec![],
            supplies: vec![],
            parameter_overrides: vec![],
            scope: vec![],
            issues: vec![],
            cost_inputs: vec![],
        }
    }

    fn current_scenario_path(archive: &ProjectArchive) -> String {
        archive
            .manifest
            .documents
            .iter()
            .find(|e| matches!(e.role, DocumentRole::ScenarioInput))
            .unwrap()
            .file_ref
            .path
            .as_str()
            .into()
    }

    fn save_draft(
        path: &Path,
        draft: &ProjectDraft,
        backing: Option<&ProjectArchive>,
    ) -> ProjectArchive {
        super::write_project_archive(path, draft, backing, &AtomicBool::new(false))
            .unwrap_or_else(|e| panic!("{:?}: {}", e.code, e.message))
    }

    #[test]
    fn current_scenario_create_update_and_unreferenced_delete_share_the_root_transaction() {
        let directory = TempDirectory::new();
        let mut draft = ProjectDraft {
            project: project(),
            scenarios: BTreeMap::new(),
            professional: crate::ProfessionalDocuments::default(),
        };
        let scenario = empty_scenario(&draft.project, "opaque:/../NUL scenario");
        let id = scenario.id.clone();
        draft.project.scenario_refs.push(id.clone());
        draft.scenarios.insert(id.clone(), scenario);
        let first = save_draft(&directory.path, &draft, None);
        let old_path = current_scenario_path(&first);
        let old_bytes = first.files[&old_path].clone();
        assert!(old_path.ends_with("/scenario-0001.json"));
        assert!(!old_path.contains("opaque"));
        set_revision(&mut draft.project, 2);
        let scenario = draft.scenarios.get_mut(&id).unwrap();
        scenario.title = "edited current input".into();
        scenario.revision = Revision(SafeUint::try_from(2).unwrap());
        scenario.project_revision = draft.project.revision.clone();
        let second = save_draft(&directory.path, &draft, Some(&first));
        let reopened = read_project_archive(&directory.path).unwrap();
        assert!(same(&reopened.scenarios[&id], &draft.scenarios[&id]));
        assert_eq!(second.manifest.generation.value(), 2);
        assert!(current_scenario_path(&second).starts_with("data/g000002/"));
        assert!(!second.files.contains_key(&old_path));
        assert_eq!(fs::read(directory.path.join(&old_path)).unwrap(), old_bytes);
        set_revision(&mut draft.project, 3);
        draft.project.scenario_refs.clear();
        draft.scenarios.clear();
        let deleted = save_draft(&directory.path, &draft, Some(&second));
        assert!(deleted.scenarios.is_empty());
        assert!(
            !deleted
                .manifest
                .documents
                .iter()
                .any(|e| matches!(e.file_ref.kind, DocumentKind::Scenario))
        );
        assert!(
            read_project_archive(&directory.path)
                .unwrap()
                .scenarios
                .is_empty()
        );
        assert_eq!(fs::read(directory.path.join(old_path)).unwrap(), old_bytes);
    }

    #[test]
    fn changed_current_orders_survive_another_directory_without_reusing_backing_orders() {
        let source = complete_archive();
        let original = TempDirectory::new();
        let first = save(&original.path, &source.project, Some(&source));
        let old_path = current_scenario_path(&first);
        let old_bytes = first.files[&old_path].clone();
        let mut draft = draft_for(&first.project, Some(&first));
        set_revision(&mut draft.project, 2);
        let id = ScenarioId::try_from("SC1".to_owned()).unwrap();
        let current = draft.scenarios.get_mut(&id).unwrap();
        current.revision = Revision(SafeUint::try_from(2).unwrap());
        current.project_revision = draft.project.revision.clone();
        current.orders = vec![serde_json::from_value(json!({
            "id":"current-order", "product_ref":{"id":"PRODUCT1","revision":"r1"},
            "quantity":{"value":{"coefficient":17,"decimal_places":0},"unit":"piece"},
            "release_at_ms":0,"due_at_ms":null,"destination":"S2", "acceptance_stage":"factory_released", "allow_partial":false,"priority":1
        })).unwrap()];
        let destination = TempDirectory::new();
        let saved = save_draft(&destination.path, &draft, Some(&first));
        let reopened = read_project_archive(&destination.path).unwrap();
        assert!(same(&reopened.scenarios[&id], &draft.scenarios[&id]));
        assert_eq!(reopened.scenarios[&id].orders.len(), 1);
        assert!(reopened.scenarios[&id].orders[0].due_at_ms.is_none());
        assert_eq!(
            serde_json::to_value(&reopened.scenarios[&id].orders[0]).unwrap()["quantity"]["value"]
                ["coefficient"],
            17
        );
        assert_eq!(saved.files[&old_path], old_bytes);
        assert_ne!(current_scenario_path(&saved), old_path);
        assert_eq!(saved.manifest.generation.value(), 2);
        let current_entry = saved
            .manifest
            .documents
            .iter()
            .find(|e| matches!(e.role, DocumentRole::ScenarioInput))
            .unwrap();
        assert_eq!(current_entry.revision.as_ref().unwrap().0.value(), 2);
        assert_eq!(saved.project.revision.0.value(), 2);
    }

    #[test]
    fn deleting_current_scenario_preserves_old_run_and_evaluation_revision_without_reactivating_it()
    {
        let source = complete_archive();
        let directory = TempDirectory::new();
        let first = save(&directory.path, &source.project, Some(&source));
        let path = current_scenario_path(&first);
        let old_bytes = first.files[&path].clone();
        let mut draft = ProjectDraft {
            project: first.project.clone(),
            scenarios: BTreeMap::new(),
            professional: crate::ProfessionalDocuments::default(),
        };
        draft.project.scenario_refs.clear();
        set_revision(&mut draft.project, 2);
        let saved = save_draft(&directory.path, &draft, Some(&first));
        let reopened = read_project_archive(&directory.path).unwrap();
        assert!(reopened.scenarios.is_empty());
        assert!(reopened.project.scenario_refs.is_empty());
        assert!(
            !saved
                .manifest
                .documents
                .iter()
                .any(|e| matches!(e.role, DocumentRole::ScenarioInput))
        );
        assert!(saved.manifest.documents.iter().any(|e| matches!(
            e.role,
            DocumentRole::SupportingFile
        ) && e.file_ref.path.as_str() == path
            && e.revision.as_ref().unwrap().0.value() == 1));
        assert_eq!(saved.files[&path], old_bytes);
        assert_eq!(saved.files["run.json"], first.files["run.json"]);
        assert_eq!(
            saved.files["evaluation.json"],
            first.files["evaluation.json"]
        );
        assert_eq!(
            saved.files["snapshots/scenario.json"],
            first.files["snapshots/scenario.json"]
        );
        assert!(saved.manifest.documents.iter().any(|e| matches!(
            e.role,
            DocumentRole::FrozenRunScenario
        ) && e.file_ref.path.as_str()
            == "snapshots/scenario.json"));
    }

    #[test]
    fn retained_scenario_roles_cannot_satisfy_a_missing_current_index() {
        let source = complete_archive();
        let directory = TempDirectory::new();
        let saved = save(&directory.path, &source.project, Some(&source));
        let mut root = saved.manifest.clone();
        for entry in &mut root.documents {
            if matches!(entry.role, DocumentRole::ScenarioInput) {
                entry.role = DocumentRole::SupportingFile;
            }
        }
        fs::write(directory.path.join("project.json"), encode(&root)).unwrap();
        error(
            read_project_archive(&directory.path),
            ErrorCode::MissingReference,
        );
        let mut draft = draft_for(&saved.project, Some(&saved));
        let id = draft.scenarios.keys().next().unwrap().clone();
        let value = draft.scenarios.remove(&id).unwrap();
        draft.scenarios.insert(
            ScenarioId::try_from("wrong-map-key".to_owned()).unwrap(),
            value,
        );
        let another = TempDirectory::new();
        error(
            super::write_project_archive(
                &another.path,
                &draft,
                Some(&saved),
                &AtomicBool::new(false),
            ),
            ErrorCode::InvalidInput,
        );
    }

    #[test]
    fn scenario_and_project_both_remain_old_when_the_real_root_replace_fails() {
        let directory = TempDirectory::new();
        let mut draft = ProjectDraft {
            project: project(),
            scenarios: BTreeMap::new(),
            professional: crate::ProfessionalDocuments::default(),
        };
        let scenario = empty_scenario(&draft.project, "atomic-scene");
        let id = scenario.id.clone();
        draft.project.scenario_refs.push(id.clone());
        draft.scenarios.insert(id.clone(), scenario);
        let first = save_draft(&directory.path, &draft, None);
        let root_bytes = fs::read(directory.path.join("project.json")).unwrap();
        let old_path = current_scenario_path(&first);
        set_revision(&mut draft.project, 2);
        let scenario = draft.scenarios.get_mut(&id).unwrap();
        scenario.title = "not committed".into();
        scenario.revision = Revision(SafeUint::try_from(2).unwrap());
        scenario.project_revision = draft.project.revision.clone();
        let held = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(directory.path.join("project.json"))
            .unwrap();
        let e = error(
            super::write_project_archive(
                &directory.path,
                &draft,
                Some(&first),
                &AtomicBool::new(false),
            ),
            ErrorCode::IoFailure,
        );
        assert!(e.message.contains("os error 32"), "{}", e.message);
        drop(held);
        let reopened = read_project_archive(&directory.path).unwrap();
        assert!(same(&reopened.project, &first.project));
        assert!(same(&reopened.scenarios[&id], &first.scenarios[&id]));
        assert_eq!(
            fs::read(directory.path.join("project.json")).unwrap(),
            root_bytes
        );
        assert_eq!(
            fs::read(directory.path.join(&old_path)).unwrap(),
            first.files[&old_path]
        );
    }

    fn located_missing_module(device: bool) -> (ProjectArchive, ProjectDraft, ModuleRef) {
        let source = complete_archive();
        let mut project = source.project.clone();
        set_revision(&mut project, 2);
        let reference = ModuleRef {
            id: ModuleId::try_from("not-supplied.module".to_owned()).unwrap(),
            version: VersionId::try_from("1.2.3".to_owned()).unwrap(),
        };
        let (code, object_ref, pointer) = if device {
            project.devices[0].module_ref = reference.clone();
            (
                "business_inputs.missing_device_module",
                ObjectRef {
                    kind: ObjectKind::Device,
                    id: project.devices[0].id.clone(),
                },
                "/module_ref",
            )
        } else {
            project.routes[0].steps[0].operation_ref = reference.clone();
            (
                "business_inputs.missing_module",
                ObjectRef {
                    kind: ObjectKind::Route,
                    id: project.routes[0].id.clone(),
                },
                "/steps/0/operation_ref",
            )
        };
        project.issues.push(Issue {
            code: code.into(),
            object_ref: Some(object_ref),
            field_path: Some(pointer.into()),
            // Deliberately does not encode a target ID: messages are never parsed.
            message: "declared input is not available".into(),
            blocks: vec![],
        });
        let draft = draft_for(&project, Some(&source));
        (source, draft, reference)
    }

    #[test]
    fn located_route_and_device_module_gaps_roundtrip_without_fabricating_required_files() {
        for device in [false, true] {
            let (source, draft, reference) = located_missing_module(device);
            let directory = TempDirectory::new();
            let saved = save_draft(&directory.path, &draft, Some(&source));
            let reopened = read_project_archive(&directory.path).unwrap();
            let actual = if device {
                &reopened.project.devices[0].module_ref
            } else {
                &reopened.project.routes[0].steps[0].operation_ref
            };
            assert!(same(actual, &reference));
            assert!(same(&reopened.project.issues, &draft.project.issues));
            assert!(
                !saved
                    .manifest
                    .module_refs
                    .iter()
                    .any(|r| same(r, &reference))
            );
            let mut root = saved.manifest.clone();
            root.module_refs.push(reference.clone());
            let mut required = entry(
                DocumentRole::ModuleDefinition,
                DocumentKind::Module,
                ObjectKind::Module,
                reference.id.as_str(),
                "modules/not-supplied.json",
                None,
            );
            required.module_ref = Some(reference);
            root.documents.push(required);
            fs::write(directory.path.join("project.json"), encode(&root)).unwrap();
            error(
                read_project_archive(&directory.path),
                ErrorCode::MissingReference,
            );
        }
    }

    #[test]
    fn located_module_gap_requires_the_exact_code_actual_owner_and_typed_pointer() {
        for device in [false, true] {
            for variant in ["code", "owner", "field", "index"] {
                let (source, mut draft, reference) = located_missing_module(device);
                let issue = draft.project.issues.last_mut().unwrap();
                match variant {
                    "code" => issue.code = "business_inputs.unrelated".into(),
                    "owner" => {
                        issue.object_ref.as_mut().unwrap().id =
                            ObjectId::try_from("absent-owner".to_owned()).unwrap()
                    }
                    "field" => issue.field_path = Some("/title".into()),
                    "index" => {
                        issue.field_path = Some(
                            if device {
                                "/module_ref/id"
                            } else {
                                "/steps/99/operation_ref"
                            }
                            .into(),
                        )
                    }
                    _ => unreachable!(),
                }
                assert!(!reported_missing(
                    &source.manifest,
                    &draft.project,
                    &ObjectKind::Module,
                    reference.id.as_str()
                ));
                let directory = TempDirectory::new();
                error(
                    super::write_project_archive(
                        &directory.path,
                        &draft,
                        Some(&source),
                        &AtomicBool::new(false),
                    ),
                    ErrorCode::MissingReference,
                );
                assert!(!directory.path.join("project.json").exists());
            }
            let (source, draft, _) = located_missing_module(device);
            assert!(!reported_missing(
                &source.manifest,
                &draft.project,
                &ObjectKind::Module,
                "different-target"
            ));
        }
    }

    fn stock_quantity(amount: u64) -> Value {
        json!({"value":{"coefficient":amount,"decimal_places":0},"unit":"piece"})
    }

    fn stock_draft() -> (ProjectArchive, ProjectDraft, ModuleRef) {
        let source = complete_archive();
        let mut project = source.project.clone();
        set_revision(&mut project, 2);
        let stock_type = ObjectId::try_from("STOCK".to_owned()).unwrap();
        project.item_types.push(ItemType {
            contents_layout: None,
            id: stock_type.clone(),
            revision: SubjectRevisionId::try_from("stock-r1".to_owned()).unwrap(),
            title: "declared batch material".into(),
            module_ref: None,
            unit: UnitId::try_from("piece".to_owned()).unwrap(),
            tracking: TrackingKind::Batch,
            spec_ref: None,
            attributes: BTreeMap::new(),
            attribute_schema: vec![],
        });
        for location in &mut project.locations {
            location.accepted_item_types.push(stock_type.clone());
        }
        let mut draft = draft_for(&project, Some(&source));
        let scenario = draft.scenarios.values_mut().next().unwrap();
        let item_type = json!({"id":"STOCK","revision":"stock-r1"});
        scenario.initial_conditions = serde_json::from_value(json!({
            "items":[{"id":"start-lot","item_type_ref":item_type,"quantity":stock_quantity(10),
                "holder":{"kind":"location","id":"S1"},"quality":"pending","components":[],"contents":[],"source":"user_provided"}],
            "initial_setup_families":{},"declared_quality_decisions":[],
            "material_allocations":[
                {"id":"hard-four","demand_ref":"TASK1","item_refs":["start-lot"],"quantity":stock_quantity(4),"mode":"hard_reserved"},
                {"id":"hard-six","demand_ref":"TASK1","item_refs":["start-lot"],"quantity":stock_quantity(6),"mode":"hard_reserved"},
                {"id":"future-plan","demand_ref":"O1","item_refs":["future-lot"],"quantity":stock_quantity(5),"mode":"soft_plan"}
            ]
        })).unwrap();
        scenario.arrivals = serde_json::from_value(json!([{
            "id":"arrival-five","at_ms":1000,"destination":"S1","release_policy":"unchecked_allowed",
            "items":[{"id":"future-lot","item_type_ref":item_type,"quantity":stock_quantity(5),
                "holder":{"kind":"location","id":"S1"},"quality":"unchecked","components":[],"contents":[],"source":"user_provided"}]
        }])).unwrap();
        let transport = ModuleRef {
            id: ModuleId::try_from("not-supplied.transport".to_owned()).unwrap(),
            version: VersionId::try_from("1.2.3".to_owned()).unwrap(),
        };
        scenario.supplies = serde_json::from_value(json!([{
            "id":"supply-twenty","source":"S1","destination":"S2","item_type_ref":item_type,
            "target_quantity":stock_quantity(20),"window":scenario.window,"transport_ref":transport,
            "resources":{},"departure_windows":[],"allow_partial_load":true
        }]))
        .unwrap();
        scenario.orders[0].quantity = Some(serde_json::from_value(stock_quantity(8)).unwrap());
        scenario.production_tasks[0].planned_quantity =
            serde_json::from_value(stock_quantity(8)).unwrap();
        scenario.production_tasks[0].order_allocations[0].quantity =
            serde_json::from_value(stock_quantity(8)).unwrap();
        let scenario_id = ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap();
        draft.project.issues.push(Issue {
            code: "business_inputs.material_missing_transport_module".into(),
            object_ref: Some(ObjectRef {
                kind: ObjectKind::Scenario,
                id: scenario_id,
            }),
            field_path: Some("/supplies/0/transport_ref".into()),
            message: "transport declaration has not been supplied".into(),
            blocks: vec![
                CapabilityPurpose::Execution,
                CapabilityPurpose::CompleteConclusion,
            ],
        });
        (source, draft, transport)
    }

    #[test]
    fn material_inputs_stay_separate_with_current_transport_gap_after_save_and_reopen() {
        let (source, draft, transport) = stock_draft();
        let directory = TempDirectory::new();
        let saved = save_draft(&directory.path, &draft, Some(&source));
        let another = TempDirectory::new();
        save_draft(&another.path, &draft, Some(&saved));
        let reopened = read_project_archive(&another.path).unwrap();
        assert!(same(&reopened.scenarios, &draft.scenarios));
        let scenario = reopened.scenarios.values().next().unwrap();
        let value = serde_json::to_value(scenario).unwrap();
        assert_eq!(
            value["initial_conditions"]["items"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            value["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"],
            10
        );
        assert_eq!(
            value["arrivals"][0]["items"][0]["quantity"]["value"]["coefficient"],
            5
        );
        assert_eq!(
            value["supplies"][0]["target_quantity"]["value"]["coefficient"],
            20
        );
        assert_eq!(value["orders"][0]["quantity"]["value"]["coefficient"], 8);
        assert_eq!(
            value["initial_conditions"]["items"][0]["quality"],
            "pending"
        );
        assert_eq!(
            value["initial_conditions"]["items"][0]["holder"],
            json!({"kind":"location","id":"S1"})
        );
        let allocations = &value["initial_conditions"]["material_allocations"];
        assert_eq!(allocations[0]["quantity"]["value"]["coefficient"], 4);
        assert_eq!(allocations[1]["quantity"]["value"]["coefficient"], 6);
        assert_eq!(allocations[0]["mode"], "hard_reserved");
        assert_eq!(allocations[1]["mode"], "hard_reserved");
        assert_eq!(allocations[2]["mode"], "soft_plan");
        assert_eq!(allocations[2]["item_refs"], json!(["future-lot"]));
        assert_ne!(value["initial_conditions"]["items"][0]["id"], "future-lot");
        assert!(same(&scenario.supplies[0].transport_ref, &transport));
        assert!(same(&reopened.project.issues, &draft.project.issues));
        assert!(
            !reopened
                .manifest
                .module_refs
                .iter()
                .any(|r| same(r, &transport))
        );
        assert_eq!(reopened.files["run.json"], source.files["run.json"]);
        assert_eq!(
            reopened.files["snapshots/scenario.json"],
            source.files["snapshots/scenario.json"]
        );
    }

    #[test]
    fn material_transport_issue_rejects_wrong_code_owner_and_pointer() {
        for variant in ["code", "missing-owner", "module-owner", "pointer", "target"] {
            let (source, mut draft, transport) = stock_draft();
            let issue = draft.project.issues.last_mut().unwrap();
            match variant {
                "code" => issue.code = "business_inputs.material_missing_transport_typo".into(),
                "missing-owner" => {
                    issue.object_ref.as_mut().unwrap().id =
                        ObjectId::try_from("absent-scenario".to_owned()).unwrap()
                }
                "module-owner" => {
                    issue.object_ref = Some(ObjectRef {
                        kind: ObjectKind::Module,
                        id: ObjectId::try_from(transport.id.as_str().to_owned()).unwrap(),
                    })
                }
                "pointer" => issue.field_path = Some("/supplies/99/transport_ref".into()),
                "target" => issue.field_path = Some("/supplies/0/item_type_ref".into()),
                _ => unreachable!(),
            }
            let directory = TempDirectory::new();
            error(
                super::write_project_archive(
                    &directory.path,
                    &draft,
                    Some(&source),
                    &AtomicBool::new(false),
                ),
                ErrorCode::MissingReference,
            );
            assert!(!directory.path.join("project.json").exists());
        }
    }

    #[test]
    fn material_transport_gap_does_not_cover_other_versions_in_current_or_retained_documents() {
        for variant in ["current-index", "frozen", "supporting"] {
            let (source, mut draft, transport) = stock_draft();
            let directory = TempDirectory::new();
            let mut saved = save_draft(&directory.path, &draft, Some(&source));
            let other = ModuleRef {
                id: transport.id.clone(),
                version: VersionId::try_from("9.9.9".to_owned()).unwrap(),
            };
            if variant == "current-index" {
                draft.project.module_refs.push(other);
            } else {
                let path = "snapshots/scenario.json";
                let mut frozen: ScenarioInput = decode(&saved.files[path], path).unwrap();
                frozen.supplies = draft.scenarios.values().next().unwrap().supplies.clone();
                frozen.supplies[0].transport_ref = other;
                saved.files.insert(path.into(), encode(&frozen));
                if variant == "supporting" {
                    saved
                        .manifest
                        .documents
                        .iter_mut()
                        .find(|e| e.file_ref.path.as_str() == path)
                        .unwrap()
                        .role = DocumentRole::SupportingFile;
                }
            }
            let another = TempDirectory::new();
            error(
                super::write_project_archive(
                    &another.path,
                    &draft,
                    Some(&saved),
                    &AtomicBool::new(false),
                ),
                ErrorCode::MissingReference,
            );
            assert!(!another.path.join("project.json").exists());
        }
    }

    #[test]
    fn material_transport_issue_does_not_excuse_a_missing_required_module_file() {
        let (source, draft, transport) = stock_draft();
        let directory = TempDirectory::new();
        let saved = save_draft(&directory.path, &draft, Some(&source));
        let mut manifest = saved.manifest;
        manifest.module_refs.push(transport.clone());
        let mut required = entry(
            DocumentRole::ModuleDefinition,
            DocumentKind::Module,
            ObjectKind::Module,
            transport.id.as_str(),
            "modules/missing-transport.json",
            None,
        );
        required.module_ref = Some(transport);
        manifest.documents.push(required);
        fs::write(directory.path.join("project.json"), encode(&manifest)).unwrap();
        error(
            read_project_archive(&directory.path),
            ErrorCode::MissingReference,
        );
    }

    #[test]
    fn material_transport_issue_never_resolves_its_owner_from_frozen_or_supporting_scenarios() {
        for supporting in [false, true] {
            let (mut source, draft, _) = stock_draft();
            let mut active = source.project.clone();
            active.scenario_refs.clear();
            active.issues = draft.project.issues.clone();
            let active_path = source
                .manifest
                .documents
                .iter()
                .find(|e| matches!(e.role, DocumentRole::ActiveProject))
                .unwrap()
                .file_ref
                .path
                .as_str()
                .to_owned();
            source.files.insert(active_path, encode(&active));
            for entry in &mut source.manifest.documents {
                if matches!(entry.role, DocumentRole::ScenarioInput) {
                    entry.role = DocumentRole::SupportingFile;
                }
            }
            let path = "snapshots/scenario.json";
            let mut frozen: ScenarioInput = decode(&source.files[path], path).unwrap();
            frozen.revision = Revision(SafeUint::try_from(9).unwrap());
            frozen.supplies = draft.scenarios.values().next().unwrap().supplies.clone();
            source.files.insert(path.into(), encode(&frozen));
            let entry = source
                .manifest
                .documents
                .iter_mut()
                .find(|e| e.file_ref.path.as_str() == path)
                .unwrap();
            entry.revision = Some(frozen.revision.clone());
            if supporting {
                entry.role = DocumentRole::SupportingFile;
            }
            let mut run: RunResult = decode(&source.files["run.json"], "run.json").unwrap();
            run.input_ref.scenario_revision = frozen.revision;
            source.files.insert("run.json".into(), encode(&run));
            let directory = TempDirectory::new();
            let opened = Directory::open(&directory.path).unwrap();
            for (path, bytes) in &source.files {
                opened.write_new(path, bytes).unwrap();
            }
            opened
                .write_new("project.json", &encode(&source.manifest))
                .unwrap();
            error(
                read_project_archive(&directory.path),
                ErrorCode::MissingReference,
            );
        }
    }
}

/// Frozen selected project package. Asset permission is supplied by the Host's
/// current trusted confirmation, never granted by incoming archive metadata.
pub(crate) fn prepare_handoff_archive(
    draft: &ProjectDraft,
    backing: Option<&ProjectArchive>,
    additions: &[ArchiveAddition],
    selection: &crate::PackageSelection,
    confirmed_assets: &BTreeSet<String>,
    cancelled: &AtomicBool,
) -> Result<ProjectArchive, ApiError> {
    check_cancelled(cancelled)?;
    if let Some(source) = backing {
        if source.project.id != draft.project.id {
            return Err(invalid(
                "project.json",
                "Backing archive belongs to a different project",
            ));
        }
        validate_archive(source.manifest.clone(), source.files.clone()).map_err(|mut error| {
            error.message = format!("Invalid backing archive: {}", error.message);
            error
        })?;
    }
    let mut selected = draft.clone();
    let standalone_module = matches!(
        selection.purpose,
        crate::PackagePurpose::PrivateModule | crate::PackagePurpose::CommunityModule
    );
    if standalone_module {
        if !selection.scenarios.is_empty()
            || !selection.run_ids.is_empty()
            || selection.module_refs.is_empty()
        {
            return Err(invalid(
                "package_scope",
                "Standalone module export requires explicit exact modules and excludes Project scenarios/Runs",
            ));
        }
        selected.project.item_types.clear();
        selected.project.locations.clear();
        selected.project.halls.clear();
        selected.project.lines.clear();
        selected.project.devices.clear();
        selected.project.resources.clear();
        selected.project.calendars.clear();
        selected.project.products.clear();
        selected.project.boms.clear();
        selected.project.routes.clear();
        selected.project.connections.clear();
        selected.project.setup_policies.clear();
        selected.project.acknowledgments.clear();
        selected.project.bindings.clear();
        selected.project.parameter_sources.clear();
        selected.project.template_adoptions.clear();
        selected.project.generation_context = None;
        selected.project.issues.clear();
        selected.project.module_refs = selection.module_refs.clone();
        selected.project.artifact_refs = selection.artifact_ids.clone();
        selected.project.views.selected_hall = None;
        selected.project.views.selected_objects.clear();
    }
    let mut scenarios = BTreeSet::new();
    for id in &selection.scenarios {
        if !scenarios.insert(id.clone()) {
            return Err(invalid(
                "package_scope/scenarios",
                "Duplicate selected scenario",
            ));
        }
        if !draft.scenarios.contains_key(id) {
            return Err(missing(
                "package_scope/scenarios",
                "Selected editable scenario is absent",
            ));
        }
    }
    selected
        .project
        .scenario_refs
        .retain(|id| scenarios.contains(id));
    selected.scenarios.retain(|id, _| scenarios.contains(id));
    let needed_artifacts: BTreeSet<_> = selection
        .artifact_ids
        .iter()
        .map(|id| id.as_str().to_owned())
        .chain(
            selected
                .project
                .bindings
                .iter()
                .map(|b| b.artifact_ref.as_str().to_owned()),
        )
        .chain(
            selected
                .project
                .products
                .iter()
                .flat_map(|p| p.artifact_refs.iter().map(|id| id.as_str().to_owned())),
        )
        .collect();
    selected
        .project
        .artifact_refs
        .retain(|id| needed_artifacts.contains(id.as_str()));
    for id in &selection.artifact_ids {
        if !selected.project.artifact_refs.contains(id) {
            selected.project.artifact_refs.push(id.clone());
        }
    }
    selected
        .project
        .acknowledgments
        .retain(|a| needed_artifacts.contains(a.artifact_ref.as_str()));
    // Imported diagnostic labels are not authority. Re-derive current gaps from
    // the actual exact definitions before serializing a portable editable draft.
    selected.project.issues.retain(|issue| {
        issue.code != "business_inputs.missing_exact_definition"
            && issue.code != "handoff.missing_exact_module"
    });
    let mut definitions = BTreeMap::new();
    if let Some(source) = backing {
        for module in source.modules()? {
            definitions.insert(
                module_key(&ModuleRef {
                    id: module.definition.id.clone(),
                    version: module.definition.version.clone(),
                }),
                module.definition,
            );
        }
    }
    for addition in additions {
        if matches!(addition.entry.file_ref.kind, DocumentKind::Module) {
            if let Document::Module(module) = parse_document(&addition.entry, &addition.bytes)? {
                definitions.insert(
                    module_key(&ModuleRef {
                        id: module.definition.id.clone(),
                        version: module.definition.version.clone(),
                    }),
                    module.definition,
                );
            }
        }
    }
    for (index, reference) in selected.project.module_refs.iter().enumerate() {
        if !definitions.contains_key(&module_key(reference)) {
            selected.project.issues.push(Issue{code:"business_inputs.missing_exact_definition".into(),object_ref:Some(ObjectRef{kind:ObjectKind::Module,id:ObjectId::try_from(reference.id.as_str().to_owned()).expect("module identity")}),field_path:Some(format!("/module_refs/{index}")),
                message:format!("Exact module {}@{} has no included definition; affected behavior stays unavailable",reference.id.as_str(),String::from(reference.version.clone())),
                blocks:vec![CapabilityPurpose::Execution,CapabilityPurpose::CompleteConclusion]});
        }
    }
    for issue in check_business_inputs_with_definitions(
        &selected.project,
        &selected.scenarios,
        &definitions.into_values().collect::<Vec<_>>(),
    ) {
        if !selected
            .project
            .issues
            .iter()
            .any(|existing| same(existing, &issue))
        {
            selected.project.issues.push(issue);
        }
    }
    let mut generation = backing.map_or(0, |a| a.manifest.generation.value());
    loop {
        generation = generation
            .checked_add(1)
            .filter(|g| *g <= MAX_SAFE_UINT)
            .ok_or_else(|| invalid("project.json", "Generation exhausted"))?;
        let prefix = format!("DATA/G{generation:06}/");
        if !backing.is_some_and(|a| a.files.keys().any(|p| folded(p).starts_with(&prefix))) {
            break;
        }
    }
    let full =
        assemble_archive(&selected, backing, additions, generation).map_err(|mut error| {
            error.message = format!("Selected handoff assembly: {}", error.message);
            error
        })?;
    let mut parsed = BTreeMap::new();
    for entry in &full.manifest.documents {
        if let Some(bytes) = full.files.get(entry.file_ref.path.as_str()) {
            parsed.insert(
                entry.file_ref.path.as_str().to_owned(),
                parse_document(entry, bytes)?,
            );
        }
    }
    let mut runs = BTreeSet::new();
    for id in &selection.run_ids {
        if !runs.insert(id.as_str()) {
            return Err(invalid("package_scope/run_ids", "Duplicate selected Run"));
        }
        if !parsed
            .values()
            .any(|d| matches!(d,Document::Run(r) if &r.id==id))
        {
            return Err(missing(
                "package_scope/run_ids",
                "Selected actual Run is absent",
            ));
        }
    }
    for id in &selection.artifact_ids {
        if !parsed
            .values()
            .any(|d| matches!(d,Document::Artifact(a) if &a.id==id))
        {
            return Err(missing(
                "package_scope/artifact_ids",
                "Selected professional artifact is absent",
            ));
        }
    }
    for reference in &selection.module_refs {
        if !full
            .manifest
            .documents
            .iter()
            .any(|e| e.module_ref.as_ref().is_some_and(|r| same(r, reference)))
        {
            return Err(missing(
                "package_scope/module_refs",
                "Selected exact module ID/version is absent",
            ));
        }
    }
    let mut keep = BTreeSet::new();
    for entry in &full.manifest.documents {
        if matches!(
            entry.role,
            DocumentRole::ActiveProject | DocumentRole::ScenarioInput
        ) || (!standalone_module && matches!(entry.file_ref.kind,DocumentKind::ExecutionArchive))
            || matches!(entry.object_ref.kind, ObjectKind::Run)
            && runs.contains(entry.object_ref.id.as_str())
            || matches!(entry.role, DocumentRole::ArtifactMetadata)
                && selection
                    .artifact_ids
                    .iter()
                    .any(|id| id.as_str() == entry.object_ref.id.as_str())
            || entry
                .module_ref
                .as_ref()
                .is_some_and(|r| selection.module_refs.iter().any(|s| same(r, s)))
        {
            keep.insert(entry.file_ref.path.as_str().to_owned());
        }
    }
    // Traverse file references, exact module references and semantic document
    // identities. Old frozen Project/Scenario documents remain exact old bytes.
    loop {
        check_cancelled(cancelled)?;
        let before = keep.len();
        for path in keep.clone() {
            let Some(document) = parsed.get(&path) else {
                continue;
            };
            if let Some(value) = document.reference_json() {
                let mut refs = vec![];
                file_references(&value, &mut refs);
                keep.extend(refs.into_iter().map(|(path, _)| path));
                let mut refs = vec![];
                module_references(&value, &mut refs);
                for entry in &full.manifest.documents {
                    if entry
                        .module_ref
                        .as_ref()
                        .is_some_and(|r| refs.iter().any(|m| same(m, r)))
                    {
                        keep.insert(entry.file_ref.path.as_str().into());
                    }
                }
                let mut artifacts = BTreeSet::new();
                artifact_references(&value, &mut artifacts);
                for entry in &full.manifest.documents {
                    if matches!(entry.role, DocumentRole::ArtifactMetadata)
                        && artifacts.contains(entry.object_ref.id.as_str())
                    {
                        keep.insert(entry.file_ref.path.as_str().into());
                    }
                }
            }
            if let Document::Project(project) = document {
                for entry in &full.manifest.documents {
                    if let Some(Document::Scenario(s)) = parsed.get(entry.file_ref.path.as_str()) {
                        if project.scenario_refs.contains(&s.id)
                            && same(&project.revision, &s.project_revision)
                        {
                            keep.insert(entry.file_ref.path.as_str().into());
                        }
                    }
                }
                for device in &project.devices {
                    if let (Some(TypedValue::String(id)), Some(TypedValue::Integer(revision))) = (
                        device.configuration.get("packaging_job_id"),
                        device.configuration.get("packaging_job_revision"),
                    ) {
                        for (job_path, document) in &parsed {
                            if matches!(document,Document::Job(job) if job.id.as_str()==id&&job.revision.0.value()==revision.value())
                            {
                                keep.insert(job_path.clone());
                            }
                        }
                    }
                }
            }
        }
        if keep.len() == before {
            break;
        }
    }
    let mut manifest = full.manifest.clone();
    manifest.missing.retain(|issue| {
        issue.code != "business_inputs.missing_exact_definition"
            && issue.code != "handoff.missing_exact_module"
            && (!standalone_module
                || issue.field_path.as_ref().is_some_and(|path| keep.contains(path))
                || issue.object_ref.as_ref().is_some_and(|object| {
                    full.manifest.documents.iter().any(|entry| {
                        keep.contains(entry.file_ref.path.as_str())
                            && !matches!(entry.role, DocumentRole::ActiveProject)
                            && same(&entry.object_ref, object)
                    })
                }))
    });
    for issue in selected
        .project
        .issues
        .iter()
        .chain(selected.scenarios.values().flat_map(|s| s.issues.iter()))
    {
        if !manifest
            .missing
            .iter()
            .any(|existing| same(existing, issue))
        {
            manifest.missing.push(issue.clone());
        }
    }
    manifest
        .documents
        .retain(|e| keep.contains(e.file_ref.path.as_str()));
    manifest.module_refs.retain(|r| {
        manifest
            .documents
            .iter()
            .any(|e| e.module_ref.as_ref().is_some_and(|m| same(m, r)))
    });
    let asset_ids: BTreeSet<_> = manifest
        .documents
        .iter()
        .filter(|e| matches!(e.object_ref.kind, ObjectKind::Asset))
        .map(|e| e.object_ref.id.as_str().to_owned())
        .collect();
    manifest
        .assets
        .retain(|r| asset_ids.contains(r.asset_ref.as_str()));
    manifest.restart_policy = RestartPolicy::RestartFromInitialInput;
    let mut files: BTreeMap<_, _> = full
        .files
        .into_iter()
        .filter(|(p, _)| keep.contains(p))
        .collect();
    let professional_paths: BTreeSet<_> = manifest
        .documents
        .iter()
        .filter(|e| {
            matches!(
                e.role,
                DocumentRole::ProfessionalInput
                    | DocumentRole::ProfessionalResult
                    | DocumentRole::ArtifactMetadata
            )
        })
        .filter_map(|e| parsed.get(e.file_ref.path.as_str()))
        .filter_map(Document::reference_json)
        .flat_map(|v| {
            let mut paths = vec![];
            file_references(&v, &mut paths);
            paths.into_iter().map(|(p, _)| p).collect::<Vec<_>>()
        })
        .collect();
    for entry in &mut manifest.documents {
        if !matches!(entry.object_ref.kind, ObjectKind::Asset) {
            continue;
        }
        let path = entry.file_ref.path.as_str();
        let right = manifest
            .assets
            .iter()
            .find(|r| r.asset_ref.as_str() == entry.object_ref.id.as_str());
        let allowed = selection.include_redistributable_assets
            && confirmed_assets.contains(entry.object_ref.id.as_str())
            && right.is_some_and(|r| {
                r.redistribution_confirmed
                    && r.source.as_ref().is_some_and(|s| !s.trim().is_empty())
                    && r.license.as_ref().is_some_and(|s| !s.trim().is_empty())
            });
        if !allowed {
            files.remove(path);
            entry.required = false;
            manifest.missing.push(Issue{code:"handoff.asset_not_redistributed".into(),object_ref:Some(entry.object_ref.clone()),field_path:Some(path.into()),
                message:"Asset bytes omitted: no current Host-confirmed redistribution permission; original source/rights retained for explicit replacement".into(),
                blocks:if professional_paths.contains(path){vec![CapabilityPurpose::Execution,CapabilityPurpose::CompleteConclusion]}else{vec![CapabilityPurpose::Preview]}});
        }
    }
    let instructions = format!(
        "# Editable project handoff\n\nProject: {}\nRevision: {}\n\nSelected scenarios: {:?}\nSelected actual Runs: {:?}\n\nOpen project.json or import this .sfproj through import_package, inspect missing references and explicitly adopt the draft. Use the embedded exact module versions; never replace them with machine latest. Edit supply in a new scenario revision and run from initial input with a new RunId. Included results belong to their own frozen inputs; they are not current stock. This package has no resumable Engine snapshot. A design-only package has no actual production quantity. Missing presentation may be replaced explicitly; missing behavior/professional state does not establish a complete result. Module/asset declarations do not grant execution or redistribution permission.\n",
        manifest.project_id.as_str(),
        manifest.project_revision.0.value(),
        selection
            .scenarios
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
        selection
            .run_ids
            .iter()
            .map(|r| r.as_str())
            .collect::<Vec<_>>()
    );
    let instruction_path = format!("data/g{generation:06}/HANDOFF.md");
    files.insert(instruction_path.clone(), instructions.into_bytes());
    manifest.documents.push(DocumentEntry {
        role: DocumentRole::SupportingFile,
        module_ref: None,
        object_ref: ObjectRef {
            kind: ObjectKind::Project,
            id: ObjectId::try_from(manifest.project_id.as_str().to_owned()).expect("project ID"),
        },
        revision: Some(manifest.project_revision.clone()),
        file_ref: FileRef {
            path: RelativeFilePath::try_from(instruction_path).expect("internal path"),
            kind: DocumentKind::Asset,
        },
        required: true,
    });
    check_cancelled(cancelled)?;
    validate_archive(manifest, files)
}

fn artifact_references(value: &Value, refs: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                if matches!(key.as_str(), "artifact_ref" | "artifact_id") {
                    if let Some(id) = v.as_str() {
                        refs.insert(id.to_owned());
                    }
                }
                if key == "artifact_refs" {
                    if let Some(ids) = v.as_array() {
                        refs.extend(ids.iter().filter_map(Value::as_str).map(str::to_owned));
                    }
                }
                artifact_references(v, refs);
            }
        }
        Value::Array(values) => {
            for v in values {
                artifact_references(v, refs);
            }
        }
        _ => {}
    }
}

/// Writes already-selected exact bytes; root replacement is the commit point.
pub(crate) fn write_package_directory(
    destination: &Path,
    archive: &ProjectArchive,
    cancelled: &AtomicBool,
) -> Result<(), ApiError> {
    validate_archive(archive.manifest.clone(), archive.files.clone())?;
    check_cancelled(cancelled)?;
    let directory = Directory::open(destination)?;
    let old = match directory.read("project.json") {
        Ok(_) => Some(read_root(&directory, "project.json")?),
        Err(e) if matches!(e.code, ErrorCode::MissingReference) => None,
        Err(e) => return Err(e),
    };
    for (path, bytes) in &archive.files {
        check_cancelled(cancelled)?;
        match directory.read(path) {
            Ok(existing) if existing == *bytes => {}
            Ok(_) => {
                return Err(failure(
                    ErrorCode::IoFailure,
                    "Immutable package path contains different bytes; choose another destination",
                    path,
                ));
            }
            Err(e) if matches!(e.code, ErrorCode::MissingReference) => {
                directory.write_new(path, bytes)?
            }
            Err(e) => return Err(e),
        }
    }
    let mut manifest = archive.manifest.clone();
    if let Some(old) = &old {
        manifest.generation = SafeUint::try_from(
            old.manifest
                .generation
                .value()
                .checked_add(1)
                .filter(|n| *n <= MAX_SAFE_UINT)
                .ok_or_else(|| invalid("project.json", "Generation exhausted"))?,
        )
        .expect("bounded generation");
    }
    let temporary = internal_name(&directory, "handoff-root")?;
    directory.write_new(&temporary, &encode(&manifest))?;
    check_cancelled(cancelled)?;
    if let Some(old) = &old {
        let retained = internal_name(&directory, "retained-root")?;
        if let Err(e) = replace_root(&directory, &temporary, &retained) {
            return Err(recover_old_root(&directory, old, &temporary, &retained, e));
        }
        let _ = fs::remove_file(directory.root.join(retained));
    } else {
        fs::rename(
            directory.root.join(&temporary),
            directory.root.join("project.json"),
        )
        .map_err(|e| io_failure("project.json", e))?;
    }
    Ok(())
}

/// Parse standard ZIP with the same typed, relative-path and exact-version
/// boundary as directory Open. No member is extracted or executed.
pub(crate) fn read_project_zip(
    source: &Path,
    cancelled: &AtomicBool,
) -> Result<ProjectArchive, ApiError> {
    check_cancelled(cancelled)?;
    let parent = source
        .parent()
        .ok_or_else(|| invalid("selected file", "ZIP requires an actual parent directory"))?;
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("selected file", "Windows file name required"))?;
    let directory = Directory::open(parent)?;
    let bytes = directory.read(name)?;
    decode_project_zip(&bytes, cancelled)
}
fn decode_project_zip(bytes: &[u8], cancelled: &AtomicBool) -> Result<ProjectArchive, ApiError> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| invalid("selected file", format!("Invalid ZIP: {e}")))?;
    let mut names = vec![];
    for n in 0..zip.len() {
        check_cancelled(cancelled)?;
        let file = zip.by_index(n).map_err(|e| invalid("ZIP", e.to_string()))?;
        let name = file
            .name()
            .map_err(|e| invalid("ZIP filename", e.to_string()))?
            .into_owned();
        if file
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err(invalid(&name, "ZIP symbolic links are not project files"));
        }
        if file.is_dir() {
            checked_path(name.trim_end_matches('/'))?;
        } else {
            names.push(name);
        }
    }
    check_paths(names.iter().map(String::as_str))?;
    let mut files = BTreeMap::new();
    for n in 0..zip.len() {
        check_cancelled(cancelled)?;
        let mut file = zip.by_index(n).map_err(|e| invalid("ZIP", e.to_string()))?;
        if file.is_dir() {
            continue;
        }
        let name = file
            .name()
            .map_err(|e| invalid("ZIP filename", e.to_string()))?
            .into_owned();
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)
            .map_err(|e| io_failure(&name, e))?;
        files.insert(name, bytes);
    }
    let root = files
        .remove("project.json")
        .ok_or_else(|| missing("project.json", "ZIP has no root manifest"))?;
    let manifest = decode(&root, "project.json")?;
    validate_archive(manifest, files)
}

pub(crate) fn write_project_zip(
    destination: &Path,
    archive: &ProjectArchive,
    cancelled: &AtomicBool,
) -> Result<(), ApiError> {
    validate_archive(archive.manifest.clone(), archive.files.clone())?;
    check_cancelled(cancelled)?;
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("selected file", "ZIP requires an actual parent directory"))?;
    let name = destination
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("selected file", "Windows file name required"))?;
    checked_path(name)?;
    let directory = Directory::open(parent)?;
    let old = match directory.read(name) {
        Ok(bytes) => Some(bytes),
        Err(e) if matches!(e.code, ErrorCode::MissingReference) => None,
        Err(e) => return Err(e),
    };
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::<u8>::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let root = encode(&archive.manifest);
    for (path, bytes) in std::iter::once(("project.json", &root))
        .chain(archive.files.iter().map(|(p, b)| (p.as_str(), b)))
    {
        check_cancelled(cancelled)?;
        zip.start_file(path, options)
            .map_err(|e| failure(ErrorCode::IoFailure, e.to_string(), path))?;
        zip.write_all(bytes).map_err(|e| io_failure(path, e))?;
    }
    let bytes = zip
        .finish()
        .map_err(|e| failure(ErrorCode::IoFailure, e.to_string(), "ZIP"))?
        .into_inner();
    let temporary = internal_name(&directory, "new-zip")?;
    directory.write_new(&temporary, &bytes)?;
    check_cancelled(cancelled)?;
    if let Some(old) = old {
        let retained = internal_name(&directory, "retained-zip")?;
        if let Err(error) = replace_named_file(&directory, &temporary, name, &retained) {
            let original_valid = directory.read(name).is_ok_and(|bytes| bytes == old);
            let restored = if original_valid {
                true
            } else if directory.read(&retained).is_ok_and(|bytes| bytes == old) {
                let restore = if directory.read(name).is_ok() {
                    let displaced = internal_name(&directory, "failed-zip")?;
                    replace_named_file(&directory, &retained, name, &displaced)
                } else {
                    fs::rename(directory.root.join(&retained), directory.root.join(name))
                };
                restore.is_ok() && directory.read(name).is_ok_and(|bytes| bytes == old)
            } else {
                false
            };
            return Err(failure(
                ErrorCode::IoFailure,
                format!(
                    "ZIP commit failed: {error}; previous bytes restored={restored}; retained candidate={retained}; new candidate={temporary}"
                ),
                name,
            ));
        }
        let _ = fs::remove_file(directory.root.join(retained));
    } else {
        fs::rename(directory.root.join(&temporary), directory.root.join(name))
            .map_err(|e| io_failure(name, e))?;
    }
    Ok(())
}
#[cfg(windows)]
fn replace_named_file(
    directory: &Directory,
    replacement: &str,
    target: &str,
    retained: &str,
) -> std::io::Result<()> {
    let wide = |name: &str| {
        directory
            .root
            .join(name)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let target = wide(target);
    let replacement = wide(replacement);
    let retained = wide(retained);
    let ok = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            replacement.as_ptr(),
            retained.as_ptr(),
            0,
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
#[cfg(not(windows))]
fn replace_named_file(
    directory: &Directory,
    replacement: &str,
    target: &str,
    retained: &str,
) -> std::io::Result<()> {
    fs::copy(directory.root.join(target), directory.root.join(retained))?;
    fs::rename(
        directory.root.join(replacement),
        directory.root.join(target),
    )
}

// Pure generation assembly shared by Save and selected handoff.
fn assemble_archive(
    draft: &ProjectDraft,
    backing: Option<&ProjectArchive>,
    additions: &[ArchiveAddition],
    generation: u64,
) -> Result<ProjectArchive, ApiError> {
    let retained = match backing {
        Some(source) => retained_paths(draft, source)?,
        None => BTreeSet::new(),
    };
    assemble_archive_with_retained(draft, backing, additions, generation, retained)
}

fn assemble_archive_with_retained(
    draft: &ProjectDraft,
    backing: Option<&ProjectArchive>,
    additions: &[ArchiveAddition],
    generation: u64,
    retained: BTreeSet<String>,
) -> Result<ProjectArchive, ApiError> {
    let project = &draft.project;
    let active_path = format!("data/g{generation:06}/project-state.json");
    let mut files = backing.map_or_else(BTreeMap::new, |a| {
        a.files
            .iter()
            .filter(|(path, _)| retained.contains(*path))
            .map(|(path, bytes)| (path.clone(), bytes.clone()))
            .collect()
    });
    let mut manifest = backing.map_or_else(
        || PackageManifest {
            project_id: project.id.clone(),
            project_revision: project.revision.clone(),
            title: project.title.clone(),
            generation: SafeUint::try_from(generation).expect("bounded generation"),
            documents: vec![],
            module_refs: vec![],
            assets: vec![],
            missing: vec![],
            restart_policy: RestartPolicy::RestartFromInitialInput,
        },
        |a| a.manifest.clone(),
    );
    manifest
        .documents
        .retain(|entry| retained.contains(entry.file_ref.path.as_str()));
    for entry in &mut manifest.documents {
        if matches!(
            entry.role,
            DocumentRole::ActiveProject | DocumentRole::ScenarioInput
        ) {
            // Retained exact old bytes are support, not an invented Run/current input.
            entry.role = DocumentRole::SupportingFile;
        }
    }
    files.insert(active_path.clone(), encode(project));
    manifest.project_id = project.id.clone();
    manifest.project_revision = project.revision.clone();
    manifest.title = project.title.clone();
    manifest.generation = SafeUint::try_from(generation).expect("bounded generation");
    manifest.documents.push(DocumentEntry {
        role: DocumentRole::ActiveProject,
        module_ref: None,
        object_ref: ObjectRef {
            kind: ObjectKind::Project,
            id: ObjectId::try_from(project.id.as_str().to_owned()).expect("existing opaque ID"),
        },
        revision: Some(project.revision.clone()),
        file_ref: FileRef {
            path: RelativeFilePath::try_from(active_path).expect("internal relative path"),
            kind: DocumentKind::Project,
        },
        required: true,
    });
    for (index, scenario) in draft.scenarios.values().enumerate() {
        let path = format!("data/g{generation:06}/scenario-{:04}.json", index + 1);
        files.insert(path.clone(), encode(scenario));
        manifest.documents.push(DocumentEntry {
            role: DocumentRole::ScenarioInput,
            module_ref: None,
            object_ref: ObjectRef {
                kind: ObjectKind::Scenario,
                id: ObjectId::try_from(scenario.id.as_str().to_owned())
                    .expect("existing opaque ID"),
            },
            revision: Some(scenario.revision.clone()),
            file_ref: FileRef {
                path: RelativeFilePath::try_from(path).expect("internal relative path"),
                kind: DocumentKind::Scenario,
            },
            required: true,
        });
    }
    let professional = professional_additions(draft, backing, generation)?;
    for addition in additions.iter().chain(&professional) {
        let path = addition.entry.file_ref.path.as_str();
        if let Some(existing) = files.get(path) {
            if existing != &addition.bytes
                || !manifest
                    .documents
                    .iter()
                    .any(|entry| same(entry, &addition.entry))
            {
                return Err(invalid(
                    path,
                    "immutable produced document conflicts with existing bytes or declaration",
                ));
            }
            continue;
        }
        if manifest
            .documents
            .iter()
            .any(|entry| folded(entry.file_ref.path.as_str()) == folded(path))
        {
            return Err(invalid(
                path,
                "produced document path conflicts with a declared package entry",
            ));
        }
        if let Some(reference) = &addition.entry.module_ref {
            if !manifest
                .module_refs
                .iter()
                .any(|existing| same(existing, reference))
            {
                manifest.module_refs.push(reference.clone());
            }
            if let Document::Module(package) = parse_document(&addition.entry, &addition.bytes)? {
                for right in package.rights {
                    if !manifest
                        .assets
                        .iter()
                        .any(|existing| existing.asset_ref == right.asset_ref)
                    {
                        manifest.assets.push(right);
                    }
                }
            }
        }
        files.insert(path.to_owned(), addition.bytes.clone());
        manifest.documents.push(addition.entry.clone());
    }
    // An explicitly accepted definition can retain an omitted asset reference.
    // Preserve its optional index and reported gap, without inventing file bytes
    // or weakening any existing required document entry.
    let mut omitted_assets = vec![];
    for addition in additions {
        if matches!(addition.entry.file_ref.kind, DocumentKind::Module) {
            if let Document::Module(package) = parse_document(&addition.entry, &addition.bytes)? {
                omitted_assets.extend(package.assets);
            }
        }
    }
    for asset in omitted_assets {
        if manifest.documents.iter().any(|entry| entry.file_ref.path == asset.path) {
            continue;
        }
        let Some(issue) = project.issues.iter().find(|issue|
            issue.field_path.as_deref() == Some(asset.path.as_str())
                && !issue.message.is_empty()
                && issue.object_ref.as_ref().is_some_and(|target|
                    matches!(target.kind, ObjectKind::Asset) && target.id.as_str() == asset.id.as_str()))
        else { continue; };
        manifest.documents.push(DocumentEntry {
            role: DocumentRole::Asset,
            module_ref: None,
            object_ref: issue.object_ref.clone().unwrap(),
            revision: None,
            file_ref: FileRef { path: asset.path, kind: DocumentKind::Asset },
            required: false,
        });
        if !manifest.missing.iter().any(|old| same(old, issue)) {
            manifest.missing.push(issue.clone());
        }
    }
    validate_archive(manifest, files)
}
#[cfg(test)]
mod optimization_reference_tests {
    use super::*;
    use serde_json::json;
    fn references(value:&Value)->(Vec<String>,Vec<String>,BTreeSet<String>){
        let mut files=vec![];file_references(value,&mut files);
        let mut files:Vec<_>=files.iter().map(|v|serde_json::to_string(v).unwrap()).collect();files.sort();
        let mut modules=vec![];module_references(value,&mut modules);
        let mut modules:Vec<_>=modules.iter().map(|v|serde_json::to_string(v).unwrap()).collect();modules.sort();
        let mut artifacts=BTreeSet::new();artifact_references(value,&mut artifacts);
        (files,modules,artifacts)
    }
    #[test]
    fn all_run_reference_branches_match_full_serialization(){
        let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut value:Value=serde_json::from_slice(&fs::read(root.join("tools/type-contract-proof/fixtures/run.json")).unwrap()).unwrap();
        let module=|id:&str|json!({"id":id,"version":"1.0.0"});
        value["data"]["jobs"]=json!([{"id":"j","task_ref":"t","operation_ref":module("job-operation"),"execution_ref":{"kind":"definition_cycle","data":{"module_ref":module("job-cycle")}},"input_refs":[],"job_resources":[],"phase_ref":null,"phase_resources":[],"effective_progress_ms":0,"started_at_ms":null,"finished_at_ms":null,"state":"ready","pending_outputs":[]}]);
        let wait=|id:&str|json!({"object_ref":{"kind":"device","id":"d"},"execution_ref":{"kind":"definition_cycle","data":{"module_ref":module(id)}},"phase_ref":null,"interval":{"start_ms":0,"end_ms":1},"reasons":[],"end_cause":"observation_end"});
        value["data"]["waits"]=json!([wait("direct-wait-cycle")]);
        value["data"]["events"]=json!([
            {"sequence":1,"at_ms":0,"subject":{"kind":"device","id":"d"},"payload":{"kind":"wait_changed","data":wait("event-wait-cycle")}},
            {"sequence":2,"at_ms":0,"subject":{"kind":"device","id":"d"},"payload":{"kind":"quality_changed","data":{"id":"q","item_ref":"i","inspection_ref":module("event-inspection"),"source_kind":"manual","observed_result":null,"disposition":"release","effective_at_ms":0,"decision_source":{"kind":"human_statement","data":"test"}}}}
        ]);
        let run:RunResult=serde_json::from_value(value["data"].clone()).unwrap();
        let full=references(&serde_json::to_value(&run).unwrap());
        let projected=references(&serde_json::to_value(run_references(&run)).unwrap());
        assert_eq!(full,projected);assert_eq!(full.0.len(),2);assert_eq!(full.1.len(),5);
    }
}
