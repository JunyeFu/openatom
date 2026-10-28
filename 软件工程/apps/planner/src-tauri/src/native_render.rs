//! Product-owned task windows. No Host mutex or client path crosses capture.
use serde::{Deserialize, Serialize};
use serde_json::json;
use sf_commands::{
    ApiError, ApiScope, ErrorCode, FrozenRenderInput, ObjectSource, RenderBackend, RenderDelivery,
    RenderFrame, RenderJob, RenderedCapture,
};
use sf_domain::{
    AssetId, AssetRef, CameraState, EditScope, FileRef, Issue, LodLevel, ObjectId, ObjectRef,
    ProjectId, RenderViewKind, RenderViewSpec, Revision, RunId, SimTimeMs,
};
use std::collections::BTreeMap;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use std::time::Duration;
use tauri::{Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[derive(Clone, Default)]
pub struct NativeRenderState(Arc<Mutex<BTreeMap<String, Entry>>>);
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Waiting,
    Capturing,
    Completed,
}
struct Entry {
    job: RenderJob,
    phase: Phase,
    cancelled: bool,
    sender: Option<mpsc::Sender<Result<RenderedCapture, ApiError>>>,
}
pub struct NativeRenderBackend {
    app: tauri::AppHandle,
    state: NativeRenderState,
    supported: Vec<RenderViewKind>,
}
impl NativeRenderBackend {
    /// Trusted setup supplies only views implemented by the current product route.
    pub fn new(
        app: tauri::AppHandle,
        state: NativeRenderState,
        supported: Vec<RenderViewKind>,
    ) -> Self {
        Self {
            app,
            state,
            supported,
        }
    }
}
fn error(code: ErrorCode, message: impl Into<String>) -> ApiError {
    ApiError {
        code,
        message: message.into(),
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}
fn failed(message: impl Into<String>) -> ApiError {
    error(ErrorCode::ComputationFailed, message)
}
fn cancelled() -> ApiError {
    error(
        ErrorCode::Cancelled,
        "Rendering cancelled before capture result commit",
    )
}
fn api_scope(scope: &EditScope) -> ApiScope {
    match scope {
        EditScope::Project(_) => ApiScope::Project,
        EditScope::DefinitionDraft(d) => ApiScope::DefinitionDraft(d.id.clone()),
    }
}
impl NativeRenderState {
    fn job(&self, label: &str) -> Result<RenderJob, ApiError> {
        self.0
            .lock()
            .map_err(|_| failed("Render registry poisoned"))?
            .get(label)
            .map(|e| e.job.clone())
            .ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "This real window owns no rendering task",
                )
            })
    }
    fn finish(&self, label: &str, result: Result<RenderedCapture, ApiError>) {
        if let Ok(mut entries) = self.0.lock() {
            if let Some(entry) = entries.get_mut(label) {
                entry.phase = Phase::Completed;
                if let Some(sender) = entry.sender.take() {
                    let _ = sender.send(result);
                }
            }
        }
    }
    fn remove(&self, label: &str) {
        if let Ok(mut entries) = self.0.lock() {
            entries.remove(label);
        }
    }
    fn waiting(&self, label: &str) -> Result<bool, ApiError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| failed("Render registry poisoned"))?
            .get(label)
            .is_some_and(|e| e.phase == Phase::Waiting && !e.cancelled))
    }
    fn fail_waiting(
        &self,
        label: &str,
        frame: Option<&RenderFrame>,
        failure: ApiError,
    ) -> Result<(), ApiError> {
        let mut entries = self
            .0
            .lock()
            .map_err(|_| failed("Render registry poisoned"))?;
        let entry = entries.get_mut(label).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "This real window owns no rendering task",
            )
        })?;
        if frame.is_some_and(|frame| json!(frame) != json!(entry.job.frame)) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Failure belongs to another Task",
            ));
        }
        if entry.phase != Phase::Waiting {
            return Err(failed(
                "Capture callback already owns this Task's completion",
            ));
        }
        entry.phase = Phase::Completed;
        if let Some(sender) = entry.sender.take() {
            let _ = sender.send(Err(failure));
        }
        Ok(())
    }
}
impl RenderBackend for NativeRenderBackend {
    fn supported_kinds(&self) -> Vec<RenderViewKind> {
        self.supported.clone()
    }
    fn render(&self, job: RenderJob, cancel: &AtomicBool) -> Result<RenderedCapture, ApiError> {
        if cancel.load(Ordering::Acquire) {
            return Err(cancelled());
        }
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let label = format!("render-task-{}", NEXT.fetch_add(1, Ordering::Relaxed));
        let (sender, receiver) = mpsc::channel();
        let ui_sender = sender.clone();
        self.state
            .0
            .lock()
            .map_err(|_| failed("Render registry poisoned"))?
            .insert(
                label.clone(),
                Entry {
                    job: job.clone(),
                    phase: Phase::Waiting,
                    cancelled: false,
                    sender: Some(sender),
                },
            );
        let app = self.app.clone();
        let state = self.state.clone();
        let window_label = label.clone();
        let window_frame = job.frame.clone();
        let viewport = job.viewport_css;
        if let Err(e) = self.app.run_on_main_thread(move || {
            match state.waiting(&window_label) {
                Ok(true) => {}
                Ok(false) => return,
                Err(e) => {
                    let _ = ui_sender.send(Err(e));
                    return;
                }
            }
            let built = WebviewWindowBuilder::new(
                &app,
                &window_label,
                WebviewUrl::App(format!("index.html?render_task={window_label}").into()),
            )
            .initialization_script(failure_script(&window_frame))
            .title("超级工厂 · 只读视图")
            .inner_size(viewport.0 as f64, viewport.1 as f64)
            .visible(false)
            .focused(false)
            .skip_taskbar(true)
            .resizable(false)
            .build();
            match built {
                Err(e) => state.finish(&window_label, Err(failed(format!("Task window: {e}")))),
                Ok(window) => {
                    // Cancellation may win while the UI is constructing its
                    // WebView. Do not leave a late, unregistered hidden window.
                    match state.waiting(&window_label) {
                        Ok(true) => {}
                        result => {
                            if let Err(e) = result {
                                let _ = ui_sender.send(Err(e));
                            }
                            let _ = window.close();
                            return;
                        }
                    }
                    let events = state.clone();
                    let event_label = window_label.clone();
                    window.on_window_event(move |event| match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            if let Ok(entries) = events.0.lock() {
                                if entries
                                    .get(&event_label)
                                    .is_some_and(|e| e.phase == Phase::Capturing)
                                {
                                    api.prevent_close();
                                }
                            }
                        }
                        tauri::WindowEvent::Destroyed => {
                            events.finish(
                                &event_label,
                                Err(failed("Task window was destroyed before capture completed")),
                            );
                            events.remove(&event_label);
                        }
                        _ => {}
                    });
                    let controller_state = state.clone();
                    let controller_label = window_label.clone();
                    if let Err(e) = window.with_webview(move |webview| {
                        let controller = webview.controller();
                        let mut actual = windows::core::BOOL::default();
                        let result = unsafe {
                            controller
                                .SetIsVisible(true)
                                .and_then(|()| controller.IsVisible(&mut actual))
                        };
                        if let Err(e) = result {
                            let _ = controller_state.fail_waiting(
                                &controller_label,
                                None,
                                failed(format!("Task controller HRESULT={:?}: {e}", e.code().0)),
                            );
                        } else if !actual.as_bool() {
                            let _ = controller_state.fail_waiting(
                                &controller_label,
                                None,
                                failed("Task controller did not become drawable"),
                            );
                        }
                    }) {
                        state.finish(&window_label, Err(failed(format!("Task controller: {e}"))));
                    }
                }
            }
        }) {
            self.state.remove(&label);
            return Err(failed(format!("Queue task window: {e}")));
        }
        let result = loop {
            match receiver.recv_timeout(Duration::from_millis(25)) {
                Ok(result) => break result,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err(failed("Task capture channel closed"));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if cancel.load(Ordering::Acquire) {
                        let capturing = {
                            let mut entries = self
                                .state
                                .0
                                .lock()
                                .map_err(|_| failed("Render registry poisoned"))?;
                            let entry = entries
                                .get_mut(&label)
                                .ok_or_else(|| failed("Task window disappeared"))?;
                            entry.cancelled = true;
                            entry.phase == Phase::Capturing
                        };
                        // A pending COM callback retains its window. The callback,
                        // not dropping a future, terminates the in-flight capture.
                        if !capturing {
                            self.state.finish(&label, Err(cancelled()));
                        }
                    }
                }
            }
        };
        let mut result = if cancel.load(Ordering::Acquire) {
            Err(cancelled())
        } else {
            result
        };
        let keep =
            result.is_ok() && matches!(job.frame.request.delivery, RenderDelivery::ShowAndCapture);
        if !keep {
            if let Some(window) = self.app.get_webview_window(&label) {
                if let Err(close) = window.close() {
                    result = Err(match result {
                        Ok(_) => failed(format!("Task window close could not be queued: {close}")),
                        Err(mut failure) => {
                            failure
                                .message
                                .push_str(&format!("; task window close: {close}"));
                            failure
                        }
                    });
                }
            }
            self.state.remove(&label);
        }
        result
    }
}

#[derive(Serialize)]
pub struct RenderInputReply {
    frame: RenderFrame,
    input: FrozenRenderInput,
}
#[tauri::command]
pub fn native_render_input(
    window: WebviewWindow,
    state: tauri::State<'_, NativeRenderState>,
) -> Result<Option<RenderInputReply>, ApiError> {
    if window.label() == "main" {
        crate::project_api::public_render_smoke::trigger(window.app_handle())?;
        return Ok(None);
    }
    let job = state.job(window.label())?;
    Ok(Some(RenderInputReply {
        frame: job.frame,
        input: (*job.input).clone(),
    }))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRequest {
    source: ObjectSource,
    asset: AssetRef,
}
#[tauri::command]
pub fn native_render_asset(
    window: WebviewWindow,
    state: tauri::State<'_, NativeRenderState>,
    request: AssetRequest,
) -> Result<tauri::ipc::Response, ApiError> {
    state
        .job(window.label())?
        .input
        .read_asset(&request.source, &request.asset)
        .map(tauri::ipc::Response::new)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRequest {
    file: FileRef,
}
#[tauri::command]
pub fn native_render_file(
    window: WebviewWindow,
    state: tauri::State<'_, NativeRenderState>,
    request: FileRequest,
) -> Result<tauri::ipc::Response, ApiError> {
    state
        .job(window.label())?
        .input
        .read_file(&request.file)
        .map(tauri::ipc::Response::new)
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyPixels {
    width: u32,
    height: u32,
    css_width: f64,
    css_height: f64,
    pixel_ratio: f64,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyLimitation {
    object_ref: Option<ObjectRef>,
    message: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyFrame {
    key: String,
    frame_id: u64,
    camera: CameraState,
    lod: LodLevel,
    pixels: ReadyPixels,
    visible_objects: Vec<ObjectRef>,
    limitations: Vec<ReadyLimitation>,
    source_key: String,
    project_id: ProjectId,
    scope: ApiScope,
    scene_revision: Revision,
    project_revision: Revision,
    view: RenderViewSpec,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadyRequest {
    frame: RenderFrame,
    ready: ReadyFrame,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ViewTime {
    run_id: RunId,
    at_ms: SimTimeMs,
}
type SourceKey = (
    ObjectId,
    ProjectId,
    ApiScope,
    Revision,
    ProjectId,
    Revision,
    Option<ViewTime>,
);
fn validate_ready(expected: &RenderFrame, request: &ReadyRequest) -> Result<(), ApiError> {
    let ready = &request.ready;
    let view = &expected.request.view;
    let source: SourceKey = serde_json::from_str(&ready.source_key)
        .map_err(|e| failed(format!("Frame source key: {e}")))?;
    let key: (String, Revision, RenderViewSpec) =
        serde_json::from_str(&ready.key).map_err(|e| failed(format!("Frame key: {e}")))?;
    let expected_source = json!([
        expected.task_id,
        expected.project_id,
        api_scope(&expected.scope),
        expected.scope_revision,
        expected.project_id,
        view.project_revision,
        view.run_id
            .as_ref()
            .map(|id| json!({"run_id":id,"at_ms":view.view_time_ms}))
    ]);
    if json!(request.frame) != json!(expected)
        || json!(source) != expected_source
        || key.0 != ready.source_key
        || json!(key.1) != json!(expected.scope_revision)
        || json!(key.2) != json!(view)
        || json!(ready.view) != json!(view)
        || json!(ready.camera) != json!(view.camera)
        || json!(ready.lod) != json!(view.lod)
        || ready.project_id != expected.project_id
        || json!(ready.scope) != json!(api_scope(&expected.scope))
        || json!(ready.scene_revision) != json!(expected.scope_revision)
        || json!(ready.project_revision) != json!(view.project_revision)
        || ready.frame_id == 0
        || ready.pixels.width == 0
        || ready.pixels.height == 0
        || !ready.pixels.css_width.is_finite()
        || ready.pixels.css_width <= 0.0
        || !ready.pixels.css_height.is_finite()
        || ready.pixels.css_height <= 0.0
        || !ready.pixels.pixel_ratio.is_finite()
        || ready.pixels.pixel_ratio <= 0.0
    {
        return Err(failed(
            "FrameReady differs from the exact frozen Task/source/view/revision/viewport",
        ));
    }
    Ok(())
}
#[tauri::command]
pub async fn native_render_ready(
    window: WebviewWindow,
    state: tauri::State<'_, NativeRenderState>,
    request: ReadyRequest,
) -> Result<(), ApiError> {
    let job = {
        let mut entries = state
            .0
            .lock()
            .map_err(|_| failed("Render registry poisoned"))?;
        let entry = entries.get_mut(window.label()).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "This real window owns no rendering task",
            )
        })?;
        if entry.phase != Phase::Waiting {
            return Err(failed("Task frame was already submitted"));
        }
        if entry.cancelled {
            return Err(cancelled());
        }
        if let Err(e) = validate_ready(&entry.job.frame, &request) {
            entry.phase = Phase::Completed;
            if let Some(sender) = entry.sender.take() {
                let _ = sender.send(Err(e.clone()));
            }
            return Err(e);
        }
        entry.phase = Phase::Capturing;
        entry.job.clone()
    };
    let result = async {
        if matches!(job.frame.request.delivery, RenderDelivery::ShowAndCapture) {
            let (sender, receiver) = tokio::sync::oneshot::channel();
            let gate = job.show_authorized.clone();
            let display = window.clone();
            let display_state = state.inner().clone();
            window
                .app_handle()
                .run_on_main_thread(move || {
                    let result = (|| {
                        let entries = display_state
                            .0
                            .lock()
                            .map_err(|_| failed("Render registry poisoned"))?;
                        if entries
                            .get(display.label())
                            .is_none_or(|entry| entry.cancelled)
                        {
                            return Err(cancelled());
                        }
                        if !gate.load(Ordering::Acquire) {
                            return Err(error(
                                ErrorCode::PermissionDenied,
                                "View control expired before actual display",
                            ));
                        }
                        display.show().map_err(|e| failed(e.to_string()))
                    })();
                    let _ = sender.send(result);
                })
                .map_err(|e| failed(e.to_string()))?;
            receiver
                .await
                .map_err(|_| failed("Display callback dropped"))??;
        } else if window.is_visible().map_err(|e| failed(e.to_string()))?
            || window.is_focused().map_err(|e| failed(e.to_string()))?
        {
            return Err(failed("CaptureOnly task window became visible or focused"));
        }
        let size = window.inner_size().map_err(|e| failed(e.to_string()))?;
        let scale = window.scale_factor().map_err(|e| failed(e.to_string()))?;
        if window.is_minimized().map_err(|e| failed(e.to_string()))? {
            return Err(failed("Ready task window is minimized"));
        }
        let pixels = &request.ready.pixels;
        if size.width != pixels.width
            || size.height != pixels.height
            || scale != pixels.pixel_ratio
            || (pixels.css_width as f64 * scale).round() as u32 != size.width
            || (pixels.css_height as f64 * scale).round() as u32 != size.height
        {
            return Err(failed("Ready viewport does not match the real task window"));
        }
        let bytes = crate::native_render_capture::capture_png(window.clone())
            .await
            .map_err(|e| {
                failed(format!(
                    "{} HRESULT={:?}: {}",
                    e.stage, e.hresult, e.message
                ))
            })?;
        if window.is_minimized().map_err(|e| failed(e.to_string()))? {
            return Err(failed("Task window minimized during capture"));
        }
        if matches!(job.frame.request.delivery, RenderDelivery::CaptureOnly)
            && (window.is_visible().map_err(|e| failed(e.to_string()))?
                || window.is_focused().map_err(|e| failed(e.to_string()))?)
        {
            return Err(failed(
                "CaptureOnly task window became visible or focused during capture",
            ));
        }
        if bytes.len() < 24
            || &bytes[..8] != b"\x89PNG\r\n\x1a\n"
            || &bytes[12..16] != b"IHDR"
            || u32::from_be_bytes(bytes[16..20].try_into().unwrap()) != size.width
            || u32::from_be_bytes(bytes[20..24].try_into().unwrap()) != size.height
        {
            return Err(failed("CapturePreview dimensions do not match FrameReady"));
        }
        if state
            .0
            .lock()
            .map_err(|_| failed("Render registry poisoned"))?
            .get(window.label())
            .is_none_or(|e| e.cancelled)
        {
            return Err(cancelled());
        }
        let issues = request
            .ready
            .limitations
            .iter()
            .map(|l| Issue {
                code: "render_view.presentation_limitation".into(),
                object_ref: l.object_ref.clone(),
                field_path: None,
                message: l.message.clone(),
                blocks: vec![],
            })
            .collect();
        Ok(RenderedCapture {
            frame: job.frame.clone(),
            scene_id: ObjectId::try_from(format!("{}:scene", window.label()))
                .expect("Native ordinal is valid"),
            image_id: AssetId::try_from(format!("{}:png", window.label()))
                .expect("Native ordinal is valid"),
            mime_type: "image/png".into(),
            bytes,
            visible_objects: request.ready.visible_objects,
            issues,
        })
    }
    .await;
    let reply = result.as_ref().map(|_| ()).map_err(Clone::clone);
    state.finish(window.label(), result);
    reply
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureRequest {
    frame: RenderFrame,
    stage: String,
    message: String,
    object_ref: Option<ObjectRef>,
}
#[tauri::command]
pub fn native_render_failure(
    window: WebviewWindow,
    state: tauri::State<'_, NativeRenderState>,
    request: FailureRequest,
) -> Result<(), ApiError> {
    let mut failure = failed(format!(
        "Product renderer {}: {}",
        request.stage, request.message
    ));
    failure.object_refs = request.object_ref.into_iter().collect();
    state.fail_waiting(window.label(), Some(&request.frame), failure)
}

fn failure_script(frame: &RenderFrame) -> String {
    format!(
        r#"(() => {{
      if (window.top !== window) return;
      const frame = {};
      const report = (stage, message) => {{
        const ipc = window.__TAURI_INTERNALS__;
        if (ipc) ipc.invoke('native_render_failure', {{request: {{frame, stage, message, object_ref: null}}}})
          .catch(error => console.error('Task failure reporting rejected', error));
      }};
      window.addEventListener('error', event => {{
        if (event instanceof ErrorEvent) report('javascript_error', `${{event.message}}; ${{event.filename}}:${{event.lineno}}:${{event.colno}}`);
        else if (event.target instanceof HTMLScriptElement) report('script_load', event.target.src);
      }}, true);
      window.addEventListener('unhandledrejection', event => report('unhandled_rejection', String(event.reason)));
    }})();"#,
        json!(frame)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (RenderFrame, ReadyRequest) {
        // Original browser-produced complete frame; this is an admission unit
        // fixture, never evidence of a new Native capture.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/Agent共建/runtime-native-product-capture/frame-key-original-browser.json");
        let mut ready: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let source = json!(["render-task-1","surface-1",{"kind":"project"},9,"surface-1",1,null])
            .to_string();
        let mut key: serde_json::Value =
            serde_json::from_str(ready["key"].as_str().unwrap()).unwrap();
        key[0] = json!(source);
        ready["source_key"] = json!(source);
        ready["key"] = json!(key.to_string());
        let frame: RenderFrame = serde_json::from_value(json!({"task_id":"render-task-1","epoch":4,"project_id":"surface-1","scope":{"kind":"project","data":"surface-1"},"scope_revision":9,"source":{"kind":"project"},"request":{"revision":9,"view":ready["view"],"object_id":null,"preview":null,"delivery":"capture_only"}})).unwrap();
        let request = serde_json::from_value(json!({"frame":frame,"ready":ready})).unwrap();
        (frame, request)
    }
    #[test]
    fn original_browser_numeric_spelling_passes_complete_typed_task_frame() {
        let (frame, ready) = fixture();
        let raw: serde_json::Value = serde_json::from_str(&ready.ready.key).unwrap();
        assert_ne!(
            raw[2]["camera"]["zoom"],
            json!(frame.request.view.camera.zoom)
        );
        validate_ready(&frame, &ready).unwrap();
    }
    #[test]
    fn fractional_css_viewport_from_scaled_webview_is_accepted() {
        let (frame, ready) = fixture();
        let mut value=json!(ready);
        value["ready"]["pixels"]["css_height"]=json!(889.3333740234375);
        let parsed:ReadyRequest=serde_json::from_value(value).unwrap();
        validate_ready(&frame,&parsed).unwrap();
    }
    #[test]
    fn task_epoch_scope_source_run_camera_viewport_and_revision_mismatches_fail() {
        let (frame, ready) = fixture();
        for (path, replacement) in [
            ("/frame/task_id", json!("other-task")),
            ("/frame/epoch", json!(5)),
            (
                "/frame/source",
                json!({"kind":"definition_draft","data":{"id":"other","revision":9}}),
            ),
            ("/ready/scene_revision", json!(1)),
            ("/ready/project_revision", json!(9)),
            (
                "/ready/scope",
                json!({"kind":"definition_draft","data":"other"}),
            ),
            ("/ready/view/view_time_ms", json!(0)),
            ("/ready/camera/eye/x", json!(21)),
            ("/ready/pixels/width", json!(0)),
            ("/ready/pixels/pixel_ratio", json!(0)),
            (
                "/ready/source_key",
                json!("[\"other\",\"surface-1\",{\"kind\":\"project\"},9,\"surface-1\",1,null]"),
            ),
        ] {
            let mut changed = json!(ready);
            *changed.pointer_mut(path).unwrap() = replacement;
            let parsed: ReadyRequest = serde_json::from_value(changed).unwrap();
            assert!(validate_ready(&frame, &parsed).is_err(), "{path}");
        }
    }
    #[test]
    fn request_carriers_reject_client_png_paths_labels_and_undeclared_fields() {
        let (_, ready) = fixture();
        for key in ["png", "path", "window_label", "caller"] {
            let mut changed = json!(ready);
            changed[key] = json!("client-supplied");
            assert!(serde_json::from_value::<ReadyRequest>(changed).is_err());
        }
        assert!(
            serde_json::from_value::<FileRequest>(
                json!({"file":{"kind":"asset","path":"assets/a.svg"},"path":"C:/secret"})
            )
            .is_err()
        );
    }
    #[test]
    fn main_unknown_or_guessed_ordinal_has_no_registered_frozen_input() {
        let state = NativeRenderState::default();
        for label in ["main", "render-task-1", "foreign"] {
            assert!(matches!(
                state.job(label),
                Err(ApiError {
                    code: ErrorCode::PermissionDenied,
                    ..
                })
            ));
        }
    }
    #[test]
    fn late_renderer_failure_cannot_release_an_in_flight_capture_or_complete_twice() {
        let (frame, _) = fixture();
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../scenarios/Agent共建/runtime-native-product-capture/frame-key-js-admission-fixture.json");
        let original: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let input = FrozenRenderInput {
            project: serde_json::from_value(original["plan"]["project"].clone()).unwrap(),
            scenarios: vec![],
            modules: vec![],
            definition: None,
            template_subset: None,
            run: None,
            active_job_id: None,
            objects_at: vec![],
            port_outputs: vec![],
            port_output_failures: vec![],
            packaging: None,
            comparison: None,
            comparison_runs: vec![],
            port_output_summaries: vec![],
            assets: vec![],
            files: vec![],
            issues: vec![],
        };
        let job = RenderJob {
            frame: frame.clone(),
            input: Arc::new(input),
            viewport_css: (1440, 900),
            show_authorized: Arc::new(AtomicBool::new(false)),
        };
        let (sender, receiver) = mpsc::channel();
        let state = NativeRenderState::default();
        state.0.lock().unwrap().insert(
            "owned".into(),
            Entry {
                job,
                phase: Phase::Capturing,
                cancelled: true,
                sender: Some(sender),
            },
        );
        assert!(
            state
                .fail_waiting("owned", Some(&frame), failed("late JS failure"))
                .is_err()
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        state.finish("owned", Err(cancelled()));
        assert!(matches!(
            receiver.recv().unwrap(),
            Err(ApiError {
                code: ErrorCode::Cancelled,
                ..
            })
        ));
        state.finish("owned", Err(failed("late duplicate callback")));
        assert!(matches!(
            receiver.try_recv(),
            Err(mpsc::TryRecvError::Disconnected)
        ));
    }
}
