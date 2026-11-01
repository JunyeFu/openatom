//! Tauri boundary for the shared, single-project Host. Caller identity is Host-owned.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sf_commands::{
    ApiError, ApiFieldError, ApiOutcome, ApiRequest, ApiScope, ApiVersion, ErrorCode, FilePurpose,
    HostCallContext, ObjectSource, OperationName, ProjectHost, ProjectIoWork, Response,
    SelectedFileRef,
};
use sf_domain::{
    ArtifactId, ArtifactRecord, AssetRef, FileRef, ObjectId, PackageStructureJob, PackagingResult,
    ProjectId, Revision,
};
use tauri_plugin_dialog::DialogExt;

pub(super) mod manufacturing_export;
pub(super) mod native_capture_probe;
pub(super) mod port_output;
pub(super) mod run_observation;
pub(super) mod public_render_smoke;

#[derive(Clone)]
pub struct ProjectApiState {
    host: Arc<Mutex<ProjectHost>>,
    ui_session_id: ObjectId,
    io_sender: mpsc::Sender<ProjectIoWork>,
    io_completed: Arc<(Mutex<()>, Condvar)>,
    pub(super) host_instance_id: ObjectId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaAssetReadRequest {
    project_id: ProjectId,
    scope: ApiScope,
    scope_revision: Revision,
    source: ObjectSource,
    asset: AssetRef,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredPackagingReadRequest {
    project_id: ProjectId,
    scope: ApiScope,
    scope_revision: Revision,
    artifact_id: ArtifactId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredPackagingScopeRequest {
    project_id: ProjectId,
    scope: ApiScope,
    scope_revision: Revision,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredManufacturingReadRequest {
    project_id: ProjectId,
    scope: ApiScope,
    scope_revision: Revision,
    artifact_id: ArtifactId,
    file: FileRef,
}

#[derive(Serialize)]
pub struct StoredPackagingEvidence {
    artifact: ArtifactRecord,
    result: PackagingResult,
    input: PackageStructureJob,
}

impl ProjectApiState {
    fn desktop_close_state(&self, window_label: &str) -> Result<sf_commands::DesktopCloseState, ApiError> {
        if window_label != "main" {
            return Err(host_error(ErrorCode::PermissionDenied, "Only the main desktop window may inspect close state"));
        }
        self.host.lock()
            .map(|host| host.desktop_close_state())
            .map_err(|_| host_error(ErrorCode::ComputationFailed, "Managed Project Host state is poisoned"))
    }

    fn finish_desktop_close(
        &self,
        window_label: &str,
        expected: &sf_commands::DesktopCloseState,
        discard_confirmed: bool,
        destroy: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), ApiError> {
        if window_label != "main" {
            return Err(host_error(ErrorCode::PermissionDenied, "Only the main desktop window may close the project"));
        }
        let mut host = self.host.lock()
            .map_err(|_| host_error(ErrorCode::ComputationFailed, "Managed Project Host state is poisoned"))?;
        host.finish_desktop_close(expected, discard_confirmed, destroy)
    }

    fn read_stored_manufacturing_ui(
        &self,
        window_label: &str,
        request: &StoredManufacturingReadRequest,
    ) -> Result<Vec<u8>, ApiError> {
        if window_label != "main" {
            return Err(host_error(
                ErrorCode::PermissionDenied,
                "Only the main desktop window may read a stored manufacturing file",
            ));
        }
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Managed Project Host state is poisoned",
                )
            })?
            .read_stored_manufacturing_file(
                &self.ui_session_id,
                &request.project_id,
                &request.scope,
                &request.scope_revision,
                &request.artifact_id,
                &request.file,
            )
    }

    pub(super) fn set_render_backend(
        &self,
        backend: Arc<dyn sf_commands::RenderBackend>,
    ) -> Result<(), ApiError> {
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Project Host state is poisoned",
                )
            })?
            .set_render_backend(backend);
        Ok(())
    }

    pub(super) fn rendered_task_image(
        &self,
        session: &ObjectId,
        request: &Value,
        reply: &Value,
    ) -> Result<
        Option<(
            sf_commands::TaskState,
            sf_commands::RenderOutput,
            sf_commands::RenderedImage,
        )>,
        ApiError,
    > {
        if request["operation"] != "get_task"
            || reply["outcome"]["status"] != "ok"
            || reply["outcome"]["data"]["result"]["kind"] != "render"
        {
            return Ok(None);
        }
        let task: sf_commands::TaskState = serde_json::from_value(reply["outcome"]["data"].clone())
            .map_err(|e| host_error(ErrorCode::ComputationFailed, &e.to_string()))?;
        let Some(sf_commands::TaskOutput::Render(output)) = &task.result else {
            return Ok(None);
        };
        if !matches!(task.task.state, sf_domain::ComputationStatus::Completed)
            || !matches!(task.originating_operation, OperationName::RenderView)
        {
            return Ok(None);
        }
        let project_id: ProjectId = serde_json::from_value(request["project_id"].clone())
            .map_err(|e| host_error(ErrorCode::InvalidInput, &e.to_string()))?;
        let scope: ApiScope = serde_json::from_value(request["scope"].clone())
            .map_err(|e| host_error(ErrorCode::InvalidInput, &e.to_string()))?;
        let image = self
            .host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Project Host state is poisoned",
                )
            })?
            .read_rendered_image(
                session,
                &project_id,
                &scope,
                &task.task.id,
                &output.scene.file.id,
            )?;
        let output = output.clone();
        Ok(Some((task, output, image)))
    }

    fn list_stored_packaging_ui(
        &self,
        window_label: &str,
        request: &StoredPackagingScopeRequest,
    ) -> Result<Vec<ArtifactId>, ApiError> {
        if window_label != "main" {
            return Err(host_error(
                ErrorCode::PermissionDenied,
                "Only the main desktop window may list saved packaging artifacts",
            ));
        }
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Managed Project Host state is poisoned",
                )
            })?
            .list_stored_packaging_artifacts(
                &self.ui_session_id,
                &request.project_id,
                &request.scope,
                &request.scope_revision,
            )
    }

    fn read_stored_packaging_ui(
        &self,
        window_label: &str,
        request: &StoredPackagingReadRequest,
    ) -> Result<StoredPackagingEvidence, ApiError> {
        if window_label != "main" {
            return Err(host_error(
                ErrorCode::PermissionDenied,
                "Only the main desktop window may read a saved packaging artifact",
            ));
        }
        let (artifact, result, input) = self
            .host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Managed Project Host state is poisoned",
                )
            })?
            .read_stored_packaging_artifact(
                &self.ui_session_id,
                &request.project_id,
                &request.scope,
                &request.scope_revision,
                &request.artifact_id,
            )?;
        Ok(StoredPackagingEvidence {
            artifact,
            result,
            input,
        })
    }

    fn read_media_asset_ui(
        &self,
        window_label: &str,
        request: &MediaAssetReadRequest,
    ) -> Result<Vec<u8>, ApiError> {
        if window_label != "main" {
            return Err(host_error(
                ErrorCode::PermissionDenied,
                "Only the main desktop window may read a declared media asset",
            ));
        }
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Managed Project Host state is poisoned",
                )
            })?
            .read_media_asset(
                &self.ui_session_id,
                &request.project_id,
                &request.scope,
                &request.scope_revision,
                &request.source,
                &request.asset,
            )
    }
}

#[tauri::command]
pub fn project_read_stored_manufacturing_file(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: StoredManufacturingReadRequest,
) -> Result<tauri::ipc::Response, ApiError> {
    state
        .read_stored_manufacturing_ui(window.label(), &request)
        .map(tauri::ipc::Response::new)
}

#[tauri::command]
pub fn project_read_stored_packaging(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: StoredPackagingReadRequest,
) -> Result<StoredPackagingEvidence, ApiError> {
    state.read_stored_packaging_ui(window.label(), &request)
}

#[tauri::command]
pub fn project_list_stored_packaging(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: StoredPackagingScopeRequest,
) -> Result<Vec<ArtifactId>, ApiError> {
    state.list_stored_packaging_ui(window.label(), &request)
}

#[tauri::command]
pub fn project_read_media_asset(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: MediaAssetReadRequest,
) -> Result<tauri::ipc::Response, ApiError> {
    state
        .read_media_asset_ui(window.label(), &request)
        .map(tauri::ipc::Response::new)
}

impl ProjectApiState {
    pub fn register_evaluation_resources(&self, root: &Path) -> Result<(), String> {
        let cjk_font = std::fs::read(root.join("SourceHanSansCN-Regular.otf"))
            .map_err(|error| format!("Evaluation CJK font: {error}"))?;
        let font_license = std::fs::read(root.join("LICENSE.SourceHanSans.txt"))
            .map_err(|error| format!("Evaluation font license: {error}"))?;
        self.host
            .lock()
            .map_err(|_| "Managed Project Host state is poisoned".to_owned())?
            .set_evaluation_resources(sf_commands::EvaluationResources {
                cjk_font: Some(cjk_font),
                font_license: Some(font_license),
                files: Default::default(),
            });
        Ok(())
    }

    pub fn register_bundled_library(&self, root: &Path) -> Result<(), String> {
        let mut host = self
            .host
            .lock()
            .map_err(|_| "Managed Project Host state is poisoned".to_owned())?;
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
        Ok(())
    }

    pub fn new() -> Self {
        // These trusted identifiers cannot be supplied or replaced by the request.
        let host_id = ObjectId::try_from(format!("desktop-host-{}", std::process::id()))
            .expect("generated Host identifier is nonempty and contains no NUL");
        let ui_session_id = ObjectId::try_from("desktop-main-ui".to_owned())
            .expect("literal UI identifier is nonempty and contains no NUL");
        let host = Arc::new(Mutex::new(ProjectHost::new(
            host_id.clone(),
            ui_session_id.clone(),
        )));
        let io_completed = Arc::new((Mutex::new(()), Condvar::new()));
        let (io_sender, io_receiver) = mpsc::channel::<ProjectIoWork>();
        let worker_host = Arc::clone(&host);
        let worker_completed = Arc::clone(&io_completed);
        std::thread::Builder::new()
            .name("superfactory-project-io".to_owned())
            .spawn(move || {
                for work in io_receiver {
                    // Disk I/O never holds the Host mutex. One receiver preserves FIFO.
                    let completion = work.run();
                    let finished = match worker_host.lock() {
                        Ok(mut host) => {
                            host.finish_io(completion);
                            true
                        }
                        Err(_) => false,
                    };
                    // Drop the Host guard before taking the completion gate.
                    signal_io_completion(&worker_completed);
                    if !finished {
                        eprintln!("Project I/O completion rejected: Host state is poisoned");
                        return;
                    }
                }
            })
            .expect("Project I/O worker must start before the desktop Host is exposed");
        Self {
            host,
            ui_session_id,
            io_sender,
            io_completed,
            host_instance_id: host_id,
        }
    }

    pub(super) fn dispatch_ui(&self, window_label: &str, request: Value) -> Value {
        // Correlate a boundary error only when the supplied ID itself is valid.
        let request_id = request
            .get("request_id")
            .and_then(|value| serde_json::from_value::<ObjectId>(value.clone()).ok());
        if window_label != "main" {
            return boundary_error(
                request_id,
                ErrorCode::PermissionDenied,
                "Only the main desktop window may use the interactive UI session".to_owned(),
                vec![],
            );
        }
        // Negotiate string headers before decoding the operation-specific body.
        // Other malformed inputs still go through the strict frozen ApiRequest.
        if let (Some(version @ Value::String(_)), Some(operation @ Value::String(_))) =
            (request.get("api_version"), request.get("operation"))
        {
            let header_error = if serde_json::from_value::<ApiVersion>(version.clone()).is_err() {
                Some((
                    ErrorCode::UnsupportedSchema,
                    "/api_version",
                    "Unsupported API version",
                ))
            } else if serde_json::from_value::<OperationName>(operation.clone()).is_err() {
                Some((
                    ErrorCode::UnsupportedCapability,
                    "/operation",
                    "Unknown operation",
                ))
            } else {
                None
            };
            if let Some((code, pointer, message)) = header_error {
                return boundary_error(
                    request_id,
                    code,
                    message.to_owned(),
                    vec![ApiFieldError {
                        pointer: pointer.to_owned(),
                        reason: message.to_owned(),
                    }],
                );
            }
        }
        let request = match serde_json::from_value::<ApiRequest>(request) {
            Ok(request) => request,
            Err(error) => {
                return boundary_error(
                    request_id,
                    ErrorCode::InvalidInput,
                    "Request does not match the frozen ApiRequest contract".to_owned(),
                    vec![ApiFieldError {
                        pointer: "".to_owned(),
                        reason: error.to_string(),
                    }],
                );
            }
        };
        self.dispatch_session(&self.ui_session_id, request)
    }

    pub(super) fn register_external(&self, caller: HostCallContext) -> Result<(), ApiError> {
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Project Host state is poisoned",
                )
            })?
            .register_external_session(caller)
    }

    pub(super) fn update_external_authority(
        &self,
        mut next: HostCallContext,
    ) -> Result<(), ApiError> {
        let mut host = self.host.lock().map_err(|_| {
            host_error(
                ErrorCode::ComputationFailed,
                "Project Host state is poisoned",
            )
        })?;
        let current = host.caller_context(&next.session_id).ok_or_else(|| {
            host_error(
                ErrorCode::PermissionDenied,
                "External session is no longer registered",
            )
        })?;
        // Only existing selections of this exact native session may survive an
        // update. A different project grant never restores file authority.
        if next.granted_project == current.granted_project {
            next.selected_file_tokens = current.selected_file_tokens.clone();
        }
        host.update_external_session_authority(&self.ui_session_id, next)
    }

    pub(super) fn revoke_external(&self, session: &ObjectId) -> Result<(), ApiError> {
        let mut host = self.host.lock().map_err(|_| {
            host_error(
                ErrorCode::ComputationFailed,
                "Project Host state is poisoned",
            )
        })?;
        // A project switch already removes and tombstones its external sessions.
        // Native EOF cleanup is also called after that established transition.
        if host.caller_context(session).is_none() {
            return Ok(());
        }
        host.revoke_external_session(session)
    }

    pub(super) fn grant_external_task(
        &self,
        session: &ObjectId,
        task: &sf_domain::TaskId,
        allow_controls: bool,
    ) -> Result<(), ApiError> {
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Project Host state is poisoned",
                )
            })?
            .grant_external_task_access(&self.ui_session_id, session, task, allow_controls)
    }

    pub(super) fn select_external_directory(
        &self,
        session: &ObjectId,
        purpose: FilePurpose,
        path: PathBuf,
    ) -> Result<SelectedFileRef, ApiError> {
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Project Host state is poisoned",
                )
            })?
            .select_directory(session, purpose, path)
    }

    pub(super) fn dispatch_session(&self, session: &ObjectId, request: ApiRequest) -> Value {
        let request_id = serde_json::to_value(&request)
            .ok()
            .and_then(|v| serde_json::from_value(v["request_id"].clone()).ok());
        let mut host = match self.host.lock() {
            Ok(host) => host,
            Err(_) => {
                return boundary_error(
                    request_id,
                    ErrorCode::ComputationFailed,
                    "Project Host state is poisoned; request was not dispatched".to_owned(),
                    vec![],
                );
            }
        };
        let response = host.dispatch(session, request);
        let mut queue_failed = false;
        // Enqueue while holding the admission lock so concurrent callers cannot
        // reverse the order of tasks accepted by this Host.
        for work in host.drain_io_work() {
            if let Err(error) = self.io_sender.send(work) {
                host.finish_io(error.0.fail(host_error(
                    ErrorCode::ComputationFailed,
                    "Project I/O worker is no longer available",
                )));
                queue_failed = true;
            }
        }
        drop(host);
        if queue_failed {
            signal_io_completion(&self.io_completed);
        }
        response
    }

    // Private trusted Host entry: machine paths are never deserialized from an API request.
    fn register_directory(
        &self,
        purpose: FilePurpose,
        selected: Option<PathBuf>,
    ) -> Result<SelectedFileRef, ApiError> {
        let directory = selected
            .ok_or_else(|| host_error(ErrorCode::Cancelled, "Directory selection cancelled"))?;
        let mut host = self.host.lock().map_err(|_| {
            host_error(
                ErrorCode::ComputationFailed,
                "Project Host state is poisoned",
            )
        })?;
        host.select_directory(&self.ui_session_id, purpose, directory)
    }

    fn register_file(
        &self,
        purpose: FilePurpose,
        selected: Option<PathBuf>,
    ) -> Result<SelectedFileRef, ApiError> {
        let file =
            selected.ok_or_else(|| host_error(ErrorCode::Cancelled, "File selection cancelled"))?;
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Project Host state is poisoned",
                )
            })?
            .select_file(&self.ui_session_id, purpose, file)
    }

    fn wait_for_task(&self, project_id: Value, task_id: Value) -> Value {
        let deadline = Instant::now() + Duration::from_secs(15);
        let (gate, changed) = &*self.io_completed;
        let mut guard = match gate.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return boundary_error(
                    None,
                    ErrorCode::ComputationFailed,
                    "I/O completion gate is poisoned".to_owned(),
                    vec![],
                );
            }
        };
        loop {
            let response = self.dispatch_ui(
                "main",
                smoke_request(
                    "await-file-task",
                    project_id.clone(),
                    "get_task",
                    json!({ "task_id": task_id }),
                ),
            );
            if response["outcome"]["status"] != "ok"
                || matches!(
                    response["outcome"]["data"]["task"]["state"].as_str(),
                    Some("completed" | "failed" | "cancelled")
                )
            {
                return response;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return boundary_error(
                    None,
                    ErrorCode::ComputationFailed,
                    "File task did not reach a terminal state within the smoke deadline".to_owned(),
                    vec![],
                );
            };
            guard = match changed.wait_timeout(guard, remaining) {
                Ok((guard, _)) => guard,
                Err(_) => {
                    return boundary_error(
                        None,
                        ErrorCode::ComputationFailed,
                        "I/O completion gate is poisoned".to_owned(),
                        vec![],
                    );
                }
            };
        }
    }
}

fn host_error(code: ErrorCode, message: &str) -> ApiError {
    ApiError {
        code,
        message: message.to_owned(),
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}

fn signal_io_completion(signal: &(Mutex<()>, Condvar)) {
    match signal.0.lock() {
        Ok(_guard) => signal.1.notify_all(),
        Err(_) => eprintln!("Project I/O completion gate is poisoned"),
    }
}

fn directory_dialog_title(
    window_label: &str,
    purpose: &FilePurpose,
) -> Result<&'static str, ApiError> {
    if window_label != "main" {
        return Err(host_error(
            ErrorCode::PermissionDenied,
            "Only the main desktop window may select a project directory",
        ));
    }
    match purpose {
        FilePurpose::SaveProject => Ok("选择工程保存目录"),
        FilePurpose::ImportPackage => Ok("选择要打开的工程目录"),
        FilePurpose::ExportPackage => Ok("选择工程或贡献包导出目录"),
        FilePurpose::ExportEvaluation => Ok("选择评估报告导出目录"),
        _ => Err(host_error(
            ErrorCode::UnsupportedCapability,
            "This Host selects project, handoff and evaluation directories",
        )),
    }
}

#[tauri::command]
pub async fn project_select_directory(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, ProjectApiState>,
    purpose: FilePurpose,
) -> Result<SelectedFileRef, ApiError> {
    let title = directory_dialog_title(window.label(), &purpose)?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_parent(&window)
        .set_title(title)
        .pick_folder(move |selected| {
            // A dropped command receiver has no caller left to receive a selection.
            let _ = sender.send(selected);
        });
    let selected = receiver.await.map_err(|_| {
        host_error(
            ErrorCode::IoFailure,
            "Directory picker ended without a response",
        )
    })?;
    let selected = selected
        .map(|path| {
            path.into_path().map_err(|_| {
                host_error(
                    ErrorCode::InvalidInput,
                    "Only local directories can be selected",
                )
            })
        })
        .transpose()?;
    state.register_directory(purpose, selected)
}

impl Default for ProjectApiState {
    fn default() -> Self {
        Self::new()
    }
}

fn file_dialog_title(window_label: &str, purpose: &FilePurpose) -> Result<&'static str, ApiError> {
    if window_label != "main" {
        return Err(host_error(
            ErrorCode::PermissionDenied,
            "Only the main desktop window may select a package file",
        ));
    }
    match purpose {
        FilePurpose::ImportPackage => Ok("打开工程或模块ZIP"),
        FilePurpose::ExportPackage => Ok("导出工程或贡献ZIP"),
        _ => Err(host_error(
            ErrorCode::UnsupportedCapability,
            "This file picker only opens or exports package files",
        )),
    }
}

#[tauri::command]
pub async fn project_select_file(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, ProjectApiState>,
    purpose: FilePurpose,
) -> Result<SelectedFileRef, ApiError> {
    let title = file_dialog_title(window.label(), &purpose)?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let picker = app
        .dialog()
        .file()
        .set_parent(&window)
        .set_title(title)
        .add_filter("超级工厂工程或模块", &["zip", "sfproj"]);
    if matches!(purpose, FilePurpose::ExportPackage) {
        picker
            .set_file_name("超级工厂工程包.zip")
            .save_file(move |selected| {
                let _ = sender.send(selected);
            });
    } else {
        picker.pick_file(move |selected| {
            let _ = sender.send(selected);
        });
    }
    let selected = receiver
        .await
        .map_err(|_| host_error(ErrorCode::IoFailure, "File picker ended without a response"))?;
    let selected = selected
        .map(|path| {
            path.into_path().map_err(|_| {
                host_error(
                    ErrorCode::InvalidInput,
                    "Only local package files can be selected",
                )
            })
        })
        .transpose()?;
    state.register_file(purpose, selected)
}

fn boundary_error(
    request_id: Option<ObjectId>,
    code: ErrorCode,
    message: String,
    field_errors: Vec<ApiFieldError>,
) -> Value {
    json!(Response::<Value> {
        api_version: ApiVersion::V01,
        request_id,
        context: None,
        outcome: ApiOutcome::Error(ApiError {
            code,
            message,
            object_refs: vec![],
            field_errors,
            expected_revision: None,
            actual_revision: None,
        }),
    })
}

#[tauri::command]
pub fn project_close_state(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
) -> Result<sf_commands::DesktopCloseState, ApiError> {
    state.desktop_close_state(window.label())
}

#[tauri::command]
pub fn project_finish_close(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    expected: sf_commands::DesktopCloseState,
    discard_confirmed: bool,
) -> Result<(), ApiError> {
    state.finish_desktop_close(window.label(), &expected, discard_confirmed, || window.destroy().map_err(|error| error.to_string()))
}

#[tauri::command]
pub fn project_api(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: Value,
) -> Value {
    state.dispatch_ui(window.label(), request)
}

fn smoke_request(id: &str, project_id: Value, operation: &str, params: Value) -> Value {
    let scope = if project_id.is_null() {
        "workspace"
    } else {
        "project"
    };
    json!({
        "api_version": "0.1",
        "request_id": id,
        "project_id": project_id,
        "scope": { "kind": scope },
        "operation": operation,
        "params": params,
    })
}

fn smoke_report(state: &ProjectApiState) -> Value {
    let mut checks: Vec<Value> = vec![];
    macro_rules! check {
        ($name:literal, $condition:expr, $response:expr) => {{
            let passed = $condition;
            checks.push(json!({ "name": $name, "passed": passed, "response": $response }));
            if !passed {
                return json!({ "kind": "P0_01_host_smoke", "passed": false, "checks": checks });
            }
        }};
    }

    let initial = state.dispatch_ui(
        "main",
        smoke_request(
            "initial",
            Value::Null,
            "get_context",
            json!({ "selection": [] }),
        ),
    );
    check!(
        "managed_host_starts_without_project",
        initial["outcome"]["status"] == "ok" && initial["outcome"]["data"]["project"].is_null(),
        initial
    );

    let time_reference =
        json!({ "label": "P0-01 smoke", "origin_iso8601": null, "timezone": "Asia/Shanghai" });
    let created = state.dispatch_ui("main", smoke_request("create", Value::Null, "apply_commands", json!({
        "base_revision": null,
        "commands": [{ "kind": "create_project", "data": {
            "title": "P0-01 Host smoke", "time_reference": time_reference, "departure": "stay"
        }}]
    })));
    let project_id = created["outcome"]["data"]["project_id"].clone();
    check!(
        "workspace_create_commits_revision_one",
        created["outcome"]["status"] == "ok"
            && created["outcome"]["data"]["new_revision"] == 1
            && project_id.as_str().is_some_and(|id| !id.is_empty()),
        created
    );

    let project_ref = json!({ "kind": "project", "id": project_id });
    let read_request = |id: &str| {
        smoke_request(
            id,
            project_id.clone(),
            "get_object",
            json!({
                "object_id": project_ref, "source": { "kind": "draft" }, "node_index": null
            }),
        )
    };
    let read = state.dispatch_ui("main", read_request("read-created"));
    check!(
        "read_returns_created_project",
        read["outcome"]["status"] == "ok"
            && read["outcome"]["data"]["kind"] == "project"
            && read["outcome"]["data"]["data"]["id"] == project_id
            && read["outcome"]["data"]["data"]["revision"] == 1
            && read["outcome"]["data"]["data"]["title"] == "P0-01 Host smoke",
        read
    );

    let settings = json!({ "kind": "set_project_settings", "data": {
        "title": "P0-01 edited through desktop Host", "time_reference": time_reference,
        "generation_context": null, "sources": []
    }});
    let edit = state.dispatch_ui(
        "main",
        smoke_request(
            "edit",
            project_id.clone(),
            "apply_commands",
            json!({ "base_revision": 1, "commands": [settings] }),
        ),
    );
    check!(
        "edit_commits_revision_two",
        edit["outcome"]["status"] == "ok"
            && edit["outcome"]["data"]["previous_revision"] == 1
            && edit["outcome"]["data"]["new_revision"] == 2,
        edit
    );

    let read = state.dispatch_ui("main", read_request("read-edited"));
    check!(
        "read_returns_edited_data",
        read["outcome"]["status"] == "ok"
            && read["outcome"]["data"]["data"]["revision"] == 2
            && read["outcome"]["data"]["data"]["title"] == "P0-01 edited through desktop Host",
        read
    );

    let stale = state.dispatch_ui(
        "main",
        smoke_request(
            "stale",
            project_id.clone(),
            "apply_commands",
            json!({ "base_revision": 1, "commands": [settings] }),
        ),
    );
    check!(
        "stale_edit_returns_conflict",
        stale["outcome"]["status"] == "error"
            && stale["outcome"]["data"]["code"] == "RevisionConflict"
            && stale["outcome"]["data"]["actual_revision"] == 2,
        stale
    );

    let mut forged = read_request("forged-caller");
    forged["caller"] = json!({ "origin": "interactive_ui", "session_id": "attacker" });
    let forged = state.dispatch_ui("main", forged);
    check!(
        "json_cannot_supply_caller",
        forged["outcome"]["status"] == "error"
            && forged["outcome"]["data"]["code"] == "InvalidInput",
        forged
    );

    let denied = state.dispatch_ui("secondary", read_request("secondary-window"));
    check!(
        "non_main_window_denied",
        denied["outcome"]["status"] == "error"
            && denied["outcome"]["data"]["code"] == "PermissionDenied",
        denied
    );

    let invalid_comparison = state.dispatch_ui(
        "main",
        smoke_request(
            "unsupported",
            project_id.clone(),
            "compare_runs",
            json!({
                "run_ids": [], "metric_scope": {"keys": [], "objects": [], "acceptance": []}
            }),
        ),
    );
    check!(
        "implemented_comparison_requires_two_real_runs",
        invalid_comparison["outcome"]["status"] == "error"
            && invalid_comparison["outcome"]["data"]["code"] == "InvalidInput",
        invalid_comparison
    );
    // A plain adapter has no renderer. The actual desktop registers one before
    // this startup consumer runs; test its admitted task without waiting for a
    // frame on the setup thread, which has not entered the window event loop.
    let unsupported = state.dispatch_ui(
        "main",
        smoke_request(
            "unsupported-render",
            project_id.clone(),
            "render_view",
            json!({
                "revision": 2,
                "view": { "kind": "line_canvas", "canvas": "line_canvas", "hall_id": null,
                    "object_ref": null, "project_revision": 2, "run_id": null,
                    "view_time_ms": null, "lod": "detailed",
                    "camera": read["outcome"]["data"]["data"]["views"]["line_camera"] },
                "object_id": null, "preview": null, "delivery": "capture_only"
            }),
        ),
    );
    if unsupported["outcome"]["status"] == "ok" {
        let task = unsupported["outcome"]["data"]["id"].clone();
        check!(
            "registered_native_renderer_admits_real_task",
            unsupported["outcome"]["data"]["kind"] == "rendering"
                && unsupported["outcome"]["data"]["state"] == "queued",
            unsupported
        );
        let cancelled = state.dispatch_ui("main", smoke_request(
            "cancel-startup-render", project_id.clone(), "cancel_task",
            json!({"task_id":task,"action":"cancel"}),
        ));
        check!(
            "startup_render_cancel_is_accepted",
            cancelled["outcome"]["status"] == "ok"
                && cancelled["outcome"]["data"]["action_accepted"] == true,
            cancelled
        );
        let terminal = state.wait_for_task(project_id.clone(), task);
        check!(
            "startup_render_is_terminal_before_persistence_checks",
            terminal["outcome"]["status"] == "ok"
                && terminal["outcome"]["data"]["task"]["state"] == "cancelled",
            terminal
        );
    } else {
        check!(
            "known_unimplemented_operation_is_explicit",
            unsupported["outcome"]["status"] == "error"
                && unsupported["outcome"]["data"]["code"] == "UnsupportedCapability",
            unsupported
        );
    }

    let context = state.dispatch_ui(
        "main",
        smoke_request(
            "final-context",
            project_id.clone(),
            "get_context",
            json!({ "selection": [] }),
        ),
    );
    check!(
        "rejected_requests_preserve_same_host_project",
        context["outcome"]["status"] == "ok"
            && context["outcome"]["data"]["project"]["id"] == project_id
            && context["outcome"]["data"]["project"]["revision"] == 2
            && context["outcome"]["data"]["project"]["title"]
                == "P0-01 edited through desktop Host",
        context
    );

    json!({ "kind": "P0_01_host_smoke", "passed": true, "checks": checks })
}

fn smoke_directory() -> std::io::Result<(PathBuf, PathBuf)> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let temp = std::env::temp_dir().canonicalize()?;
    let root = temp.join(format!(
        "superfactory-host-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    // Never reuse or delete a pre-existing directory belonging to another run.
    std::fs::create_dir(&root)?;
    Ok((temp, root))
}

fn smoke_files(root: &Path) -> std::io::Result<std::collections::BTreeMap<PathBuf, Vec<u8>>> {
    fn visit(
        base: &Path,
        directory: &Path,
        files: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(directory)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                return Err(std::io::Error::other(
                    "Smoke archive contains an unexpected symbolic link",
                ));
            }
            if kind.is_dir() {
                visit(base, &entry.path(), files)?;
            } else if kind.is_file() {
                files.insert(
                    entry
                        .path()
                        .strip_prefix(base)
                        .expect("visited child of archive")
                        .to_path_buf(),
                    std::fs::read(entry.path())?,
                );
            }
        }
        Ok(())
    }
    let mut files = std::collections::BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}

fn remove_smoke_directory(temp: &Path, root: &Path) -> std::io::Result<()> {
    let resolved = root.canonicalize()?;
    if resolved == temp || !resolved.starts_with(temp) {
        return Err(std::io::Error::other(
            "Smoke cleanup target is outside its temporary directory",
        ));
    }
    std::fs::remove_dir_all(&resolved)
}

fn smoke_settings(title: &str) -> Value {
    json!({ "kind": "set_project_settings", "data": {
        "title": title,
        "time_reference": { "label": "P0-01 smoke", "origin_iso8601": null, "timezone": "Asia/Shanghai" },
        "generation_context": null, "sources": []
    }})
}

fn persistence_smoke_report(state: &ProjectApiState) -> Value {
    let mut checks: Vec<Value> = vec![];
    macro_rules! check {
        ($name:literal, $condition:expr, $response:expr) => {{
            let passed = $condition;
            checks.push(json!({ "name": $name, "passed": passed, "response": $response }));
            if !passed { return json!({ "passed": false, "checks": checks }); }
        }};
    }
    macro_rules! file_io {
        ($name:literal, $operation:expr) => {
            match $operation {
                Ok(value) => value,
                Err(error) => {
                    checks.push(json!({ "name": $name, "passed": false, "error": format!("{error:?}") }));
                    return json!({ "passed": false, "checks": checks });
                }
            }
        };
    }
    let context_request = || {
        smoke_request(
            "file-context",
            Value::Null,
            "get_context",
            json!({ "selection": [] }),
        )
    };
    let context = state.dispatch_ui("main", context_request());
    let project_id = context["outcome"]["data"]["project"]["id"].clone();
    let revision = context["outcome"]["data"]["project"]["revision"].clone();
    let original_title = context["outcome"]["data"]["project"]["title"].clone();
    let (temp, root) = file_io!("create_owned_temporary_directory", smoke_directory());
    let source = root.join("source");
    let relocated = root.join("relocated");
    file_io!("create_save_destination", std::fs::create_dir(&source));
    let selected = file_io!(
        "register_save_directory",
        state.register_directory(FilePurpose::SaveProject, Some(source.clone()))
    );
    let save = state.dispatch_ui(
        "main",
        smoke_request(
            "save-frozen",
            project_id.clone(),
            "save_project",
            json!({ "revision": revision, "selected_directory_ref": selected }),
        ),
    );
    let save_task = save["outcome"]["data"]["id"].clone();
    check!(
        "save_returns_real_task_handle",
        save["outcome"]["status"] == "ok" && save_task.is_string(),
        save
    );

    let edited = state.dispatch_ui("main", smoke_request("edit-after-save-admission", project_id.clone(), "apply_commands", json!({ "base_revision": revision, "commands": [smoke_settings("Changed after save admission")] })));
    check!(
        "edit_does_not_wait_for_disk_task",
        edited["outcome"]["status"] == "ok",
        edited
    );
    let saved = state.wait_for_task(project_id.clone(), save_task);
    check!(
        "completed_save_contains_frozen_revision",
        saved["outcome"]["status"] == "ok"
            && saved["outcome"]["data"]["task"]["state"] == "completed"
            && saved["outcome"]["data"]["result"]["kind"] == "save"
            && saved["outcome"]["data"]["result"]["data"]["saved_revision"] == revision,
        saved
    );
    let context = state.dispatch_ui("main", context_request());
    check!(
        "newer_current_revision_remains_dirty",
        context["outcome"]["data"]["project"]["dirty"] == true
            && context["outcome"]["data"]["project"]["revision"]
                == edited["outcome"]["data"]["new_revision"],
        context
    );

    let original_files = file_io!("read_saved_archive", smoke_files(&source));
    file_io!(
        "create_relocated_directory",
        std::fs::create_dir(&relocated)
    );
    for (relative, bytes) in &original_files {
        let destination = relocated.join(relative);
        if let Some(parent) = destination.parent() {
            file_io!(
                "create_relocated_subdirectory",
                std::fs::create_dir_all(parent)
            );
        }
        file_io!("copy_saved_archive", std::fs::write(destination, bytes));
    }
    let relocated_files = file_io!("read_relocated_archive", smoke_files(&relocated));
    check!(
        "relocated_archive_is_byte_identical",
        original_files == relocated_files && original_files.contains_key(Path::new("project.json")),
        json!({ "files": original_files.len() })
    );

    let selected = file_io!(
        "register_import_directory",
        state.register_directory(FilePurpose::ImportPackage, Some(relocated.clone()))
    );
    let import = state.dispatch_ui("main", smoke_request("import-relocated", Value::Null, "import_package", json!({ "selected_file_ref": selected, "import_policy": "open_project", "base_revision": null })));
    let import_task = import["outcome"]["data"]["id"].clone();
    check!(
        "import_returns_real_workspace_task",
        import["outcome"]["status"] == "ok"
            && import_task.is_string()
            && import["outcome"]["data"]["scope"]["kind"] == "workspace",
        import
    );
    let imported = state.wait_for_task(Value::Null, import_task.clone());
    check!(
        "completed_import_contains_staged_draft",
        imported["outcome"]["status"] == "ok"
            && imported["outcome"]["data"]["task"]["state"] == "completed"
            && imported["outcome"]["data"]["result"]["kind"] == "import",
        imported
    );
    let context = state.dispatch_ui("main", context_request());
    check!(
        "import_does_not_implicitly_replace_project",
        context["outcome"]["data"]["project"]["title"] == "Changed after save admission",
        context
    );

    let adopt = state.dispatch_ui("main", smoke_request("adopt-open-project", Value::Null, "apply_commands", json!({ "base_revision": null, "commands": [{ "kind": "adopt_imported_draft", "data": {
        "task_id": import_task, "choices": {
            "chosen_policy": "open_project", "objects_to_create": [], "module_versions": [],
            "reused_bindings": { "item_types": {}, "resources": {}, "targets": {}, "ports": {} },
            "selected_assets": [], "accept_missing_as_draft": false, "departure": "discard_after_tasks_terminal"
        }
    }}] })));
    check!(
        "explicit_adoption_opens_relocated_project",
        adopt["outcome"]["status"] == "ok",
        adopt
    );
    let context = state.dispatch_ui("main", context_request());
    let reopened_revision = context["outcome"]["data"]["project"]["revision"].clone();
    check!(
        "reopened_data_matches_saved_snapshot",
        context["outcome"]["data"]["project"]["id"] == project_id
            && context["outcome"]["data"]["project"]["title"] == original_title,
        context
    );

    let edited = state.dispatch_ui("main", smoke_request("edit-reopened", project_id.clone(), "apply_commands", json!({ "base_revision": reopened_revision, "commands": [smoke_settings("Relocated project edited")] })));
    check!(
        "reopened_project_remains_editable",
        edited["outcome"]["status"] == "ok",
        edited
    );
    let undo = state.dispatch_ui("main", smoke_request("undo-reopened-edit", project_id.clone(), "apply_commands", json!({ "base_revision": edited["outcome"]["data"]["new_revision"], "commands": [{ "kind": "undo" }] })));
    check!(
        "undo_commits_a_new_revision",
        undo["outcome"]["status"] == "ok"
            && undo["outcome"]["data"]["new_revision"] != edited["outcome"]["data"]["new_revision"],
        undo
    );
    let context = state.dispatch_ui("main", context_request());
    check!(
        "undo_restores_draft_title",
        context["outcome"]["data"]["project"]["title"] == original_title,
        context
    );
    let redo = state.dispatch_ui("main", smoke_request("redo-reopened-edit", project_id.clone(), "apply_commands", json!({ "base_revision": undo["outcome"]["data"]["new_revision"], "commands": [{ "kind": "redo" }] })));
    check!(
        "redo_commits_a_new_revision",
        redo["outcome"]["status"] == "ok"
            && redo["outcome"]["data"]["new_revision"] != undo["outcome"]["data"]["new_revision"],
        redo
    );
    let context = state.dispatch_ui("main", context_request());
    check!(
        "redo_restores_edited_title",
        context["outcome"]["data"]["project"]["title"] == "Relocated project edited",
        context
    );
    let source_after_history = file_io!("read_source_after_history", smoke_files(&source));
    let relocated_after_history = file_io!("read_relocated_after_history", smoke_files(&relocated));
    check!(
        "history_never_rewrites_disk_archives",
        source_after_history == original_files && relocated_after_history == relocated_files,
        json!({ "source_files": source_after_history.len(), "relocated_files": relocated_after_history.len() })
    );

    let final_revision = redo["outcome"]["data"]["new_revision"].clone();
    let selected = file_io!(
        "register_relocated_save_directory",
        state.register_directory(FilePurpose::SaveProject, Some(relocated.clone()))
    );
    let save = state.dispatch_ui(
        "main",
        smoke_request(
            "save-relocated",
            project_id.clone(),
            "save_project",
            json!({ "revision": final_revision, "selected_directory_ref": selected }),
        ),
    );
    let task_id = save["outcome"]["data"]["id"].clone();
    check!(
        "relocated_project_can_be_saved_again",
        save["outcome"]["status"] == "ok" && task_id.is_string(),
        save
    );
    let saved = state.wait_for_task(project_id, task_id);
    check!(
        "relocated_save_has_actual_completion",
        saved["outcome"]["status"] == "ok"
            && saved["outcome"]["data"]["task"]["state"] == "completed"
            && saved["outcome"]["data"]["result"]["kind"] == "save"
            && saved["outcome"]["data"]["result"]["data"]["saved_revision"] == final_revision,
        saved
    );
    let context = state.dispatch_ui("main", context_request());
    check!(
        "saved_current_revision_is_clean",
        context["outcome"]["data"]["project"]["dirty"] == false
            && context["outcome"]["data"]["project"]["revision"] == final_revision,
        context
    );
    let manifest_bytes = file_io!(
        "read_relocated_current_manifest",
        std::fs::read(relocated.join("project.json"))
    );
    let manifest = file_io!(
        "parse_relocated_current_manifest",
        serde_json::from_slice::<Value>(&manifest_bytes)
    );
    check!(
        "relocated_directory_contains_saved_current_revision",
        manifest["data"]["project_revision"] == final_revision
            && manifest["data"]["title"] == "Relocated project edited",
        manifest
    );
    let source_after_save = file_io!("read_original_after_relocated_save", smoke_files(&source));
    check!(
        "relocated_save_does_not_touch_original_directory",
        source_after_save == original_files,
        json!({ "files": source_after_save.len() })
    );
    file_io!(
        "cleanup_owned_temporary_directory",
        remove_smoke_directory(&temp, &root)
    );
    json!({ "passed": true, "checks": checks })
}

fn full_smoke_report(state: &ProjectApiState) -> Value {
    let mut report = smoke_report(state);
    if report["passed"] == true {
        let persistence = persistence_smoke_report(state);
        report["passed"] = persistence["passed"].clone();
        report["checks"]
            .as_array_mut()
            .expect("owned smoke checks array")
            .extend(
                persistence["checks"]
                    .as_array()
                    .expect("owned persistence checks array")
                    .iter()
                    .cloned(),
            );
    }
    report
}

fn seed_business_smoke_archive(directory: &Path) -> std::io::Result<()> {
    macro_rules! fixture {
        ($name:literal) => {
            include_bytes!(concat!(
                "../../../../tools/type-contract-proof/fixtures/",
                $name,
                ".json"
            ))
            .as_slice()
        };
    }
    let mut manifest: Value =
        serde_json::from_slice(fixture!("manifest")).map_err(std::io::Error::other)?;
    let mut project: Value =
        serde_json::from_slice(fixture!("project")).map_err(std::io::Error::other)?;
    // Match the existing independent positive fixture: the actual external
    // Operations module is selected, rather than pretending a Devices module is it.
    project["data"]["devices"][0]["operation_binding"] =
        json!({"id":"demo.operations","version":"0.1.0"});
    for entry in manifest["data"]["documents"]
        .as_array_mut()
        .expect("frozen manifest documents")
    {
        if entry["role"] == "active_project" {
            entry["file_ref"]["path"] = json!("active/project.json");
        }
    }
    std::fs::create_dir_all(directory.join("active"))?;
    std::fs::write(
        directory.join("active/project.json"),
        serde_json::to_vec(&project).map_err(std::io::Error::other)?,
    )?;
    for (name, bytes) in [
        ("scenario", fixture!("scenario")),
        ("module-items", fixture!("module-items")),
        ("module-devices", fixture!("module-devices")),
        ("module-resources", fixture!("module-resources")),
        ("module-tasks", fixture!("module-tasks")),
        ("module-operations", fixture!("module-operations")),
        ("module-transport", fixture!("module-transport")),
        ("module-templates", fixture!("module-templates")),
        ("module-professional", fixture!("module-professional")),
        ("module-media", fixture!("module-media")),
        ("template", fixture!("template")),
        ("generation-profile", fixture!("generation-profile")),
    ] {
        std::fs::write(directory.join(format!("{name}.json")), bytes)?;
    }
    std::fs::write(
        directory.join("project.json"),
        serde_json::to_vec(&manifest).map_err(std::io::Error::other)?,
    )
}

fn business_object(state: &ProjectApiState, project: &Value, kind: &str, id: &str) -> Value {
    state.dispatch_ui(
        "main",
        smoke_request(
            "business-object",
            project.clone(),
            "get_object",
            json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"draft"},"node_index":null}),
        ),
    )
}

fn business_edit_scenario(state: &ProjectApiState, project: &Value, mut scenario: Value) -> Value {
    let context = state.dispatch_ui(
        "main",
        smoke_request(
            "business-edit-context",
            project.clone(),
            "get_context",
            json!({"selection":[]}),
        ),
    );
    let base = context["outcome"]["data"]["project"]["revision"].clone();
    scenario["project_revision"] = base.clone();
    state.dispatch_ui("main", smoke_request("business-edit-scenario", project.clone(), "apply_commands", json!({"base_revision":base,"commands":[{"kind":"put_object","data":{"kind":"scenario","data":scenario}}]})))
}

fn business_adoption(task: Value, imported: &Value) -> Value {
    let draft = &imported["outcome"]["data"]["result"]["data"];
    json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task,"choices":{
        "chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"
    }}}]})
}

fn business_archive_scenario(directory: &Path, scenario_id: &str) -> std::io::Result<Value> {
    let manifest: sf_domain::Versioned<sf_domain::PackageManifest> =
        serde_json::from_slice(&std::fs::read(directory.join("project.json"))?)
            .map_err(std::io::Error::other)?;
    let entry = manifest
        .data
        .documents
        .iter()
        .find(|entry| {
            matches!(entry.role, sf_domain::DocumentRole::ScenarioInput)
                && entry.object_ref.id.as_str() == scenario_id
        })
        .ok_or_else(|| {
            std::io::Error::other("Requested saved ScenarioInput is absent from the manifest")
        })?;
    serde_json::from_slice(&std::fs::read(
        directory.join(entry.file_ref.path.as_str()),
    )?)
    .map_err(std::io::Error::other)
}

fn business_full_smoke_report(state: &ProjectApiState) -> Value {
    let previous = full_smoke_report(state);
    if previous["passed"] != true {
        return previous;
    }
    let mut checks = previous["checks"]
        .as_array()
        .expect("owned previous smoke checks")
        .clone();
    macro_rules! check {
        ($name:literal, $condition:expr, $response:expr) => {{
            let passed = $condition;
            checks.push(json!({"name":$name,"passed":passed,"response":$response}));
            if !passed { return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    macro_rules! io {
        ($name:literal, $operation:expr) => { match $operation {
            Ok(value) => value,
            Err(error) => { checks.push(json!({"name":$name,"passed":false,"error":format!("{error:?}")})); return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    let (temp, root) = io!("business_owned_temporary_directory", smoke_directory());
    let seed = root.join("seed");
    let saved_directory = root.join("saved");
    let relocated = root.join("relocated");
    io!("business_seed_directory", std::fs::create_dir(&seed));
    io!(
        "business_seed_nine_categories",
        seed_business_smoke_archive(&seed)
    );
    let selection = io!(
        "business_seed_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(seed))
    );
    let import = state.dispatch_ui("main", smoke_request("business-seed-import", Value::Null, "import_package", json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null})));
    check!(
        "business_seed_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task.clone());
    check!(
        "business_seed_real_task_complete_without_missing",
        imported["outcome"]["data"]["task"]["state"] == "completed"
            && imported["outcome"]["data"]["result"]["data"]["missing"]
                .as_array()
                .is_some_and(Vec::is_empty),
        imported
    );
    let opened = state.dispatch_ui(
        "main",
        smoke_request(
            "business-seed-adopt",
            Value::Null,
            "apply_commands",
            business_adoption(task, &imported),
        ),
    );
    check!(
        "business_seed_explicit_open",
        opened["outcome"]["status"] == "ok",
        opened
    );
    let project = json!("P1");
    let mut fixture: Value = io!(
        "business_formal_dto_fixture",
        serde_json::from_str(include_str!(
            "../../../../packages/commands/tests/fixtures/business-inputs/multi-order.dto.json"
        ))
    );
    let context = state.dispatch_ui(
        "main",
        smoke_request(
            "business-base",
            project.clone(),
            "get_context",
            json!({"selection":[]}),
        ),
    );
    let base = context["outcome"]["data"]["project"]["revision"].clone();
    fixture["scenario"]["project_revision"] = base.clone();
    let put =
        |kind: &str, value: &Value| json!({"kind":"put_object","data":{"kind":kind,"data":value}});
    let mut commands = vec![put("scenario", &fixture["scenario"])];
    for (key, kind) in [("products", "product"), ("boms", "bom")] {
        for value in fixture[key].as_array().expect("formal fixture object list") {
            commands.push(put(kind, value));
        }
    }
    commands.push(put("route", &fixture["route"]));
    for value in fixture["items"]
        .as_array()
        .expect("formal fixture item types")
    {
        commands.push(put("item_type", value));
    }
    let commit = state.dispatch_ui(
        "main",
        smoke_request(
            "business-forward-batch",
            project.clone(),
            "apply_commands",
            json!({"base_revision":base,"commands":commands}),
        ),
    );
    check!(
        "business_forward_batch_is_one_project_commit",
        commit["outcome"]["status"] == "ok"
            && commit["outcome"]["data"]["previous_revision"] == base
            && commit["outcome"]["data"]["new_revision"].as_u64()
                == base.as_u64().and_then(|n| n.checked_add(1)),
        commit
    );
    let context = state.dispatch_ui(
        "main",
        smoke_request(
            "business-input-issues",
            project.clone(),
            "get_context",
            json!({"selection":[]}),
        ),
    );
    check!(
        "business_complete_positive_has_no_issues",
        context["outcome"]["data"]["issues"]
            .as_array()
            .is_some_and(Vec::is_empty),
        context
    );
    let first = business_object(state, &project, "scenario", "ORDERS");
    check!(
        "business_formal_order_and_task_roundtrip",
        first["outcome"]["status"] == "ok"
            && first["outcome"]["data"]["data"]["orders"] == fixture["scenario"]["orders"]
            && first["outcome"]["data"]["data"]["production_tasks"]
                == fixture["scenario"]["production_tasks"]
            && first["outcome"]["data"]["data"]["project_revision"]
                == commit["outcome"]["data"]["new_revision"],
        first
    );
    let set_amount = |scenario: &mut Value, n: u64| {
        scenario["orders"][0]["quantity"]["value"]["coefficient"] = json!(n);
        scenario["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(n);
        scenario["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
            json!(n);
    };
    let mut changed = first["outcome"]["data"]["data"].clone();
    set_amount(&mut changed, 3);
    let edit = business_edit_scenario(state, &project, changed);
    check!(
        "business_scenario_edit_commits",
        edit["outcome"]["status"] == "ok",
        edit
    );
    let undo = state.dispatch_ui("main", smoke_request("business-undo", project.clone(), "apply_commands", json!({"base_revision":edit["outcome"]["data"]["new_revision"],"commands":[{"kind":"undo"}]})));
    check!(
        "business_project_and_scenario_undo_commits",
        undo["outcome"]["status"] == "ok",
        undo
    );
    let undone = business_object(state, &project, "scenario", "ORDERS");
    check!(
        "business_undo_restores_orders_and_tasks",
        undone["outcome"]["data"]["data"]["orders"] == fixture["scenario"]["orders"]
            && undone["outcome"]["data"]["data"]["production_tasks"]
                == fixture["scenario"]["production_tasks"]
            && undone["outcome"]["data"]["data"]["project_revision"]
                == undo["outcome"]["data"]["new_revision"],
        undone
    );
    let redo = state.dispatch_ui("main", smoke_request("business-redo", project.clone(), "apply_commands", json!({"base_revision":undo["outcome"]["data"]["new_revision"],"commands":[{"kind":"redo"}]})));
    check!(
        "business_project_and_scenario_redo_commits",
        redo["outcome"]["status"] == "ok",
        redo
    );
    let frozen = business_object(state, &project, "scenario", "ORDERS");
    check!(
        "business_redo_restores_quantity_with_fresh_structure_revision",
        frozen["outcome"]["data"]["data"]["orders"][0]["quantity"]["value"]["coefficient"] == 3
            && frozen["outcome"]["data"]["data"]["revision"].as_u64()
                > undone["outcome"]["data"]["data"]["revision"].as_u64()
            && frozen["outcome"]["data"]["data"]["project_revision"]
                == redo["outcome"]["data"]["new_revision"],
        frozen
    );
    io!(
        "business_save_directory",
        std::fs::create_dir(&saved_directory)
    );
    let selection = io!(
        "business_save_selection",
        state.register_directory(FilePurpose::SaveProject, Some(saved_directory.clone()))
    );
    let revision = redo["outcome"]["data"]["new_revision"].clone();
    let save = state.dispatch_ui(
        "main",
        smoke_request(
            "business-frozen-save",
            project.clone(),
            "save_project",
            json!({"revision":revision,"selected_directory_ref":selection}),
        ),
    );
    check!(
        "business_multi_document_save_admitted",
        save["outcome"]["status"] == "ok",
        save
    );
    let mut newer = frozen["outcome"]["data"]["data"].clone();
    set_amount(&mut newer, 4);
    let edit = business_edit_scenario(state, &project, newer);
    check!(
        "business_newer_scenario_edit_is_independent",
        edit["outcome"]["status"] == "ok",
        edit
    );
    let saved = state.wait_for_task(project.clone(), save["outcome"]["data"]["id"].clone());
    check!(
        "business_frozen_save_task_finishes",
        saved["outcome"]["data"]["task"]["state"] == "completed"
            && saved["outcome"]["data"]["result"]["data"]["saved_revision"] == revision,
        saved
    );
    let on_disk = io!(
        "business_read_frozen_scenario",
        business_archive_scenario(&saved_directory, "ORDERS")
    );
    check!(
        "business_saved_scenario_is_same_generation_as_project",
        on_disk["data"]["project_revision"] == revision
            && on_disk["data"]["orders"] == frozen["outcome"]["data"]["data"]["orders"]
            && on_disk["data"]["production_tasks"]
                == frozen["outcome"]["data"]["data"]["production_tasks"],
        on_disk
    );
    let current = business_object(state, &project, "scenario", "ORDERS");
    let context = state.dispatch_ui(
        "main",
        smoke_request(
            "business-newer-context",
            project.clone(),
            "get_context",
            json!({"selection":[]}),
        ),
    );
    check!(
        "business_newer_orders_remain_current_and_dirty",
        current["outcome"]["data"]["data"]["orders"][0]["quantity"]["value"]["coefficient"] == 4
            && context["outcome"]["data"]["project"]["dirty"] == true,
        json!({"object":current,"context":context})
    );
    let files = io!("business_saved_files", smoke_files(&saved_directory));
    io!(
        "business_relocated_directory",
        std::fs::create_dir(&relocated)
    );
    for (path, bytes) in &files {
        let target = relocated.join(path);
        io!(
            "business_relocated_parent",
            std::fs::create_dir_all(target.parent().expect("archive child"))
        );
        io!("business_copy_archive", std::fs::write(target, bytes));
    }
    let selection = io!(
        "business_relocated_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(relocated.clone()))
    );
    let import = state.dispatch_ui("main", smoke_request("business-reopen-import", Value::Null, "import_package", json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null})));
    check!(
        "business_relocated_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task.clone());
    check!(
        "business_relocated_import_has_no_missing_inputs",
        imported["outcome"]["data"]["task"]["state"] == "completed"
            && imported["outcome"]["data"]["result"]["data"]["missing"]
                .as_array()
                .is_some_and(Vec::is_empty),
        imported
    );
    let before_adopt = business_object(state, &project, "scenario", "ORDERS");
    check!(
        "business_import_is_not_implicit_adoption",
        before_adopt["outcome"]["data"]["data"]["orders"][0]["quantity"]["value"]["coefficient"]
            == 4,
        before_adopt
    );
    let adopt = state.dispatch_ui(
        "main",
        smoke_request(
            "business-reopen-adopt",
            Value::Null,
            "apply_commands",
            business_adoption(task, &imported),
        ),
    );
    check!(
        "business_relocated_explicit_open_succeeds",
        adopt["outcome"]["status"] == "ok",
        adopt
    );
    let reopened = business_object(state, &project, "scenario", "ORDERS");
    check!(
        "business_reopened_orders_and_tasks_are_exact_saved_values",
        reopened["outcome"]["data"]["data"]["orders"] == on_disk["data"]["orders"]
            && reopened["outcome"]["data"]["data"]["production_tasks"]
                == on_disk["data"]["production_tasks"],
        reopened
    );
    let mut changed = reopened["outcome"]["data"]["data"].clone();
    set_amount(&mut changed, 5);
    let edit = business_edit_scenario(state, &project, changed);
    check!(
        "business_reopened_scenario_edit_succeeds",
        edit["outcome"]["status"] == "ok",
        edit
    );
    let undo = state.dispatch_ui("main", smoke_request("business-reopened-undo", project.clone(), "apply_commands", json!({"base_revision":edit["outcome"]["data"]["new_revision"],"commands":[{"kind":"undo"}]})));
    let undone = business_object(state, &project, "scenario", "ORDERS");
    check!(
        "business_reopened_history_restores_saved_input",
        undo["outcome"]["status"] == "ok"
            && undone["outcome"]["data"]["data"]["orders"] == on_disk["data"]["orders"],
        json!({"commit":undo,"object":undone})
    );
    let redo = state.dispatch_ui("main", smoke_request("business-reopened-redo", project.clone(), "apply_commands", json!({"base_revision":undo["outcome"]["data"]["new_revision"],"commands":[{"kind":"redo"}]})));
    let redone = business_object(state, &project, "scenario", "ORDERS");
    check!(
        "business_reopened_redo_restores_new_input",
        redo["outcome"]["status"] == "ok"
            && redone["outcome"]["data"]["data"]["orders"][0]["quantity"]["value"]["coefficient"]
                == 5,
        json!({"commit":redo,"object":redone})
    );
    let after_source = io!(
        "business_read_source_after_history",
        smoke_files(&saved_directory)
    );
    let after_relocated = io!(
        "business_read_relocated_after_history",
        smoke_files(&relocated)
    );
    check!(
        "business_history_does_not_modify_any_archive_file",
        after_source == files && after_relocated == files,
        json!({"files":files.len()})
    );
    io!(
        "business_cleanup_owned_directory",
        remove_smoke_directory(&temp, &root)
    );
    json!({"kind":"P0_01_host_smoke","passed":true,"checks":checks})
}

fn seed_material_smoke_archive(directory: &Path) -> std::io::Result<()> {
    seed_business_smoke_archive(directory)?;
    let path = directory.join("active/project.json");
    let mut project: Value =
        serde_json::from_slice(&std::fs::read(&path)?).map_err(std::io::Error::other)?;
    // Explicit fixture setup matching window5's formal RAW-STOCK contract.
    // This is imported through the same Host, not an out-of-band Host mutation.
    project["data"]["item_types"]
        .as_array_mut()
        .expect("typed seed item types")
        .push(json!({
            "id":"RAW-STOCK","revision":"r1","title":"declared raw batch",
            "unit":"piece","tracking":"batch","spec_ref":null,"attributes":{},"attribute_schema":[]
        }));
    for location in project["data"]["locations"]
        .as_array_mut()
        .expect("typed seed locations")
    {
        if location["id"] == "S1" || location["id"] == "S2" {
            location["accepted_item_types"]
                .as_array_mut()
                .expect("typed declared acceptance")
                .push(json!("RAW-STOCK"));
        }
    }
    std::fs::write(
        path,
        serde_json::to_vec(&project).map_err(std::io::Error::other)?,
    )
}

fn material_input_projection(scenario: &Value) -> Value {
    json!({
        "id":scenario["id"],"initial_conditions":scenario["initial_conditions"],
        "arrivals":scenario["arrivals"],"supplies":scenario["supplies"],
        "orders":scenario["orders"],"production_tasks":scenario["production_tasks"]
    })
}

fn material_blocker(
    context: &Value,
    scenario_id: &str,
    pointer_prefix: &str,
    code: Option<&str>,
) -> bool {
    context["outcome"]["data"]["issues"]
        .as_array()
        .is_some_and(|issues| {
            issues.iter().any(|issue| {
                issue["code"].as_str().is_some_and(|actual| {
                    actual.starts_with("business_inputs.material_")
                        && code.is_none_or(|expected| actual == expected)
                }) && issue["object_ref"] == json!({"kind":"scenario","id":scenario_id})
                    && issue["field_path"]
                        .as_str()
                        .is_some_and(|path| path.starts_with(pointer_prefix))
                    && issue["blocks"].as_array().is_some_and(|blocks| {
                        blocks.contains(&json!("execution"))
                            && blocks.contains(&json!("complete_conclusion"))
                    })
            })
        })
}

fn material_context(state: &ProjectApiState, project: &Value) -> Value {
    state.dispatch_ui(
        "main",
        smoke_request(
            "material-context",
            project.clone(),
            "get_context",
            json!({"selection":[]}),
        ),
    )
}

fn material_full_smoke_report(state: &ProjectApiState) -> Value {
    let previous = business_full_smoke_report(state);
    if previous["passed"] != true {
        return previous;
    }
    let mut checks = previous["checks"]
        .as_array()
        .expect("owned previous checks")
        .clone();
    macro_rules! check {
        ($name:literal, $condition:expr, $response:expr) => {{
            let passed = $condition;
            checks.push(json!({"name":$name,"passed":passed,"response":$response}));
            if !passed { return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    macro_rules! io {
        ($name:literal, $operation:expr) => { match $operation {
            Ok(value) => value,
            Err(error) => { checks.push(json!({"name":$name,"passed":false,"error":format!("{error:?}")})); return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    let (temp, root) = io!("material_owned_directory", smoke_directory());
    let seed = root.join("seed");
    let saved_directory = root.join("saved");
    let relocated = root.join("relocated");
    io!("material_seed_directory", std::fs::create_dir(&seed));
    io!(
        "material_seed_declared_raw_type",
        seed_material_smoke_archive(&seed)
    );
    let selected = io!(
        "material_seed_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(seed))
    );
    let import = state.dispatch_ui("main", smoke_request("material-seed-import", Value::Null, "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));
    check!(
        "material_seed_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task.clone());
    check!(
        "material_seed_real_task_completed",
        imported["outcome"]["data"]["task"]["state"] == "completed"
            && imported["outcome"]["data"]["result"]["data"]["missing"]
                .as_array()
                .is_some_and(Vec::is_empty),
        imported
    );
    let opened = state.dispatch_ui(
        "main",
        smoke_request(
            "material-seed-open",
            Value::Null,
            "apply_commands",
            business_adoption(task, &imported),
        ),
    );
    check!(
        "material_seed_explicit_same_host_open",
        opened["outcome"]["status"] == "ok",
        opened
    );
    let project = opened["outcome"]["data"]["project_id"].clone();
    let fixture: Value = io!(
        "material_formal_scenario_fixture",
        serde_json::from_str(include_str!(
            "../../../../packages/commands/tests/fixtures/business-inputs/initial-stock.scenario.json"
        ))
    );
    let _: sf_domain::ScenarioInput = io!(
        "material_fixture_is_typed_ScenarioInput",
        serde_json::from_value(fixture.clone())
    );
    let business: Value = io!(
        "material_business_fixture",
        serde_json::from_str(include_str!(
            "../../../../packages/commands/tests/fixtures/business-inputs/multi-order.dto.json"
        ))
    );
    let context = material_context(state, &project);
    check!(
        "material_context_resolves_current_project",
        context["outcome"]["status"] == "ok",
        context
    );
    let base = context["outcome"]["data"]["project"]["revision"].clone();
    let mut initial = fixture.clone();
    initial["project_revision"] = base.clone();
    let put =
        |kind: &str, value: &Value| json!({"kind":"put_object","data":{"kind":kind,"data":value}});
    let mut commands = vec![put("scenario", &initial)];
    for (key, kind) in [
        ("items", "item_type"),
        ("products", "product"),
        ("boms", "bom"),
    ] {
        for value in business[key]
            .as_array()
            .expect("formal business fixture list")
        {
            commands.push(put(kind, value));
        }
    }
    commands.push(put("route", &business["route"]));
    let committed = state.dispatch_ui(
        "main",
        smoke_request(
            "material-formal-input-batch",
            project.clone(),
            "apply_commands",
            json!({"base_revision":base,"commands":commands}),
        ),
    );
    check!(
        "material_formal_inputs_commit_once",
        committed["outcome"]["status"] == "ok"
            && committed["outcome"]["data"]["new_revision"].as_u64()
                == base.as_u64().and_then(|r| r.checked_add(1)),
        committed
    );
    let current = business_object(state, &project, "scenario", "STOCK");
    check!(
        "material_six_input_axes_roundtrip",
        current["outcome"]["status"] == "ok"
            && material_input_projection(&current["outcome"]["data"]["data"])
                == material_input_projection(&fixture),
        current
    );
    let positive = current["outcome"]["data"]["data"].clone();
    check!(
        "material_declared_initial_future_supply_demand_are_separate_inputs",
        positive["initial_conditions"]["items"][0]["id"] == "BATCH10"
            && positive["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"] == 10
            && positive["arrivals"][0]["items"][0]["id"] == "FUTURE5"
            && positive["arrivals"][0]["items"][0]["quantity"]["value"]["coefficient"] == 5
            && positive["arrivals"][0]["at_ms"] == 100000
            && positive["supplies"][0]["target_quantity"]["value"]["coefficient"] == 20
            && positive["orders"][0]["quantity"]["value"]["coefficient"] == 4
            && positive["orders"][1]["quantity"]["value"]["coefficient"] == 4,
        positive
    );
    check!(
        "material_pending_hard_and_soft_inputs_are_not_execution_results",
        positive["initial_conditions"]["items"][0]["quality"] == "pending"
            && positive["initial_conditions"]["material_allocations"][0]["quantity"]["value"]["coefficient"]
                == 4
            && positive["initial_conditions"]["material_allocations"][1]["quantity"]["value"]["coefficient"]
                == 6
            && positive["initial_conditions"]["material_allocations"][2]["item_refs"]
                == json!(["FUTURE5"])
            && positive["initial_conditions"]["material_allocations"][2]["mode"] == "soft_plan"
            && positive["initial_conditions"]["material_allocations"][3]["item_refs"] == json!([]),
        positive
    );
    let context = material_context(state, &project);
    check!(
        "material_positive_declared_inputs_have_no_material_issue",
        context["outcome"]["status"] == "ok"
            && context["outcome"]["data"]["issues"]
                .as_array()
                .is_some_and(|issues| issues.iter().all(|issue| !issue["code"]
                    .as_str()
                    .is_some_and(|code| code.starts_with("business_inputs.material_")))),
        context
    );

    let mut bad = positive.clone();
    bad["initial_conditions"]["material_allocations"]
        .as_array_mut()
        .expect("typed allocations")
        .iter_mut()
        .find(|allocation| allocation["id"] == "HARD-B")
        .expect("declared HARD-B")["quantity"]["value"]["coefficient"] = json!(7);
    bad["supplies"][0]["transport_ref"] =
        json!({"id":"material.missing.transport","version":"0.1.0"});
    let edited = business_edit_scenario(state, &project, bad.clone());
    check!(
        "material_semantic_failures_are_saved_as_explicit_draft",
        edited["outcome"]["status"] == "ok",
        edited
    );
    let current = business_object(state, &project, "scenario", "STOCK");
    let context = material_context(state, &project);
    check!(
        "material_invalid_inputs_remain_visible_with_both_use_blockers",
        material_input_projection(&current["outcome"]["data"]["data"])
            == material_input_projection(&bad)
            && material_blocker(
                &context,
                "STOCK",
                "/initial_conditions/material_allocations",
                None
            )
            && material_blocker(
                &context,
                "STOCK",
                "/supplies/0/transport_ref",
                Some("business_inputs.material_missing_transport_module")
            ),
        json!({"object":current,"context":context})
    );
    let undo = state.dispatch_ui("main", smoke_request("material-undo", project.clone(), "apply_commands",
        json!({"base_revision":edited["outcome"]["data"]["new_revision"],"commands":[{"kind":"undo"}]})));
    let undone = business_object(state, &project, "scenario", "STOCK");
    let context = material_context(state, &project);
    check!(
        "material_undo_restores_input_graph_and_recomputed_issues",
        undo["outcome"]["status"] == "ok"
            && context["outcome"]["status"] == "ok"
            && material_input_projection(&undone["outcome"]["data"]["data"])
                == material_input_projection(&positive)
            && !material_blocker(&context, "STOCK", "/", None),
        json!({"commit":undo,"object":undone,"context":context})
    );
    let redo = state.dispatch_ui("main", smoke_request("material-redo", project.clone(), "apply_commands",
        json!({"base_revision":undo["outcome"]["data"]["new_revision"],"commands":[{"kind":"redo"}]})));
    let frozen = business_object(state, &project, "scenario", "STOCK");
    let context = material_context(state, &project);
    check!(
        "material_redo_restores_blocked_input_and_fresh_scenario_revision",
        redo["outcome"]["status"] == "ok"
            && material_input_projection(&frozen["outcome"]["data"]["data"])
                == material_input_projection(&bad)
            && frozen["outcome"]["data"]["data"]["revision"].as_u64()
                > undone["outcome"]["data"]["data"]["revision"].as_u64()
            && material_blocker(
                &context,
                "STOCK",
                "/supplies/0/transport_ref",
                Some("business_inputs.material_missing_transport_module")
            ),
        json!({"commit":redo,"object":frozen,"context":context})
    );
    io!(
        "material_save_directory",
        std::fs::create_dir(&saved_directory)
    );
    let selected = io!(
        "material_save_selection",
        state.register_directory(FilePurpose::SaveProject, Some(saved_directory.clone()))
    );
    let frozen_revision = redo["outcome"]["data"]["new_revision"].clone();
    let save = state.dispatch_ui(
        "main",
        smoke_request(
            "material-save-draft",
            project.clone(),
            "save_project",
            json!({"revision":frozen_revision,"selected_directory_ref":selected}),
        ),
    );
    check!(
        "material_frozen_draft_save_admitted",
        save["outcome"]["status"] == "ok",
        save
    );
    let mut corrected = frozen["outcome"]["data"]["data"].clone();
    corrected["initial_conditions"] = positive["initial_conditions"].clone();
    corrected["supplies"] = positive["supplies"].clone();
    let edit = business_edit_scenario(state, &project, corrected);
    check!(
        "material_newer_input_edit_is_independent_of_frozen_save",
        edit["outcome"]["status"] == "ok",
        edit
    );
    let saved = state.wait_for_task(project.clone(), save["outcome"]["data"]["id"].clone());
    check!(
        "material_frozen_draft_save_reaches_real_completion",
        saved["outcome"]["data"]["task"]["state"] == "completed"
            && saved["outcome"]["data"]["result"]["data"]["saved_revision"] == frozen_revision,
        saved
    );
    let disk = io!(
        "material_read_actual_saved_scenario",
        business_archive_scenario(&saved_directory, "STOCK")
    );
    let current = business_object(state, &project, "scenario", "STOCK");
    let context = material_context(state, &project);
    check!(
        "material_save_keeps_frozen_blocked_input_while_newer_input_stays_dirty",
        disk["data"]["project_revision"] == frozen_revision
            && material_input_projection(&disk["data"]) == material_input_projection(&bad)
            && material_input_projection(&current["outcome"]["data"]["data"])
                == material_input_projection(&positive)
            && context["outcome"]["data"]["project"]["dirty"] == true,
        json!({"disk":disk,"current":current,"context":context})
    );
    let files = io!(
        "material_saved_archive_files",
        smoke_files(&saved_directory)
    );
    io!(
        "material_relocated_directory",
        std::fs::create_dir(&relocated)
    );
    for (path, bytes) in &files {
        let target = relocated.join(path);
        io!(
            "material_relocated_parent",
            std::fs::create_dir_all(target.parent().expect("archive child"))
        );
        io!("material_copy_archive", std::fs::write(target, bytes));
    }
    let selected = io!(
        "material_reopen_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(relocated.clone()))
    );
    let import = state.dispatch_ui("main", smoke_request("material-reopen-draft", Value::Null, "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));
    check!(
        "material_relocated_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task.clone());
    check!(
        "material_relocated_parser_keeps_typed_missing_transport_issue",
        imported["outcome"]["data"]["task"]["state"] == "completed"
            && imported["outcome"]["data"]["result"]["data"]["missing"]
                .as_array()
                .is_some_and(|issues| issues.iter().any(|issue| issue["code"]
                    == "business_inputs.material_missing_transport_module"
                    && issue["object_ref"] == json!({"kind":"scenario","id":"STOCK"})
                    && issue["field_path"] == "/supplies/0/transport_ref")),
        imported
    );
    let refused = state.dispatch_ui(
        "main",
        smoke_request(
            "material-refuse-unaccepted-gap",
            Value::Null,
            "apply_commands",
            business_adoption(task.clone(), &imported),
        ),
    );
    let current = business_object(state, &project, "scenario", "STOCK");
    check!(
        "material_import_without_explicit_gap_acceptance_preserves_current_input",
        refused["outcome"]["status"] == "error"
            && material_input_projection(&current["outcome"]["data"]["data"])
                == material_input_projection(&positive),
        json!({"refused":refused,"current":current})
    );
    let mut adoption = business_adoption(task, &imported);
    adoption["commands"][0]["data"]["choices"]["accept_missing_as_draft"] = json!(true);
    let opened = state.dispatch_ui(
        "main",
        smoke_request(
            "material-accept-draft-open",
            Value::Null,
            "apply_commands",
            adoption,
        ),
    );
    check!(
        "material_relocated_draft_is_explicitly_accepted",
        opened["outcome"]["status"] == "ok",
        opened
    );
    let reopened = business_object(state, &project, "scenario", "STOCK");
    let context = material_context(state, &project);
    check!(
        "material_reopened_input_and_use_blockers_are_preserved",
        material_input_projection(&reopened["outcome"]["data"]["data"])
            == material_input_projection(&bad)
            && material_blocker(
                &context,
                "STOCK",
                "/initial_conditions/material_allocations",
                None
            )
            && material_blocker(
                &context,
                "STOCK",
                "/supplies/0/transport_ref",
                Some("business_inputs.material_missing_transport_module")
            ),
        json!({"object":reopened,"context":context})
    );
    let mut corrected = reopened["outcome"]["data"]["data"].clone();
    corrected["initial_conditions"] = positive["initial_conditions"].clone();
    corrected["supplies"] = positive["supplies"].clone();
    let edit = business_edit_scenario(state, &project, corrected);
    check!(
        "material_reopened_draft_edit_commits",
        edit["outcome"]["status"] == "ok",
        edit
    );
    let undo = state.dispatch_ui("main", smoke_request("material-reopened-undo", project.clone(), "apply_commands",
        json!({"base_revision":edit["outcome"]["data"]["new_revision"],"commands":[{"kind":"undo"}]})));
    let current = business_object(state, &project, "scenario", "STOCK");
    let context = material_context(state, &project);
    check!(
        "material_reopened_undo_restores_blocked_draft",
        undo["outcome"]["status"] == "ok"
            && material_input_projection(&current["outcome"]["data"]["data"])
                == material_input_projection(&bad)
            && material_blocker(
                &context,
                "STOCK",
                "/supplies/0/transport_ref",
                Some("business_inputs.material_missing_transport_module")
            ),
        json!({"commit":undo,"object":current,"context":context})
    );
    let redo = state.dispatch_ui("main", smoke_request("material-reopened-redo", project.clone(), "apply_commands",
        json!({"base_revision":undo["outcome"]["data"]["new_revision"],"commands":[{"kind":"redo"}]})));
    let current = business_object(state, &project, "scenario", "STOCK");
    let context = material_context(state, &project);
    check!(
        "material_reopened_redo_restores_declared_positive_input",
        redo["outcome"]["status"] == "ok"
            && context["outcome"]["status"] == "ok"
            && material_input_projection(&current["outcome"]["data"]["data"])
                == material_input_projection(&positive)
            && !material_blocker(&context, "STOCK", "/", None),
        json!({"commit":redo,"object":current,"context":context})
    );
    let source_after = io!(
        "material_source_files_after_history",
        smoke_files(&saved_directory)
    );
    let relocated_after = io!(
        "material_relocated_files_after_history",
        smoke_files(&relocated)
    );
    check!(
        "material_history_never_changes_saved_or_relocated_archives",
        source_after == files && relocated_after == files,
        json!({"files":files.len(),"evidence_kind":"actual_archive_bytes_not_runtime_inventory"})
    );
    io!(
        "material_cleanup_owned_directory",
        remove_smoke_directory(&temp, &root)
    );
    json!({"kind":"P0_01_host_smoke","passed":true,"checks":checks,
        "material_evidence_scope":"declared Scenario input, blocking issues and actual file/history behavior; no Engine, inventory query or allocation execution"})
}

fn replanned_input_smoke_report(state: &ProjectApiState) -> Value {
    let previous = material_full_smoke_report(state);
    if previous["passed"] != true {
        return previous;
    }
    let mut checks = previous["checks"].as_array().expect("owned checks").clone();
    macro_rules! check {
        ($name:literal, $condition:expr, $response:expr) => {{
            let passed = $condition;
            checks.push(json!({"name":$name,"passed":passed,"response":$response}));
            if !passed { return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    macro_rules! attempt {
        ($name:literal, $operation:expr) => { match $operation {
            Ok(value) => value,
            Err(error) => { checks.push(json!({"name":$name,"passed":false,"error":format!("{error:?}")})); return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    let context = material_context(state, &Value::Null);
    let project = context["outcome"]["data"]["project"]["id"].clone();
    let source = business_object(state, &project, "scenario", "STOCK");
    let source: sf_domain::ScenarioInput = attempt!(
        "replan_typed_current_source",
        serde_json::from_value(source["outcome"]["data"]["data"].clone())
    );
    let before = attempt!("replan_original_source", serde_json::to_value(&source));
    let revision = attempt!(
        "replan_current_project_revision",
        serde_json::from_value(context["outcome"]["data"]["project"]["revision"].clone())
    );
    let candidate = attempt!(
        "replan_existing_domain_consumer",
        sf_domain::fork_without_unstarted_demands(
            &source,
            sf_domain::ScenarioId::try_from("STOCK-CANCEL-A".to_owned()).unwrap(),
            revision,
            &[ObjectId::try_from("ORDER-A".to_owned()).unwrap()],
            &[],
        )
    );
    let expected = attempt!(
        "replan_serialized_candidate",
        serde_json::to_value(&candidate)
    );
    check!(
        "replan_cancellation_preserves_physical_inputs_and_source",
        serde_json::to_value(&source).unwrap() == before
            && expected["initial_conditions"]["items"] == before["initial_conditions"]["items"]
            && expected["arrivals"] == before["arrivals"]
            && expected["supplies"] == before["supplies"]
            && expected["orders"]
                .as_array()
                .is_some_and(|orders| orders.len() == 1 && orders[0]["id"] == "ORDER-B")
            && expected["production_tasks"]
                .as_array()
                .is_some_and(|tasks| tasks.len() == 1 && tasks[0]["id"] == "TASK-B")
            && expected["initial_conditions"]["material_allocations"]
                .as_array()
                .is_some_and(|allocations| {
                    allocations.len() == 2
                        && allocations
                            .iter()
                            .all(|a| a["demand_ref"] != "TASK-A" && a["demand_ref"] != "ORDER-A")
                }),
        json!({"source":before,"candidate":expected})
    );
    let put = business_edit_scenario(state, &project, expected.clone());
    let current = business_object(state, &project, "scenario", "STOCK-CANCEL-A");
    let original = business_object(state, &project, "scenario", "STOCK");
    check!(
        "replan_existing_put_commits_independent_scenario_once",
        put["outcome"]["status"] == "ok"
            && current["outcome"]["data"]["data"]["revision"] == 1
            && material_input_projection(&current["outcome"]["data"]["data"])
                == material_input_projection(&expected)
            && material_input_projection(&original["outcome"]["data"]["data"])
                == material_input_projection(&before),
        json!({"commit":put,"candidate":current,"original":original})
    );
    let (temp, root) = attempt!("replan_owned_directory", smoke_directory());
    let saved = root.join("saved");
    let relocated = root.join("relocated");
    attempt!("replan_saved_directory", std::fs::create_dir(&saved));
    let selected = attempt!(
        "replan_selected_directory",
        state.register_directory(FilePurpose::SaveProject, Some(saved.clone()))
    );
    let save = state.dispatch_ui("main", smoke_request("replan-save", project.clone(), "save_project",
        json!({"revision":put["outcome"]["data"]["new_revision"],"selected_directory_ref":selected})));
    check!(
        "replan_save_admitted",
        save["outcome"]["status"] == "ok",
        save
    );
    let saved_task = state.wait_for_task(project.clone(), save["outcome"]["data"]["id"].clone());
    let disk_source = attempt!(
        "replan_saved_source",
        business_archive_scenario(&saved, "STOCK")
    );
    let disk_candidate = attempt!(
        "replan_saved_candidate",
        business_archive_scenario(&saved, "STOCK-CANCEL-A")
    );
    check!(
        "replan_actual_archive_keeps_original_and_cancelled_plan",
        saved_task["outcome"]["data"]["task"]["state"] == "completed"
            && material_input_projection(&disk_source["data"])
                == material_input_projection(&before)
            && material_input_projection(&disk_candidate["data"])
                == material_input_projection(&expected),
        json!({"task":saved_task,"source":disk_source,"candidate":disk_candidate})
    );
    let files = attempt!("replan_archive_files", smoke_files(&saved));
    attempt!(
        "replan_relocated_directory",
        std::fs::create_dir(&relocated)
    );
    for (path, bytes) in &files {
        let target = relocated.join(path);
        attempt!(
            "replan_relocated_parent",
            std::fs::create_dir_all(target.parent().expect("archive child"))
        );
        attempt!("replan_copy_archive", std::fs::write(target, bytes));
    }
    let selected = attempt!(
        "replan_import_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(relocated.clone()))
    );
    let import = state.dispatch_ui("main", smoke_request("replan-import", Value::Null, "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));
    check!(
        "replan_relocated_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task.clone());
    check!(
        "replan_relocated_import_completed",
        imported["outcome"]["data"]["task"]["state"] == "completed",
        imported
    );
    let mut adoption = business_adoption(task, &imported);
    // This input fixture intentionally retains a missing transport module.
    adoption["commands"][0]["data"]["choices"]["accept_missing_as_draft"] = json!(true);
    let opened = state.dispatch_ui(
        "main",
        smoke_request("replan-open", Value::Null, "apply_commands", adoption),
    );
    let reopened = business_object(state, &project, "scenario", "STOCK-CANCEL-A");
    let original = business_object(state, &project, "scenario", "STOCK");
    check!(
        "replan_relocated_same_host_open_preserves_both_inputs",
        opened["outcome"]["status"] == "ok"
            && material_input_projection(&reopened["outcome"]["data"]["data"])
                == material_input_projection(&expected)
            && material_input_projection(&original["outcome"]["data"]["data"])
                == material_input_projection(&before),
        json!({"open":opened,"candidate":reopened,"original":original})
    );
    attempt!(
        "replan_cleanup_owned_directory",
        remove_smoke_directory(&temp, &root)
    );
    json!({"kind":"P0_01_host_smoke","passed":true,"checks":checks,
        "replan_evidence_scope":"actual same-Host Put/save/relocated Open of independent cancellation input; running Engine/WIP preservation is tested by production integration"})
}

fn seed_production_smoke_archive(directory: &Path) -> std::io::Result<()> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../packages/commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .map_err(std::io::Error::other)?;
    seed_typed_production_archive(directory, &fixture)
}

fn seed_typed_production_archive(directory: &Path, fixture: &Value) -> std::io::Result<()> {
    let _: sf_domain::ProjectState =
        serde_json::from_value(fixture["project"].clone()).map_err(std::io::Error::other)?;
    let _: sf_domain::ScenarioInput =
        serde_json::from_value(fixture["scenario"].clone()).map_err(std::io::Error::other)?;
    let write = |path: &str, value: Value| {
        std::fs::write(
            directory.join(path),
            serde_json::to_vec(&value).map_err(std::io::Error::other)?,
        )
    };
    let mut documents = vec![];
    for (role, kind, id, path, value) in [
        (
            "active_project",
            "project",
            "P1",
            "active.json",
            &fixture["project"],
        ),
        (
            "scenario_input",
            "scenario",
            "LOCAL",
            "scenario.json",
            &fixture["scenario"],
        ),
    ] {
        write(path, json!({"schema_version":"1.0","data":value}))?;
        documents.push(json!({"role":role,"object_ref":{"kind":kind,"id":id},
            "revision":1,"module_ref":null,"file_ref":{"path":path,"kind":kind},"required":true}));
    }
    for (index, definition) in fixture["definitions"]
        .as_array()
        .expect("formal fixture definitions")
        .iter()
        .enumerate()
    {
        let _: sf_domain::ModuleDefinition =
            serde_json::from_value(definition.clone()).map_err(std::io::Error::other)?;
        let path = format!("module-{index}.json");
        write(
            &path,
            json!({"schema_version":"1.0","data":{"definition":definition,
            "assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]}}),
        )?;
        documents.push(
            json!({"role":"module_definition","object_ref":{"kind":"module","id":definition["id"]},
            "revision":null,"module_ref":{"id":definition["id"],"version":definition["version"]},
            "file_ref":{"path":path,"kind":"module"},"required":true}),
        );
    }
    write(
        "project.json",
        json!({"schema_version":"1.0","data":{
        "project_id":"P1","project_revision":1,"title":fixture["project"]["title"],"generation":1,
        "documents":documents,"module_refs":fixture["project"]["module_refs"],"assets":[],"missing":[],
        "restart_policy":"restart_from_initial_input"}}),
    )
}

fn production_smoke_report(state: &ProjectApiState) -> Value {
    let previous = replanned_input_smoke_report(state);
    if previous["passed"] != true {
        return previous;
    }
    let mut checks = previous["checks"].as_array().expect("owned checks").clone();
    macro_rules! check {
        ($name:literal, $condition:expr, $response:expr) => {{
            let passed = $condition;
            checks.push(json!({"name":$name,"passed":passed,"response":$response}));
            if !passed { return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    macro_rules! attempt {
        ($name:literal, $operation:expr) => { match $operation {
            Ok(value) => value,
            Err(error) => { checks.push(json!({"name":$name,"passed":false,"error":format!("{error:?}")})); return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    let (temp, root) = attempt!("production_owned_directory", smoke_directory());
    let seed = root.join("seed");
    let saved = root.join("saved");
    let relocated = root.join("relocated");
    attempt!("production_seed_directory", std::fs::create_dir(&seed));
    attempt!(
        "production_formal_typed_seed",
        seed_production_smoke_archive(&seed)
    );
    let selected = attempt!(
        "production_import_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(seed))
    );
    let import = state.dispatch_ui("main", smoke_request("production-import", Value::Null, "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));
    check!(
        "production_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task.clone());
    check!(
        "production_actual_seed_import_completed",
        imported["outcome"]["data"]["task"]["state"] == "completed",
        imported
    );
    let opened = state.dispatch_ui(
        "main",
        smoke_request(
            "production-open",
            Value::Null,
            "apply_commands",
            business_adoption(task, &imported),
        ),
    );
    check!(
        "production_seed_opened_in_same_managed_host",
        opened["outcome"]["status"] == "ok",
        opened
    );
    let project = opened["outcome"]["data"]["project_id"].clone();
    let source = business_object(state, &project, "scenario", "LOCAL");
    let mut incomplete = source["outcome"]["data"]["data"].clone();
    incomplete["id"] = json!("OTHER");
    incomplete["title"] = json!("independent incomplete input must not block LOCAL");
    incomplete["orders"][0]["quantity"] = Value::Null;
    let saved_incomplete = business_edit_scenario(state, &project, incomplete);
    check!(
        "production_independent_incomplete_input_remains_a_saved_draft",
        saved_incomplete["outcome"]["status"] == "ok",
        saved_incomplete
    );
    let source = business_object(state, &project, "scenario", "LOCAL");
    let context = material_context(state, &project);
    let base = context["outcome"]["data"]["project"]["revision"].clone();
    check!(
        "production_other_input_diagnostic_is_preserved_in_project_context",
        context["outcome"]["data"]["issues"]
            .as_array()
            .is_some_and(|issues| issues.iter().any(|issue| issue["code"]
                == "business_inputs.missing_order_quantity"
                && issue["object_ref"] == json!({"kind":"scenario","id":"OTHER"}))),
        context
    );
    let blocked = state.dispatch_ui("main", smoke_request("production-other-run", project.clone(), "run_scenario",
        json!({"scenario_id":"OTHER","input_revision":base,"observation_window":source["outcome"]["data"]["data"]["window"],
            "intent":{"kind":"project_scenario"}})));
    check!(
        "production_incomplete_other_input_is_still_blocked",
        blocked["outcome"]["status"] == "error",
        blocked
    );
    let handle = state.dispatch_ui("main", smoke_request("production-run", project.clone(), "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":base,"observation_window":source["outcome"]["data"]["data"]["window"],
            "intent":{"kind":"project_scenario"}})));
    check!(
        "production_real_worker_admitted_as_queued",
        handle["outcome"]["status"] == "ok" && handle["outcome"]["data"]["state"] == "queued",
        handle
    );
    let mut changed = source["outcome"]["data"]["data"].clone();
    changed["title"] = json!("new input while the frozen production Run is independent");
    let edit = business_edit_scenario(state, &project, changed.clone());
    check!(
        "production_new_revision_does_not_replace_frozen_run_input",
        edit["outcome"]["status"] == "ok"
            && edit["outcome"]["data"]["new_revision"].as_u64() == base.as_u64().map(|r| r + 1),
        edit
    );
    let task = state.wait_for_task(project.clone(), handle["outcome"]["data"]["id"].clone());
    check!(
        "production_actual_engine_task_completed",
        task["outcome"]["data"]["task"]["state"] == "completed"
            && task["outcome"]["data"]["result"]["kind"] == "run",
        task
    );
    let run_id = task["outcome"]["data"]["result"]["data"]["run_id"].clone();
    let full_request = || {
        smoke_request(
            "production-full",
            project.clone(),
            "get_run",
            json!({"run_id":run_id,"query":{"kind":"full_result"}}),
        )
    };
    let full = state.dispatch_ui("main", full_request());
    let result = full["outcome"]["data"]["data"].clone();
    check!(
        "production_result_is_frozen_actual_one_package_at_75000",
        full["outcome"]["status"] == "ok"
            && full["outcome"]["data"]["kind"] == "full"
            && result["evidence_origin"] == "actual_software"
            && result["input_ref"]["project_revision"] == base
            && result["acceptance_events"]
                .as_array()
                .is_some_and(|events| events.len() == 1 && events[0]["at_ms"] == 75000)
            && result["whole_window_computed"] == true
            && result["goal_met"] == false,
        full
    );
    let history_request = || {
        smoke_request(
            "production-history",
            project.clone(),
            "get_run",
            json!({"run_id":run_id,
        "query":{"kind":"objects_at","data":{"at_ms":0,"objects":[],"page":{"after_index":null,"limit":100}}}}),
        )
    };
    let history = state.dispatch_ui("main", history_request());
    check!(
        "production_history_reads_computed_start_instead_of_final_items",
        history["outcome"]["status"] == "ok"
            && history["outcome"]["data"]["kind"] == "objects"
            && history["outcome"]["data"]["data"]["at_ms"] == 0
            && history["outcome"]["data"]["data"]["objects"]
                .as_array()
                .is_some_and(|objects| !objects.is_empty()
                    && objects.iter().all(|object| object["data"]["contents"]
                        .as_array()
                        .is_some_and(Vec::is_empty))),
        history
    );
    attempt!("production_saved_directory", std::fs::create_dir(&saved));
    let selected = attempt!(
        "production_save_selection",
        state.register_directory(FilePurpose::SaveProject, Some(saved.clone()))
    );
    let save = state.dispatch_ui("main", smoke_request("production-save", project.clone(), "save_project",
        json!({"revision":edit["outcome"]["data"]["new_revision"],"selected_directory_ref":selected})));
    check!(
        "production_save_admitted",
        save["outcome"]["status"] == "ok",
        save
    );
    let saved_task = state.wait_for_task(project.clone(), save["outcome"]["data"]["id"].clone());
    check!(
        "production_frozen_run_and_new_input_saved",
        saved_task["outcome"]["data"]["task"]["state"] == "completed",
        saved_task
    );
    let files = attempt!("production_actual_archive_files", smoke_files(&saved));
    attempt!(
        "production_relocated_directory",
        std::fs::create_dir(&relocated)
    );
    for (path, bytes) in &files {
        let target = relocated.join(path);
        attempt!(
            "production_relocated_parent",
            std::fs::create_dir_all(target.parent().expect("archive child"))
        );
        attempt!("production_copy_archive", std::fs::write(target, bytes));
    }
    let selected = attempt!(
        "production_reopen_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(relocated.clone()))
    );
    let import = state.dispatch_ui("main", smoke_request("production-reopen", Value::Null, "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));
    check!(
        "production_relocated_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task.clone());
    check!(
        "production_relocated_import_completed",
        imported["outcome"]["data"]["task"]["state"] == "completed",
        imported
    );
    let mut adoption = business_adoption(task.clone(), &imported);
    let blocked_open = state.dispatch_ui(
        "main",
        smoke_request(
            "production-reopen-needs-choice",
            Value::Null,
            "apply_commands",
            adoption.clone(),
        ),
    );
    check!(
        "production_incomplete_reopen_requires_explicit_draft_choice",
        blocked_open["outcome"]["status"] == "error"
            && blocked_open["outcome"]["data"]["code"] == "InvalidInput"
            && state.dispatch_ui("main", full_request())["outcome"]["data"]["data"] == result,
        blocked_open
    );
    adoption["commands"][0]["data"]["choices"]["accept_missing_as_draft"] = json!(true);
    let opened = state.dispatch_ui(
        "main",
        smoke_request(
            "production-reopen-adopt",
            Value::Null,
            "apply_commands",
            adoption,
        ),
    );
    check!(
        "production_relocated_open_completed",
        opened["outcome"]["status"] == "ok",
        opened
    );
    let reopened = state.dispatch_ui("main", full_request());
    let current = business_object(state, &project, "scenario", "LOCAL");
    check!(
        "production_relocated_old_run_and_new_input_remain_separate",
        reopened["outcome"]["status"] == "ok"
            && reopened["outcome"]["data"]["data"] == result
            && current["outcome"]["data"]["data"]["title"] == changed["title"],
        json!({"run":reopened,"current_input":current})
    );
    let reopened_history = state.dispatch_ui("main", history_request());
    check!(
        "production_relocated_run_history_uses_saved_actual_history",
        reopened_history["outcome"]["status"] == "ok"
            && reopened_history["outcome"]["data"] == history["outcome"]["data"],
        reopened_history
    );
    let mut legacy = current["outcome"]["data"]["data"].clone();
    legacy["id"] = json!("MANUAL-GOAL");
    legacy["revision"] = json!(1);
    legacy["orders"] = json!([]);
    legacy["production_tasks"][0]["order_allocations"] = json!([]);
    let mut pcb = legacy["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("manual-goal-pcb-002");
    legacy["arrivals"] = json!([{"id":"manual-goal-arrival","at_ms":90000,
        "destination":"pcbs-A","items":[pcb],"release_policy":"unchecked_allowed"}]);
    let goal_fixture: Value = attempt!(
        "production_manual_goal_typed_source",
        serde_json::from_str(include_str!(
            "../../../../tools/type-contract-proof/api-fixtures/check_factory_draft.request.json"
        ))
    );
    let mut goal = goal_fixture["params"]["generation_goal"].clone();
    goal["industry"] = json!("explicit finite native production check");
    goal["product_family"] = json!("declared package");
    goal["constraints"] = json!([]);
    goal["acceptance_scope"] = json!({"product_ref":{"id":"PACKAGE","revision":"r1"},
        "unit":"piece","destination":"packages-B","stage":"factory_released"});
    goal["accepted_quantity_per_day"] =
        json!({"value":{"coefficient":1,"decimal_places":0},"unit":"piece"});
    goal["observation_window"] = legacy["window"].clone();
    legacy["acceptance_goal"] = goal.clone();
    let goal_edit = business_edit_scenario(state, &project, legacy.clone());
    check!(
        "production_manual_goal_is_an_explicit_saved_scenario_without_synthetic_generation",
        goal_edit["outcome"]["status"] == "ok"
            && business_object(state, &project, "scenario", "MANUAL-GOAL")["outcome"]["data"]
                ["data"]["acceptance_goal"]
                == goal
            && material_context(state, &project)["outcome"]["data"]["project"]
                ["generation_context"]
                .is_null(),
        goal_edit
    );
    let run_manual = || {
        let revision =
            material_context(state, &project)["outcome"]["data"]["project"]["revision"].clone();
        state.dispatch_ui(
            "main",
            smoke_request(
                "manual-goal-run",
                project.clone(),
                "run_scenario",
                json!({"scenario_id":"MANUAL-GOAL","input_revision":revision,
                "observation_window":legacy["window"],"intent":{"kind":"project_scenario"}}),
            ),
        )
    };
    let goal_handle = run_manual();
    check!(
        "production_manual_goal_actual_run_admitted",
        goal_handle["outcome"]["status"] == "ok",
        goal_handle
    );
    let goal_task = state.wait_for_task(
        project.clone(),
        goal_handle["outcome"]["data"]["id"].clone(),
    );
    check!(
        "production_manual_goal_actual_worker_completed",
        goal_task["outcome"]["data"]["task"]["state"] == "completed",
        goal_task
    );
    let goal_run_id = goal_task["outcome"]["data"]["result"]["data"]["run_id"].clone();
    let goal_result_request = || {
        smoke_request(
            "manual-goal-full",
            project.clone(),
            "get_run",
            json!({"run_id":goal_run_id,"query":{"kind":"full_result"}}),
        )
    };
    let goal_result =
        state.dispatch_ui("main", goal_result_request())["outcome"]["data"]["data"].clone();
    check!(
        "production_manual_goal_accepts_two_real_items_without_orders_or_quantity_cap",
        goal_result["goal_met"] == true
            && goal_result["acceptance_events"]
                .as_array()
                .is_some_and(|events| events.len() == 2
                    && events[0]["at_ms"] == 75000
                    && events[1]["at_ms"] == 165000
                    && events.iter().all(|event| event["order_id"].is_null()
                        && event["item_refs"]
                            .as_array()
                            .is_some_and(|items| !items.is_empty())))
            && business_object(state, &project, "scenario", "MANUAL-GOAL")["outcome"]["data"]
                ["data"]["orders"]
                .as_array()
                .is_some_and(Vec::is_empty),
        goal_result
    );
    let mut higher_goal =
        business_object(state, &project, "scenario", "MANUAL-GOAL")["outcome"]["data"]["data"]
            .clone();
    higher_goal["acceptance_goal"]["accepted_quantity_per_day"]["value"]["coefficient"] = json!(3);
    let higher_edit = business_edit_scenario(state, &project, higher_goal);
    check!(
        "production_manual_goal_later_edit_preserves_both_prior_frozen_runs",
        higher_edit["outcome"]["status"] == "ok"
            && state.dispatch_ui("main", goal_result_request())["outcome"]["data"]["data"]
                == goal_result
            && state.dispatch_ui("main", full_request())["outcome"]["data"]["data"] == result,
        higher_edit
    );
    let higher_handle = run_manual();
    check!(
        "production_manual_goal_higher_run_admitted",
        higher_handle["outcome"]["status"] == "ok",
        higher_handle
    );
    let higher_task = state.wait_for_task(
        project.clone(),
        higher_handle["outcome"]["data"]["id"].clone(),
    );
    let higher_result = state.dispatch_ui("main", smoke_request("manual-higher-result", project.clone(), "get_run",
        json!({"run_id":higher_task["outcome"]["data"]["result"]["data"]["run_id"],"query":{"kind":"full_result"}})));
    check!(
        "production_manual_goal_higher_target_does_not_fabricate_accepted_output",
        higher_task["outcome"]["data"]["task"]["state"] == "completed"
            && higher_result["outcome"]["data"]["data"]["goal_met"] == false
            && higher_result["outcome"]["data"]["data"]["acceptance_events"]
                == goal_result["acceptance_events"],
        higher_result
    );
    let selected = attempt!(
        "production_manual_goal_save_selection",
        state.register_directory(FilePurpose::SaveProject, Some(saved.clone()))
    );
    let goal_save = state.dispatch_ui("main", smoke_request("manual-goal-save", project.clone(), "save_project",
        json!({"revision":higher_edit["outcome"]["data"]["new_revision"],"selected_directory_ref":selected})));
    check!(
        "production_manual_goal_save_admitted",
        goal_save["outcome"]["status"] == "ok",
        goal_save
    );
    let goal_saved =
        state.wait_for_task(project.clone(), goal_save["outcome"]["data"]["id"].clone());
    check!(
        "production_manual_goal_actual_archive_completed",
        goal_saved["outcome"]["data"]["task"]["state"] == "completed",
        goal_saved
    );
    let goal_files = attempt!("production_manual_goal_archive_files", smoke_files(&saved));
    let frozen_goal = goal_files
        .get(Path::new(
            goal_result["input_ref"]["frozen_scenario_file"]["path"]
                .as_str()
                .expect("real frozen Scenario file"),
        ))
        .and_then(|bytes| serde_json::from_slice::<Value>(bytes).ok());
    check!(
        "production_manual_goal_frozen_scenario_keeps_the_actual_original_target",
        frozen_goal
            .as_ref()
            .is_some_and(|document| document["data"]["acceptance_goal"] == goal),
        frozen_goal
    );
    let goal_relocated = root.join("goal-relocated");
    attempt!(
        "production_manual_goal_relocated_directory",
        std::fs::create_dir(&goal_relocated)
    );
    for (path, bytes) in &goal_files {
        let target = goal_relocated.join(path);
        attempt!(
            "production_manual_goal_relocated_parent",
            std::fs::create_dir_all(target.parent().expect("archive child"))
        );
        attempt!(
            "production_manual_goal_relocated_copy",
            std::fs::write(target, bytes)
        );
    }
    let selection = attempt!(
        "production_manual_goal_reopen_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(goal_relocated))
    );
    let goal_import = state.dispatch_ui("main", smoke_request("manual-goal-reopen", Value::Null, "import_package",
        json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null})));
    check!(
        "production_manual_goal_reopen_admitted",
        goal_import["outcome"]["status"] == "ok",
        goal_import
    );
    let goal_import_id = goal_import["outcome"]["data"]["id"].clone();
    let goal_imported = state.wait_for_task(Value::Null, goal_import_id.clone());
    check!(
        "production_manual_goal_relocated_import_completed",
        goal_imported["outcome"]["data"]["task"]["state"] == "completed",
        goal_imported
    );
    let mut goal_adoption = business_adoption(goal_import_id, &goal_imported);
    goal_adoption["commands"][0]["data"]["choices"]["accept_missing_as_draft"] = json!(true);
    let goal_opened = state.dispatch_ui(
        "main",
        smoke_request(
            "manual-goal-reopen-adopt",
            Value::Null,
            "apply_commands",
            goal_adoption,
        ),
    );
    check!(
        "production_manual_goal_relocated_open_keeps_current_and_frozen_goals_separate",
        goal_opened["outcome"]["status"] == "ok"
            && business_object(state, &project, "scenario", "MANUAL-GOAL")["outcome"]["data"]
                ["data"]["acceptance_goal"]["accepted_quantity_per_day"]["value"]["coefficient"]
                == 3
            && state.dispatch_ui("main", goal_result_request())["outcome"]["data"]["data"]
                == goal_result
            && material_context(state, &project)["outcome"]["data"]["project"]
                ["generation_context"]
                .is_null(),
        goal_opened
    );
    attempt!(
        "production_cleanup_owned_directory",
        remove_smoke_directory(&temp, &root)
    );
    json!({"kind":"P0_01_host_smoke","passed":true,"checks":checks,
        "production_evidence_scope":"same managed desktop Host and FIFO worker, actual unique Engine, frozen Run R plus newer draft, explicit manual finite-window goal without synthetic orders/generation metadata, actual archived history and relocated Open; no daily extrapolation"})
}

fn packaging_smoke_report(state: &ProjectApiState) -> Value {
    let previous = production_smoke_report(state);
    if previous["passed"] != true {
        return previous;
    }
    let mut checks = previous["checks"].as_array().expect("owned checks").clone();
    macro_rules! check {
        ($name:literal, $condition:expr, $response:expr) => {{
            let passed = $condition;
            checks.push(json!({"name":$name,"passed":passed,"response":$response}));
            if !passed { return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    macro_rules! attempt {
        ($name:literal, $operation:expr) => { match $operation {
            Ok(value) => value,
            Err(error) => { checks.push(json!({"name":$name,"passed":false,"error":format!("{error:?}")})); return json!({"kind":"P0_01_host_smoke","passed":false,"checks":checks}); }
        }};
    }
    let mut fixture: Value = attempt!(
        "packaging_production_fixture",
        serde_json::from_str(include_str!(
            "../../../../packages/commands/tests/fixtures/production/local-assembly-pack.json"
        ))
    );
    let declaration: Value = attempt!(
        "packaging_parameter_declarations",
        serde_json::from_str(include_str!(
            "../../../../packages/packaging/consumer-device.design.json"
        ))
    );
    fixture["definitions"][5]["parameters"] = json!(
        declaration["data"]["definition"]["parameters"]
            .as_array()
            .expect("frozen parameter declarations")
            .iter()
            .filter(|parameter| parameter["id"] == "packaging_job_id"
                || parameter["id"] == "packaging_job_revision")
            .cloned()
            .collect::<Vec<_>>()
    );
    fixture["project"]["devices"][1]["configuration"] = json!({
        "packaging_job_id":{"kind":"string","value":"NATIVE-PACKAGE"},
        "packaging_job_revision":{"kind":"integer","value":1}});
    let inputs: Value = attempt!(
        "packaging_full_job_fixture",
        serde_json::from_str(include_str!(
            "../../../../packages/commands/tests/fixtures/production/packaging-acceptance-cases.json"
        ))
    );
    let mut job = inputs["cases"]
        .as_array()
        .expect("frozen packaging inputs")
        .iter()
        .find(|case| case["id"] == "COVERAGE-restricted_folded_container")
        .expect("explicit full restricted folded container")["job"]["data"]
        .clone();
    job["id"] = json!("NATIVE-PACKAGE");
    job["revision"] = json!(1);
    job["subject_ref"] = json!({"id":"PACKAGE","revision":"r1"});
    job["conditions"]["product_envelope"]["subject"] = job["subject_ref"].clone();
    let (temp, root) = attempt!("packaging_owned_directory", smoke_directory());
    let seed = root.join("seed");
    let saved = root.join("saved");
    let relocated = root.join("relocated");
    attempt!("packaging_seed_directory", std::fs::create_dir(&seed));
    attempt!(
        "packaging_typed_production_seed",
        seed_typed_production_archive(&seed, &fixture)
    );
    let selected = attempt!(
        "packaging_seed_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(seed))
    );
    let import = state.dispatch_ui("main", smoke_request("packaging-import", Value::Null, "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));
    check!(
        "packaging_import_admitted",
        import["outcome"]["status"] == "ok",
        import
    );
    let task_id = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task_id.clone());
    check!(
        "packaging_import_completed",
        imported["outcome"]["data"]["task"]["state"] == "completed",
        imported
    );
    let open = state.dispatch_ui(
        "main",
        smoke_request(
            "packaging-open",
            Value::Null,
            "apply_commands",
            business_adoption(task_id, &imported),
        ),
    );
    check!(
        "packaging_same_managed_host_opened",
        open["outcome"]["status"] == "ok",
        open
    );
    let project = open["outcome"]["data"]["project_id"].clone();
    let current_revision =
        || material_context(state, &project)["outcome"]["data"]["project"]["revision"].clone();
    let apply = |commands: Value| {
        state.dispatch_ui(
            "main",
            smoke_request(
                "packaging-edit",
                project.clone(),
                "apply_commands",
                json!({"base_revision":current_revision(),"commands":commands}),
            ),
        )
    };
    let stored = apply(json!([{"kind":"put_packaging_job","data":job}]));
    check!(
        "packaging_explicit_job_committed",
        stored["outcome"]["status"] == "ok",
        stored
    );
    let physical = business_object(state, &project, "scenario", "LOCAL")["outcome"]["data"]["data"]
        ["initial_conditions"]
        .clone();
    let compute = state.dispatch_ui("main", smoke_request("packaging-compute", project.clone(), "compute_packaging",
        json!({"packaging_job":job,"input_revision":current_revision(),"computation_budget":job["budget"]})));
    check!(
        "packaging_real_compute_worker_queued",
        compute["outcome"]["status"] == "ok" && compute["outcome"]["data"]["state"] == "queued",
        compute
    );
    let completed = state.wait_for_task(project.clone(), compute["outcome"]["data"]["id"].clone());
    let result = completed["outcome"]["data"]["result"]["data"].clone();
    check!(
        "packaging_real_kernel_produced_complete_pass",
        completed["outcome"]["data"]["task"]["state"] == "completed"
            && completed["outcome"]["data"]["result"]["kind"] == "packaging"
            && result["evidence_origin"] == "actual_software"
            && result["outcome"] == "pass"
            && result["job_ref"] == json!({"id":"NATIVE-PACKAGE","revision":1})
            && result["manufacturing_files"]
                .as_array()
                .is_some_and(|files| !files.is_empty()),
        completed
    );
    let artifact_id = result["artifact_id"].as_str().expect("actual artifact ID");
    let artifact =
        business_object(state, &project, "artifact", artifact_id)["outcome"]["data"]["data"]
            .clone();
    let project_before_ack =
        business_object(state, &project, "project", "P1")["outcome"]["data"]["data"].clone();
    check!(
        "packaging_computation_preserves_stock_and_does_not_acknowledge_or_bind",
        business_object(state, &project, "scenario", "LOCAL")["outcome"]["data"]["data"]["initial_conditions"]
            == physical
            && project_before_ack["acknowledgments"]
                .as_array()
                .is_some_and(Vec::is_empty)
            && project_before_ack["bindings"]
                .as_array()
                .is_some_and(Vec::is_empty),
        project_before_ack
    );
    let requested_scope = json!([
        "forming",
        "opening",
        "linear_extraction",
        "net_space",
        "manufacturing"
    ]);
    let adoption = |revision: Value| {
        json!({"artifact_id":artifact_id,"acceptance_ref":"NATIVE-ACK",
        "consumer_ref":{"kind":"device","id":"PACK-01"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},
        "requested_scope":requested_scope,"base_revision":revision})
    };
    let denied = state.dispatch_ui(
        "main",
        smoke_request(
            "packaging-before-ack",
            project.clone(),
            "adopt_artifact",
            adoption(current_revision()),
        ),
    );
    check!(
        "packaging_nominal_adoption_requires_separate_human_purpose",
        denied["outcome"]["status"] == "error",
        denied
    );
    let ack = apply(json!([{"kind":"record_artifact_acknowledgment","data":{
        "id":"NATIVE-ACK","artifact_ref":artifact_id,"accepted_subject":{"id":"PACKAGE","revision":"r1"},
        "accepted_scope":requested_scope,"decision_source":{"kind":"human_statement","data":"native integration input acknowledging the explicit purposes; not user acceptance"},
        "conditions":[],"recorded_at_iso8601":null}}]));
    check!(
        "packaging_ui_purpose_acknowledgment_commits_without_rewriting_result",
        ack["outcome"]["status"] == "ok"
            && business_object(state, &project, "artifact", artifact_id)["outcome"]["data"]["data"]
                == artifact
            && business_object(state, &project, "project", "P1")["outcome"]["data"]["data"]
                ["bindings"]
                .as_array()
                .is_some_and(Vec::is_empty),
        ack
    );
    let capabilities = state.dispatch_ui(
        "main",
        smoke_request(
            "packaging-capabilities",
            Value::Null,
            "describe_capabilities",
            json!({}),
        ),
    );
    check!(
        "packaging_acknowledged_business_purposes_expose_the_actual_adoption_path",
        capabilities["outcome"]["status"] == "ok"
            && capabilities["outcome"]["data"]["implemented_operations"]
                .as_array()
                .is_some_and(|operations| operations
                    .iter()
                    .any(|operation| operation["operation"] == "adopt_artifact")),
        capabilities
    );
    let adopted = state.dispatch_ui(
        "main",
        smoke_request(
            "packaging-adopt",
            project.clone(),
            "adopt_artifact",
            adoption(current_revision()),
        ),
    );
    check!(
        "packaging_separate_calculated_nominal_binding_commits",
        adopted["outcome"]["status"] == "ok"
            && adopted["outcome"]["data"]["binding"]["mode"] == "calculated_nominal",
        adopted
    );
    let run = || {
        state.dispatch_ui("main", smoke_request("packaging-factory-run", project.clone(), "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":current_revision(),"observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})))
    };
    let handle = run();
    check!(
        "packaging_bound_factory_run_queued",
        handle["outcome"]["status"] == "ok",
        handle
    );
    let finished = state.wait_for_task(project.clone(), handle["outcome"]["data"]["id"].clone());
    check!(
        "packaging_bound_factory_run_completed",
        finished["outcome"]["data"]["task"]["state"] == "completed",
        finished
    );
    let run_id = finished["outcome"]["data"]["result"]["data"]["run_id"].clone();
    let full_request = || {
        smoke_request(
            "packaging-full-run",
            project.clone(),
            "get_run",
            json!({"run_id":run_id,"query":{"kind":"full_result"}}),
        )
    };
    let full = state.dispatch_ui("main", full_request())["outcome"]["data"]["data"].clone();
    check!(
        "packaging_geometry_does_not_invent_factory_time_or_stock",
        full["acceptance_events"]
            .as_array()
            .is_some_and(|events| events.len() == 1 && events[0]["at_ms"] == 75000)
            && business_object(state, &project, "scenario", "LOCAL")["outcome"]["data"]["data"]["initial_conditions"]
                == physical,
        full
    );
    let accepted =
        business_object(state, &project, "project", "P1")["outcome"]["data"]["data"].clone();
    attempt!("packaging_saved_directory", std::fs::create_dir(&saved));
    let selected = attempt!(
        "packaging_save_selection",
        state.register_directory(FilePurpose::SaveProject, Some(saved.clone()))
    );
    let save = state.dispatch_ui(
        "main",
        smoke_request(
            "packaging-save",
            project.clone(),
            "save_project",
            json!({"revision":current_revision(),"selected_directory_ref":selected}),
        ),
    );
    check!(
        "packaging_archive_save_queued",
        save["outcome"]["status"] == "ok",
        save
    );
    let saved_task = state.wait_for_task(project.clone(), save["outcome"]["data"]["id"].clone());
    check!(
        "packaging_actual_inputs_results_ack_binding_and_run_saved",
        saved_task["outcome"]["data"]["task"]["state"] == "completed",
        saved_task
    );
    let files = attempt!("packaging_saved_files", smoke_files(&saved));
    check!(
        "packaging_saved_manufacturing_contains_actual_svg_bytes",
        files.iter().any(|(path, bytes)| path
            .extension()
            .is_some_and(|extension| extension == "svg")
            && std::str::from_utf8(bytes).is_ok_and(|svg| svg.contains("<svg"))),
        json!({"file_count":files.len()})
    );
    attempt!(
        "packaging_relocated_directory",
        std::fs::create_dir(&relocated)
    );
    for (path, bytes) in files {
        let target = relocated.join(path);
        attempt!(
            "packaging_relocated_parent",
            std::fs::create_dir_all(target.parent().expect("archive child"))
        );
        attempt!("packaging_relocated_file", std::fs::write(target, bytes));
    }
    let selected = attempt!(
        "packaging_reopen_selection",
        state.register_directory(FilePurpose::ImportPackage, Some(relocated))
    );
    let import = state.dispatch_ui("main", smoke_request("packaging-reopen", Value::Null, "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));
    check!(
        "packaging_relocated_import_queued",
        import["outcome"]["status"] == "ok",
        import
    );
    let task_id = import["outcome"]["data"]["id"].clone();
    let imported = state.wait_for_task(Value::Null, task_id.clone());
    check!(
        "packaging_relocated_import_completed",
        imported["outcome"]["data"]["task"]["state"] == "completed",
        imported
    );
    let reopened = state.dispatch_ui(
        "main",
        smoke_request(
            "packaging-reopen-adopt",
            Value::Null,
            "apply_commands",
            business_adoption(task_id, &imported),
        ),
    );
    check!(
        "packaging_relocated_open_committed",
        reopened["outcome"]["status"] == "ok",
        reopened
    );
    let restored =
        business_object(state, &project, "project", "P1")["outcome"]["data"]["data"].clone();
    check!(
        "packaging_relocated_result_purpose_binding_and_old_run_preserved",
        business_object(state, &project, "artifact", artifact_id)["outcome"]["data"]["data"]
            == artifact
            && restored["acknowledgments"] == accepted["acknowledgments"]
            && restored["bindings"] == accepted["bindings"]
            && state.dispatch_ui("main", full_request())["outcome"]["data"]["data"] == full,
        restored
    );
    let mut product =
        business_object(state, &project, "product", "PACKAGE")["outcome"]["data"]["data"].clone();
    product["revision"] = json!("r2");
    let edited = apply(json!([{"kind":"put_object","data":{"kind":"product","data":product}}]));
    check!(
        "packaging_product_change_keeps_old_artifact_and_blocks_old_binding_run",
        edited["outcome"]["status"] == "ok"
            && run()["outcome"]["status"] == "error"
            && business_object(state, &project, "artifact", artifact_id)["outcome"]["data"]["data"]
                == artifact,
        json!({"edit":edited,"context":material_context(state,&project)})
    );
    attempt!(
        "packaging_cleanup_owned_directory",
        remove_smoke_directory(&temp, &root)
    );
    json!({"kind":"P0_01_host_smoke","passed":true,"checks":checks,
        "packaging_evidence_scope":"same managed desktop Host and FIFO worker; actual kernel, explicit test UI purpose, separate nominal binding, unchanged physical input, actual factory Run and relocated archive; not human/product acceptance"})
}

/// Startup-only check. The caller supplies the state registered by app.manage;
/// this uses the command's dispatch path, not a second Host or business frontend.
pub fn smoke(state: &ProjectApiState) -> bool {
    let report = packaging_smoke_report(state);
    let passed = report["passed"] == true;
    println!("{report}");
    passed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_close_snapshot_is_main_only_and_uses_the_registered_host() {
        let state = ProjectApiState::new();
        assert!(matches!(state.desktop_close_state("render-task-1"), Err(ApiError { code: ErrorCode::PermissionDenied, .. })));
        let close = state.desktop_close_state("main").unwrap();
        assert!(close.project_id.is_none());
        assert!(!close.unsaved);
        assert!(close.active_tasks.is_empty());
        assert!(close.definition_revisions.is_empty());
    }

    #[test]
    fn desktop_close_final_admission_is_main_only_and_serializes_with_real_requests() {
        let state = ProjectApiState::new();
        let expected = state.desktop_close_state("main").unwrap();
        assert!(matches!(state.finish_desktop_close("render-task-1", &expected, false, || panic!("secondary window must not close main")), Err(ApiError { code: ErrorCode::PermissionDenied, .. })));
        assert!(matches!(state.finish_desktop_close("main", &expected, false, || Err("destroy refused".into())), Err(ApiError { code: ErrorCode::ComputationFailed, .. })));
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let closing = state.clone();
        let final_close = std::thread::spawn(move || closing.finish_desktop_close("main", &expected, false, || {
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(())
        }));
        entered_rx.recv().unwrap();
        let request_state = state.clone();
        let waiting_request = std::thread::spawn(move || {
            let request = serde_json::from_value(json!({"api_version":"0.1","request_id":"after-close","project_id":null,"scope":{"kind":"workspace"},"operation":"apply_commands","params":{"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"must not commit after close","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}})).unwrap();
            request_state.dispatch_session(&request_state.ui_session_id, request)
        });
        release_tx.send(()).unwrap();
        final_close.join().unwrap().unwrap();
        let response = waiting_request.join().unwrap();
        assert_eq!(response["outcome"]["status"], "error", "{response}");
        assert_eq!(response["outcome"]["data"]["code"], "InvalidInput", "{response}");
        assert!(state.desktop_close_state("main").unwrap().project_id.is_none());
    }

    #[test]
    fn stored_manufacturing_reader_uses_main_identity_and_rejects_paths_or_supplied_caller() {
        let value = json!({"project_id":"P","scope":{"kind":"project"},"scope_revision":9,"artifact_id":"A","file":{"kind":"asset","path":"professional/sheet.svg"}});
        let request: StoredManufacturingReadRequest =
            serde_json::from_value(value.clone()).unwrap();
        let state = ProjectApiState::new();
        assert!(matches!(
            state.read_stored_manufacturing_ui("render-task-1", &request),
            Err(ApiError {
                code: ErrorCode::PermissionDenied,
                ..
            })
        ));
        for field in ["path", "caller", "session_id", "window_label", "bytes"] {
            let mut forged = value.clone();
            forged[field] = json!("client supplied");
            assert!(
                serde_json::from_value::<StoredManufacturingReadRequest>(forged).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn completed_render_image_uses_same_managed_host_fifo_and_current_task_authority() {
        // Controlled backend pixels test only the real Native Host adapter/FIFO;
        // this deliberately does not construct a WebView or claim Native PNG.
        struct ControlledImage;
        impl sf_commands::RenderBackend for ControlledImage {
            fn supported_kinds(&self) -> Vec<sf_domain::RenderViewKind> {
                vec![sf_domain::RenderViewKind::LineCanvas]
            }
            fn render(
                &self,
                job: sf_commands::RenderJob,
                _: &std::sync::atomic::AtomicBool,
            ) -> Result<sf_commands::RenderedCapture, ApiError> {
                Ok(sf_commands::RenderedCapture { frame:job.frame,scene_id:"adapter-scene".to_owned().try_into().unwrap(),image_id:"adapter-image".to_owned().try_into().unwrap(),mime_type:"image/png".into(),bytes:include_bytes!("../../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png").to_vec(),visible_objects:vec![],issues:vec![] })
            }
        }
        let state = ProjectApiState::new();
        state.set_render_backend(Arc::new(ControlledImage)).unwrap();
        let created=state.dispatch_ui("main",smoke_request("adapter-create",Value::Null,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"render adapter","time_reference":{"label":"relative","origin_iso8601":null,"timezone":"Asia/Shanghai"},"departure":"stay"}}]})));
        assert_eq!(created["outcome"]["status"], "ok", "{created}");
        let project = created["outcome"]["data"]["project_id"].clone();
        let before = business_object(&state, &project, "project", project.as_str().unwrap());
        let params = json!({"revision":1,"view":{"kind":"line_canvas","canvas":"line_canvas","hall_id":null,"object_ref":null,"project_revision":1,"run_id":null,"view_time_ms":null,"lod":"detailed","camera":before["outcome"]["data"]["data"]["views"]["line_camera"]},"object_id":null,"preview":null,"delivery":"capture_only"});
        let render = smoke_request("adapter-render", project.clone(), "render_view", params);
        assert_eq!(
            state.dispatch_ui("render-task-guessed", render.clone())["outcome"]["data"]["code"],
            "PermissionDenied"
        );
        let admitted = state.dispatch_ui("main", render);
        assert_eq!(admitted["outcome"]["status"], "ok", "{admitted}");
        let task_id = admitted["outcome"]["data"]["id"].clone();
        let reply = state.wait_for_task(project.clone(), task_id.clone());
        assert_eq!(
            reply["outcome"]["data"]["task"]["state"], "completed",
            "{reply}"
        );
        let request = smoke_request(
            "adapter-image",
            project.clone(),
            "get_task",
            json!({"task_id":task_id}),
        );
        let (_, output, image) = state
            .rendered_task_image(&state.ui_session_id, &request, &reply)
            .unwrap()
            .unwrap();
        assert_eq!(json!(output.scene.scene_revision), json!(1));
        assert_eq!(
            image.bytes.as_ref(),
            include_bytes!(
                "../../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png"
            )
        );
        let session: ObjectId = "image-agent".to_owned().try_into().unwrap();
        state
            .register_external(HostCallContext {
                origin: sf_commands::CallerOrigin::ExternalAgent,
                session_id: session.clone(),
                granted_project: Some(serde_json::from_value(project.clone()).unwrap()),
                allowed_operations: vec![OperationName::GetTask],
                allowed_definition_drafts: vec![],
                selected_file_tokens: vec![],
                edit_grants: vec![],
                allow_view_control: false,
            })
            .unwrap();
        assert!(matches!(
            state.rendered_task_image(&session, &request, &reply),
            Err(ApiError {
                code: ErrorCode::PermissionDenied,
                ..
            })
        ));
        state
            .host
            .lock()
            .unwrap()
            .grant_external_task_access(
                &state.ui_session_id,
                &session,
                &serde_json::from_value(task_id).unwrap(),
                false,
            )
            .unwrap();
        assert!(
            state
                .rendered_task_image(&session, &request, &reply)
                .unwrap()
                .is_some()
        );
        state.revoke_external(&session).unwrap();
        assert!(matches!(
            state.rendered_task_image(&session, &request, &reply),
            Err(ApiError {
                code: ErrorCode::PermissionDenied,
                ..
            })
        ));
        assert_eq!(
            business_object(&state, &project, "project", project.as_str().unwrap()),
            before
        );
    }

    #[test]
    fn actual_bundled_library_registers_its_standard_disk_manifests() {
        let state = ProjectApiState::new();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../modules");
        state.register_bundled_library(&root).unwrap();
        let library = state.dispatch_ui(
            "main",
            smoke_request("bundled-media-versions", Value::Null, "query_module_library", json!({
                "category_filters": ["media"], "industry": null,
                "product_family": null, "site_envelope": null,
                "source_scope": ["bundled"]
            })),
        );
        let candidates = library["outcome"]["data"]["candidates"].as_array().unwrap();
        for suffix in [".simple", ""] {
            for family in ["assembly", "inspection", "packing", "rework"] {
                let id = format!("p302.line-grade.{family}{suffix}");
                for version in ["0.4.0", "0.4.1"] {
                    assert!(candidates.iter().any(|row| {
                        row["module_ref"]["id"].as_str() == Some(id.as_str())
                            && row["module_ref"]["version"].as_str() == Some(version)
                    }), "Missing bundled {id}@{version}");
                }
            }
        }
        let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(
            config["bundle"]["resources"]["../../../modules/media/equipment-quality-v041/installable/"],
            "modules/media/equipment-quality-v041/installable/"
        );
    }

    #[test]
    fn stored_packaging_list_cannot_accept_caller_paths_or_cross_window_identity() {
        let request = json!({
            "project_id": "packaging-project",
            "scope": { "kind": "definition_draft", "data": "packaging-draft" },
            "scope_revision": 1
        });
        let parsed: StoredPackagingScopeRequest = serde_json::from_value(request.clone()).unwrap();
        let state = ProjectApiState::new();
        assert!(matches!(
            state.list_stored_packaging_ui("secondary", &parsed),
            Err(ApiError {
                code: ErrorCode::PermissionDenied,
                ..
            })
        ));
        for field in ["session_id", "caller", "origin", "path", "artifact_id"] {
            let mut forged = request.clone();
            forged[field] = json!("caller-controlled");
            assert!(serde_json::from_value::<StoredPackagingScopeRequest>(forged).is_err());
        }
    }

    #[test]
    fn stored_packaging_read_has_private_identity_and_main_window_boundary() {
        let request = json!({
            "project_id": "packaging-project",
            "scope": { "kind": "project" },
            "scope_revision": 1,
            "artifact_id": "packaging-artifact"
        });
        let parsed: StoredPackagingReadRequest = serde_json::from_value(request.clone()).unwrap();
        let state = ProjectApiState::new();
        assert!(matches!(
            state.read_stored_packaging_ui("secondary", &parsed),
            Err(ApiError {
                code: ErrorCode::PermissionDenied,
                ..
            })
        ));
        for field in [
            "session_id",
            "caller",
            "origin",
            "path",
            "result_ref",
            "input_ref",
        ] {
            let mut forged = request.clone();
            forged[field] = json!("caller-controlled");
            assert!(serde_json::from_value::<StoredPackagingReadRequest>(forged).is_err());
        }
    }

    #[test]
    fn private_media_read_keeps_identity_and_main_window_boundaries() {
        let request = json!({
            "project_id": "media-project",
            "scope": { "kind": "project" },
            "scope_revision": 1,
            "source": { "kind": "module_version", "data": {
                "module_ref": { "id": "media-module", "version": "1.0.0" }
            }},
            "asset": { "id": "media-asset", "path": "assets/media.json", "kind": "json" }
        });
        let parsed: MediaAssetReadRequest = serde_json::from_value(request.clone()).unwrap();
        let state = ProjectApiState::new();
        assert!(matches!(
            state
                .read_media_asset_ui("secondary", &parsed)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        for field in ["session_id", "caller", "origin", "path"] {
            let mut forged = request.clone();
            forged[field] = json!("caller-controlled");
            assert!(serde_json::from_value::<MediaAssetReadRequest>(forged).is_err());
        }
    }

    fn assert_error(response: Value, code: &str, request_id: Value) {
        assert_eq!(response["outcome"]["status"], "error");
        assert_eq!(response["outcome"]["data"]["code"], code);
        assert_eq!(response["request_id"], request_id);
        serde_json::from_value::<Response<Value>>(response).expect("frozen response envelope");
    }

    #[test]
    fn package_file_picker_preserves_window_purpose_and_cancel_boundaries() {
        let state = ProjectApiState::new();
        assert!(file_dialog_title("main", &FilePurpose::ImportPackage).is_ok());
        assert!(file_dialog_title("main", &FilePurpose::ExportPackage).is_ok());
        assert!(matches!(
            file_dialog_title("secondary", &FilePurpose::ImportPackage)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        assert!(matches!(
            file_dialog_title("main", &FilePurpose::SaveProject)
                .unwrap_err()
                .code,
            ErrorCode::UnsupportedCapability
        ));
        let request = smoke_request(
            "cancelled-file-context",
            Value::Null,
            "get_context",
            json!({ "selection": [] }),
        );
        let before = state.dispatch_ui("main", request.clone());
        assert!(matches!(
            state
                .register_file(FilePurpose::ImportPackage, None)
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        ));
        assert_eq!(state.dispatch_ui("main", request), before);
    }

    #[test]
    fn project_api_rejects_wire_identity_and_malformed_requests() {
        let state = ProjectApiState::new();
        let request = smoke_request(
            "boundary",
            Value::Null,
            "get_context",
            json!({ "selection": [] }),
        );
        for field in ["caller", "grants", "session_id"] {
            let mut forged = request.clone();
            forged[field] = json!({ "origin": "interactive_ui" });
            assert_error(
                state.dispatch_ui("main", forged),
                "InvalidInput",
                json!("boundary"),
            );
        }
        let mut missing_project_id = request.clone();
        missing_project_id
            .as_object_mut()
            .unwrap()
            .remove("project_id");
        assert_error(
            state.dispatch_ui("main", missing_project_id),
            "InvalidInput",
            json!("boundary"),
        );
        assert_error(
            state.dispatch_ui("main", json!({ "request_id": "" })),
            "InvalidInput",
            Value::Null,
        );
        assert_error(
            state.dispatch_ui("main", Value::Null),
            "InvalidInput",
            Value::Null,
        );
        assert_error(
            state.dispatch_ui("secondary", request),
            "PermissionDenied",
            json!("boundary"),
        );
    }

    #[test]
    fn project_api_unknown_headers_keep_frozen_errors_without_mutation() {
        let state = ProjectApiState::new();
        assert_eq!(smoke_report(&state)["passed"], true);
        let request = smoke_request(
            "unknown-header",
            Value::Null,
            "get_context",
            json!({ "selection": [] }),
        );
        let before = state.dispatch_ui("main", request.clone());
        for (field, value, expected_code) in [
            ("api_version", json!("0.2"), "UnsupportedSchema"),
            (
                "operation",
                json!("w6_unknown_operation"),
                "UnsupportedCapability",
            ),
            ("api_version", json!(2), "InvalidInput"),
            ("operation", json!(12), "InvalidInput"),
            ("params", json!({ "selection": 12 }), "InvalidInput"),
        ] {
            let mut invalid = request.clone();
            invalid[field] = value;
            assert_error(
                state.dispatch_ui("main", invalid.clone()),
                expected_code,
                json!("unknown-header"),
            );
            assert_error(
                state.dispatch_ui("secondary", invalid),
                "PermissionDenied",
                json!("unknown-header"),
            );
            assert_eq!(state.dispatch_ui("main", request.clone()), before);
        }
    }

    #[test]
    fn project_api_reports_poison_without_dispatching() {
        let state = ProjectApiState::new();
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _host = state.host.lock().unwrap();
            panic!("intentionally poison the Host lock for this boundary test");
        }));
        assert!(panic.is_err());
        assert_error(
            state.dispatch_ui(
                "main",
                smoke_request(
                    "poison",
                    Value::Null,
                    "get_context",
                    json!({ "selection": [] }),
                ),
            ),
            "ComputationFailed",
            json!("poison"),
        );
    }

    #[test]
    fn project_directory_picker_gates_purpose_and_preserves_cancelled_state() {
        let state = ProjectApiState::new();
        assert!(directory_dialog_title("main", &FilePurpose::SaveProject).is_ok());
        assert!(directory_dialog_title("main", &FilePurpose::ImportPackage).is_ok());
        assert_eq!(
            directory_dialog_title("main", &FilePurpose::ExportPackage).unwrap(),
            "选择工程或贡献包导出目录"
        );
        assert_eq!(
            directory_dialog_title("main", &FilePurpose::ExportEvaluation).unwrap(),
            "选择评估报告导出目录"
        );
        assert!(matches!(
            directory_dialog_title("secondary", &FilePurpose::ExportEvaluation)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        assert!(matches!(
            directory_dialog_title("secondary", &FilePurpose::ExportPackage)
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
        let request = smoke_request(
            "picker-context",
            Value::Null,
            "get_context",
            json!({ "selection": [] }),
        );
        let before = state.dispatch_ui("main", request.clone());
        assert!(matches!(
            state
                .register_directory(FilePurpose::SaveProject, None)
                .unwrap_err()
                .code,
            ErrorCode::Cancelled
        ));
        assert_eq!(state.dispatch_ui("main", request), before);
    }

    #[test]
    fn project_api_save_reopen_and_history_use_the_same_fifo_host() {
        let state = ProjectApiState::new();
        let report = full_smoke_report(&state);
        assert_eq!(report["passed"], true, "{report}");
        assert_eq!(report["checks"].as_array().unwrap().len(), 32);
        let context = state.dispatch_ui(
            "main",
            smoke_request(
                "after-file-smoke",
                Value::Null,
                "get_context",
                json!({ "selection": [] }),
            ),
        );
        assert_eq!(
            context["outcome"]["data"]["project"]["title"],
            "Relocated project edited"
        );
        assert_eq!(context["outcome"]["data"]["project"]["dirty"], false);
    }

    #[test]
    fn project_api_fifo_commits_same_directory_in_admission_order() {
        let state = ProjectApiState::new();
        assert_eq!(smoke_report(&state)["passed"], true);
        let context = state.dispatch_ui(
            "main",
            smoke_request(
                "fifo-context",
                Value::Null,
                "get_context",
                json!({ "selection": [] }),
            ),
        );
        let project_id = context["outcome"]["data"]["project"]["id"].clone();
        let (temp, root) = smoke_directory().expect("create owned temporary directory");
        let first_directory = state
            .register_directory(FilePurpose::SaveProject, Some(root.clone()))
            .unwrap();
        let second_directory = state
            .register_directory(FilePurpose::SaveProject, Some(root.clone()))
            .unwrap();
        let wire = serde_json::to_value(&first_directory).unwrap();
        assert_eq!(wire.as_object().unwrap().len(), 2);
        assert!(wire.get("selection_id").is_some() && wire.get("purpose").is_some());
        let first = state.dispatch_ui(
            "main",
            smoke_request(
                "fifo-save-first",
                project_id.clone(),
                "save_project",
                json!({ "revision": 2, "selected_directory_ref": first_directory }),
            ),
        );
        assert_eq!(first["outcome"]["status"], "ok", "{first}");
        let edit = state.dispatch_ui(
            "main",
            smoke_request(
                "fifo-edit",
                project_id.clone(),
                "apply_commands",
                json!({ "base_revision": 2, "commands": [smoke_settings("FIFO latest revision")] }),
            ),
        );
        assert_eq!(edit["outcome"]["status"], "ok", "{edit}");
        let second = state.dispatch_ui(
            "main",
            smoke_request(
                "fifo-save-second",
                project_id.clone(),
                "save_project",
                json!({ "revision": 3, "selected_directory_ref": second_directory }),
            ),
        );
        assert_eq!(second["outcome"]["status"], "ok", "{second}");
        let first = state.wait_for_task(project_id.clone(), first["outcome"]["data"]["id"].clone());
        let second = state.wait_for_task(project_id, second["outcome"]["data"]["id"].clone());
        assert_eq!(
            first["outcome"]["data"]["task"]["state"], "completed",
            "{first}"
        );
        assert_eq!(
            second["outcome"]["data"]["task"]["state"], "completed",
            "{second}"
        );
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(root.join("project.json")).unwrap()).unwrap();
        assert_eq!(manifest["data"]["project_revision"], 3);
        assert_eq!(manifest["data"]["title"], "FIFO latest revision");
        remove_smoke_directory(&temp, &root).unwrap();
    }

    #[test]
    fn project_api_business_inputs_save_reopen_and_history_are_real() {
        let state = ProjectApiState::new();
        let report = business_full_smoke_report(&state);
        assert_eq!(report["passed"], true, "{report}");
        let current = business_object(&state, &json!("P1"), "scenario", "ORDERS");
        assert_eq!(
            current["outcome"]["data"]["data"]["orders"][0]["quantity"]["value"]["coefficient"],
            5
        );
        assert_eq!(
            current["outcome"]["data"]["data"]["production_tasks"][0]["planned_quantity"]["value"]
                ["coefficient"],
            5
        );
        let project = business_object(&state, &json!("P1"), "project", "P1");
        assert_eq!(
            project["outcome"]["data"]["data"]["module_refs"]
                .as_array()
                .unwrap()
                .len(),
            9
        );
        assert!(
            project["outcome"]["data"]["data"]["products"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["id"] == "A" && p["revision"] == "r1")
        );
    }

    #[test]
    fn project_api_smoke_uses_the_supplied_host_and_checks_real_data() {
        let state = ProjectApiState::new();
        let report = smoke_report(&state);
        assert_eq!(report["passed"], true, "{report}");
        assert_eq!(report["checks"].as_array().unwrap().len(), 11);
        let context = state.dispatch_ui(
            "main",
            smoke_request(
                "after-smoke",
                Value::Null,
                "get_context",
                json!({ "selection": [] }),
            ),
        );
        assert_eq!(context["outcome"]["data"]["project"]["revision"], 2);
        assert_eq!(
            context["outcome"]["data"]["project"]["title"],
            "P0-01 edited through desktop Host"
        );
    }

    #[test]
    fn project_api_material_inputs_are_real_drafts_across_history_save_and_reopen() {
        let state = ProjectApiState::new();
        let report = material_full_smoke_report(&state);
        assert_eq!(report["passed"], true, "{report}");
        let current = business_object(&state, &json!("P1"), "scenario", "STOCK");
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../packages/commands/tests/fixtures/business-inputs/initial-stock.scenario.json"
        )).unwrap();
        assert_eq!(
            material_input_projection(&current["outcome"]["data"]["data"]),
            material_input_projection(&fixture)
        );
        assert!(
            report["checks"]
                .as_array()
                .unwrap()
                .iter()
                .any(|check| check["name"]
                    == "material_history_never_changes_saved_or_relocated_archives"
                    && check["passed"] == true)
        );
    }
}
