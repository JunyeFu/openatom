//! Private, explicitly selected CLI experiment; not public RenderView or Task IO.
use super::*;
use sf_commands::ProjectContext;
use sf_domain::{CameraState, LodLevel, ObjectRef, ProjectState, RenderViewSpec};
use tauri::Manager;

/// Installed only in the explicitly requested probe. The frozen product bundle
/// is unchanged; these listeners report actual startup failure, not readiness.
pub const INITIALIZATION_SCRIPT: &str = r#"
(() => {
  if (window.top !== window) return;
  const report = (stage, message) => {
    const ipc = window.__TAURI_INTERNALS__;
    if (!ipc) { console.error('Native capture diagnostic IPC is unavailable', stage, message); return; }
    ipc.invoke('native_capture_probe_event', { request: { stage, message } })
      .catch(error => console.error('Native capture diagnostic failed', error));
  };
  window.addEventListener('DOMContentLoaded', () => report('dom_content_loaded', document.readyState), { once: true });
  window.addEventListener('error', event => report('error', event instanceof ErrorEvent
    ? `${event.message}; ${event.filename}:${event.lineno}:${event.colno}`
    : `Resource failed: ${event.target?.src ?? event.target?.href ?? event.type}`), true);
  window.addEventListener('unhandledrejection', event => report('unhandled_rejection', String(event.reason)));
})();
"#;

#[derive(Clone, Debug)]
struct ProbeConfig {
    input: PathBuf,
    output: PathBuf,
    views: PathBuf,
}
impl ProbeConfig {
    fn parse(args: &[String]) -> Result<Option<Self>, String> {
        if !args.iter().any(|arg| arg == "--p3-04-capture-smoke") {
            return Ok(None);
        }
        if ["--s07-smoke", "--p0-01-host-smoke", "--p3-04-mcp-smoke"]
            .iter()
            .any(|flag| args.iter().any(|arg| arg == flag))
        {
            return Err("Capture smoke cannot be combined with another smoke mode".into());
        }
        let option = |flag: &str| -> Result<PathBuf, String> {
            let positions: Vec<_> = args
                .iter()
                .enumerate()
                .filter(|(_, arg)| arg.as_str() == flag)
                .map(|(i, _)| i)
                .collect();
            if positions.len() != 1 {
                return Err(format!("Capture smoke requires exactly one {flag}"));
            }
            let value = args
                .get(positions[0] + 1)
                .filter(|arg| !arg.starts_with("--"))
                .ok_or_else(|| format!("Missing value for {flag}"))?;
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err(format!("{flag} must be an absolute operator-selected path"));
            }
            Ok(path)
        };
        Ok(Some(Self {
            input: option("--capture-input")?,
            output: option("--capture-output")?,
            views: option("--capture-views")?,
        }))
    }
}

#[derive(Clone, Serialize)]
pub struct ProbePlan {
    plan_id: String,
    project: ProjectState,
    context: ProjectContext,
    views: Vec<RenderViewSpec>,
}
struct Probe {
    config: ProbeConfig,
    plan: ProbePlan,
    project_before: Value,
    runs_before: Vec<(Value, Value)>,
    next_step: usize,
    in_flight: bool,
    terminal: bool,
    records: Vec<Value>,
    trace_sequence: u64,
}
#[derive(Default)]
pub struct NativeCaptureProbeState(Mutex<Option<Probe>>);

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbePixels {
    width: u32,
    height: u32,
    css_width: u32,
    css_height: u32,
    pixel_ratio: f64,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeLimitation {
    object_ref: Option<ObjectRef>,
    message: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeFrame {
    key: String,
    frame_id: u64,
    camera: CameraState,
    lod: LodLevel,
    pixels: ProbePixels,
    visible_objects: Vec<ObjectRef>,
    limitations: Vec<ProbeLimitation>,
    source_key: String,
    project_id: ProjectId,
    scope: ApiScope,
    scene_revision: Revision,
    project_revision: Revision,
    view: RenderViewSpec,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FrameRequest {
    plan_id: String,
    step: usize,
    frame: ProbeFrame,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailureRequest {
    plan_id: String,
    step: usize,
    key: String,
    stage: String,
    message: String,
    object_ref: Option<ObjectRef>,
}
#[derive(Serialize)]
pub struct StepReply {
    next_step: Option<usize>,
    ignored: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererStage {
    DomContentLoaded,
    Error,
    UnhandledRejection,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RendererEvent {
    stage: RendererStage,
    message: String,
}

fn trace(state: &NativeCaptureProbeState, stage: &str, details: Value) -> Result<(), String> {
    use std::io::Write;
    let mut guard = state.0.lock().map_err(|_| "Capture state poisoned")?;
    let Some(probe) = guard.as_mut() else {
        return Ok(());
    };
    let mut options = std::fs::OpenOptions::new();
    options.write(true);
    if probe.trace_sequence == 0 {
        options.create_new(true);
    } else {
        options.append(true);
    }
    let mut file = options
        .open(probe.config.output.join("stages.jsonl"))
        .map_err(|error| error.to_string())?;
    let mut bytes = serde_json::to_vec(
        &json!({"sequence":probe.trace_sequence,"stage":stage,"details":details}),
    )
    .map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    probe.trace_sequence += 1;
    Ok(())
}

pub fn record(app: &tauri::AppHandle, stage: &str, details: Value) -> Result<(), String> {
    let state = app.state::<NativeCaptureProbeState>();
    if let Err(error) = trace(&state, stage, details) {
        terminal_failure(&state, app, "stage_record", error.clone());
        return Err(error);
    }
    Ok(())
}

fn data(reply: Value) -> Result<Value, String> {
    if reply["outcome"]["status"] != "ok" {
        return Err(format!("Actual Host request failed: {reply}"));
    }
    Ok(reply["outcome"]["data"].clone())
}
fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    use std::io::Write;
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())
}
fn current_project(state: &ProjectApiState, id: &Value) -> Result<Value, String> {
    let object = data(business_object(
        state,
        id,
        "project",
        id.as_str().ok_or("Project ID is not a string")?,
    ))?;
    if object["kind"] != "project" {
        return Err("Actual Host did not return Project".into());
    }
    Ok(object["data"].clone())
}
fn full_run(state: &ProjectApiState, project: &Value, run: &Value) -> Result<Value, String> {
    let view = data(state.dispatch_ui(
        "main",
        smoke_request(
            "capture-baseline-run",
            project.clone(),
            "get_run",
            json!({"run_id":run,"query":{"kind":"full_result"}}),
        ),
    ))?;
    if view["kind"] != "full" {
        return Err("Actual Host did not return the complete stored Run".into());
    }
    Ok(view["data"].clone())
}

pub fn prepare(
    state: &ProjectApiState,
    probe: &NativeCaptureProbeState,
    args: &[String],
) -> Result<bool, String> {
    let Some(config) = ProbeConfig::parse(args)? else {
        return Ok(false);
    };
    std::fs::create_dir(&config.output)
        .map_err(|error| format!("Capture output must be a new directory: {error}"))?;
    let prepared = (|| {
        let input = config
            .input
            .canonicalize()
            .map_err(|error| error.to_string())?;
        let views: Vec<RenderViewSpec> = serde_json::from_slice(
            &std::fs::read(&config.views).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        if views.len() != 2 {
            return Err("This technical gate requires exactly two explicit views".into());
        }
        let selected = state
            .register_directory(FilePurpose::ImportPackage, Some(input.clone()))
            .map_err(|error| error.message)?;
        let admitted = data(state.dispatch_ui("main", smoke_request("capture-import", Value::Null, "import_package", json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}))))?;
        let task_id = admitted["id"].clone();
        let imported = state.wait_for_task(Value::Null, task_id.clone());
        if imported["outcome"]["data"]["task"]["state"] != "completed" {
            return Err(format!(
                "Capture source import did not complete: {imported}"
            ));
        }
        data(state.dispatch_ui(
            "main",
            smoke_request(
                "capture-open",
                Value::Null,
                "apply_commands",
                business_adoption(task_id, &imported),
            ),
        ))?;
        let context_value = data(state.dispatch_ui(
            "main",
            smoke_request(
                "capture-context",
                Value::Null,
                "get_context",
                json!({"selection":[]}),
            ),
        ))?;
        let project_id = context_value["project"]["id"].clone();
        let project_before = current_project(state, &project_id)?;
        let project: ProjectState =
            serde_json::from_value(project_before.clone()).map_err(|error| error.to_string())?;
        let context: ProjectContext =
            serde_json::from_value(context_value.clone()).map_err(|error| error.to_string())?;
        let mut runs_before = vec![];
        for summary in &context.run_summaries {
            let id = json!(summary.run_id);
            runs_before.push((id.clone(), full_run(state, &project_id, &id)?));
        }
        for view in &views {
            if !matches!(
                view.kind,
                sf_domain::RenderViewKind::LineCanvas | sf_domain::RenderViewKind::MultiHallCanvas
            ) {
                return Err("This probe only captures the actual factory canvases".into());
            }
            let frozen = if let Some(run_id) = &view.run_id {
                let at = view
                    .view_time_ms
                    .as_ref()
                    .ok_or("An actual Run view requires its exact time")?;
                let object = data(state.dispatch_ui("main", smoke_request("capture-frozen-project", project_id.clone(), "get_object", json!({"object_id":{"kind":"project","id":project_id},"source":{"kind":"run_snapshot","data":{"run_id":run_id,"at_ms":at}},"node_index":null}))))?;
                object["data"].clone()
            } else {
                if view.view_time_ms.is_some() {
                    return Err("Draft capture must not invent a Run time".into());
                }
                project_before.clone()
            };
            if frozen["id"] != project_id || frozen["revision"] != json!(view.project_revision) {
                return Err(
                    "Requested view does not match the actual frozen Project revision".into(),
                );
            }
        }
        let plan = ProbePlan {
            plan_id: format!("native-capture-{}", std::process::id()),
            project,
            context,
            views,
        };
        write_json(
            &config.output.join("plan.json"),
            &json!({"kind":"P3-04_native_capture_probe","input":input,"plan":plan,"public_render_view":false}),
        )?;
        Ok(Probe {
            config: config.clone(),
            plan,
            project_before,
            runs_before,
            next_step: 0,
            in_flight: false,
            terminal: false,
            records: vec![],
            trace_sequence: 0,
        })
    })();
    match prepared {
        Ok(value) => {
            *probe.0.lock().map_err(|_| "Capture state poisoned")? = Some(value);
            trace(probe, "prepared", json!({"same_native_Host":true}))?;
            Ok(true)
        }
        Err(error) => {
            let _ = write_json(
                &config.output.join("failure.json"),
                &json!({"stage":"prepare","error":error,"passed":false}),
            );
            Err(error)
        }
    }
}

fn main_only(label: &str) -> Result<(), String> {
    if label == "main" {
        Ok(())
    } else {
        Err("Only the original main window may use this private capture smoke".into())
    }
}
/// Called after the trusted hidden main window is built, before any JS plan
/// request. Controller drawing no longer depends on JS reaching that request.
pub fn window_built(window: &tauri::WebviewWindow) -> Result<(), String> {
    main_only(window.label())?;
    let initialized = (|| -> Result<(), String> {
        let app = window.app_handle();
        record(app, "window-built", json!({}))?;
        let size = window.inner_size().map_err(|error| error.to_string())?;
        record(
            app,
            "window-built.state",
            json!({"visible":window.is_visible().map_err(|error|error.to_string())?,"focused":window.is_focused().map_err(|error|error.to_string())?,"minimized":window.is_minimized().map_err(|error|error.to_string())?,"inner_size":{"width":size.width,"height":size.height}}),
        )?;
        record(app, "controller-bootstrap.queued", json!({}))?;
        let app = app.clone();
        window
            .with_webview(move |webview| {
                let result = (|| -> Result<(), String> {
                    record(&app, "controller-bootstrap.entered", json!({}))?;
                    let controller = webview.controller();
                    let mut before = windows::core::BOOL::default();
                    let mut after = windows::core::BOOL::default();
                    unsafe {
                        controller
                            .IsVisible(&mut before)
                            .and_then(|()| controller.SetIsVisible(true))
                            .and_then(|()| controller.IsVisible(&mut after))
                    }
                    .map_err(|error| {
                        format!("Controller bootstrap HRESULT={:?}: {error}", error.code().0)
                    })?;
                    record(
                        &app,
                        "controller-bootstrap.com-returned",
                        json!({"before":before.as_bool(),"after":after.as_bool()}),
                    )?;
                    if !after.as_bool() {
                        return Err("Controller bootstrap did not enable drawing".into());
                    }
                    Ok(())
                })();
                if let Err(error) = result {
                    terminal_failure(
                        &app.state::<NativeCaptureProbeState>(),
                        &app,
                        "controller-bootstrap",
                        error,
                    );
                }
            })
            .map_err(|error| error.to_string())
    })();
    if let Err(error) = &initialized {
        terminal_failure(
            &window.app_handle().state::<NativeCaptureProbeState>(),
            window.app_handle(),
            "window-bootstrap",
            error.clone(),
        );
    }
    initialized
}

async fn window_state(
    window: &tauri::WebviewWindow,
    app: &tauri::AppHandle,
    phase: &'static str,
) -> Result<Value, String> {
    record(app, &format!("{phase}.queued"), json!({}))?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let callback_app = app.clone();
    window
        .with_webview(move |webview| {
            let result = (|| -> Result<Value, String> {
                record(&callback_app, &format!("{phase}.entered"), json!({}))?;
                let controller = webview.controller();
                let mut before = windows::core::BOOL::default();
                let mut after = windows::core::BOOL::default();
                let result = unsafe {
                    controller
                        .IsVisible(&mut before)
                        .and_then(|()| controller.IsVisible(&mut after))
                }
                .map(|()| json!({"before":before.as_bool(),"after":after.as_bool()}))
                .map_err(|error| {
                    format!(
                        "WebView2 controller state HRESULT={:?}: {error}",
                        error.code().0
                    )
                })?;
                record(
                    &callback_app,
                    &format!("{phase}.com-returned"),
                    result.clone(),
                )?;
                Ok(result)
            })();
            let _ = sender.send(result);
        })
        .map_err(|error| error.to_string())?;
    let controller = receiver
        .await
        .map_err(|_| "Controller callback dropped")??;
    record(
        app,
        &format!("{phase}.oneshot-returned"),
        controller.clone(),
    )?;
    let size = window.inner_size().map_err(|error| error.to_string())?;
    let visible = window.is_visible().map_err(|error| error.to_string())?;
    let focused = window.is_focused().map_err(|error| error.to_string())?;
    let minimized = window.is_minimized().map_err(|error| error.to_string())?;
    if visible
        || focused
        || minimized
        || size.width == 0
        || size.height == 0
        || controller["after"] != true
    {
        return Err(format!(
            "Capture window must stay hidden, unfocused, not minimized, nonzero and drawable: visible={visible}, focused={focused}, minimized={minimized}, size={size:?}, controller={controller}"
        ));
    }
    Ok(
        json!({"visible":visible,"focused":focused,"minimized":minimized,"inner_size":{"width":size.width,"height":size.height},"scale_factor":window.scale_factor().map_err(|error|error.to_string())?,"controller":controller}),
    )
}
fn terminal_failure(
    probe: &NativeCaptureProbeState,
    app: &tauri::AppHandle,
    stage: &str,
    error: String,
) {
    if let Ok(mut guard) = probe.0.lock() {
        if let Some(probe) = guard.as_mut() {
            if !probe.terminal {
                probe.terminal = true;
                let _ = write_json(
                    &probe.config.output.join("failure.json"),
                    &json!({"stage":stage,"error":error,"step":probe.next_step,"passed":false,"completed_steps":probe.records}),
                );
            }
        }
    }
    eprintln!(
        "{}",
        json!({"kind":"P3-04_native_capture_failure","stage":stage,"error":error})
    );
    app.exit(1);
}

#[tauri::command]
pub async fn native_capture_probe_plan(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, NativeCaptureProbeState>,
) -> Result<Option<ProbePlan>, String> {
    main_only(window.label())?;
    record(&app, "plan.command-entered", json!({}))?;
    let plan = state
        .0
        .lock()
        .map_err(|_| "Capture state poisoned")?
        .as_ref()
        .map(|probe| probe.plan.clone());
    if plan.is_none() {
        return Ok(None);
    }
    match window_state(&window, &app, "plan.controller").await {
        Ok(actual) => {
            let recorded = (|| -> Result<_, String> {
                let guard = state.0.lock().map_err(|_| "Capture state poisoned")?;
                let probe = guard.as_ref().ok_or("Missing probe")?;
                write_json(&probe.config.output.join("window-initial.json"), &actual)?;
                Ok(plan)
            })();
            if let Err(error) = &recorded {
                terminal_failure(&state, &app, "window_record", error.clone());
            }
            recorded
        }
        Err(error) => {
            terminal_failure(&state, &app, "window", error.clone());
            Err(error)
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CaptureViewTime {
    run_id: sf_domain::RunId,
    at_ms: sf_domain::SimTimeMs,
}
type CaptureSourceKey = (
    ObjectId,
    ProjectId,
    ApiScope,
    Revision,
    ProjectId,
    Revision,
    Option<CaptureViewTime>,
);

fn expected_source(probe: &Probe, view: &RenderViewSpec) -> Value {
    json!([probe.plan.context.host.session_id,probe.plan.project.id,{"kind":"project"},probe.plan.project.revision,probe.plan.project.id,probe.plan.project.revision,
        view.run_id.as_ref().map(|id| json!({"run_id":id,"at_ms":view.view_time_ms}))])
}

fn validate_frame(probe: &Probe, request: &FrameRequest) -> Result<(), String> {
    if request.plan_id != probe.plan.plan_id {
        return Err("Frame belongs to a different capture plan".into());
    }
    let frame = &request.frame;
    let view = &probe.plan.views[request.step];
    let source: CaptureSourceKey = serde_json::from_str(&frame.source_key)
        .map_err(|error| format!("frame.source_key: {error}"))?;
    let key: (String, Revision, RenderViewSpec) =
        serde_json::from_str(&frame.key).map_err(|error| format!("frame.key: {error}"))?;
    // JSON.stringify writes integral camera values as 23/1; serde_json::Value
    // distinguishes those integers from typed f64 23.0/1.0. Deserialize the
    // original tuple types first, retaining every declared view field.
    let checks = [
        (
            "frame.source_key",
            json!(source) == expected_source(probe, view),
        ),
        ("frame.key.source_key", key.0 == frame.source_key),
        (
            "frame.key.scope_revision",
            json!(key.1) == json!(probe.plan.project.revision),
        ),
        ("frame.key.view", json!(key.2) == json!(view)),
        ("frame.view", json!(frame.view) == json!(view)),
        ("frame.camera", json!(frame.camera) == json!(view.camera)),
        ("frame.lod", json!(frame.lod) == json!(view.lod)),
        (
            "frame.project_id",
            frame.project_id == probe.plan.project.id,
        ),
        ("frame.scope", matches!(frame.scope, ApiScope::Project)),
        (
            "frame.scene_revision",
            json!(frame.scene_revision) == json!(probe.plan.project.revision),
        ),
        (
            "frame.project_revision",
            json!(frame.project_revision) == json!(view.project_revision),
        ),
        ("frame.frame_id", frame.frame_id != 0),
        ("frame.pixels.width", frame.pixels.width != 0),
        ("frame.pixels.height", frame.pixels.height != 0),
        ("frame.pixels.css_width", frame.pixels.css_width != 0),
        ("frame.pixels.css_height", frame.pixels.css_height != 0),
        (
            "frame.pixels.pixel_ratio",
            frame.pixels.pixel_ratio.is_finite() && frame.pixels.pixel_ratio > 0.0,
        ),
    ];
    if let Some((field, _)) = checks.into_iter().find(|(_, equal)| !*equal) {
        return Err(format!("FrameReady mismatch at {field}"));
    }
    Ok(())
}

#[tauri::command]
pub async fn native_capture_probe_frame(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    host: tauri::State<'_, ProjectApiState>,
    state: tauri::State<'_, NativeCaptureProbeState>,
    request: FrameRequest,
) -> Result<StepReply, String> {
    main_only(window.label())?;
    let admission = (|| -> Result<_, String> {
        let mut guard = state.0.lock().map_err(|_| "Capture state poisoned")?;
        let probe = guard.as_mut().ok_or("Capture smoke is not enabled")?;
        if request.plan_id == probe.plan.plan_id
            && (request.step != probe.next_step || probe.in_flight || probe.terminal)
        {
            return Ok(None);
        }
        write_json(
            &probe
                .config
                .output
                .join(format!("frame-ready-{:02}.json", probe.next_step + 1)),
            &json!({"request":request,"expected_view":probe.plan.views.get(probe.next_step),"expected_source_key":probe.plan.views.get(probe.next_step).map(|view|expected_source(probe,view))}),
        )?;
        validate_frame(probe, &request)?;
        probe.in_flight = true;
        Ok(Some((
            probe.config.output.clone(),
            probe.project_before.clone(),
            probe.runs_before.clone(),
        )))
    })();
    let (output, before, runs_before) = match admission {
        Ok(Some(value)) => value,
        Ok(None) => {
            return Ok(StepReply {
                next_step: None,
                ignored: true,
            });
        }
        Err(error) => {
            terminal_failure(&state, &app, "frame_admission", error.clone());
            return Err(error);
        }
    };
    let captured: Result<Value, String> = async {
        let window_before = window_state(&window, &app, "capture.before").await?;
        let png = crate::native_render_capture::capture_png(window.clone()).await.map_err(|error| format!("stage={} HRESULT={:?}: {}", error.stage, error.hresult, error.message))?;
        let window_after = window_state(&window, &app, "capture.after").await?;
        if png.len() < 24 || &png[..8] != b"\x89PNG\r\n\x1a\n" || &png[12..16] != b"IHDR" { return Err("CapturePreview did not return a PNG with IHDR".into()); }
        let width = u32::from_be_bytes(png[16..20].try_into().unwrap()); let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
        if width == 0 || height == 0 { return Err("CapturePreview PNG dimensions are zero".into()); }
        if width != request.frame.pixels.width || height != request.frame.pixels.height {
            return Err("CapturePreview dimensions do not match the actual ready viewport".into());
        }
        let project_id = json!(request.frame.project_id);
        if current_project(&host, &project_id)? != before { return Err("Capture changed the active Project".into()); }
        for (id, original) in &runs_before { if full_run(&host, &project_id, id)? != *original { return Err("Capture changed a stored Run".into()); } }
        use std::io::Write;
        let file = format!("step-{:02}.png", request.step + 1);
        std::fs::OpenOptions::new().write(true).create_new(true).open(output.join(&file)).map_err(|error| error.to_string())?.write_all(&png).map_err(|error|error.to_string())?;
        let record = json!({"step":request.step,"frame":request.frame,"window_before":window_before,"window_after":window_after,"file":file,"png":{"width":width,"height":height,"bytes":png.len()},"project_unchanged":true,"all_stored_runs_unchanged":true});
        write_json(&output.join(format!("step-{:02}.json",request.step+1)),&record)?;
        Ok(record)
    }.await;
    match captured {
        Err(error) => {
            terminal_failure(&state, &app, "capture", error.clone());
            Err(error)
        }
        Ok(record) => {
            let completed = (|| -> Result<_, String> {
                let mut guard = state.0.lock().map_err(|_| "Capture state poisoned")?;
                let probe = guard.as_mut().ok_or("Missing probe")?;
                probe.records.push(record);
                probe.next_step += 1;
                probe.in_flight = false;
                if probe.next_step == probe.plan.views.len() {
                    write_json(
                        &output.join("capture-result.json"),
                        &json!({"passed":true,"kind":"P3-04_native_capture_probe","plan":probe.plan,"steps":probe.records,"public_render_view":false}),
                    )?;
                    probe.terminal = true;
                    println!(
                        "{}",
                        json!({"kind":"P3-04_native_capture_probe","passed":true,"steps":probe.next_step})
                    );
                    Ok(StepReply {
                        next_step: None,
                        ignored: false,
                    })
                } else {
                    Ok(StepReply {
                        next_step: Some(probe.next_step),
                        ignored: false,
                    })
                }
            })();
            match &completed {
                Ok(reply) if reply.next_step.is_none() => app.exit(0),
                Err(error) => terminal_failure(&state, &app, "capture_record", error.clone()),
                _ => (),
            }
            completed
        }
    }
}

#[tauri::command]
pub fn native_capture_probe_event(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, NativeCaptureProbeState>,
    request: RendererEvent,
) -> Result<(), String> {
    main_only(window.label())?;
    if state
        .0
        .lock()
        .map_err(|_| "Capture state poisoned")?
        .is_none()
    {
        return Err("Capture smoke is not enabled".into());
    }
    let stage = match request.stage {
        RendererStage::DomContentLoaded => "renderer.dom_content_loaded",
        RendererStage::Error => "renderer.error",
        RendererStage::UnhandledRejection => "renderer.unhandled_rejection",
    };
    record(&app, stage, json!({"message":request.message}))?;
    if !matches!(request.stage, RendererStage::DomContentLoaded) {
        terminal_failure(&state, &app, stage, request.message);
    }
    Ok(())
}

#[tauri::command]
pub fn native_capture_probe_failure(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, NativeCaptureProbeState>,
    request: FailureRequest,
) -> Result<(), String> {
    main_only(window.label())?;
    {
        let guard = state.0.lock().map_err(|_| "Capture state poisoned")?;
        let probe = guard.as_ref().ok_or("Capture smoke is not enabled")?;
        if request.plan_id != probe.plan.plan_id
            || request.step != probe.next_step
            || probe.terminal
        {
            return Ok(());
        }
    }
    terminal_failure(
        &state,
        &app,
        &request.stage,
        format!(
            "{}; key={}; object={:?}",
            request.message, request.key, request.object_ref
        ),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_browser_frame_key_has_integer_spelling_but_the_same_complete_typed_view() {
        let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../scenarios/Agent共建/runtime-native-product-capture");
        let frame: ProbeFrame = serde_json::from_slice(
            &std::fs::read(runtime.join("frame-key-original-browser.json")).unwrap(),
        )
        .unwrap();
        let raw: Value = serde_json::from_str(&frame.key).unwrap();
        let expected = json!([frame.source_key, frame.scene_revision, frame.view]);
        assert!(raw[2]["camera"]["zoom"].is_u64());
        assert!(expected[2]["camera"]["zoom"].is_f64());
        assert_ne!(
            raw, expected,
            "The old Value equality reproduces the actual browser integer/float representation mismatch"
        );
        let typed: (String, Revision, RenderViewSpec) = serde_json::from_str(&frame.key).unwrap();
        assert_eq!(json!(typed.2), json!(frame.view));
        println!(
            "{}",
            json!({"actual_original_browser_key":frame.key,"old_Value_equal":false,"typed_complete_view_equal":true,"JS_zoom":raw[2]["camera"]["zoom"],"Rust_zoom":expected[2]["camera"]["zoom"]})
        );
    }
    #[test]
    fn js_frame_key_admission_keeps_source_scope_revision_camera_and_viewport_gates() {
        let runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../scenarios/Agent共建/runtime-native-product-capture");
        let fixture: Value = serde_json::from_slice(
            &std::fs::read(runtime.join("frame-key-js-admission-fixture.json")).unwrap(),
        )
        .unwrap();
        let p = &fixture["plan"];
        let probe = Probe {
            config: ProbeConfig {
                input: PathBuf::new(),
                output: PathBuf::new(),
                views: PathBuf::new(),
            },
            plan: ProbePlan {
                plan_id: p["plan_id"].as_str().unwrap().to_owned(),
                project: serde_json::from_value(p["project"].clone()).unwrap(),
                context: serde_json::from_value(p["context"].clone()).unwrap(),
                views: serde_json::from_value(p["views"].clone()).unwrap(),
            },
            project_before: p["project"].clone(),
            runs_before: vec![],
            next_step: 0,
            in_flight: false,
            terminal: false,
            records: vec![],
            trace_sequence: 0,
        };
        let request: FrameRequest = serde_json::from_value(fixture["request"].clone()).unwrap();
        validate_frame(&probe,&request).expect("The actual JS JSON.stringify numeric spelling is valid for the exact planned typed view");
        for (name, path, value) in [
            ("camera", "/frame/camera/eye/x", json!(24)),
            (
                "source",
                "/frame/source_key",
                json!("[\"other-session\",\"P\",{\"kind\":\"project\"},11,\"P\",11,null]"),
            ),
            ("scope", "/frame/scope", json!({"kind":"workspace"})),
            ("project", "/frame/project_id", json!("other-project")),
            ("scope_revision", "/frame/scene_revision", json!(12)),
            ("project_revision", "/frame/project_revision", json!(12)),
            ("viewport", "/frame/pixels/width", json!(0)),
        ] {
            let mut changed = fixture["request"].clone();
            *changed.pointer_mut(path).unwrap() = value;
            let changed: FrameRequest = serde_json::from_value(changed).unwrap();
            assert!(
                validate_frame(&probe, &changed).is_err(),
                "Must reject changed {name}"
            );
        }
        for (path, value) in [("/2/camera/eye/x", json!(24)), ("/1", json!(12))] {
            let mut changed = fixture["request"].clone();
            let mut key: Value =
                serde_json::from_str(changed["frame"]["key"].as_str().unwrap()).unwrap();
            *key.pointer_mut(path).unwrap() = value;
            changed["frame"]["key"] = json!(key.to_string());
            let changed: FrameRequest = serde_json::from_value(changed).unwrap();
            assert!(
                validate_frame(&probe, &changed).is_err(),
                "Must reject changed typed tuple {path}"
            );
        }
    }
    #[cfg(feature = "custom-protocol")]
    #[test]
    fn frozen_directory_override_embeds_the_actual_product_entry_in_generated_context() {
        use tauri::utils::{assets::AssetKey, config::FrontendDist};
        let native_config_parent = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let runtime = native_config_parent
            .join("../../../scenarios/Agent共建/runtime-native-product-capture");
        let old: Value =
            serde_json::from_slice(&std::fs::read(runtime.join("tauri.frozen-dist.json")).unwrap())
                .unwrap();
        let old: FrontendDist =
            serde_json::from_value(old["build"]["frontendDist"].clone()).unwrap();
        let FrontendDist::Url(ref bad_url) = old else {
            panic!("Original drive string must reproduce the observed URL interpretation")
        };
        assert_eq!(bad_url.scheme(), "d");
        let fixed: Value = serde_json::from_slice(
            &std::fs::read(runtime.join("tauri.frozen-dist.embedded.json")).unwrap(),
        )
        .unwrap();
        let fixed: FrontendDist =
            serde_json::from_value(fixed["build"]["frontendDist"].clone()).unwrap();
        let FrontendDist::Directory(ref relative) = fixed else {
            panic!("Corrected override must be a directory")
        };
        assert_eq!(
            native_config_parent.join(relative).canonicalize().unwrap(),
            runtime.join("dist-frozen").canonicalize().unwrap()
        );
        let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
        assert!(
            matches!(
                context.config().build.frontend_dist,
                Some(FrontendDist::Directory(_))
            ),
            "Production generated context must embed local assets, not navigate to an external directory URL"
        );
        let index = context
            .assets()
            .get(&AssetKey::from("index.html"))
            .expect("Embedded product index.html");
        let index = std::str::from_utf8(&index).unwrap();
        let source = index
            .split("src=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(source.ends_with(".js"));
        let javascript = context
            .assets()
            .get(&AssetKey::from(source))
            .expect("Embedded product entry JS");
        assert!(
            std::str::from_utf8(&javascript)
                .unwrap()
                .contains("native_capture_probe_plan")
        );
        let frozen_entry = runtime
            .join("dist-frozen")
            .join(source.trim_start_matches('/'));
        assert_eq!(&*javascript, std::fs::read(frozen_entry).unwrap());
        println!(
            "{}",
            json!({"old_frontend_dist":format!("{old:?}"),"fixed_frontend_dist":format!("{fixed:?}"),"generated_frontend_dist":format!("{:?}",context.config().build.frontend_dist),"entry":source,"javascript_bytes":javascript.len(),"same_original_frozen_bytes":true,"Native_started":false})
        );
    }
    #[test]
    fn normal_launch_has_no_probe_plan_or_archive_paths() {
        assert!(
            ProbeConfig::parse(&["superfactory".into()])
                .unwrap()
                .is_none()
        );
        assert!(
            NativeCaptureProbeState::default()
                .0
                .lock()
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn private_cli_requires_three_explicit_absolute_paths_and_rejects_mixed_modes() {
        assert!(ProbeConfig::parse(&["--p3-04-capture-smoke".into()]).is_err());
        let args: Vec<_> = [
            "--p3-04-capture-smoke",
            "--capture-input",
            "D:/source",
            "--capture-output",
            "D:/new-output",
            "--capture-views",
            "D:/views.json",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        assert!(ProbeConfig::parse(&args).unwrap().is_some());
        let mut mixed = args;
        mixed.push("--s07-smoke".into());
        assert!(ProbeConfig::parse(&mixed).is_err());
    }
    #[test]
    fn private_frame_does_not_accept_image_path_or_window_carriers() {
        let camera = json!({"coordinate_frame":"right_handed_z_up","unit":"meter","eye":{"x":23,"y":17,"z":12},"target":{"x":15,"y":8,"z":1},"zoom":1});
        let frame = json!({"key":"K","frame_id":1,"camera":camera,"lod":"detailed",
            "pixels":{"width":1440,"height":900,"css_width":1440,"css_height":900,"pixel_ratio":1},
            "visible_objects":[],"limitations":[],"source_key":"S","project_id":"P","scope":{"kind":"project"},
            "scene_revision":11,"project_revision":11,"view":{"kind":"line_canvas","canvas":"line_canvas","hall_id":"H","object_ref":null,
                "project_revision":11,"run_id":null,"view_time_ms":null,"lod":"detailed","camera":camera}});
        let accepted = json!({"plan_id":"P","step":0,"frame":frame});
        assert!(serde_json::from_value::<FrameRequest>(accepted.clone()).is_ok());
        for (key, value) in [
            ("png", json!("untrusted")),
            ("path", json!("D:/file")),
            ("window", json!("main")),
        ] {
            let mut carrier = accepted.clone();
            carrier[key] = value.clone();
            assert!(serde_json::from_value::<FrameRequest>(carrier).is_err());
            let mut nested = accepted.clone();
            nested["frame"][key] = value;
            assert!(serde_json::from_value::<FrameRequest>(nested).is_err());
        }
        assert!(
            serde_json::from_value::<FailureRequest>(
                json!({"plan_id":"P","step":0,"key":"K","stage":"source","message":"M","png":[]})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<RendererEvent>(
                json!({"stage":"dom_content_loaded","message":"interactive"})
            )
            .is_ok()
        );
        for field in ["png", "path", "window", "html"] {
            let mut event = json!({"stage":"error","message":"actual error"});
            event[field] = json!("untrusted");
            assert!(serde_json::from_value::<RendererEvent>(event).is_err());
        }
        assert!(
            serde_json::from_value::<RendererEvent>(
                json!({"stage":"ready","message":"fake readiness"})
            )
            .is_err()
        );
    }
    #[test]
    fn other_windows_do_not_inherit_hidden_main_read_authority() {
        assert!(main_only("main").is_ok());
        for label in ["capture", "agent", "other", "main/child"] {
            assert!(main_only(label).is_err());
        }
    }
}
