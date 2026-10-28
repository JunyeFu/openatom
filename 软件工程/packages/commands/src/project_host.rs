//! Actual single-project Host state. Transport authentication remains the caller's responsibility.
use crate::project_history::ProjectHistory;
use crate::project_store::{
    ArchiveAddition, ProjectArchive, encode_document, read_project_archive,
    write_project_archive_with_additions,
};
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
#[path = "host_definitions.rs"]
mod host_definitions;
#[path = "host_item_layout.rs"]
mod host_item_layout;
#[path = "host_scene_node.rs"]
mod host_scene_node;
use host_definitions::DefinitionDraftState;
#[path = "host_authority.rs"]
mod host_authority;
#[path = "host_comparison.rs"]
mod host_comparison;
#[path = "host_manufacturing_export.rs"]
mod host_manufacturing_export;
pub use host_manufacturing_export::ManufacturingFileDeclaration;
#[path = "host_library.rs"]
mod host_library;
#[path = "host_module_import.rs"]
mod host_module_import;
#[path = "host_port_output.rs"]
mod host_port_output;
#[path = "host_run_history.rs"]
mod host_run_history;
pub use host_port_output::RunPortOutput;
#[path = "host_run_inputs.rs"]
mod host_run_inputs;
use host_library::{BundledModule, LibraryImports};
#[path = "host_evaluation.rs"]
mod host_evaluation;
use host_evaluation::EvaluationWork;
#[path = "host_definition_cycle.rs"]
mod host_definition_cycle;
#[path = "host_media_assets.rs"]
mod host_media_assets;
#[path = "host_stored_artifacts.rs"]
mod host_stored_artifacts;
#[path = "host_render.rs"]
mod host_render;
pub use host_render::{
    FrozenRenderAsset, FrozenRenderFile, FrozenRenderInput, RenderBackend, RenderComparisonInput, RenderFrame,
    RenderInputSource, RenderJob, RenderPackagingInput, RenderedCapture, RenderedImage,
};
#[path = "host_template_composition.rs"]
mod host_template_composition;
#[path = "host_factory_check.rs"]
mod host_factory_check;
#[path = "host_cloud_read.rs"]
mod host_cloud_read;
pub use host_cloud_read::CloudReadConnection;
#[path = "host_execution.rs"]
mod host_execution;
pub use host_execution::{ExecutionControlGrant, ExecutionGatewayLease};
#[path = "host_execution_preparation.rs"]
mod host_execution_preparation;
pub use host_execution_preparation::{ExecutionPreparationJob, ExecutionPreparationLease, ExecutionPreparationRuntime, FrozenExecutionFile};
#[path = "host_execution_exports.rs"]
mod host_execution_exports;

struct DirectorySelection {
    session: ObjectId,
    purpose: FilePurpose,
    project: Option<ProjectId>,
    directory: PathBuf,
    kind: SelectedPathKind,
}

#[derive(Clone, Copy)]
enum SelectedPathKind {
    Directory,
    File,
}

struct HostIoTask {
    owner: ObjectId,
    epoch: u64,
    state: TaskState,
    cancelled: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
    selection: Option<SelectedFileRef>,
    staged: Option<ProjectArchive>,
    saved_content_generation: Option<u64>,
    progress: Arc<Mutex<Option<TaskProgress>>>,
    packaging_input: Option<(String, ObjectId, u64)>,
}

enum IoInput {
    ExecutionExport(Box<host_execution_exports::ExecutionExportWork>),
    CloudRead(host_cloud_read::CloudReadWork),
    Render(Box<host_render::RenderWork>),
    Evaluation(Box<EvaluationWork>),
    Export {
        destination: PathBuf,
        kind: SelectedPathKind,
        selection: SelectedFileRef,
        package: FileRef,
        draft: ProjectDraft,
        backing: Option<ProjectArchive>,
        additions: Vec<ArchiveAddition>,
        package_scope: PackageSelection,
        confirmed_assets: BTreeSet<String>,
        execution_captures: Vec<crate::execution_archive::ExecutionArchiveCapture>,
    },
    Save {
        directory: PathBuf,
        draft: ProjectDraft,
        backing: Option<ProjectArchive>,
        additions: Vec<ArchiveAddition>,
        execution_captures: Vec<crate::execution_archive::ExecutionArchiveCapture>,
    },
    Open {
        directory: PathBuf,
    },
    Run {
        input: sf_simulation::EngineInput,
        supporting_scenarios: Vec<ScenarioInput>,
    },
    Packaging {
        job: PackageStructureJob,
        budget: CalculationBudget,
        context: sf_packaging::PackagingArtifactContext,
        input_bytes: Vec<u8>,
        searching: bool,
    },
}

enum WorkOutput {
    ExecutionExport(Box<host_execution_exports::PreparedExecutionExport>),
    CloudRead(Box<crate::execution_api::CloudReadResult>),
    Render(Box<RenderedCapture>),
    Evaluation(Box<ExportEvaluation>),
    Archive(ProjectArchive),
    ExportPackage(ExportPackage),
    Run {
        engine: sf_simulation::Engine,
        input: sf_simulation::EngineInput,
        supporting_scenarios: Vec<ScenarioInput>,
    },
    Packaging {
        product: sf_packaging::PackagingProduct,
        input_bytes: Vec<u8>,
    },
}

#[derive(Clone)]
struct CanonicalPackagingInput {
    job: PackageStructureJob,
    file: FileRef,
    bytes: Vec<u8>,
    pending_search: bool,
}

/// Current-process evidence of a binding actually committed here. Imported
/// metadata never creates this receipt or current producer/UI authority.
#[derive(Clone)]
struct CommittedNominalBinding {
    project_id: ProjectId,
    binding: ConsumerBinding,
    artifact: ArtifactRecord,
    result: PackagingResult,
    job: PackageStructureJob,
    acknowledgment: ArtifactAcknowledgment,
    files: BTreeMap<String, Vec<u8>>,
    modules: Vec<ModulePackage>,
    product: ProductRevision,
}

struct HostRun {
    result: RunResult,
    engine: Option<sf_simulation::Engine>,
    history: Option<StoredRunHistory>,
    scope: EditScope,
    observation_cache: Mutex<host_run_history::ActualObjectCache>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRunHistory {
    run_id: RunId,
    input_ref: RunInputRef,
    scope: EditScope,
    records: Vec<(SimTimeMs, RuntimeItem)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    job_records: Option<Vec<(SimTimeMs, JobState)>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    port_outputs: Option<Vec<sf_simulation::PortOutputRecord>>,
}

/// Private native work item: contains no serializable paths or caller authority.
pub struct ProjectIoWork {
    task_id: TaskId,
    cancelled: Arc<AtomicBool>,
    started: Arc<AtomicBool>,
    input: IoInput,
    progress: Arc<Mutex<Option<TaskProgress>>>,
}

/// Only actual execution of a work item constructs its completion.
pub struct ProjectIoCompletion {
    task_id: TaskId,
    result: Result<WorkOutput, ApiError>,
}

impl ProjectIoWork {
    pub fn fail(self, failure: ApiError) -> ProjectIoCompletion {
        ProjectIoCompletion {
            task_id: self.task_id,
            result: Err(failure),
        }
    }
    pub fn run(self) -> ProjectIoCompletion {
        self.started.store(true, Ordering::Release);
        let result = if self.cancelled.load(Ordering::Acquire) {
            Err(error(
                ErrorCode::Cancelled,
                "File task cancelled before execution",
                None,
            ))
        } else {
            match self.input {
                IoInput::ExecutionExport(work) => work.run(&self.cancelled, &self.progress).map(|result|WorkOutput::ExecutionExport(Box::new(result))),
                IoInput::CloudRead(work) => work.run(&self.cancelled).map(|result|WorkOutput::CloudRead(Box::new(result))),
                IoInput::Render(work) => work
                    .run(&self.cancelled)
                    .map(|capture| WorkOutput::Render(Box::new(capture))),
                IoInput::Evaluation(work) => {
                    let mut publish = |value| {
                        *self.progress.lock().expect("worker progress lock") = Some(value);
                    };
                    work.run(&self.cancelled, &mut publish)
                        .map(|output| WorkOutput::Evaluation(Box::new(output)))
                }
                IoInput::Export {
                    destination,
                    kind,
                    selection,
                    package,
                    draft,
                    backing,
                    mut additions,
                    package_scope,
                    confirmed_assets,
                    execution_captures,
                } => (|| {
                    for capture in execution_captures { additions.extend(capture.additions()?); }
                    *self.progress.lock().expect("worker progress lock") = Some(TaskProgress {
                        phase: "整理工程与冻结结果".into(), processed: SafeUint::try_from(0_u64).unwrap(), total: Some(SafeUint::try_from(2_u64).unwrap()),
                        message: Some("计数为已完成实际阶段，不代表耗时百分比。".into()),
                    });
                    crate::project_store::prepare_handoff_archive(
                    &draft,
                    backing.as_ref(),
                    &additions,
                    &package_scope,
                    &confirmed_assets,
                    &self.cancelled,
                )
                .and_then(|archive| {
                    *self.progress.lock().expect("worker progress lock") = Some(TaskProgress {
                        phase: "写入可编辑工程包".into(), processed: SafeUint::try_from(1_u64).unwrap(), total: Some(SafeUint::try_from(2_u64).unwrap()),
                        message: Some("工程与冻结结果已整理；等待原子提交。".into()),
                    });
                    match kind {
                        SelectedPathKind::Directory => {
                            crate::project_store::write_package_directory(
                                &destination,
                                &archive,
                                &self.cancelled,
                            )
                        }
                        SelectedPathKind::File => crate::project_store::write_project_zip(
                            &destination,
                            &archive,
                            &self.cancelled,
                        ),
                    }?;
                    let included_files =
                        std::iter::once(file_ref("project.json", DocumentKind::Project))
                            .chain(archive.files.keys().map(|path| {
                                archive
                                    .manifest
                                    .documents
                                    .iter()
                                    .find(|entry| entry.file_ref.path.as_str() == path)
                                    .map(|entry| entry.file_ref.clone())
                                    .unwrap_or_else(|| file_ref(path, DocumentKind::Asset))
                            }))
                            .collect();
                    *self.progress.lock().expect("worker progress lock") = Some(TaskProgress {
                        phase: "工程包已提交".into(), processed: SafeUint::try_from(2_u64).unwrap(), total: Some(SafeUint::try_from(2_u64).unwrap()),
                        message: Some("等待宿主接收真实任务终态。".into()),
                    });
                    Ok(WorkOutput::ExportPackage(ExportPackage {
                        destination: selection,
                        package,
                        included_files,
                        missing: archive.manifest.missing.clone(),
                    }))
                })
                })(),
                IoInput::Save {
                    directory,
                    draft,
                    backing,
                    mut additions,
                    execution_captures,
                } => (|| {
                    for capture in execution_captures { additions.extend(capture.additions()?); }
                    write_project_archive_with_additions(&directory,&draft,backing.as_ref(),&additions,&self.cancelled)
                        .map(WorkOutput::Archive)
                })(),
                IoInput::Open { directory } => {
                    let archive = if directory.is_dir() {
                        read_project_archive(&directory)
                    } else {
                        crate::project_store::read_project_zip(&directory, &self.cancelled)
                    };
                    archive.and_then(|archive| {
                        if self.cancelled.load(Ordering::Acquire) {
                            Err(error(
                                ErrorCode::Cancelled,
                                "Open task cancelled before staging",
                                None,
                            ))
                        } else {
                            Ok(WorkOutput::Archive(archive))
                        }
                    })
                }
                IoInput::Run {
                    input,
                    supporting_scenarios,
                } => sf_simulation::execute_engine(input.clone(), &self.cancelled)
                    .map(|engine| WorkOutput::Run {
                        engine,
                        input,
                        supporting_scenarios,
                    })
                    .map_err(engine_failure),
                IoInput::Packaging {
                    job,
                    budget,
                    context,
                    input_bytes,
                    searching,
                } => {
                    let mut progress = |actual: sf_packaging::PackagingProgress| {
                        *self.progress.lock().expect("worker progress lock") = Some(TaskProgress {
                            phase: actual.phase,
                            processed: actual.budget_used.interval_queries.clone(),
                            total: None,
                            message: Some(format!(
                                "candidates={}; interval_queries={}/{}; subdivisions={}/{}; stopped_by={:?}",
                                actual.budget_used.candidates_evaluated.value(),
                                actual.budget_used.interval_queries.value(),
                                budget.interval_query_limit.value(),
                                actual.budget_used.subdivisions.value(),
                                budget.subdivision_limit.value(),
                                actual.budget_used.stopped_by
                            )),
                        });
                    };
                    sf_packaging::compute_packaging(
                        &job,
                        &budget,
                        context,
                        &self.cancelled,
                        &mut progress,
                    )
                    .map(|product| {
                        let input_bytes = if searching {
                            encode_document(&product.selected_job)
                        } else {
                            input_bytes
                        };
                        WorkOutput::Packaging {
                            product,
                            input_bytes,
                        }
                    })
                    .map_err(packaging_failure)
                }
            }
        };
        ProjectIoCompletion {
            task_id: self.task_id,
            result,
        }
    }
}

pub struct ProjectHost {
    host_instance_id: ObjectId,
    sessions: BTreeMap<ObjectId, HostCallContext>,
    revoked_external_sessions: BTreeSet<ObjectId>,
    project: Option<ProjectState>,
    scenarios: BTreeMap<ScenarioId, ScenarioInput>,
    scenario_highwater: BTreeMap<ScenarioId, Revision>,
    dirty: bool,
    desktop_closing: bool,
    next_project: u64,
    next_file_token: u64,
    next_task: u64,
    epoch: u64,
    selections: BTreeMap<ObjectId, DirectorySelection>,
    tasks: BTreeMap<TaskId, HostIoTask>,
    cloud_connections: BTreeMap<ProjectId,Arc<CloudReadConnection>>,
    cloud_read_requests: BTreeMap<(ObjectId,ObjectId),host_cloud_read::RegisteredCloudRead>,
    original_execution_requests: BTreeMap<(ObjectId,ObjectId),host_execution_exports::OriginalExecutionRequest>,
    executions: BTreeMap<ExecutionSessionId, host_execution::HostedExecution>,
    execution_preparation: host_execution_preparation::PreparationRegistry,
    external_task_access: BTreeMap<(ObjectId, TaskId), bool>,
    io_work: Vec<ProjectIoWork>,
    backing: Option<ProjectArchive>,
    history: ProjectHistory,
    modules: BTreeMap<(String, String), ModulePackage>,
    bundled_modules: BTreeMap<(String, String), BundledModule>,
    runs: BTreeMap<RunId, HostRun>,
    definition_cycle_inputs: BTreeMap<RunId, host_definition_cycle::DefinitionCycleInput>,
    additions: Vec<ArchiveAddition>,
    content_generation: u64,
    professional: ProfessionalDocuments,
    professional_trust: ProfessionalTrust,
    providers: Vec<ProviderCapability>,
    definition_drafts: BTreeMap<ModuleId, DefinitionDraftState>,
    next_definition_draft: u64,
    next_document: u64,
    packaging_inputs: BTreeMap<(String, ObjectId, u64), CanonicalPackagingInput>,
    committed_nominal_bindings: Vec<CommittedNominalBinding>,
    asset_confirmations: BTreeMap<String, (Value, BTreeMap<String, Vec<u8>>)>,
    evaluation_resources: Arc<sf_reporting::EvaluationResources>,
    render_state: host_render::RenderState,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionCloseBlocker {
    pub session_id: ExecutionSessionId,
    pub execution_epoch_id: ExecutionEpochId,
    pub execution_revision: SafeUint,
    pub reason: String,
}

#[derive(Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(deny_unknown_fields)]
pub struct DesktopCloseState {
    #[ts(type = "string | null")]
    pub project_id: Option<ProjectId>,
    #[ts(type = "number | null")]
    pub project_revision: Option<Revision>,
    pub unsaved: bool,
    #[ts(type = "Array<[string, number]>")]
    pub definition_revisions: Vec<(ModuleId, Revision)>,
    #[ts(type = "Array<string>")]
    pub active_tasks: Vec<TaskId>,
    #[ts(type = "Array<{ session_id: string, execution_epoch_id: string, execution_revision: number, reason: string }>")]
    pub execution_blockers: Vec<ExecutionCloseBlocker>,
}

impl ProjectHost {
    /// Private desktop lifecycle state, not a new public business operation.
    /// Scope-filtered get_context intentionally cannot answer this question.
    pub fn desktop_close_state(&self) -> DesktopCloseState {
        DesktopCloseState {
            project_id: self.project.as_ref().map(|project| project.id.clone()),
            project_revision: self.project.as_ref().map(|project| project.revision.clone()),
            unsaved: self.dirty || self.definition_drafts.values().any(|draft| draft.dirty),
            definition_revisions: self.definition_drafts.iter().map(|(id, draft)| (id.clone(), draft.revision.clone())).collect(),
            active_tasks: self.tasks.values().filter_map(|task| {
                let state = current_task_state(task).task;
                (!terminal(&state.state)).then_some(state.id)
            }).collect(),
            execution_blockers: self.execution_blockers(),
        }
    }

    /// The native adapter holds the same Host lock as UI/Agent admission while
    /// checking the accepted snapshot and destroying the actual main window.
    pub fn finish_desktop_close(
        &mut self,
        expected: &DesktopCloseState,
        discard_confirmed: bool,
        destroy: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), ApiError> {
        if self.desktop_closing {
            return Err(error(ErrorCode::InvalidInput, "Desktop is already closing", None));
        }
        let actual = self.desktop_close_state();
        if !exact_data(&actual, expected) {
            return Err(error(ErrorCode::RevisionConflict, "关闭期间工程、草稿或任务已变化；当前内容保留，请重新核对。", None));
        }
        if !actual.active_tasks.is_empty() || !actual.execution_blockers.is_empty() || (actual.unsaved && !discard_confirmed) {
            return Err(error(ErrorCode::InvalidInput, "仍有活动任务、未结清生产或未经确认的未保存内容；窗口保留。", None));
        }
        self.desktop_closing = true;
        if let Err(message) = destroy() {
            self.desktop_closing = false;
            return Err(error(ErrorCode::ComputationFailed, &message, None));
        }
        Ok(())
    }

    pub fn new(host_instance_id: ObjectId, ui_session_id: ObjectId) -> Self {
        let caller = HostCallContext {
            origin: CallerOrigin::InteractiveUi,
            allow_view_control: true,
            session_id: ui_session_id.clone(),
            granted_project: None,
            allowed_operations: frozen_operations(),
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        };
        Self {
            host_instance_id,
            sessions: BTreeMap::from([(ui_session_id, caller)]),
            revoked_external_sessions: BTreeSet::new(),
            project: None,
            scenarios: BTreeMap::new(),
            scenario_highwater: BTreeMap::new(),
            dirty: false,
            desktop_closing: false,
            next_project: 1,
            next_file_token: 1,
            next_task: 1,
            epoch: 0,
            selections: BTreeMap::new(),
            tasks: BTreeMap::new(),
            cloud_connections: BTreeMap::new(),
            cloud_read_requests: BTreeMap::new(),
            original_execution_requests: BTreeMap::new(),
            executions: BTreeMap::new(),
            execution_preparation: host_execution_preparation::PreparationRegistry::default(),
            external_task_access: BTreeMap::new(),
            io_work: vec![],
            backing: None,
            history: ProjectHistory::new(),
            modules: BTreeMap::new(),
            bundled_modules: BTreeMap::new(),
            runs: BTreeMap::new(),
            definition_cycle_inputs: BTreeMap::new(),
            additions: vec![],
            content_generation: 0,
            professional: ProfessionalDocuments::default(),
            professional_trust: ProfessionalTrust::default(),
            providers: vec![],
            definition_drafts: BTreeMap::new(),
            next_definition_draft: 1,
            next_document: 1,
            packaging_inputs: BTreeMap::new(),
            committed_nominal_bindings: vec![],
            asset_confirmations: BTreeMap::new(),
            evaluation_resources: Arc::new(sf_reporting::EvaluationResources::default()),
            render_state: host_render::RenderState::default(),
        }
    }

    /// Called by trusted Host administration, never deserialized from a request.
    pub fn register_external_session(&mut self, caller: HostCallContext) -> Result<(), ApiError> {
        if !matches!(caller.origin, CallerOrigin::ExternalAgent)
            || self.sessions.contains_key(&caller.session_id)
            || self.revoked_external_sessions.contains(&caller.session_id)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Session cannot be registered or replaced",
                None,
            ));
        }
        self.sessions.insert(caller.session_id.clone(), caller);
        Ok(())
    }

    pub fn caller_context(&self, session_id: &ObjectId) -> Option<&HostCallContext> {
        self.sessions.get(session_id)
    }

    fn current_draft(&self) -> ProjectDraft {
        ProjectDraft {
            project: self
                .project
                .as_ref()
                .expect("admitted active project")
                .clone(),
            scenarios: self.scenarios.clone(),
            professional: self.professional.clone(),
        }
    }

    /// Trusted native picker admission; a wire path is never accepted as authority.
    pub fn select_directory(
        &mut self,
        session: &ObjectId,
        purpose: FilePurpose,
        directory: PathBuf,
    ) -> Result<SelectedFileRef, ApiError> {
        self.select_path(session, purpose, directory, SelectedPathKind::Directory)
    }

    /// Trusted native open/save-file picker; the selected file path is private.
    pub fn select_file(
        &mut self,
        session: &ObjectId,
        purpose: FilePurpose,
        file: PathBuf,
    ) -> Result<SelectedFileRef, ApiError> {
        self.select_path(session, purpose, file, SelectedPathKind::File)
    }

    fn select_path(
        &mut self,
        session: &ObjectId,
        purpose: FilePurpose,
        path: PathBuf,
        kind: SelectedPathKind,
    ) -> Result<SelectedFileRef, ApiError> {
        let caller = self.sessions.get(session).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        let operation = match purpose {
            FilePurpose::SaveProject => OperationName::SaveProject,
            FilePurpose::ImportPackage => OperationName::ImportPackage,
            FilePurpose::ExportPackage => OperationName::ExportPackage,
            FilePurpose::ExportEvaluation => OperationName::ExportEvaluation,
            FilePurpose::ExportExecutionReport => OperationName::ExportExecutionReport,
            FilePurpose::ExportCloudData => OperationName::ExportCloudData,
        };
        if !caller
            .allowed_operations
            .iter()
            .any(|op| same(op, &operation))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "File purpose is outside the session operations",
                None,
            ));
        }
        let project = self.project.as_ref().map(|p| p.id.clone());
        if matches!(
            purpose,
            FilePurpose::SaveProject | FilePurpose::ExportPackage | FilePurpose::ExportEvaluation | FilePurpose::ExportExecutionReport | FilePurpose::ExportCloudData
        ) && project.is_none()
        {
            return Err(error(
                ErrorCode::NoActiveProject,
                "Saving/exporting requires an active project",
                None,
            ));
        }
        if matches!(caller.origin, CallerOrigin::ExternalAgent)
            && project.is_some()
            && caller.granted_project.as_ref() != project.as_ref()
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Directory selection is outside the session project",
                None,
            ));
        }
        if matches!(kind, SelectedPathKind::File)
            && matches!(
                purpose,
                FilePurpose::SaveProject | FilePurpose::ExportEvaluation | FilePurpose::ExportExecutionReport | FilePurpose::ExportCloudData
            )
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "SaveProject and ExportEvaluation require a directory selection",
                None,
            ));
        }
        let directory = if matches!(kind, SelectedPathKind::File)
            && matches!(purpose, FilePurpose::ExportPackage)
            && !path.exists()
        {
            let name = path.file_name().ok_or_else(|| {
                error(
                    ErrorCode::InvalidInput,
                    "Export file requires an explicit file name",
                    None,
                )
            })?;
            let parent = path.parent().ok_or_else(|| {
                error(
                    ErrorCode::InvalidInput,
                    "Export file requires its selected parent directory",
                    None,
                )
            })?;
            std::fs::canonicalize(parent).map(|parent| parent.join(name))
        } else {
            std::fs::canonicalize(path)
        }
        .map_err(|_| {
            error(
                ErrorCode::IoFailure,
                "Selected path cannot be resolved",
                None,
            )
        })?;
        if (matches!(kind, SelectedPathKind::Directory) && !directory.is_dir())
            || (matches!(kind, SelectedPathKind::File)
                && directory.exists()
                && !directory.is_file())
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Selected path does not match the native picker kind",
                None,
            ));
        }
        let token = ObjectId::try_from(format!(
            "{}:file:{}",
            self.host_instance_id.as_str(),
            self.next_file_token
        ))
        .unwrap();
        self.next_file_token += 1;
        self.selections.insert(
            token.clone(),
            DirectorySelection {
                session: session.clone(),
                purpose: purpose.clone(),
                project,
                directory,
                kind,
            },
        );
        self.sessions
            .get_mut(session)
            .unwrap()
            .selected_file_tokens
            .push(token.clone());
        Ok(SelectedFileRef {
            selection_id: token,
            purpose,
        })
    }

    pub fn drain_io_work(&mut self) -> Vec<ProjectIoWork> {
        std::mem::take(&mut self.io_work)
    }

    pub fn finish_io(&mut self, completion: ProjectIoCompletion) {
        let completion = match completion.result {
            Ok(WorkOutput::ExecutionExport(prepared)) => {
                self.finish_execution_export(completion.task_id, *prepared);
                return;
            }
            Ok(WorkOutput::Render(capture)) => {
                self.finish_render(completion.task_id, *capture);
                return;
            }
            Ok(WorkOutput::Run { input, engine, .. })
                if self.tasks.get(&completion.task_id).is_some_and(|task| {
                    matches!(
                        task.state.task.scope,
                        TaskScope::Edit(EditScope::DefinitionDraft(_))
                    )
                }) =>
            {
                self.finish_definition_cycle(completion.task_id, input, engine);
                return;
            }
            Ok(WorkOutput::Packaging {
                product,
                input_bytes,
            }) => {
                self.finish_packaging(completion.task_id, product, input_bytes);
                return;
            }
            result => ProjectIoCompletion {
                task_id: completion.task_id,
                result,
            },
        };
        let failed = completion.result.is_err();
        let Some(task) = self.tasks.get_mut(&completion.task_id) else {
            return;
        };
        if terminal(&task.state.task.state) {
            return;
        }
        match completion.result {
            Ok(WorkOutput::CloudRead(result)) => {
                if task.cancelled.load(Ordering::Acquire) {
                    task.state.task.state=ComputationStatus::Cancelled;
                } else {
                    task.state.result=Some(TaskOutput::CloudRead(result));
                    task.state.task.state=ComputationStatus::Completed;
                }
                task.state.allowed_controls.clear();
            }
            Ok(WorkOutput::Archive(archive)) => {
                match task.state.originating_operation {
                    OperationName::SaveProject => {
                        let is_current = task.epoch == self.epoch
                            && self
                                .project
                                .as_ref()
                                .is_some_and(|p| p.id == archive.project.id);
                        let current_revision = if is_current {
                            self.project.as_ref().unwrap().revision.clone()
                        } else {
                            archive.project.revision.clone()
                        };
                        let still_dirty = !is_current
                            || current_revision.0.value() != archive.project.revision.0.value()
                            || task.saved_content_generation != Some(self.content_generation);
                        task.state.result = Some(TaskOutput::Save(SavedProject {
                            project_id: archive.project.id.clone(),
                            saved_revision: archive.project.revision.clone(),
                            current_revision,
                            still_dirty,
                            destination: task.selection.clone().expect("save selection"),
                            manifest: FileRef {
                                path: RelativeFilePath::try_from("project.json".to_owned())
                                    .unwrap(),
                                kind: DocumentKind::Project,
                            },
                        }));
                        if is_current {
                            self.dirty = still_dirty;
                            self.backing = Some(archive);
                        }
                    }
                    OperationName::ImportPackage => {
                        if matches!(task.state.task.scope, TaskScope::Edit(EditScope::Project(_))) {
                            task.state.result = Some(TaskOutput::Import(host_module_import::proposal(
                                &archive, &self.modules, task.selection.clone().expect("import selection"),
                            )));
                            task.staged = Some(archive);
                            task.state.task.state = ComputationStatus::Completed;
                            task.state.allowed_controls.clear();
                            return;
                        }
                        let target = ObjectRef {
                            kind: ObjectKind::Project,
                            id: ObjectId::try_from(archive.project.id.as_str().to_owned()).unwrap(),
                        };
                        let mut draft = ProjectDraft {
                            project: archive.project.clone(),
                            scenarios: archive.scenarios.clone(),
                            professional: archive.professional.clone(),
                        };
                        let definitions: Vec<_> = archive
                            .modules()
                            .expect("read archive validated modules")
                            .into_iter()
                            .map(|package| package.definition)
                            .collect();
                        refresh_business_issues_with_definitions(&mut draft, &definitions);
                        let mut objects = vec![target];
                        objects.extend(draft_objects(&draft).into_iter().map(|(target, _)| target));
                        let mut proposed_data =
                            vec![DomainObject::Project(Box::new(draft.project.clone()))];
                        proposed_data.extend(
                            archive
                                .scenarios
                                .values()
                                .cloned()
                                .map(|scenario| DomainObject::Scenario(Box::new(scenario))),
                        );
                        task.state.result = Some(TaskOutput::Import(ImportDraft {
                            input_selection: task.selection.clone().expect("open selection"),
                            proposed_definitions: archive.manifest.module_refs.clone(),
                            proposed_objects: objects,
                            missing: import_preview_missing(&archive, &draft),
                            conflicts: vec![],
                            required_human_choices: vec![
                                "核对对象与精确模块版本，并确认打开工程及当前工程的离开方式。".into(),
                            ],
                            proposed_data,
                        }));
                        task.staged = Some(archive);
                    }
                    _ => unreachable!("only file tasks produce private work items"),
                }
                // A successful store result means the commit already won cancellation.
                task.state.task.state = ComputationStatus::Completed;
                task.state.allowed_controls.clear();
            }
            Ok(WorkOutput::Run {
                engine,
                input,
                supporting_scenarios,
            }) => {
                let result = engine.snapshot();
                task.state.task.state = result.computation_status.clone();
                task.state.task.result_ref = Some(run_result_file(&input));
                task.state.result = Some(TaskOutput::Run(run_summary(&result)));
                task.state.allowed_controls.clear();
                if task.epoch == self.epoch
                    && self
                        .project
                        .as_ref()
                        .is_some_and(|p| p.id == input.project.id)
                {
                    let TaskScope::Edit(scope) = &task.state.task.scope else {
                        unreachable!("run task has an edit scope")
                    };
                    let history = StoredRunHistory {
                        run_id: result.id.clone(),
                        input_ref: result.input_ref.clone(),
                        scope: scope.clone(),
                        records: engine.item_history(),
                        job_records: Some(engine.job_history()),
                        port_outputs: Some(engine.port_output_history()),
                    };
                    self.additions.extend(run_additions(
                        &input,
                        &result,
                        &history,
                        &supporting_scenarios,
                    ));
                    self.runs.insert(
                        result.id.clone(),
                        HostRun {
                            result,
                            engine: Some(engine),
                            history: Some(history),
                            scope: scope.clone(),
                            observation_cache: Mutex::new(Default::default()),
                        },
                    );
                    self.content_generation += 1;
                    self.dirty = true;
                }
            }
            Ok(WorkOutput::ExportPackage(exported)) => {
                task.state.task.result_ref = Some(exported.package.clone());
                task.state.result = Some(TaskOutput::Package(exported));
                // The store returns only after its atomic output commit. A late
                // cancellation never deletes that output or changes its result.
                task.state.task.state = ComputationStatus::Completed;
                task.state.allowed_controls.clear();
            }
            Ok(WorkOutput::Evaluation(exported)) => {
                task.state.task.result_ref = exported
                    .files
                    .iter()
                    .find(|file| matches!(file.kind, DocumentKind::Evaluation))
                    .or_else(|| exported.files.first())
                    .cloned();
                task.state.result = Some(TaskOutput::Evaluation(exported));
                task.state.task.state = ComputationStatus::Completed;
                task.state.allowed_controls.clear();
            }
            Ok(WorkOutput::Packaging { .. }) => {
                unreachable!("packaging completion handled before borrowing task")
            }
            Ok(WorkOutput::Render(_)) => {
                unreachable!("render completion handled before borrowing task")
            }
            Ok(WorkOutput::ExecutionExport(_)) => unreachable!("export completion handled before borrowing task"),
            Err(failure) => {
                task.state.task.state = if matches!(failure.code, ErrorCode::Cancelled) {
                    ComputationStatus::Cancelled
                } else {
                    ComputationStatus::Failed
                };
                task.state.task.failure = Some(Issue {
                    code: format!("{:?}", failure.code),
                    object_ref: None,
                    field_path: None,
                    message: failure.message.clone(),
                    blocks: vec![],
                });
                task.state.failure = Some(failure);
                task.state.allowed_controls.clear();
            }
        }
        if failed {
            self.release_failed_packaging_input(&completion.task_id);
        }
    }

    fn selected_directory(
        &self,
        caller: &HostCallContext,
        selected: &SelectedFileRef,
        purpose: &FilePurpose,
    ) -> Result<PathBuf, ApiError> {
        let (path, kind) = self.selected_path(caller, selected, purpose)?;
        if !matches!(kind, SelectedPathKind::Directory) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Operation requires a directory selection",
                None,
            ));
        }
        Ok(path)
    }

    fn selected_path(
        &self,
        caller: &HostCallContext,
        selected: &SelectedFileRef,
        purpose: &FilePurpose,
    ) -> Result<(PathBuf, SelectedPathKind), ApiError> {
        let binding = self.selections.get(&selected.selection_id).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or expired file selection",
                None,
            )
        })?;
        if binding.session != caller.session_id
            || !same(&selected.purpose, purpose)
            || !same(&binding.purpose, purpose)
            || binding.project != self.project.as_ref().map(|p| p.id.clone())
            || !caller.selected_file_tokens.contains(&selected.selection_id)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "File selection session, purpose or project does not match",
                None,
            ));
        }
        Ok((binding.directory.clone(), binding.kind))
    }

    fn enqueue_io(
        &mut self,
        caller: &HostCallContext,
        scope: TaskScope,
        operation: OperationName,
        revision: Revision,
        selection: Option<SelectedFileRef>,
        input: IoInput,
    ) -> TaskHandle {
        let id = TaskId::try_from(format!(
            "{}:task:{}",
            self.host_instance_id.as_str(),
            self.next_task
        ))
        .unwrap();
        self.next_task += 1;
        let cancelled = Arc::new(AtomicBool::new(false));
        let started = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(None));
        let packaging_input = if let IoInput::Packaging { job, .. } = &input {
            let api_scope = match &scope {
                TaskScope::Edit(EditScope::DefinitionDraft(identity)) => {
                    ApiScope::DefinitionDraft(identity.id.clone())
                }
                _ => ApiScope::Project,
            };
            Some((
                self.packaging_scope_key(&api_scope),
                job.id.clone(),
                job.revision.0.value(),
            ))
        } else {
            None
        };
        let task = TaskHandle {
            id: id.clone(),
            scope,
            kind: match operation {
                OperationName::SaveProject
                | OperationName::ExportPackage
                | OperationName::ExportEvaluation => LongTaskKind::Exporting,
                OperationName::ExportExecutionReport | OperationName::ExportCloudData => LongTaskKind::Exporting,
                OperationName::RunScenario => LongTaskKind::Simulation,
                OperationName::ComputePackaging => LongTaskKind::Packaging,
                OperationName::RenderView => LongTaskKind::Rendering,
                OperationName::ReadCloudProduction => LongTaskKind::CloudRead,
                _ => LongTaskKind::Importing,
            },
            state: ComputationStatus::Queued,
            input_revision: revision,
            progress: None,
            result_ref: None,
            failure: None,
        };
        self.tasks.insert(
            id.clone(),
            HostIoTask {
                owner: caller.session_id.clone(),
                epoch: self.epoch,
                state: TaskState {
                    originating_request_id: None,
                    task: task.clone(),
                    originating_operation: operation.clone(),
                    allowed_controls: vec![TaskControlAction::Cancel],
                    result: None,
                    failure: None,
                },
                cancelled: cancelled.clone(),
                started: started.clone(),
                selection,
                staged: None,
                saved_content_generation: if matches!(operation, OperationName::SaveProject) {
                    Some(self.content_generation)
                } else {
                    None
                },
                progress: progress.clone(),
                packaging_input,
            },
        );
        self.io_work.push(ProjectIoWork {
            task_id: id,
            cancelled,
            started,
            input,
            progress,
        });
        task
    }

    fn save(
        &mut self,
        caller: &HostCallContext,
        _: &ApiScope,
        params: SaveProjectParams,
    ) -> Result<TaskHandle, ApiError> {
        let project = self.project.as_ref().expect("admitted Project scope");
        check_revision(&params.revision, &project.revision, "/params/revision")?;
        let directory = self.selected_directory(
            caller,
            &params.selected_directory_ref,
            &FilePurpose::SaveProject,
        )?;
        let project = project.clone();
        let draft = self.current_draft();
        let scope = TaskScope::Edit(EditScope::Project(project.id.clone()));
        let execution_captures=self.capture_execution_archives()?;
        Ok(self.enqueue_io(
            caller,
            scope,
            OperationName::SaveProject,
            project.revision.clone(),
            Some(params.selected_directory_ref),
            IoInput::Save {
                directory,
                draft,
                backing: self.backing.clone(),
                additions: self.additions.clone(),
                execution_captures,
            },
        ))
    }

    fn import(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: ImportPackageParams,
    ) -> Result<TaskHandle, ApiError> {
        let (task_scope, revision) = match (scope, &params.import_policy) {
            (ApiScope::Workspace, ImportPolicy::OpenProject) => {
                if params.base_revision.is_some() {
                    return Err(error(ErrorCode::InvalidInput, "Workspace open requires base_revision=null", Some("/params/base_revision")));
                }
                (TaskScope::Workspace(caller.session_id.clone()), Revision(SafeUint::try_from(0).unwrap()))
            }
            (ApiScope::Project, ImportPolicy::IntoProject) => {
                let project=self.project.as_ref().expect("admitted Project scope");
                let supplied=params.base_revision.as_ref().ok_or_else(||error(ErrorCode::InvalidInput,"导入当前工程需要 base_revision。",Some("/params/base_revision")))?;
                check_revision(supplied,&project.revision,"/params/base_revision")?;
                (TaskScope::Edit(EditScope::Project(project.id.clone())),project.revision.clone())
            }
            _=>return Err(unsupported("请选择工作区打开工程，或在当前工程中导入精确定义。")),
        };
        let (directory, _) = self.selected_path(
            caller,
            &params.selected_file_ref,
            &FilePurpose::ImportPackage,
        )?;
        Ok(self.enqueue_io(
            caller,
            task_scope,
            OperationName::ImportPackage,
            revision,
            Some(params.selected_file_ref),
            IoInput::Open { directory },
        ))
    }

    fn packaging_scope_key(&self, scope: &ApiScope) -> String {
        serde_json::to_string(&(
            self.project.as_ref().unwrap().id.clone(),
            match scope {
                ApiScope::DefinitionDraft(id) => Some(id.clone()),
                _ => None,
            },
        ))
        .unwrap()
    }

    fn packaging_draft(&self, scope: &ApiScope) -> ProjectDraft {
        let mut draft = self.current_draft();
        if let ApiScope::DefinitionDraft(id) = scope {
            draft.professional = self.definition_drafts[id].professional.clone();
        }
        draft
    }

    fn check_packaging_job_reservation(
        &self,
        scope: &ApiScope,
        job: &PackageStructureJob,
    ) -> Result<(), ApiError> {
        let key = (
            self.packaging_scope_key(scope),
            job.id.clone(),
            job.revision.0.value(),
        );
        if self
            .packaging_inputs
            .get(&key)
            .is_some_and(|input| input.pending_search)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "This exact execution Job revision is reserved for the admitted search result",
                Some("/commands"),
            ));
        }
        Ok(())
    }

    fn release_failed_packaging_input(&mut self, task_id: &TaskId) {
        let task = &self.tasks[task_id];
        let Some(key) = task.packaging_input.clone() else {
            return;
        };
        if self.tasks.iter().any(|(id, task)| {
            id != task_id
                && task.packaging_input.as_ref() == Some(&key)
                && !terminal(&task.state.task.state)
        }) {
            return;
        }
        let documents = match &task.state.task.scope {
            TaskScope::Edit(EditScope::Project(id))
                if task.epoch == self.epoch
                    && self
                        .project
                        .as_ref()
                        .is_some_and(|project| &project.id == id) =>
            {
                Some(&self.professional)
            }
            TaskScope::Edit(EditScope::DefinitionDraft(identity)) if task.epoch == self.epoch => {
                self.definition_drafts
                    .get(&identity.id)
                    .map(|draft| &draft.professional)
            }
            _ => None,
        };
        let committed = documents
            .and_then(|documents| documents.jobs.get(&(key.1.clone(), key.2)))
            .is_some_and(|job| {
                self.packaging_inputs
                    .get(&key)
                    .is_some_and(|input| exact_data(job, &input.job))
            });
        if committed {
            self.packaging_inputs.get_mut(&key).unwrap().pending_search = false;
        } else {
            self.packaging_inputs.remove(&key);
        }
    }

    fn compute_packaging(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: ComputePackagingParams,
    ) -> Result<TaskHandle, ApiError> {
        let project = self.project.as_ref().expect("admitted active project");
        let (revision, task_scope) = if let ApiScope::DefinitionDraft(id) = scope {
            let revision = self.definition_drafts[id].revision.clone();
            (
                revision.clone(),
                TaskScope::Edit(EditScope::DefinitionDraft(DefinitionScopeIdentity {
                    id: id.clone(),
                    revision,
                })),
            )
        } else {
            (
                project.revision.clone(),
                TaskScope::Edit(EditScope::Project(project.id.clone())),
            )
        };
        check_revision(&params.input_revision, &revision, "/params/input_revision")?;
        let key = self.packaging_scope_key(scope);
        let mut draft = self.packaging_draft(scope);
        for ((current_scope, _, _), input) in &self.packaging_inputs {
            if current_scope == &key {
                put_packaging_job(&mut draft, input.job.clone())?;
            }
        }
        let mut job = freeze_packaging_execution_input(
            &draft,
            &params.packaging_job,
            &params.computation_budget,
        )?;
        let searching = !job.variables.is_empty();
        if searching
            && draft
                .professional
                .jobs
                .contains_key(&(job.id.clone(), job.revision.0.value()))
        {
            let highest = draft
                .professional
                .jobs
                .keys()
                .filter(|(id, _)| id == &job.id)
                .map(|(_, revision)| *revision)
                .max()
                .unwrap_or(job.revision.0.value());
            job.revision = highest
                .checked_add(1)
                .and_then(|revision| SafeUint::try_from(revision).ok())
                .map(Revision)
                .ok_or_else(|| {
                    error(
                        ErrorCode::InvalidInput,
                        "No unused exact execution Job revision remains for a searched candidate",
                        Some("/params/packaging_job/revision"),
                    )
                })?;
        }
        let cache_key = (key, job.id.clone(), job.revision.0.value());
        let canonical = if let Some(input) = self.packaging_inputs.get(&cache_key) {
            input.clone()
        } else {
            let retained = draft
                .professional
                .artifacts
                .values()
                .find(|artifact| {
                    draft
                        .professional
                        .results
                        .get(&artifact.id)
                        .is_some_and(|result| {
                            result.job_ref.id == job.id
                                && result.job_ref.revision.0.value() == job.revision.0.value()
                        })
                })
                .map(|artifact| artifact.input_ref.clone())
                .or_else(|| {
                    if matches!(scope, ApiScope::DefinitionDraft(_)) {
                        return None;
                    }
                    self.backing
                        .as_ref()?
                        .manifest
                        .documents
                        .iter()
                        .find(|entry| {
                            matches!(entry.file_ref.kind, DocumentKind::PackagingJob)
                                && entry.object_ref.id == job.id
                                && entry.revision.as_ref().is_some_and(|revision| {
                                    revision.0.value() == job.revision.0.value()
                                })
                        })
                        .map(|entry| entry.file_ref.clone())
                });
            let file = retained.unwrap_or_else(|| {
                let directory = self.next_document_directory("packaging-input");
                file_ref(&format!("{directory}/job.json"), DocumentKind::PackagingJob)
            });
            let bytes = draft
                .professional
                .files
                .get(file.path.as_str())
                .or_else(|| {
                    self.backing
                        .as_ref()
                        .and_then(|archive| archive.files.get(file.path.as_str()))
                })
                .cloned()
                .unwrap_or_else(|| encode_document(&job));
            CanonicalPackagingInput {
                job: job.clone(),
                file,
                bytes,
                pending_search: searching,
            }
        };
        // A repeated exact frozen Job uses its original canonical input bytes;
        // only actual new results/manufacturing receive a new internal directory.
        let directory = self.next_document_directory("packaging-result");
        let mut artifact_id = ArtifactId::try_from(format!(
            "{}:artifact:{}:{}",
            self.host_instance_id.as_str(),
            self.epoch,
            self.next_task
        ))
        .unwrap();
        while draft.professional.artifacts.contains_key(&artifact_id) {
            self.next_task += 1;
            artifact_id = ArtifactId::try_from(format!(
                "{}:artifact:{}:{}",
                self.host_instance_id.as_str(),
                self.epoch,
                self.next_task
            ))
            .unwrap();
        }
        let context = sf_packaging::PackagingArtifactContext {
            artifact_id,
            input_ref: canonical.file.clone(),
            result_ref: file_ref(
                &format!("{directory}/result.json"),
                DocumentKind::PackagingResult,
            ),
            manufacturing_directory: RelativeFilePath::try_from(format!(
                "{directory}/manufacturing"
            ))
            .unwrap(),
        };
        self.packaging_inputs.insert(cache_key, canonical.clone());
        Ok(self.enqueue_io(
            caller,
            task_scope,
            OperationName::ComputePackaging,
            revision,
            None,
            IoInput::Packaging {
                job,
                budget: params.computation_budget,
                context,
                input_bytes: canonical.bytes,
                searching,
            },
        ))
    }

    fn finish_packaging(
        &mut self,
        task_id: TaskId,
        product: sf_packaging::PackagingProduct,
        input_bytes: Vec<u8>,
    ) {
        let Some(task) = self.tasks.get(&task_id) else {
            return;
        };
        if terminal(&task.state.task.state) {
            return;
        }
        let scope = match &task.state.task.scope {
            TaskScope::Edit(EditScope::Project(id))
                if task.epoch == self.epoch
                    && self
                        .project
                        .as_ref()
                        .is_some_and(|project| &project.id == id) =>
            {
                Some(ApiScope::Project)
            }
            TaskScope::Edit(EditScope::DefinitionDraft(identity))
                if task.epoch == self.epoch
                    && self.definition_drafts.contains_key(&identity.id) =>
            {
                Some(ApiScope::DefinitionDraft(identity.id.clone()))
            }
            _ => None,
        };
        let published = scope
            .ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Packaging completion no longer belongs to the active scope",
                    None,
                )
            })
            .and_then(|scope| self.publish_packaging(&scope, &product, input_bytes.clone()));
        if let Some(key) = self.tasks[&task_id].packaging_input.as_ref() {
            if let Some(input) = self.packaging_inputs.get_mut(key) {
                input.pending_search = false;
                if published.is_ok() {
                    input.job = product.selected_job.clone();
                    input.bytes = input_bytes;
                }
            }
        }
        let failed = published.is_err();
        let task = self.tasks.get_mut(&task_id).unwrap();
        match published {
            Ok(()) => {
                task.state.task.state = if product.cancelled {
                    ComputationStatus::Cancelled
                } else {
                    ComputationStatus::Completed
                };
                task.state.task.result_ref = Some(product.artifact.result_ref.clone());
                task.state.result = Some(TaskOutput::Packaging(product.result));
                task.state.allowed_controls.clear();
            }
            Err(failure) => fail_task(task, failure),
        }
        if failed {
            self.release_failed_packaging_input(&task_id);
        }
    }

    fn publish_packaging(
        &mut self,
        scope: &ApiScope,
        product: &sf_packaging::PackagingProduct,
        input_bytes: Vec<u8>,
    ) -> Result<(), ApiError> {
        let mut candidate = self.packaging_draft(scope);
        let mut files = BTreeMap::from([
            (
                product.artifact.input_ref.path.as_str().to_owned(),
                input_bytes,
            ),
            (
                product.artifact.result_ref.path.as_str().to_owned(),
                encode_document(&product.result),
            ),
        ]);
        for (file, bytes) in &product.manufacturing_bytes {
            if let Some(old) = files.insert(file.path.as_str().into(), bytes.clone()) {
                if old != *bytes {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Actual packaging output contains conflicting immutable file bytes",
                        None,
                    ));
                }
            }
        }
        ingest_professional_result(
            &mut candidate,
            product.selected_job.clone(),
            product.artifact.clone(),
            product.result.clone(),
            files,
        )?;
        if let ApiScope::DefinitionDraft(id) = scope {
            self.publish_definition_professional(id, candidate.professional)?;
        } else {
            validate_candidate(&candidate)?;
            let highwater = finalize_draft(&mut candidate, &self.scenario_highwater)?;
            self.refresh_current_business(&mut candidate);
            self.history.professional_boundary(&candidate);
            self.project = Some(candidate.project);
            self.scenarios = candidate.scenarios;
            self.professional = candidate.professional;
            self.scenario_highwater = highwater;
            self.content_generation += 1;
            self.dirty = true;
            if !product.cancelled {
                self.professional_trust
                    .record_actual_producer(product.artifact.id.clone());
            }
        }
        // This capability comes from the real compiled producer's execution,
        // never an imported provider declaration or a nominal outcome claim.
        if !product.cancelled {
            if let Some(existing) = self
                .providers
                .iter_mut()
                .find(|provider| provider.provider_id == product.provider.provider_id)
            {
                *existing = product.provider.clone();
            } else {
                self.providers.push(product.provider.clone());
            }
        }
        Ok(())
    }

    fn run_scenario(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: RunScenarioParams,
    ) -> Result<TaskHandle, ApiError> {
        if matches!(scope, ApiScope::DefinitionDraft(_)) {
            return self.run_definition_cycle(caller, scope, params);
        }
        let project = self.project.as_ref().expect("admitted Project scope");
        check_revision(
            &params.input_revision,
            &project.revision,
            "/params/input_revision",
        )?;
        if !matches!(params.intent, RunIntent::ProjectScenario) {
            return Err(unsupported(
                "Definition cycles require an actual definition draft",
            ));
        }
        if self.tasks.values().any(|task| matches!(task.state.originating_operation, OperationName::RunScenario) && matches!(&task.state.task.scope, TaskScope::Edit(EditScope::Project(id)) if id == &project.id) && !terminal(&task.state.task.state)) { return Err(error(ErrorCode::InvalidInput, "The current formal Run must reach a terminal state before another formal Run is admitted", None)); }
        let scenario = self.scenarios.get(&params.scenario_id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Current ScenarioInput does not exist",
                Some("/params/scenario_id"),
            )
        })?;
        if serde_json::to_value(&params.observation_window).unwrap()
            != serde_json::to_value(&scenario.window).unwrap()
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "The observation window must match the frozen ScenarioInput window",
                Some("/params/observation_window"),
            ));
        }
        let (project, scenario, definitions) = self.frozen_project_execution(&params.scenario_id)?;
        let mut id = RunId::try_from(format!(
            "{}:run:{}:{}",
            self.host_instance_id.as_str(),
            self.epoch,
            self.next_task
        ))
        .unwrap();
        if self.runs.contains_key(&id) {
            let prefix = id.as_str().to_owned();
            let mut ordinal = self.next_document;
            loop {
                id = RunId::try_from(format!("{prefix}:copy:{ordinal}")).unwrap();
                if !self.runs.contains_key(&id) {
                    break;
                }
                ordinal += 1;
            }
        }
        let path = self.next_document_directory("run");
        let input = sf_simulation::EngineInput {
            run_id: id,
            input_ref: RunInputRef {
                project_id: project.id.clone(),
                project_revision: project.revision.clone(),
                scenario_id: scenario.id.clone(),
                scenario_revision: scenario.revision.clone(),
                frozen_project_file: file_ref(
                    &format!("{path}/project.json"),
                    DocumentKind::Project,
                ),
                frozen_scenario_file: file_ref(
                    &format!("{path}/scenario.json"),
                    DocumentKind::Scenario,
                ),
            },
            project: project.clone(),
            scenario: scenario.clone(),
            definitions,
            scope: scenario.scope.clone(),
        };
        sf_simulation::check_inputs(&input).map_err(engine_failure)?;
        let supporting_scenarios = project
            .scenario_refs
            .iter()
            .filter(|id| *id != &scenario.id)
            .filter_map(|id| self.scenarios.get(id).cloned())
            .collect();
        Ok(self.enqueue_io(
            caller,
            TaskScope::Edit(EditScope::Project(project.id.clone())),
            OperationName::RunScenario,
            project.revision.clone(),
            None,
            IoInput::Run {
                input,
                supporting_scenarios,
            },
        ))
    }

    fn get_run(
        &self,
        _: &HostCallContext,
        scope: &ApiScope,
        params: GetRunParams,
    ) -> Result<RunView, ApiError> {
        let run = self.runs.get(&params.run_id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Run does not exist in the active project",
                Some("/params/run_id"),
            )
        })?;
        if !match (&run.scope, scope) {
            (EditScope::Project(id), ApiScope::Project) => self
                .project
                .as_ref()
                .is_some_and(|project| &project.id == id),
            (EditScope::DefinitionDraft(identity), ApiScope::DefinitionDraft(id)) => {
                &identity.id == id
            }
            _ => false,
        } {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Run belongs to a different edit scope",
                None,
            ));
        }
        match params.query {
            RunQuery::Summary => Ok(RunView::Summary(run_summary(&run.result))),
            RunQuery::FullResult => Ok(RunView::Full(Box::new(run.result.clone()))),
            RunQuery::Events { interval, page } => {
                if interval
                    .as_ref()
                    .is_some_and(|range| range.start_ms.0.value() > range.end_ms.0.value())
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Event interval is reversed",
                        Some("/params/query/data/interval"),
                    ));
                }
                let events: Vec<_> = run
                    .result
                    .events
                    .iter()
                    .filter(|event| {
                        interval.as_ref().is_none_or(|range| {
                            event.at_ms.0.value() >= range.start_ms.0.value()
                                && event.at_ms.0.value() < range.end_ms.0.value()
                        })
                    })
                    .cloned()
                    .collect();
                let (events, next_index) = page_values(events, page)?;
                Ok(RunView::Events(RunEventsPage {
                    run_id: params.run_id,
                    events,
                    next_index,
                }))
            }
            RunQuery::ObjectsAt {
                at_ms,
                objects,
                page,
            } => host_run_history::objects_at(run, params.run_id, at_ms, objects, page),
        }
    }

    fn artifact_consumer_authorized(&self, caller: &HostCallContext, consumer: &ObjectRef) -> bool {
        matches!(caller.origin, CallerOrigin::InteractiveUi)
            || (self
                .project
                .as_ref()
                .is_some_and(|project| lookup(project, &self.scenarios, consumer).is_ok())
                && caller.edit_grants.iter().any(|grant| {
                    self.current_grant(grant)
                        && grant
                            .branches
                            .iter()
                            .any(|branch| matches!(branch, HostEditBranch::AdoptArtifact))
                        && grant
                            .objects
                            .iter()
                            .any(|object| same_ref(&object.target, consumer))
                }))
    }

    fn adopt_artifact(
        &mut self,
        caller: &HostCallContext,
        _: &ApiScope,
        params: AdoptArtifactParams,
    ) -> Result<BindingCommit, ApiError> {
        if !self.artifact_consumer_authorized(caller, &params.consumer_ref) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "AdoptArtifact requires its dedicated branch and exact current consumer authority",
                None,
            ));
        }
        let before = self.current_draft();
        check_revision(
            &params.base_revision,
            &before.project.revision,
            "/params/base_revision",
        )?;
        let modules: Vec<_> = self.modules.values().cloned().collect();
        if let Some(current) = current_matching_adoption(&before.project, &params) {
            let trust = self.nominal_binding_trust(&before, current);
            let repeated = prepare_artifact_binding(
                &before,
                &modules,
                &params,
                current.id.clone(),
                &trust,
                &self.providers,
            )?;
            if exact_data(&repeated, current) {
                return Ok(BindingCommit {
                    project_id: before.project.id.clone(),
                    previous_revision: before.project.revision.clone(),
                    new_revision: before.project.revision.clone(),
                    binding: current.clone(),
                    issues: before.project.issues.clone(),
                });
            }
        }
        let mut candidate = before.clone();
        let id = ObjectId::try_from(format!(
            "{}:binding:{}:{}",
            self.host_instance_id.as_str(),
            self.epoch,
            self.content_generation
        ))
        .unwrap();
        let trust = self.adoption_trust(&before, &params);
        let binding =
            prepare_artifact_binding(&candidate, &modules, &params, id, &trust, &self.providers)?;
        candidate.project.bindings.push(binding.clone());
        let highwater = finalize_draft(&mut candidate, &self.scenario_highwater)?;
        self.refresh_current_business(&mut candidate);
        validate_candidate(&candidate)?;
        let receipt = self.committed_binding_receipt(&candidate, &binding);
        let commit = BindingCommit {
            project_id: candidate.project.id.clone(),
            previous_revision: before.project.revision.clone(),
            new_revision: candidate.project.revision.clone(),
            binding,
            issues: candidate.project.issues.clone(),
        };
        self.history.professional_boundary(&candidate);
        self.project = Some(candidate.project);
        self.scenarios = candidate.scenarios;
        self.professional = candidate.professional;
        self.scenario_highwater = highwater;
        self.content_generation += 1;
        self.dirty = true;
        self.committed_nominal_bindings.push(receipt);
        Ok(commit)
    }

    fn committed_binding_receipt(
        &self,
        draft: &ProjectDraft,
        binding: &ConsumerBinding,
    ) -> CommittedNominalBinding {
        let artifact = draft.professional.artifacts[&binding.artifact_ref].clone();
        let result = draft.professional.results[&binding.artifact_ref].clone();
        let job = draft.professional.jobs
            [&(result.job_ref.id.clone(), result.job_ref.revision.0.value())]
            .clone();
        let acknowledgment = draft
            .project
            .acknowledgments
            .iter()
            .find(|ack| ack.id == binding.acceptance_ref)
            .expect("binding validated acknowledgment")
            .clone();
        let device = draft
            .project
            .devices
            .iter()
            .find(|device| device.id == binding.consumer_ref.id)
            .expect("binding validated consumer");
        let modules = self
            .modules
            .values()
            .filter(|package| {
                let reference = ModuleRef {
                    id: package.definition.id.clone(),
                    version: package.definition.version.clone(),
                };
                exact_data(&reference, &device.module_ref)
                    || exact_data(&reference, &binding.operation_ref)
            })
            .cloned()
            .collect();
        let product = draft
            .project
            .products
            .iter()
            .find(|product| {
                product.id == job.subject_ref.id && product.revision == job.subject_ref.revision
            })
            .expect("binding validated subject")
            .clone();
        let files = std::iter::once(&artifact.input_ref)
            .chain(std::iter::once(&artifact.result_ref))
            .chain(result.manufacturing_files.iter().map(|file| &file.file))
            .map(|file| {
                let path = file.path.as_str().to_owned();
                (path.clone(), draft.professional.files[&path].clone())
            })
            .collect();
        CommittedNominalBinding {
            project_id: draft.project.id.clone(),
            binding: binding.clone(),
            artifact,
            result,
            job,
            acknowledgment,
            files,
            modules,
            product,
        }
    }

    fn nominal_binding_trust(
        &self,
        draft: &ProjectDraft,
        binding: &ConsumerBinding,
    ) -> ProfessionalTrust {
        let mut trust = self.professional_trust.clone();
        if self.has_committed_binding_receipt(draft, binding) {
            trust.record_actual_producer(binding.artifact_ref.clone());
            trust.record_human_acknowledgment(binding.acceptance_ref.clone());
        }
        trust
    }

    fn adoption_trust(
        &self,
        draft: &ProjectDraft,
        params: &AdoptArtifactParams,
    ) -> ProfessionalTrust {
        let mut trust = self.professional_trust.clone();
        // Retained exact genuine history can supply observed producer/ack facts
        // for a newly authorized adoption. The full candidate gate still checks
        // the current independent inputs and requested purposes. This local
        // trust is never persisted or inferred from imported source labels.
        if draft.project.bindings.iter().any(|binding| {
            matches!(binding.mode, ProfessionalMode::CalculatedNominal)
                && same_ref(&binding.consumer_ref, &params.consumer_ref)
                && binding.artifact_ref == params.artifact_id
                && binding.acceptance_ref == params.acceptance_ref
                && exact_data(&binding.operation_ref, &params.operation_ref)
                && self.has_committed_binding_receipt(draft, binding)
        }) {
            trust.record_actual_producer(params.artifact_id.clone());
            trust.record_human_acknowledgment(params.acceptance_ref.clone());
        }
        trust
    }

    fn has_committed_binding_receipt(
        &self,
        draft: &ProjectDraft,
        binding: &ConsumerBinding,
    ) -> bool {
        self.committed_nominal_bindings.iter().any(|receipt| {
            receipt.project_id == draft.project.id
                && exact_data(&receipt.binding, binding)
                && draft
                    .professional
                    .artifacts
                    .get(&binding.artifact_ref)
                    .is_some_and(|value| exact_data(value, &receipt.artifact))
                && draft
                    .professional
                    .results
                    .get(&binding.artifact_ref)
                    .is_some_and(|value| exact_data(value, &receipt.result))
                && draft
                    .professional
                    .jobs
                    .get(&(receipt.job.id.clone(), receipt.job.revision.0.value()))
                    .is_some_and(|value| exact_data(value, &receipt.job))
                && draft
                    .project
                    .acknowledgments
                    .iter()
                    .any(|value| exact_data(value, &receipt.acknowledgment))
                && draft
                    .project
                    .products
                    .iter()
                    .any(|value| exact_data(value, &receipt.product))
                && receipt
                    .files
                    .iter()
                    .all(|(path, bytes)| draft.professional.files.get(path) == Some(bytes))
                && receipt.modules.iter().all(|module| {
                    self.modules
                        .get(&module_key(&ModuleRef {
                            id: module.definition.id.clone(),
                            version: module.definition.version.clone(),
                        }))
                        .is_some_and(|value| exact_data(value, module))
                })
        })
    }

    fn artifact_path_available(&self, caller: &HostCallContext) -> bool {
        if self.providers.is_empty() || self.project.is_none() {
            return false;
        }
        let draft = self.current_draft();
        let modules: Vec<_> = self.modules.values().cloned().collect();
        draft.project.devices.iter().any(|device| {
            let consumer = ObjectRef {
                kind: ObjectKind::Device,
                id: device.id.clone(),
            };
            if !self.artifact_consumer_authorized(caller, &consumer) {
                return false;
            }
            if draft.project.bindings.iter().any(|binding| {
                same_ref(&binding.consumer_ref, &consumer)
                    && matches!(binding.mode, ProfessionalMode::CalculatedNominal)
                    && {
                        let params = AdoptArtifactParams {
                            artifact_id: binding.artifact_ref.clone(),
                            acceptance_ref: binding.acceptance_ref.clone(),
                            consumer_ref: binding.consumer_ref.clone(),
                            operation_ref: binding.operation_ref.clone(),
                            requested_scope: binding.requested_scope.clone(),
                            base_revision: draft.project.revision.clone(),
                        };
                        let trust = self.adoption_trust(&draft, &params);
                        prepare_artifact_binding(
                            &draft,
                            &modules,
                            &params,
                            binding.id.clone(),
                            &trust,
                            &self.providers,
                        )
                        .is_ok()
                    }
            }) {
                return true;
            }
            draft.project.acknowledgments.iter().any(|ack| {
                let Some(artifact) = draft.professional.artifacts.get(&ack.artifact_ref) else {
                    return false;
                };
                let Some(operation) = self.selected_device_operation(device) else {
                    return false;
                };
                let params = AdoptArtifactParams {
                    artifact_id: artifact.id.clone(),
                    acceptance_ref: ack.id.clone(),
                    consumer_ref: consumer.clone(),
                    operation_ref: operation,
                    requested_scope: ack.accepted_scope.clone(),
                    base_revision: draft.project.revision.clone(),
                };
                let trust = self.adoption_trust(&draft, &params);
                prepare_artifact_binding(
                    &draft,
                    &modules,
                    &params,
                    ObjectId::try_from("capability-probe".to_owned()).unwrap(),
                    &trust,
                    &self.providers,
                )
                .is_ok()
            })
        })
    }

    fn selected_device_operation(&self, device: &DeviceInstance) -> Option<ModuleRef> {
        if let Some(selected) = &device.operation_binding {
            return Some(selected.clone());
        }
        let module = self.modules.get(&module_key(&device.module_ref))?;
        let ModulePayload::Devices(definition) = &module.definition.payload else {
            return None;
        };
        match definition.behavior.selected {
            Some(BehaviorSourceKind::OperationRef) => definition.behavior.operation_ref.clone(),
            Some(BehaviorSourceKind::Inline) if definition.behavior.inline.is_some() => {
                Some(device.module_ref.clone())
            }
            _ => None,
        }
    }

    fn owned_task(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        id: &TaskId,
    ) -> Result<&HostIoTask, ApiError> {
        let task = self
            .tasks
            .get(id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "Task does not exist", None))?;
        let scope_matches = match (&task.state.task.scope, scope) {
            (TaskScope::Workspace(owner), ApiScope::Workspace) => {
                owner == &caller.session_id || self.has_external_task_access(caller, task, false)
            }
            (TaskScope::Edit(EditScope::Project(id)), ApiScope::Project) => {
                self.project.as_ref().is_some_and(|p| &p.id == id)
            }
            (
                TaskScope::Edit(EditScope::DefinitionDraft(identity)),
                ApiScope::DefinitionDraft(id),
            ) => &identity.id == id,
            _ => false,
        };
        if (task.owner != caller.session_id && !self.has_external_task_access(caller, task, false))
            || !scope_matches
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Task is outside the session and scope",
                None,
            ));
        }
        Ok(task)
    }

    fn get_task(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: GetTaskParams,
    ) -> Result<TaskState, ApiError> {
        let task = self.owned_task(caller, scope, &params.task_id)?;
        let mut state = current_task_state(task);
        if (task.owner != caller.session_id && !self.has_external_task_access(caller, task, true))
            || !caller
                .allowed_operations
                .iter()
                .any(|operation| matches!(operation, OperationName::CancelTask))
        {
            state.allowed_controls.clear();
        }
        Ok(state)
    }

    fn cancel_task(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: CancelTaskParams,
    ) -> Result<CancellationState, ApiError> {
        let task = self.owned_task(caller, scope, &params.task_id)?;
        if task.owner != caller.session_id && !self.has_external_task_access(caller, task, true) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Task access is read-only",
                None,
            ));
        }
        if !matches!(params.action, TaskControlAction::Cancel) {
            return Err(unsupported("This task currently supports Cancel only"));
        }
        if !terminal(&task.state.task.state)
            && !task
                .state
                .allowed_controls
                .iter()
                .any(|control| same(control, &params.action))
        {
            return Err(unsupported(
                "The task does not currently allow this control",
            ));
        }
        let task = self.tasks.get_mut(&params.task_id).unwrap();
        let won = terminal(&task.state.task.state);
        if !won {
            task.cancelled.store(true, Ordering::Release);
            task.state.task.state = ComputationStatus::CancelRequested;
        }
        Ok(CancellationState {
            task: task.state.task.clone(),
            requested_action: params.action,
            action_accepted: !won,
            terminal_won: won,
            issues: vec![],
        })
    }

    pub fn dispatch(&mut self, session_id: &ObjectId, request: ApiRequest) -> Value {
        if matches!(&request, ApiRequest::StartExecutionSession(_) | ApiRequest::ReadCloudProduction(_) | ApiRequest::ExportExecutionReport(_) | ApiRequest::ExportCloudData(_)) {
            return self.dispatch_execution_original(session_id, request);
        }
        self.dispatch_once(session_id, request)
    }

    fn dispatch_once(&mut self, session_id: &ObjectId, request: ApiRequest) -> Value {
        macro_rules! call {
            ($request:expr, $operation:expr, $handler:ident) => {{
                let request = $request;
                let caller = self.sessions.get(session_id).cloned();
                let admission = caller
                    .as_ref()
                    .ok_or_else(|| {
                        error(
                            ErrorCode::PermissionDenied,
                            "Unknown or revoked session",
                            None,
                        )
                    })
                    .and_then(|caller| {
                        self.admit(caller, &request.project_id, &request.scope, &$operation)
                    });
                match admission {
                    Err(failure) => {
                        self.failure(request.request_id, session_id, &request.scope, failure)
                    }
                    Ok(()) => {
                        let caller = caller.expect("admission requires registered session");
                        let result = self.$handler(&caller, &request.scope, request.params);
                        self.respond(request.request_id, session_id, &request.scope, result)
                    }
                }
            }};
        }
        match request {
            gateway @ (ApiRequest::ApplyDeviceReceipt(_) | ApiRequest::GetExecutionCloudWork(_) | ApiRequest::AcknowledgeExecutionCloudWork(_) | ApiRequest::ApplyDeviceObservation(_)) => {
                let (id,_,scope,_)=request_header(gateway);
                self.failure(id,session_id,&scope,error(ErrorCode::PermissionDenied,"Gateway operations require the actual Native private context",None))
            },
            ApiRequest::ReadCloudProduction(r) => self.dispatch_cloud_read(session_id,r),
            ApiRequest::StartExecutionSession(r) => self.dispatch_execution_start(session_id,r),
            ApiRequest::ExportExecutionReport(r) => self.dispatch_execution_report(session_id,r),
            ApiRequest::ExportCloudData(r) => self.dispatch_cloud_data_export(session_id,r),
            ApiRequest::GetExecutionSnapshot(r) => call!(r.into_inner(), OperationName::GetExecutionSnapshot, read_execution_snapshot),
            ApiRequest::ReadExecutionView(r) => call!(r.into_inner(), OperationName::ReadExecutionView, read_execution_view),
            ApiRequest::ControlExecutionSession(r) => self.dispatch_execution_control(session_id, r),
            ApiRequest::DescribeCapabilities(r) => {
                call!(r, OperationName::DescribeCapabilities, capabilities)
            }
            ApiRequest::GetContext(r) => call!(r, OperationName::GetContext, context),
            ApiRequest::GetObject(r) => call!(r, OperationName::GetObject, object),
            ApiRequest::RenderView(r) => call!(r, OperationName::RenderView, render_view),
            ApiRequest::ApplyCommands(r) => call!(r, OperationName::ApplyCommands, apply),
            ApiRequest::SaveProject(r) => call!(r, OperationName::SaveProject, save),
            ApiRequest::ImportPackage(r) => call!(r, OperationName::ImportPackage, import),
            ApiRequest::GetTask(r) => call!(r, OperationName::GetTask, get_task),
            ApiRequest::CancelTask(r) => call!(r, OperationName::CancelTask, cancel_task),
            ApiRequest::RunScenario(r) => call!(r, OperationName::RunScenario, run_scenario),
            ApiRequest::GetRun(r) => call!(r, OperationName::GetRun, get_run),
            ApiRequest::CompareRuns(r) => call!(r, OperationName::CompareRuns, compare_runs),
            ApiRequest::ExportPackage(r) => call!(r, OperationName::ExportPackage, export_package),
            ApiRequest::ExportEvaluation(r) => {
                call!(r, OperationName::ExportEvaluation, export_evaluation)
            }
            ApiRequest::ComputePackaging(r) => {
                call!(r, OperationName::ComputePackaging, compute_packaging)
            }
            ApiRequest::AdoptArtifact(r) => call!(r, OperationName::AdoptArtifact, adopt_artifact),
            ApiRequest::QueryModuleLibrary(r) => {
                call!(r, OperationName::QueryModuleLibrary, query_modules)
            }
            ApiRequest::CheckFactoryDraft(r) => {
                call!(r, OperationName::CheckFactoryDraft, check_factory_draft)
            }
        }
    }

    fn admit(
        &self,
        caller: &HostCallContext,
        project_id: &Option<ProjectId>,
        scope: &ApiScope,
        operation: &OperationName,
    ) -> Result<(), ApiError> {
        if self.desktop_closing {
            return Err(error(ErrorCode::InvalidInput, "Desktop is closing; no further requests are admitted", None));
        }
        if !caller
            .allowed_operations
            .iter()
            .any(|allowed| same(allowed, operation))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Operation is not authorized for this session",
                None,
            ));
        }
        match scope {
            ApiScope::Workspace => {
                if project_id.is_some() {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Workspace requires project_id=null",
                        Some("/project_id"),
                    ));
                }
                if !matches!(
                    operation,
                    OperationName::DescribeCapabilities
                        | OperationName::GetContext
                        | OperationName::ApplyCommands
                        | OperationName::ImportPackage
                        | OperationName::GetTask
                        | OperationName::CancelTask
                        | OperationName::QueryModuleLibrary
                ) {
                    return Err(unsupported("Operation does not support Workspace scope"));
                }
                if matches!(caller.origin, CallerOrigin::ExternalAgent)
                    && matches!(
                        operation,
                        OperationName::ApplyCommands | OperationName::ImportPackage
                    )
                {
                    return Err(error(
                        ErrorCode::PermissionDenied,
                        "Workspace creation/open requires the actual UI session",
                        None,
                    ));
                }
                Ok(())
            }
            ApiScope::Project | ApiScope::DefinitionDraft(_) => {
                // An external session's grant is checked before looking up current project state.
                if matches!(caller.origin, CallerOrigin::ExternalAgent)
                    && (project_id.is_none()
                        || caller.granted_project.as_ref() != project_id.as_ref())
                {
                    return Err(error(
                        ErrorCode::PermissionDenied,
                        "Project is outside the session grant",
                        None,
                    ));
                }
                let project = self
                    .project
                    .as_ref()
                    .ok_or_else(|| error(ErrorCode::NoActiveProject, "No active project", None))?;
                if project_id.as_ref() != Some(&project.id) {
                    return Err(error(
                        ErrorCode::PermissionDenied,
                        "Project is not the active authorized project",
                        None,
                    ));
                }
                if let ApiScope::DefinitionDraft(id) = scope {
                    if matches!(caller.origin, CallerOrigin::ExternalAgent)
                        && !caller.allowed_definition_drafts.contains(id)
                    {
                        return Err(error(
                            ErrorCode::PermissionDenied,
                            "Definition draft is outside the session grant",
                            None,
                        ));
                    }
                    if !self.definition_drafts.contains_key(id) {
                        return Err(unsupported(
                            "DefinitionDraft has no registered runtime document",
                        ));
                    }
                    if !matches!(
                        operation,
                        OperationName::GetContext
                            | OperationName::GetObject
                            | OperationName::ApplyCommands
                            | OperationName::RenderView
                            | OperationName::RunScenario
                            | OperationName::ComputePackaging
                            | OperationName::GetRun
                            | OperationName::GetTask
                            | OperationName::CancelTask
                            | OperationName::ExportPackage
                    ) {
                        return Err(unsupported(
                            "Operation is not implemented for DefinitionDraft",
                        ));
                    }
                }
                if matches!(operation, OperationName::DescribeCapabilities) {
                    return Err(unsupported(
                        "describe_capabilities requires Workspace scope",
                    ));
                }
                Ok(())
            }
        }
    }

    fn response_context(&self, session: &ObjectId, scope: &ApiScope) -> ResponseContext {
        let caller = self.sessions.get(session);
        let visible = self.project.as_ref().filter(|project| {
            caller.is_some_and(|c| {
                matches!(c.origin, CallerOrigin::InteractiveUi)
                    || c.granted_project.as_ref() == Some(&project.id)
            })
        });
        let edit_scope = match scope {
            ApiScope::Project => visible.map(|p| EditScope::Project(p.id.clone())),
            ApiScope::DefinitionDraft(id) => self.definition_drafts.get(id).map(|draft| {
                EditScope::DefinitionDraft(DefinitionScopeIdentity {
                    id: id.clone(),
                    revision: draft.revision.clone(),
                })
            }),
            _ => None,
        };
        ResponseContext {
            host_instance_id: self.host_instance_id.clone(),
            session_id: session.clone(),
            project_id: visible.map(|p| p.id.clone()),
            // A rejected DefinitionDraft call has no resolved revision. Do not
            // substitute the active Project's revision for the absent draft.
            project_revision: if matches!(scope, ApiScope::DefinitionDraft(_)) {
                None
            } else {
                visible.map(|p| p.revision.clone())
            },
            scope: edit_scope,
            scope_revision: match scope {
                ApiScope::Project => visible.map(|p| p.revision.clone()),
                ApiScope::DefinitionDraft(id) => self
                    .definition_drafts
                    .get(id)
                    .map(|draft| draft.revision.clone()),
                _ => None,
            },
        }
    }

    fn respond<R: Serialize>(
        &self,
        id: ObjectId,
        session: &ObjectId,
        scope: &ApiScope,
        result: Result<R, ApiError>,
    ) -> Value {
        let denied = matches!(&result, Err(e) if matches!(e.code, ErrorCode::PermissionDenied));
        let unresolved_definition = matches!(scope, ApiScope::DefinitionDraft(_))
            && matches!(&result, Err(e) if matches!(e.code, ErrorCode::UnsupportedCapability));
        let response = Response {
            api_version: ApiVersion::V01,
            request_id: Some(id),
            context: (!denied && !unresolved_definition)
                .then(|| self.response_context(session, scope)),
            outcome: match result {
                Ok(value) => ApiOutcome::Ok(value),
                Err(e) => ApiOutcome::Error(e),
            },
        };
        serde_json::to_value(response).expect("validated Host response serializes")
    }

    fn failure(
        &self,
        id: ObjectId,
        session: &ObjectId,
        scope: &ApiScope,
        failure: ApiError,
    ) -> Value {
        self.respond::<Value>(id, session, scope, Err(failure))
    }

    fn capabilities(
        &self,
        caller: &HostCallContext,
        _: &ApiScope,
        _: DescribeCapabilitiesParams,
    ) -> Result<CapabilityCatalog, ApiError> {
        let commands = self.available_commands(caller);
        let operations = implemented_operations()
            .into_iter()
            .filter(|op| !matches!(op, OperationName::StartExecutionSession) || self.execution_preparation_available())
            .filter(|op| !matches!(op, OperationName::ControlExecutionSession) || self.execution_control_capability(caller))
            .filter(|op| {
                caller
                    .allowed_operations
                    .iter()
                    .any(|allowed| same(allowed, op))
            })
            .filter(|op| {
                matches!(caller.origin, CallerOrigin::InteractiveUi)
                    || !matches!(op, OperationName::ImportPackage)
            })
            .filter(|op| {
                !matches!(op, OperationName::AdoptArtifact) || self.artifact_path_available(caller)
            })
            .filter(|op| {
                !matches!(op, OperationName::RenderView)
                    || (self.rendering_available()
                        && (self.render_scope_available(caller, &ApiScope::Project)
                            || self.definition_drafts.keys().any(|id| {
                                self.render_scope_available(
                                    caller,
                                    &ApiScope::DefinitionDraft(id.clone()),
                                )
                            })))
            })
            .filter(|op| {
                matches!(caller.origin, CallerOrigin::InteractiveUi)
                    || !matches!(op, OperationName::ApplyCommands)
                    || !commands.is_empty()
            })
            .map(|operation| {
                let (name, mut scopes) = match operation {
                    OperationName::DescribeCapabilities => {
                        ("describe_capabilities", vec![ApiScope::Workspace])
                    }
                    OperationName::GetContext => {
                        ("get_context", vec![ApiScope::Workspace, ApiScope::Project])
                    }
                    OperationName::GetObject => ("get_object", vec![ApiScope::Project]),
                    OperationName::RenderView => ("render_view", vec![ApiScope::Project]),
                    OperationName::ApplyCommands => (
                        "apply_commands",
                        if matches!(caller.origin, CallerOrigin::InteractiveUi) {
                            vec![ApiScope::Workspace, ApiScope::Project]
                        } else {
                            vec![ApiScope::Project]
                        },
                    ),
                    OperationName::SaveProject => ("save_project", vec![ApiScope::Project]),
                    OperationName::ImportPackage => ("import_package", vec![ApiScope::Workspace, ApiScope::Project]),
                    OperationName::GetTask => {
                        ("get_task", vec![ApiScope::Workspace, ApiScope::Project])
                    }
                    OperationName::CancelTask => {
                        ("cancel_task", vec![ApiScope::Workspace, ApiScope::Project])
                    }
                    OperationName::RunScenario => ("run_scenario", vec![ApiScope::Project]),
                    OperationName::ComputePackaging => {
                        ("compute_packaging", vec![ApiScope::Project])
                    }
                    OperationName::GetRun => ("get_run", vec![ApiScope::Project]),
                    OperationName::CompareRuns => ("compare_runs", vec![ApiScope::Project]),
                    OperationName::ExportPackage => ("export_package", vec![ApiScope::Project]),
                    OperationName::ExportEvaluation => {
                        ("export_evaluation", vec![ApiScope::Project])
                    }
                    OperationName::AdoptArtifact => ("adopt_artifact", vec![ApiScope::Project]),
                    OperationName::QueryModuleLibrary => (
                        "query_module_library",
                        vec![ApiScope::Workspace, ApiScope::Project],
                    ),
                    OperationName::CheckFactoryDraft => (
                        "check_factory_draft",
                        vec![ApiScope::Project],
                    ),
                    OperationName::ReadCloudProduction => ("read_cloud_production",vec![ApiScope::Project]),
                    OperationName::ExportExecutionReport => ("export_execution_report",vec![ApiScope::Project]),
                    OperationName::ExportCloudData => ("export_cloud_data",vec![ApiScope::Project]),
                    OperationName::StartExecutionSession => ("start_execution_session",vec![ApiScope::Project]),
                    OperationName::GetExecutionSnapshot => ("get_execution_snapshot",vec![ApiScope::Project]),
                    OperationName::ReadExecutionView => ("read_execution_view",vec![ApiScope::Project]),
                    OperationName::ControlExecutionSession => ("control_execution_session",vec![ApiScope::Project]),
                    _ => unreachable!(),
                };
                if matches!(
                    operation,
                    OperationName::GetContext
                        | OperationName::GetObject
                        | OperationName::ApplyCommands
                        | OperationName::RenderView
                        | OperationName::ComputePackaging
                        | OperationName::RunScenario
                        | OperationName::GetRun
                        | OperationName::GetTask
                        | OperationName::CancelTask
                        | OperationName::ExportPackage
                ) {
                    scopes.extend(
                        self.definition_drafts
                            .keys()
                            .filter(|id| {
                                (matches!(caller.origin, CallerOrigin::InteractiveUi)
                                    || caller.allowed_definition_drafts.contains(id))
                                    && (!matches!(operation, OperationName::RunScenario)
                                        || self.definition_cycle_scope_available(caller, id))
                                    && (!matches!(operation, OperationName::RenderView)
                                        || self.render_scope_available(
                                            caller,
                                            &ApiScope::DefinitionDraft((*id).clone()),
                                        ))
                            })
                            .cloned()
                            .map(ApiScope::DefinitionDraft),
                    );
                }
                if matches!(operation, OperationName::RenderView) {
                    scopes.retain(|scope| self.render_scope_available(caller, scope));
                }
                OperationCapability {
                    operation: operation.clone(),
                    input_schema_ref: format!(
                        "packages/commands/generated/schemas/{name}.request.schema.json"
                    ),
                    output_schema_ref: format!(
                        "packages/commands/generated/schemas/{name}.response.schema.json"
                    ),
                    allowed_scopes: scopes,
                    available_controls: if matches!(operation, OperationName::CancelTask) {
                        vec![TaskControlAction::Cancel]
                    } else {
                        vec![]
                    },
                }
            })
            .collect();
        Ok(CapabilityCatalog {
            api_version: ApiVersion::V01, implemented_operations: operations,
            available_command_kinds: commands, providers: self.providers.clone(), category_ids: vec![ApiCategory::Items, ApiCategory::Devices, ApiCategory::Resources, ApiCategory::Tasks, ApiCategory::Operations, ApiCategory::Transport, ApiCategory::Templates, ApiCategory::Professional, ApiCategory::Media],
            media_kinds: vec![], native_primitives: vec![], max_ipc_frame_bytes: SafeUint::try_from(1_048_576).unwrap(),
            limitations: vec![Issue { code: "P0_01_MINIMAL_HOST".into(), object_ref: None, field_path: None,
                message: "Single active Project; all twelve typed draft objects share atomic edits, session history and actual save/Open tasks. Stable-ID array scalar leaves are supported, array structure changes require UI. Runs use the actual Engine with frozen project/scenario/module versions. Definition cycle samples are session-only and require explicitly mapped executable definitions; other task controls are not implemented. Native transport authenticates peers before registering Host sessions; category labels do not establish complete provider acceptance.".into(), blocks: vec![] }],
        })
    }

    fn available_commands(&self, caller: &HostCallContext) -> Vec<String> {
        if !caller
            .allowed_operations
            .iter()
            .any(|op| matches!(op, OperationName::ApplyCommands))
        {
            return vec![];
        }
        if matches!(caller.origin, CallerOrigin::InteractiveUi) {
            let mut commands = [
                "create_project",
                "set_project_settings",
                "put_object",
                "remove_object",
                "create_definition_draft",
                "put_definition",
                "put_template_composition",
                "set_declared_parameter",
                "commit_definition_version",
                "adopt_module",
                "instantiate_template",
                "put_packaging_job",
                "record_artifact_acknowledgment",
            ]
            .map(str::to_owned)
            .to_vec();
            let history = self.history_for(caller);
            if history.can_undo {
                commands.push("undo".into());
            }
            if history.can_redo {
                commands.push("redo".into());
            }
            if self.tasks.values().any(|task| {
                task.owner == caller.session_id
                    && task.epoch == self.epoch
                    && task.staged.is_some()
                    && matches!(task.state.task.state, ComputationStatus::Completed)
            }) {
                commands.push("adopt_imported_draft".into());
            }
            return commands;
        }
        let Some(project) = self
            .project
            .as_ref()
            .filter(|p| caller.granted_project.as_ref() == Some(&p.id))
        else {
            return vec![];
        };
        let draft = self.current_draft();
        let settings = serde_json::to_value(ProjectSettings {
            title: project.title.clone(),
            time_reference: project.time_reference.clone(),
            generation_context: project.generation_context.clone(),
            sources: project.sources.clone(),
            setup_policies: Some(project.setup_policies.clone()),
        })
        .unwrap();
        let mut commands = vec![];
        for (kind, branch) in [
            ("set_project_settings", HostEditBranch::SetProjectSettings),
            ("put_object", HostEditBranch::PutObject),
            ("remove_object", HostEditBranch::RemoveObject),
        ] {
            let available = caller
                .edit_grants
                .iter()
                .filter(|grant| {
                    self.current_grant(grant) && grant.branches.iter().any(|b| same(b, &branch))
                })
                .any(|grant| {
                    let object_path = grant.objects.iter().any(|object| match branch {
                        HostEditBranch::SetProjectSettings => {
                            matches!(object.target.kind, ObjectKind::Project)
                                && object.target.id.as_str() == project.id.as_str()
                                && object.fields.iter().any(|field| {
                                    editable_existing_leaf(
                                        field,
                                        &settings,
                                        matches!(
                                            field.pointer.as_str(),
                                            "/time_reference/origin_iso8601"
                                                | "/time_reference/timezone"
                                        ) || (field.pointer.starts_with("/setup_policies/")
                                            && field.pointer.ends_with("/initial_family")),
                                    )
                                })
                        }
                        HostEditBranch::PutObject => {
                            supported_kind(&object.target.kind)
                                && draft_value(&draft, &object.target).is_some_and(|value| {
                                    let value = authorization_value(&object.target, value);
                                    object.fields.iter().any(|field| {
                                        field.pointer != "/id"
                                            && editable_existing_leaf(
                                                field,
                                                &value,
                                                matches!(
                                                    field.pointer.as_str(),
                                                    "/time_reference/origin_iso8601"
                                                        | "/time_reference/timezone"
                                                ) || host_item_layout::anchor_leaf(&field.pointer),
                                            )
                                    })
                                })
                        }
                        HostEditBranch::RemoveObject => {
                            supported_kind(&object.target.kind)
                                && object.can_remove
                                && draft_value(&draft, &object.target).is_some()
                        }
                        _ => false,
                    });
                    let creation_path = matches!(branch, HostEditBranch::PutObject)
                        && grant.creates.iter().any(|allowance| {
                            supported_kind(&allowance.kind)
                                && (grant
                                    .created_objects
                                    .iter()
                                    .filter(|object| same(&object.kind, &allowance.kind))
                                    .map(|object| &object.id)
                                    .collect::<BTreeSet<_>>()
                                    .len()
                                    < allowance.max_new_objects.value() as usize
                                    || grant.created_objects.iter().any(|object| {
                                        same(&object.kind, &allowance.kind)
                                            && draft_value(&draft, object).is_none()
                                    }))
                                && creation_path_available(allowance)
                        });
                    object_path || creation_path
                });
            if available {
                commands.push(kind.into());
            }
        }
        let history = self.history_for(caller);
        for command in self.available_definition_commands(caller) {
            if !commands.contains(&command) {
                commands.push(command);
            }
        }
        for (name, branch) in [
            ("adopt_module", HostEditBranch::AdoptModule),
            ("instantiate_template", HostEditBranch::InstantiateTemplate),
        ] {
            if caller.edit_grants.iter().any(|grant| {
                self.current_grant(grant)
                    && grant.branches.iter().any(|actual| same(actual, &branch))
                    && grant.source_versions.iter().any(|reference| {
                        self.adoption_source(reference).is_some()
                            && (!matches!(branch, HostEditBranch::InstantiateTemplate)
                                || matches!(
                                    self.adoption_source(reference).unwrap().definition.payload,
                                    ModulePayload::Templates(_)
                                ))
                    })
            }) {
                commands.push(name.into());
            }
        }
        if caller.edit_grants.iter().any(|grant| {
            self.current_grant(grant)
                && grant
                    .branches
                    .iter()
                    .any(|branch| matches!(branch, HostEditBranch::SetDeclaredParameter))
                && grant.objects.iter().any(|object| {
                    matches!(object.target.kind, ObjectKind::Device)
                        && self
                            .project
                            .as_ref()
                            .unwrap()
                            .devices
                            .iter()
                            .find(|device| device.id == object.target.id)
                            .is_some_and(|device| {
                                self.modules
                                    .get(&module_key(&device.module_ref))
                                    .is_some_and(|module| {
                                        module.definition.parameters.iter().any(|parameter| {
                                            let prefix = format!(
                                                "/configuration/{}",
                                                parameter
                                                    .id
                                                    .as_str()
                                                    .replace('~', "~0")
                                                    .replace('/', "~1")
                                            );
                                            object.fields.iter().any(|field| {
                                                leaf_granted(field, &format!("{prefix}/value"))
                                            }) && (device
                                                .configuration
                                                .contains_key(parameter.id.as_str())
                                                || object.fields.iter().any(|field| {
                                                    leaf_granted(field, &format!("{prefix}/kind"))
                                                }))
                                        })
                                    })
                            })
                })
        }) {
            commands.push("set_declared_parameter".into());
        }
        if caller.edit_grants.iter().any(|grant| {
            self.current_grant(grant)
                && grant
                    .branches
                    .iter()
                    .any(|branch| matches!(branch, HostEditBranch::PutPackagingJob))
                && grant.objects.iter().any(|object| {
                    matches!(object.target.kind, ObjectKind::PackagingJob)
                        && self
                            .professional
                            .jobs
                            .values()
                            .filter(|job| job.id == object.target.id)
                            .max_by_key(|job| job.revision.0.value())
                            .is_some_and(|job| {
                                object.fields.iter().any(|field| {
                                    editable_existing_leaf(
                                        field,
                                        &serde_json::to_value(job).unwrap(),
                                        false,
                                    )
                                })
                            })
                })
        }) {
            commands.push("put_packaging_job".into());
        }
        if history.can_undo {
            commands.push("undo".into());
        }
        if history.can_redo {
            commands.push("redo".into());
        }
        commands
    }

    fn context(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: GetContextParams,
    ) -> Result<ProjectContext, ApiError> {
        let visible = self.project.as_ref().filter(|p| {
            matches!(caller.origin, CallerOrigin::InteractiveUi)
                || caller.granted_project.as_ref() == Some(&p.id)
        });
        if !params.selection.is_empty() {
            let project = visible.ok_or_else(|| {
                error(
                    ErrorCode::NoActiveProject,
                    "No authorized active project",
                    None,
                )
            })?;
            for target in &params.selection {
                lookup(project, &self.scenarios, target)?;
            }
        }
        let resolved = self.response_context(&caller.session_id, scope);
        let mut issues = visible
            .map(|project| project.issues.clone())
            .unwrap_or_default();
        if visible.is_some() {
            if let Some(backing) = &self.backing {
                issues.extend(
                    backing
                        .manifest
                        .missing
                        .iter()
                        .filter(|issue| !issue.code.starts_with("business_inputs."))
                        .cloned(),
                );
            }
        }
        let mut seen_issues = BTreeSet::new();
        issues.retain(|issue| {
            seen_issues.insert(serde_json::to_string(issue).expect("typed issue serializes"))
        });
        Ok(ProjectContext {
            host: HostContextIdentity {
                session_id: caller.session_id.clone(),
                active_project_id: resolved.project_id,
                active_project_revision: resolved.project_revision,
                active_run_id: None,
                selected_scope: resolved.scope,
            },
            project: visible.map(|p| ProjectSummary {
                id: p.id.clone(),
                revision: p.revision.clone(),
                title: p.title.clone(),
                dirty: self.dirty,
            }),
            selected_objects: if params.selection.is_empty() {
                visible
                    .map(|p| p.views.selected_objects.clone())
                    .unwrap_or_default()
            } else {
                params.selection
            },
            view_state: visible.map(|p| p.views.clone()),
            definition: if let ApiScope::DefinitionDraft(id) = scope {
                self.definition_drafts
                    .get(id)
                    .map(|draft| DefinitionSummary {
                        draft_id: id.clone(),
                        revision: draft.revision.clone(),
                        history: Some(draft.history()),
                        definition_ref: ModuleRef {
                            id: draft.definition.id.clone(),
                            version: draft.definition.version.clone(),
                        },
                        issues: { let mut issues = if draft.dirty { vec![Issue { code: "definition.unsaved_changes".into(), object_ref: Some(ObjectRef { kind: ObjectKind::DefinitionDraft, id: ObjectId::try_from(id.as_str().to_owned()).unwrap() }), field_path: None, message: "This editable definition is not a committed local version; commit or explicitly discard it before switching project".into(), blocks: vec![] }] } else { vec![] }; if let Some(composition)=&draft.composition {issues.extend(composition.issues.clone());} issues },
                    })
            } else {
                None
            },
            tasks: self
                .tasks
                .values()
                .filter(|task| self.owned_task(caller, scope, &task.state.task.id).is_ok())
                .map(|task| current_task_state(task).task)
                .collect(),
            authorized_operations: implemented_operations()
                .into_iter()
                .filter(|op| caller.allowed_operations.iter().any(|a| same(a, op)))
                .filter(|op| {
                    matches!(caller.origin, CallerOrigin::InteractiveUi)
                        || !matches!(op, OperationName::ImportPackage)
                })
                .filter(|op| {
                    !matches!(op, OperationName::AdoptArtifact)
                        || self.artifact_path_available(caller)
                })
                .collect(),
            issues,
            run_summaries: if !matches!(scope, ApiScope::DefinitionDraft(_))
                && caller
                    .allowed_operations
                    .iter()
                    .any(|operation| matches!(operation, OperationName::GetRun))
            {
                self.runs
                    .values()
                    .filter(|run| {
                        visible.is_some_and(|project| {
                            matches!(&run.scope, EditScope::Project(id) if id == &project.id)
                                && run.result.input_ref.project_id == project.id
                        })
                    })
                    .map(|run| run_summary(&run.result))
                    .collect()
            } else {
                vec![]
            },
        })
    }

    fn object(
        &self,
        caller: &HostCallContext,
        scope: &ApiScope,
        mut params: GetObjectParams,
    ) -> Result<DomainObject, ApiError> {
        if let Some(index) = params.node_index.take() {
            let parent = self.object(caller, scope, params)?;
            return host_scene_node::read(&parent, &index);
        }
        if let Some(object) = self.frozen_definition_cycle_object(caller, scope, &params)? {
            return Ok(object);
        }
        if let ObjectSource::TemplatePreview {
            module_ref,
            base_revision,
            choices,
            seed,
        } = &params.source
        {
            if let ApiScope::DefinitionDraft(id) = scope {
                return self
                    .composition_preview(
                        caller,
                        id,
                        &params.object_id,
                        module_ref,
                        base_revision,
                        choices.as_ref(),
                        seed.as_ref(),
                    )
                    .map(|preview| DomainObject::TemplatePreview(Box::new(preview)));
            }
            return self
                .template_preview(
                    scope,
                    &params.object_id,
                    module_ref,
                    base_revision,
                    choices.as_ref(),
                    seed.as_ref(),
                )
                .map(|preview| DomainObject::TemplatePreview(Box::new(preview)));
        }
        if let ObjectSource::RunModuleVersion {
            run_id,
            at_ms,
            module_ref,
        } = &params.source
        {
            let summary = self.frozen_run_summary(caller, scope, run_id, at_ms)?;
            if !matches!(params.object_id.kind, ObjectKind::Module)
                || params.object_id.id.as_str() != module_ref.id.as_str()
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Historical module reads require a matching Module object ID",
                    Some("/params/source"),
                ));
            }
            return Ok(DomainObject::Module(Box::new(
                self.frozen_run_module(&summary, module_ref)?,
            )));
        }
        if let ObjectSource::ModuleVersion { module_ref } = &params.source {
            if !matches!(scope, ApiScope::Project | ApiScope::DefinitionDraft(_))
                || !matches!(params.object_id.kind, ObjectKind::Module)
                || params.object_id.id.as_str() != module_ref.id.as_str()
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Exact module reads require Project or DefinitionDraft scope and a matching Module object ID",
                    Some("/params/source"),
                ));
            }
            if matches!(scope, ApiScope::DefinitionDraft(_)) {
                self.admit(
                    caller,
                    &self.project.as_ref().map(|project| project.id.clone()),
                    &ApiScope::Project,
                    &OperationName::GetObject,
                )?;
            }
            return self
                .scoped_media_package(scope, module_ref)
                .map_err(|mut failure| {
                    for field in &mut failure.field_errors {
                        if field.pointer == "/asset" {
                            field.pointer = "/params/source/data/module_ref".into();
                        }
                    }
                    failure
                })
                .map(|(package, _)| DomainObject::Module(Box::new(package)));
        }
        if let ObjectSource::RunSnapshot { run_id, at_ms } = params.source {
            if matches!(
                params.object_id.kind,
                ObjectKind::Project
                    | ObjectKind::Scenario
                    | ObjectKind::Device
                    | ObjectKind::Resource
            ) {
                let summary = self.frozen_run_summary(caller, scope, &run_id, &at_ms)?;
                if matches!(
                    params.object_id.kind,
                    ObjectKind::Device | ObjectKind::Resource
                ) {
                    // Static declarations frozen in the Run input, not runtime
                    // pose/resource snapshots or the current editable Project.
                    let frozen = self.frozen_run_project(&summary)?;
                    return lookup(&frozen, &BTreeMap::new(), &params.object_id);
                }
                if matches!(params.object_id.kind, ObjectKind::Project) {
                    if params.object_id.id.as_str() != summary.input.project_id.as_str() {
                        return Err(missing(
                            &params.object_id,
                            "Project is not this Run's frozen input",
                        ));
                    }
                    return Ok(DomainObject::Project(Box::new(
                        self.frozen_run_project(&summary)?,
                    )));
                }
                if params.object_id.id.as_str() != summary.input.scenario_id.as_str() {
                    return Err(missing(
                        &params.object_id,
                        "Scenario is not this Run's frozen input",
                    ));
                }
                return Ok(DomainObject::Scenario(Box::new(
                    self.frozen_run_scenario(&summary)?,
                )));
            }
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
            let RunView::Objects(mut values) = self.get_run(
                caller,
                scope,
                GetRunParams {
                    run_id,
                    query: RunQuery::ObjectsAt {
                        at_ms,
                        objects: vec![params.object_id.clone()],
                        page: PageRequest {
                            after_index: None,
                            limit: SafeUint::try_from(1).unwrap(),
                        },
                    },
                },
            )?
            else {
                unreachable!()
            };
            return values.objects.pop().ok_or_else(|| {
                missing(
                    &params.object_id,
                    "Object does not exist at this computed Run time",
                )
            });
        }
        if let ApiScope::DefinitionDraft(id) = scope {
            let draft = self
                .definition_drafts
                .get(id)
                .expect("admitted definition draft");
            if matches!(params.object_id.kind, ObjectKind::DefinitionDraft)
                && params.object_id.id.as_str() == id.as_str()
            {
                return Ok(DomainObject::Definition(Box::new(draft.definition.clone())));
            }
            if matches!(params.object_id.kind, ObjectKind::PackagingJob) {
                if let Some(job) = draft
                    .professional
                    .jobs
                    .values()
                    .filter(|job| job.id == params.object_id.id)
                    .max_by_key(|job| job.revision.0.value())
                {
                    return Ok(DomainObject::PackagingJob(Box::new(job.clone())));
                }
            }
            if matches!(params.object_id.kind, ObjectKind::Artifact) {
                if let Some(artifact) = draft
                    .professional
                    .artifacts
                    .values()
                    .find(|artifact| artifact.id.as_str() == params.object_id.id.as_str())
                {
                    return Ok(DomainObject::Artifact(artifact.clone()));
                }
            }
            return Err(missing(
                &params.object_id,
                "Object is not in this definition draft",
            ));
        }
        if matches!(params.object_id.kind, ObjectKind::Module) {
            let selected: Vec<_> = self
                .modules
                .values()
                .filter(|module| {
                    module.definition.id.as_str() == params.object_id.id.as_str()
                        && self
                            .project
                            .as_ref()
                            .unwrap()
                            .module_refs
                            .iter()
                            .any(|reference| {
                                reference.id == module.definition.id
                                    && reference.version == module.definition.version
                            })
                })
                .collect();
            return if selected.len() == 1 {
                Ok(DomainObject::Module(Box::new(selected[0].clone())))
            } else {
                if selected.is_empty() {
                    Err(missing(
                        &params.object_id,
                        "Module ID has no selected exact version",
                    ))
                } else {
                    Err(error(
                        ErrorCode::InvalidInput,
                        "Module ID does not resolve to one selected exact version",
                        Some("/params/object_id"),
                    ))
                }
            };
        }
        if matches!(params.object_id.kind, ObjectKind::PackagingJob) {
            if let Some(job) = self
                .professional
                .jobs
                .values()
                .filter(|job| job.id == params.object_id.id)
                .max_by_key(|job| job.revision.0.value())
            {
                return Ok(DomainObject::PackagingJob(Box::new(job.clone())));
            }
            return Err(missing(
                &params.object_id,
                "PackagingJob does not exist in the Project",
            ));
        }
        if matches!(params.object_id.kind, ObjectKind::Artifact) {
            if let Some(artifact) = self
                .professional
                .artifacts
                .values()
                .find(|artifact| artifact.id.as_str() == params.object_id.id.as_str())
            {
                return Ok(DomainObject::Artifact(artifact.clone()));
            }
            return Err(missing(
                &params.object_id,
                "Artifact does not exist in the Project",
            ));
        }
        lookup(
            self.project.as_ref().expect("admitted Project scope"),
            &self.scenarios,
            &params.object_id,
        )
    }

    fn refresh_current_business(&self, draft: &mut ProjectDraft) {
        let definitions: Vec<_> = self
            .modules
            .values()
            .map(|package| package.definition.clone())
            .collect();
        refresh_business_issues_with_definitions(draft, &definitions);
    }

    fn execution_draft(
        &self,
        scenario: &ScenarioInput,
        definitions: &[ModuleDefinition],
    ) -> ProjectDraft {
        let mut draft = self.current_draft();
        draft.scenarios.retain(|id, _| id == &scenario.id);
        // Recompute the same shared checks with just the selected Scenario.
        // Keep manual shared/selected issues and all shared object declarations;
        // the full editable Project retains diagnostics for every Scenario.
        draft.project.issues.retain(|issue| {
            !issue.object_ref.as_ref().is_some_and(|target| {
                matches!(target.kind, ObjectKind::Scenario)
                    && target.id.as_str() != scenario.id.as_str()
            })
        });
        let declared_refs = draft.project.scenario_refs.clone();
        draft.project.scenario_refs.retain(|id| id == &scenario.id);
        refresh_business_issues_with_definitions(&mut draft, definitions);
        // Only diagnostics are projected. The frozen Project keeps its real
        // declared indexes and exact business inputs at the admitted revision.
        draft.project.scenario_refs = declared_refs;
        draft
    }

    fn current_grant(&self, grant: &HostEditGrant) -> bool {
        matches!(&grant.scope, TaskScope::Edit(EditScope::Project(id)) if self.project.as_ref().is_some_and(|p| &p.id == id))
    }

    fn next_document_directory(&mut self, purpose: &str) -> String {
        loop {
            let path = format!("documents/{purpose}-{:08}", self.next_document);
            self.next_document += 1;
            let prefix = format!("{path}/");
            if !self.backing.as_ref().is_some_and(|archive| {
                archive.files.keys().any(|existing| {
                    existing.to_lowercase() == path || existing.to_lowercase().starts_with(&prefix)
                })
            }) && !self.additions.iter().any(|addition| {
                addition.entry.file_ref.path.as_str().to_lowercase() == path
                    || addition
                        .entry
                        .file_ref
                        .path
                        .as_str()
                        .to_lowercase()
                        .starts_with(&prefix)
            }) {
                return path;
            }
        }
    }

    fn apply(
        &mut self,
        caller: &HostCallContext,
        scope: &ApiScope,
        params: ApplyCommandsParams,
    ) -> Result<EditCommit, ApiError> {
        if matches!(scope, ApiScope::Workspace) {
            return self.create(caller, params);
        }
        if let ApiScope::DefinitionDraft(id) = scope {
            return self.apply_definition(caller, id, params);
        }
        let before = self.current_draft();
        let supplied = params.base_revision.as_ref().ok_or_else(|| {
            error(
                ErrorCode::InvalidInput,
                "Project edits require base_revision",
                Some("/params/base_revision"),
            )
        })?;
        check_revision(supplied, &before.project.revision, "/params/base_revision")?;
        if let [EditCommand::AdoptImportedDraft { task_id, choices }] = params.commands.as_slice() {
            return self.adopt_imported_modules(caller, task_id, choices);
        }
        if let [EditCommand::CreateDefinitionDraft(definition)] = params.commands.as_slice() {
            return self.create_definition(caller, definition.clone());
        }
        if params.commands.is_empty() {
            return Err(error(
                ErrorCode::InvalidInput,
                "An edit batch must contain a command",
                Some("/params/commands"),
            ));
        }
        if params.commands.len() == 1 {
            match &params.commands[0] {
                EditCommand::Undo => return self.apply_history(caller, true),
                EditCommand::Redo => return self.apply_history(caller, false),
                _ => {}
            }
        }
        let mut candidate = before.clone();
        let mut ledger = caller.clone();
        let mut changed = vec![];
        let mut deferred_failure = None;
        let mut acknowledged_ids = BTreeSet::new();
        let mut library_sources = vec![];
        for command in params.commands {
            match command {
                EditCommand::SetProjectSettings(mut settings) => {
                    if settings.setup_policies.is_none() {
                        settings.setup_policies = Some(candidate.project.setup_policies.clone());
                    }
                    let target = ObjectRef {
                        kind: ObjectKind::Project,
                        id: ObjectId::try_from(candidate.project.id.as_str().to_owned()).unwrap(),
                    };
                    let old = settings_value(&candidate.project);
                    let new = serde_json::to_value(&settings).unwrap();
                    if !effect_admitted(
                        self.authorize_update(
                            &ledger,
                            &HostEditBranch::SetProjectSettings,
                            &target,
                            &old,
                            &new,
                        ),
                        &mut deferred_failure,
                    )? {
                        continue;
                    }
                    candidate.project.title = settings.title;
                    candidate.project.time_reference = settings.time_reference;
                    candidate.project.generation_context = settings.generation_context;
                    candidate.project.sources = settings.sources;
                    candidate.project.setup_policies = settings.setup_policies.unwrap();
                    push_ref(&mut changed, target);
                }
                EditCommand::PutObject(object) if editable_target(&object).is_some() => {
                    let target = editable_target(&object).unwrap();
                    let new = editable_object_value(&object);
                    let effect = if let Some(old) = draft_value(&candidate, &target) {
                        self.authorize_update(
                            &ledger,
                            &HostEditBranch::PutObject,
                            &target,
                            &authorization_value(&target, old),
                            &authorization_value(&target, new.clone()),
                        )
                    } else {
                        self.authorize_create(
                            &mut ledger,
                            &target,
                            &authorization_value(&target, new.clone()),
                        )
                    };
                    if !effect_admitted(effect, &mut deferred_failure)? {
                        continue;
                    }
                    // A deliberate configuration-key removal restores the selected
                    // definition. Remove only that override's paired source entry;
                    // changed values must still use SetDeclaredParameter.
                    let removed_parameter_paths: Vec<String> = match &object {
                        EditableProjectObject::Device(updated) => candidate.project.devices.iter()
                            .find(|device| device.id == updated.id)
                            .into_iter().flat_map(|device| device.configuration.keys())
                            .filter(|key| !updated.configuration.contains_key(*key))
                            .map(|key| format!("/configuration/{}/value", key.replace('~', "~0").replace('/', "~1")))
                            .collect(),
                        _ => vec![],
                    };
                    if candidate.project.parameter_sources.iter().any(|source| {
                        same_ref(&source.target, &target)
                            && !removed_parameter_paths.contains(&source.field_path)
                            && source.value.as_ref().is_some_and(|value| {
                                new.pointer(&source.field_path)
                                    != serde_json::to_value(value).unwrap().get("value")
                            })
                    }) {
                        deferred_failure.get_or_insert_with(|| error(ErrorCode::InvalidInput, "PutObject would leave a stored parameter source attached to a different value; use SetDeclaredParameter", Some("/params/commands")));
                        continue;
                    }
                    if let EditableProjectObject::Scenario(scenario) = &object {
                        let expected = candidate
                            .scenarios
                            .get(&scenario.id)
                            .map(|old| old.revision.0.value())
                            .unwrap_or(1);
                        if scenario.revision.0.value() != expected
                            || scenario.project_revision.0.value()
                                != before.project.revision.0.value()
                        {
                            if deferred_failure.is_none() {
                                deferred_failure = Some(error(
                                    ErrorCode::InvalidInput,
                                    "Scenario metadata must name current Scenario revision (new=1) and this Project base_revision",
                                    Some("/params/commands"),
                                ));
                            }
                            continue;
                        }
                    }
                    candidate.project.parameter_sources.retain(|source| {
                        !same_ref(&source.target, &target)
                            || !removed_parameter_paths.contains(&source.field_path)
                    });
                    put_draft_object(&mut candidate, object);
                    push_ref(&mut changed, target);
                }
                EditCommand::SetDeclaredParameter {
                    target,
                    parameter_id,
                    value,
                    basis,
                } => {
                    let Some(index) = candidate.project.devices.iter().position(|device| {
                        matches!(target.kind, ObjectKind::Device) && device.id == target.id
                    }) else {
                        self.authorize_branch(&ledger, &HostEditBranch::SetDeclaredParameter)?;
                        deferred_failure.get_or_insert_with(|| missing(&target, "Declared instance parameter requires an existing Device configuration"));
                        continue;
                    };
                    let device = &candidate.project.devices[index];
                    let Some(module) = self.modules.get(&module_key(&device.module_ref)) else {
                        self.authorize_update(
                            &ledger,
                            &HostEditBranch::SetDeclaredParameter,
                            &target,
                            &Value::Null,
                            &Value::Null,
                        )?;
                        deferred_failure.get_or_insert_with(|| {
                            missing(&target, "Exact selected module definition is not loaded")
                        });
                        continue;
                    };
                    if !effect_admitted(
                        host_definitions::instance_parameter_value(
                            &module.definition,
                            &parameter_id,
                            &value,
                        )
                        .map(|_| ()),
                        &mut deferred_failure,
                    )? {
                        self.authorize_update(
                            &ledger,
                            &HostEditBranch::SetDeclaredParameter,
                            &target,
                            &Value::Null,
                            &Value::Null,
                        )?;
                        continue;
                    }
                    let old = serde_json::to_value(device).unwrap();
                    let mut updated = device.clone();
                    updated
                        .configuration
                        .insert(parameter_id.as_str().into(), value.clone());
                    let new = serde_json::to_value(&updated).unwrap();
                    if !effect_admitted(
                        self.authorize_update(
                            &ledger,
                            &HostEditBranch::SetDeclaredParameter,
                            &target,
                            &old,
                            &new,
                        ),
                        &mut deferred_failure,
                    )? {
                        continue;
                    }
                    let path = format!(
                        "/configuration/{}/value",
                        parameter_id.as_str().replace('~', "~0").replace('/', "~1")
                    );
                    self.authorize_parameter_source(
                        &ledger,
                        &HostEditBranch::SetDeclaredParameter,
                        &target,
                        &path,
                        &new,
                    )?;
                    candidate.project.devices[index] = updated;
                    candidate.project.parameter_sources.retain(|source| {
                        !same_ref(&source.target, &target) || source.field_path != path
                    });
                    candidate.project.parameter_sources.push(ParameterSource {
                        target: target.clone(),
                        field_path: path,
                        value: Some(value),
                        basis,
                        source_ref: None,
                    });
                    push_ref(&mut changed, target);
                }
                EditCommand::RemoveObject(target) if supported_kind(&target.kind) => {
                    self.authorize_remove(&ledger, &target)?;
                    if !remove_draft_object(&mut candidate, &target) && deferred_failure.is_none() {
                        deferred_failure = Some(missing(&target, "Object does not exist"));
                    }
                    candidate
                        .project
                        .parameter_sources
                        .retain(|source| !same_ref(&source.target, &target));
                    push_ref(&mut changed, target);
                }
                EditCommand::PutPackagingJob(job) => {
                    let target = ObjectRef {
                        kind: ObjectKind::PackagingJob,
                        id: job.id.clone(),
                    };
                    let value = serde_json::to_value(job.as_ref()).unwrap();
                    let old = candidate
                        .professional
                        .jobs
                        .values()
                        .filter(|old| old.id == job.id)
                        .max_by_key(|old| old.revision.0.value());
                    let authorized = if let Some(old) = old {
                        self.authorize_update(
                            &ledger,
                            &HostEditBranch::PutPackagingJob,
                            &target,
                            &serde_json::to_value(old).unwrap(),
                            &value,
                        )
                    } else {
                        self.authorize_create_as(
                            &mut ledger,
                            &target,
                            &value,
                            &HostEditBranch::PutPackagingJob,
                        )
                    };
                    if effect_admitted(authorized, &mut deferred_failure)? {
                        if effect_admitted(
                            self.check_packaging_job_reservation(&ApiScope::Project, &job),
                            &mut deferred_failure,
                        )? {
                            effect_admitted(
                                put_packaging_job(&mut candidate, *job),
                                &mut deferred_failure,
                            )?;
                        }
                    }
                    push_ref(&mut changed, target);
                }
                EditCommand::AdoptModule(adoption) => {
                    let source_ref = adoption.module_ref.clone();
                    match self.apply_adoption_candidate(
                        &mut ledger,
                        &mut candidate,
                        &HostEditBranch::AdoptModule,
                        adoption,
                    ) {
                        Ok(objects) => {
                            library_sources.push(source_ref);
                            for object in objects {
                                push_ref(&mut changed, object);
                            }
                        }
                        Err(failure) => {
                            effect_admitted(Err(failure), &mut deferred_failure)?;
                        }
                    }
                }
                EditCommand::InstantiateTemplate(instantiation) => {
                    let source_ref = instantiation.template_ref.clone();
                    self.authorize_branch(&ledger, &HostEditBranch::InstantiateTemplate)?;
                    if let Some(policies) = &instantiation.project_setup_policies {
                        if !exact_data(policies, &candidate.project.setup_policies) {
                            let target = ObjectRef {
                                kind: ObjectKind::Project,
                                id: ObjectId::try_from(candidate.project.id.as_str().to_owned())
                                    .unwrap(),
                            };
                            let old = settings_value(&candidate.project);
                            let mut new = old.clone();
                            new["setup_policies"] = serde_json::to_value(policies).unwrap();
                            effect_admitted(
                                self.authorize_update(
                                    &ledger,
                                    &HostEditBranch::SetProjectSettings,
                                    &target,
                                    &old,
                                    &new,
                                ),
                                &mut deferred_failure,
                            )?;
                            push_ref(&mut changed, target);
                        }
                    }
                    let prepared =
                        self.prepare_template_instantiation(&candidate.project, &instantiation);
                    let adoption = ModuleAdoption {
                        module_ref: instantiation.template_ref,
                        targets: vec![],
                        bindings: instantiation.mapped_bindings,
                        new_instances: instantiation.proposed_objects,
                        explicit_seed: instantiation.explicit_new_scenario_seed,
                    };
                    match self.apply_adoption_candidate(
                        &mut ledger,
                        &mut candidate,
                        &HostEditBranch::InstantiateTemplate,
                        adoption,
                    ) {
                        Ok(objects) => {
                            library_sources.push(source_ref);
                            for object in objects {
                                push_ref(&mut changed, object);
                            }
                            match prepared {
                                Ok(prepared) => {
                                    if let Some(policies) = prepared.project_setup_policies {
                                        candidate.project.setup_policies = policies;
                                    }
                                    for reference in prepared.proposed_project.module_refs {
                                        if !candidate.project.module_refs.iter().any(|current| {
                                            module_key(current) == module_key(&reference)
                                        }) {
                                            candidate.project.module_refs.push(reference);
                                        }
                                    }
                                    candidate
                                        .project
                                        .template_adoptions
                                        .extend(prepared.proposed_project.template_adoptions);
                                    candidate
                                        .project
                                        .sources
                                        .extend(prepared.proposed_project.sources);
                                    candidate
                                        .project
                                        .parameter_sources
                                        .extend(prepared.proposed_project.parameter_sources);
                                }
                                Err(failure) => {
                                    effect_admitted(Err(failure), &mut deferred_failure)?;
                                }
                            }
                        }
                        Err(failure) => {
                            effect_admitted(Err(failure), &mut deferred_failure)?;
                        }
                    }
                }
                EditCommand::RecordArtifactAcknowledgment(ack) => {
                    if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
                        return Err(error(
                            ErrorCode::PermissionDenied,
                            "Only the actual UI session can record a human acknowledgment",
                            None,
                        ));
                    }
                    let target = ObjectRef {
                        kind: ObjectKind::Artifact,
                        id: ObjectId::try_from(ack.artifact_ref.as_str().to_owned()).unwrap(),
                    };
                    match prepare_acknowledgment(&candidate, ack) {
                        Ok(ack) => {
                            acknowledged_ids.insert(ack.id.clone());
                            if !candidate
                                .project
                                .acknowledgments
                                .iter()
                                .any(|old| old.id == ack.id)
                            {
                                candidate.project.acknowledgments.push(ack);
                            }
                        }
                        Err(failure) => {
                            effect_admitted(Err(failure), &mut deferred_failure)?;
                        }
                    }
                    push_ref(&mut changed, target);
                }
                unsupported_command => {
                    self.authorize_branch(&ledger, &branch(&unsupported_command))?;
                    if deferred_failure.is_none() {
                        deferred_failure = Some(unsupported(
                            "Command or editable object kind is not implemented in this slice",
                        ));
                    }
                }
            }
        }
        if let Some(failure) = deferred_failure {
            return Err(failure);
        }
        let mut library_imports = LibraryImports::default();
        for reference in library_sources {
            self.stage_library_import(&reference, &mut candidate, &mut library_imports)?;
        }
        validate_candidate(&candidate)?;
        let highwater = finalize_draft(&mut candidate, &self.scenario_highwater)?;
        let mut definitions = self.modules.clone();
        definitions.extend(library_imports.modules.clone());
        refresh_business_issues_with_definitions(
            &mut candidate,
            &definitions
                .values()
                .map(|package| package.definition.clone())
                .collect::<Vec<_>>(),
        );
        for scenario in candidate.scenarios.values() {
            push_ref(
                &mut changed,
                ObjectRef {
                    kind: ObjectKind::Scenario,
                    id: ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap(),
                },
            );
        }
        self.history.record(&before, &candidate);
        if !acknowledged_ids.is_empty() {
            self.history.professional_boundary(&candidate);
        }
        let mut result = EditCommit {
            project_id: candidate.project.id.clone(),
            scope: EditScope::Project(candidate.project.id.clone()),
            previous_revision: Some(before.project.revision),
            new_revision: candidate.project.revision.clone(),
            changed_objects: changed,
            issues: candidate.project.issues.clone(),
            history: self.history.availability(),
        };
        self.project = Some(candidate.project);
        self.scenarios = candidate.scenarios;
        self.professional = candidate.professional;
        self.modules.extend(library_imports.modules);
        self.additions
            .extend(library_imports.additions.into_values());
        self.content_generation += 1;
        self.scenario_highwater = highwater;
        self.dirty = true;
        self.sessions.insert(ledger.session_id.clone(), ledger);
        for id in acknowledged_ids {
            self.professional_trust.record_human_acknowledgment(id);
        }
        result.history = self.history_for(caller);
        Ok(result)
    }

    fn create(
        &mut self,
        caller: &HostCallContext,
        params: ApplyCommandsParams,
    ) -> Result<EditCommit, ApiError> {
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Only actual UI may create a project",
                None,
            ));
        }
        if params.base_revision.is_some() {
            return Err(error(
                ErrorCode::InvalidInput,
                "Workspace creation requires base_revision=null",
                Some("/params/base_revision"),
            ));
        }
        if let [EditCommand::AdoptImportedDraft { task_id, choices }] = params.commands.as_slice() {
            return self.adopt_open(caller, task_id, choices);
        }
        let mut commands = params.commands.into_iter();
        let Some(EditCommand::CreateProject(new)) = commands.next() else {
            return Err(unsupported(
                "Workspace accepts only one CreateProject command",
            ));
        };
        if commands.next().is_some() {
            return Err(error(
                ErrorCode::InvalidInput,
                "CreateProject must be the only command",
                Some("/params/commands"),
            ));
        }
        self.check_departure(&new.departure)?;
        let id = ProjectId::try_from(format!(
            "{}:project:{}",
            self.host_instance_id.as_str(),
            self.next_project
        ))
        .unwrap();
        let project = empty_project(id.clone(), new);
        self.history.reset(&ProjectDraft {
            project: project.clone(),
            scenarios: BTreeMap::new(),
            professional: ProfessionalDocuments::default(),
        });
        let result = EditCommit {
            project_id: id.clone(),
            scope: EditScope::Project(id.clone()),
            previous_revision: None,
            new_revision: project.revision.clone(),
            changed_objects: vec![ObjectRef {
                kind: ObjectKind::Project,
                id: ObjectId::try_from(id.as_str().to_owned()).unwrap(),
            }],
            issues: vec![],
            history: self.history.availability(),
        };
        self.next_project += 1;
        self.project = Some(project);
        self.scenarios.clear();
        self.scenario_highwater.clear();
        self.dirty = true;
        self.backing = None;
        self.modules.clear();
        self.runs.clear();
        self.definition_cycle_inputs.clear();
        self.additions.clear();
        self.definition_drafts.clear();
        self.professional = ProfessionalDocuments::default();
        self.professional_trust.clear();
        self.asset_confirmations.clear();
        self.content_generation += 1;
        self.switch_epoch(&id);
        Ok(result)
    }

    fn check_departure(&self, departure: &DepartureDecision) -> Result<(), ApiError> {
        if !self.execution_blockers().is_empty() {
            return Err(error(ErrorCode::InvalidInput, "生产、结果发布及设备退出须实际结清后才能切换工程；丢弃草稿不能跳过。", None));
        }
        if self
            .tasks
            .values()
            .any(|task| !terminal(&task.state.task.state))
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Active tasks must reach a real terminal state before switching project",
                None,
            ));
        }
        if self.project.is_some()
            && (self.dirty || self.definition_drafts.values().any(|draft| draft.dirty))
            && !matches!(departure, DepartureDecision::DiscardAfterTasksTerminal)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Unsaved project or definition changes require commit/save or explicit discard; Stay and AlreadySaved do not discard",
                Some("/params/commands/0/data/departure"),
            ));
        }
        Ok(())
    }

    fn switch_epoch(&mut self, id: &ProjectId) {
        self.epoch += 1;
        self.executions.clear();
        self.execution_preparation = host_execution_preparation::PreparationRegistry::default();
        self.definition_cycle_inputs.clear();
        self.definition_drafts.clear();
        self.packaging_inputs.clear();
        self.selections.clear();
        self.external_task_access.clear();
        // Changing the active project revokes all old external sessions and file grants.
        self.revoked_external_sessions.extend(
            self.sessions
                .iter()
                .filter(|(_, session)| matches!(session.origin, CallerOrigin::ExternalAgent))
                .map(|(id, _)| id.clone()),
        );
        self.sessions
            .retain(|_, session| matches!(session.origin, CallerOrigin::InteractiveUi));
        for session in self.sessions.values_mut() {
            session.granted_project = Some(id.clone());
            session.selected_file_tokens.clear();
            session.edit_grants.clear();
        }
        self.refresh_render_controls();
    }

    fn adopt_open(
        &mut self,
        caller: &HostCallContext,
        task_id: &TaskId,
        choices: &ImportAcceptanceChoices,
    ) -> Result<EditCommit, ApiError> {
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Open adoption requires actual UI",
                None,
            ));
        }
        if !matches!(choices.chosen_policy, ImportPolicy::OpenProject) {
            return Err(unsupported(
                "Only complete OpenProject adoption is implemented",
            ));
        }
        if !choices.reused_bindings.item_types.is_empty()
            || !choices.reused_bindings.resources.is_empty()
            || !choices.reused_bindings.targets.is_empty()
            || !choices.reused_bindings.ports.is_empty()
        {
            return Err(unsupported(
                "OpenProject preserves the complete archived project; subset/remapping choices are not implemented",
            ));
        }
        let task = self.owned_task(caller, &ApiScope::Workspace, task_id)?;
        if task.epoch != self.epoch
            || !matches!(
                task.state.originating_operation,
                OperationName::ImportPackage
            )
            || !matches!(task.state.task.state, ComputationStatus::Completed)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Open task must be completed in this session and current context",
                None,
            ));
        }
        let archive = task
            .staged
            .as_ref()
            .ok_or_else(|| {
                error(
                    ErrorCode::InvalidInput,
                    "Task has no staged project archive",
                    None,
                )
            })?
            .clone();
        self.validate_execution_archive_prefix(&archive)?;
        let mut adopted = ProjectDraft {
            project: archive.project.clone(),
            scenarios: archive.scenarios.clone(),
            professional: archive.professional.clone(),
        };
        validate_candidate(&adopted)?;
        let definitions: Vec<_> = archive
            .modules()?
            .into_iter()
            .map(|package| package.definition)
            .collect();
        refresh_business_issues_with_definitions(&mut adopted, &definitions);
        let Some(TaskOutput::Import(draft)) = &task.state.result else {
            return Err(error(
                ErrorCode::InvalidInput,
                "Completed task has no import proposal",
                None,
            ));
        };
        let assets: Vec<_> = archive
            .manifest
            .assets
            .iter()
            .map(|asset| asset.asset_ref.clone())
            .collect();
        if !confirms_complete_or_unspecified(&choices.objects_to_create, &draft.proposed_objects)
            || !confirms_complete_or_unspecified(
                &choices.module_versions,
                &draft.proposed_definitions,
            )
            || !confirms_complete_or_unspecified(&choices.selected_assets, &assets)
        {
            return Err(unsupported(
                "OpenProject accepts an empty selection or exact complete proposal confirmation; subsets, additions and duplicate identities are not implemented",
            ));
        }
        if !import_preview_missing(&archive, &adopted).is_empty()
            && !choices.accept_missing_as_draft
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Open project contains declared missing/blocking draft issues; explicit accept_missing_as_draft is required",
                Some("/params/commands/0/data/choices/accept_missing_as_draft"),
            ));
        }
        self.check_departure(&choices.departure)?;
        let modules = archive
            .modules()?
            .into_iter()
            .map(|module| {
                (
                    module_key(&ModuleRef {
                        id: module.definition.id.clone(),
                        version: module.definition.version.clone(),
                    }),
                    module,
                )
            })
            .collect();
        let runs = archived_runs(&archive)?;
        let project = adopted.project.clone();
        let id = project.id.clone();
        self.history.reset(&adopted);
        let result = EditCommit {
            project_id: id.clone(),
            scope: EditScope::Project(id.clone()),
            previous_revision: None,
            new_revision: project.revision.clone(),
            changed_objects: vec![ObjectRef {
                kind: ObjectKind::Project,
                id: ObjectId::try_from(id.as_str().to_owned()).unwrap(),
            }],
            issues: project.issues.clone(),
            history: self.history.availability(),
        };
        self.project = Some(project);
        self.scenario_highwater = archive_scenario_highwater(&archive);
        self.scenarios = adopted.scenarios;
        self.professional = adopted.professional;
        self.professional_trust.clear();
        self.asset_confirmations.clear();
        self.modules = modules;
        self.runs = runs;
        self.additions.clear();
        self.content_generation += 1;
        self.backing = Some(archive);
        self.dirty = false;
        self.switch_epoch(&id);
        Ok(result)
    }

    fn history_for(&self, caller: &HostCallContext) -> HistoryAvailability {
        let caller = self.sessions.get(&caller.session_id).unwrap_or(caller);
        let mut availability = self.history.availability();
        if matches!(caller.origin, CallerOrigin::ExternalAgent) {
            let can = |undo| {
                self.project.as_ref().is_some_and(|_| {
                    let current = self.current_draft();
                    let candidate = if undo {
                        self.history.undo_candidate(&current)
                    } else {
                        self.history.redo_candidate(&current)
                    };
                    candidate.is_ok_and(|candidate| {
                        self.authorize_history(
                            caller,
                            &current,
                            &candidate,
                            if undo {
                                &HostEditBranch::Undo
                            } else {
                                &HostEditBranch::Redo
                            },
                        )
                        .is_ok()
                    })
                })
            };
            availability.can_undo &= can(true);
            availability.can_redo &= can(false);
            if !availability.can_undo && !availability.can_redo {
                availability.boundary_reason =
                    Some("No history action is currently authorized for this session".into());
            }
        }
        availability
    }

    fn authorize_history(
        &self,
        caller: &HostCallContext,
        before: &ProjectDraft,
        after: &ProjectDraft,
        branch: &HostEditBranch,
    ) -> Result<(HostCallContext, Vec<ObjectRef>), ApiError> {
        self.authorize_branch(caller, branch)?;
        let mut ledger = caller.clone();
        let mut changed = vec![];
        let mut deferred_failure = None;
        let old = settings_value(&before.project);
        let new = settings_value(&after.project);
        if old != new {
            let target = ObjectRef {
                kind: ObjectKind::Project,
                id: ObjectId::try_from(before.project.id.as_str().to_owned()).unwrap(),
            };
            effect_admitted(
                self.authorize_update(&ledger, branch, &target, &old, &new),
                &mut deferred_failure,
            )?;
            push_ref(&mut changed, target);
        }
        let old_objects = draft_objects(before);
        let new_objects = draft_objects(after);
        for (target, value) in &old_objects {
            if let Some((_, new)) = new_objects
                .iter()
                .find(|(other, _)| same_ref(other, target))
            {
                let old = authorization_value(target, value.clone());
                let new = authorization_value(target, new.clone());
                if old != new {
                    effect_admitted(
                        self.authorize_update(&ledger, branch, target, &old, &new),
                        &mut deferred_failure,
                    )?;
                    push_ref(&mut changed, target.clone());
                }
            } else {
                self.authorize_remove_as(&ledger, target, branch)?;
                push_ref(&mut changed, target.clone());
            }
        }
        for (target, value) in new_objects {
            if !old_objects
                .iter()
                .any(|(other, _)| same_ref(other, &target))
            {
                effect_admitted(
                    self.authorize_create_as(
                        &mut ledger,
                        &target,
                        &authorization_value(&target, value),
                        branch,
                    ),
                    &mut deferred_failure,
                )?;
                push_ref(&mut changed, target);
            }
        }
        // Explicit Put can attach a previously unreferenced current Scenario.
        // Reversing that Host-managed binding still needs authority over its ID.
        let reference_ids: BTreeSet<_> = before
            .project
            .scenario_refs
            .iter()
            .chain(after.project.scenario_refs.iter())
            .collect();
        for id in reference_ids {
            if before.project.scenario_refs.contains(id) == after.project.scenario_refs.contains(id)
            {
                continue;
            }
            let target = ObjectRef {
                kind: ObjectKind::Scenario,
                id: ObjectId::try_from(id.as_str().to_owned()).unwrap(),
            };
            if !changed.iter().any(|object| same_ref(object, &target)) {
                let identity = serde_json::json!({"id":id});
                self.authorize_update(&ledger, branch, &target, &identity, &identity)?;
                push_ref(&mut changed, target);
            }
        }
        for source in before
            .project
            .parameter_sources
            .iter()
            .chain(after.project.parameter_sources.iter())
        {
            let before_source = before.project.parameter_sources.iter().find(|old| {
                same_ref(&old.target, &source.target) && old.field_path == source.field_path
            });
            let after_source = after.project.parameter_sources.iter().find(|new| {
                same_ref(&new.target, &source.target) && new.field_path == source.field_path
            });
            if serde_json::to_value(before_source).unwrap()
                == serde_json::to_value(after_source).unwrap()
            {
                continue;
            }
            if let Some(value) =
                draft_value(after, &source.target).or_else(|| draft_value(before, &source.target))
            {
                self.authorize_parameter_source(
                    &ledger,
                    branch,
                    &source.target,
                    &source.field_path,
                    &value,
                )?;
                push_ref(&mut changed, source.target.clone());
            }
        }
        for reference in before
            .project
            .module_refs
            .iter()
            .chain(after.project.module_refs.iter())
        {
            if before
                .project
                .module_refs
                .iter()
                .any(|actual| module_key(actual) == module_key(reference))
                == after
                    .project
                    .module_refs
                    .iter()
                    .any(|actual| module_key(actual) == module_key(reference))
            {
                continue;
            }
            if matches!(caller.origin, CallerOrigin::ExternalAgent)
                && !caller.edit_grants.iter().any(|grant| {
                    self.current_grant(grant)
                        && grant.branches.iter().any(|actual| same(actual, branch))
                        && grant
                            .source_versions
                            .iter()
                            .any(|actual| module_key(actual) == module_key(reference))
                })
            {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "History module binding requires current exact source-version authority",
                    None,
                ));
            }
        }
        if let Some(failure) = deferred_failure {
            return Err(failure);
        }
        Ok((ledger, changed))
    }

    fn apply_history(
        &mut self,
        caller: &HostCallContext,
        undo: bool,
    ) -> Result<EditCommit, ApiError> {
        let branch = if undo {
            HostEditBranch::Undo
        } else {
            HostEditBranch::Redo
        };
        self.authorize_branch(caller, &branch)?;
        let before = self.current_draft();
        let mut candidate = if undo {
            self.history.undo_candidate(&before)?
        } else {
            self.history.redo_candidate(&before)?
        };
        let (ledger, mut changed) = self.authorize_history(caller, &before, &candidate, &branch)?;
        validate_candidate(&candidate)?;
        let highwater = finalize_draft(&mut candidate, &self.scenario_highwater)?;
        self.refresh_current_business(&mut candidate);
        for scenario in candidate.scenarios.values() {
            push_ref(
                &mut changed,
                ObjectRef {
                    kind: ObjectKind::Scenario,
                    id: ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap(),
                },
            );
        }
        if undo {
            self.history.commit_undo(&before, &candidate);
        } else {
            self.history.commit_redo(&before, &candidate);
        }
        let mut result = EditCommit {
            project_id: candidate.project.id.clone(),
            scope: EditScope::Project(candidate.project.id.clone()),
            previous_revision: Some(before.project.revision),
            new_revision: candidate.project.revision.clone(),
            changed_objects: changed,
            issues: candidate.project.issues.clone(),
            history: self.history.availability(),
        };
        self.project = Some(candidate.project);
        self.scenarios = candidate.scenarios;
        self.professional = candidate.professional;
        self.content_generation += 1;
        self.scenario_highwater = highwater;
        self.dirty = true;
        self.sessions.insert(ledger.session_id.clone(), ledger);
        result.history = self.history_for(caller);
        Ok(result)
    }

    fn authorize_branch(
        &self,
        caller: &HostCallContext,
        branch: &HostEditBranch,
    ) -> Result<(), ApiError> {
        if matches!(caller.origin, CallerOrigin::ExternalAgent)
            && matches!(branch, HostEditBranch::CreateProject)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "CreateProject requires the actual UI session",
                None,
            ));
        }
        if matches!(caller.origin, CallerOrigin::InteractiveUi)
            || caller
                .edit_grants
                .iter()
                .any(|g| self.current_grant(g) && g.branches.iter().any(|b| same(b, branch)))
        {
            Ok(())
        } else {
            Err(error(
                ErrorCode::PermissionDenied,
                "Command branch is not authorized",
                None,
            ))
        }
    }

    fn authorize_update(
        &self,
        caller: &HostCallContext,
        branch: &HostEditBranch,
        target: &ObjectRef,
        old: &Value,
        new: &Value,
    ) -> Result<(), ApiError> {
        if matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Ok(());
        }
        self.authorize_branch(caller, branch)?;
        let changes = changed_fields(old, new)?;
        for (pointer, element) in changes {
            let authorized = caller.edit_grants.iter().any(|grant| {
                self.current_grant(grant)
                    && grant.branches.iter().any(|b| same(b, branch))
                    && grant.objects.iter().any(|object| {
                        same_ref(&object.target, target)
                            && object.fields.iter().any(|field| {
                                field_granted(field, &pointer, element.as_deref(), old)
                            })
                    })
            });
            if !authorized {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Changed field is not authorized",
                    None,
                ));
            }
        }
        // Even an unchanged put requires authority over its stable target.
        if !caller.edit_grants.iter().any(|grant| {
            self.current_grant(grant)
                && grant.branches.iter().any(|b| same(b, branch))
                && grant
                    .objects
                    .iter()
                    .any(|object| same_ref(&object.target, target))
        }) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Object is not authorized",
                None,
            ));
        }
        Ok(())
    }

    fn authorize_parameter_source(
        &self,
        caller: &HostCallContext,
        branch: &HostEditBranch,
        target: &ObjectRef,
        path: &str,
        value: &Value,
    ) -> Result<(), ApiError> {
        if matches!(caller.origin, CallerOrigin::InteractiveUi)
            || caller.edit_grants.iter().any(|grant| {
                self.current_grant(grant)
                    && grant.branches.iter().any(|actual| same(actual, branch))
                    && grant.objects.iter().any(|object| {
                        same_ref(&object.target, target)
                            && object
                                .fields
                                .iter()
                                .any(|field| field_granted(field, path, None, value))
                    })
            })
        {
            Ok(())
        } else {
            Err(error(
                ErrorCode::PermissionDenied,
                "The exact parameter leaf and its source record are not authorized",
                None,
            ))
        }
    }

    fn authorize_remove(
        &self,
        caller: &HostCallContext,
        target: &ObjectRef,
    ) -> Result<(), ApiError> {
        self.authorize_remove_as(caller, target, &HostEditBranch::RemoveObject)
    }

    fn authorize_remove_as(
        &self,
        caller: &HostCallContext,
        target: &ObjectRef,
        branch: &HostEditBranch,
    ) -> Result<(), ApiError> {
        if matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Ok(());
        }
        self.authorize_branch(caller, branch)?;
        if caller.edit_grants.iter().any(|grant| {
            self.current_grant(grant)
                && grant.branches.iter().any(|b| same(b, branch))
                && grant
                    .objects
                    .iter()
                    .any(|object| same_ref(&object.target, target) && object.can_remove)
        }) {
            Ok(())
        } else {
            Err(error(
                ErrorCode::PermissionDenied,
                "Object removal is not authorized",
                None,
            ))
        }
    }

    fn authorize_create(
        &self,
        caller: &mut HostCallContext,
        target: &ObjectRef,
        new: &Value,
    ) -> Result<(), ApiError> {
        self.authorize_create_as(caller, target, new, &HostEditBranch::PutObject)
    }

    fn authorize_create_as(
        &self,
        caller: &mut HostCallContext,
        target: &ObjectRef,
        new: &Value,
        branch: &HostEditBranch,
    ) -> Result<(), ApiError> {
        if matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Ok(());
        }
        self.authorize_branch(caller, branch)?;
        let mut fields = new.clone();
        fields
            .as_object_mut()
            .expect("ItemType object")
            .remove("id");
        let changes = creation_leaves(&fields);
        for grant in &mut caller.edit_grants {
            if !self.current_grant(grant) || !grant.branches.iter().any(|b| same(b, branch)) {
                continue;
            }
            for allowance in &grant.creates {
                if !same(&allowance.kind, &target.kind) {
                    continue;
                }
                let already_created = grant
                    .created_objects
                    .iter()
                    .any(|object| same_ref(object, target));
                let used = grant
                    .created_objects
                    .iter()
                    .filter(|object| same(&object.kind, &target.kind))
                    .map(|object| object.id.as_str())
                    .collect::<BTreeSet<_>>()
                    .len() as u64;
                if !already_created && used >= allowance.max_new_objects.value() {
                    continue;
                }
                if changes.iter().all(|pointer| {
                    allowance
                        .fields
                        .iter()
                        .any(|field| leaf_granted(field, pointer))
                }) {
                    if !already_created {
                        grant.created_objects.push(target.clone());
                    }
                    return Ok(());
                }
            }
        }
        Err(error(
            ErrorCode::PermissionDenied,
            "Creation fields or cumulative distinct-ID budget are not authorized",
            None,
        ))
    }
}

fn implemented_operations() -> Vec<OperationName> {
    vec![
        OperationName::ExportExecutionReport,
        OperationName::ExportCloudData,
        OperationName::StartExecutionSession,
        OperationName::GetExecutionSnapshot,
        OperationName::ReadExecutionView,
        OperationName::ControlExecutionSession,
        OperationName::ReadCloudProduction,
        OperationName::DescribeCapabilities,
        OperationName::GetContext,
        OperationName::GetObject,
        OperationName::RenderView,
        OperationName::ApplyCommands,
        OperationName::SaveProject,
        OperationName::ImportPackage,
        OperationName::GetTask,
        OperationName::CancelTask,
        OperationName::RunScenario,
        OperationName::ComputePackaging,
        OperationName::GetRun,
        OperationName::CompareRuns,
        OperationName::ExportPackage,
        OperationName::ExportEvaluation,
        OperationName::AdoptArtifact,
        OperationName::QueryModuleLibrary,
        OperationName::CheckFactoryDraft,
    ]
}

fn frozen_operations() -> Vec<OperationName> {
    use OperationName::*;
    vec![
        ExportExecutionReport,
        ExportCloudData,
        StartExecutionSession,
        GetExecutionSnapshot,
        ReadExecutionView,
        ControlExecutionSession,
        ReadCloudProduction,
        DescribeCapabilities,
        GetContext,
        GetObject,
        ApplyCommands,
        RenderView,
        RunScenario,
        GetRun,
        CompareRuns,
        ComputePackaging,
        GetTask,
        CancelTask,
        AdoptArtifact,
        ImportPackage,
        SaveProject,
        ExportPackage,
        ExportEvaluation,
        QueryModuleLibrary,
        CheckFactoryDraft,
    ]
}

// Authorization is completed for the whole batch before exposing reference or
// unsupported-effect errors. The candidate and temporary ledger are disposable.
fn effect_admitted(
    result: Result<(), ApiError>,
    deferred: &mut Option<ApiError>,
) -> Result<bool, ApiError> {
    match result {
        Ok(()) => Ok(true),
        Err(failure) if matches!(failure.code, ErrorCode::PermissionDenied) => Err(failure),
        Err(failure) => {
            if deferred.is_none() {
                *deferred = Some(failure);
            }
            Ok(false)
        }
    }
}
fn same<T>(a: &T, b: &T) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}
fn exact_data<T: Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).expect("typed Host data serializes")
        == serde_json::to_value(b).expect("typed Host data serializes")
}
fn current_matching_adoption<'a>(
    project: &'a ProjectState,
    params: &AdoptArtifactParams,
) -> Option<&'a ConsumerBinding> {
    current_consumer_bindings(project)
        .into_iter()
        .find(|binding| {
            same_ref(&binding.consumer_ref, &params.consumer_ref)
                && matches!(binding.mode, ProfessionalMode::CalculatedNominal)
                && binding.artifact_ref == params.artifact_id
                && binding.acceptance_ref == params.acceptance_ref
                && exact_data(&binding.operation_ref, &params.operation_ref)
                && binding.requested_scope == params.requested_scope
        })
}
fn same_ref(a: &ObjectRef, b: &ObjectRef) -> bool {
    same(&a.kind, &b.kind) && a.id == b.id
}
fn push_ref(changed: &mut Vec<ObjectRef>, object: ObjectRef) {
    if !changed.iter().any(|old| same_ref(old, &object)) {
        changed.push(object);
    }
}
fn terminal(state: &ComputationStatus) -> bool {
    matches!(
        state,
        ComputationStatus::Completed | ComputationStatus::Failed | ComputationStatus::Cancelled
    )
}
fn current_task_state(task: &HostIoTask) -> TaskState {
    let mut state = task.state.clone();
    state.task.progress = task.progress.lock().expect("task progress lock").clone();
    if matches!(state.task.state, ComputationStatus::Queued) && task.started.load(Ordering::Acquire)
    {
        state.task.state = ComputationStatus::Running;
    }
    state
}
fn confirms_complete_or_unspecified<T: Serialize>(chosen: &[T], proposed: &[T]) -> bool {
    if chosen.is_empty() {
        return true;
    }
    let identities = |values: &[T]| {
        values
            .iter()
            .map(|value| serde_json::to_string(value).expect("typed selection identity serializes"))
            .collect::<BTreeSet<_>>()
    };
    let selected = identities(chosen);
    selected.len() == chosen.len() && selected == identities(proposed)
}
fn next_revision(revision: &Revision) -> Result<Revision, ApiError> {
    revision
        .0
        .value()
        .checked_add(1)
        .and_then(|value| SafeUint::try_from(value).ok())
        .map(Revision)
        .ok_or_else(|| {
            error(
                ErrorCode::InvalidInput,
                "Project revision range exhausted",
                None,
            )
        })
}
fn check_revision(expected: &Revision, actual: &Revision, pointer: &str) -> Result<(), ApiError> {
    if expected.0.value() == actual.0.value() {
        return Ok(());
    }
    let mut failure = error(
        ErrorCode::RevisionConflict,
        "Requested revision is stale",
        Some(pointer),
    );
    failure.expected_revision = Some(expected.clone());
    failure.actual_revision = Some(actual.clone());
    Err(failure)
}
fn error(code: ErrorCode, message: &str, pointer: Option<&str>) -> ApiError {
    ApiError {
        code,
        message: message.into(),
        object_refs: vec![],
        field_errors: pointer
            .map(|p| {
                vec![ApiFieldError {
                    pointer: p.into(),
                    reason: message.into(),
                }]
            })
            .unwrap_or_default(),
        expected_revision: None,
        actual_revision: None,
    }
}

fn module_key(reference: &ModuleRef) -> (String, String) {
    (
        reference.id.as_str().into(),
        String::from(reference.version.clone()),
    )
}
fn engine_failure(issues: Vec<Issue>) -> ApiError {
    ApiError {
        code: ErrorCode::InvalidInput,
        message: "Frozen run inputs did not pass execution admission".into(),
        object_refs: issues
            .iter()
            .filter_map(|issue| issue.object_ref.clone())
            .collect(),
        field_errors: issues
            .into_iter()
            .map(|issue| ApiFieldError {
                pointer: issue.field_path.unwrap_or_default(),
                reason: format!("{}: {}", issue.code, issue.message),
            })
            .collect(),
        expected_revision: None,
        actual_revision: None,
    }
}

fn packaging_failure(issues: Vec<Issue>) -> ApiError {
    let mut failure = engine_failure(issues);
    failure.message = "Packaging computation rejected its frozen input".into();
    failure
}

fn fail_task(task: &mut HostIoTask, failure: ApiError) {
    task.state.task.state = if matches!(failure.code, ErrorCode::Cancelled) {
        ComputationStatus::Cancelled
    } else {
        ComputationStatus::Failed
    };
    task.state.task.failure = Some(Issue {
        code: format!("{:?}", failure.code),
        object_ref: None,
        field_path: None,
        message: failure.message.clone(),
        blocks: vec![],
    });
    task.state.failure = Some(failure);
    task.state.allowed_controls.clear();
}
fn file_ref(path: &str, kind: DocumentKind) -> FileRef {
    FileRef {
        path: RelativeFilePath::try_from(path.to_owned()).expect("Host-generated relative path"),
        kind,
    }
}
fn run_result_file(input: &sf_simulation::EngineInput) -> FileRef {
    let parent = input
        .input_ref
        .frozen_project_file
        .path
        .as_str()
        .rsplit_once('/')
        .unwrap()
        .0;
    file_ref(&format!("{parent}/result.json"), DocumentKind::RunResult)
}
fn run_additions(
    input: &sf_simulation::EngineInput,
    result: &RunResult,
    history: &StoredRunHistory,
    supporting_scenarios: &[ScenarioInput],
) -> Vec<ArchiveAddition> {
    let entry = |role, kind, id: &str, revision, file_ref| DocumentEntry {
        role,
        module_ref: None,
        object_ref: ObjectRef {
            kind,
            id: ObjectId::try_from(id.to_owned()).unwrap(),
        },
        revision,
        file_ref,
        required: true,
    };
    let mut additions = vec![
        ArchiveAddition {
            entry: entry(
                DocumentRole::FrozenRunProject,
                ObjectKind::Project,
                input.project.id.as_str(),
                Some(input.project.revision.clone()),
                input.input_ref.frozen_project_file.clone(),
            ),
            bytes: encode_document(&input.project),
        },
        ArchiveAddition {
            entry: entry(
                DocumentRole::FrozenRunScenario,
                ObjectKind::Scenario,
                input.scenario.id.as_str(),
                Some(input.scenario.revision.clone()),
                input.input_ref.frozen_scenario_file.clone(),
            ),
            bytes: encode_document(&input.scenario),
        },
        ArchiveAddition {
            entry: entry(
                DocumentRole::RunResult,
                ObjectKind::Run,
                result.id.as_str(),
                None,
                run_result_file(input),
            ),
            bytes: encode_document(result),
        },
        ArchiveAddition {
            entry: entry(
                DocumentRole::SupportingFile,
                ObjectKind::Run,
                result.id.as_str(),
                None,
                run_history_file(result),
            ),
            bytes: encode_document(history),
        },
    ];
    let directory = input
        .input_ref
        .frozen_project_file
        .path
        .as_str()
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .expect("Host-generated Run directory");
    for (index, scenario) in supporting_scenarios.iter().enumerate() {
        additions.push(ArchiveAddition {
            entry: entry(
                DocumentRole::SupportingFile,
                ObjectKind::Scenario,
                scenario.id.as_str(),
                Some(scenario.revision.clone()),
                file_ref(
                    &format!("{directory}/scenario-support-{:04}.json", index + 1),
                    DocumentKind::Scenario,
                ),
            ),
            bytes: encode_document(scenario),
        });
    }
    additions
}
fn run_history_file(result: &RunResult) -> FileRef {
    let parent = result
        .input_ref
        .frozen_project_file
        .path
        .as_str()
        .rsplit_once('/')
        .map(|(parent, _)| parent);
    file_ref(
        &parent.map_or_else(
            || "item-history.json".into(),
            |parent| format!("{parent}/item-history.json"),
        ),
        DocumentKind::Asset,
    )
}

fn archived_runs(archive: &ProjectArchive) -> Result<BTreeMap<RunId, HostRun>, ApiError> {
    let mut runs = BTreeMap::new();
    for result in archive.run_results()? {
        let file = run_history_file(&result);
        let history: Option<StoredRunHistory> = if let Some(entry) =
            archive.manifest.documents.iter().find(|entry| {
                entry.file_ref.path == file.path
                    && matches!(entry.file_ref.kind, DocumentKind::Asset)
                    && matches!(entry.object_ref.kind, ObjectKind::Run)
                    && entry.object_ref.id.as_str() == result.id.as_str()
            }) {
            archive
                .files
                .get(entry.file_ref.path.as_str())
                .map(|bytes| {
                    serde_json::from_slice::<Versioned<StoredRunHistory>>(bytes)
                        .map(|value| value.data)
                        .map_err(|failure| {
                            error(
                                ErrorCode::InvalidInput,
                                &format!("Invalid Run history record: {failure}"),
                                None,
                            )
                        })
                })
                .transpose()?
        } else {
            None
        };
        if history.as_ref().is_some_and(|history| {
            history.run_id != result.id
                || serde_json::to_value(&history.input_ref).unwrap()
                    != serde_json::to_value(&result.input_ref).unwrap()
        }) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Run history identity differs from its frozen result",
                None,
            ));
        }
        let scope = history
            .as_ref()
            .map(|history| history.scope.clone())
            .unwrap_or_else(|| EditScope::Project(result.input_ref.project_id.clone()));
        if runs
            .insert(
                result.id.clone(),
                HostRun {
                    result,
                    engine: None,
                    history,
                    scope,
                    observation_cache: Mutex::new(Default::default()),
                },
            )
            .is_some()
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Duplicate stored Run identity",
                None,
            ));
        }
    }
    Ok(runs)
}

fn run_summary(result: &RunResult) -> RunSummary {
    RunSummary {
        run_id: result.id.clone(),
        input: result.input_ref.clone(),
        computation_status: result.computation_status.clone(),
        window: result.window.clone(),
        computed_until_ms: result.computed_until_ms.clone(),
        whole_window_computed: result.whole_window_computed,
        goal_met: result.goal_met,
        accepted: if result.orders.len() == 1 {
            Some(result.orders[0].accepted.clone())
        } else {
            None
        },
        issues: result.limitations.clone(),
    }
}
fn page_values<T>(
    values: Vec<T>,
    page: PageRequest,
) -> Result<(Vec<T>, Option<SafeUint>), ApiError> {
    if page.limit.value() == 0 {
        return Err(error(
            ErrorCode::InvalidInput,
            "Page limit must be positive",
            None,
        ));
    }
    let start = page
        .after_index
        .as_ref()
        .map_or(0, |index| index.value().saturating_add(1)) as usize;
    let end = start
        .saturating_add(page.limit.value() as usize)
        .min(values.len());
    let next = if end < values.len() {
        Some(SafeUint::try_from((end - 1) as u64).unwrap())
    } else {
        None
    };
    Ok((
        values
            .into_iter()
            .skip(start)
            .take(page.limit.value() as usize)
            .collect(),
        next,
    ))
}
fn unsupported(message: &str) -> ApiError {
    error(ErrorCode::UnsupportedCapability, message, None)
}
fn missing(target: &ObjectRef, message: &str) -> ApiError {
    let mut e = error(ErrorCode::MissingReference, message, None);
    e.object_refs.push(target.clone());
    e
}

fn supported_kind(kind: &ObjectKind) -> bool {
    matches!(
        kind,
        ObjectKind::ItemType
            | ObjectKind::Location
            | ObjectKind::Hall
            | ObjectKind::Line
            | ObjectKind::Device
            | ObjectKind::Resource
            | ObjectKind::Calendar
            | ObjectKind::Connection
            | ObjectKind::Product
            | ObjectKind::Bom
            | ObjectKind::Route
            | ObjectKind::Scenario
    )
}

fn minimal_creation_fields(kind: &ObjectKind) -> &'static [&'static str] {
    match kind {
        ObjectKind::ItemType => &["/revision", "/title", "/unit", "/tracking", "/spec_ref"],
        ObjectKind::Product => &[
            "/revision",
            "/item_type_ref/id",
            "/item_type_ref/revision",
            "/bom_ref/id",
            "/bom_ref/revision",
            "/routing_ref/id",
            "/routing_ref/revision",
        ],
        ObjectKind::Bom | ObjectKind::Route => &["/revision"],
        ObjectKind::Scenario => &[
            "/title",
            "/time_reference/label",
            "/time_reference/origin_iso8601",
            "/time_reference/timezone",
            "/window/start_ms",
            "/window/end_ms",
        ],
        ObjectKind::Location => &[
            "/title",
            "/hall_id",
            "/capacity",
            "/purpose",
            "/ordinary_issue_enabled",
        ],
        ObjectKind::Hall => &["/title", "/width_m", "/depth_m", "/height_m"],
        ObjectKind::Line => &["/title", "/hall_id"],
        ObjectKind::Device => &[
            "/title",
            "/hall_id",
            "/line_id",
            "/module_ref/id",
            "/module_ref/version",
            "/height_m",
            "/presentation_override",
            "/operation_binding",
        ],
        ObjectKind::Resource => &[
            "/definition_ref/id",
            "/definition_ref/version",
            "/title",
            "/kind",
            "/role",
            "/capacity/value/coefficient",
            "/capacity/value/decimal_places",
            "/capacity/unit",
            "/calendar_id",
            "/home_location",
            "/presentation",
        ],
        ObjectKind::Calendar => &["/title", "/source"],
        _ => &[],
    }
}

fn creation_path_available(allowance: &HostCreateAllowance) -> bool {
    let has = |path: &str| {
        allowance
            .fields
            .iter()
            .any(|field| leaf_granted(field, path))
    };
    if matches!(allowance.kind, ObjectKind::Connection) {
        let material = [
            "/kind",
            "/data/source/kind",
            "/data/source/data",
            "/data/destination/kind",
            "/data/destination/data",
            "/data/item_type",
            "/data/unit",
            "/data/quantity_rule",
            "/data/destination_store",
            "/data/capacity_rule",
        ];
        let ports = [
            "/kind",
            "/data/source/module_ref/id",
            "/data/source/module_ref/version",
            "/data/source/instance_id",
            "/data/source/port_id",
            "/data/destination/module_ref/id",
            "/data/destination/module_ref/version",
            "/data/destination/instance_id",
            "/data/destination/port_id",
        ];
        return material.into_iter().all(has)
            || (ports.into_iter().all(has)
                && ((has("/data/value_type") && has("/data/effect_target"))
                    || has("/data/binding_ref")));
    }
    let fields = minimal_creation_fields(&allowance.kind);
    if fields.is_empty() || !fields.iter().all(|path| has(path)) {
        return false;
    }
    let pose = match allowance.kind {
        ObjectKind::Location => Some("/position"),
        ObjectKind::Hall => Some("/origin"),
        ObjectKind::Line => Some("/transform"),
        ObjectKind::Device => Some("/pose"),
        _ => None,
    };
    pose.is_none_or(|prefix| {
        [
            "/translation/x",
            "/translation/y",
            "/translation/z",
            "/rotation/x",
            "/rotation/y",
            "/rotation/z",
            "/rotation/w",
            "/scale/x",
            "/scale/y",
            "/scale/z",
            "/unit",
            "/coordinate_frame",
        ]
        .into_iter()
        .all(|suffix| has(&format!("{prefix}{suffix}")))
    })
}

fn connection_id(connection: &Connection) -> &ObjectId {
    match connection {
        Connection::Material(value) => &value.id,
        Connection::Signal(value) => &value.id,
        Connection::Artifact(value) => &value.id,
    }
}

fn editable_target(object: &EditableProjectObject) -> Option<ObjectRef> {
    let (kind, id) = match object {
        EditableProjectObject::ItemType(value) => (ObjectKind::ItemType, value.id.clone()),
        EditableProjectObject::Location(value) => (ObjectKind::Location, value.id.clone()),
        EditableProjectObject::Hall(value) => (ObjectKind::Hall, value.id.clone()),
        EditableProjectObject::Line(value) => (ObjectKind::Line, value.id.clone()),
        EditableProjectObject::Device(value) => (ObjectKind::Device, value.id.clone()),
        EditableProjectObject::Resource(value) => (ObjectKind::Resource, value.id.clone()),
        EditableProjectObject::Calendar(value) => (ObjectKind::Calendar, value.id.clone()),
        EditableProjectObject::Connection(value) => {
            (ObjectKind::Connection, connection_id(value).clone())
        }
        EditableProjectObject::Product(value) => (ObjectKind::Product, value.id.clone()),
        EditableProjectObject::Bom(value) => (ObjectKind::Bom, value.id.clone()),
        EditableProjectObject::Route(value) => (ObjectKind::Route, value.id.clone()),
        EditableProjectObject::Scenario(value) => (
            ObjectKind::Scenario,
            ObjectId::try_from(value.id.as_str().to_owned()).unwrap(),
        ),
    };
    Some(ObjectRef { kind, id })
}

fn editable_object_value(object: &EditableProjectObject) -> Value {
    match object {
        EditableProjectObject::ItemType(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Location(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Hall(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Line(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Device(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Resource(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Calendar(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Connection(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Product(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Bom(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Route(value) => serde_json::to_value(value).unwrap(),
        EditableProjectObject::Scenario(value) => serde_json::to_value(value).unwrap(),
    }
}

fn authorization_value(target: &ObjectRef, mut value: Value) -> Value {
    if matches!(target.kind, ObjectKind::Connection) {
        value
            .get_mut("data")
            .and_then(Value::as_object_mut)
            .expect("typed Connection payload")
            .remove("id");
    }
    if matches!(target.kind, ObjectKind::Scenario) {
        let body = value.as_object_mut().expect("typed Scenario object");
        body.remove("revision");
        body.remove("project_revision");
    }
    value
}

fn settings_value(project: &ProjectState) -> Value {
    serde_json::to_value(ProjectSettings {
        title: project.title.clone(),
        time_reference: project.time_reference.clone(),
        generation_context: project.generation_context.clone(),
        sources: project.sources.clone(),
        setup_policies: Some(project.setup_policies.clone()),
    })
    .unwrap()
}

fn draft_objects(draft: &ProjectDraft) -> Vec<(ObjectRef, Value)> {
    let mut objects = vec![];
    macro_rules! collect {
        ($field:ident, $kind:ident) => {
            for value in &draft.project.$field {
                objects.push((
                    ObjectRef {
                        kind: ObjectKind::$kind,
                        id: value.id.clone(),
                    },
                    serde_json::to_value(value).unwrap(),
                ));
            }
        };
    }
    collect!(item_types, ItemType);
    collect!(locations, Location);
    collect!(halls, Hall);
    collect!(lines, Line);
    collect!(devices, Device);
    collect!(resources, Resource);
    collect!(calendars, Calendar);
    collect!(products, Product);
    collect!(boms, Bom);
    collect!(routes, Route);
    for value in &draft.project.connections {
        objects.push((
            ObjectRef {
                kind: ObjectKind::Connection,
                id: connection_id(value).clone(),
            },
            serde_json::to_value(value).unwrap(),
        ));
    }
    for value in draft.scenarios.values() {
        objects.push((
            ObjectRef {
                kind: ObjectKind::Scenario,
                id: ObjectId::try_from(value.id.as_str().to_owned()).unwrap(),
            },
            serde_json::to_value(value).unwrap(),
        ));
    }
    objects
}

fn draft_value(draft: &ProjectDraft, target: &ObjectRef) -> Option<Value> {
    macro_rules! find {
        ($field:ident) => {
            draft.project.$field.iter()
                .find(|value| value.id.as_str() == target.id.as_str())
                .map(|value| serde_json::to_value(value).unwrap())
        };
    }
    match target.kind {
        ObjectKind::ItemType => find!(item_types),
        ObjectKind::Location => find!(locations),
        ObjectKind::Hall => find!(halls),
        ObjectKind::Line => find!(lines),
        ObjectKind::Device => find!(devices),
        ObjectKind::Resource => find!(resources),
        ObjectKind::Calendar => find!(calendars),
        ObjectKind::Product => find!(products),
        ObjectKind::Bom => find!(boms),
        ObjectKind::Route => find!(routes),
        ObjectKind::Connection => draft.project.connections.iter()
            .find(|value| connection_id(value).as_str() == target.id.as_str())
            .map(|value| serde_json::to_value(value).unwrap()),
        ObjectKind::Scenario => draft.scenarios.values()
            .find(|value| value.id.as_str() == target.id.as_str())
            .map(|value| serde_json::to_value(value).unwrap()),
        _ => None,
    }
}

fn put_draft_object(draft: &mut ProjectDraft, object: EditableProjectObject) {
    macro_rules! put {
        ($field:ident, $value:ident) => {
            if let Some(index) = draft
                .project
                .$field
                .iter()
                .position(|old| old.id == $value.id)
            {
                draft.project.$field[index] = $value;
            } else {
                draft.project.$field.push($value);
            }
        };
    }
    match object {
        EditableProjectObject::ItemType(value) => put!(item_types, value),
        EditableProjectObject::Location(value) => put!(locations, value),
        EditableProjectObject::Hall(value) => put!(halls, value),
        EditableProjectObject::Line(value) => put!(lines, value),
        EditableProjectObject::Device(value) => put!(devices, value),
        EditableProjectObject::Resource(value) => put!(resources, value),
        EditableProjectObject::Calendar(value) => put!(calendars, value),
        EditableProjectObject::Connection(value) => {
            if let Some(index) = draft
                .project
                .connections
                .iter()
                .position(|old| connection_id(old) == connection_id(&value))
            {
                draft.project.connections[index] = value;
            } else {
                draft.project.connections.push(value);
            }
        }
        EditableProjectObject::Product(value) => put!(products, value),
        EditableProjectObject::Bom(value) => put!(boms, value),
        EditableProjectObject::Route(value) => put!(routes, value),
        EditableProjectObject::Scenario(value) => {
            if !draft.project.scenario_refs.contains(&value.id) {
                draft.project.scenario_refs.push(value.id.clone());
            }
            draft.scenarios.insert(value.id.clone(), *value);
        }
    }
}

fn remove_draft_object(draft: &mut ProjectDraft, target: &ObjectRef) -> bool {
    macro_rules! remove {
        ($field:ident) => {{
            if let Some(index) = draft
                .project
                .$field
                .iter()
                .position(|old| old.id == target.id)
            {
                draft.project.$field.remove(index);
                true
            } else {
                false
            }
        }};
    }
    match target.kind {
        ObjectKind::ItemType => remove!(item_types),
        ObjectKind::Location => remove!(locations),
        ObjectKind::Hall => remove!(halls),
        ObjectKind::Line => remove!(lines),
        ObjectKind::Device => remove!(devices),
        ObjectKind::Resource => remove!(resources),
        ObjectKind::Calendar => remove!(calendars),
        ObjectKind::Connection => {
            if let Some(index) = draft
                .project
                .connections
                .iter()
                .position(|old| connection_id(old) == &target.id)
            {
                draft.project.connections.remove(index);
                true
            } else {
                false
            }
        }
        ObjectKind::Product => remove!(products),
        ObjectKind::Bom => remove!(boms),
        ObjectKind::Route => remove!(routes),
        ObjectKind::Scenario => {
            let id = ScenarioId::try_from(target.id.as_str().to_owned()).unwrap();
            if draft.scenarios.remove(&id).is_some() {
                draft
                    .project
                    .scenario_refs
                    .retain(|reference| reference != &id);
                true
            } else {
                false
            }
        }
        _ => unreachable!("supported typed edit object"),
    }
}

fn refresh_business_issues(draft: &mut ProjectDraft) {
    draft.project.issues.retain(|issue| {
        !issue.code.starts_with("business_inputs.")
            && !issue.code.starts_with("professional_inputs.")
            && !issue.code.starts_with("item_layout.")
    });
    let issues = sf_domain::check_business_inputs(&draft.project, &draft.scenarios);
    draft.project.issues.extend(issues);
    draft.project.issues.extend(professional_issues(draft));
}

fn refresh_business_issues_with_definitions(
    draft: &mut ProjectDraft,
    definitions: &[ModuleDefinition],
) {
    draft.project.issues.retain(|issue| {
        !issue.code.starts_with("business_inputs.")
            && !issue.code.starts_with("professional_inputs.")
            && !issue.code.starts_with("item_layout.")
    });
    draft
        .project
        .issues
        .extend(sf_domain::check_business_inputs_with_definitions(
            &draft.project,
            &draft.scenarios,
            definitions,
        ));
    draft.project.issues.extend(professional_issues(draft));
    draft
        .project
        .issues
        .extend(host_item_layout::issues(&draft.project, definitions));
}

fn import_preview_missing(archive: &ProjectArchive, draft: &ProjectDraft) -> Vec<Issue> {
    let mut issues: Vec<_> = archive
        .manifest
        .missing
        .iter()
        .filter(|issue| !issue.code.starts_with("business_inputs."))
        .cloned()
        .collect();
    issues.extend(
        draft
            .project
            .issues
            .iter()
            .filter(|issue| !issue.blocks.is_empty())
            .cloned(),
    );
    let mut seen = BTreeSet::new();
    issues
        .retain(|issue| seen.insert(serde_json::to_string(issue).expect("typed issue serializes")));
    issues
}

fn finalize_draft(
    draft: &mut ProjectDraft,
    previous_highwater: &BTreeMap<ScenarioId, Revision>,
) -> Result<BTreeMap<ScenarioId, Revision>, ApiError> {
    draft.project.revision = next_revision(&draft.project.revision)?;
    let mut highwater = previous_highwater.clone();
    for (id, scenario) in &mut draft.scenarios {
        scenario.revision = match highwater.get(id) {
            Some(previous) => next_revision(previous)?,
            None => Revision(SafeUint::try_from(1).unwrap()),
        };
        scenario.project_revision = draft.project.revision.clone();
        highwater.insert(id.clone(), scenario.revision.clone());
    }
    refresh_business_issues(draft);
    Ok(highwater)
}

fn archive_scenario_highwater(archive: &ProjectArchive) -> BTreeMap<ScenarioId, Revision> {
    let mut highwater: BTreeMap<_, _> = archive
        .scenarios
        .iter()
        .map(|(id, scenario)| (id.clone(), scenario.revision.clone()))
        .collect();
    for document in &archive.manifest.documents {
        if !matches!(document.object_ref.kind, ObjectKind::Scenario)
            || !matches!(document.file_ref.kind, DocumentKind::Scenario)
            || !archive.files.contains_key(document.file_ref.path.as_str())
        {
            continue;
        }
        if let Some(revision) = &document.revision {
            let id = ScenarioId::try_from(document.object_ref.id.as_str().to_owned()).unwrap();
            if highwater
                .get(&id)
                .is_none_or(|current| revision.0.value() > current.0.value())
            {
                highwater.insert(id, revision.clone());
            }
        }
    }
    highwater
}

fn lookup(
    project: &ProjectState,
    scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
    target: &ObjectRef,
) -> Result<DomainObject, ApiError> {
    match target.kind {
        ObjectKind::Project if target.id.as_str() == project.id.as_str() => {
            Ok(DomainObject::Project(Box::new(project.clone())))
        }
        ObjectKind::Project => Err(missing(
            target,
            "Project object does not exist in active scope",
        )),
        ObjectKind::ItemType => project
            .item_types
            .iter()
            .find(|item| item.id == target.id)
            .cloned()
            .map(DomainObject::ItemType)
            .ok_or_else(|| missing(target, "ItemType does not exist")),
        ObjectKind::Location => project
            .locations
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Location)
            .ok_or_else(|| missing(target, "Location does not exist")),
        ObjectKind::Hall => project
            .halls
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Hall)
            .ok_or_else(|| missing(target, "Hall does not exist")),
        ObjectKind::Line => project
            .lines
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Line)
            .ok_or_else(|| missing(target, "Line does not exist")),
        ObjectKind::Device => project
            .devices
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Device)
            .ok_or_else(|| missing(target, "Device does not exist")),
        ObjectKind::Resource => project
            .resources
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Resource)
            .ok_or_else(|| missing(target, "Resource does not exist")),
        ObjectKind::Calendar => project
            .calendars
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Calendar)
            .ok_or_else(|| missing(target, "Calendar does not exist")),
        ObjectKind::Connection => project
            .connections
            .iter()
            .find(|value| connection_id(value) == &target.id)
            .cloned()
            .map(DomainObject::Connection)
            .ok_or_else(|| missing(target, "Connection does not exist")),
        ObjectKind::Product => project
            .products
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Product)
            .ok_or_else(|| missing(target, "Product does not exist")),
        ObjectKind::Bom => project
            .boms
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Bom)
            .ok_or_else(|| missing(target, "Bom does not exist")),
        ObjectKind::Route => project
            .routes
            .iter()
            .find(|value| value.id == target.id)
            .cloned()
            .map(DomainObject::Route)
            .ok_or_else(|| missing(target, "Route does not exist")),
        ObjectKind::Scenario => scenarios
            .values()
            .find(|value| value.id.as_str() == target.id.as_str())
            .cloned()
            .map(|value| DomainObject::Scenario(Box::new(value)))
            .ok_or_else(|| missing(target, "Scenario does not exist")),
        _ => Err(unsupported(
            "Object kind is not implemented for Draft reads",
        )),
    }
}

fn empty_project(id: ProjectId, new: NewProject) -> ProjectState {
    let number = |n| FiniteNumber::try_from(n).unwrap();
    let vector = |x, y, z| Vec3 {
        x: number(x),
        y: number(y),
        z: number(z),
    };
    let camera = CameraState {
        coordinate_frame: CoordinateFrame::RightHandedZUp,
        unit: LengthUnit::Meter,
        eye: vector(10., -10., 10.),
        target: vector(0., 0., 0.),
        zoom: number(1.),
    };
    ProjectState {
        setup_policies: vec![],
        id,
        revision: Revision(SafeUint::try_from(1).unwrap()),
        title: new.title,
        time_reference: new.time_reference,
        item_types: vec![],
        locations: vec![],
        halls: vec![],
        lines: vec![],
        devices: vec![],
        resources: vec![],
        calendars: vec![],
        products: vec![],
        boms: vec![],
        routes: vec![],
        connections: vec![],
        module_refs: vec![],
        scenario_refs: vec![],
        artifact_refs: vec![],
        acknowledgments: vec![],
        bindings: vec![],
        views: ViewState {
            normal_canvas: CanvasKind::LineCanvas,
            selected_hall: None,
            selected_objects: vec![],
            lod: LodLevel::Detailed,
            line_camera: camera.clone(),
            multi_hall_camera: camera,
        },
        generation_context: None,
        parameter_sources: vec![],
        sources: vec![],
        issues: vec![],
        template_adoptions: vec![],
    }
}

/// Exact initial leaves of a newly created, typed object. Numeric array indices
/// describe this candidate only; existing-object edits still use stable IDs.
fn creation_leaves(value: &Value) -> Vec<String> {
    fn visit(value: &Value, path: String, out: &mut Vec<String>) {
        match value {
            Value::Object(fields) => {
                for (key, value) in fields {
                    visit(
                        value,
                        format!("{}/{}", path, key.replace('~', "~0").replace('/', "~1")),
                        out,
                    );
                }
            }
            Value::Array(values) => {
                for (index, value) in values.iter().enumerate() {
                    visit(value, format!("{path}/{index}"), out);
                }
            }
            _ => out.push(path),
        }
    }
    let mut paths = Vec::new();
    visit(value, String::new(), &mut paths);
    paths
}

/// All changed paths are exact RFC6901 leaves. Array effects are deliberately unsupported.
fn changed_leaves(old: &Value, new: &Value) -> Result<Vec<String>, ApiError> {
    fn visit(
        old: Option<&Value>,
        new: Option<&Value>,
        path: String,
        out: &mut Vec<String>,
    ) -> Result<(), ApiError> {
        if old == new {
            return Ok(());
        }
        if old.is_some_and(Value::is_array) || new.is_some_and(Value::is_array) {
            if old.is_none() && host_item_layout::slot_array(&path) {
                if let Some(slots) = new.and_then(Value::as_array) {
                    for (index, slot) in slots.iter().enumerate() {
                        visit(None, Some(slot), format!("{path}/{index}"), out)?;
                    }
                    return Ok(());
                }
            }
            if old.is_none() && new.is_some_and(|v| v.as_array().is_some_and(Vec::is_empty)) {
                return Ok(());
            }
            return Err(unsupported(
                "External array changes require stable element resolution, which is not implemented",
            ));
        }
        let old_map = old.and_then(Value::as_object);
        let new_map = new.and_then(Value::as_object);
        if (old_map.is_some() && new.is_none_or(|v| !v.is_object()))
            || (new_map.is_some() && old.is_some_and(|v| !v.is_object() && !v.is_null()))
        {
            return Err(unsupported(
                "External container/scalar replacement is not a leaf-field edit",
            ));
        }
        if old_map.is_some() || new_map.is_some() {
            let keys: BTreeSet<_> = old_map
                .into_iter()
                .flat_map(|m| m.keys())
                .chain(new_map.into_iter().flat_map(|m| m.keys()))
                .collect();
            for key in keys {
                visit(
                    old_map.and_then(|m| m.get(key)),
                    new_map.and_then(|m| m.get(key)),
                    format!("{}/{}", path, key.replace('~', "~0").replace('/', "~1")),
                    out,
                )?;
            }
        } else {
            out.push(path);
        }
        Ok(())
    }
    let mut paths = vec![];
    visit(
        if old.is_null() { None } else { Some(old) },
        Some(new),
        String::new(),
        &mut paths,
    )?;
    Ok(paths)
}

fn leaf_granted(grant: &HostFieldGrant, pointer: &str) -> bool {
    if grant.element_id.is_some()
        || grant.pointer.is_empty()
        || !grant.pointer.starts_with('/')
        || grant.pointer.contains('*')
    {
        return false;
    }
    // Validate RFC6901 escapes rather than silently normalizing malformed grants.
    let mut chars = grant.pointer.chars();
    while let Some(c) = chars.next() {
        if c == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    grant.pointer == pointer
}

fn editable_existing_leaf(grant: &HostFieldGrant, value: &Value, nullable_string: bool) -> bool {
    let Some(pointer) = resolved_field_pointer(grant, value) else {
        return false;
    };
    let Some(current) = value.pointer(&pointer) else {
        return false;
    };
    current.is_string()
        || current.is_boolean()
        || current.is_number()
        // Exact whole-field authority for the typed device polygon.
        || (pointer == "/footprint_m" && current.is_array())
        || (nullable_string && current.is_null())
}

fn array_element_id<'a>(path: &str, value: &'a Value) -> Option<&'a str> {
    value
        .get(if path == "/setup_policies" {
            "device_id"
        } else {
            "id"
        })
        .and_then(Value::as_str)
}

/// Array leaves carry the actual stable containing ID; shape/order effects remain
/// distinct from a field edit and cannot borrow authority from one child.
fn changed_fields(old: &Value, new: &Value) -> Result<Vec<(String, Option<String>)>, ApiError> {
    fn visit(
        old: Option<&Value>,
        new: Option<&Value>,
        path: String,
        element: Option<&str>,
        out: &mut Vec<(String, Option<String>)>,
    ) -> Result<(), ApiError> {
        if old == new {
            return Ok(());
        }
        if old.is_some_and(Value::is_array) || new.is_some_and(Value::is_array) {
            let (Some(before), Some(after)) =
                (old.and_then(Value::as_array), new.and_then(Value::as_array))
            else {
                return Err(unsupported(
                    "External array structure changes require a separate object creation/removal authority",
                ));
            };
            if path == "/footprint_m" && element.is_none() {
                out.push((path, None));
                return Ok(());
            }
            if host_item_layout::slot_array(&path) && element.is_none() {
                if before.len() != after.len() {
                    return Err(unsupported(
                        "External display-slot creation/removal needs separate object authority",
                    ));
                }
                for (index, (before, after)) in before.iter().zip(after).enumerate() {
                    visit(
                        Some(before),
                        Some(after),
                        format!("{path}/{index}"),
                        None,
                        out,
                    )?;
                }
                return Ok(());
            }
            if element.is_some() || before.len() != after.len() {
                return Err(unsupported(
                    "Nested or structural array changes are not scalar leaf edits",
                ));
            }
            let mut seen = BTreeSet::new();
            for (index, (before, after)) in before.iter().zip(after).enumerate() {
                let Some(id) = array_element_id(&path, before) else {
                    return Err(unsupported(
                        "External array fields require declared stable element IDs",
                    ));
                };
                if !seen.insert(id) || array_element_id(&path, after) != Some(id) {
                    return Err(unsupported(
                        "External edits cannot create, remove or reorder declared array elements",
                    ));
                }
                visit(
                    Some(before),
                    Some(after),
                    format!("{path}/{index}"),
                    Some(id),
                    out,
                )?;
            }
            return Ok(());
        }
        let old_map = old.and_then(Value::as_object);
        let new_map = new.and_then(Value::as_object);
        if (old_map.is_some() && new.is_none_or(|v| !v.is_object()))
            || (new_map.is_some() && old.is_some_and(|v| !v.is_object() && !v.is_null()))
        {
            return Err(unsupported(
                "External container/scalar replacement is not a leaf-field edit",
            ));
        }
        if old_map.is_some() || new_map.is_some() {
            let keys: BTreeSet<_> = old_map
                .into_iter()
                .flat_map(|m| m.keys())
                .chain(new_map.into_iter().flat_map(|m| m.keys()))
                .collect();
            for key in keys {
                visit(
                    old_map.and_then(|m| m.get(key)),
                    new_map.and_then(|m| m.get(key)),
                    format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                    element,
                    out,
                )?;
            }
        } else {
            out.push((path, element.map(str::to_owned)));
        }
        Ok(())
    }
    let mut out = vec![];
    visit(Some(old), Some(new), String::new(), None, &mut out)?;
    Ok(out)
}

fn resolved_field_pointer(grant: &HostFieldGrant, value: &Value) -> Option<String> {
    let scalar_grant = HostFieldGrant {
        pointer: grant.pointer.clone(),
        element_id: None,
    };
    if !leaf_granted(&scalar_grant, &grant.pointer) {
        return None;
    }
    let mut current = value;
    let mut path = String::new();
    let mut element_used = false;
    for raw in grant.pointer.split('/').skip(1) {
        let token = raw.replace("~1", "/").replace("~0", "~");
        if let Some(values) = current.as_array() {
            if host_item_layout::slot_array(&path) && grant.element_id.is_none() {
                let index = raw.parse::<usize>().ok()?;
                if raw != index.to_string() {
                    return None;
                }
                current = values.get(index)?;
                path.push_str(&format!("/{index}"));
                continue;
            }
            if element_used || raw.parse::<usize>().is_err() {
                return None;
            }
            let id = grant.element_id.as_ref()?;
            let matches: Vec<_> = values
                .iter()
                .enumerate()
                .filter(|(_, value)| array_element_id(&path, value) == Some(id.as_str()))
                .collect();
            if matches.len() != 1 {
                return None;
            }
            let (index, value) = matches[0];
            path.push_str(&format!("/{index}"));
            current = value;
            element_used = true;
        } else {
            current = current.as_object()?.get(&token)?;
            path.push('/');
            path.push_str(raw);
        }
    }
    if element_used != grant.element_id.is_some() {
        return None;
    }
    Some(path)
}

fn field_granted(
    grant: &HostFieldGrant,
    pointer: &str,
    element: Option<&str>,
    old: &Value,
) -> bool {
    if grant.element_id.as_ref().map(ObjectId::as_str) != element {
        return false;
    }
    resolved_field_pointer(grant, old).as_deref() == Some(pointer)
        || (element.is_none() && leaf_granted(grant, pointer))
}

fn validate_candidate(draft: &ProjectDraft) -> Result<(), ApiError> {
    host_item_layout::validate_references(&draft.project)?;
    fn unique<'a>(ids: impl Iterator<Item = &'a str>, pointer: &str) -> Result<(), ApiError> {
        let mut seen = BTreeSet::new();
        if ids.into_iter().any(|id| !seen.insert(id)) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Duplicate stable object ID in this collection",
                Some(pointer),
            ));
        }
        Ok(())
    }
    macro_rules! unique_project {
        ($field:ident) => {
            unique(
                draft.project.$field.iter().map(|value| value.id.as_str()),
                concat!("/", stringify!($field)),
            )?;
        };
    }
    unique_project!(item_types);
    unique_project!(products);
    unique_project!(boms);
    unique_project!(routes);
    unique_project!(locations);
    unique_project!(halls);
    unique_project!(lines);
    unique_project!(devices);
    unique_project!(resources);
    unique_project!(calendars);
    unique(
        draft
            .project
            .setup_policies
            .iter()
            .map(|policy| policy.device_id.as_str()),
        "/setup_policies",
    )?;
    unique(
        draft
            .project
            .connections
            .iter()
            .map(|value| connection_id(value).as_str()),
        "/connections",
    )?;
    for route in &draft.project.routes {
        unique(
            route.steps.iter().map(|step| step.id.as_str()),
            "/routes/steps",
        )?;
    }
    for (id, scenario) in &draft.scenarios {
        if id != &scenario.id {
            return Err(error(
                ErrorCode::InvalidInput,
                "Scenario map identity does not match its document",
                Some("/scenarios"),
            ));
        }
        unique(
            scenario.orders.iter().map(|value| value.id.as_str()),
            "/scenarios/orders",
        )?;
        unique(
            scenario
                .production_tasks
                .iter()
                .map(|value| value.id.as_str()),
            "/scenarios/production_tasks",
        )?;
        unique(
            scenario.arrivals.iter().map(|value| value.id.as_str()),
            "/scenarios/arrivals",
        )?;
        unique(
            scenario.supplies.iter().map(|value| value.id.as_str()),
            "/scenarios/supplies",
        )?;
        unique(
            scenario
                .initial_conditions
                .items
                .iter()
                .map(|value| value.id.as_str()),
            "/scenarios/initial_conditions/items",
        )?;
        unique(
            scenario
                .quality_presets
                .iter()
                .map(|value| value.id.as_str()),
            "/scenarios/quality_presets",
        )?;
        unique(
            scenario.cost_inputs.iter().map(|value| value.id.as_str()),
            "/scenarios/cost_inputs",
        )?;
    }
    let mut sources = BTreeSet::new();
    for source in &draft.project.parameter_sources {
        let target = serde_json::to_string(&source.target).unwrap();
        if !sources.insert((target, &source.field_path)) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Duplicate parameter source target/path",
                Some("/parameter_sources"),
            ));
        }
        if let Some(expected) = &source.value {
            let actual = draft_value(draft, &source.target)
                .ok_or_else(|| missing(&source.target, "Parameter source target is absent"))?;
            if actual.pointer(&source.field_path)
                != serde_json::to_value(expected).unwrap().get("value")
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Parameter source value does not match its actual target leaf",
                    Some("/parameter_sources"),
                ));
            }
        }
    }
    Ok(())
}

fn branch(command: &EditCommand) -> HostEditBranch {
    match command {
        EditCommand::CreateProject(_) => HostEditBranch::CreateProject,
        EditCommand::PutObject(_) => HostEditBranch::PutObject,
        EditCommand::RemoveObject(_) => HostEditBranch::RemoveObject,
        EditCommand::SetProjectSettings(_) => HostEditBranch::SetProjectSettings,
        EditCommand::CreateDefinitionDraft(_) => HostEditBranch::CreateDefinitionDraft,
        EditCommand::PutDefinition(_) | EditCommand::PutTemplateComposition(_) => {
            HostEditBranch::PutDefinition
        }
        EditCommand::SetDeclaredParameter { .. } => HostEditBranch::SetDeclaredParameter,
        EditCommand::AdoptModule(_) => HostEditBranch::AdoptModule,
        EditCommand::InstantiateTemplate(_) => HostEditBranch::InstantiateTemplate,
        EditCommand::PutPackagingJob(_) => HostEditBranch::PutPackagingJob,
        EditCommand::RecordArtifactAcknowledgment(_) => {
            HostEditBranch::RecordArtifactAcknowledgment
        }
        EditCommand::AdoptImportedDraft { .. } => HostEditBranch::AdoptImportedDraft,
        EditCommand::CommitDefinitionVersion { .. } => HostEditBranch::CommitDefinitionVersion,
        EditCommand::Undo => HostEditBranch::Undo,
        EditCommand::Redo => HostEditBranch::Redo,
    }
}

fn request_header(request: ApiRequest) -> (ObjectId, Option<ProjectId>, ApiScope, OperationName) {
    macro_rules! header {
        ($r:expr, $op:ident) => {{
            let r = $r;
            (r.request_id, r.project_id, r.scope, OperationName::$op)
        }};
    }
    match request {
        ApiRequest::StartExecutionSession(r) => header!(r.into_inner(),StartExecutionSession),
        ApiRequest::GetExecutionSnapshot(r) => header!(r.into_inner(),GetExecutionSnapshot),
        ApiRequest::ControlExecutionSession(r) => header!(r.into_inner(),ControlExecutionSession),
        ApiRequest::ApplyDeviceReceipt(r) => header!(r.into_inner(),ApplyDeviceReceipt),
        ApiRequest::GetExecutionCloudWork(r) => header!(r.into_inner(),GetExecutionCloudWork),
        ApiRequest::AcknowledgeExecutionCloudWork(r) => header!(r.into_inner(),AcknowledgeExecutionCloudWork),
        ApiRequest::ApplyDeviceObservation(r) => header!(r.into_inner(),ApplyDeviceObservation),
        ApiRequest::ReadExecutionView(r) => header!(r.into_inner(),ReadExecutionView),
        ApiRequest::ExportExecutionReport(r) => header!(r.into_inner(),ExportExecutionReport),
        ApiRequest::ExportCloudData(r) => header!(r.into_inner(),ExportCloudData),
        ApiRequest::ReadCloudProduction(r) => header!(r.into_inner(),ReadCloudProduction),
        ApiRequest::DescribeCapabilities(r) => header!(r, DescribeCapabilities),
        ApiRequest::GetContext(r) => header!(r, GetContext),
        ApiRequest::GetObject(r) => header!(r, GetObject),
        ApiRequest::ApplyCommands(r) => header!(r, ApplyCommands),
        ApiRequest::RenderView(r) => header!(r, RenderView),
        ApiRequest::RunScenario(r) => header!(r, RunScenario),
        ApiRequest::GetRun(r) => header!(r, GetRun),
        ApiRequest::CompareRuns(r) => header!(r, CompareRuns),
        ApiRequest::ComputePackaging(r) => header!(r, ComputePackaging),
        ApiRequest::GetTask(r) => header!(r, GetTask),
        ApiRequest::CancelTask(r) => header!(r, CancelTask),
        ApiRequest::AdoptArtifact(r) => header!(r, AdoptArtifact),
        ApiRequest::ImportPackage(r) => header!(r, ImportPackage),
        ApiRequest::SaveProject(r) => header!(r, SaveProject),
        ApiRequest::ExportPackage(r) => header!(r, ExportPackage),
        ApiRequest::ExportEvaluation(r) => header!(r, ExportEvaluation),
        ApiRequest::QueryModuleLibrary(r) => header!(r, QueryModuleLibrary),
        ApiRequest::CheckFactoryDraft(r) => header!(r, CheckFactoryDraft),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn packaging_call(host: &mut ProjectHost, operation: &str, params: Value) -> Value {
        let project = host.project.as_ref().map(|project| project.id.clone());
        let request = serde_json::from_value(json!({"api_version":"0.1","request_id":"packaging-host-test","project_id":project,"scope":{"kind":if project.is_some(){"project"}else{"workspace"}},"operation":operation,"params":params})).unwrap();
        host.dispatch(
            &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
            request,
        )
    }

    fn packaging_host() -> ProjectHost {
        let mut host = ProjectHost::new(
            ObjectId::try_from("packaging-test-host".to_owned()).unwrap(),
            ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
        );
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"actual packaging","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        host
    }

    #[test]
    fn desktop_close_state_covers_scoped_io_and_uncommitted_saved_definition() {
        let mut host = packaging_host();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let directory = ProfessionalTestDirectory::new();
        let selection = host.select_directory(&ui, FilePurpose::SaveProject, directory.root.clone()).unwrap();
        let response = packaging_call(&mut host, "save_project", json!({"revision":1,"selected_directory_ref":selection}));
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let request = serde_json::from_value(json!({"api_version":"0.1","request_id":"close-workspace-scope","project_id":null,"scope":{"kind":"workspace"},"operation":"get_context","params":{"selection":[]}})).unwrap();
        let workspace = host.dispatch(&ui, request);
        assert_eq!(workspace["outcome"]["status"], "ok", "{workspace}");
        assert!(workspace["outcome"]["data"]["tasks"].as_array().unwrap().is_empty());
        assert!(host.desktop_close_state().active_tasks.contains(&task.id));
        let closing = host.desktop_close_state();
        assert!(matches!(host.finish_desktop_close(&closing, true, || panic!("active save must not destroy the window")), Err(ApiError { code: ErrorCode::InvalidInput, .. })));
        assert!(matches!(packaging_complete(&mut host, &task).task.state, ComputationStatus::Completed));
        assert!(!host.dirty);
        assert!(!host.desktop_close_state().unsaved);
        assert!(host.desktop_close_state().active_tasks.is_empty());

        let package: Value = serde_json::from_str(include_str!("../../../tools/type-contract-proof/fixtures/module-items.json")).unwrap();
        let created = packaging_call(&mut host, "apply_commands", json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":package["data"]["definition"]}]}));
        assert_eq!(created["outcome"]["status"], "ok", "{created}");
        // Saving a definition Scope does not change the Project's revision or
        // persisted dirty flag; its uncommitted content is nevertheless lost
        // when the process closes and must be included in the close decision.
        assert!(!host.dirty);
        let close = host.desktop_close_state();
        assert!(close.unsaved);
        assert_eq!(close.project_revision.unwrap().0.value(), 1);
        assert_eq!(close.definition_revisions.len(), 1);
        assert_eq!(close.definition_revisions[0].1.0.value(), 1);
    }

    #[test]
    fn desktop_close_rechecks_revision_and_import_admission_against_the_real_host() {
        let mut host = packaging_host();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let directory = ProfessionalTestDirectory::new();
        let selection = host.select_directory(&ui, FilePurpose::SaveProject, directory.root.clone()).unwrap();
        let response = packaging_call(&mut host, "save_project", json!({"revision":1,"selected_directory_ref":selection}));
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(packaging_complete(&mut host, &task).task.state, ComputationStatus::Completed));
        let clean = host.desktop_close_state();
        assert!(!clean.unsaved);
        let selection = host.select_directory(&ui, FilePurpose::ImportPackage, directory.root.clone()).unwrap();
        let imported = workspace_test_call(&mut host, "import_package", json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null}));
        assert_eq!(imported["outcome"]["status"], "ok", "{imported}");
        assert!(matches!(host.finish_desktop_close(&clean, false, || panic!("queued import must retain the window")), Err(ApiError { code: ErrorCode::RevisionConflict, .. })));
        let task: TaskHandle = serde_json::from_value(imported["outcome"]["data"].clone()).unwrap();
        let work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        for pending in work {
            host.finish_io(pending.run());
        }
        let completed = workspace_test_call(&mut host, "get_task", json!({"task_id":task.id}));
        assert_eq!(completed["outcome"]["data"]["task"]["state"], "completed", "{completed}");
        let clean = host.desktop_close_state();
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["title"] = json!("edit admitted after close snapshot");
        let edited = packaging_call(&mut host, "apply_commands", json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings}]}));
        assert_eq!(edited["outcome"]["status"], "ok", "{edited}");
        assert!(matches!(host.finish_desktop_close(&clean, false, || panic!("new edit must retain the window")), Err(ApiError { code: ErrorCode::RevisionConflict, .. })));
        assert!(host.desktop_close_state().unsaved);
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 2);
        assert!(!host.desktop_closing);
    }

    #[test]
    fn desktop_close_freezes_admission_after_confirmed_destruction() {
        let mut host = packaging_host();
        let expected = host.desktop_close_state();
        let mut destroyed = false;
        host.finish_desktop_close(&expected, true, || { destroyed = true; Ok(()) }).unwrap();
        assert!(destroyed);
        let request = packaging_call(&mut host, "get_context", json!({"selection":[]}));
        assert_eq!(request["outcome"]["status"], "error", "{request}");
        assert_eq!(request["outcome"]["data"]["code"], "InvalidInput", "{request}");
        assert!(request["outcome"]["data"]["message"].as_str().unwrap().contains("Desktop is closing"));
    }

    #[test]
    fn desktop_close_requires_discard_and_reopens_admission_on_native_failure() {
        let mut host = packaging_host();
        let expected = host.desktop_close_state();
        assert!(matches!(host.finish_desktop_close(&expected, false, || panic!("no discard confirmed")), Err(ApiError { code: ErrorCode::InvalidInput, .. })));
        assert!(matches!(host.finish_desktop_close(&expected, true, || Err("native destroy failed".into())), Err(ApiError { code: ErrorCode::ComputationFailed, .. })));
        assert!(!host.desktop_closing);
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["title"] = json!("still editable after refused close");
        let edited = packaging_call(&mut host, "apply_commands", json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings}]}));
        assert_eq!(edited["outcome"]["status"], "ok", "{edited}");
        assert!(host.dirty);
    }

    #[test]
    fn frozen_run_inputs_keep_the_original_scenario_and_exact_module_after_current_edits_save_and_open()
     {
        let mut host = local_production_host();
        stage_fixture_module_files(&mut host);
        let expected_project = host.project.as_ref().unwrap().clone();
        let expected_scenario = host.scenarios.values().next().unwrap().clone();
        let device = expected_project.devices[0].clone();
        let old_ref = device.module_ref.clone();
        let expected_module = host.modules[&module_key(&old_ref)].clone();
        let run = actual_local_run(&mut host);
        let frontier = run.computed_until_ms.as_ref().unwrap().0.value();
        let snapshot = json!({"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":frontier}});
        let source = json!({"kind":"run_module_version","data":{"run_id":run.id,"at_ms":frontier,"module_ref":old_ref}});
        let read = |host: &mut ProjectHost, kind: &str, id: &str, source: Value| {
            packaging_call(
                host,
                "get_object",
                json!({"object_id":{"kind":kind,"id":id},"source":source,"node_index":null}),
            )
        };
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let created = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":expected_module.definition}]}),
        );
        assert_eq!(created["outcome"]["status"], "ok", "{created}");
        let draft: ModuleId =
            serde_json::from_value(created["outcome"]["data"]["scope"]["data"]["id"].clone())
                .unwrap();
        let committed = authority_call(
            &mut host,
            &ui,
            ApiScope::DefinitionDraft(draft),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"commit_definition_version","data":{"version":"9.9.9"}}]}),
        );
        assert_eq!(committed["outcome"]["status"], "ok", "{committed}");
        let new_ref = ModuleRef {
            id: old_ref.id.clone(),
            version: VersionId::try_from("9.9.9".to_owned()).unwrap(),
        };
        let mut scenario = expected_scenario.clone();
        scenario.title = "current input differs from selected old Run".into();
        let edited = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"adopt_module","data":{"module_ref":new_ref,"targets":[{"kind":"device","id":device.id}],"bindings":device.role_bindings,"new_instances":[],"explicit_seed":null}},{"kind":"put_object","data":{"kind":"scenario","data":scenario}}]}),
        );
        assert_eq!(edited["outcome"]["status"], "ok", "{edited}");
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 2);
        assert_eq!(host.scenarios[&expected_scenario.id].revision.0.value(), 2);
        assert_eq!(
            module_key(&host.project.as_ref().unwrap().devices[0].module_ref),
            module_key(&new_ref)
        );
        for (kind, id, source, expected) in [
            (
                "project",
                expected_project.id.as_str(),
                snapshot.clone(),
                json!({"kind":"project","data":expected_project}),
            ),
            (
                "scenario",
                expected_scenario.id.as_str(),
                snapshot.clone(),
                json!({"kind":"scenario","data":expected_scenario}),
            ),
            (
                "module",
                old_ref.id.as_str(),
                source.clone(),
                json!({"kind":"module","data":expected_module}),
            ),
        ] {
            let response = read(&mut host, kind, id, source);
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            assert_eq!(response["outcome"]["data"], expected);
        }
        let directory = ProfessionalTestDirectory::new();
        let saved = directory.root.join("saved");
        std::fs::create_dir(&saved).unwrap();
        let selected = host
            .select_directory(&ui, FilePurpose::SaveProject, saved.clone())
            .unwrap();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":2,"selected_directory_ref":selected}),
        );
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        // A valid separately supplied editable revision may drop the old version
        // from its current selection; retained old Run inputs still require it.
        let backing = host.backing.as_ref().unwrap().clone();
        let mut current = host.current_draft();
        current
            .project
            .module_refs
            .retain(|reference| module_key(reference) != module_key(&old_ref));
        current.project.revision = next_revision(&current.project.revision).unwrap();
        for scenario in current.scenarios.values_mut() {
            scenario.project_revision = current.project.revision.clone();
            scenario.revision = next_revision(&scenario.revision).unwrap();
        }
        let moved = directory.root.join("changed-current-selection");
        std::fs::create_dir(&moved).unwrap();
        crate::write_project_archive(&moved, &current, Some(&backing), &AtomicBool::new(false))
            .unwrap();
        reopen_test_project(&mut host, &moved);
        assert!(
            !host
                .project
                .as_ref()
                .unwrap()
                .module_refs
                .iter()
                .any(|reference| module_key(reference) == module_key(&old_ref))
        );
        let before_local_read = host.project.as_ref().unwrap().clone();
        let current_old = read(
            &mut host,
            "module",
            old_ref.id.as_str(),
            json!({"kind":"module_version","data":{"module_ref":old_ref}}),
        );
        // Exact retained archive metadata is readable independently of current
        // selection. This does not adopt it or change the frozen Run source.
        assert_eq!(current_old["outcome"]["status"], "ok", "{current_old}");
        assert_eq!(
            current_old["outcome"]["data"],
            json!({"kind":"module","data":expected_module})
        );
        assert!(exact_data(
            host.project.as_ref().unwrap(),
            &before_local_read
        ));
        let actual = read(&mut host, "module", old_ref.id.as_str(), source.clone());
        assert_eq!(actual["outcome"]["status"], "ok", "{actual}");
        assert_eq!(
            actual["outcome"]["data"],
            json!({"kind":"module","data":expected_module})
        );
        let actual = read(
            &mut host,
            "scenario",
            expected_scenario.id.as_str(),
            snapshot.clone(),
        );
        assert_eq!(actual["outcome"]["status"], "ok", "{actual}");
        assert_eq!(
            actual["outcome"]["data"],
            json!({"kind":"scenario","data":expected_scenario})
        );
        let actual = read(
            &mut host,
            "project",
            expected_project.id.as_str(),
            snapshot.clone(),
        );
        assert_eq!(actual["outcome"]["status"], "ok", "{actual}");
        assert_eq!(
            actual["outcome"]["data"],
            json!({"kind":"project","data":expected_project})
        );
        assert!(exact_data(&host.runs[&run.id].result, &run));
        let project_before = host.project.as_ref().unwrap().clone();
        for (kind, id, mut invalid, code) in [
            ("module", "different-id", source.clone(), "InvalidInput"),
            (
                "device",
                old_ref.id.as_str(),
                source.clone(),
                "InvalidInput",
            ),
            (
                "module",
                old_ref.id.as_str(),
                json!({"kind":"run_module_version","data":{"run_id":run.id,"at_ms":frontier,"module_ref":new_ref}}),
                "MissingReference",
            ),
            (
                "module",
                old_ref.id.as_str(),
                json!({"kind":"run_module_version","data":{"run_id":run.id,"at_ms":frontier,"module_ref":{"id":old_ref.id,"version":"8.8.8"}}}),
                "MissingReference",
            ),
            (
                "scenario",
                "other-scenario",
                snapshot.clone(),
                "MissingReference",
            ),
            (
                "module",
                old_ref.id.as_str(),
                json!({"kind":"run_module_version","data":{"run_id":"unknown-run","at_ms":frontier,"module_ref":old_ref}}),
                "MissingReference",
            ),
        ] {
            let response = read(&mut host, kind, id, invalid.take());
            assert_eq!(response["outcome"]["data"]["code"], code, "{response}");
        }
        for (kind, id, mut request) in [
            ("module", old_ref.id.as_str(), source.clone()),
            ("scenario", expected_scenario.id.as_str(), snapshot.clone()),
        ] {
            request["data"]["at_ms"] = json!(frontier + 1);
            let response = read(&mut host, kind, id, request);
            assert_eq!(
                response["outcome"]["data"]["code"], "InvalidInput",
                "{response}"
            );
        }
        let no_run = ObjectId::try_from("object-authority-without-run-read".to_owned()).unwrap();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: no_run.clone(),
            granted_project: Some(project_before.id.clone()),
            allowed_operations: vec![OperationName::GetObject],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        for (kind, id, request) in [
            ("module", old_ref.id.as_str(), source.clone()),
            ("scenario", expected_scenario.id.as_str(), snapshot.clone()),
            ("project", expected_project.id.as_str(), snapshot.clone()),
        ] {
            let response = authority_call(
                &mut host,
                &no_run,
                ApiScope::Project,
                "get_object",
                json!({"object_id":{"kind":kind,"id":id},"source":request,"node_index":null}),
            );
            assert_eq!(
                response["outcome"]["data"]["code"], "PermissionDenied",
                "{response}"
            );
            assert!(response["context"].is_null());
        }
        let module_paths: Vec<_> = host
            .backing
            .as_ref()
            .unwrap()
            .manifest
            .documents
            .iter()
            .filter(|entry| {
                entry
                    .module_ref
                    .as_ref()
                    .is_some_and(|reference| module_key(reference) == module_key(&old_ref))
            })
            .map(|entry| entry.file_ref.path.as_str().to_owned())
            .collect();
        assert!(!module_paths.is_empty());
        let removed: Vec<_> = module_paths
            .iter()
            .filter_map(|path| {
                host.backing
                    .as_mut()
                    .unwrap()
                    .files
                    .remove(path)
                    .map(|bytes| (path.clone(), bytes))
            })
            .collect();
        assert!(
            host.modules.contains_key(&module_key(&old_ref)),
            "a loaded registry entry cannot replace missing historical bytes"
        );
        let missing = read(&mut host, "module", old_ref.id.as_str(), source.clone());
        assert_eq!(
            missing["outcome"]["data"]["code"], "MissingReference",
            "{missing}"
        );
        for (path, bytes) in removed {
            host.backing.as_mut().unwrap().files.insert(path, bytes);
        }
        let path = module_paths[0].clone();
        let original = host.backing.as_mut().unwrap().files.remove(&path).unwrap();
        let mut malformed: Value = serde_json::from_slice(&original).unwrap();
        malformed["data"]["definition"]["version"] = json!("9.9.9");
        host.backing
            .as_mut()
            .unwrap()
            .files
            .insert(path.clone(), serde_json::to_vec(&malformed).unwrap());
        let invalid = read(&mut host, "module", old_ref.id.as_str(), source.clone());
        assert_eq!(
            invalid["outcome"]["data"]["code"], "InvalidInput",
            "{invalid}"
        );
        host.backing.as_mut().unwrap().files.insert(path, original);
        let path = run.input_ref.frozen_scenario_file.path.as_str().to_owned();
        let original = host.backing.as_mut().unwrap().files.remove(&path).unwrap();
        let missing = read(
            &mut host,
            "scenario",
            expected_scenario.id.as_str(),
            snapshot.clone(),
        );
        assert_eq!(
            missing["outcome"]["data"]["code"], "MissingReference",
            "{missing}"
        );
        for field in ["id", "revision", "project_revision"] {
            let mut malformed: Value = serde_json::from_slice(&original).unwrap();
            malformed["data"][field] = if field == "id" {
                json!("not-the-frozen-scenario")
            } else {
                json!(99)
            };
            host.backing
                .as_mut()
                .unwrap()
                .files
                .insert(path.clone(), serde_json::to_vec(&malformed).unwrap());
            let invalid = read(
                &mut host,
                "scenario",
                expected_scenario.id.as_str(),
                snapshot.clone(),
            );
            assert_eq!(
                invalid["outcome"]["data"]["code"], "InvalidInput",
                "{invalid}"
            );
        }
        host.backing.as_mut().unwrap().files.insert(path, original);
        let path = run.input_ref.frozen_project_file.path.as_str().to_owned();
        let original = host.backing.as_mut().unwrap().files.remove(&path).unwrap();
        let missing = read(&mut host, "module", old_ref.id.as_str(), source.clone());
        assert_eq!(
            missing["outcome"]["data"]["code"], "MissingReference",
            "{missing}"
        );
        host.backing.as_mut().unwrap().files.insert(path, original);
        assert!(exact_data(host.project.as_ref().unwrap(), &project_before));
        assert!(exact_data(&host.runs[&run.id].result, &run));
        let created = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":project_before.revision,"commands":[{"kind":"create_definition_draft","data":expected_module.definition}]}),
        );
        assert_eq!(created["outcome"]["status"], "ok", "{created}");
        let draft: ModuleId =
            serde_json::from_value(created["outcome"]["data"]["scope"]["data"]["id"].clone())
                .unwrap();
        for (kind, id, request) in [
            ("module", old_ref.id.as_str(), source.clone()),
            ("scenario", expected_scenario.id.as_str(), snapshot.clone()),
        ] {
            let wrong_scope = authority_call(
                &mut host,
                &ui,
                ApiScope::DefinitionDraft(draft.clone()),
                "get_object",
                json!({"object_id":{"kind":kind,"id":id},"source":request,"node_index":null}),
            );
            assert_eq!(
                wrong_scope["outcome"]["data"]["code"], "InvalidInput",
                "{wrong_scope}"
            );
        }
        let mut foreign = task_authority_caller(&host, "foreign-project-frozen-read");
        foreign.granted_project = Some(ProjectId::try_from("foreign-project".to_owned()).unwrap());
        foreign.allowed_operations = vec![OperationName::GetObject, OperationName::GetRun];
        let foreign_id = foreign.session_id.clone();
        host.register_external_session(foreign).unwrap();
        let denied = authority_call(
            &mut host,
            &foreign_id,
            ApiScope::Project,
            "get_object",
            json!({"object_id":{"kind":"module","id":old_ref.id},"source":source,"node_index":null}),
        );
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(denied["context"].is_null());
    }

    #[test]
    fn external_authority_updates_require_real_ui_and_preserve_committed_creation_budget() {
        let mut host = packaging_host();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let session = ObjectId::try_from("authority-update-agent".to_owned()).unwrap();
        let project = host.project.as_ref().unwrap().id.clone();
        let field = |path: &str| HostFieldGrant {
            pointer: path.to_owned(),
            element_id: None,
        };
        let grant = HostEditGrant {
            id: ObjectId::try_from("retained-budget".to_owned()).unwrap(),
            scope: TaskScope::Edit(EditScope::Project(project.clone())),
            branches: vec![
                HostEditBranch::PutObject,
                HostEditBranch::SetProjectSettings,
            ],
            objects: vec![HostObjectEditGrant {
                target: ObjectRef {
                    kind: ObjectKind::Project,
                    id: ObjectId::try_from(project.as_str().to_owned()).unwrap(),
                },
                fields: vec![field("/title")],
                can_remove: false,
            }],
            creates: vec![HostCreateAllowance {
                kind: ObjectKind::ItemType,
                max_new_objects: SafeUint::try_from(1).unwrap(),
                fields: ["/revision", "/title", "/unit", "/tracking", "/spec_ref"]
                    .map(field)
                    .to_vec(),
            }],
            source_versions: vec![],
            reusable_resources: vec![],
            created_objects: vec![],
        };
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: session.clone(),
            granted_project: Some(project),
            allowed_operations: vec![
                OperationName::DescribeCapabilities,
                OperationName::GetObject,
                OperationName::ApplyCommands,
            ],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![grant],
        })
        .unwrap();
        let item = |id: &str| json!({"id":id,"revision":"r1","title":"created once","unit":"piece","tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]});
        let created = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"item_type","data":item("A")}}]}),
        );
        assert_eq!(created["outcome"]["status"], "ok", "{created}");
        let original = host.caller_context(&session).unwrap().clone();
        assert_eq!(original.edit_grants[0].created_objects.len(), 1);
        assert!(matches!(
            host.update_external_session_authority(&session, original.clone())
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        assert!(matches!(
            host.update_external_session_authority(
                &ObjectId::try_from("unknown-ui".to_owned()).unwrap(),
                original.clone()
            )
            .unwrap_err()
            .code,
            ErrorCode::PermissionDenied
        ));
        let mut forged = original.clone();
        forged.edit_grants[0].created_objects[0].id =
            ObjectId::try_from("forged-B".to_owned()).unwrap();
        assert!(matches!(
            host.update_external_session_authority(&ui, forged)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        let mut forged = original.clone();
        forged.origin = CallerOrigin::InteractiveUi;
        assert!(matches!(
            host.update_external_session_authority(&ui, forged)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        let mut forged = original.clone();
        forged.session_id = ui.clone();
        assert!(matches!(
            host.update_external_session_authority(&ui, forged)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        assert!(matches!(
            host.revoke_external_session(&ui).unwrap_err().code,
            ErrorCode::PermissionDenied
        ));
        let mut limited = original.clone();
        limited.allowed_operations.retain(|operation| {
            !matches!(
                operation,
                OperationName::ApplyCommands | OperationName::GetObject
            )
        });
        limited.edit_grants[0].created_objects.clear();
        limited.edit_grants[0].objects.clear();
        host.update_external_session_authority(&ui, limited.clone())
            .unwrap();
        assert!(exact_data(
            &host.caller_context(&session).unwrap().edit_grants[0].created_objects,
            &original.edit_grants[0].created_objects
        ));
        let catalog = authority_call(
            &mut host,
            &session,
            ApiScope::Workspace,
            "describe_capabilities",
            json!({}),
        );
        assert_eq!(catalog["outcome"]["status"], "ok", "{catalog}");
        assert!(
            catalog["outcome"]["data"]["available_command_kinds"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(!catalog["outcome"]["data"]["implemented_operations"].as_array().unwrap().iter().any(|cap|cap["operation"]=="get_object"||cap["operation"]=="apply_commands"));
        let unchanged = host.project.as_ref().unwrap().clone();
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"put_object","data":{"kind":"item_type","data":item("B")}}]}),
        );
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(denied["context"].is_null());
        limited
            .allowed_operations
            .push(OperationName::ApplyCommands);
        host.update_external_session_authority(&ui, limited)
            .unwrap();
        let catalog = authority_call(
            &mut host,
            &session,
            ApiScope::Workspace,
            "describe_capabilities",
            json!({}),
        );
        assert!(
            catalog["outcome"]["data"]["available_command_kinds"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{catalog}"
        );
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"put_object","data":{"kind":"item_type","data":item("B")}}]}),
        );
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        let mut settings = settings_value(&unchanged);
        settings["title"] = json!("removed field cannot change");
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(exact_data(host.project.as_ref().unwrap(), &unchanged));
        assert!(exact_data(
            &host.caller_context(&session).unwrap().edit_grants[0].created_objects,
            &original.edit_grants[0].created_objects
        ));
    }

    fn authority_call(
        host: &mut ProjectHost,
        session: &ObjectId,
        scope: ApiScope,
        operation: &str,
        params: Value,
    ) -> Value {
        let project = if matches!(scope, ApiScope::Workspace) {
            None
        } else {
            host.project.as_ref().map(|project| project.id.clone())
        };
        host.dispatch(session,serde_json::from_value(json!({"api_version":"0.1","request_id":"actual-authority-test","project_id":project,"scope":scope,"operation":operation,"params":params})).unwrap())
    }

    fn task_authority_caller(host: &ProjectHost, name: &str) -> HostCallContext {
        HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: ObjectId::try_from(name.to_owned()).unwrap(),
            granted_project: host.project.as_ref().map(|project| project.id.clone()),
            allowed_operations: vec![
                OperationName::DescribeCapabilities,
                OperationName::RunScenario,
                OperationName::SaveProject,
                OperationName::GetTask,
                OperationName::CancelTask,
            ],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        }
    }

    #[test]
    fn external_reconnect_grants_only_the_original_task_and_never_restores_files_or_resubmits_work()
    {
        let mut host = local_production_host();
        stage_fixture_module_files(&mut host);
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let old = task_authority_caller(&host, "old-connection");
        let old_id = old.session_id.clone();
        host.register_external_session(old).unwrap();
        let directory = ProfessionalTestDirectory::new();
        let old_token = host
            .select_directory(&old_id, FilePurpose::SaveProject, directory.root.clone())
            .unwrap();
        let window = host.scenarios.values().next().unwrap().window.clone();
        let response = authority_call(
            &mut host,
            &old_id,
            ApiScope::Project,
            "run_scenario",
            json!({"scenario_id":"LOCAL","input_revision":1,"observation_window":window,"intent":{"kind":"project_scenario"}}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let original = host.tasks[&task.id].state.clone();
        let next_task = host.next_task;
        host.revoke_external_session(&old_id).unwrap();
        let reused = task_authority_caller(&host, "old-connection");
        assert!(matches!(
            host.register_external_session(reused).unwrap_err().code,
            ErrorCode::PermissionDenied
        ));
        assert!(!host.tasks[&task.id].cancelled.load(Ordering::Acquire));
        assert!(exact_data(&host.tasks[&task.id].state, &original));
        let response = authority_call(
            &mut host,
            &old_id,
            ApiScope::Project,
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(response["outcome"]["data"]["code"], "PermissionDenied");
        assert!(response["context"].is_null());
        let fresh = task_authority_caller(&host, "fresh-connection");
        let fresh_id = fresh.session_id.clone();
        host.register_external_session(fresh.clone()).unwrap();
        let no_grant = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(no_grant["outcome"]["data"]["code"], "PermissionDenied");
        let mut forged = fresh.clone();
        forged
            .selected_file_tokens
            .push(old_token.selection_id.clone());
        assert!(matches!(
            host.update_external_session_authority(&ui, forged)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        let no_file = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "save_project",
            json!({"revision":1,"selected_directory_ref":old_token}),
        );
        assert_eq!(no_file["outcome"]["data"]["code"], "PermissionDenied");
        assert!(matches!(
            host.grant_external_task_access(&fresh_id, &fresh_id, &task.id, false)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        assert!(matches!(
            host.grant_external_task_access(&ui, &ui, &task.id, false)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        assert!(matches!(
            host.grant_external_task_access(
                &ui,
                &fresh_id,
                &TaskId::try_from("missing-task".to_owned()).unwrap(),
                false
            )
            .unwrap_err()
            .code,
            ErrorCode::MissingReference
        ));
        let mut wrong = task_authority_caller(&host, "wrong-project");
        wrong.granted_project = Some(ProjectId::try_from("OTHER".to_owned()).unwrap());
        let wrong_id = wrong.session_id.clone();
        host.register_external_session(wrong).unwrap();
        assert!(matches!(
            host.grant_external_task_access(&ui, &wrong_id, &task.id, false)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        host.grant_external_task_access(&ui, &fresh_id, &task.id, false)
            .unwrap();
        let read = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(read["outcome"]["status"], "ok", "{read}");
        let readonly: TaskState = serde_json::from_value(read["outcome"]["data"].clone()).unwrap();
        assert!(exact_data(&readonly.task, &original.task));
        assert!(same(
            &readonly.originating_operation,
            &original.originating_operation
        ));
        assert!(readonly.allowed_controls.is_empty());
        let wrong_scope = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Workspace,
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(
            wrong_scope["outcome"]["data"]["code"], "PermissionDenied",
            "{wrong_scope}"
        );
        let denied = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "cancel_task",
            json!({"task_id":task.id,"action":"cancel"}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        host.grant_external_task_access(&ui, &fresh_id, &task.id, true)
            .unwrap();
        let mut no_control = fresh.clone();
        no_control
            .allowed_operations
            .retain(|operation| !matches!(operation, OperationName::CancelTask));
        host.update_external_session_authority(&ui, no_control)
            .unwrap();
        let denied = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "cancel_task",
            json!({"task_id":task.id,"action":"cancel"}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        host.update_external_session_authority(&ui, fresh).unwrap();
        let unsupported = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "cancel_task",
            json!({"task_id":task.id,"action":"pause_calculation"}),
        );
        assert_eq!(
            unsupported["outcome"]["data"]["code"],
            "UnsupportedCapability"
        );
        assert!(!host.tasks[&task.id].cancelled.load(Ordering::Acquire));
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let read = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(
            read["outcome"]["data"]["task"]["state"], "completed",
            "{read}"
        );
        let completed: TaskState = serde_json::from_value(read["outcome"]["data"].clone()).unwrap();
        assert_eq!(completed.task.id, task.id);
        assert_eq!(completed.task.input_revision.0.value(), 1);
        assert!(exact_data(&completed.task.scope, &task.scope));
        assert!(matches!(
            completed.originating_operation,
            OperationName::RunScenario
        ));
        let Some(TaskOutput::Run(summary)) = &completed.result else {
            panic!("original actual Run result")
        };
        assert_eq!(
            host.runs[&summary.run_id].result.acceptance_events[0]
                .at_ms
                .0
                .value(),
            75000
        );
        assert_eq!(
            host.next_task, next_task,
            "authority changes never re-enqueue the original task"
        );
        let late = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "cancel_task",
            json!({"task_id":task.id,"action":"cancel"}),
        );
        assert_eq!(late["outcome"]["data"]["terminal_won"], true);
        assert!(exact_data(&host.tasks[&task.id].state, &completed));
        host.grant_external_task_access(&ui, &fresh_id, &task.id, false)
            .unwrap();
        let denied = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "cancel_task",
            json!({"task_id":task.id,"action":"cancel"}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        host.revoke_external_session(&fresh_id).unwrap();
        assert!(
            !host
                .external_task_access
                .keys()
                .any(|(session, _)| session == &fresh_id)
        );
        let denied = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        assert!(denied["context"].is_null());
    }

    #[test]
    fn external_task_access_respects_definition_scope_cancel_commit_boundary_and_project_switch() {
        let mut host = local_production_host();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let definition = host.modules.values().next().unwrap().definition.clone();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":definition}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let draft_id: ModuleId =
            serde_json::from_value(response["outcome"]["data"]["scope"]["data"]["id"].clone())
                .unwrap();
        let scope = ApiScope::DefinitionDraft(draft_id.clone());
        let mut original = task_authority_caller(&host, "definition-original");
        original
            .allowed_operations
            .push(OperationName::ComputePackaging);
        original.allowed_definition_drafts.push(draft_id.clone());
        let original_id = original.session_id.clone();
        host.register_external_session(original).unwrap();
        let cases: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/packaging-acceptance-cases.json"
        ))
        .unwrap();
        let job = cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == "HOLD01")
            .unwrap()["job"]["data"]
            .clone();
        let response = authority_call(
            &mut host,
            &original_id,
            scope.clone(),
            "compute_packaging",
            json!({"packaging_job":job,"input_revision":1,"computation_budget":job["budget"]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let next_task = host.next_task;
        host.revoke_external_session(&original_id).unwrap();
        let mut fresh = task_authority_caller(&host, "definition-reconnected");
        let fresh_id = fresh.session_id.clone();
        host.register_external_session(fresh.clone()).unwrap();
        assert!(matches!(
            host.grant_external_task_access(&ui, &fresh_id, &task.id, false)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        fresh.allowed_definition_drafts.push(draft_id.clone());
        host.update_external_session_authority(&ui, fresh.clone())
            .unwrap();
        host.grant_external_task_access(&ui, &fresh_id, &task.id, true)
            .unwrap();
        let wrong = authority_call(
            &mut host,
            &fresh_id,
            ApiScope::Project,
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(wrong["outcome"]["data"]["code"], "PermissionDenied");
        let read = authority_call(
            &mut host,
            &fresh_id,
            scope.clone(),
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(read["outcome"]["status"], "ok", "{read}");
        assert_eq!(
            read["outcome"]["data"]["task"]["scope"],
            serde_json::to_value(&task.scope).unwrap()
        );
        let mut removed = fresh.clone();
        removed.allowed_definition_drafts.clear();
        host.update_external_session_authority(&ui, removed)
            .unwrap();
        let denied = authority_call(
            &mut host,
            &fresh_id,
            scope.clone(),
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        assert!(denied["context"].is_null());
        host.update_external_session_authority(&ui, fresh).unwrap();
        let cancel = authority_call(
            &mut host,
            &fresh_id,
            scope.clone(),
            "cancel_task",
            json!({"task_id":task.id,"action":"cancel"}),
        );
        assert_eq!(
            cancel["outcome"]["data"]["action_accepted"], true,
            "{cancel}"
        );
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let finished = authority_call(
            &mut host,
            &fresh_id,
            scope.clone(),
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(
            finished["outcome"]["data"]["task"]["state"], "cancelled",
            "{finished}"
        );
        assert_eq!(host.next_task, next_task);
        assert!(host.professional.artifacts.is_empty());
        assert!(
            host.definition_drafts[&draft_id]
                .professional
                .artifacts
                .is_empty()
        );
        let switched = workspace_test_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"after original tasks reached terminal","time_reference":{"label":"explicit"},"departure":"discard_after_tasks_terminal"}}]}),
        );
        assert_eq!(switched["outcome"]["status"], "ok", "{switched}");
        assert!(host.external_task_access.is_empty());
        assert!(host.caller_context(&fresh_id).is_none());
        let reused = task_authority_caller(&host, "definition-reconnected");
        assert!(matches!(
            host.register_external_session(reused).unwrap_err().code,
            ErrorCode::PermissionDenied
        ));
        let later = task_authority_caller(&host, "different-project-reconnect");
        let later_id = later.session_id.clone();
        host.register_external_session(later).unwrap();
        assert!(matches!(
            host.grant_external_task_access(&ui, &later_id, &task.id, false)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
    }

    #[test]
    fn actual_ui_run_snapshot_reads_its_frozen_project_after_edits_and_reopen() {
        let mut host = local_production_host();
        let expected = host.project.as_ref().unwrap().clone();
        let run = actual_local_run(&mut host);
        let frontier = run.computed_until_ms.as_ref().unwrap().0.value();
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["title"] = json!("new draft differs from selected old Run");
        let edited = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(edited["outcome"]["status"], "ok", "{edited}");
        let read = |host: &mut ProjectHost, at: u64, id: &str| {
            packaging_call(
                host,
                "get_object",
                json!({"object_id":{"kind":"project","id":id},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":at}},"node_index":null}),
            )
        };
        let response = read(&mut host, frontier, expected.id.as_str());
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let DomainObject::Project(frozen) =
            serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
        else {
            panic!("actual frozen Project")
        };
        assert!(exact_data(&expected, &frozen));
        assert_eq!(frozen.revision.0.value(), 1);
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 2);
        let beyond = read(&mut host, frontier + 1, expected.id.as_str());
        assert_eq!(
            beyond["outcome"]["data"]["code"], "InvalidInput",
            "{beyond}"
        );
        let wrong = read(&mut host, frontier, "different-project");
        assert_eq!(
            wrong["outcome"]["data"]["code"], "MissingReference",
            "{wrong}"
        );
        stage_fixture_module_files(&mut host);
        let directory = ProfessionalTestDirectory::new();
        let selection = host
            .select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::SaveProject,
                directory.root.clone(),
            )
            .unwrap();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":2,"selected_directory_ref":selection}),
        );
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        reopen_test_project(&mut host, &directory.root);
        let response = read(&mut host, frontier, expected.id.as_str());
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let DomainObject::Project(reopened) =
            serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
        else {
            panic!("stored frozen Project")
        };
        assert!(exact_data(&expected, &reopened));
        assert!(exact_data(&host.runs[&run.id].result, &run));
        let external = ObjectId::try_from("out-of-project-run-read".to_owned()).unwrap();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(ProjectId::try_from("other-project".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::GetObject],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        let denied=host.dispatch(&external,serde_json::from_value(json!({"api_version":"0.1","request_id":"denied-frozen-project","project_id":expected.id,"scope":{"kind":"project"},"operation":"get_object","params":{"object_id":{"kind":"project","id":expected.id},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":frontier}},"node_index":null}})).unwrap());
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(denied["context"].is_null());
        // The frontend can stage an Open while this Project is active, but its
        // task remains Workspace-owned rather than adopting the visible Project.
        let selected = host
            .select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::ImportPackage,
                directory.root.clone(),
            )
            .unwrap();
        let opened = workspace_test_call(
            &mut host,
            "import_package",
            json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
        );
        assert_eq!(opened["outcome"]["status"], "ok", "{opened}");
        let pending: TaskHandle =
            serde_json::from_value(opened["outcome"]["data"].clone()).unwrap();
        let wrong_cancel = packaging_call(
            &mut host,
            "cancel_task",
            json!({"task_id":pending.id,"action":"cancel"}),
        );
        assert_eq!(
            wrong_cancel["outcome"]["data"]["code"], "PermissionDenied",
            "{wrong_cancel}"
        );
        assert!(wrong_cancel["context"].is_null());
        let cancel = workspace_test_call(
            &mut host,
            "cancel_task",
            json!({"task_id":pending.id,"action":"cancel"}),
        );
        assert_eq!(
            cancel["outcome"]["data"]["action_accepted"], true,
            "{cancel}"
        );
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let cancelled = workspace_test_call(&mut host, "get_task", json!({"task_id":pending.id}));
        assert_eq!(
            cancelled["outcome"]["data"]["task"]["state"], "cancelled",
            "{cancelled}"
        );
    }

    #[test]
    fn exact_module_reads_resolve_declared_versions_without_exposing_other_sources() {
        let mut host = local_production_host();
        let first = host.modules.values().next().unwrap().clone();
        let mut second = first.clone();
        second.definition.version = VersionId::try_from("9.9.9".to_owned()).unwrap();
        second.definition.title = "explicit second version".into();
        let reference = ModuleRef {
            id: second.definition.id.clone(),
            version: second.definition.version.clone(),
        };
        host.modules.insert(module_key(&reference), second.clone());
        host.project
            .as_mut()
            .unwrap()
            .module_refs
            .push(reference.clone());
        for expected in [&first, &second] {
            let response = packaging_call(
                &mut host,
                "get_object",
                json!({"object_id":{"kind":"module","id":expected.definition.id},"source":{"kind":"module_version","data":{"module_ref":{"id":expected.definition.id,"version":expected.definition.version}}},"node_index":null}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            let DomainObject::Module(actual) =
                serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
            else {
                panic!("actual module")
            };
            assert!(exact_data(expected, &actual));
        }
        let ambiguous = packaging_call(
            &mut host,
            "get_object",
            json!({"object_id":{"kind":"module","id":first.definition.id},"source":{"kind":"draft"},"node_index":null}),
        );
        assert_eq!(ambiguous["outcome"]["data"]["code"], "InvalidInput");
        for (kind, id, version, code) in [
            (
                "module",
                first.definition.id.as_str(),
                "8.8.8",
                "MissingReference",
            ),
            ("module", "different-id", "9.9.9", "InvalidInput"),
            (
                "device",
                first.definition.id.as_str(),
                "9.9.9",
                "InvalidInput",
            ),
        ] {
            let response = packaging_call(
                &mut host,
                "get_object",
                json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"module_version","data":{"module_ref":{"id":first.definition.id,"version":version}}},"node_index":null}),
            );
            assert_eq!(response["outcome"]["data"]["code"], code, "{response}");
        }
        host.project
            .as_mut()
            .unwrap()
            .module_refs
            .retain(|r| module_key(r) != module_key(&reference));
        let unreferenced = packaging_call(
            &mut host,
            "get_object",
            json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null}),
        );
        assert_eq!(unreferenced["outcome"]["data"]["code"], "MissingReference");
        host.project
            .as_mut()
            .unwrap()
            .module_refs
            .push(reference.clone());
        host.modules.remove(&module_key(&reference));
        let unloaded = packaging_call(
            &mut host,
            "get_object",
            json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null}),
        );
        assert_eq!(unloaded["outcome"]["data"]["code"], "MissingReference");
        let created = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":first.definition}]}),
        );
        assert_eq!(created["outcome"]["status"], "ok", "{created}");
        let project = host.project.as_ref().unwrap().id.clone();
        let project_before = serde_json::to_value(host.project.as_ref().unwrap()).unwrap();
        let request=serde_json::from_value(json!({"api_version":"0.1","request_id":"scope-exact-module","project_id":project,"scope":{"kind":"definition_draft","data":created["outcome"]["data"]["scope"]["data"]["id"]},"operation":"get_object","params":{"object_id":{"kind":"module","id":first.definition.id},"source":{"kind":"module_version","data":{"module_ref":{"id":first.definition.id,"version":first.definition.version}}},"node_index":null}})).unwrap();
        let response = host.dispatch(
            &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
            request,
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let DomainObject::Module(actual) =
            serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
        else {
            panic!("actual draft module")
        };
        assert!(exact_data(&actual.definition, &first.definition));
        assert_eq!(response["context"]["scope"]["kind"], "definition_draft");
        assert_eq!(
            response["context"]["scope"]["data"]["id"],
            created["outcome"]["data"]["scope"]["data"]["id"]
        );
        assert_eq!(response["context"]["scope"]["data"]["revision"], 1);
        assert_eq!(response["context"]["scope_revision"], 1);
        assert_eq!(
            serde_json::to_value(host.project.as_ref().unwrap()).unwrap(),
            project_before
        );
    }

    #[test]
    fn native_file_selection_binds_the_actual_picker_kind_session_purpose_and_project() {
        let directory = ProfessionalTestDirectory::new();
        let mut host = packaging_host();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let destination = directory.root.join("actual-project.sfproj");
        let selection = host
            .select_file(&ui, FilePurpose::ExportPackage, destination.clone())
            .unwrap();
        assert!(
            !destination.exists(),
            "selecting a destination must not create a file"
        );
        assert_eq!(
            serde_json::to_value(&selection)
                .unwrap()
                .as_object()
                .unwrap()
                .len(),
            2
        );
        assert!(
            !serde_json::to_string(&selection)
                .unwrap()
                .contains("actual-project.sfproj")
        );
        let caller = host.caller_context(&ui).unwrap().clone();
        let (actual, kind) = host
            .selected_path(&caller, &selection, &FilePurpose::ExportPackage)
            .unwrap();
        assert_eq!(actual, directory.root.join("actual-project.sfproj"));
        assert!(matches!(kind, SelectedPathKind::File));
        assert!(matches!(
            host.selected_directory(&caller, &selection, &FilePurpose::ExportPackage)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        ));
        assert!(matches!(
            host.selected_path(&caller, &selection, &FilePurpose::ImportPackage)
                .err()
                .unwrap()
                .code,
            ErrorCode::PermissionDenied
        ));
        assert!(matches!(
            host.select_file(&ui, FilePurpose::SaveProject, destination.clone())
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        ));
        assert!(matches!(
            host.select_file(&ui, FilePurpose::ImportPackage, destination.clone())
                .unwrap_err()
                .code,
            ErrorCode::IoFailure
        ));
        std::fs::write(&destination, b"unchanged until an actual worker commit").unwrap();
        let selected_import = host
            .select_file(&ui, FilePurpose::ImportPackage, destination.clone())
            .unwrap();
        assert!(matches!(
            host.selected_path(
                &host.caller_context(&ui).unwrap().clone(),
                &selected_import,
                &FilePurpose::ImportPackage
            )
            .unwrap()
            .1,
            SelectedPathKind::File
        ));
        let response = workspace_test_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"another project","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"discard_after_tasks_terminal"}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(matches!(
            host.selected_path(
                &host.caller_context(&ui).unwrap().clone(),
                &selection,
                &FilePurpose::ExportPackage
            )
            .err()
            .unwrap()
            .code,
            ErrorCode::PermissionDenied
        ));
        assert_eq!(
            std::fs::read(destination).unwrap(),
            b"unchanged until an actual worker commit"
        );
    }

    #[test]
    fn comparison_input_adapter_uses_each_actual_frozen_run_before_and_after_save_open() {
        let mut host = local_production_host();
        let original_title = host.project.as_ref().unwrap().title.clone();
        let first = actual_local_run(&mut host);
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["title"] = json!("second frozen scenario");
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let second = actual_local_run(&mut host);
        let snapshots = host
            .comparison_inputs(&[first.id.clone(), second.id.clone()])
            .unwrap();
        assert_eq!(snapshots[0].0.title, original_title);
        assert_eq!(snapshots[1].0.title, "second frozen scenario");
        assert_eq!(snapshots[0].1.project_revision.0.value(), 1);
        assert_eq!(snapshots[1].1.project_revision.0.value(), 2);
        assert!(exact_data(&snapshots[0].3, &first));
        assert!(exact_data(&snapshots[1].3, &second));
        assert!(matches!(
            host.comparison_inputs(&[first.id.clone(), first.id.clone()])
                .err()
                .unwrap()
                .code,
            ErrorCode::InvalidInput
        ));
        assert!(matches!(
            host.comparison_inputs(&[
                first.id.clone(),
                RunId::try_from("missing-run".to_owned()).unwrap()
            ])
            .err()
            .unwrap()
            .code,
            ErrorCode::MissingReference
        ));
        for (index, package) in host.modules.values().enumerate() {
            let reference = ModuleRef {
                id: package.definition.id.clone(),
                version: package.definition.version.clone(),
            };
            host.additions.push(ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::ModuleDefinition,
                    module_ref: Some(reference),
                    object_ref: ObjectRef {
                        kind: ObjectKind::Module,
                        id: ObjectId::try_from(package.definition.id.as_str().to_owned()).unwrap(),
                    },
                    revision: None,
                    file_ref: file_ref(
                        &format!("comparison-modules/{index}.json"),
                        DocumentKind::Module,
                    ),
                    required: true,
                },
                bytes: encode_document(package),
            });
        }
        let directory = ProfessionalTestDirectory::new();
        let selected = host
            .select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::SaveProject,
                directory.root.clone(),
            )
            .unwrap();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":2,"selected_directory_ref":selected}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        reopen_test_project(&mut host, &directory.root);
        let reopened = host
            .comparison_inputs(&[first.id.clone(), second.id.clone()])
            .unwrap();
        for (expected, actual) in snapshots.iter().zip(&reopened) {
            assert!(exact_data(&expected.0, &actual.0));
            assert!(exact_data(&expected.1, &actual.1));
            assert!(exact_data(&expected.2, &actual.2));
            assert!(exact_data(&expected.3, &actual.3));
        }
        let context = packaging_call(&mut host, "get_context", json!({"selection":[]}));
        let summaries: Vec<RunSummary> =
            serde_json::from_value(context["outcome"]["data"]["run_summaries"].clone()).unwrap();
        assert_eq!(summaries.len(), 2);
        for result in [&first, &second] {
            assert!(
                summaries
                    .iter()
                    .any(|summary| exact_data(summary, &run_summary(result)))
            );
        }
        let caller = ObjectId::try_from("context-without-run-authority".to_owned()).unwrap();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: caller.clone(),
            granted_project: Some(host.project.as_ref().unwrap().id.clone()),
            allowed_operations: vec![OperationName::GetContext],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        let response=host.dispatch(&caller,serde_json::from_value(json!({"api_version":"0.1","request_id":"limited-context","project_id":host.project.as_ref().unwrap().id,"scope":{"kind":"project"},"operation":"get_context","params":{"selection":[]}})).unwrap());
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(response["outcome"]["data"].get("run_summaries").is_none());
        host.sessions.get_mut(&caller).unwrap().granted_project = None;
        host.sessions
            .get_mut(&caller)
            .unwrap()
            .allowed_operations
            .push(OperationName::GetRun);
        let response=host.dispatch(&caller,serde_json::from_value(json!({"api_version":"0.1","request_id":"invisible-context","project_id":null,"scope":{"kind":"workspace"},"operation":"get_context","params":{"selection":[]}})).unwrap());
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(response["outcome"]["data"].get("run_summaries").is_none());
        let definition = host.modules.values().next().unwrap().definition.clone();
        let created = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"create_definition_draft","data":definition}]}),
        );
        assert_eq!(created["outcome"]["status"], "ok", "{created}");
        let response=host.dispatch(&ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),serde_json::from_value(json!({"api_version":"0.1","request_id":"draft-context","project_id":host.project.as_ref().unwrap().id,"scope":{"kind":"definition_draft","data":created["outcome"]["data"]["scope"]["data"]["id"]},"operation":"get_context","params":{"selection":[]}})).unwrap());
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(response["outcome"]["data"].get("run_summaries").is_none());
    }

    #[test]
    fn actual_export_asset_permission_requires_current_ui_confirmation_of_actual_source() {
        let directory = ProfessionalTestDirectory::new();
        let mut host = local_production_host();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let asset = AssetId::try_from("handoff-visual".to_owned()).unwrap();
        let path = "media/handoff.svg";
        let bytes =
            b"<svg xmlns=\"http://www.w3.org/2000/svg\"><rect width=\"1\" height=\"1\"/></svg>"
                .to_vec();
        let module = host.modules.values_mut().next().unwrap();
        module.assets.push(AssetRef {
            id: asset.clone(),
            path: RelativeFilePath::try_from(path.to_owned()).unwrap(),
            kind: AssetKind::Svg,
        });
        let right = AssetRights {
            asset_ref: asset.clone(),
            source: Some("own test drawing".into()),
            license: Some("CC0-1.0".into()),
            redistribution_confirmed: true,
            missing_reason: None,
        };
        module.rights.push(right.clone());
        module.definition.contribution.assets.push(right);
        stage_fixture_module_files(&mut host);
        host.additions.push(ArchiveAddition {
            entry: DocumentEntry {
                role: DocumentRole::Asset,
                module_ref: None,
                object_ref: ObjectRef {
                    kind: ObjectKind::Asset,
                    id: ObjectId::try_from(asset.as_str().to_owned()).unwrap(),
                },
                revision: None,
                file_ref: file_ref(path, DocumentKind::Asset),
                required: true,
            },
            bytes: bytes.clone(),
        });
        let saved = directory.root.join("source");
        std::fs::create_dir(&saved).unwrap();
        let selected = host
            .select_directory(&ui, FilePurpose::SaveProject, saved.clone())
            .unwrap();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":1,"selected_directory_ref":selected}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        reopen_test_project(&mut host, &saved);
        assert!(
            host.asset_confirmations.is_empty(),
            "imported rights do not establish current UI permission"
        );
        let export = |host: &mut ProjectHost, name: &str, include: bool| {
            let destination = directory.root.join(name);
            let token = host
                .select_file(&ui, FilePurpose::ExportPackage, destination.clone())
                .unwrap();
            let response = packaging_call(
                host,
                "export_package",
                json!({"revision":1,"package_scope":{"purpose":"handoff_project","scenarios":["LOCAL"],"run_ids":[],"artifact_ids":[],"module_refs":[],"include_redistributable_assets":include},"selected_destination_ref":token}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            let task: TaskHandle =
                serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
            let state = packaging_complete(host, &task);
            let progress = state.task.progress.as_ref().expect("actual package publication progress");
            assert_eq!(progress.processed.value(), 2);
            assert_eq!(progress.total.as_ref().unwrap().value(), 2);
            assert_eq!(progress.phase, "工程包已提交");
            assert!(
                matches!(state.task.state, ComputationStatus::Completed),
                "{state:?}"
            );
            crate::project_store::read_project_zip(&destination, &AtomicBool::new(false)).unwrap()
        };
        let omitted = export(&mut host, "unconfirmed.sfproj", true);
        assert!(!omitted.files.contains_key(path));
        assert!(
            omitted
                .manifest
                .missing
                .iter()
                .any(|issue| issue.code == "handoff.asset_not_redistributed")
        );
        let external = ObjectId::try_from("asset-external".to_owned()).unwrap();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(host.project.as_ref().unwrap().id.clone()),
            allowed_operations: vec![OperationName::ExportPackage],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        assert!(matches!(
            host.confirm_asset_redistribution(&external, &asset)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        host.confirm_asset_redistribution(&ui, &asset).unwrap();
        let included = export(&mut host, "confirmed.sfproj", true);
        assert_eq!(
            included.files.get(path),
            Some(&bytes),
            "exported rights {:?}; source rights {:?}; missing {:?}; confirmed {}",
            included.manifest.assets,
            host.backing.as_ref().unwrap().manifest.assets,
            included.manifest.missing,
            host.asset_confirmations.len()
        );
        assert!(
            !included
                .manifest
                .missing
                .iter()
                .any(|issue| issue.code == "handoff.asset_not_redistributed")
        );
        let excluded = export(&mut host, "explicitly-excluded.sfproj", false);
        assert!(!excluded.files.contains_key(path));
        reopen_test_project(&mut host, &saved);
        let after_open = export(&mut host, "after-open.sfproj", true);
        assert!(!after_open.files.contains_key(path));
    }

    fn stage_fixture_module_files(host: &mut ProjectHost) {
        for (index, package) in host.modules.values().enumerate() {
            host.additions.push(ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::ModuleDefinition,
                    module_ref: Some(ModuleRef {
                        id: package.definition.id.clone(),
                        version: package.definition.version.clone(),
                    }),
                    object_ref: ObjectRef {
                        kind: ObjectKind::Module,
                        id: ObjectId::try_from(package.definition.id.as_str().to_owned()).unwrap(),
                    },
                    revision: None,
                    file_ref: file_ref(
                        &format!("export-modules/{index}.json"),
                        DocumentKind::Module,
                    ),
                    required: true,
                },
                bytes: encode_document(package),
            });
        }
    }

    #[test]
    fn actual_comparison_dispatch_keeps_metric_scope_authority_and_old_run_inputs() {
        let mut host = local_production_host();
        let first = actual_local_run(&mut host);
        let second = actual_local_run(&mut host);
        let before = serde_json::to_value(host.project.as_ref().unwrap()).unwrap();
        let response = packaging_call(
            &mut host,
            "compare_runs",
            json!({"run_ids":[first.id,second.id],"metric_scope":{"keys":["accepted_quantity","completion_time","unit_cost"],"objects":[],"acceptance":[]}}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let comparison: RunComparison =
            serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert_eq!(comparison.pairs.len(), 1);
        assert!(matches!(
            comparison.pairs[0].status,
            ComparisonStatus::Strict
        ));
        assert_eq!(comparison.pairs[0].conditions.len(), 8);
        assert!(comparison.pairs[0].input_changes.is_empty());
        let accepted = comparison
            .metrics
            .iter()
            .find(|metric| matches!(metric.metric, MetricKey::AcceptedQuantity))
            .unwrap();
        assert_eq!(accepted.values.len(), 2);
        assert!(accepted.values.iter().all(|metric| matches!(&metric.value,MetricValue::Quantity(value) if value.value.coefficient.value() == 1 && value.value.decimal_places.value() == 0)));
        let costs: Vec<_> = comparison
            .metrics
            .iter()
            .filter(|metric| matches!(metric.metric, MetricKey::UnitCost))
            .collect();
        assert!(!costs.is_empty());
        assert!(costs.iter().all(|metric| {
            metric
                .values
                .iter()
                .all(|value| matches!(value.value, MetricValue::Unknown(_)))
        }));
        assert_eq!(
            serde_json::to_value(host.project.as_ref().unwrap()).unwrap(),
            before
        );
        assert!(exact_data(&host.runs[&first.id].result, &first));
        assert!(exact_data(&host.runs[&second.id].result, &second));
        let external = ObjectId::try_from("no-comparison-authority".to_owned()).unwrap();
        let project = host.project.as_ref().unwrap().id.clone();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(project.clone()),
            allowed_operations: vec![OperationName::GetRun],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        let denied = host.dispatch(&external,serde_json::from_value(json!({"api_version":"0.1","request_id":"compare-denied","project_id":project,"scope":{"kind":"project"},"operation":"compare_runs","params":{"run_ids":[first.id,second.id],"metric_scope":{"keys":[],"objects":[],"acceptance":[]}}})).unwrap());
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(denied["context"].is_null());
    }

    #[test]
    fn actual_export_tasks_freeze_selection_cancel_atomically_and_reopen_zip_from_initial_inputs() {
        let directory = ProfessionalTestDirectory::new();
        let mut host = local_production_host();
        stage_fixture_module_files(&mut host);
        let first = actual_local_run(&mut host);
        let excluded = actual_local_run(&mut host);
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let destination = directory.root.join("selected.sfproj");
        let selection = host
            .select_file(&ui, FilePurpose::ExportPackage, destination.clone())
            .unwrap();
        let package_scope = json!({"purpose":"handoff_project","scenarios":["LOCAL"],"run_ids":[first.id],"artifact_ids":[],"module_refs":[],"include_redistributable_assets":false});
        let response = packaging_call(
            &mut host,
            "export_package",
            json!({"revision":1,"package_scope":package_scope,"selected_destination_ref":selection}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(task.state, ComputationStatus::Queued));
        assert!(matches!(task.kind, LongTaskKind::Exporting));
        assert!(!destination.exists());
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["title"] = json!("edit while frozen export is queued");
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let completed = packaging_complete(&mut host, &task);
        assert!(
            matches!(completed.task.state, ComputationStatus::Completed),
            "{completed:?}"
        );
        assert!(matches!(
            completed.originating_operation,
            OperationName::ExportPackage
        ));
        assert_eq!(completed.task.input_revision.0.value(), 1);
        let Some(TaskOutput::Package(output)) = completed.result else {
            panic!("actual exported package output")
        };
        assert_eq!(output.package.path.as_str(), "selected.sfproj");
        assert!(
            output
                .included_files
                .iter()
                .any(|file| file.path.as_str() == "project.json")
        );
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 2);
        assert!(
            host.dirty,
            "export does not save or clean the current edited project"
        );
        let archive =
            crate::project_store::read_project_zip(&destination, &AtomicBool::new(false)).unwrap();
        assert_eq!(archive.project.revision.0.value(), 1);
        let saved_runs = archive.run_results().unwrap();
        assert_eq!(saved_runs.len(), 1);
        assert!(exact_data(&saved_runs[0], &first));
        assert!(!saved_runs.iter().any(|run| run.id == excluded.id));
        assert!(matches!(
            archive.manifest.restart_policy,
            RestartPolicy::RestartFromInitialInput
        ));
        let late = packaging_call(
            &mut host,
            "cancel_task",
            json!({"task_id":task.id,"action":"cancel"}),
        );
        assert_eq!(late["outcome"]["data"]["terminal_won"], true);
        let committed_bytes = std::fs::read(&destination).unwrap();
        let next_task = host.next_task;
        let stale = packaging_call(
            &mut host,
            "export_package",
            json!({"revision":1,"package_scope":package_scope,"selected_destination_ref":selection}),
        );
        assert_eq!(
            stale["outcome"]["data"]["code"], "RevisionConflict",
            "{stale}"
        );
        assert_eq!(host.next_task, next_task);
        let response = packaging_call(
            &mut host,
            "export_package",
            json!({"revision":2,"package_scope":package_scope,"selected_destination_ref":selection}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let cancelled: TaskHandle =
            serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let response = packaging_call(
            &mut host,
            "cancel_task",
            json!({"task_id":cancelled.id,"action":"cancel"}),
        );
        assert_eq!(response["outcome"]["data"]["action_accepted"], true);
        assert!(matches!(
            packaging_complete(&mut host, &cancelled).task.state,
            ComputationStatus::Cancelled
        ));
        assert_eq!(std::fs::read(&destination).unwrap(), committed_bytes);
        let exported_directory = directory.root.join("directory-export");
        std::fs::create_dir(&exported_directory).unwrap();
        let selected_directory = host
            .select_directory(&ui, FilePurpose::ExportPackage, exported_directory.clone())
            .unwrap();
        let response = packaging_call(
            &mut host,
            "export_package",
            json!({"revision":2,"package_scope":package_scope,"selected_destination_ref":selected_directory}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        assert_eq!(
            read_project_archive(&exported_directory)
                .unwrap()
                .project
                .revision
                .0
                .value(),
            2
        );
        reopen_test_project(&mut host, &destination);
        assert!(exact_data(&host.runs[&first.id].result, &first));
        assert!(!host.runs.contains_key(&excluded.id));
        let repeated = actual_local_run(&mut host);
        assert_ne!(repeated.id, first.id);
        assert_eq!(repeated.acceptance_events[0].at_ms.0.value(), 75000);
        assert!(exact_data(&host.runs[&first.id].result, &first));
    }

    fn template_reception_host(
        which: sf_module_kit::BuiltinTemplate,
        directory: &std::path::Path,
    ) -> (ProjectHost, sf_module_kit::TemplateCandidate) {
        let source = sf_module_kit::builtin_template(which).unwrap();
        let mut host = packaging_host();
        let mut draft = host.current_draft();
        draft.project.time_reference = source.example_project.time_reference.clone();
        let mut packages: Vec<ModulePackage> = source.definitions.iter().map(|definition| {
            serde_json::from_value(json!({"definition":definition,"assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]})).unwrap()
        }).collect();
        packages.push(source.package.clone());
        draft.project.module_refs = packages
            .iter()
            .map(|package| ModuleRef {
                id: package.definition.id.clone(),
                version: package.definition.version.clone(),
            })
            .collect();
        let mut additions: Vec<_> = packages
            .iter()
            .enumerate()
            .map(|(index, package)| ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::ModuleDefinition,
                    module_ref: Some(ModuleRef {
                        id: package.definition.id.clone(),
                        version: package.definition.version.clone(),
                    }),
                    object_ref: ObjectRef {
                        kind: ObjectKind::Module,
                        id: ObjectId::try_from(package.definition.id.as_str().to_owned()).unwrap(),
                    },
                    revision: None,
                    file_ref: file_ref(
                        &format!("source-definitions/{index}.json"),
                        DocumentKind::Module,
                    ),
                    required: true,
                },
                bytes: encode_document(package),
            })
            .collect();
        let name = match which {
            sf_module_kit::BuiltinTemplate::MachiningDelivery => "machining-delivery",
            sf_module_kit::BuiltinTemplate::PurchasedAssembly => "purchased-assembly",
            sf_module_kit::BuiltinTemplate::ParallelElectromechanical => {
                "parallel-electromechanical"
            }
            sf_module_kit::BuiltinTemplate::CartonFactory => "carton-factory",
        };
        let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/templates")
            .join(name)
            .join("candidate");
        for file in &source.package.files {
            additions.push(ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::SupportingFile,
                    module_ref: None,
                    object_ref: ObjectRef {
                        kind: ObjectKind::Module,
                        id: ObjectId::try_from(source.package.definition.id.as_str().to_owned())
                            .unwrap(),
                    },
                    revision: None,
                    file_ref: file.clone(),
                    required: true,
                },
                bytes: std::fs::read(source_path.join(file.path.as_str())).unwrap(),
            });
        }
        write_project_archive_with_additions(
            directory,
            &draft,
            None,
            &additions,
            &AtomicBool::new(false),
        )
        .unwrap();
        reopen_test_project(&mut host, directory);
        (host, source)
    }

    fn template_choices(
        source: &sf_module_kit::TemplateCandidate,
        namespace: &str,
    ) -> sf_module_kit::TemplateChoices {
        sf_module_kit::TemplateChoices {
            namespace: namespace.into(),
            resources: source
                .example_project
                .resources
                .iter()
                .map(|resource| {
                    (
                        resource.id.clone(),
                        sf_module_kit::TemplateResourceSelection::CreateDeclaredExample,
                    )
                })
                .collect(),
            reuse_items: BTreeMap::new(),
            include_example_product: true,
        }
    }

    fn evaluation_test_host() -> ProjectHost {
        let mut host = local_production_host();
        stage_fixture_module_files(&mut host);
        let mut scenario = host.scenarios.values().next().unwrap().clone();
        scenario.orders[0].quantity.as_mut().unwrap().value =
            serde_json::from_value(json!({"coefficient":1,"decimal_places":0})).unwrap();
        scenario.production_tasks[0].planned_quantity =
            scenario.orders[0].quantity.as_ref().unwrap().clone();
        scenario.production_tasks[0].order_allocations[0].quantity =
            scenario.orders[0].quantity.as_ref().unwrap().clone();
        scenario.cost_inputs=vec![serde_json::from_value(json!({"id":"actual-carton-rate","object_ref":{"kind":"item_type","id":"carton"},"component":"material","basis":"consumed_material","rate":{"coefficient":135000,"decimal_places":0},"currency":"CNY","rate_quantity":{"value":{"coefficient":7,"decimal_places":0},"unit":"piece"},"includes":[],"source":{"target":{"kind":"item_type","id":"carton"},"field_path":"/declared_cost_rate","value":null,"basis":"user_provided","source_ref":null}})).unwrap()];
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"scenario","data":scenario}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        host
    }

    fn evaluation_admit(
        host: &mut ProjectHost,
        path: &std::path::Path,
        mode: ReportMode,
        runs: &[RunId],
        scenarios: Vec<ScenarioReference>,
        formats: Value,
    ) -> TaskHandle {
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let selection = host
            .select_directory(&ui, FilePurpose::ExportEvaluation, path.to_owned())
            .unwrap();
        let params = json!({"revision":host.project.as_ref().unwrap().revision,"report_mode":mode,"scenario_refs":scenarios,"run_refs":runs,"metric_scope":{"keys":[],"objects":[],"acceptance":[]},"formats":formats,"selected_destination_ref":selection});
        let response = packaging_call(host, "export_evaluation", params);
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
    }

    fn evaluation_result(host: &mut ProjectHost, task: &TaskHandle) -> ExportEvaluation {
        let state = packaging_complete(host, task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        assert!(matches!(
            state.originating_operation,
            OperationName::ExportEvaluation
        ));
        let Some(TaskOutput::Evaluation(output)) = state.result else {
            panic!("actual evaluation task result")
        };
        *output
    }

    fn evaluation_font_resources() -> sf_reporting::EvaluationResources {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/architecture-proof/assets");
        sf_reporting::EvaluationResources {
            cjk_font: Some(std::fs::read(root.join("SourceHanSansCN-Regular.otf")).unwrap()),
            font_license: Some(std::fs::read(root.join("LICENSE.SourceHanSans.txt")).unwrap()),
            files: BTreeMap::new(),
        }
    }

    #[test]
    fn evaluation_host_exports_actual_four_formats_and_a_portable_frozen_project_without_saving_current()
     {
        let mut host = evaluation_test_host();
        let run = actual_local_run(&mut host);
        assert_eq!(run.goal_met, Some(true));
        let frozen = host.current_draft();
        let old_additions = host.additions.len();
        host.set_evaluation_resources(evaluation_font_resources());
        let directory = ProfessionalTestDirectory::new();
        let task = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::SimulationEvaluation,
            std::slice::from_ref(&run.id),
            vec![],
            json!(["pdf", "html", "csv", "json"]),
        );
        assert!(matches!(task.kind, LongTaskKind::Exporting));
        assert_eq!(
            task.input_revision.0.value(),
            frozen.project.revision.0.value()
        );
        // Resources and Project input were frozen when the task was admitted.
        host.set_evaluation_resources(sf_reporting::EvaluationResources::default());
        let settings = json!({"title":"edited after report admission","time_reference":frozen.project.time_reference,"generation_context":frozen.project.generation_context,"sources":frozen.project.sources});
        let edit = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":frozen.project.revision,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(edit["outcome"]["status"], "ok");
        let current = host.current_draft();
        let report_history = serde_json::to_value(host.history.availability()).unwrap();
        let output = evaluation_result(&mut host, &task);
        assert_eq!(
            output.context.project_revision.0.value(),
            frozen.project.revision.0.value()
        );
        assert!(output.context.scenario_comparison.is_none());
        assert_eq!(output.context.run_evaluations.len(), 1);
        assert_eq!(output.context.run_evaluations[0].goal_met, Some(true));
        assert_eq!(
            output.context.run_evaluations[0].costs.lines[0]
                .amount
                .as_ref()
                .unwrap()
                .value(),
            135000.0 / 7.0
        );
        assert!(output.context.layout_and_schedule.is_empty());
        assert!(
            output
                .context
                .missing_conditions
                .iter()
                .any(|issue| issue.code == "evaluation.actual_render_missing")
        );
        assert!(exact_data(&current.project, host.project.as_ref().unwrap()));
        assert!(host.dirty && host.backing.is_none());
        assert_eq!(
            serde_json::to_value(host.history.availability()).unwrap(),
            report_history
        );
        assert_eq!(host.additions.len(), old_additions);
        assert!(exact_data(&host.runs[&run.id].result, &run));
        let json_file = output
            .files
            .iter()
            .find(|file| matches!(file.kind, DocumentKind::Evaluation))
            .unwrap();
        let report_root = directory
            .root
            .join(json_file.path.as_str())
            .parent()
            .unwrap()
            .to_owned();
        let saved: Versioned<EvaluationReport> = serde_json::from_slice(
            &std::fs::read(directory.root.join(json_file.path.as_str())).unwrap(),
        )
        .unwrap();
        assert!(exact_data(&saved.data, &output.context));
        let html = std::fs::read_to_string(report_root.join("index.html")).unwrap();
        let embedded = html
            .split("<script id=report-data type=application/json>")
            .nth(1)
            .unwrap()
            .split("</script>")
            .next()
            .unwrap();
        let embedded: Versioned<EvaluationReport> = serde_json::from_str(embedded).unwrap();
        assert!(exact_data(&embedded.data, &output.context));
        assert!(
            std::fs::read(report_root.join("report.pdf"))
                .unwrap()
                .starts_with(b"%PDF")
        );
        assert_eq!(
            std::fs::read_dir(report_root.join("csv")).unwrap().count(),
            15
        );
        for entry in std::fs::read_dir(report_root.join("csv")).unwrap() {
            assert!(
                std::fs::read(entry.unwrap().path())
                    .unwrap()
                    .starts_with(&[0xef, 0xbb, 0xbf])
            );
        }
        let project_path = &output.context.editable_project_ref.as_ref().unwrap().path;
        let archive =
            read_project_archive(report_root.join(project_path.as_str()).parent().unwrap())
                .unwrap();
        assert!(exact_data(&archive.project, &frozen.project));
        assert!(matches!(
            archive.manifest.restart_policy,
            RestartPolicy::RestartFromInitialInput
        ));
        let moved = ProfessionalTestDirectory::new();
        for file in &output.files {
            let target = moved.root.join(file.path.as_str());
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(directory.root.join(file.path.as_str()), target).unwrap();
        }
        let moved_report = moved
            .root
            .join(json_file.path.as_str())
            .parent()
            .unwrap()
            .to_owned();
        let mut reopened = packaging_host();
        reopen_test_project(
            &mut reopened,
            moved_report.join(project_path.as_str()).parent().unwrap(),
        );
        assert!(exact_data(
            reopened.project.as_ref().unwrap(),
            &frozen.project
        ));
        assert!(exact_data(&reopened.runs[&run.id].result, &run));
        let readback: Versioned<EvaluationReport> =
            serde_json::from_slice(&std::fs::read(moved_report.join("report.json")).unwrap())
                .unwrap();
        assert!(exact_data(&readback.data, &output.context));
    }

    #[test]
    fn evaluation_host_keeps_each_run_price_revision_and_explicit_current_and_historical_scenarios()
    {
        let mut host = evaluation_test_host();
        host.set_evaluation_resources(evaluation_font_resources());
        let first = actual_local_run(&mut host);
        let mut scenario = host.scenarios.values().next().unwrap().clone();
        scenario.cost_inputs[0].rate =
            Some(serde_json::from_value(json!({"coefficient":270000,"decimal_places":0})).unwrap());
        let base = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":base,"commands":[{"kind":"put_object","data":{"kind":"scenario","data":scenario}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let second = actual_local_run(&mut host);
        let selected = vec![
            ScenarioReference {
                scenario_id: first.input_ref.scenario_id.clone(),
                revision: first.input_ref.scenario_revision.clone(),
            },
            ScenarioReference {
                scenario_id: second.input_ref.scenario_id.clone(),
                revision: second.input_ref.scenario_revision.clone(),
            },
        ];
        let before = host.current_draft();
        let directory = ProfessionalTestDirectory::new();
        let task = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::SimulationEvaluation,
            &[first.id.clone(), second.id.clone()],
            selected,
            json!(["pdf", "json", "html", "csv"]),
        );
        let output = evaluation_result(&mut host, &task);
        assert_eq!(output.context.run_evaluations.len(), 2);
        assert!(output.context.scenario_comparison.is_some());
        assert_eq!(output.context.scenario_refs.len(), 2);
        for (index, rate) in [135000u64, 270000].into_iter().enumerate() {
            let evaluation = &output.context.run_evaluations[index];
            assert_eq!(
                evaluation.costs.lines[0].amount.as_ref().unwrap().value(),
                rate as f64 / 7.0
            );
            assert_eq!(
                serde_json::to_value(evaluation.costs.inputs[0].rate.as_ref().unwrap()).unwrap()["coefficient"],
                rate
            );
        }
        assert!(exact_data(&before.project, host.project.as_ref().unwrap()));
        assert!(exact_data(&host.runs[&first.id].result, &first));
        assert!(exact_data(&host.runs[&second.id].result, &second));
        let mut invalid = host.scenarios.values().next().unwrap().clone();
        invalid.title = "current design inputs with unknown material price".into();
        invalid.cost_inputs[0].rate = None;
        let base = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":base,"commands":[{"kind":"put_object","data":{"kind":"scenario","data":invalid}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok");
        let third = actual_local_run(&mut host);
        let task = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::SimulationEvaluation,
            std::slice::from_ref(&third.id),
            vec![],
            json!(["json"]),
        );
        let output = evaluation_result(&mut host, &task);
        assert!(
            output.context.run_evaluations[0].costs.lines[0]
                .amount
                .is_none()
        );
        assert!(
            output.context.run_evaluations[0].costs.currencies[0]
                .complete_total
                .is_none()
        );
    }

    #[test]
    fn evaluation_host_design_mode_has_no_results_and_cancel_failure_and_commit_winner_preserve_old_reports()
     {
        let mut host = packaging_host();
        host.set_evaluation_resources(evaluation_font_resources());
        let directory = ProfessionalTestDirectory::new();
        let task = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::DesignDescription,
            &[],
            vec![],
            json!(["pdf", "html", "json", "csv"]),
        );
        let output = evaluation_result(&mut host, &task);
        assert!(
            output.context.run_refs.is_empty()
                && output.context.run_evaluations.is_empty()
                && output.context.scenario_comparison.is_none()
        );
        host.set_evaluation_resources(sf_reporting::EvaluationResources::default());
        let old_files: BTreeMap<_, _> = output
            .files
            .iter()
            .map(|file| {
                (
                    file.path.as_str().to_owned(),
                    std::fs::read(directory.root.join(file.path.as_str())).unwrap(),
                )
            })
            .collect();
        let before = host.current_draft();
        let cancelled = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::DesignDescription,
            &[],
            vec![],
            json!(["json"]),
        );
        let work = host.drain_io_work().pop().unwrap();
        let response = packaging_call(
            &mut host,
            "cancel_task",
            json!({"task_id":cancelled.id,"action":"cancel"}),
        );
        assert_eq!(response["outcome"]["status"], "ok");
        host.finish_io(work.run());
        assert!(matches!(
            host.tasks[&cancelled.id].state.task.state,
            ComputationStatus::Cancelled
        ));
        let missing_font = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::DesignDescription,
            &[],
            vec![],
            json!(["pdf", "json"]),
        );
        let failed = packaging_complete(&mut host, &missing_font);
        assert!(matches!(failed.task.state, ComputationStatus::Failed));
        assert!(matches!(
            failed.failure.unwrap().code,
            ErrorCode::MissingReference
        ));
        let won = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::DesignDescription,
            &[],
            vec![],
            json!(["json"]),
        );
        let completion = host.drain_io_work().pop().unwrap().run();
        let response = packaging_call(
            &mut host,
            "cancel_task",
            json!({"task_id":won.id,"action":"cancel"}),
        );
        assert_eq!(response["outcome"]["status"], "ok");
        host.finish_io(completion);
        assert!(matches!(
            host.tasks[&won.id].state.task.state,
            ComputationStatus::Completed
        ));
        let collision_name = format!("evaluation-{:08}", host.next_task);
        let collision_root = directory.root.join(&collision_name);
        for (path, bytes) in &old_files {
            let suffix = path.split_once('/').unwrap().1;
            let file = collision_root.join(suffix);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        let collision = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::DesignDescription,
            &[],
            vec![],
            json!(["json"]),
        );
        let failed = packaging_complete(&mut host, &collision);
        assert!(matches!(failed.task.state, ComputationStatus::Failed));
        for (path, bytes) in &old_files {
            assert_eq!(
                std::fs::read(collision_root.join(path.split_once('/').unwrap().1)).unwrap(),
                *bytes
            );
        }
        for (path, bytes) in old_files {
            assert_eq!(std::fs::read(directory.root.join(path)).unwrap(), bytes);
        }
        assert!(exact_data(&before.project, host.project.as_ref().unwrap()));
        assert!(host.dirty && host.backing.is_none());
    }

    #[test]
    fn evaluation_host_authority_revision_scope_and_actual_frozen_files_are_checked_before_enqueue()
    {
        let mut host = evaluation_test_host();
        let run = actual_local_run(&mut host);
        let directory = ProfessionalTestDirectory::new();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let selected = host
            .select_directory(&ui, FilePurpose::ExportEvaluation, directory.root.clone())
            .unwrap();
        let params = json!({"revision":host.project.as_ref().unwrap().revision,"report_mode":"simulation_evaluation","scenario_refs":[],"run_refs":[run.id],"metric_scope":{"keys":[],"objects":[],"acceptance":[]},"formats":["json"],"selected_destination_ref":selected});
        let next = host.next_task;
        let mut stale = params.clone();
        stale["revision"] = json!(0);
        assert_eq!(
            packaging_call(&mut host, "export_evaluation", stale)["outcome"]["data"]["code"],
            "RevisionConflict"
        );
        let wrong_purpose = host
            .select_directory(&ui, FilePurpose::SaveProject, directory.root.clone())
            .unwrap();
        let mut forged = params.clone();
        forged["selected_destination_ref"] = serde_json::to_value(wrong_purpose).unwrap();
        assert_eq!(
            packaging_call(&mut host, "export_evaluation", forged)["outcome"]["data"]["code"],
            "PermissionDenied"
        );
        let mut external = task_authority_caller(&host, "report-current-external");
        external.allowed_operations = vec![
            OperationName::ExportEvaluation,
            OperationName::GetTask,
            OperationName::CancelTask,
        ];
        let session = external.session_id.clone();
        host.register_external_session(external).unwrap();
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "export_evaluation",
            params.clone(),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        let token = host
            .select_directory(
                &session,
                FilePurpose::ExportEvaluation,
                directory.root.clone(),
            )
            .unwrap();
        let mut allowed = params.clone();
        allowed["selected_destination_ref"] = serde_json::to_value(token).unwrap();
        let response = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "export_evaluation",
            allowed,
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let mut foreign = task_authority_caller(&host, "report-foreign-external");
        foreign.allowed_operations = vec![OperationName::ExportEvaluation];
        foreign.granted_project =
            Some(ProjectId::try_from("different-project".to_owned()).unwrap());
        let foreign_id = foreign.session_id.clone();
        host.register_external_session(foreign).unwrap();
        let denied = authority_call(
            &mut host,
            &foreign_id,
            ApiScope::Project,
            "export_evaluation",
            params.clone(),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        assert!(denied["context"].is_null());
        assert_eq!(host.next_task, next + 1);
        let work = host.drain_io_work().pop().unwrap();
        host.revoke_external_session(&session).unwrap();
        host.finish_io(work.run());
        assert!(
            matches!(
                host.tasks[&task.id].state.task.state,
                ComputationStatus::Completed
            ),
            "revocation does not cancel admitted work"
        );
        let definition = host.modules.values().next().unwrap().definition.clone();
        let base = host.project.as_ref().unwrap().revision.clone();
        let created = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":base,"commands":[{"kind":"create_definition_draft","data":definition}]}),
        );
        let id: ModuleId =
            serde_json::from_value(created["outcome"]["data"]["scope"]["data"]["id"].clone())
                .unwrap();
        let invalid = authority_call(
            &mut host,
            &ui,
            ApiScope::DefinitionDraft(id),
            "export_evaluation",
            params.clone(),
        );
        assert_eq!(invalid["outcome"]["data"]["code"], "UnsupportedCapability");
        let path = host
            .additions
            .iter()
            .find(|file| matches!(file.entry.file_ref.kind, DocumentKind::Module))
            .unwrap()
            .entry
            .file_ref
            .path
            .clone();
        host.additions
            .retain(|file| file.entry.file_ref.path != path);
        let next = host.next_task;
        let missing = packaging_call(&mut host, "export_evaluation", params);
        assert_eq!(
            missing["outcome"]["data"]["code"], "MissingReference",
            "{missing}"
        );
        assert_eq!(host.next_task, next);
    }

    #[test]
    fn evaluation_host_preserves_explicit_saved_historical_design_input_without_activating_it() {
        let mut host = evaluation_test_host();
        let historical = host.scenarios.values().next().unwrap().clone();
        let saved_dir = ProfessionalTestDirectory::new();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let token = host
            .select_directory(&ui, FilePurpose::SaveProject, saved_dir.root.clone())
            .unwrap();
        let base = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":base,"selected_directory_ref":token}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        let mut current = historical.clone();
        current.title = "new editable design input".into();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":base,"commands":[{"kind":"put_object","data":{"kind":"scenario","data":current}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok");
        let current = host.scenarios[&historical.id].clone();
        assert_ne!(current.revision.0.value(), historical.revision.0.value());
        assert!(host.runs.is_empty());
        let directory = ProfessionalTestDirectory::new();
        let task = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::DesignDescription,
            &[],
            vec![ScenarioReference {
                scenario_id: historical.id.clone(),
                revision: historical.revision.clone(),
            }],
            json!(["json"]),
        );
        let output = evaluation_result(&mut host, &task);
        assert_eq!(
            output.context.scenario_refs[0].revision.0.value(),
            historical.revision.0.value()
        );
        assert!(output.context.run_evaluations.is_empty());
        let report = output
            .files
            .iter()
            .find(|file| matches!(file.kind, DocumentKind::Evaluation))
            .unwrap();
        let archive = read_project_archive(
            &directory
                .root
                .join(report.path.as_str())
                .parent()
                .unwrap()
                .join("project"),
        )
        .unwrap();
        assert!(exact_data(&archive.scenarios[&historical.id], &current));
        let old = archive
            .manifest
            .documents
            .iter()
            .find(|entry| {
                matches!(entry.file_ref.kind, DocumentKind::Scenario)
                    && entry.object_ref.id.as_str() == historical.id.as_str()
                    && entry
                        .revision
                        .as_ref()
                        .is_some_and(|revision| exact_data(revision, &historical.revision))
            })
            .unwrap();
        assert!(matches!(old.role, DocumentRole::SupportingFile));
        let value: Versioned<ScenarioInput> =
            serde_json::from_slice(&archive.files[old.file_ref.path.as_str()]).unwrap();
        assert!(exact_data(&value.data, &historical));
        assert!(exact_data(&host.scenarios[&historical.id], &current));
        assert!(host.dirty);
        assert_eq!(
            host.backing.as_ref().unwrap().project.revision.0.value(),
            base.0.value()
        );
    }

    #[test]
    fn evaluation_host_reports_actual_incomplete_acceptance_without_filling_completion_time() {
        let mut host = local_production_host();
        stage_fixture_module_files(&mut host);
        let run = actual_local_run(&mut host);
        assert_eq!(run.goal_met, Some(false));
        let directory = ProfessionalTestDirectory::new();
        let task = evaluation_admit(
            &mut host,
            &directory.root,
            ReportMode::SimulationEvaluation,
            std::slice::from_ref(&run.id),
            vec![],
            json!(["json", "csv"]),
        );
        let output = evaluation_result(&mut host, &task);
        assert_eq!(output.context.run_evaluations[0].goal_met, Some(false));
        assert!(
            output
                .context
                .conclusions
                .iter()
                .any(|conclusion| conclusion.contains("C=null"))
        );
        assert_eq!(output.context.run_evaluations[0].acceptance_events.len(), 1);
        assert!(output.context.scenario_comparison.is_none());
    }

    #[test]
    fn bundled_support_packages_cover_nine_categories_and_explicit_adoption_survives_open() {
        let mut host = packaging_host();
        register_test_bundled_library(&mut host);
        let before = host.current_draft();
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/devices/template-support");
        let mut references = vec![];
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
            let directory = base.join(category);
            let raw: Versioned<ModulePackage> =
                serde_json::from_slice(&std::fs::read(directory.join("module.json")).unwrap())
                    .unwrap();
            let reference = ModuleRef {
                id: raw.data.definition.id.clone(),
                version: raw.data.definition.version.clone(),
            };
            host.register_bundled_module_directory(&directory).unwrap();
            let registered = &host.bundled_modules[&module_key(&reference)];
            assert!(registered.additions.iter().any(|addition| {
                addition
                    .entry
                    .file_ref
                    .path
                    .as_str()
                    .ends_with("/original-module.json")
                    && serde_json::from_slice::<Versioned<ModulePackage>>(&addition.bytes)
                        .is_ok_and(|original| exact_data(&original.data, &raw.data))
            }));
            assert!(exact_data(
                &registered.package.definition.payload,
                &raw.data.definition.payload
            ));
            assert!(
                registered
                    .package
                    .rights
                    .iter()
                    .all(|rights| !rights.redistribution_confirmed)
            );
            for file in &raw.data.files {
                assert!(registered.additions.iter().any(|addition| addition.bytes
                    == std::fs::read(directory.join(file.path.as_str())).unwrap()));
            }
            references.push(reference);
        }
        assert!(exact_data(&host.current_draft().project, &before.project));
        assert!(host.modules.is_empty() && host.additions.is_empty() && host.scenarios.is_empty());
        let all = bundled_query(&mut host, json!(["bundled"]));
        let categories: BTreeSet<_> = all["outcome"]["data"]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|candidate| candidate["category"].as_str().unwrap())
            .collect();
        assert_eq!(
            categories,
            BTreeSet::from([
                "items",
                "devices",
                "resources",
                "tasks",
                "operations",
                "transport",
                "professional",
                "media",
                "templates"
            ])
        );
        let commands: Vec<_> = references.iter().map(|reference| json!({"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}})).collect();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":commands}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert_eq!(host.modules.len(), 8);
        assert!(host.project.as_ref().unwrap().devices.is_empty() && host.scenarios.is_empty());
        let exact: BTreeMap<_, _> = host.modules.clone();
        let directory = ProfessionalTestDirectory::new();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let token = host
            .select_directory(&ui, FilePurpose::SaveProject, directory.root.clone())
            .unwrap();
        let saved = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":2,"selected_directory_ref":token}),
        );
        assert_eq!(saved["outcome"]["status"], "ok", "{saved}");
        let task: TaskHandle = serde_json::from_value(saved["outcome"]["data"].clone()).unwrap();
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let mut reopened = packaging_host();
        reopen_test_project(&mut reopened, &directory.root);
        assert!(reopened.bundled_modules.is_empty());
        for reference in references {
            let read = packaging_call(
                &mut reopened,
                "get_object",
                json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null}),
            );
            assert_eq!(read["outcome"]["status"], "ok", "{read}");
            assert!(exact_data(
                &reopened.modules[&module_key(&reference)],
                &exact[&module_key(&reference)]
            ));
        }
    }

    #[test]
    fn bundled_support_registration_rejects_conflicting_repeat_without_mutating_registry_or_project()
     {
        let mut host = packaging_host();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/devices/template-support/media");
        host.register_bundled_module_directory(&root).unwrap();
        let before = host.current_draft();
        let original: Versioned<ModulePackage> =
            serde_json::from_slice(&std::fs::read(root.join("module.json")).unwrap()).unwrap();
        let reference = ModuleRef {
            id: original.data.definition.id.clone(),
            version: original.data.definition.version.clone(),
        };
        let exact = host.bundled_modules[&module_key(&reference)]
            .package
            .clone();
        let directory = ProfessionalTestDirectory::new();
        for file in &original.data.files {
            let target = directory.root.join(file.path.as_str());
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(
                target,
                std::fs::read(root.join(file.path.as_str())).unwrap(),
            )
            .unwrap();
        }
        let mut conflicting = original.clone();
        conflicting.data.definition.title = "same immutable version, changed content".into();
        std::fs::write(
            directory.root.join("module.json"),
            serde_json::to_vec(&conflicting).unwrap(),
        )
        .unwrap();
        let failure = host
            .register_bundled_module_directory(&directory.root)
            .unwrap_err();
        assert!(matches!(failure.code, ErrorCode::InvalidInput));
        assert_eq!(host.bundled_modules.len(), 1);
        assert!(exact_data(
            &host.bundled_modules[&module_key(&reference)].package,
            &exact
        ));
        assert!(exact_data(host.project.as_ref().unwrap(), &before.project));
        assert!(host.modules.is_empty() && host.additions.is_empty());
        let failure = host.register_bundled_template_directory(&root).unwrap_err();
        assert!(matches!(failure.code, ErrorCode::InvalidInput));
    }

    #[test]
    fn settings_setup_matrix_changes_real_run_and_survives_old_wire_history_and_reopen() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/p104-A-first.json"
        ))
        .unwrap();
        let mut host = packaging_host();
        let project: ProjectState = serde_json::from_value(fixture["project"].clone()).unwrap();
        let scenario: ScenarioInput = serde_json::from_value(fixture["scenario"].clone()).unwrap();
        host.modules = fixture["definitions"].as_array().unwrap().iter().map(|definition| {
            let package: ModulePackage = serde_json::from_value(json!({"definition":definition,"assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]})).unwrap();
            (module_key(&ModuleRef{id:package.definition.id.clone(),version:package.definition.version.clone()}),package)
        }).collect();
        host.sessions
            .get_mut(&ObjectId::try_from("packaging-test-ui".to_owned()).unwrap())
            .unwrap()
            .granted_project = Some(project.id.clone());
        host.project = Some(project);
        host.scenario_highwater
            .insert(scenario.id.clone(), scenario.revision.clone());
        host.scenarios.insert(scenario.id.clone(), scenario);
        host.history.reset(&host.current_draft());
        stage_fixture_module_files(&mut host);
        let scenario_id = host.scenarios.keys().next().unwrap().clone();
        let actual_run = |host: &mut ProjectHost| {
            let response = local_run(host, &scenario_id);
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            let task: TaskHandle =
                serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
            let state = packaging_complete(host, &task);
            assert!(
                matches!(state.task.state, ComputationStatus::Completed),
                "{state:?}"
            );
            let Some(TaskOutput::Run(summary)) = state.result else {
                panic!("actual setup Run")
            };
            host.runs[&summary.run_id].result.clone()
        };
        let run = actual_run(&mut host);
        assert_eq!(
            run.acceptance_events.last().unwrap().at_ms.0.value(),
            205000
        );
        let original = host.project.as_ref().unwrap().setup_policies.clone();
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["setup_policies"][0]["changeovers"][0]["duration"]["value"] = json!(30000);
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":revision,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let changed = host.project.as_ref().unwrap().setup_policies.clone();
        let next = actual_run(&mut host);
        let setup_work = |result: &RunResult| {
            result
                .jobs
                .iter()
                .filter(|job| job.id.as_str().starts_with("setup:"))
                .map(|job| {
                    job.finished_at_ms.as_ref().unwrap().0.value()
                        - job.started_at_ms.as_ref().unwrap().0.value()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(setup_work(&run), vec![15000]);
        assert_eq!(setup_work(&next), vec![30000]);
        // This fixture has downstream slack; the material path still ends at 205s.
        assert_eq!(
            next.acceptance_events.last().unwrap().at_ms.0.value(),
            205000
        );
        assert!(exact_data(&host.runs[&run.id].result, &run));
        let mut legacy = settings_value(host.project.as_ref().unwrap());
        legacy.as_object_mut().unwrap().remove("setup_policies");
        legacy["title"] = json!("legacy title-only carrier keeps matrix");
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":revision,"commands":[{"kind":"set_project_settings","data":legacy}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(exact_data(
            &host.project.as_ref().unwrap().setup_policies,
            &changed
        ));
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["setup_policies"] = json!([]);
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":revision,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(host.project.as_ref().unwrap().setup_policies.is_empty());
        for (command, expected) in [
            ("undo", changed.clone()),
            ("redo", vec![]),
            ("undo", changed.clone()),
        ] {
            let revision = host.project.as_ref().unwrap().revision.0.value();
            let response = packaging_call(
                &mut host,
                "apply_commands",
                json!({"base_revision":revision,"commands":[{"kind":command}]}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            assert_eq!(
                host.project.as_ref().unwrap().revision.0.value(),
                revision + 1
            );
            assert!(exact_data(
                &host.project.as_ref().unwrap().setup_policies,
                &expected
            ));
        }
        let before = host.current_draft();
        let mut duplicate = settings_value(&before.project);
        duplicate["setup_policies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::to_value(&changed[0]).unwrap());
        let failed = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":before.project.revision,"commands":[{"kind":"set_project_settings","data":duplicate}]}),
        );
        assert_eq!(
            failed["outcome"]["data"]["code"], "InvalidInput",
            "{failed}"
        );
        let stale = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings_value(&before.project)}]}),
        );
        assert_eq!(stale["outcome"]["data"]["code"], "RevisionConflict");
        assert!(exact_data(host.project.as_ref().unwrap(), &before.project));
        assert!(!exact_data(&original, &changed));
        let directory = ProfessionalTestDirectory::new();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let token = host
            .select_directory(&ui, FilePurpose::SaveProject, directory.root.clone())
            .unwrap();
        let saved = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":before.project.revision,"selected_directory_ref":token}),
        );
        let task: TaskHandle = serde_json::from_value(saved["outcome"]["data"].clone()).unwrap();
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        reopen_test_project(&mut host, &directory.root);
        assert!(exact_data(
            &host.project.as_ref().unwrap().setup_policies,
            &changed
        ));
        assert!(exact_data(&host.runs[&run.id].result, &run));
        assert!(exact_data(&host.runs[&next.id].result, &next));
    }

    #[test]
    fn settings_setup_policy_authority_uses_the_actual_stable_device_and_preserves_omitted_matrix()
    {
        let mut host = local_production_host();
        let device = host.project.as_ref().unwrap().devices[0].id.clone();
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["setup_policies"] = json!([{"device_id":device,"initial_family":"A","changeovers":[],"first_article":null}]);
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let mut caller = task_authority_caller(&host, "setup-matrix-agent");
        caller.allowed_operations = vec![
            OperationName::ApplyCommands,
            OperationName::DescribeCapabilities,
        ];
        caller.edit_grants = vec![HostEditGrant {
            id: ObjectId::try_from("setup-fields".to_owned()).unwrap(),
            scope: TaskScope::Edit(EditScope::Project(
                host.project.as_ref().unwrap().id.clone(),
            )),
            branches: vec![
                HostEditBranch::SetProjectSettings,
                HostEditBranch::Undo,
                HostEditBranch::Redo,
            ],
            objects: vec![HostObjectEditGrant {
                target: ObjectRef {
                    kind: ObjectKind::Project,
                    id: ObjectId::try_from(host.project.as_ref().unwrap().id.as_str().to_owned())
                        .unwrap(),
                },
                fields: vec![HostFieldGrant {
                    pointer: "/setup_policies/99/initial_family".into(),
                    element_id: Some(device.clone()),
                }],
                can_remove: false,
            }],
            creates: vec![],
            source_versions: vec![],
            reusable_resources: vec![],
            created_objects: vec![],
        }];
        let session = caller.session_id.clone();
        host.register_external_session(caller).unwrap();
        let mut settings = settings_value(host.project.as_ref().unwrap());
        settings["setup_policies"][0]["initial_family"] = json!("B");
        let accepted = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(accepted["outcome"]["status"], "ok", "{accepted}");
        for (command, family) in [("undo", "A"), ("redo", "B")] {
            let revision = host.project.as_ref().unwrap().revision.clone();
            let response = authority_call(
                &mut host,
                &session,
                ApiScope::Project,
                "apply_commands",
                json!({"base_revision":revision,"commands":[{"kind":command}]}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            assert_eq!(
                host.project.as_ref().unwrap().setup_policies[0]
                    .initial_family
                    .as_deref(),
                Some(family)
            );
        }
        let before = host.current_draft();
        let mut settings = settings_value(&before.project);
        settings["title"] = json!("unauthorized title");
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":before.project.revision,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        let mut settings = settings_value(&before.project);
        settings["setup_policies"][0]["device_id"] = json!("wrong-stable-device");
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":before.project.revision,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "UnsupportedCapability");
        let mut settings = settings_value(&before.project);
        settings["setup_policies"] = json!([]);
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":before.project.revision,"commands":[{"kind":"set_project_settings","data":settings}]}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "UnsupportedCapability");
        assert!(exact_data(host.project.as_ref().unwrap(), &before.project));
        let mut legacy = settings_value(&before.project);
        legacy.as_object_mut().unwrap().remove("setup_policies");
        let accepted = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":before.project.revision,"commands":[{"kind":"set_project_settings","data":legacy}]}),
        );
        assert_eq!(accepted["outcome"]["status"], "ok", "{accepted}");
        assert!(exact_data(
            &host.project.as_ref().unwrap().setup_policies,
            &before.project.setup_policies
        ));
    }

    fn register_test_bundled_library(host: &mut ProjectHost) {
        for name in [
            "purchased-assembly",
            "machining-delivery",
            "parallel-electromechanical",
            "carton-factory",
        ] {
            host.register_bundled_template_directory(
                &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../modules/templates")
                    .join(name)
                    .join("candidate"),
            )
            .unwrap();
        }
    }

    fn bundled_query(host: &mut ProjectHost, scopes: Value) -> Value {
        packaging_call(
            host,
            "query_module_library",
            json!({"category_filters":[],"industry":null,"product_family":null,"site_envelope":null,"source_scope":scopes}),
        )
    }

    fn bundled_preview_call(
        host: &mut ProjectHost,
        reference: &ModuleRef,
        choices: Value,
        seed: Value,
    ) -> Value {
        let params = json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":host.project.as_ref().unwrap().revision,"choices":choices,"seed":seed}},"node_index":null});
        packaging_call(host, "get_object", params)
    }

    #[test]
    fn bundled_readonly_preview_uses_actual_source_and_explicit_seed_then_commit_rechecks_revision()
    {
        let mut host = packaging_host();
        register_test_bundled_library(&mut host);
        let source =
            sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::PurchasedAssembly)
                .unwrap();
        let reference = ModuleRef {
            id: source.package.definition.id.clone(),
            version: source.package.definition.version.clone(),
        };
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":{"title":"explicit source time","time_reference":source.example_project.time_reference,"generation_context":null,"sources":[]}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let before = host.current_draft();
        let metadata = bundled_preview_call(&mut host, &reference, Value::Null, Value::Null);
        assert_eq!(metadata["outcome"]["status"], "ok", "{metadata}");
        let metadata: TemplatePreview =
            serde_json::from_value(metadata["outcome"]["data"]["data"].clone()).unwrap();
        assert!(metadata.command.is_none());
        assert!(exact_data(
            &metadata.resources,
            &source.example_project.resources
        ));
        assert!(host.modules.is_empty() && host.additions.is_empty() && host.scenarios.is_empty());
        let choices = json!({"source_project":metadata.source_project,"namespace":"preview-real","resources":metadata.resources.iter().map(|resource|(resource.id.clone(),Option::<ObjectId>::None)).collect::<BTreeMap<_,_>>(),"reuse_items":{},"include_example_product":true,"site_envelope":null});
        let seed = json!({"scenario_id":"PREVIEW-RUN","supply":true,"demand":true,"initial_quality":source.subset.adoption.as_ref().unwrap().deferred_seed.as_ref().unwrap().initial_stock.iter().map(|stock|(stock.location_id.clone(),QualityState::Released)).collect::<BTreeMap<_,_>>(),"adopt_declared_inspection_results":true});
        let denied = bundled_preview_call(&mut host, &reference, Value::Null, seed.clone());
        assert_eq!(denied["outcome"]["data"]["code"], "InvalidInput");
        let ready = bundled_preview_call(&mut host, &reference, choices.clone(), seed.clone());
        assert_eq!(ready["outcome"]["status"], "ok", "{ready}");
        let preview: TemplatePreview =
            serde_json::from_value(ready["outcome"]["data"]["data"].clone()).unwrap();
        let command = preview.command.unwrap();
        assert!(command.explicit_new_scenario_seed.is_some());
        assert!(exact_data(&host.current_draft().project, &before.project));
        assert!(host.modules.is_empty() && host.additions.is_empty() && host.scenarios.is_empty());
        let wrong_id = packaging_call(
            &mut host,
            "get_object",
            json!({"object_id":{"kind":"module","id":"other-template"},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":2,"choices":null,"seed":null}},"node_index":null}),
        );
        assert_eq!(wrong_id["outcome"]["data"]["code"], "InvalidInput");
        let wrong_version = ModuleRef {
            id: reference.id.clone(),
            version: VersionId::try_from("9.9.9".to_owned()).unwrap(),
        };
        assert_eq!(
            bundled_preview_call(&mut host, &wrong_version, Value::Null, Value::Null)["outcome"]["data"]
                ["code"],
            "MissingReference"
        );
        let mut foreign = task_authority_caller(&host, "foreign-bundled-preview");
        foreign.granted_project = Some(ProjectId::try_from("other-project".to_owned()).unwrap());
        foreign.allowed_operations = vec![OperationName::GetObject];
        let session = foreign.session_id.clone();
        host.register_external_session(foreign).unwrap();
        let denied = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "get_object",
            json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":2,"choices":null,"seed":null}},"node_index":null}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        assert!(denied["context"].is_null());
        let edit = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"set_project_settings","data":{"title":"a real edit after preview","time_reference":source.example_project.time_reference,"generation_context":null,"sources":[]}}]}),
        );
        assert_eq!(edit["outcome"]["status"], "ok");
        let stale = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":preview.base_revision,"commands":[{"kind":"instantiate_template","data":command}]}),
        );
        assert_eq!(stale["outcome"]["data"]["code"], "RevisionConflict");
        assert!(host.modules.is_empty() && host.scenarios.is_empty());
        let ready = bundled_preview_call(&mut host, &reference, choices, seed);
        assert_eq!(ready["outcome"]["status"], "ok", "{ready}");
        let preview: TemplatePreview =
            serde_json::from_value(ready["outcome"]["data"]["data"].clone()).unwrap();
        let result = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":preview.base_revision,"commands":[{"kind":"instantiate_template","data":preview.command.unwrap()}]}),
        );
        assert_eq!(result["outcome"]["status"], "ok", "{result}");
        assert_eq!(host.project.as_ref().unwrap().devices.len(), 3);
        assert_eq!(host.scenarios.len(), 1);
        let run = local_run(
            &mut host,
            &ScenarioId::try_from("PREVIEW-RUN".to_owned()).unwrap(),
        );
        assert_eq!(run["outcome"]["status"], "ok", "{run}");
        let task: TaskHandle = serde_json::from_value(run["outcome"]["data"].clone()).unwrap();
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let Some(TaskOutput::Run(summary)) = state.result else {
            panic!("real Run")
        };
        assert_eq!(
            host.runs[&summary.run_id]
                .result
                .acceptance_events
                .iter()
                .map(|event| event.at_ms.0.value())
                .collect::<Vec<_>>(),
            vec![70000, 110000]
        );
    }

    #[test]
    fn bundled_library_registration_is_not_adoption_and_survives_project_switch() {
        let mut host = ProjectHost::new(
            ObjectId::try_from("packaging-test-host".to_owned()).unwrap(),
            ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
        );
        register_test_bundled_library(&mut host);
        let count = host.bundled_modules.len();
        register_test_bundled_library(&mut host);
        assert_eq!(host.bundled_modules.len(), count);
        assert!(
            host.project.is_none()
                && host.modules.is_empty()
                && host.additions.is_empty()
                && host.scenarios.is_empty()
        );
        let all = bundled_query(&mut host, json!(["bundled"]));
        assert_eq!(all["outcome"]["status"], "ok", "{all}");
        let candidates = all["outcome"]["data"]["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), count);
        assert_eq!(
            candidates
                .iter()
                .filter(|entry| entry["category"] == "devices")
                .count(),
            15
        );
        assert_eq!(
            candidates
                .iter()
                .filter(|entry| entry["category"] == "templates")
                .count(),
            4
        );
        assert!(
            candidates
                .iter()
                .filter(|entry| entry["category"] == "templates")
                .all(|entry| entry["applicable"] == true
                    && !entry["missing"].as_array().unwrap().is_empty()),
            "unknown redistribution/conclusion evidence remains visible without prohibiting local adoption"
        );
        assert!(
            candidates
                .iter()
                .find(|entry| entry["module_ref"]["id"] == "community.template-carton-factory")
                .unwrap()["template_profile"]
                .is_object()
        );
        assert_eq!(
            bundled_query(&mut host, json!(["project_packages"]))["outcome"]["data"]["candidates"],
            json!([])
        );
        assert_eq!(
            bundled_query(&mut host, json!(["user_granted_libraries"]))["outcome"]["data"]["candidates"],
            json!([])
        );
        for departure in ["stay", "discard_after_tasks_terminal"] {
            let created = workspace_test_call(
                &mut host,
                "apply_commands",
                json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"empty bundled browser","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":departure}}]}),
            );
            assert_eq!(created["outcome"]["status"], "ok", "{created}");
            assert!(
                host.project.as_ref().unwrap().devices.is_empty()
                    && host.project.as_ref().unwrap().module_refs.is_empty()
            );
            assert!(
                host.scenarios.is_empty() && host.modules.is_empty() && host.additions.is_empty()
            );
            assert_eq!(
                bundled_query(&mut host, json!(["bundled"]))["outcome"]["data"]["candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                count
            );
        }
    }

    #[test]
    fn bundled_four_candidates_import_real_packages_and_assets_then_run_save_move_open() {
        let mut host = packaging_host();
        register_test_bundled_library(&mut host);
        let mut purchased = None;
        let mut others = vec![];
        for (which, namespace) in [
            (
                sf_module_kit::BuiltinTemplate::PurchasedAssembly,
                "bundled-assembly",
            ),
            (
                sf_module_kit::BuiltinTemplate::MachiningDelivery,
                "bundled-machining",
            ),
            (
                sf_module_kit::BuiltinTemplate::CartonFactory,
                "bundled-carton",
            ),
            (
                sf_module_kit::BuiltinTemplate::ParallelElectromechanical,
                "bundled-parallel",
            ),
        ] {
            let source = sf_module_kit::builtin_template(which).unwrap();
            let choices = template_choices(&source, namespace);
            let plan =
                sf_module_kit::prepare_template(&source, host.project.as_ref().unwrap(), &choices)
                    .unwrap();
            let command = plan.command_data_with_preparation(
                &RelativeFilePath::try_from("example-project.json".to_owned()).unwrap(),
                &choices,
                None,
                None,
            );
            let base = host.project.as_ref().unwrap().revision.clone();
            let result = packaging_call(
                &mut host,
                "apply_commands",
                json!({"base_revision":base,"commands":[{"kind":"instantiate_template","data":command}]}),
            );
            assert_eq!(result["outcome"]["status"], "ok", "{namespace}: {result}");
            assert!(
                host.scenarios.is_empty(),
                "supply and orders must remain a separate explicit selection"
            );
            if namespace == "bundled-assembly" {
                purchased = Some((source, plan));
            } else {
                let expected = match namespace {
                    "bundled-machining" => vec![90000, 150000],
                    "bundled-carton" => vec![45000, 65000],
                    "bundled-parallel" => vec![160000, 200000],
                    _ => unreachable!(),
                };
                others.push((source, plan, expected));
            }
        }
        let (source, plan) = purchased.unwrap();
        assert_eq!(host.project.as_ref().unwrap().devices.len(), 15);
        let assets: Vec<_> = host
            .modules
            .values()
            .flat_map(|package| &package.assets)
            .collect();
        assert!(assets.len() >= 6);
        assert_eq!(
            assets
                .iter()
                .map(|asset| asset.id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            assets.len()
        );
        assert_eq!(
            assets
                .iter()
                .map(|asset| asset.path.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            assets.len()
        );
        for module in host.bundled_modules.values() {
            assert!(exact_data(
                &host.modules[&(
                    module.package.definition.id.as_str().to_owned(),
                    String::from(module.package.definition.version.clone())
                )],
                &module.package
            ));
            for file in &module.additions {
                assert!(
                    host.additions
                        .iter()
                        .any(|actual| actual.bytes == file.bytes
                            && exact_data(&actual.entry, &file.entry))
                );
            }
        }
        let mut seed = sf_module_kit::prepare_template_seed(
            &source,
            &plan,
            &sf_module_kit::TemplateSeedChoices {
                scenario_id: ScenarioId::try_from("BUNDLED-RUN".to_owned()).unwrap(),
                supply: true,
                demand: true,
                initial_quality: source
                    .subset
                    .adoption
                    .as_ref()
                    .unwrap()
                    .deferred_seed
                    .as_ref()
                    .unwrap()
                    .initial_stock
                    .iter()
                    .map(|stock| (stock.location_id.clone(), QualityState::Released))
                    .collect(),
                adopt_declared_inspection_results: true,
            },
        )
        .unwrap();
        let project = host.project.as_ref().unwrap();
        seed.project_revision = project.revision.clone();
        let settings = json!({"title":project.title,"time_reference":seed.time_reference,"generation_context":project.generation_context,"sources":project.sources});
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":seed.project_revision,"commands":[{"kind":"set_project_settings","data":settings},{"kind":"put_object","data":{"kind":"scenario","data":seed}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let run = local_run(&mut host, &seed.id);
        assert_eq!(run["outcome"]["status"], "ok", "{run}");
        let task: TaskHandle = serde_json::from_value(run["outcome"]["data"].clone()).unwrap();
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let Some(TaskOutput::Run(summary)) = state.result else {
            panic!("real Run output")
        };
        let old_run = host.runs[&summary.run_id].result.clone();
        assert_eq!(
            old_run
                .acceptance_events
                .iter()
                .map(|event| event.at_ms.0.value())
                .collect::<Vec<_>>(),
            vec![70000, 110000]
        );
        for (index, (source, plan, expected)) in others.into_iter().enumerate() {
            let mut seed = sf_module_kit::prepare_template_seed(
                &source,
                &plan,
                &sf_module_kit::TemplateSeedChoices {
                    scenario_id: ScenarioId::try_from(format!("BUNDLED-OTHER-{index}")).unwrap(),
                    supply: true,
                    demand: true,
                    initial_quality: source
                        .subset
                        .adoption
                        .as_ref()
                        .unwrap()
                        .deferred_seed
                        .as_ref()
                        .unwrap()
                        .initial_stock
                        .iter()
                        .map(|stock| (stock.location_id.clone(), QualityState::Released))
                        .collect(),
                    adopt_declared_inspection_results: true,
                },
            )
            .unwrap();
            let project = host.project.as_ref().unwrap();
            seed.project_revision = project.revision.clone();
            let settings = json!({"title":project.title,"time_reference":seed.time_reference,"generation_context":project.generation_context,"sources":project.sources});
            let response = packaging_call(
                &mut host,
                "apply_commands",
                json!({"base_revision":seed.project_revision,"commands":[{"kind":"set_project_settings","data":settings},{"kind":"put_object","data":{"kind":"scenario","data":seed}}]}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            let run = local_run(&mut host, &seed.id);
            assert_eq!(run["outcome"]["status"], "ok", "{run}");
            let task: TaskHandle = serde_json::from_value(run["outcome"]["data"].clone()).unwrap();
            let state = packaging_complete(&mut host, &task);
            assert!(
                matches!(state.task.state, ComputationStatus::Completed),
                "{state:?}"
            );
            let Some(TaskOutput::Run(summary)) = state.result else {
                panic!("real template Run")
            };
            assert_eq!(
                host.runs[&summary.run_id]
                    .result
                    .acceptance_events
                    .iter()
                    .map(|event| event.at_ms.0.value())
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(exact_data(&host.runs[&old_run.id].result, &old_run));
        }
        let directory = ProfessionalTestDirectory::new();
        let moved = ProfessionalTestDirectory::new();
        let ui = ObjectId::try_from("packaging-test-ui".to_owned()).unwrap();
        let selected = host
            .select_directory(&ui, FilePurpose::SaveProject, directory.root.clone())
            .unwrap();
        let revision = host.project.as_ref().unwrap().revision.clone();
        let saved = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":revision,"selected_directory_ref":selected}),
        );
        assert_eq!(saved["outcome"]["status"], "ok", "{saved}");
        let task: TaskHandle = serde_json::from_value(saved["outcome"]["data"].clone()).unwrap();
        let saved_state = packaging_complete(&mut host, &task);
        assert!(
            matches!(saved_state.task.state, ComputationStatus::Completed),
            "{saved_state:?}"
        );
        let archive = read_project_archive(&directory.root).unwrap();
        crate::project_store::write_package_directory(
            &moved.root,
            &archive,
            &AtomicBool::new(false),
        )
        .unwrap();
        let project_before = host.project.as_ref().unwrap().clone();
        reopen_test_project(&mut host, &moved.root);
        assert!(exact_data(host.project.as_ref().unwrap(), &project_before));
        assert!(exact_data(&host.runs[&old_run.id].result, &old_run));
        for device in &project_before.devices {
            let read = packaging_call(
                &mut host,
                "get_object",
                json!({"object_id":{"kind":"module","id":device.module_ref.id},"source":{"kind":"run_module_version","data":{"run_id":old_run.id,"at_ms":110000,"module_ref":device.module_ref}},"node_index":null}),
            );
            assert_eq!(read["outcome"]["status"], "ok", "{read}");
        }
        assert_eq!(host.bundled_modules.len(), host.modules.len());
        for adoption in &project_before.template_adoptions {
            assert!(
                archive
                    .files
                    .contains_key(adoption.source_archive.path.as_str())
            );
        }
        let mut standalone = packaging_host();
        reopen_test_project(&mut standalone, &moved.root);
        assert!(standalone.bundled_modules.is_empty());
        assert!(exact_data(&standalone.runs[&old_run.id].result, &old_run));
        let template_ref = ModuleRef {
            id: source.package.definition.id.clone(),
            version: source.package.definition.version.clone(),
        };
        let archive_preview =
            bundled_preview_call(&mut standalone, &template_ref, Value::Null, Value::Null);
        assert_eq!(
            archive_preview["outcome"]["status"], "ok",
            "{archive_preview}"
        );
        let choices = template_choices(&source, "standalone-second-use");
        let standalone_plan = sf_module_kit::prepare_template(
            &source,
            standalone.project.as_ref().unwrap(),
            &choices,
        )
        .unwrap();
        let command = standalone_plan.command_data_with_preparation(
            &RelativeFilePath::try_from("example-project.json".to_owned()).unwrap(),
            &choices,
            None,
            None,
        );
        let revision = standalone.project.as_ref().unwrap().revision.clone();
        let reply = packaging_call(
            &mut standalone,
            "apply_commands",
            json!({"base_revision":revision,"commands":[{"kind":"instantiate_template","data":command}]}),
        );
        assert_eq!(
            reply["outcome"]["status"], "ok",
            "standalone archived source: {reply}"
        );
        assert!(exact_data(&standalone.runs[&old_run.id].result, &old_run));
        // A registered library must not repair a missing current-archive file
        // behind the caller's back or contaminate the saved version boundary.
        let ModulePayload::Templates(template) =
            &host.modules[&module_key(&template_ref)].definition.payload
        else {
            panic!("template source")
        };
        let template_path = template.project.as_str().to_owned();
        host.backing
            .as_mut()
            .unwrap()
            .files
            .remove(&template_path)
            .unwrap();
        let before = host.current_draft();
        let reply = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":revision,"commands":[{"kind":"instantiate_template","data":command}]}),
        );
        assert_eq!(
            reply["outcome"]["data"]["code"], "MissingReference",
            "{reply}"
        );
        assert!(exact_data(&before.project, host.project.as_ref().unwrap()));
        standalone
            .project
            .as_mut()
            .unwrap()
            .module_refs
            .retain(|reference| module_key(reference) != module_key(&template_ref));
        let before_local_read = standalone.project.as_ref().unwrap().clone();
        let retained_preview =
            bundled_preview_call(&mut standalone, &template_ref, Value::Null, Value::Null);
        assert_eq!(
            retained_preview["outcome"]["status"], "ok",
            "current archive exact readonly source: {retained_preview}"
        );
        assert!(retained_preview["outcome"]["data"]["data"]["command"].is_null());
        assert_eq!(
            retained_preview["outcome"]["data"]["data"]["module_ref"],
            json!(template_ref)
        );
        assert!(exact_data(
            standalone.project.as_ref().unwrap(),
            &before_local_read
        ));
    }

    #[test]
    fn bundled_adoption_is_authorized_and_atomic_on_later_failure_or_exact_version_conflict() {
        let mut host = packaging_host();
        register_test_bundled_library(&mut host);
        let source =
            sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::PurchasedAssembly)
                .unwrap();
        let plan = sf_module_kit::prepare_template(
            &source,
            host.project.as_ref().unwrap(),
            &template_choices(&source, "atomic-bundled"),
        )
        .unwrap();
        let command = plan.command_data(None);
        let before = serde_json::to_value(&host.current_draft().project).unwrap();
        let failure = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"instantiate_template","data":command},{"kind":"remove_object","data":{"kind":"device","id":"absent"}}]}),
        );
        assert_eq!(failure["outcome"]["status"], "error");
        assert_eq!(
            serde_json::to_value(&host.current_draft().project).unwrap(),
            before
        );
        assert!(host.modules.is_empty() && host.additions.is_empty() && host.scenarios.is_empty());
        let mut external = task_authority_caller(&host, "bundled-no-source-grant");
        external.allowed_operations = vec![
            OperationName::ApplyCommands,
            OperationName::QueryModuleLibrary,
        ];
        let id = external.session_id.clone();
        host.register_external_session(external).unwrap();
        let denied = authority_call(
            &mut host,
            &id,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"instantiate_template","data":command}]}),
        );
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(host.modules.is_empty() && host.additions.is_empty());
        let dependency = source.package.dependencies[0].clone();
        let mut conflict = host.bundled_modules[&module_key(&dependency)]
            .package
            .clone();
        conflict
            .definition
            .title
            .push_str(" changed actual contents");
        host.modules
            .insert(module_key(&dependency), conflict.clone());
        let failure = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"instantiate_template","data":command}]}),
        );
        assert_eq!(
            failure["outcome"]["data"]["code"], "InvalidInput",
            "{failure}"
        );
        assert_eq!(
            serde_json::to_value(&host.current_draft().project).unwrap(),
            before
        );
        assert_eq!(host.modules.len(), 1);
        assert!(exact_data(
            &host.modules[&module_key(&dependency)],
            &conflict
        ));
        assert!(host.additions.is_empty());
    }

    #[test]
    fn bundled_device_adoption_imports_only_the_selected_exact_package_and_keeps_unknown_rights() {
        let mut host = packaging_host();
        register_test_bundled_library(&mut host);
        let source =
            sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::PurchasedAssembly)
                .unwrap();
        let reference = source.subset.devices[0].module_ref.clone();
        let package = host.bundled_modules[&module_key(&reference)]
            .package
            .clone();
        let mut device = source.subset.devices[0].clone();
        device.id = ObjectId::try_from("explicit-new-device".to_owned()).unwrap();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":device.role_bindings,"new_instances":[{"kind":"device","data":device}],"explicit_seed":null}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert_eq!(host.modules.len(), 1);
        assert_eq!(host.project.as_ref().unwrap().devices.len(), 1);
        assert!(host.scenarios.is_empty());
        assert!(exact_data(&host.modules[&module_key(&reference)], &package));
        assert!(
            package
                .rights
                .iter()
                .all(|rights| !rights.redistribution_confirmed && rights.license.is_none())
        );
        assert_eq!(
            bundled_query(&mut host, json!(["project_packages"]))["outcome"]["data"]["candidates"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        let read = packaging_call(
            &mut host,
            "get_object",
            json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null}),
        );
        assert_eq!(read["outcome"]["status"], "ok", "{read}");
        let stale = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"adopt_module","data":{"module_ref":source.package.dependencies[1],"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}]}),
        );
        assert_eq!(stale["outcome"]["data"]["code"], "RevisionConflict");
        assert_eq!(host.modules.len(), 1);
        let allowed_ref = source.package.dependencies[1].clone();
        let mut external = task_authority_caller(&host, "bundled-exact-source-agent");
        external.allowed_operations = vec![OperationName::ApplyCommands];
        external.edit_grants = vec![HostEditGrant {
            id: ObjectId::try_from("bundled-exact-reference-only".to_owned()).unwrap(),
            scope: TaskScope::Edit(EditScope::Project(
                host.project.as_ref().unwrap().id.clone(),
            )),
            branches: vec![HostEditBranch::AdoptModule],
            objects: vec![],
            creates: vec![],
            source_versions: vec![allowed_ref.clone()],
            reusable_resources: vec![],
            created_objects: vec![],
        }];
        let session = external.session_id.clone();
        host.register_external_session(external).unwrap();
        let accepted = authority_call(
            &mut host,
            &session,
            ApiScope::Project,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"adopt_module","data":{"module_ref":allowed_ref,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}]}),
        );
        assert_eq!(accepted["outcome"]["status"], "ok", "{accepted}");
        assert_eq!(host.modules.len(), 2);
        assert_eq!(host.project.as_ref().unwrap().devices.len(), 1);
        assert!(
            host.sessions[&session].edit_grants[0]
                .created_objects
                .is_empty()
        );
    }

    #[test]
    fn bundled_registration_rejects_missing_asset_rights_and_conflicting_repeat_without_partial_entries()
     {
        let temporary = ProfessionalTestDirectory::new();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/templates/purchased-assembly/candidate");
        let document: Versioned<ModulePackage> =
            serde_json::from_slice(&std::fs::read(root.join("module.json")).unwrap()).unwrap();
        for path in std::iter::once("module.json")
            .chain(document.data.files.iter().map(|file| file.path.as_str()))
        {
            let target = temporary.root.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(root.join(path), target).unwrap();
        }
        let path = temporary.root.join("definitions/Assembly01/module.json");
        let original = std::fs::read(&path).unwrap();
        let mut tampered: Versioned<ModulePackage> = serde_json::from_slice(&original).unwrap();
        tampered.data.definition.contribution.assets.clear();
        std::fs::write(&path, serde_json::to_vec(&tampered).unwrap()).unwrap();
        let mut host = packaging_host();
        let before = host.current_draft();
        assert!(matches!(
            host.register_bundled_template_directory(&temporary.root)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        ));
        assert!(
            host.bundled_modules.is_empty() && host.modules.is_empty() && host.additions.is_empty()
        );
        assert!(exact_data(&before.project, host.project.as_ref().unwrap()));
        std::fs::write(&path, &original).unwrap();
        host.register_bundled_template_directory(&temporary.root)
            .unwrap();
        let prior = host.bundled_modules.len();
        // An actual immutable supporting-file change cannot be adopted by
        // registering the same ModuleRef again, even with identical definitions.
        let scene = temporary
            .root
            .join("definitions/Assembly01/sources/presentation/native-scene.json");
        let mut bytes = std::fs::read(&scene).unwrap();
        bytes.push(b'\n');
        std::fs::write(scene, bytes).unwrap();
        assert!(matches!(
            host.register_bundled_template_directory(&temporary.root)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        ));
        assert_eq!(host.bundled_modules.len(), prior);
        assert!(host.modules.is_empty() && host.additions.is_empty());
    }

    #[test]
    fn template_reception_replays_explicit_site_and_resource_choices_then_preserves_metadata_and_history()
     {
        let directory = ProfessionalTestDirectory::new();
        let (mut host, source) = template_reception_host(
            sf_module_kit::BuiltinTemplate::CartonFactory,
            &directory.root,
        );
        let mut existing_resource = source.example_project.resources[0].clone();
        let mut existing_calendar = source
            .example_project
            .calendars
            .iter()
            .find(|calendar| calendar.id == existing_resource.calendar_id)
            .unwrap()
            .clone();
        existing_calendar.id = ObjectId::try_from("actual-calendar".to_owned()).unwrap();
        existing_resource.id = ObjectId::try_from("actual-resource".to_owned()).unwrap();
        existing_resource.calendar_id = existing_calendar.id.clone();
        existing_resource.home_location = None;
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"resource","data":existing_resource}},{"kind":"put_object","data":{"kind":"calendar","data":existing_calendar}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let before = host.current_draft();
        let mut choices = template_choices(&source, "actual-template");
        choices.resources.insert(
            source.example_project.resources[0].id.clone(),
            sf_module_kit::TemplateResourceSelection::Reuse(existing_resource.id.clone()),
        );
        let mut plan = sf_module_kit::prepare_template(&source, &before.project, &choices).unwrap();
        let site: SiteEnvelope = serde_json::from_value(json!({"regions":[],"length_m":31,"width_m":13,"height_m":5,"usable_polygon_m":[],"clearance_m":0})).unwrap();
        sf_module_kit::fill_template_site(&mut plan, &site).unwrap();
        let resources: BTreeMap<_, Option<ObjectId>> = choices
            .resources
            .iter()
            .map(|(id, choice)| {
                (
                    id.clone(),
                    match choice {
                        sf_module_kit::TemplateResourceSelection::CreateDeclaredExample => None,
                        sf_module_kit::TemplateResourceSelection::Reuse(target) => {
                            Some(target.clone())
                        }
                    },
                )
            })
            .collect();
        let mut command = plan.command_data(None);
        command["preparation"] = json!({"source_project":"example-project.json","namespace":choices.namespace,"resources":resources,"reuse_items":{},"include_example_product":true,"site_envelope":site});
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"instantiate_template","data":command}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(exact_data(
            &host.project.as_ref().unwrap().template_adoptions,
            &plan.proposed_project.template_adoptions
        ));
        assert!(exact_data(
            &host.project.as_ref().unwrap().sources,
            &plan.proposed_project.sources
        ));
        assert!(exact_data(
            &host.project.as_ref().unwrap().parameter_sources,
            &plan.proposed_project.parameter_sources
        ));
        assert!(exact_data(
            host.project
                .as_ref()
                .unwrap()
                .resources
                .iter()
                .find(|resource| resource.id == existing_resource.id)
                .unwrap(),
            &existing_resource
        ));
        assert!(
            host.scenarios.is_empty(),
            "adoption alone must not supply orders or inventory"
        );
        assert!(host.project.as_ref().unwrap().acknowledgments.is_empty());
        assert!(host.project.as_ref().unwrap().bindings.is_empty());
        let committed = host.current_draft();
        for (operation, expected) in [("undo", &before), ("redo", &committed)] {
            let revision = host.project.as_ref().unwrap().revision.clone();
            let response = packaging_call(
                &mut host,
                "apply_commands",
                json!({"base_revision":revision,"commands":[{"kind":operation}]}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
            let mut actual = host.project.as_ref().unwrap().clone();
            actual.revision = expected.project.revision.clone();
            assert!(exact_data(&actual, &expected.project));
        }
        let moved = directory.root.join("saved-elsewhere");
        std::fs::create_dir(&moved).unwrap();
        let selected = host
            .select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::SaveProject,
                moved.clone(),
            )
            .unwrap();
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":revision,"selected_directory_ref":selected}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        let saved = host.project.as_ref().unwrap().clone();
        reopen_test_project(&mut host, &moved);
        assert!(exact_data(host.project.as_ref().unwrap(), &saved));
        let hall = &host.project.as_ref().unwrap().halls[0];
        for path in ["/width_m", "/depth_m", "/height_m"] {
            let record = host
                .project
                .as_ref()
                .unwrap()
                .parameter_sources
                .iter()
                .find(|record| record.target.id == hall.id && record.field_path == path)
                .unwrap();
            assert!(matches!(record.basis, SourceBasis::UserProvided));
            assert!(record.source_ref.is_none());
        }
    }

    #[test]
    fn template_reception_rejects_forged_source_or_mapping_and_keeps_failed_batch_atomic() {
        let directory = ProfessionalTestDirectory::new();
        let (mut host, source) = template_reception_host(
            sf_module_kit::BuiltinTemplate::ParallelElectromechanical,
            &directory.root,
        );
        let choices = template_choices(&source, "source-bound");
        let plan =
            sf_module_kit::prepare_template(&source, host.project.as_ref().unwrap(), &choices)
                .unwrap();
        let command = plan.command_data(None);
        let before = serde_json::to_value(host.project.as_ref().unwrap()).unwrap();
        let mut forged_subset = command.clone();
        forged_subset["subset"]["adoption"]["imported_claims"][0]["declared_scope"] =
            json!(["fabricated_full_support"]);
        let mut forged_objects = command.clone();
        let device = forged_objects["proposed_objects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|object| object["kind"] == "device")
            .unwrap();
        device["data"]["title"] = json!("Caller-supplied unrelated replacement");
        let mut forged_path = command.clone();
        forged_path["preparation"] = json!({"source_project":"unlisted-project.json","namespace":"source-bound","resources":{},"reuse_items":{},"include_example_product":true,"site_envelope":null});
        for (forged, code) in [
            (forged_subset, "MissingReference"),
            (forged_objects, "InvalidInput"),
            (forged_path, "InvalidInput"),
        ] {
            let response = packaging_call(
                &mut host,
                "apply_commands",
                json!({"base_revision":1,"commands":[{"kind":"instantiate_template","data":forged}]}),
            );
            assert_eq!(response["outcome"]["data"]["code"], code, "{response}");
            assert_eq!(
                serde_json::to_value(host.project.as_ref().unwrap()).unwrap(),
                before
            );
            assert!(!host.history.availability().can_undo);
        }
        let project = host.project.as_ref().unwrap().id.clone();
        let external = ObjectId::try_from("template-limited-agent".to_owned()).unwrap();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(project.clone()),
            allowed_operations: vec![OperationName::ApplyCommands],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![HostEditGrant {
                id: ObjectId::try_from("exact-template-no-creation".to_owned()).unwrap(),
                scope: TaskScope::Edit(EditScope::Project(project.clone())),
                branches: vec![HostEditBranch::InstantiateTemplate],
                objects: vec![],
                creates: vec![],
                source_versions: vec![plan.source_ref.clone()],
                reusable_resources: vec![],
                created_objects: vec![],
            }],
        })
        .unwrap();
        let denied = host.dispatch(&external,serde_json::from_value(json!({"api_version":"0.1","request_id":"template-permission-priority","project_id":project,"scope":{"kind":"project"},"operation":"apply_commands","params":{"base_revision":1,"commands":[{"kind":"instantiate_template","data":command}]}})).unwrap());
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(
            host.caller_context(&external).unwrap().edit_grants[0]
                .created_objects
                .is_empty()
        );
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"instantiate_template","data":plan.command_data(None)},{"kind":"remove_object","data":{"kind":"device","id":"absent-device"}}]}),
        );
        assert_eq!(response["outcome"]["status"], "error", "{response}");
        assert_eq!(
            serde_json::to_value(host.project.as_ref().unwrap()).unwrap(),
            before
        );
        assert!(!host.history.availability().can_undo);
        let mut reordered = plan.command_data(None);
        reordered["proposed_objects"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"instantiate_template","data":reordered}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        assert!(exact_data(
            &host.project.as_ref().unwrap().template_adoptions,
            &plan.proposed_project.template_adoptions
        ));
        assert!(exact_data(
            &host.project.as_ref().unwrap().sources,
            &plan.proposed_project.sources
        ));
        assert!(exact_data(
            &host.project.as_ref().unwrap().parameter_sources,
            &plan.proposed_project.parameter_sources
        ));
        let current = host.current_draft();
        let source = current
            .project
            .parameter_sources
            .iter()
            .find(|source| {
                matches!(source.target.kind, ObjectKind::Device)
                    && matches!(source.value, Some(TypedValue::Duration(_)))
            })
            .unwrap();
        let mut changed = draft_value(&current, &source.target).unwrap();
        let duration = changed.pointer_mut(&source.field_path).unwrap();
        duration["value"] = serde_json::json!(duration["value"].as_u64().unwrap() + 1);
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":current.project.revision,"commands":[{"kind":"put_object","data":{"kind":source.target.kind,"data":changed}}]}),
        );
        assert_eq!(
            response["outcome"]["data"]["code"], "InvalidInput",
            "{response}"
        );
        assert!(
            exact_data(host.project.as_ref().unwrap(), &current.project),
            "an actual recorded parameter value remains protected against unrelated PutObject changes"
        );
    }

    fn packaging_case(name: &str) -> PackageStructureJob {
        let fixtures: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/packaging-acceptance-cases.json"
        ))
        .unwrap();
        serde_json::from_value(
            fixtures["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["id"] == name)
                .unwrap()["job"]["data"]
                .clone(),
        )
        .unwrap()
    }

    fn packaging_admit(
        host: &mut ProjectHost,
        job: &PackageStructureJob,
        budget: &CalculationBudget,
    ) -> TaskHandle {
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            host,
            "compute_packaging",
            json!({"packaging_job":job,"input_revision":revision,"computation_budget":budget}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
    }

    fn packaging_task(host: &mut ProjectHost, task: &TaskHandle) -> TaskState {
        let response = packaging_call(host, "get_task", json!({"task_id":task.id}));
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
    }

    fn packaging_complete(host: &mut ProjectHost, task: &TaskHandle) -> TaskState {
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        let completion = work.pop().unwrap().run();
        if let Ok(WorkOutput::Packaging {
            product,
            input_bytes,
        }) = &completion.result
        {
            fn difference(a: &Value, b: &Value, path: String) -> Option<String> {
                if a == b {
                    return None;
                }
                match (a, b) {
                    (Value::Object(a), Value::Object(b)) => a.iter().find_map(|(key, value)| {
                        difference(
                            value,
                            b.get(key).unwrap_or(&Value::Null),
                            format!("{path}/{key}"),
                        )
                    }),
                    (Value::Array(a), Value::Array(b)) => a
                        .iter()
                        .zip(b)
                        .enumerate()
                        .find_map(|(i, (a, b))| difference(a, b, format!("{path}/{i}"))),
                    _ => Some(format!("{path}: {a} != {b}")),
                }
            }
            let decoded_job: Versioned<PackageStructureJob> =
                serde_json::from_slice(input_bytes).unwrap();
            let decoded_result: Versioned<PackagingResult> =
                serde_json::from_slice(&encode_document(&product.result)).unwrap();
            assert!(
                exact_data(&product.selected_job, &decoded_job.data),
                "Actual frozen Job roundtrip difference: {:?}",
                difference(
                    &serde_json::to_value(&product.selected_job).unwrap(),
                    &serde_json::to_value(&decoded_job.data).unwrap(),
                    String::new()
                )
            );
            assert!(
                exact_data(&product.result, &decoded_result.data),
                "Actual result roundtrip difference: {:?}",
                difference(
                    &serde_json::to_value(&product.result).unwrap(),
                    &serde_json::to_value(&decoded_result.data).unwrap(),
                    String::new()
                )
            );
        }
        host.finish_io(completion);
        packaging_task(host, task)
    }

    fn local_production_host() -> ProjectHost {
        let mut host = packaging_host();
        let fixture: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/local-assembly-pack.json"
        ))
        .unwrap();
        let project: ProjectState = serde_json::from_value(fixture["project"].clone()).unwrap();
        let scenario: ScenarioInput = serde_json::from_value(fixture["scenario"].clone()).unwrap();
        host.modules = fixture["definitions"].as_array().unwrap().iter().map(|definition| {
            let package: ModulePackage = serde_json::from_value(json!({"definition":definition,"assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]})).unwrap();
            (module_key(&ModuleRef { id:package.definition.id.clone(),version:package.definition.version.clone() }),package)
        }).collect();
        host.sessions
            .get_mut(&ObjectId::try_from("packaging-test-ui".to_owned()).unwrap())
            .unwrap()
            .granted_project = Some(project.id.clone());
        host.project = Some(project);
        host.scenario_highwater
            .insert(scenario.id.clone(), scenario.revision.clone());
        host.scenarios.insert(scenario.id.clone(), scenario);
        let mut draft = host.current_draft();
        host.refresh_current_business(&mut draft);
        host.project = Some(draft.project);
        host.history.reset(&host.current_draft());
        host
    }

    fn workspace_test_call(host: &mut ProjectHost, operation: &str, params: Value) -> Value {
        host.dispatch(&ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),serde_json::from_value(json!({"api_version":"0.1","request_id":"packaging-workspace-test","project_id":null,"scope":{"kind":"workspace"},"operation":operation,"params":params})).unwrap())
    }

    fn reopen_test_project(host: &mut ProjectHost, path: &std::path::Path) {
        let selected = if path.is_dir() {
            host.select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::ImportPackage,
                path.to_owned(),
            )
        } else {
            host.select_file(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::ImportPackage,
                path.to_owned(),
            )
        }
        .unwrap();
        let response = workspace_test_call(
            host,
            "import_package",
            json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let state = workspace_test_call(host, "get_task", json!({"task_id":task.id}));
        assert_eq!(
            state["outcome"]["data"]["task"]["state"], "completed",
            "{state}"
        );
        let response = workspace_test_call(
            host,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task.id,"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
    }

    fn professional_production_host() -> (ProjectHost, PackageStructureJob) {
        let mut host = local_production_host();
        let declarations: Value =
            serde_json::from_str(include_str!("../../packaging/consumer-device.design.json"))
                .unwrap();
        let mut job = packaging_case("COVERAGE-restricted_folded_container");
        job.subject_ref = SubjectRevisionRef {
            id: ObjectId::try_from("PACKAGE".to_owned()).unwrap(),
            revision: SubjectRevisionId::try_from("r1".to_owned()).unwrap(),
        };
        job.conditions.product_envelope.as_mut().unwrap().subject = job.subject_ref.clone();
        let device = host
            .project
            .as_mut()
            .unwrap()
            .devices
            .iter_mut()
            .find(|device| device.id.as_str() == "PACK-01")
            .unwrap();
        device.configuration = serde_json::from_value(json!({"packaging_job_id":{"kind":"string","value":job.id},"packaging_job_revision":{"kind":"integer","value":job.revision}})).unwrap();
        let package = host
            .modules
            .get_mut(&module_key(&device.module_ref))
            .unwrap();
        package.definition.parameters = serde_json::from_value(json!(
            declarations["data"]["definition"]["parameters"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|declaration| declaration["id"] == "packaging_job_id"
                    || declaration["id"] == "packaging_job_revision")
                .cloned()
                .collect::<Vec<_>>()
        ))
        .unwrap();
        for (index, package) in host.modules.values().enumerate() {
            host.additions.push(ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::ModuleDefinition,
                    module_ref: Some(ModuleRef {
                        id: package.definition.id.clone(),
                        version: package.definition.version.clone(),
                    }),
                    object_ref: ObjectRef {
                        kind: ObjectKind::Module,
                        id: ObjectId::try_from(package.definition.id.as_str().to_owned()).unwrap(),
                    },
                    revision: None,
                    file_ref: file_ref(
                        &format!("fixture/module-{:04}.json", index + 1),
                        DocumentKind::Module,
                    ),
                    required: true,
                },
                bytes: encode_document(package),
            });
        }
        (host, job)
    }

    fn actual_packaging_result(
        host: &mut ProjectHost,
        job: &PackageStructureJob,
    ) -> PackagingResult {
        let task = packaging_admit(host, job, job.budget.as_ref().unwrap());
        let state = packaging_complete(host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let Some(TaskOutput::Packaging(result)) = state.result else {
            panic!("actual PackagingResult")
        };
        assert!(matches!(result.outcome, CheckOutcome::Pass));
        result
    }

    fn adopt_packaging_choice(
        host: &mut ProjectHost,
        artifact: &ArtifactId,
        acknowledgment: &str,
    ) -> BindingCommit {
        let params = packaging_adoption_params(host, artifact, acknowledgment);
        let response = packaging_call(host, "adopt_artifact", params);
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
    }

    fn packaging_adoption_params(
        host: &ProjectHost,
        artifact: &ArtifactId,
        acknowledgment: &str,
    ) -> Value {
        let revision = host.project.as_ref().unwrap().revision.clone();
        json!({"artifact_id":artifact,"acceptance_ref":acknowledgment,"consumer_ref":{"kind":"device","id":"PACK-01"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},"requested_scope":["forming","opening","linear_extraction","net_space","manufacturing"],"base_revision":revision})
    }

    fn select_product_and_packaging_job(
        host: &mut ProjectHost,
        product: &ProductRevision,
        job_revision: &Revision,
    ) {
        let mut scenario =
            host.scenarios[&ScenarioId::try_from("LOCAL".to_owned()).unwrap()].clone();
        for order in &mut scenario.orders {
            if order.product_ref.id == product.id {
                order.product_ref.revision = product.revision.clone();
            }
        }
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            host,
            "apply_commands",
            json!({"base_revision":revision,"commands":[{"kind":"put_object","data":{"kind":"product","data":product}},{"kind":"put_object","data":{"kind":"scenario","data":scenario}},{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":job_revision},"basis":"user_provided"}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
    }

    fn write_test_archive(path: &std::path::Path, archive: &ProjectArchive) {
        std::fs::create_dir(path).unwrap();
        for (name, bytes) in &archive.files {
            let file = path.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        std::fs::write(
            path.join("project.json"),
            encode_document(&archive.manifest),
        )
        .unwrap();
    }

    fn acknowledge_packaging_choice(
        host: &mut ProjectHost,
        job: &PackageStructureJob,
        artifact: &ArtifactId,
        acknowledgment: &str,
    ) {
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            host,
            "apply_commands",
            json!({"base_revision":revision,"commands":[{"kind":"record_artifact_acknowledgment","data":{"id":acknowledgment,"artifact_ref":artifact,"accepted_subject":job.subject_ref,"accepted_scope":["manufacturing","net_space","linear_extraction","opening","forming"],"decision_source":{"kind":"human_statement","data":"actual UI explicit purposes"},"conditions":[],"recorded_at_iso8601":null}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
    }

    fn select_packaging_job_revision(host: &mut ProjectHost, revision: &Revision) {
        let base = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            host,
            "apply_commands",
            json!({"base_revision":base,"commands":[{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":revision},"basis":"user_provided"}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
    }

    fn actual_local_run(host: &mut ProjectHost) -> RunResult {
        let response = local_run(host, &ScenarioId::try_from("LOCAL".to_owned()).unwrap());
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let state = packaging_complete(host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let Some(TaskOutput::Run(summary)) = state.result else {
            panic!("actual RunSummary")
        };
        let result = host.runs[&summary.run_id].result.clone();
        assert_eq!(result.acceptance_events.len(), 1);
        assert_eq!(result.acceptance_events[0].at_ms.0.value(), 75000);
        result
    }

    #[test]
    fn explicit_new_adoptions_select_current_use_and_preserve_old_versions_bindings_and_runs() {
        let (mut host, job_r1) = professional_production_host();
        let initial =
            serde_json::to_value(&host.scenarios.values().next().unwrap().initial_conditions)
                .unwrap();
        let result_r1 = actual_packaging_result(&mut host, &job_r1);
        acknowledge_packaging_choice(&mut host, &job_r1, &result_r1.artifact_id, "CHOICE-R1");
        let binding_r1 =
            adopt_packaging_choice(&mut host, &result_r1.artifact_id, "CHOICE-R1").binding;
        let project_before_repeat = serde_json::to_value(host.project.as_ref().unwrap()).unwrap();
        let generation_before_repeat = host.content_generation;
        let repeated = adopt_packaging_choice(&mut host, &result_r1.artifact_id, "CHOICE-R1");
        assert_eq!(repeated.binding.id, binding_r1.id);
        assert_eq!(
            repeated.new_revision.0.value(),
            repeated.previous_revision.0.value()
        );
        assert_eq!(host.content_generation, generation_before_repeat);
        assert_eq!(
            serde_json::to_value(host.project.as_ref().unwrap()).unwrap(),
            project_before_repeat
        );
        let old_run = actual_local_run(&mut host);
        let old_project_bytes = host
            .additions
            .iter()
            .find(|addition| {
                addition.entry.file_ref.path == old_run.input_ref.frozen_project_file.path
            })
            .unwrap()
            .bytes
            .clone();
        let old_artifact = host.professional.artifacts[&result_r1.artifact_id].clone();
        let old_ack = host.project.as_ref().unwrap().acknowledgments[0].clone();

        let mut job_r2 = packaging_case("COVERAGE-material-change");
        job_r2.id = job_r1.id.clone();
        job_r2.revision = Revision(SafeUint::try_from(2).unwrap());
        job_r2.subject_ref = job_r1.subject_ref.clone();
        job_r2.conditions.product_envelope.as_mut().unwrap().subject = job_r2.subject_ref.clone();
        assert!(!exact_data(&job_r1.materials, &job_r2.materials));
        let result_r2 = actual_packaging_result(&mut host, &job_r2);
        assert_eq!(
            current_consumer_bindings(host.project.as_ref().unwrap())[0].id,
            binding_r1.id,
            "actual computation does not select the newer artifact"
        );
        select_packaging_job_revision(&mut host, &job_r2.revision);
        let next_task = host.next_task;
        let rejected = local_run(
            &mut host,
            &ScenarioId::try_from("LOCAL".to_owned()).unwrap(),
        );
        assert_eq!(rejected["outcome"]["status"], "error", "{rejected}");
        assert_eq!(host.next_task, next_task);
        assert!(host.drain_io_work().is_empty());
        acknowledge_packaging_choice(&mut host, &job_r2, &result_r2.artifact_id, "CHOICE-R2");
        let binding_r2 =
            adopt_packaging_choice(&mut host, &result_r2.artifact_id, "CHOICE-R2").binding;
        assert_ne!(binding_r2.id, binding_r1.id);
        assert_eq!(host.project.as_ref().unwrap().bindings.len(), 2);
        assert_eq!(
            current_consumer_bindings(host.project.as_ref().unwrap())[0].id,
            binding_r2.id
        );
        actual_local_run(&mut host);

        // Re-selecting historical r1 is a new explicit fact, not a latest lookup
        // or deletion of r2. Repeating that now-current choice is idempotent.
        select_packaging_job_revision(&mut host, &job_r1.revision);
        let selected_again =
            adopt_packaging_choice(&mut host, &result_r1.artifact_id, "CHOICE-R1").binding;
        assert_ne!(selected_again.id, binding_r1.id);
        assert_ne!(selected_again.id, binding_r2.id);
        assert_eq!(host.project.as_ref().unwrap().bindings.len(), 3);
        let repeated = adopt_packaging_choice(&mut host, &result_r1.artifact_id, "CHOICE-R1");
        assert_eq!(repeated.binding.id, selected_again.id);
        assert_eq!(host.project.as_ref().unwrap().bindings.len(), 3);
        actual_local_run(&mut host);
        let history = host.project.as_ref().unwrap().bindings.clone();
        assert!(exact_data(&history[0], &binding_r1));
        assert!(exact_data(&history[1], &binding_r2));
        assert!(exact_data(
            &host.professional.jobs[&(job_r1.id.clone(), 1)],
            &job_r1
        ));
        assert!(exact_data(
            &host.professional.results[&result_r1.artifact_id],
            &result_r1
        ));
        assert!(exact_data(
            &host.professional.artifacts[&result_r1.artifact_id],
            &old_artifact
        ));
        assert!(exact_data(
            &host.project.as_ref().unwrap().acknowledgments[0],
            &old_ack
        ));
        assert!(exact_data(&host.runs[&old_run.id].result, &old_run));
        assert_eq!(
            host.additions
                .iter()
                .find(|addition| addition.entry.file_ref.path
                    == old_run.input_ref.frozen_project_file.path)
                .unwrap()
                .bytes,
            old_project_bytes
        );
        assert_eq!(
            serde_json::to_value(&host.scenarios.values().next().unwrap().initial_conditions)
                .unwrap(),
            initial
        );

        let directory = ProfessionalTestDirectory::new();
        let revision = host.project.as_ref().unwrap().revision.clone();
        let selection = host
            .select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::SaveProject,
                directory.root.clone(),
            )
            .unwrap();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":revision,"selected_directory_ref":selection}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        reopen_test_project(&mut host, &directory.root);
        assert!(exact_data(
            &host.project.as_ref().unwrap().bindings,
            &history
        ));
        assert!(exact_data(&host.runs[&old_run.id].result, &old_run));
        let catalog = workspace_test_call(&mut host, "describe_capabilities", json!({}));
        assert!(
            catalog["outcome"]["data"]["implemented_operations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|operation| operation["operation"] == "adopt_artifact"),
            "a valid current repeat remains an actual adoption path after Open"
        );
        let reopened_revision = host.project.as_ref().unwrap().revision.clone();
        let repeated = adopt_packaging_choice(&mut host, &result_r1.artifact_id, "CHOICE-R1");
        assert_eq!(repeated.binding.id, selected_again.id);
        assert_eq!(repeated.new_revision.0.value(), reopened_revision.0.value());
        assert!(exact_data(
            &host.project.as_ref().unwrap().bindings,
            &history
        ));
        actual_local_run(&mut host);
    }

    #[test]
    fn historical_choice_after_open_requires_exact_genuine_facts_and_a_new_authorized_adoption() {
        let (mut host, job_r1) = professional_production_host();
        let product_r1 = host.project.as_ref().unwrap().products[0].clone();
        let initial =
            serde_json::to_value(&host.scenarios.values().next().unwrap().initial_conditions)
                .unwrap();
        let result_r1 = actual_packaging_result(&mut host, &job_r1);
        acknowledge_packaging_choice(&mut host, &job_r1, &result_r1.artifact_id, "OPEN-R1");
        let binding_r1 =
            adopt_packaging_choice(&mut host, &result_r1.artifact_id, "OPEN-R1").binding;
        let run_r1 = actual_local_run(&mut host);

        let mut product_r2 = product_r1.clone();
        product_r2.revision = SubjectRevisionId::try_from("r2".to_owned()).unwrap();
        let mut job_r2 = packaging_case("COVERAGE-material-change");
        job_r2.id = job_r1.id.clone();
        job_r2.revision = Revision(SafeUint::try_from(2).unwrap());
        job_r2.subject_ref = SubjectRevisionRef {
            id: product_r2.id.clone(),
            revision: product_r2.revision.clone(),
        };
        job_r2.conditions.product_envelope.as_mut().unwrap().subject = job_r2.subject_ref.clone();
        select_product_and_packaging_job(&mut host, &product_r2, &job_r2.revision);
        let result_r2 = actual_packaging_result(&mut host, &job_r2);
        acknowledge_packaging_choice(&mut host, &job_r2, &result_r2.artifact_id, "OPEN-R2");
        let binding_r2 =
            adopt_packaging_choice(&mut host, &result_r2.artifact_id, "OPEN-R2").binding;
        let run_r2 = actual_local_run(&mut host);
        let actual_trust = host.professional_trust.clone();
        let old_artifacts = serde_json::to_value(&host.professional.artifacts).unwrap();
        let old_acknowledgments =
            serde_json::to_value(&host.project.as_ref().unwrap().acknowledgments).unwrap();
        let frozen_inputs: Vec<_> = [&run_r1, &run_r2]
            .into_iter()
            .map(|run| {
                let path = run.input_ref.frozen_project_file.path.clone();
                let bytes = host
                    .additions
                    .iter()
                    .find(|addition| addition.entry.file_ref.path == path)
                    .unwrap()
                    .bytes
                    .clone();
                (path, bytes)
            })
            .collect();

        let directory = ProfessionalTestDirectory::new();
        let saved = directory.root.join("saved");
        let moved = directory.root.join("moved");
        let altered = directory.root.join("altered");
        std::fs::create_dir(&saved).unwrap();
        let selection = host
            .select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::SaveProject,
                saved.clone(),
            )
            .unwrap();
        let revision = host.project.as_ref().unwrap().revision.clone();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":revision,"selected_directory_ref":selection}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));
        let archive = read_project_archive(&saved).unwrap();
        write_test_archive(&moved, &archive);
        write_test_archive(&altered, &archive);

        // A real producer registration in another Host is insufficient: Open
        // cannot turn the archive's source labels into that Host's own receipt.
        let (mut fresh, _) = professional_production_host();
        actual_packaging_result(&mut fresh, &job_r1);
        reopen_test_project(&mut fresh, &moved);
        select_product_and_packaging_job(&mut fresh, &product_r1, &job_r1.revision);
        let before = serde_json::to_value(fresh.project.as_ref().unwrap()).unwrap();
        let params = packaging_adoption_params(&fresh, &result_r1.artifact_id, "OPEN-R1");
        let denied = packaging_call(&mut fresh, "adopt_artifact", params);
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(
            denied["outcome"]["data"]["field_errors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field["pointer"] == "/artifact_id")
        );
        assert_eq!(
            serde_json::to_value(fresh.project.as_ref().unwrap()).unwrap(),
            before
        );

        let entry = archive
            .manifest
            .documents
            .iter()
            .find(|entry| {
                entry
                    .module_ref
                    .as_ref()
                    .is_some_and(|reference| reference.id.as_str() == "fixture.pack")
            })
            .unwrap();
        let mut package: Versioned<ModulePackage> =
            serde_json::from_slice(&archive.files[entry.file_ref.path.as_str()]).unwrap();
        package
            .data
            .definition
            .title
            .push_str("; altered after real adoption");
        std::fs::write(
            altered.join(entry.file_ref.path.as_str()),
            encode_document(&package.data),
        )
        .unwrap();
        reopen_test_project(&mut host, &altered);
        select_product_and_packaging_job(&mut host, &product_r1, &job_r1.revision);
        let params = packaging_adoption_params(&host, &result_r1.artifact_id, "OPEN-R1");
        let modules: Vec<_> = host.modules.values().cloned().collect();
        assert!(
            prepare_artifact_binding(
                &host.current_draft(),
                &modules,
                &serde_json::from_value(params.clone()).unwrap(),
                ObjectId::try_from("pure-comparison".to_owned()).unwrap(),
                &actual_trust,
                &host.providers
            )
            .is_ok(),
            "the content change passes ordinary consumer gates, so the receipt must reject it"
        );
        let before = serde_json::to_value(host.project.as_ref().unwrap()).unwrap();
        let denied = packaging_call(&mut host, "adopt_artifact", params);
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert_eq!(
            serde_json::to_value(host.project.as_ref().unwrap()).unwrap(),
            before
        );
        let catalog = workspace_test_call(&mut host, "describe_capabilities", json!({}));
        assert!(
            !catalog["outcome"]["data"]["implemented_operations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|operation| operation["operation"] == "adopt_artifact")
        );

        // The saved current choice is still b2 after moving and opening. Merely
        // restoring independent r1 inputs cannot select the historical b1.
        reopen_test_project(&mut host, &moved);
        select_product_and_packaging_job(&mut host, &product_r1, &job_r1.revision);
        assert_eq!(
            current_consumer_bindings(host.project.as_ref().unwrap())[0].id,
            binding_r2.id
        );
        let next_task = host.next_task;
        let rejected = local_run(
            &mut host,
            &ScenarioId::try_from("LOCAL".to_owned()).unwrap(),
        );
        assert_eq!(rejected["outcome"]["status"], "error", "{rejected}");
        assert_eq!(host.next_task, next_task);
        assert!(host.drain_io_work().is_empty());
        let catalog = workspace_test_call(&mut host, "describe_capabilities", json!({}));

        let project_id = host.project.as_ref().unwrap().id.clone();
        let external = ObjectId::try_from("historical-choice-no-grant".to_owned()).unwrap();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(project_id.clone()),
            allowed_operations: vec![OperationName::AdoptArtifact],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        let params = packaging_adoption_params(&host, &result_r1.artifact_id, "OPEN-R1");
        let before = serde_json::to_value(host.project.as_ref().unwrap()).unwrap();
        let denied = host.dispatch(&external, serde_json::from_value(json!({"api_version":"0.1","request_id":"historical-no-grant","project_id":project_id,"scope":{"kind":"project"},"operation":"adopt_artifact","params":params})).unwrap());
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert_eq!(
            serde_json::to_value(host.project.as_ref().unwrap()).unwrap(),
            before
        );

        let previous = host.project.as_ref().unwrap().revision.0.value();
        let adopted = adopt_packaging_choice(&mut host, &result_r1.artifact_id, "OPEN-R1");
        assert_eq!(adopted.previous_revision.0.value(), previous);
        assert_eq!(adopted.new_revision.0.value(), previous + 1);
        assert_ne!(adopted.binding.id, binding_r1.id);
        assert_ne!(adopted.binding.id, binding_r2.id);
        assert_eq!(host.project.as_ref().unwrap().bindings.len(), 3);
        assert_eq!(
            current_consumer_bindings(host.project.as_ref().unwrap())[0].id,
            adopted.binding.id
        );
        assert!(
            catalog["outcome"]["data"]["implemented_operations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|operation| operation["operation"] == "adopt_artifact"),
            "eligible historical choice must be advertised before it commits"
        );
        actual_local_run(&mut host);
        assert!(exact_data(
            &host.project.as_ref().unwrap().bindings[0],
            &binding_r1
        ));
        assert!(exact_data(
            &host.project.as_ref().unwrap().bindings[1],
            &binding_r2
        ));
        assert!(exact_data(
            &host.professional.jobs[&(job_r1.id.clone(), 1)],
            &job_r1
        ));
        assert!(exact_data(
            &host.professional.jobs[&(job_r2.id.clone(), 2)],
            &job_r2
        ));
        assert!(exact_data(
            &host.professional.results[&result_r1.artifact_id],
            &result_r1
        ));
        assert!(exact_data(
            &host.professional.results[&result_r2.artifact_id],
            &result_r2
        ));
        assert_eq!(
            serde_json::to_value(&host.professional.artifacts).unwrap(),
            old_artifacts
        );
        assert_eq!(
            serde_json::to_value(&host.project.as_ref().unwrap().acknowledgments).unwrap(),
            old_acknowledgments
        );
        assert!(exact_data(&host.runs[&run_r1.id].result, &run_r1));
        assert!(exact_data(&host.runs[&run_r2.id].result, &run_r2));
        for (path, bytes) in frozen_inputs {
            assert_eq!(host.backing.as_ref().unwrap().files[path.as_str()], bytes);
        }
        assert_eq!(
            serde_json::to_value(&host.scenarios.values().next().unwrap().initial_conditions)
                .unwrap(),
            initial
        );
    }

    struct ProfessionalTestDirectory {
        root: PathBuf,
        base: PathBuf,
    }
    impl ProfessionalTestDirectory {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let base = std::env::temp_dir().canonicalize().unwrap();
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = base.join(format!(
                "sf-host-professional-{}-{nonce}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir(&root).unwrap();
            Self { root, base }
        }
    }
    impl Drop for ProfessionalTestDirectory {
        fn drop(&mut self) {
            let actual = self.root.canonicalize().unwrap();
            assert_eq!(actual.parent(), Some(self.base.as_path()));
            assert!(
                actual
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("sf-host-professional-")
            );
            std::fs::remove_dir_all(actual).unwrap();
        }
    }

    #[test]
    fn reopened_nominal_binding_requires_the_exact_actually_committed_module_content() {
        let directory = ProfessionalTestDirectory::new();
        let saved = directory.root.join("saved");
        let altered = directory.root.join("altered");
        std::fs::create_dir(&saved).unwrap();
        std::fs::create_dir(&altered).unwrap();
        let (mut host, job) = professional_production_host();
        let task = packaging_admit(&mut host, &job, job.budget.as_ref().unwrap());
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let Some(TaskOutput::Packaging(result)) = state.result else {
            panic!("actual packaging result")
        };
        assert!(matches!(result.outcome, CheckOutcome::Pass));
        let scope = json!([
            "forming",
            "opening",
            "linear_extraction",
            "net_space",
            "manufacturing"
        ]);
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"record_artifact_acknowledgment","data":{"id":"ACTUAL-RECEIPT-ACK","artifact_ref":result.artifact_id,"accepted_subject":job.subject_ref,"accepted_scope":scope,"decision_source":{"kind":"human_statement","data":"actual UI purpose acknowledgment"},"conditions":[],"recorded_at_iso8601":null}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let response = packaging_call(
            &mut host,
            "adopt_artifact",
            json!({"artifact_id":result.artifact_id,"acceptance_ref":"ACTUAL-RECEIPT-ACK","consumer_ref":{"kind":"device","id":"PACK-01"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},"requested_scope":scope,"base_revision":3}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let actual_trust = host.professional_trust.clone();
        let selected = host
            .select_directory(
                &ObjectId::try_from("packaging-test-ui".to_owned()).unwrap(),
                FilePurpose::SaveProject,
                saved.clone(),
            )
            .unwrap();
        let response = packaging_call(
            &mut host,
            "save_project",
            json!({"revision":4,"selected_directory_ref":selected}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let archive = read_project_archive(&saved).unwrap();
        for (path, bytes) in &archive.files {
            let file = altered.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
        std::fs::write(
            altered.join("project.json"),
            encode_document(&archive.manifest),
        )
        .unwrap();
        reopen_test_project(&mut host, &saved);
        let local = ScenarioId::try_from("LOCAL".to_owned()).unwrap();
        let response = local_run(&mut host, &local);
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        assert!(matches!(
            packaging_complete(&mut host, &task).task.state,
            ComputationStatus::Completed
        ));

        let entry = archive
            .manifest
            .documents
            .iter()
            .find(|entry| {
                entry
                    .module_ref
                    .as_ref()
                    .is_some_and(|reference| reference.id.as_str() == "fixture.pack")
            })
            .unwrap();
        let mut package: Versioned<ModulePackage> =
            serde_json::from_slice(&archive.files[entry.file_ref.path.as_str()]).unwrap();
        let original_reference = (
            package.data.definition.id.clone(),
            String::from(package.data.definition.version.clone()),
        );
        package
            .data
            .definition
            .title
            .push_str("; changed after the actual binding");
        assert_eq!(
            (
                package.data.definition.id.clone(),
                String::from(package.data.definition.version.clone())
            ),
            original_reference
        );
        std::fs::write(
            altered.join(entry.file_ref.path.as_str()),
            encode_document(&package.data),
        )
        .unwrap();
        reopen_test_project(&mut host, &altered);
        let binding = host.project.as_ref().unwrap().bindings[0].clone();
        let params = AdoptArtifactParams {
            artifact_id: binding.artifact_ref.clone(),
            acceptance_ref: binding.acceptance_ref.clone(),
            consumer_ref: binding.consumer_ref.clone(),
            operation_ref: binding.operation_ref.clone(),
            requested_scope: binding.requested_scope.clone(),
            base_revision: host.project.as_ref().unwrap().revision.clone(),
        };
        let modules: Vec<_> = host.modules.values().cloned().collect();
        // Historical real trust is used only as a pure comparison: the input,
        // files, profile and consumer gates still pass. It is not restored to Host.
        assert!(
            prepare_artifact_binding(
                &host.current_draft(),
                &modules,
                &params,
                binding.id.clone(),
                &actual_trust,
                &host.providers
            )
            .is_ok()
        );
        let next_task = host.next_task;
        let response = local_run(&mut host, &local);
        assert_eq!(
            response["outcome"]["data"]["code"], "PermissionDenied",
            "{response}"
        );
        assert!(
            response["outcome"]["data"]["field_errors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field["pointer"] == "/artifact_id")
        );
        assert_eq!(host.next_task, next_task);
        assert!(host.drain_io_work().is_empty());
    }

    fn local_run(host: &mut ProjectHost, scenario: &ScenarioId) -> Value {
        let revision = host.project.as_ref().unwrap().revision.clone();
        let window = host.scenarios[scenario].window.clone();
        packaging_call(
            host,
            "run_scenario",
            json!({"scenario_id":scenario,"input_revision":revision,"observation_window":window,"intent":{"kind":"project_scenario"}}),
        )
    }

    #[test]
    fn selected_scenario_runs_without_erasing_other_diagnostics_or_old_frozen_inputs() {
        let mut host = local_production_host();
        let local = ScenarioId::try_from("LOCAL".to_owned()).unwrap();
        let first = local_run(&mut host, &local);
        assert_eq!(first["outcome"]["status"], "ok", "{first}");
        let first_task: TaskHandle =
            serde_json::from_value(first["outcome"]["data"].clone()).unwrap();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        let first_work = work.pop().unwrap();
        let IoInput::Run { input, .. } = &first_work.input else {
            panic!("actual Engine input")
        };
        let old_run_id = input.run_id.clone();
        let old_project_path = input.input_ref.frozen_project_file.path.as_str().to_owned();
        host.finish_io(first_work.run());
        assert!(matches!(
            packaging_task(&mut host, &first_task).task.state,
            ComputationStatus::Completed
        ));
        let old_run = serde_json::to_value(&host.runs[&old_run_id].result).unwrap();
        assert_eq!(old_run["acceptance_events"].as_array().unwrap().len(), 1);
        let old_project_bytes = host
            .additions
            .iter()
            .find(|addition| addition.entry.file_ref.path.as_str() == old_project_path)
            .unwrap()
            .bytes
            .clone();

        let mut other = host.scenarios[&local].clone();
        other.id = ScenarioId::try_from("OTHER".to_owned()).unwrap();
        other.revision = Revision(SafeUint::try_from(1).unwrap());
        other.orders[0].quantity = None;
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"scenario","data":other}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let other_problem = |issue: &Issue| {
            issue.code == "business_inputs.missing_order_quantity"
                && issue.object_ref.as_ref().is_some_and(|target| {
                    matches!(target.kind, ObjectKind::Scenario) && target.id.as_str() == "OTHER"
                })
        };
        assert!(
            host.project
                .as_ref()
                .unwrap()
                .issues
                .iter()
                .any(other_problem)
        );

        let context = packaging_call(&mut host, "get_context", json!({"selection":[]}));
        assert_eq!(context["outcome"]["status"], "ok", "{context}");
        assert!(
            context["outcome"]["data"]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |issue| issue["code"] == "business_inputs.missing_order_quantity"
                        && issue["object_ref"]["id"] == "OTHER"
                )
        );

        let second = local_run(&mut host, &local);
        assert_eq!(second["outcome"]["status"], "ok", "{second}");
        let second_task: TaskHandle =
            serde_json::from_value(second["outcome"]["data"].clone()).unwrap();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        let second_work = work.pop().unwrap();
        let IoInput::Run {
            input,
            supporting_scenarios,
        } = &second_work.input
        else {
            panic!("actual Engine input")
        };
        let second_run_id = input.run_id.clone();
        assert!(!input.project.issues.iter().any(other_problem));
        assert_eq!(
            input.project.scenario_refs.len(),
            2,
            "diagnostic projection does not remove declared inputs"
        );
        assert_eq!(supporting_scenarios.len(), 1);
        assert_eq!(supporting_scenarios[0].id, other.id);
        assert!(supporting_scenarios[0].orders[0].quantity.is_none());
        host.finish_io(second_work.run());
        assert!(matches!(
            packaging_task(&mut host, &second_task).task.state,
            ComputationStatus::Completed
        ));
        assert_eq!(host.runs[&second_run_id].result.acceptance_events.len(), 1);
        assert_eq!(
            host.runs[&second_run_id].result.acceptance_events[0]
                .at_ms
                .0
                .value(),
            75000
        );
        assert!(
            host.project
                .as_ref()
                .unwrap()
                .issues
                .iter()
                .any(other_problem)
        );
        let next_task = host.next_task;
        let rejected = local_run(&mut host, &other.id);
        assert_eq!(rejected["outcome"]["status"], "error", "{rejected}");
        assert!(
            rejected["outcome"]["data"]["field_errors"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field["pointer"] == "/orders/0/quantity"
                    && field["reason"]
                        .as_str()
                        .unwrap()
                        .contains("business_inputs.missing_order_quantity"))
        );
        assert_eq!(host.next_task, next_task);
        assert!(host.drain_io_work().is_empty());

        // A shared route defect remains an execution blocker for LOCAL.
        let mut shared_route = host.project.as_ref().unwrap().routes[0].clone();
        shared_route.steps[0].device_id =
            ObjectId::try_from("missing-shared-device".to_owned()).unwrap();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"put_object","data":{"kind":"route","data":shared_route}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let rejected = local_run(&mut host, &local);
        assert_eq!(rejected["outcome"]["status"], "error", "{rejected}");
        assert!(host.drain_io_work().is_empty());
        assert!(
            host.project
                .as_ref()
                .unwrap()
                .issues
                .iter()
                .any(other_problem)
        );
        assert_eq!(
            serde_json::to_value(&host.runs[&old_run_id].result).unwrap(),
            old_run
        );
        assert_eq!(
            host.additions
                .iter()
                .find(|addition| addition.entry.file_ref.path.as_str() == old_project_path)
                .unwrap()
                .bytes,
            old_project_bytes
        );
    }

    #[test]
    fn actual_packaging_freezes_budgetless_saved_job_and_reuses_the_exact_canonical_input() {
        let mut host = packaging_host();
        let mut original = packaging_case("COVERAGE-restricted_folded_container");
        let budget = original.budget.take().unwrap();
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_packaging_job","data":original}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task = packaging_admit(&mut host, &original, &budget);
        assert!(matches!(
            packaging_task(&mut host, &task).task.state,
            ComputationStatus::Queued
        ));
        assert!(host.professional.artifacts.is_empty());
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        assert!(matches!(
            state.originating_operation,
            OperationName::ComputePackaging
        ));
        assert!(state.task.progress.is_some());
        let TaskOutput::Packaging(first) = state.result.unwrap() else {
            panic!("actual packaging output")
        };
        assert_eq!(first.job_ref.revision.0.value(), 2);
        assert!(exact_data(
            &host.professional.jobs[&(original.id.clone(), 1)],
            &original
        ));
        let frozen = host.professional.jobs[&(original.id.clone(), 2)].clone();
        assert!(exact_data(frozen.budget.as_ref().unwrap(), &budget));
        assert!(host.project.as_ref().unwrap().acknowledgments.is_empty());
        assert!(host.project.as_ref().unwrap().bindings.is_empty());
        let file = host.professional.artifacts[&first.artifact_id]
            .input_ref
            .clone();
        let bytes = host.professional.files[file.path.as_str()].clone();
        let second_task = packaging_admit(&mut host, &frozen, &budget);
        let state = packaging_complete(&mut host, &second_task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let TaskOutput::Packaging(second) = state.result.unwrap() else {
            panic!("actual packaging output")
        };
        assert_ne!(first.artifact_id, second.artifact_id);
        assert!(exact_data(
            &file,
            &host.professional.artifacts[&second.artifact_id].input_ref
        ));
        assert_eq!(host.professional.files[file.path.as_str()], bytes);
        assert_eq!(host.professional.jobs.len(), 2);
        assert_eq!(host.providers.len(), 1);
        let cancelled = packaging_call(
            &mut host,
            "cancel_task",
            json!({"task_id":second_task.id,"action":"cancel"}),
        );
        assert_eq!(cancelled["outcome"]["data"]["terminal_won"], true);
        assert!(matches!(
            packaging_task(&mut host, &second_task).task.state,
            ComputationStatus::Completed
        ));
    }

    #[test]
    fn queued_packaging_cancel_does_not_publish_a_job_artifact_or_provider() {
        let mut host = packaging_host();
        let job = packaging_case("HOLD01");
        let task = packaging_admit(&mut host, &job, job.budget.as_ref().unwrap());
        let response = packaging_call(
            &mut host,
            "cancel_task",
            json!({"task_id":task.id,"action":"cancel"}),
        );
        assert_eq!(response["outcome"]["data"]["action_accepted"], true);
        let state = packaging_complete(&mut host, &task);
        assert!(matches!(state.task.state, ComputationStatus::Cancelled));
        assert!(state.result.is_none());
        assert!(host.professional.jobs.is_empty());
        assert!(host.professional.artifacts.is_empty());
        assert!(host.providers.is_empty());
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 1);
    }

    #[test]
    fn real_budget_exhaustion_is_a_completed_undetermined_result_without_nominal_adoption() {
        let mut host = packaging_host();
        let job = packaging_case("LOCAL_ZERO_QUERY");
        let task = packaging_admit(&mut host, &job, job.budget.as_ref().unwrap());
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let TaskOutput::Packaging(result) = state.result.unwrap() else {
            panic!("actual packaging output")
        };
        assert_eq!(
            serde_json::to_value(&result).unwrap()["outcome"],
            "undetermined"
        );
        assert!(host.project.as_ref().unwrap().acknowledgments.is_empty());
        assert!(host.project.as_ref().unwrap().bindings.is_empty());
    }

    #[test]
    fn searched_execution_reserves_a_new_revision_until_actual_candidate_publication() {
        let mut host = packaging_host();
        let original = packaging_case("SEARCH01");
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_packaging_job","data":original}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let task = packaging_admit(&mut host, &original, original.budget.as_ref().unwrap());
        let mut premature = original.clone();
        premature.revision = Revision(SafeUint::try_from(original.revision.0.value() + 1).unwrap());
        let response = packaging_call(
            &mut host,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"put_packaging_job","data":premature}]}),
        );
        assert_eq!(
            response["outcome"]["data"]["code"], "InvalidInput",
            "{response}"
        );
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 2);
        assert_eq!(host.professional.jobs.len(), 1);
        let state = packaging_complete(&mut host, &task);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        let TaskOutput::Packaging(result) = state.result.unwrap() else {
            panic!("actual packaging output")
        };
        assert_eq!(
            result.job_ref.revision.0.value(),
            premature.revision.0.value()
        );
        assert!(exact_data(
            &host.professional.jobs[&(original.id.clone(), original.revision.0.value())],
            &original
        ));
    }

    #[test]
    fn object_container_replacement_cannot_borrow_a_descendant_leaf_grant() {
        for new in [Value::Null, json!("replacement")] {
            let failure = changed_leaves(
                &json!({"spec_ref":{"id":"product","revision":"r1"}}),
                &json!({"spec_ref":new}),
            )
            .unwrap_err();
            assert!(matches!(failure.code, ErrorCode::UnsupportedCapability));
        }
        let failure = changed_leaves(
            &json!({"attributes":{"detail":{"kind":"string","value":"x"}}}),
            &json!({"attributes":{}}),
        )
        .unwrap_err();
        assert!(matches!(failure.code, ErrorCode::UnsupportedCapability));
    }

    #[test]
    fn leaf_authority_uses_exact_rfc6901_names_not_parent_or_glob_patterns() {
        let paths = changed_leaves(
            &json!({"attributes":{"a/b~c":{"kind":"string","value":"old"}}}),
            &json!({"attributes":{"a/b~c":{"kind":"string","value":"new"}}}),
        )
        .unwrap();
        assert_eq!(paths, vec!["/attributes/a~1b~0c/value"]);
        let grant = |pointer: &str| HostFieldGrant {
            pointer: pointer.into(),
            element_id: None,
        };
        assert!(leaf_granted(&grant(&paths[0]), &paths[0]));
        for pointer in [
            "",
            "/attributes",
            "/attributes/*/value",
            "/attributes/a~2b/value",
        ] {
            assert!(!leaf_granted(&grant(pointer), &paths[0]));
        }
    }

    #[test]
    fn whole_footprint_grant_allows_atomic_geometry_edit_but_child_grant_cannot() {
        for pointer in ["/footprint_m", "/footprint_m/0/x"] {
            let mut host = local_production_host();
            let before = host.current_draft();
            let mut device = serde_json::to_value(&before.project.devices[0]).unwrap();
            let target: ObjectRef = serde_json::from_value(json!({"kind":"device","id":device["id"]})).unwrap();
            let mut caller = task_authority_caller(&host, "footprint-agent");
            caller.allowed_operations = vec![OperationName::ApplyCommands, OperationName::DescribeCapabilities];
            caller.edit_grants = vec![HostEditGrant {
                id: ObjectId::try_from("footprint-grant".to_owned()).unwrap(),
                scope: TaskScope::Edit(EditScope::Project(before.project.id.clone())),
                branches: vec![HostEditBranch::PutObject, HostEditBranch::SetProjectSettings],
                objects: vec![
                    HostObjectEditGrant { target, fields: vec![HostFieldGrant { pointer:pointer.into(), element_id:None }], can_remove:false },
                    HostObjectEditGrant { target:serde_json::from_value(json!({"kind":"project","id":before.project.id})).unwrap(), fields:vec![HostFieldGrant {pointer:"/title".into(),element_id:None}],can_remove:false },
                ],
                creates:vec![],source_versions:vec![],reusable_resources:vec![],created_objects:vec![],
            }];
            let session = caller.session_id.clone();
            host.register_external_session(caller).unwrap();
            let capability = authority_call(&mut host, &session, ApiScope::Workspace, "describe_capabilities", json!({}));
            assert_eq!(capability["outcome"]["status"], "ok", "{capability}");
            let advertised = capability["outcome"]["data"]["available_command_kinds"].as_array().unwrap();
            assert_eq!(advertised.contains(&json!("put_object")), pointer == "/footprint_m");
            let points = device["footprint_m"].as_array_mut().unwrap();
            let midpoint = json!({"x":(points[0]["x"].as_f64().unwrap()+points[1]["x"].as_f64().unwrap())/2.0,
                "y":(points[0]["y"].as_f64().unwrap()+points[1]["y"].as_f64().unwrap())/2.0});
            points.insert(1,midpoint);
            let mut settings = settings_value(&before.project);
            settings["title"] = json!("atomic geometry batch");
            let response = authority_call(&mut host, &session, ApiScope::Project, "apply_commands", json!({"base_revision":before.project.revision,
                "commands":[{"kind":"set_project_settings","data":settings},{"kind":"put_object","data":{"kind":"device","data":device}}]}));
            if pointer == "/footprint_m" {
                assert_eq!(response["outcome"]["status"],"ok","{response}");
                assert_eq!(host.project.as_ref().unwrap().devices[0].footprint_m.len(),5);
                assert_eq!(host.project.as_ref().unwrap().title,"atomic geometry batch");
                assert_eq!(host.project.as_ref().unwrap().revision.0.value(),before.project.revision.0.value()+1);
            } else {
                assert_eq!(response["outcome"]["data"]["code"],"PermissionDenied","{response}");
                assert!(exact_data(&host.current_draft().project,&before.project));
            }
        }
        assert!(changed_fields(&json!({"devices":[{"id":"A"}]}),&json!({"devices":[{"id":"A"},{"id":"B"}]})).is_err());
        assert!(changed_fields(&json!({"nested":{"footprint_m":[1,2]}}),&json!({"nested":{"footprint_m":[1,2,3]}})).is_err());
    }

    #[test]
    fn capabilities_only_advertise_external_edits_with_a_usable_authorized_path() {
        fn dispatch(
            host: &mut ProjectHost,
            session: &ObjectId,
            operation: &str,
            params: Value,
        ) -> Value {
            host.dispatch(session, serde_json::from_value(json!({"api_version":"0.1","request_id":"capability-test","project_id":null,"scope":{"kind":"workspace"},"operation":operation,"params":params})).unwrap())
        }
        let id = |value: &str| ObjectId::try_from(value.to_owned()).unwrap();
        let ui = id("ui");
        let mut host = ProjectHost::new(id("host"), ui.clone());
        let created = dispatch(
            &mut host,
            &ui,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"capabilities","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}),
        );
        let project = ProjectId::try_from(
            created["outcome"]["data"]["project_id"]
                .as_str()
                .unwrap()
                .to_owned(),
        )
        .unwrap();
        // An exhausted distinct-ID budget is unavailable only while all its
        // recorded identities still exist; deleted identities remain restorable.
        let seeded = host.dispatch(&ui, serde_json::from_value(json!({"api_version":"0.1","request_id":"seed-spent-id","project_id":project,"scope":{"kind":"project"},"operation":"apply_commands","params":{"base_revision":1,"commands":[{"kind":"put_object","data":{"kind":"item_type","data":{"id":"already-created","revision":"r1","title":"existing","unit":"piece","tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]}}}]}})).unwrap());
        assert_eq!(seeded["outcome"]["status"], "ok");
        let field = |pointer: &str| HostFieldGrant {
            pointer: pointer.into(),
            element_id: None,
        };
        let grant = |branch| HostEditGrant {
            id: id("grant"),
            scope: TaskScope::Edit(EditScope::Project(project.clone())),
            branches: vec![branch],
            objects: vec![],
            creates: vec![],
            source_versions: vec![],
            reusable_resources: vec![],
            created_objects: vec![],
        };
        let mut title = grant(HostEditBranch::SetProjectSettings);
        title.objects.push(HostObjectEditGrant {
            target: ObjectRef {
                kind: ObjectKind::Project,
                id: id(project.as_str()),
            },
            fields: vec![field("/title")],
            can_remove: false,
        });
        let mut create = grant(HostEditBranch::PutObject);
        create.creates.push(HostCreateAllowance {
            kind: ObjectKind::ItemType,
            max_new_objects: SafeUint::try_from(1).unwrap(),
            fields: ["/revision", "/title", "/unit", "/tracking", "/spec_ref"]
                .map(field)
                .to_vec(),
        });
        let mut spent = create.clone();
        spent.created_objects.push(ObjectRef {
            kind: ObjectKind::ItemType,
            id: id("already-created"),
        });
        let mut parent = title.clone();
        parent.objects[0].fields = vec![field("/time_reference")];
        for (session_name, grants, expected) in [
            ("empty-put", vec![grant(HostEditBranch::PutObject)], vec![]),
            ("parent-field", vec![parent], vec![]),
            ("spent-create", vec![spent], vec![]),
            ("title", vec![title], vec!["set_project_settings"]),
            ("create", vec![create], vec!["put_object"]),
        ] {
            let session = id(session_name);
            host.register_external_session(HostCallContext {
                origin: CallerOrigin::ExternalAgent,
                allow_view_control: false,
                session_id: session.clone(),
                granted_project: Some(project.clone()),
                allowed_operations: vec![
                    OperationName::DescribeCapabilities,
                    OperationName::ApplyCommands,
                ],
                allowed_definition_drafts: vec![],
                selected_file_tokens: vec![],
                edit_grants: grants,
            })
            .unwrap();
            let response = dispatch(&mut host, &session, "describe_capabilities", json!({}));
            assert_eq!(response["outcome"]["status"], "ok");
            let catalog = &response["outcome"]["data"];
            assert_eq!(catalog["available_command_kinds"], json!(expected));
            let has_apply = catalog["implemented_operations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|op| op["operation"] == "apply_commands");
            assert_eq!(has_apply, !expected.is_empty());
        }
    }

    #[test]
    fn deleted_created_identity_remains_advertised_and_restorable_without_budget_refund() {
        fn call(
            host: &mut ProjectHost,
            session: &ObjectId,
            project: Option<&ProjectId>,
            operation: &str,
            params: Value,
        ) -> Value {
            host.dispatch(session, serde_json::from_value(json!({"api_version":"0.1","request_id":"restore-test","project_id":project,"scope":{"kind":if project.is_some(){"project"}else{"workspace"}},"operation":operation,"params":params})).unwrap())
        }
        let id = |value: &str| ObjectId::try_from(value.to_owned()).unwrap();
        let ui = id("ui");
        let external = id("external");
        let mut host = ProjectHost::new(id("host"), ui.clone());
        let created = call(
            &mut host,
            &ui,
            None,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"restore","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}),
        );
        let project = ProjectId::try_from(
            created["outcome"]["data"]["project_id"]
                .as_str()
                .unwrap()
                .to_owned(),
        )
        .unwrap();
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some(project.clone()),
            allowed_operations: vec![
                OperationName::DescribeCapabilities,
                OperationName::ApplyCommands,
            ],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![HostEditGrant {
                id: id("grant"),
                scope: TaskScope::Edit(EditScope::Project(project.clone())),
                branches: vec![HostEditBranch::PutObject, HostEditBranch::RemoveObject],
                objects: vec![HostObjectEditGrant {
                    target: ObjectRef {
                        kind: ObjectKind::ItemType,
                        id: id("A"),
                    },
                    fields: vec![],
                    can_remove: true,
                }],
                creates: vec![HostCreateAllowance {
                    kind: ObjectKind::ItemType,
                    max_new_objects: SafeUint::try_from(1).unwrap(),
                    fields: ["/revision", "/title", "/unit", "/tracking", "/spec_ref"]
                        .map(|pointer| HostFieldGrant {
                            pointer: pointer.into(),
                            element_id: None,
                        })
                        .to_vec(),
                }],
                source_versions: vec![],
                reusable_resources: vec![],
                created_objects: vec![],
            }],
        })
        .unwrap();
        let put = |name: &str| json!({"kind":"put_object","data":{"kind":"item_type","data":{"id":name,"revision":"r1","title":name,"unit":"piece","tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]}}});
        let first = call(
            &mut host,
            &external,
            Some(&project),
            "apply_commands",
            json!({"base_revision":1,"commands":[put("A")]}),
        );
        assert_eq!(first["outcome"]["status"], "ok");
        let removed = call(
            &mut host,
            &external,
            Some(&project),
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"remove_object","data":{"kind":"item_type","id":"A"}}]}),
        );
        assert_eq!(removed["outcome"]["status"], "ok");
        let catalog = call(
            &mut host,
            &external,
            None,
            "describe_capabilities",
            json!({}),
        );
        assert_eq!(
            catalog["outcome"]["data"]["available_command_kinds"],
            json!(["put_object"])
        );
        assert!(
            catalog["outcome"]["data"]["implemented_operations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|op| op["operation"] == "apply_commands")
        );
        let restored = call(
            &mut host,
            &external,
            Some(&project),
            "apply_commands",
            json!({"base_revision":3,"commands":[put("A")]}),
        );
        assert_eq!(restored["outcome"]["status"], "ok");
        let denied = call(
            &mut host,
            &external,
            Some(&project),
            "apply_commands",
            json!({"base_revision":4,"commands":[put("B")]}),
        );
        assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
        let ledger = &host.caller_context(&external).unwrap().edit_grants[0].created_objects;
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger[0].id.as_str(), "A");
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 4);
        assert_eq!(
            host.project.as_ref().unwrap().item_types[0].id.as_str(),
            "A"
        );
    }
}
