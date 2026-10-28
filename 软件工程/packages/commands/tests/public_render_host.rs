//! Actual Host/FIFO/source/authority tests. The controlled Rust backend returns
//! one declared fixture PNG; this is NOT Native, product pixels or UI acceptance.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc,
};
use support::*;
static SESSIONS: AtomicU64 = AtomicU64::new(1);
static CAPTURES: AtomicU64 = AtomicU64::new(1);

fn instance_override_presentation(detailed: bool, simplified: bool) -> Value {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tools/type-contract-proof/fixtures/module-devices.json"
    ))
    .unwrap();
    let mut presentation = fixture["data"]["definition"]["payload"]["data"]["presentation"].clone();
    presentation["kind"] = json!("model");
    presentation["source"] = json!({"kind":"media_ref","data":{"id":"inspector.glb","version":"1.0.0"}});
    let second = json!({"kind":"media_ref","data":{"id":"inspector.glb","version":"2.0.0"}});
    presentation["lod"]["detailed"] = if detailed { second.clone() } else { Value::Null };
    presentation["lod"]["simplified"] = if simplified { second } else { Value::Null };
    presentation
}

#[test]
fn instance_override_source_and_each_lod_freeze_only_the_exact_referenced_media_closure() {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    register_glb(&mut h, "1.0.0");
    register_glb(&mut h, "2.0.0");
    for (detailed, simplified) in [(false, false), (true, false), (false, true)] {
        let mut device = h.object("device", "ASM-01");
        device["presentation_override"] = instance_override_presentation(detailed, simplified);
        let revision = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(revision, json!([put("device", device)])));
        let before = h.object("project", "P1");
        for kind in ["device_detail", "device_cycle_preview"] {
            let mut request = params(&mut h, kind);
            request["object_id"] = json!({"kind":"device","id":"ASM-01"});
            if kind == "device_cycle_preview" {
                request["preview"] = json!({"phase_id":null,"progress":0.5,"pose_name":null});
            }
            let captured = visual_supplement_finish(&mut h, &backend, request);
            let media: Vec<_> = captured.input.modules.iter()
                .filter(|p| p.definition.id.as_str().starts_with("inspector.glb"))
                .collect();
            assert_eq!(media.len(), if detailed || simplified { 4 } else { 2 });
            for version in ["1.0.0", "2.0.0"] {
                let package = media.iter().find(|p|
                    p.definition.id.as_str() == "inspector.glb"
                        && json!(p.definition.version) == json!(version));
                if version == "2.0.0" && !(detailed || simplified) {
                    assert!(package.is_none(), "unreferenced registry version leaked into Task");
                    continue;
                }
                let package = package.expect("explicit instance media is absent from Task");
                let asset = package.assets.iter().find(|a| matches!(a.kind, AssetKind::Glb)).unwrap();
                let source = ObjectSource::ModuleVersion { module_ref: ModuleRef {
                    id: package.definition.id.clone(), version: package.definition.version.clone(),
                }};
                assert_eq!(&captured.input.read_asset(&source, asset).unwrap()[..4], b"glTF");
            }
            assert_eq!(h.object("project", "P1"), before);
        }
    }
}

#[test]
fn instance_override_in_current_draft_cannot_enter_an_older_frozen_run() {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    let handle = h.run("LOCAL");
    h.finish_one();
    let old_run = h.completed_run(&handle);
    register_glb(&mut h, "1.0.0");
    let mut device = h.object("device", "ASM-01");
    device["presentation_override"] = instance_override_presentation(false, false);
    let revision = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(revision, json!([put("device", device)])));
    let current = h.object("project", "P1");
    let mut request = params(&mut h, "device_detail");
    request["object_id"] = json!({"kind":"device","id":"ASM-01"});
    request["view"]["run_id"] = json!(old_run.id);
    request["view"]["view_time_ms"] = json!(120000);
    request["view"]["project_revision"] = json!(old_run.input_ref.project_revision);
    let captured = visual_supplement_finish(&mut h, &backend, request);
    assert!(captured.input.modules.iter().all(|p|
        !p.definition.id.as_str().starts_with("inspector.glb")));
    assert_eq!(json!(h.full(&old_run.id)), json!(old_run));
    assert_eq!(h.object("project", "P1"), current);
}

// These are received Host DTOs, never reconstructed requests or product pixels.
// Bytes remain private fixture material, with exact typed selectors beside them.
fn retain_received_input(job: &RenderJob) {
    let ordinal = CAPTURES.fetch_add(1, Ordering::Relaxed);
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../scenarios/Agent共建/runtime-public-render-host/actual-inputs/session-{}/capture-{}",
        std::process::id(),
        ordinal
    ));
    if ordinal == 1 {
        println!(
            "ACTUAL_RENDER_INPUTS={}",
            directory.parent().unwrap().display()
        );
    }
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("frame.json"),
        serde_json::to_vec_pretty(&job.frame).unwrap(),
    )
    .unwrap();
    std::fs::write(
        directory.join("input.json"),
        serde_json::to_vec_pretty(job.input.as_ref()).unwrap(),
    )
    .unwrap();
    let mut assets = vec![];
    for (i, entry) in job.input.assets.iter().enumerate() {
        let path = format!("asset-{i}.bin");
        let bytes = job.input.read_asset(&entry.source, &entry.asset).unwrap();
        std::fs::write(directory.join(&path), &bytes).unwrap();
        assets.push(json!({"source":entry.source,"asset":entry.asset,"bytes_file":path,"length":bytes.len()}));
    }
    let mut files = vec![];
    for (i, entry) in job.input.files.iter().enumerate() {
        let path = format!("file-{i}.bin");
        let bytes = job.input.read_file(&entry.file).unwrap();
        std::fs::write(directory.join(&path), &bytes).unwrap();
        files.push(json!({"file":entry.file,"bytes_file":path,"length":bytes.len()}));
    }
    std::fs::write(directory.join("bytes.json"), serde_json::to_vec_pretty(&json!({
        "evidence":"actual Host RenderBackend input; controlled Rust backend; not Native capture",
        "test":std::thread::current().name(),"assets":assets,"files":files
    })).unwrap()).unwrap();
}

#[derive(Clone, Copy)]
enum Mode {
    Ok,
    Fail,
    WrongFrame,
    WrongVisible,
    WrongDraftVisible,
    WrongMime,
}
struct ControlBackend {
    modes: Mode,
    kinds: Vec<RenderViewKind>,
    jobs: Mutex<Vec<RenderJob>>,
    started: Option<mpsc::Sender<()>>,
    release: Mutex<Option<mpsc::Receiver<()>>>,
}
impl ControlBackend {
    fn new(mode: Mode) -> Arc<Self> {
        Arc::new(Self {
            modes: mode,
            kinds: kinds(),
            jobs: Mutex::new(vec![]),
            started: None,
            release: Mutex::new(None),
        })
    }
    fn blocking() -> (Arc<Self>, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (tx, started) = mpsc::channel();
        let (release, rx) = mpsc::channel();
        (
            Arc::new(Self {
                modes: Mode::Ok,
                kinds: kinds(),
                jobs: Mutex::new(vec![]),
                started: Some(tx),
                release: Mutex::new(Some(rx)),
            }),
            started,
            release,
        )
    }
    fn latest(&self) -> RenderJob {
        self.jobs.lock().unwrap().last().unwrap().clone()
    }
}
fn failure(code: ErrorCode, message: &str) -> ApiError {
    ApiError {
        code,
        message: message.into(),
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}
impl RenderBackend for ControlBackend {
    fn supported_kinds(&self) -> Vec<RenderViewKind> {
        self.kinds.clone()
    }
    fn render(&self, job: RenderJob, cancelled: &AtomicBool) -> Result<RenderedCapture, ApiError> {
        self.jobs.lock().unwrap().push(job.clone());
        if matches!(self.modes, Mode::Ok) {
            retain_received_input(&job);
        }
        if let Some(started) = &self.started {
            started.send(()).unwrap();
            self.release.lock().unwrap().take().unwrap().recv().unwrap();
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(failure(
                ErrorCode::Cancelled,
                "controlled backend observes actual cancellation",
            ));
        }
        if matches!(job.frame.request.delivery, RenderDelivery::ShowAndCapture)
            && !job.show_authorized.load(Ordering::Acquire)
        {
            return Err(failure(
                ErrorCode::PermissionDenied,
                "view control expired immediately before controlled display",
            ));
        }
        if matches!(self.modes, Mode::Fail) {
            return Err(failure(
                ErrorCode::MissingReference,
                "controlled backend resource failure, no fallback image",
            ));
        }
        let mut frame = job.frame.clone();
        if matches!(self.modes, Mode::WrongFrame) {
            frame.request.view.camera.zoom = FiniteNumber::try_from(7.).unwrap();
        }
        let visible = if matches!(self.modes, Mode::WrongVisible) {
            vec![ObjectRef {
                kind: ObjectKind::Device,
                id: id("nonexistent-device"),
            }]
        } else if matches!(self.modes, Mode::WrongDraftVisible) {
            vec![ObjectRef {
                kind: ObjectKind::Device,
                id: id("ASM-01"),
            }]
        } else {
            vec![]
        };
        Ok(RenderedCapture {
            frame,
            scene_id: id("controlled-scene"),
            image_id: "controlled-image".to_owned().try_into().unwrap(),
            mime_type: if matches!(self.modes, Mode::WrongMime) {
                "text/html"
            } else {
                "image/png"
            }
            .into(),
            bytes: include_bytes!(
                "../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png"
            )
            .to_vec(),
            visible_objects: visible,
            issues: vec![],
        })
    }
}
fn kinds() -> Vec<RenderViewKind> {
    serde_json::from_value(json!([
        "line_canvas",
        "multi_hall_canvas",
        "device_detail",
        "module_preview",
        "packaging_workbench",
        "packaging_manufacturing",
        "device_cycle_preview",
        "timeline_snapshot",
        "comparison"
    ]))
    .unwrap()
}
fn params(h: &mut Harness, kind: &str) -> Value {
    let context = h.context();
    let project = context.project.unwrap();
    let snapshot = h.snapshot();
    let camera = &snapshot["project"]["views"]["line_camera"];
    json!({"revision":project.revision,"view":{"kind":kind,"canvas":if kind=="multi_hall_canvas"{"multi_hall_canvas"}else{"line_canvas"},"hall_id":null,"object_ref":null,"project_revision":project.revision,"run_id":null,"view_time_ms":null,"lod":"detailed","camera":camera},"object_id":null,"preview":null,"delivery":"capture_only"})
}
fn call(h: &mut Harness, session: &ObjectId, scope: &ApiScope, op: &str, params: Value) -> Value {
    let project = h.context().project.unwrap().id;
    h.host.dispatch(session,serde_json::from_value(json!({"api_version":"0.1","request_id":"render-host-control","project_id":project,"scope":scope,"operation":op,"params":params})).unwrap())
}
fn submit(h: &mut Harness, params: Value) -> TaskHandle {
    ok(call(
        h,
        &h.ui.clone(),
        &ApiScope::Project,
        "render_view",
        params,
    ))
}
fn task(h: &mut Harness, handle: &TaskHandle) -> TaskState {
    ok(call(
        h,
        &h.ui.clone(),
        &ApiScope::Project,
        "get_task",
        json!({"task_id":handle.id}),
    ))
}
fn reject(reply: Value, code: ErrorCode) {
    assert_eq!(
        serde_json::to_value(error(reply).code).unwrap(),
        serde_json::to_value(code).unwrap()
    );
}
fn external(h: &mut Harness, operations: Vec<OperationName>, show: bool) -> ObjectId {
    let session = id(&format!(
        "render-external-{}",
        SESSIONS.fetch_add(1, Ordering::Relaxed)
    ));
    let project_id = h.context().project.unwrap().id;
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            session_id: session.clone(),
            granted_project: Some(project_id),
            allowed_operations: operations,
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
            allow_view_control: show,
        })
        .unwrap();
    session
}
fn all_ops() -> Vec<OperationName> {
    vec![
        OperationName::RenderView,
        OperationName::GetTask,
        OperationName::CancelTask,
    ]
}

#[test]
fn no_backend_is_not_capability_and_queued_render_uses_original_fifo_running_task_and_exact_result_bytes()
 {
    let mut h = Harness::new();
    let before = h.snapshot();
    let mut p = params(&mut h, "line_canvas");
    reject(
        call(
            &mut h,
            &id("unregistered-render-review"),
            &ApiScope::Project,
            "render_view",
            p.clone(),
        ),
        ErrorCode::PermissionDenied,
    );
    let ui = h.ui.clone();
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p.clone()),
        ErrorCode::UnsupportedCapability,
    );
    assert!(h.host.drain_io_work().is_empty());
    let (backend, started, release) = ControlBackend::blocking();
    h.host.set_render_backend(backend.clone());
    let handle = submit(&mut h, p.take());
    assert!(matches!(handle.kind, LongTaskKind::Rendering));
    assert!(matches!(handle.state, ComputationStatus::Queued));
    let work = h.host.drain_io_work().pop().unwrap();
    let worker = std::thread::spawn(move || work.run());
    started.recv().unwrap();
    assert!(matches!(
        task(&mut h, &handle).task.state,
        ComputationStatus::Running
    ));
    release.send(()).unwrap();
    h.host.finish_io(worker.join().unwrap());
    let state = task(&mut h, &handle);
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    let Some(TaskOutput::Render(render)) = state.result else {
        panic!("render")
    };
    assert_eq!(
        serde_json::to_value(&render.requested_scope_revision).unwrap(),
        json!(1)
    );
    let project = h.context().project.unwrap().id;
    let image = h
        .host
        .read_rendered_image(
            &h.ui,
            &project,
            &ApiScope::Project,
            &handle.id,
            &render.scene.file.id,
        )
        .unwrap();
    assert_eq!(image.mime_type, "image/png");
    assert_eq!(
        image.bytes.as_ref(),
        include_bytes!("../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png")
    );
    let after = h.snapshot();
    assert_eq!(after["project"], before["project"]);
    assert_eq!(after["scenarios"], before["scenarios"]);
    assert_eq!(
        serde_json::to_value(backend.latest().frame.request.view).unwrap(),
        serde_json::to_value(render.scene.view).unwrap()
    );
}

#[test]
fn failure_and_mismatched_frame_visible_source_or_mime_never_publish_an_image() {
    for mode in [
        Mode::Fail,
        Mode::WrongFrame,
        Mode::WrongVisible,
        Mode::WrongMime,
    ] {
        let mut h = Harness::new();
        h.host.set_render_backend(ControlBackend::new(mode));
        let p = params(&mut h, "line_canvas");
        let handle = submit(&mut h, p);
        h.finish_one();
        let state = task(&mut h, &handle);
        assert!(
            matches!(state.task.state, ComputationStatus::Failed),
            "{state:?}"
        );
        assert!(state.result.is_none());
        let project = h.context().project.unwrap().id;
        assert!(
            h.host
                .read_rendered_image(
                    &h.ui,
                    &project,
                    &ApiScope::Project,
                    &handle.id,
                    &"controlled-image".to_owned().try_into().unwrap()
                )
                .is_err()
        );
    }
}

#[test]
fn original_cancel_race_before_worker_during_capture_before_commit_and_after_completion() {
    for stage in ["queued", "before-commit", "completed"] {
        let mut h = Harness::new();
        let backend = ControlBackend::new(Mode::Ok);
        h.host.set_render_backend(backend);
        let p = params(&mut h, "line_canvas");
        let handle = submit(&mut h, p);
        let ui = h.ui.clone();
        let work = h.host.drain_io_work().pop().unwrap();
        if stage == "queued" {
            let _: CancellationState = ok(call(
                &mut h,
                &ui,
                &ApiScope::Project,
                "cancel_task",
                json!({"task_id":handle.id,"action":"cancel"}),
            ));
        }
        let completion = work.run();
        if stage == "before-commit" {
            let _: CancellationState = ok(call(
                &mut h,
                &ui,
                &ApiScope::Project,
                "cancel_task",
                json!({"task_id":handle.id,"action":"cancel"}),
            ));
        }
        h.host.finish_io(completion);
        let state = task(&mut h, &handle);
        if matches!(stage, "queued" | "before-commit") {
            assert!(matches!(state.task.state, ComputationStatus::Cancelled));
            assert!(state.result.is_none());
        } else {
            assert!(matches!(state.task.state, ComputationStatus::Completed));
            let cancel: CancellationState = ok(call(
                &mut h,
                &ui,
                &ApiScope::Project,
                "cancel_task",
                json!({"task_id":handle.id,"action":"cancel"}),
            ));
            assert!(cancel.terminal_won);
            assert!(!cancel.action_accepted);
        }
    }
}

#[test]
fn actual_running_cancel_and_view_control_revocation_are_distinct_from_capture_only_disconnect() {
    for (show, action) in [(false, "cancel"), (true, "revoke"), (false, "disconnect")] {
        let mut h = Harness::new();
        let (backend, started, release) = ControlBackend::blocking();
        h.host.set_render_backend(backend.clone());
        let session = external(&mut h, all_ops(), show);
        let mut p = params(&mut h, "line_canvas");
        if show {
            p["delivery"] = json!("show_and_capture");
        }
        let handle: TaskHandle = ok(call(&mut h, &session, &ApiScope::Project, "render_view", p));
        let work = h.host.drain_io_work().pop().unwrap();
        let worker = std::thread::spawn(move || work.run());
        started.recv().unwrap();
        if action == "cancel" {
            let _: CancellationState = ok(call(
                &mut h,
                &session,
                &ApiScope::Project,
                "cancel_task",
                json!({"task_id":handle.id,"action":"cancel"}),
            ));
        } else {
            h.host.revoke_external_session(&session).unwrap();
        }
        release.send(()).unwrap();
        h.host.finish_io(worker.join().unwrap());
        if action == "cancel" {
            let state: TaskState = ok(call(
                &mut h,
                &session,
                &ApiScope::Project,
                "get_task",
                json!({"task_id":handle.id}),
            ));
            assert!(matches!(state.task.state, ComputationStatus::Cancelled));
        } else {
            reject(
                call(
                    &mut h,
                    &session,
                    &ApiScope::Project,
                    "get_task",
                    json!({"task_id":handle.id}),
                ),
                ErrorCode::PermissionDenied,
            );
            let next = external(&mut h, all_ops(), false);
            h.host
                .grant_external_task_access(&h.ui, &next, &handle.id, false)
                .unwrap();
            let state: TaskState = ok(call(
                &mut h,
                &next,
                &ApiScope::Project,
                "get_task",
                json!({"task_id":handle.id}),
            ));
            assert!(
                if show {
                    matches!(state.task.state, ComputationStatus::Failed)
                } else {
                    matches!(state.task.state, ComputationStatus::Completed)
                },
                "{state:?}"
            );
        }
    }
}

#[test]
fn precise_revision_target_carrier_frontier_and_scope_rejections_do_not_enqueue() {
    let mut h = Harness::new();
    h.host.set_render_backend(ControlBackend::new(Mode::Ok));
    let base = params(&mut h, "line_canvas");
    let ui = h.ui.clone();
    let mut bad = base.clone();
    bad["revision"] = json!(99);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", bad),
        ErrorCode::RevisionConflict,
    );
    let mut bad = base.clone();
    bad["view"]["project_revision"] = json!(99);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", bad),
        ErrorCode::RevisionConflict,
    );
    let mut bad = base.clone();
    bad["view"]["object_ref"] = json!({"kind":"line","id":"different"});
    bad["object_id"] = json!({"kind":"device","id":"ASM-01"});
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", bad),
        ErrorCode::InvalidInput,
    );
    for field in ["module_ref", "comparison", "preview"] {
        let mut bad = base.clone();
        bad[field] = match field {
            "module_ref" => json!({"id":"fixture.assembly-station","version":"0.1.0"}),
            "comparison" => {
                json!({"run_ids":[],"metric_scope":{"keys":[],"objects":[],"acceptance":[]}})
            }
            _ => json!({"phase_id":null,"progress":0.5,"pose_name":null}),
        };
        reject(
            call(&mut h, &ui, &ApiScope::Project, "render_view", bad),
            ErrorCode::InvalidInput,
        );
    }
    let mut bad = base.clone();
    bad["view"]["view_time_ms"] = json!(0);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", bad),
        ErrorCode::InvalidInput,
    );
    let req=serde_json::from_value(json!({"api_version":"0.1","request_id":"workspace-render","project_id":null,"scope":{"kind":"workspace"},"operation":"render_view","params":base})).unwrap();
    reject(h.host.dispatch(&ui, req), ErrorCode::UnsupportedCapability);
    assert!(h.host.drain_io_work().is_empty());
    let run = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&run);
    for (at, preview) in [
        (240001, Value::Null), // original local-assembly-pack window.end_ms=240000
        (0, json!({"phase_id":null,"progress":0.5,"pose_name":null})),
    ] {
        let mut bad = params(&mut h, "line_canvas");
        bad["view"]["run_id"] = json!(run.id);
        bad["view"]["view_time_ms"] = json!(at);
        bad["preview"] = preview;
        reject(
            call(&mut h, &ui, &ApiScope::Project, "render_view", bad),
            ErrorCode::InvalidInput,
        );
    }
    assert!(h.host.drain_io_work().is_empty());
    let no_show = external(&mut h, all_ops(), false);
    let mut show = params(&mut h, "line_canvas");
    show["delivery"] = json!("show_and_capture");
    reject(
        call(&mut h, &no_show, &ApiScope::Project, "render_view", show),
        ErrorCode::PermissionDenied,
    );
}

fn packaging(h: &mut Harness) -> PackagingResult {
    packaging_case(h, "HOLD01", "pass")
}
fn packaging_case(h: &mut Harness, case_id: &str, expected: &str) -> PackagingResult {
    let input: Value = serde_json::from_str(include_str!(
        "fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    let case = input["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == case_id)
        .unwrap();
    assert_eq!(case["expected"], expected);
    let job = case["job"]["data"].clone();
    let r = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(r, json!([{"kind":"put_packaging_job","data":job}])));
    let r = h.context().project.unwrap().revision;
    let handle: TaskHandle = ok(h.call(
        "compute_packaging",
        json!({"packaging_job":job,"input_revision":r,"computation_budget":job["budget"]}),
    ));
    h.finish_one();
    let state = h.task(&handle, true);
    let Some(TaskOutput::Packaging(result)) = state.result else {
        panic!("actual packaging")
    };
    assert_eq!(json!(result.outcome), json!(expected));
    result
}

#[test]
fn nine_kind_admission_uses_real_saved_runs_comparison_and_actual_packaging_triple_without_business_writes()
 {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    let first = h.run("LOCAL");
    h.finish_one();
    let first = h.completed_run(&first);
    let second = h.run("LOCAL");
    h.finish_one();
    let second = h.completed_run(&second);
    let original_scenario = h.object("scenario", "LOCAL");
    let result = packaging(&mut h);
    assert!(!result.manufacturing_files.is_empty());
    let before = h.snapshot();
    let ui = h.ui.clone();
    for kind in [
        "line_canvas",
        "multi_hall_canvas",
        "device_detail",
        "module_preview",
        "packaging_workbench",
        "packaging_manufacturing",
        "device_cycle_preview",
        "timeline_snapshot",
        "comparison",
    ] {
        let mut p = params(&mut h, kind);
        match kind {
            "device_detail" => {
                p["object_id"] = json!({"kind":"device","id":"ASM-01"});
                p["view"]["run_id"] = json!(first.id);
                p["view"]["view_time_ms"] = json!(120000);
                p["view"]["project_revision"] = json!(1);
            }
            "module_preview" => {
                p["object_id"] = json!({"kind":"module","id":"fixture.assembly-station"});
                p["module_ref"] = json!({"id":"fixture.assembly-station","version":"0.1.0"});
            }
            "packaging_workbench" | "packaging_manufacturing" => {
                p["object_id"] = json!({"kind":"artifact","id":result.artifact_id})
            }
            "device_cycle_preview" => {
                p["object_id"] = json!({"kind":"device","id":"ASM-01"});
                p["preview"] = json!({"phase_id":null,"progress":0.5,"pose_name":null});
            }
            "timeline_snapshot" => {
                p["view"]["run_id"] = json!(first.id);
                p["view"]["view_time_ms"] = json!(75000);
                p["view"]["project_revision"] = json!(1);
            }
            "comparison" => {
                p["comparison"] = json!({"run_ids":[first.id,second.id],"metric_scope":{"keys":["accepted_quantity"],"objects":[],"acceptance":[]}})
            }
            _ => (),
        }
        let handle = submit(&mut h, p);
        h.finish_one();
        let state = task(&mut h, &handle);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{kind}: {state:?}"
        );
        let job = backend.latest();
        if kind == "timeline_snapshot" {
            assert!(!job.input.port_output_summaries.is_empty());
            let project = h.context().project.unwrap();
            for summary in &job.input.port_output_summaries {
                let original = h
                    .host
                    .read_run_port_output(
                        &ui,
                        &project.id,
                        &ApiScope::Project,
                        &project.revision,
                        &summary.run_id,
                        &summary.at_ms,
                        &summary.port,
                    )
                    .unwrap();
                assert_eq!(json!(summary), json!(original));
            }
        }
        match kind {
            "packaging_workbench" => {
                let triple = job.input.packaging.as_ref().unwrap();
                assert_eq!(triple.artifact.id, result.artifact_id);
                assert!(triple.sample.is_none());
            }
            "packaging_manufacturing" => {
                assert_eq!(job.input.files.len(), result.manufacturing_files.len());
                for file in &job.input.files {
                    assert!(String::from_utf8_lossy(&file.bytes).contains("<svg"));
                }
            }
            "comparison" => {
                assert!(job.input.comparison.is_some());
                assert_eq!(job.input.comparison_runs.len(), 2);
                assert_eq!(json!(job.input.comparison_runs[0].run), json!(first));
                assert_eq!(json!(job.input.comparison_runs[1].run), json!(second));
                assert_eq!(
                    json!(job.input.comparison_runs[0].project.revision),
                    json!(1)
                );
                assert_eq!(
                    json!(job.input.comparison_runs[0].scenario),
                    original_scenario
                );
            }
            "device_detail" => {
                assert!(job.input.active_job_id.is_none());
                assert!(
                    job.input
                        .issues
                        .iter()
                        .any(|i| i.code == "render.frozen_static_device")
                );
            }
            _ => (),
        }
        assert_eq!(h.snapshot()["project"], before["project"]);
        assert_eq!(json!(h.full(&first.id)), json!(first));
        assert_eq!(json!(h.full(&second.id)), json!(second));
    }
    // Preserve actual current/static and frozen factory/cycle DTOs for the same
    // product consumer, without replaying a different producer or SDK.
    for (kind, at) in [
        ("line_canvas", Some(75000)),
        ("multi_hall_canvas", Some(75000)),
        ("device_detail", None),
        ("device_cycle_preview", Some(30000)),
        ("timeline_snapshot", None),
    ] {
        let mut p = params(&mut h, kind);
        if kind.starts_with("device_") {
            p["object_id"] = json!({"kind":"device","id":"ASM-01"});
        }
        if let Some(at) = at {
            p["view"]["run_id"] = json!(first.id);
            p["view"]["view_time_ms"] = json!(at);
            p["view"]["project_revision"] = json!(1);
        } else if kind == "timeline_snapshot" {
            p["object_id"] = json!({"kind":"scenario","id":"LOCAL"});
        }
        let handle = submit(&mut h, p);
        h.finish_one();
        assert!(matches!(
            task(&mut h, &handle).task.state,
            ComputationStatus::Completed
        ));
    }
    let mut inactive = params(&mut h, "device_cycle_preview");
    inactive["object_id"] = json!({"kind":"device","id":"ASM-01"});
    inactive["view"]["run_id"] = json!(first.id);
    inactive["view"]["view_time_ms"] = json!(75000);
    inactive["view"]["project_revision"] = json!(1);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", inactive),
        ErrorCode::UnsupportedCapability,
    );
    // One saved process sample is selected by exact step/progress, never index.
    let sample = result
        .process_poses
        .first()
        .expect("HOLD01 actual process sample");
    let mut p = params(&mut h, "packaging_workbench");
    p["object_id"] = json!({"kind":"artifact","id":result.artifact_id});
    p["preview"] = json!({"phase_id":sample.step_id,"progress":sample.progress,"pose_name":null});
    let handle = submit(&mut h, p.clone());
    h.finish_one();
    assert!(matches!(
        task(&mut h, &handle).task.state,
        ComputationStatus::Completed
    ));
    assert_eq!(
        serde_json::to_value(
            backend
                .latest()
                .input
                .packaging
                .as_ref()
                .unwrap()
                .sample
                .as_ref()
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(sample).unwrap()
    );
    p["preview"]["progress"] = json!(0.123456789);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p.clone()),
        ErrorCode::InvalidInput,
    );
    p["preview"]["progress"] = json!(sample.progress);
    p["preview"]["pose_name"] = json!("invented");
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p),
        ErrorCode::InvalidInput,
    );
}

#[test]
fn saved_packaging_three_outcomes_are_preserved_as_real_inputs_and_svg_is_never_invented() {
    for (id, expected) in [
        ("HOLD01", "pass"),
        ("HOLD03", "fail"),
        ("LOCAL_ZERO_QUERY", "undetermined"),
    ] {
        let mut h = Harness::new();
        let backend = ControlBackend::new(Mode::Ok);
        h.host.set_render_backend(backend.clone());
        let result = packaging_case(&mut h, id, expected);
        let before = h.snapshot();
        let mut p = params(&mut h, "packaging_workbench");
        p["object_id"] = json!({"kind":"artifact","id":result.artifact_id});
        let handle = submit(&mut h, p);
        h.finish_one();
        assert!(matches!(
            task(&mut h, &handle).task.state,
            ComputationStatus::Completed
        ));
        assert_eq!(
            json!(backend.latest().input.packaging.as_ref().unwrap().result),
            json!(result)
        );
        let mut p = params(&mut h, "packaging_manufacturing");
        p["object_id"] = json!({"kind":"artifact","id":result.artifact_id});
        if result.manufacturing_files.is_empty() {
            let ui = h.ui.clone();
            reject(
                call(&mut h, &ui, &ApiScope::Project, "render_view", p),
                ErrorCode::MissingReference,
            );
            assert!(h.host.drain_io_work().is_empty());
        } else {
            let handle = submit(&mut h, p);
            h.finish_one();
            assert!(matches!(
                task(&mut h, &handle).task.state,
                ComputationStatus::Completed
            ));
            for file in &backend.latest().input.files {
                assert!(String::from_utf8_lossy(&file.bytes).contains("<svg"));
            }
        }
        assert_eq!(h.snapshot()["project"], before["project"]);
    }
}

#[test]
fn moved_saved_run_and_legacy_item_only_run_keep_static_device_readable_without_inventing_job_animation()
 {
    let mut h = Harness::new();
    let run = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&run);
    let saved = h.tree.directory("saved-render-source");
    let save = h.save(1, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    for legacy in [false, true] {
        let tree = Sandbox::new();
        let moved = tree.directory("moved-render-source");
        copy(&saved, &moved);
        if legacy {
            let manifest = read(&moved.join("project.json"));
            let entry = manifest["data"]["documents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| {
                    e["object_ref"]["id"] == json!(run.id)
                        && e["file_ref"]["path"]
                            .as_str()
                            .is_some_and(|p| p.ends_with("/item-history.json"))
                })
                .unwrap();
            let path = moved.join(entry["file_ref"]["path"].as_str().unwrap());
            let mut history = read(&path);
            assert!(
                history["data"]
                    .as_object_mut()
                    .unwrap()
                    .remove("job_records")
                    .is_some()
            );
            assert!(
                history["data"]
                    .as_object_mut()
                    .unwrap()
                    .remove("port_outputs")
                    .is_some()
            );
            write(&path, &history);
        }
        let mut opened = Harness::from_seed(tree, &moved);
        let backend = ControlBackend::new(Mode::Ok);
        opened.host.set_render_backend(backend.clone());
        let before = opened.snapshot();
        let mut p = params(&mut opened, "device_detail");
        p["object_id"] = json!({"kind":"device","id":"ASM-01"});
        p["view"]["run_id"] = json!(run.id);
        p["view"]["view_time_ms"] = json!(30000);
        let handle = submit(&mut opened, p.clone());
        opened.finish_one();
        assert!(matches!(
            task(&mut opened, &handle).task.state,
            ComputationStatus::Completed
        ));
        assert_eq!(json!(opened.full(&run.id)), json!(run));
        if legacy {
            assert!(backend.latest().input.active_job_id.is_none());
            assert!(backend.latest().input.port_output_summaries.is_empty());
            assert!(
                backend
                    .latest()
                    .input
                    .issues
                    .iter()
                    .any(|i| i.code == "render.missing_port_output_history")
            );
            assert!(
                backend.latest().input.issues.iter().any(|i| i.code
                    == "render.missing_job_history"
                    && i.message.contains("unknown"))
            );
            assert!(
                backend
                    .latest()
                    .input
                    .issues
                    .iter()
                    .any(|i| i.code == "render.frozen_static_device")
            );
            p["view"]["kind"] = json!("device_cycle_preview");
            let ui = opened.ui.clone();
            reject(
                call(&mut opened, &ui, &ApiScope::Project, "render_view", p),
                ErrorCode::UnsupportedCapability,
            );
            assert!(opened.host.drain_io_work().is_empty());
            for kind in ["line_canvas", "multi_hall_canvas"] {
                let mut p = params(&mut opened, kind);
                p["view"]["run_id"] = json!(run.id);
                p["view"]["view_time_ms"] = json!(30000);
                let handle = submit(&mut opened, p);
                opened.finish_one();
                assert!(matches!(
                    task(&mut opened, &handle).task.state,
                    ComputationStatus::Completed
                ));
                let job = backend.latest();
                assert!(
                    job.input
                        .objects_at
                        .iter()
                        .all(|o| matches!(o, DomainObject::Item(_)))
                );
                assert!(
                    job.input
                        .issues
                        .iter()
                        .any(|i| i.code == "render.missing_job_history")
                );
            }
        } else {
            assert!(backend.latest().input.active_job_id.is_some());
        }
        assert_eq!(opened.snapshot()["project"], before["project"]);
    }
}

#[test]
fn draft_visible_objects_cannot_claim_an_unrelated_project_device_even_when_its_id_exists() {
    let mut h = Harness::new();
    h.host
        .set_render_backend(ControlBackend::new(Mode::WrongDraftVisible));
    let definition = fixture()["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "fixture.assembly-station")
        .unwrap()
        .clone();
    let scope = draft(&mut h, definition);
    let mut p = params(&mut h, "device_detail");
    p["revision"] = json!(draft_revision(&mut h, &scope));
    let ApiScope::DefinitionDraft(id) = &scope else {
        unreachable!()
    };
    p["object_id"] = json!({"kind":"definition_draft","id":id});
    let ui = h.ui.clone();
    let handle: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", p));
    h.finish_one();
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    assert!(
        matches!(state.task.state, ComputationStatus::Failed),
        "{state:?}"
    );
    assert!(state.result.is_none());
}

#[test]
fn render_only_external_grant_can_freeze_original_run_without_raw_object_or_run_rights_and_image_is_owned()
 {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    let run = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&run);
    let old = h.object("device", "ASM-01");
    let mut new = old.clone();
    new["title"] = json!("new current only");
    let _: EditCommit = ok(h.apply(1, json!([put("device", new)])));
    let session = external(
        &mut h,
        vec![OperationName::RenderView, OperationName::GetTask],
        false,
    );
    let mut p = params(&mut h, "device_detail");
    p["object_id"] = json!({"kind":"device","id":"ASM-01"});
    p["view"]["run_id"] = json!(run.id);
    p["view"]["view_time_ms"] = json!(120000);
    p["view"]["project_revision"] = json!(1);
    let handle: TaskHandle = ok(call(&mut h, &session, &ApiScope::Project, "render_view", p));
    h.finish_one();
    let frozen = backend.latest();
    assert_eq!(
        serde_json::to_value(&frozen.input.project.devices[0]).unwrap(),
        old
    );
    assert_eq!(
        serde_json::to_value(frozen.frame.scope_revision).unwrap(),
        json!(2)
    );
    assert_eq!(
        serde_json::to_value(frozen.frame.request.view.project_revision).unwrap(),
        json!(1)
    );
    let state: TaskState = ok(call(
        &mut h,
        &session,
        &ApiScope::Project,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    let Some(TaskOutput::Render(output)) = state.result else {
        panic!("render")
    };
    let project: ProjectId = "P1".to_owned().try_into().unwrap();
    h.host
        .read_rendered_image(
            &session,
            &project,
            &ApiScope::Project,
            &handle.id,
            &output.scene.file.id,
        )
        .unwrap();
    let other = external(&mut h, all_ops(), false);
    assert!(matches!(
        h.host
            .read_rendered_image(
                &other,
                &project,
                &ApiScope::Project,
                &handle.id,
                &output.scene.file.id
            )
            .err()
            .unwrap()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        h.host
            .read_rendered_image(
                &session,
                &project,
                &ApiScope::Project,
                &handle.id,
                &"wrong-image".to_owned().try_into().unwrap()
            )
            .err()
            .unwrap()
            .code,
        ErrorCode::MissingReference
    ));
    reject(
        call(
            &mut h,
            &session,
            &ApiScope::Project,
            "get_run",
            json!({"run_id":run.id,"query":{"kind":"full_result"}}),
        ),
        ErrorCode::PermissionDenied,
    );
    assert_eq!(json!(h.full(&run.id)), json!(run));
}

fn register_glb(h: &mut Harness, version: &str) {
    let dependency = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../scenarios/媒体表现/runtime-node-inspector/packages/inspector.glb.simple/{version}"
    ));
    h.host
        .register_bundled_module_directory(&dependency)
        .unwrap();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../scenarios/媒体表现/runtime-node-inspector/packages/inspector.glb/{version}"
    ));
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
}

#[test]
fn advertised_draft_scopes_respect_the_only_backend_kind_and_actual_definition_category() {
    let mut h = Harness::new();
    let mut backend = ControlBackend::new(Mode::Ok);
    Arc::get_mut(&mut backend).unwrap().kinds = vec![RenderViewKind::DeviceDetail];
    h.host.set_render_backend(backend);
    let device = fixture()["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "fixture.assembly-station")
        .unwrap()
        .clone();
    let device_scope = draft(&mut h, device);
    let professional: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/professional/module.json"
    ))
    .unwrap();
    let professional_scope = draft(&mut h, professional["data"]["definition"].clone());
    let ui = h.ui.clone();
    let catalog: Value = ok(h.call_as(&ui, false, "describe_capabilities", json!({})));
    let render = catalog["implemented_operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["operation"] == "render_view")
        .unwrap();
    let scopes = render["allowed_scopes"].as_array().unwrap();
    assert!(scopes.contains(&json!(device_scope)), "{render:#}");
    assert!(
        !scopes.contains(&json!(professional_scope)),
        "DeviceDetail-only backend falsely advertises a Professional Draft scope: {render:#}"
    );
}
fn draft(h: &mut Harness, definition: Value) -> ApiScope {
    let revision = h.context().project.unwrap().revision.0.value();
    let commit: EditCommit = ok(h.apply(
        revision,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(draft) = commit.scope else {
        panic!("draft")
    };
    ApiScope::DefinitionDraft(draft.id)
}
fn draft_revision(h: &mut Harness, scope: &ApiScope) -> Revision {
    let ui = h.ui.clone();
    let context: ProjectContext = ok(call(h, &ui, scope, "get_context", json!({"selection":[]})));
    context.definition.unwrap().revision
}

#[test]
fn module_selector_requires_unique_exact_version_and_real_source_bytes_are_frozen_not_current_registry()
 {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    register_glb(&mut h, "1.0.0");
    register_glb(&mut h, "2.0.0");
    let mut p = params(&mut h, "module_preview");
    p["object_id"] = json!({"kind":"module","id":"inspector.glb"});
    let ui = h.ui.clone();
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p.clone()),
        ErrorCode::InvalidInput,
    );
    p["module_ref"] = json!({"id":"inspector.glb","version":"1.0.0"});
    let handle = submit(&mut h, p.clone());
    // Another exact version was already readable at admission, never selected as latest.
    h.finish_one();
    assert!(matches!(
        task(&mut h, &handle).task.state,
        ComputationStatus::Completed
    ));
    let job = backend.latest();
    let package = job
        .input
        .modules
        .iter()
        .find(|p| p.definition.id.as_str() == "inspector.glb")
        .unwrap();
    assert_eq!(json!(package.definition.version), json!("1.0.0"));
    let asset = package
        .assets
        .iter()
        .find(|asset| matches!(asset.kind, AssetKind::Glb))
        .unwrap();
    let source = ObjectSource::ModuleVersion {
        module_ref: ModuleRef {
            id: package.definition.id.clone(),
            version: package.definition.version.clone(),
        },
    };
    let bytes = job.input.read_asset(&source, asset).unwrap();
    assert_eq!(&bytes[..4], b"glTF");
    let wrong = ObjectSource::ModuleVersion {
        module_ref: serde_json::from_value(json!({"id":"inspector.glb","version":"2.0.0"}))
            .unwrap(),
    };
    assert!(job.input.read_asset(&wrong, asset).is_err());
    assert!(
        job.input
            .read_file(&FileRef {
                path: RelativeFilePath::try_from("not-listed.svg".to_owned()).unwrap(),
                kind: DocumentKind::Asset
            })
            .is_err()
    );
    p["module_ref"]["id"] = json!("different");
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p),
        ErrorCode::InvalidInput,
    );
    // A plain CreateDefinitionDraft carries the definition, not bundled bytes.
    // Adopt through the original operation before cloning its actual remapped source.
    let base = h.context().project.unwrap().revision;
    let reference = source;
    let ObjectSource::ModuleVersion { module_ref } = reference else {
        unreachable!()
    };
    let _: EditCommit = ok(h.apply(base.0.value(), json!([{"kind":"adopt_module","data":{
        "module_ref":module_ref,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null
    }}])));
    let object: DomainObject = ok(h.call("get_object", json!({"object_id":{"kind":"module","id":module_ref.id},"source":{"kind":"module_version","data":{"module_ref":module_ref}},"node_index":null})));
    let DomainObject::Module(actual) = object else {
        unreachable!()
    };
    let run_handle = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&run_handle);
    let mut old = params(&mut h, "module_preview");
    old["object_id"] = json!({"kind":"module","id":module_ref.id});
    old["module_ref"] = json!(module_ref);
    old["view"]["run_id"] = json!(run.id);
    old["view"]["view_time_ms"] = json!(30000);
    old["view"]["project_revision"] = json!(run.input_ref.project_revision);
    let handle = submit(&mut h, old);
    h.finish_one();
    assert!(matches!(
        task(&mut h, &handle).task.state,
        ComputationStatus::Completed
    ));
    let frozen = backend.latest();
    let package = frozen
        .input
        .modules
        .iter()
        .find(|p| p.definition.id == module_ref.id && p.definition.version == module_ref.version)
        .unwrap();
    let asset = package
        .assets
        .iter()
        .find(|a| matches!(a.kind, AssetKind::Glb))
        .unwrap();
    assert_eq!(
        frozen
            .input
            .read_asset(
                &ObjectSource::RunModuleVersion {
                    run_id: run.id.clone(),
                    at_ms: SimTimeMs(SafeUint::try_from(30000).unwrap()),
                    module_ref: module_ref.clone()
                },
                asset
            )
            .unwrap(),
        bytes
    );
    assert_eq!(json!(h.full(&run.id)), json!(run));
    let before = h.snapshot();
    let definition = serde_json::to_value(&actual.definition).unwrap();
    let scope = draft(&mut h, definition);
    let mut p = params(&mut h, "module_preview");
    p["revision"] = json!(draft_revision(&mut h, &scope));
    let ApiScope::DefinitionDraft(id) = &scope else {
        unreachable!()
    };
    p["object_id"] = json!({"kind":"definition_draft","id":id});
    let handle: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", p));
    h.finish_one();
    let job = backend.latest();
    let package = job
        .input
        .modules
        .iter()
        .find(|p| p.definition.id.as_str() == "inspector.glb")
        .unwrap();
    let source = ObjectSource::ModuleVersion {
        module_ref: ModuleRef {
            id: package.definition.id.clone(),
            version: package.definition.version.clone(),
        },
    };
    let asset = package
        .assets
        .iter()
        .find(|a| matches!(a.kind, AssetKind::Glb))
        .unwrap();
    assert_eq!(job.input.read_asset(&source, asset).unwrap(), bytes);
    assert_eq!(h.snapshot()["project"], before["project"]);
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
}

#[test]
fn template_preview_uses_actual_private_and_committed_subset_without_instantiation_and_nonvisual_is_explicit()
 {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    let definition = json!({"id":"render.composition","version":"1.0.0","title":"Actual render subset","definition_state":"design","contribution":{"author":null,"license":null,"assets":[]},"parameters":[],"attribute_schema":[],"payload":{"category":"templates","data":{"project":"template/project.subset.json","generation_profile":null}}});
    let project = h.object("project", "P1");
    let requirements:Vec<_>=project["resources"].as_array().unwrap().iter().map(|r|json!({"kind":r["kind"],"role":r["role"],"capacity":r["capacity"],"qualifications":r["qualifications"],"default_calendar_ms":[],"presentation":null})).collect();
    let subset = json!({"layout_scope":"region","halls":project["halls"],"lines":project["lines"],"devices":project["devices"],"connections":project["connections"],"module_refs":project["module_refs"],"resource_requirements":requirements,"item_types":project["item_types"],"role_requirements":[],"suggested_role_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"seed":null,"locations":project["locations"],"adoption":null});
    let scope = draft(&mut h, definition.clone());
    let ui = h.ui.clone();
    let _: EditCommit = ok(call(
        &mut h,
        &ui,
        &scope,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":1,"definition":definition,"subset":subset,"source_scenario":null}}]}),
    ));
    let stored: DomainObject = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_object",
        json!({"object_id":{"kind":"module","id":"render.composition"},"source":{"kind":"template_preview","data":{"module_ref":{"id":"render.composition","version":"1.0.0"},"base_revision":2,"choices":null,"seed":null}},"node_index":null}),
    ));
    let DomainObject::TemplatePreview(stored) = stored else {
        unreachable!()
    };
    let ApiScope::DefinitionDraft(draft_id) = &scope else {
        unreachable!()
    };
    let mut p = params(&mut h, "module_preview");
    p["revision"] = json!(2);
    p["object_id"] = json!({"kind":"definition_draft","id":draft_id});
    let handle: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", p));
    h.finish_one();
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    assert_eq!(
        json!(backend.latest().input.template_subset),
        json!(Some(&stored.subset))
    );
    assert_eq!(json!(stored.subset.devices), subset["devices"]);
    assert_eq!(h.object("project", "P1"), project);
    let _: EditCommit = ok(call(
        &mut h,
        &ui,
        &scope,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"commit_definition_version","data":{"version":"1.0.0"}}]}),
    ));
    let before = h.snapshot();
    let mut p = params(&mut h, "module_preview");
    p["object_id"] = json!({"kind":"module","id":"render.composition"});
    p["module_ref"] = json!({"id":"render.composition","version":"1.0.0"});
    let handle = submit(&mut h, p);
    h.finish_one();
    assert!(matches!(
        task(&mut h, &handle).task.state,
        ComputationStatus::Completed
    ));
    let job = backend.latest();
    assert!(job.input.template_subset.is_some());
    assert!(
        job.input
            .files
            .iter()
            .any(|f| matches!(f.file.kind, DocumentKind::Template))
    );
    assert_eq!(h.snapshot()["project"], before["project"]);
    let professional: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/professional/module.json"
    ))
    .unwrap();
    let scope = draft(&mut h, professional["data"]["definition"].clone());
    let ApiScope::DefinitionDraft(draft_id) = &scope else {
        unreachable!()
    };
    let mut p = params(&mut h, "module_preview");
    p["revision"] = json!(1);
    p["object_id"] = json!({"kind":"definition_draft","id":draft_id});
    let handle: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", p));
    h.finish_one();
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    assert!(
        backend
            .latest()
            .input
            .issues
            .iter()
            .any(|i| i.code == "render.nonvisual_definition")
    );
    assert_eq!(h.snapshot()["project"], before["project"]);
}

#[test]
fn actual_definition_cycle_frozen_revision_and_opaque_scope_are_retained_after_current_head_edit() {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    let value = fixture();
    let definition = value["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "fixture.assembly-station")
        .unwrap()
        .clone();
    let scope = draft(&mut h, definition.clone());
    let ui = h.ui.clone();
    let input = h.object("scenario", "LOCAL");
    let handle: TaskHandle = ok(call(
        &mut h,
        &ui,
        &scope,
        "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":1,"observation_window":input["window"],"intent":{"kind":"definition_cycle","data":{"definition_revision":1,"test_input":input}}}),
    ));
    h.finish_one();
    let completed: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    let Some(TaskOutput::Run(run)) = completed.result else {
        panic!("actual cycle")
    };
    let full: RunView = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_run",
        json!({"run_id":run.run_id,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(before) = full else {
        unreachable!()
    };
    let mut changed = definition.clone();
    changed["title"] = json!("only current R2 title");
    let _: EditCommit = ok(call(
        &mut h,
        &ui,
        &scope,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_definition","data":changed}]}),
    ));
    let mut p = params(&mut h, "module_preview");
    p["revision"] = json!(2);
    p["view"]["project_revision"] = json!(1);
    p["view"]["run_id"] = json!(run.run_id);
    p["view"]["view_time_ms"] = json!(0);
    let ApiScope::DefinitionDraft(id) = &scope else {
        unreachable!()
    };
    p["object_id"] = json!({"kind":"definition_draft","id":id});
    let handle: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", p.clone()));
    h.finish_one();
    let frozen = backend.latest();
    assert_eq!(json!(frozen.frame.scope_revision), json!(2));
    assert_eq!(json!(frozen.input.definition.as_ref().unwrap()), definition);
    assert!(
        matches!(frozen.frame.source,RenderInputSource::Run{definition:Some(ref d),..}if json!(d.revision)==json!(1))
    );
    for kind in ["device_detail", "device_cycle_preview", "timeline_snapshot"] {
        let mut selected = p.clone();
        selected["view"]["kind"] = json!(kind);
        if kind == "timeline_snapshot" {
            selected["object_id"] = json!({"kind":"run","id":run.run_id});
        }
        let sample: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", selected));
        h.finish_one();
        let state: TaskState = ok(call(
            &mut h,
            &ui,
            &scope,
            "get_task",
            json!({"task_id":sample.id}),
        ));
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{kind}: {state:?}"
        );
        assert_eq!(
            json!(backend.latest().input.definition.as_ref().unwrap()),
            definition
        );
        assert!(backend.latest().input.port_output_summaries.is_empty());
        assert!(
            backend
                .latest()
                .input
                .issues
                .iter()
                .any(|i| i.code == "render.port_output_not_applicable")
        );
    }
    let mut current = p.clone();
    current["view"]["kind"] = json!("device_detail");
    current["view"]["run_id"] = Value::Null;
    current["view"]["view_time_ms"] = Value::Null;
    let current: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", current));
    h.finish_one();
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":current.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    assert_eq!(
        backend.latest().input.definition.as_ref().unwrap().title,
        "only current R2 title"
    );
    let full: RunView = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_run",
        json!({"run_id":run.run_id,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(after) = full else {
        unreachable!()
    };
    assert_eq!(json!(before), json!(after));
    let other = draft(&mut h, definition);
    p["revision"] = json!(1);
    let ApiScope::DefinitionDraft(other_id) = &other else {
        unreachable!()
    };
    p["object_id"] = json!({"kind":"definition_draft","id":other_id});
    reject(
        call(&mut h, &ui, &other, "render_view", p),
        ErrorCode::PermissionDenied,
    );
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
}

#[test]
fn unsupported_backend_kind_is_not_advertised_and_real_pending_task_blocks_project_departure() {
    let mut h = Harness::new();
    let mut backend = ControlBackend::new(Mode::Ok);
    Arc::get_mut(&mut backend).unwrap().kinds = vec![RenderViewKind::LineCanvas];
    h.host.set_render_backend(backend.clone());
    let mut p = params(&mut h, "multi_hall_canvas");
    let ui = h.ui.clone();
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p.take()),
        ErrorCode::UnsupportedCapability,
    );
    let p = params(&mut h, "line_canvas");
    let handle = submit(&mut h, p);
    let before = h.snapshot();
    let new = json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"new actual Project","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"discard_after_tasks_terminal"}}]});
    reject(
        h.call_as(&ui, false, "apply_commands", new.clone()),
        ErrorCode::InvalidInput,
    );
    assert_eq!(h.snapshot()["project"], before["project"]);
    h.finish_one();
    assert!(matches!(
        task(&mut h, &handle).task.state,
        ComputationStatus::Completed
    ));
    let captured = backend.latest();
    assert!(captured.show_authorized.load(Ordering::Acquire));
    let _: EditCommit = ok(h.call_as(&ui, false, "apply_commands", new));
    assert!(!captured.show_authorized.load(Ordering::Acquire));
    // Original Task remains scoped to P1; a new Project is not its image owner.
    let context: ProjectContext = ok(h.call_as(&ui, false, "get_context", json!({"selection":[]})));
    let project = context.project.unwrap().id;
    let request=serde_json::from_value(json!({"api_version":"0.1","request_id":"old-render-new-project","project_id":project,"scope":{"kind":"project"},"operation":"get_task","params":{"task_id":handle.id}})).unwrap();
    reject(h.host.dispatch(&ui, request), ErrorCode::PermissionDenied);
}

#[test]
fn professional_draft_render_freezes_exact_old_artifact_and_keeps_project_and_draft_revisions_distinct()
 {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    let ui = h.ui.clone();
    let professional: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/professional/module.json"
    ))
    .unwrap();
    let scope = draft(&mut h, professional["data"]["definition"].clone());
    let cases: Value = serde_json::from_str(include_str!(
        "fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "HOLD01")
        .unwrap();
    assert_eq!(case["expected"], "pass");
    let job = case["job"]["data"].clone();
    let _: EditCommit = ok(call(
        &mut h,
        &ui,
        &scope,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_packaging_job","data":job}]}),
    ));
    let revision = draft_revision(&mut h, &scope);
    let task: TaskHandle = ok(call(
        &mut h,
        &ui,
        &scope,
        "compute_packaging",
        json!({"packaging_job":job,"input_revision":revision,"computation_budget":job["budget"]}),
    ));
    h.finish_one();
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":task.id}),
    ));
    let Some(TaskOutput::Packaging(result)) = state.result else {
        panic!("actual packaging")
    };
    assert!(matches!(result.outcome, CheckOutcome::Pass));
    let revision = draft_revision(&mut h, &scope);
    assert_ne!(
        json!(revision),
        json!(h.context().project.unwrap().revision)
    );
    let project = h.context().project.unwrap().id;
    for file in &result.manufacturing_files {
        let bytes = h
            .host
            .read_stored_manufacturing_file(
                &ui,
                &project,
                &scope,
                &revision,
                &result.artifact_id,
                &file.file,
            )
            .unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("<svg"));
        assert!(matches!(
            h.host
                .read_stored_manufacturing_file(
                    &ui,
                    &project,
                    &ApiScope::Project,
                    &support::revision(1),
                    &result.artifact_id,
                    &file.file
                )
                .unwrap_err()
                .code,
            ErrorCode::MissingReference
        ));
    }
    let mut p = params(&mut h, "packaging_workbench");
    p["revision"] = json!(revision);
    p["object_id"] = json!({"kind":"artifact","id":result.artifact_id});
    let handle: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", p.clone()));
    let mut changed = job.clone();
    changed["revision"] = json!(2);
    let _: EditCommit = ok(call(
        &mut h,
        &ui,
        &scope,
        "apply_commands",
        json!({"base_revision":revision,"commands":[{"kind":"put_packaging_job","data":changed}]}),
    ));
    h.finish_one();
    let frozen = backend.latest();
    assert_eq!(json!(frozen.frame.scope_revision), json!(revision));
    assert_eq!(json!(frozen.frame.request.view.project_revision), json!(1));
    let typed_job: PackageStructureJob = serde_json::from_value(job).unwrap();
    assert_eq!(
        json!(frozen.input.packaging.as_ref().unwrap().job),
        json!(typed_job)
    );
    let state: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":handle.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    let mut manufacturing = p.clone();
    manufacturing["view"]["kind"] = json!("packaging_manufacturing");
    manufacturing["revision"] = json!(draft_revision(&mut h, &scope));
    let actual: TaskHandle = ok(call(&mut h, &ui, &scope, "render_view", manufacturing));
    h.finish_one();
    let actual: TaskState = ok(call(
        &mut h,
        &ui,
        &scope,
        "get_task",
        json!({"task_id":actual.id}),
    ));
    assert!(matches!(actual.task.state, ComputationStatus::Completed));
    assert_eq!(
        backend.latest().input.files.len(),
        result.manufacturing_files.len()
    );
    let mut other_definition = professional["data"]["definition"].clone();
    other_definition["id"] = json!("render.other.professional");
    let other = draft(&mut h, other_definition);
    p["revision"] = json!(1);
    reject(
        call(&mut h, &ui, &other, "render_view", p.clone()),
        ErrorCode::MissingReference,
    );
    p["revision"] = json!(1);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p),
        ErrorCode::MissingReference,
    );
    assert_eq!(h.context().project.unwrap().revision.0.value(), 1);
}

#[test]
fn actual_manufacturing_preview_reads_only_exact_scope_artifact_file_and_survives_save_move_without_writes()
 {
    let mut h = Harness::new();
    let result = packaging(&mut h);
    assert!(!result.manufacturing_files.is_empty());
    let first = &result.manufacturing_files[0].file;
    let context = h.context().project.unwrap();
    let project = context.id;
    let revision = context.revision;
    let before = h.snapshot();
    let bytes = h
        .host
        .read_stored_manufacturing_file(
            &h.ui,
            &project,
            &ApiScope::Project,
            &revision,
            &result.artifact_id,
            first,
        )
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("<svg"));
    let mut wrong = first.clone();
    wrong.path = "documents/not-listed.svg".to_owned().try_into().unwrap();
    assert!(matches!(
        h.host
            .read_stored_manufacturing_file(
                &h.ui,
                &project,
                &ApiScope::Project,
                &revision,
                &result.artifact_id,
                &wrong
            )
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    wrong = first.clone();
    wrong.kind = DocumentKind::Scenario;
    assert!(matches!(
        h.host
            .read_stored_manufacturing_file(
                &h.ui,
                &project,
                &ApiScope::Project,
                &revision,
                &result.artifact_id,
                &wrong
            )
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    assert!(matches!(
        h.host
            .read_stored_manufacturing_file(
                &h.ui,
                &project,
                &ApiScope::Project,
                &support::revision(1),
                &result.artifact_id,
                first
            )
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    ));
    let unknown: ArtifactId = "not-a-stored-artifact".to_owned().try_into().unwrap();
    assert!(matches!(
        h.host
            .read_stored_manufacturing_file(
                &h.ui,
                &project,
                &ApiScope::Project,
                &revision,
                &unknown,
                first
            )
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    let external = external(
        &mut h,
        vec![
            OperationName::GetObject,
            OperationName::GetRun,
            OperationName::RenderView,
        ],
        false,
    );
    assert!(matches!(
        h.host
            .read_stored_manufacturing_file(
                &external,
                &project,
                &ApiScope::Project,
                &revision,
                &unknown,
                first
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    let other: ProjectId = "other-project".to_owned().try_into().unwrap();
    assert!(matches!(
        h.host
            .read_stored_manufacturing_file(
                &h.ui,
                &other,
                &ApiScope::Project,
                &revision,
                &result.artifact_id,
                first
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
    // A new result for the same input Job is never a substitute for old bytes.
    let next = packaging(&mut h);
    assert_ne!(next.artifact_id, result.artifact_id);
    let current = h.context().project.unwrap().revision;
    assert!(matches!(
        h.host
            .read_stored_manufacturing_file(
                &h.ui,
                &project,
                &ApiScope::Project,
                &current,
                &result.artifact_id,
                &next.manufacturing_files[0].file
            )
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    assert_eq!(
        h.host
            .read_stored_manufacturing_file(
                &h.ui,
                &project,
                &ApiScope::Project,
                &current,
                &result.artifact_id,
                first
            )
            .unwrap(),
        bytes
    );
    let saved = h.tree.directory("manufacturing-preview-saved");
    let save = h.save(current.0.value(), &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("manufacturing-preview-moved");
    copy(&saved, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    let before = reopened.snapshot();
    assert_eq!(
        reopened
            .host
            .read_stored_manufacturing_file(
                &reopened.ui,
                &project,
                &ApiScope::Project,
                &current,
                &result.artifact_id,
                first
            )
            .unwrap(),
        bytes
    );
    assert_eq!(reopened.snapshot(), before);
}

#[test]
fn all_nine_kinds_reject_missing_or_incompatible_specific_sources_without_enqueuing() {
    let mut h = Harness::new();
    h.host.set_render_backend(ControlBackend::new(Mode::Ok));
    let ui = h.ui.clone();
    for (kind, field, value, code) in [
        (
            "line_canvas",
            "canvas",
            json!("multi_hall_canvas"),
            ErrorCode::InvalidInput,
        ),
        (
            "multi_hall_canvas",
            "hall_id",
            json!("H-A"),
            ErrorCode::InvalidInput,
        ),
        (
            "device_detail",
            "target",
            Value::Null,
            ErrorCode::InvalidInput,
        ),
        (
            "module_preview",
            "target",
            json!({"kind":"device","id":"ASM-01"}),
            ErrorCode::InvalidInput,
        ),
        (
            "packaging_workbench",
            "target",
            json!({"kind":"artifact","id":"no-artifact"}),
            ErrorCode::MissingReference,
        ),
        (
            "packaging_manufacturing",
            "preview",
            json!({"phase_id":null,"progress":0.5,"pose_name":null}),
            ErrorCode::InvalidInput,
        ),
        (
            "device_cycle_preview",
            "target",
            json!({"kind":"device","id":"ASM-01"}),
            ErrorCode::InvalidInput,
        ),
        (
            "timeline_snapshot",
            "target",
            json!({"kind":"scenario","id":"not-saved"}),
            ErrorCode::MissingReference,
        ),
        ("comparison", "target", Value::Null, ErrorCode::InvalidInput),
    ] {
        let mut p = params(&mut h, kind);
        match field {
            "target" => p["object_id"] = value,
            "preview" => p["preview"] = value,
            _ => p["view"][field] = value,
        }
        reject(
            call(&mut h, &ui, &ApiScope::Project, "render_view", p),
            code,
        );
        assert!(h.host.drain_io_work().is_empty());
    }
    let definition = fixture()["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "fixture.assembly-station")
        .unwrap()
        .clone();
    let scope = draft(&mut h, definition);
    for kind in ["line_canvas", "multi_hall_canvas", "comparison"] {
        let p = params(&mut h, kind);
        reject(
            call(&mut h, &ui, &scope, "render_view", p),
            ErrorCode::InvalidInput,
        );
    }
    // Real second hall makes an omitted hall ambiguous; never choose first.
    let mut hall = h.object("hall", "H-A");
    hall["id"] = json!("H-B");
    hall["title"] = json!("explicit second hall for render selector test");
    let _: EditCommit = ok(h.apply(1, json!([put("hall", hall)])));
    let p = params(&mut h, "line_canvas");
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", p),
        ErrorCode::InvalidInput,
    );
    assert!(h.host.drain_io_work().is_empty());
}

// Supplement only: public Host requests for actual sources absent from the
// original 46 captures. The original 18 tests/expectations above stay intact.
fn visual_supplement_directory(relative: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}
fn visual_supplement_finish(
    h: &mut Harness,
    backend: &ControlBackend,
    request: Value,
) -> RenderJob {
    let handle = submit(h, request);
    h.finish_one();
    let state = task(h, &handle);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    backend.latest()
}

#[test]
fn visual_supplement_actual_template_line_and_multi_project_targets_are_retained_and_wrong_hall_is_rejected()
 {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    let source = visual_supplement_directory("modules/templates/purchased-assembly/candidate");
    h.host.register_bundled_template_directory(&source).unwrap();
    let candidate =
        sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::PurchasedAssembly).unwrap();
    let actual: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
    let choices = sf_module_kit::TemplateChoices {
        namespace: "render-supplement".into(),
        resources: candidate
            .example_project
            .resources
            .iter()
            .map(|r| {
                (
                    r.id.clone(),
                    sf_module_kit::TemplateResourceSelection::CreateDeclaredExample,
                )
            })
            .collect(),
        reuse_items: std::collections::BTreeMap::new(),
        include_example_product: true,
    };
    let plan = sf_module_kit::prepare_template(&candidate, &actual, &choices).unwrap();
    let _: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"instantiate_template","data":plan.command_data(None)}]),
    ));
    let before = h.snapshot();
    let actual: ProjectState = serde_json::from_value(before["project"].clone()).unwrap();
    assert_eq!(actual.lines.len(), 1);
    let line = &actual.lines[0];
    assert!(!line.members.is_empty());
    let mut request = params(&mut h, "line_canvas");
    request["object_id"] = json!({"kind":"line","id":line.id});
    request["view"]["hall_id"] = json!(line.hall_id);
    let frozen = visual_supplement_finish(&mut h, &backend, request.clone());
    assert_eq!(
        json!(frozen.frame.request.view.object_ref),
        json!({"kind":"line","id":line.id})
    );
    assert_eq!(
        json!(frozen.frame.request.object_id),
        json!({"kind":"line","id":line.id})
    );
    assert_eq!(
        json!(frozen.frame.request.view.hall_id),
        json!(line.hall_id)
    );
    assert_eq!(json!(frozen.input.project), json!(actual));
    let ui = h.ui.clone();
    let mut wrong = request.clone();
    wrong["view"]["hall_id"] = json!("H-A");
    assert_ne!(line.hall_id.as_str(), "H-A");
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", wrong),
        ErrorCode::InvalidInput,
    );
    let mut wrong = request;
    wrong["object_id"] = json!({"kind":"line","id":"not-an-existing-line"});
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", wrong),
        ErrorCode::MissingReference,
    );
    let mut request = params(&mut h, "multi_hall_canvas");
    request["object_id"] = json!({"kind":"project","id":actual.id});
    let frozen = visual_supplement_finish(&mut h, &backend, request);
    assert_eq!(
        json!(frozen.frame.request.view.object_ref),
        json!({"kind":"project","id":actual.id})
    );
    assert!(frozen.frame.request.view.hall_id.is_none());
    assert_eq!(json!(frozen.input.project), json!(actual));
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(h.snapshot()["project"], before["project"]);
    assert_eq!(h.snapshot()["scenarios"], before["scenarios"]);
}

#[test]
fn visual_supplement_actual_create_project_empty_canvas_keeps_no_hall_or_device_and_reports_empty()
{
    let ui = id("render-empty-ui");
    let mut host = ProjectHost::new(id("render-empty-host"), ui.clone());
    let backend = ControlBackend::new(Mode::Ok);
    host.set_render_backend(backend.clone());
    let _:EditCommit=ok(host.dispatch(&ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"empty-create","project_id":null,"scope":{"kind":"workspace"},"operation":"apply_commands","params":{"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"Explicit empty rendering fixture","time_reference":{"label":"explicit"},"departure":"stay"}}]}})).unwrap()));
    let context:ProjectContext=ok(host.dispatch(&ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"empty-context","project_id":null,"scope":{"kind":"workspace"},"operation":"get_context","params":{"selection":[]}})).unwrap()));
    let project = context.project.unwrap();
    let read_project = |host: &mut ProjectHost| -> ProjectState {
        let object:DomainObject=ok(host.dispatch(&ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"empty-project","project_id":project.id,"scope":{"kind":"project"},"operation":"get_object","params":{"object_id":{"kind":"project","id":project.id},"source":{"kind":"draft"},"node_index":null}})).unwrap()));
        let DomainObject::Project(actual) = object else {
            unreachable!()
        };
        *actual
    };
    let before = read_project(&mut host);
    assert!(before.halls.is_empty() && before.devices.is_empty() && before.lines.is_empty());
    let handle:TaskHandle=ok(host.dispatch(&ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"empty-render","project_id":project.id,"scope":{"kind":"project"},"operation":"render_view","params":{"revision":project.revision,"view":{"kind":"line_canvas","canvas":"line_canvas","hall_id":null,"object_ref":null,"project_revision":project.revision,"run_id":null,"view_time_ms":null,"lod":"detailed","camera":before.views.line_camera},"object_id":null,"preview":null,"delivery":"capture_only"}})).unwrap()));
    let mut work = host.drain_io_work();
    assert_eq!(work.len(), 1);
    host.finish_io(work.pop().unwrap().run());
    let state:TaskState=ok(host.dispatch(&ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"empty-task","project_id":project.id,"scope":{"kind":"project"},"operation":"get_task","params":{"task_id":handle.id}})).unwrap()));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    let frozen = backend.latest();
    assert!(frozen.frame.request.view.hall_id.is_none());
    assert!(
        frozen.input.project.halls.is_empty()
            && frozen.input.project.devices.is_empty()
            && frozen.input.modules.is_empty()
    );
    assert!(
        frozen
            .input
            .issues
            .iter()
            .any(|i| i.code == "render.empty_project")
    );
    assert_eq!(json!(read_project(&mut host)), json!(before));
}

#[test]
fn visual_supplement_original_named_pose_is_a_declaration_without_timeline_job_or_run() {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    h.host
        .register_bundled_module_directory(&visual_supplement_directory(
            "modules/devices/template-support/devices",
        ))
        .unwrap();
    let before = h.snapshot();
    let mut request = params(&mut h, "module_preview");
    request["object_id"] = json!({"kind":"module","id":"p401.support.devices"});
    request["module_ref"] = json!({"id":"p401.support.devices","version":"1.0.0"});
    request["preview"] = json!({"phase_id":null,"progress":null,"pose_name":"pressed"});
    let frozen = visual_supplement_finish(&mut h, &backend, request.clone());
    let package = frozen
        .input
        .modules
        .iter()
        .find(|p| p.definition.id.as_str() == "p401.support.devices")
        .unwrap();
    let ModulePayload::Devices(device) = &package.definition.payload else {
        unreachable!()
    };
    let presentation = device.presentation.as_ref().unwrap();
    assert!(
        presentation
            .poses
            .iter()
            .any(|pose| pose.id.as_str() == "pressed")
    );
    assert_eq!(
        frozen
            .frame
            .request
            .preview
            .as_ref()
            .unwrap()
            .pose_name
            .as_deref(),
        Some("pressed")
    );
    assert!(
        frozen.input.run.is_none()
            && frozen.input.active_job_id.is_none()
            && frozen.input.objects_at.is_empty()
    );
    let ui = h.ui.clone();
    request["preview"]["pose_name"] = json!("not-a-declared-pose");
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", request),
        ErrorCode::InvalidInput,
    );
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(h.snapshot()["project"], before["project"]);
}

fn visual_supplement_register_image(h: &mut Harness, kind: &str) {
    h.host
        .register_bundled_module_directory(&visual_supplement_directory(
            "modules/media/p302-controlled/installable/image-simple",
        ))
        .unwrap();
    h.host
        .register_bundled_module_directory(&visual_supplement_directory(&format!(
            "modules/media/p302-controlled/installable/{kind}"
        )))
        .unwrap();
}

#[test]
fn visual_supplement_actual_image_exact_asset_bytes_do_not_become_glb_or_wrong_version_replacement()
{
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    visual_supplement_register_image(&mut h, "image");
    let before = h.snapshot();
    let mut request = params(&mut h, "module_preview");
    request["object_id"] = json!({"kind":"module","id":"p302.controlled.image"});
    request["module_ref"] = json!({"id":"p302.controlled.image","version":"0.1.0"});
    let frozen = visual_supplement_finish(&mut h, &backend, request.clone());
    let package = frozen
        .input
        .modules
        .iter()
        .find(|p| p.definition.id.as_str() == "p302.controlled.image")
        .unwrap();
    let ModulePayload::Media(presentation) = &package.definition.payload else {
        unreachable!()
    };
    assert!(matches!(presentation.kind, PresentationKind::Image));
    let MediaLocator::Asset(asset) = &presentation.source else {
        unreachable!()
    };
    assert!(matches!(asset.kind, AssetKind::Png));
    let source = ObjectSource::ModuleVersion {
        module_ref: frozen.frame.request.module_ref.clone().unwrap(),
    };
    let bytes = frozen.input.read_asset(&source, asset).unwrap();
    assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(
        bytes,
        std::fs::read(visual_supplement_directory(
            "modules/media/p302-controlled/installable/image/station.png"
        ))
        .unwrap()
    );
    let wrong_source:ObjectSource=serde_json::from_value(json!({"kind":"module_version","data":{"module_ref":{"id":"p302.controlled.image","version":"99.0.0"}}})).unwrap();
    assert!(frozen.input.read_asset(&wrong_source, asset).is_err());
    let mut missing = asset.clone();
    missing.path = "not-listed.png".to_owned().try_into().unwrap();
    assert!(frozen.input.read_asset(&source, &missing).is_err());
    let ui = h.ui.clone();
    request["module_ref"]["version"] = json!("99.0.0");
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", request),
        ErrorCode::MissingReference,
    );
    assert!(h.host.drain_io_work().is_empty());
    let broken = h.tree.directory("missing-original-image");
    copy(
        &visual_supplement_directory("modules/media/p302-controlled/installable/image"),
        &broken,
    );
    std::fs::remove_file(broken.join("station.png")).unwrap();
    assert!(
        h.host.register_bundled_module_directory(&broken).is_err(),
        "Missing original byte source must fail actual registration, not supply a replacement"
    );
    assert_eq!(h.snapshot()["project"], before["project"]);
}

#[test]
fn visual_supplement_actual_gif_bytes_and_declared_progress_are_frozen_without_playback_or_engine()
{
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    visual_supplement_register_image(&mut h, "gif");
    let before = h.snapshot();
    for progress in [0.25, 0.75] {
        let mut request = params(&mut h, "module_preview");
        request["object_id"] = json!({"kind":"module","id":"p302.controlled.gif"});
        request["module_ref"] = json!({"id":"p302.controlled.gif","version":"0.1.0"});
        request["preview"] = json!({"phase_id":"cycle","progress":progress,"pose_name":null});
        let frozen = visual_supplement_finish(&mut h, &backend, request);
        let package = frozen
            .input
            .modules
            .iter()
            .find(|p| p.definition.id.as_str() == "p302.controlled.gif")
            .unwrap();
        let ModulePayload::Media(presentation) = &package.definition.payload else {
            unreachable!()
        };
        assert!(matches!(presentation.kind, PresentationKind::Gif));
        let MediaLocator::Asset(asset) = &presentation.source else {
            unreachable!()
        };
        assert!(matches!(asset.kind, AssetKind::Gif));
        let source = ObjectSource::ModuleVersion {
            module_ref: frozen.frame.request.module_ref.clone().unwrap(),
        };
        let bytes = frozen.input.read_asset(&source, asset).unwrap();
        assert!(bytes.starts_with(b"GIF"));
        assert_eq!(
            bytes,
            std::fs::read(visual_supplement_directory(
                "modules/media/p302-controlled/installable/gif/station.gif"
            ))
            .unwrap()
        );
        assert_eq!(
            frozen
                .frame
                .request
                .preview
                .as_ref()
                .unwrap()
                .progress
                .as_ref()
                .unwrap()
                .value(),
            progress
        );
        assert!(
            frozen.input.run.is_none()
                && frozen.input.active_job_id.is_none()
                && frozen.input.objects_at.is_empty()
        );
        assert!(
            frozen.frame.request.view.run_id.is_none()
                && frozen.frame.request.view.view_time_ms.is_none()
        );
    }
    let ui = h.ui.clone();
    let mut bad = params(&mut h, "module_preview");
    bad["object_id"] = json!({"kind":"module","id":"p302.controlled.gif"});
    bad["module_ref"] = json!({"id":"p302.controlled.gif","version":"0.1.0"});
    bad["preview"] = json!({"phase_id":"cycle","progress":1.1,"pose_name":null});
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", bad),
        ErrorCode::InvalidInput,
    );
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(h.snapshot()["project"], before["project"]);
    assert_eq!(h.snapshot()["scenarios"], before["scenarios"]);
}

#[test]
fn visual_supplement_camera_close_and_far_actual_glb_requests_are_admitted_and_frozen_without_ui_clamps()
 {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    for name in ["model-simple", "model"] {
        h.host
            .register_bundled_module_directory(&visual_supplement_directory(&format!(
                "modules/media/p302-controlled/installable/{name}"
            )))
            .unwrap();
    }
    let before = h.snapshot();
    for distance in [0.04, 200.0] {
        let mut request = params(&mut h, "module_preview");
        request["object_id"] = json!({"kind":"module","id":"p302.controlled.model"});
        request["module_ref"] = json!({"id":"p302.controlled.model","version":"0.1.0"});
        request["view"]["camera"]["eye"] = json!({"x":0,"y":0,"z":distance});
        request["view"]["camera"]["target"] = json!({"x":0,"y":0,"z":0});
        request["view"]["camera"]["zoom"] = json!(1);
        let camera: CameraState =
            serde_json::from_value(request["view"]["camera"].clone()).unwrap();
        let frozen = visual_supplement_finish(&mut h, &backend, request);
        assert_eq!(json!(frozen.frame.request.view.camera), json!(camera));
        assert_eq!(frozen.frame.request.view.camera.eye.z.value(), distance);
        assert!(frozen.frame.request.view.run_id.is_none() && frozen.input.run.is_none());
        let package = frozen
            .input
            .modules
            .iter()
            .find(|p| p.definition.id.as_str() == "p302.controlled.model")
            .unwrap();
        let ModulePayload::Media(presentation) = &package.definition.payload else {
            unreachable!()
        };
        let MediaLocator::Asset(asset) = &presentation.source else {
            unreachable!()
        };
        assert!(matches!(asset.kind, AssetKind::Glb));
        let source = ObjectSource::ModuleVersion {
            module_ref: frozen.frame.request.module_ref.clone().unwrap(),
        };
        let bytes = frozen.input.read_asset(&source, asset).unwrap();
        assert!(bytes.starts_with(b"glTF"));
        assert_eq!(
            bytes,
            std::fs::read(visual_supplement_directory(
                "modules/media/p302-controlled/installable/model/station.glb"
            ))
            .unwrap()
        );
    }
    assert_eq!(h.snapshot()["project"], before["project"]);
    assert_eq!(h.snapshot()["scenarios"], before["scenarios"]);
}

#[test]
fn line_target_facing_camera_supplement_is_a_new_public_request_without_automatic_fit() {
    let mut h = Harness::new();
    let backend = ControlBackend::new(Mode::Ok);
    h.host.set_render_backend(backend.clone());
    h.host
        .register_bundled_template_directory(&visual_supplement_directory(
            "modules/templates/purchased-assembly/candidate",
        ))
        .unwrap();
    let candidate =
        sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::PurchasedAssembly).unwrap();
    let project: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
    let choices = sf_module_kit::TemplateChoices {
        namespace: "render-supplement".into(),
        resources: candidate
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
        reuse_items: std::collections::BTreeMap::new(),
        include_example_product: true,
    };
    let plan = sf_module_kit::prepare_template(&candidate, &project, &choices).unwrap();
    let _: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"instantiate_template","data":plan.command_data(None)}]),
    ));
    let before = h.snapshot();
    let project: ProjectState = serde_json::from_value(before["project"].clone()).unwrap();
    assert_eq!(project.lines.len(), 1);
    let line = &project.lines[0];
    let mut request = params(&mut h, "line_canvas");
    request["object_id"] = json!({"kind":"line","id":line.id});
    request["view"]["hall_id"] = json!(line.hall_id);
    request["view"]["camera"] = json!({
        "coordinate_frame":"right_handed_z_up","unit":"meter",
        "eye":{"x":23,"y":17,"z":12},
        "target":{"x":15,"y":8,"z":1},"zoom":1
    });
    let requested_camera = request["view"]["camera"].clone();
    let frozen = visual_supplement_finish(&mut h, &backend, request);
    let actual_camera: CameraState = serde_json::from_value(requested_camera).unwrap();
    assert_eq!(
        json!(frozen.frame.request.view.camera),
        json!(actual_camera)
    );
    assert_eq!(
        json!(frozen.frame.request.object_id),
        json!({"kind":"line","id":line.id})
    );
    assert_eq!(
        json!(frozen.frame.request.view.object_ref),
        json!({"kind":"line","id":line.id})
    );
    assert_eq!(
        json!(frozen.frame.request.view.hall_id),
        json!(line.hall_id)
    );
    assert_eq!(json!(frozen.input.project), json!(project));
    assert_eq!(h.snapshot()["project"], before["project"]);
    assert_eq!(h.snapshot()["scenarios"], before["scenarios"]);
}

// F22: the exact original Unsupported error travels only with its physical PortRef.
// This backend retains actual Host inputs under this package's separate output.
struct F22Backend {
    jobs: Mutex<Vec<RenderJob>>,
}
impl RenderBackend for F22Backend {
    fn supported_kinds(&self) -> Vec<RenderViewKind> {
        kinds()
    }
    fn render(&self, job: RenderJob, _: &AtomicBool) -> Result<RenderedCapture, ApiError> {
        let mut jobs = self.jobs.lock().unwrap();
        let directory = visual_supplement_directory(&format!(
            "scenarios/Agent共建/runtime-public-render-port-unavailable/producer/actual-inputs/session-{}/capture-{}",
            std::process::id(),
            jobs.len() + 1
        ));
        if jobs.is_empty() {
            println!(
                "F22_ACTUAL_INPUTS={}",
                directory.parent().unwrap().display()
            );
        }
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("frame.json"),
            serde_json::to_vec_pretty(&job.frame).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join("input.json"),
            serde_json::to_vec_pretty(job.input.as_ref()).unwrap(),
        )
        .unwrap();
        let mut assets = vec![];
        for (i, entry) in job.input.assets.iter().enumerate() {
            let bytes = job.input.read_asset(&entry.source, &entry.asset).unwrap();
            let path = format!("asset-{i}.bin");
            std::fs::write(directory.join(&path), &bytes).unwrap();
            assets.push(json!({"source":entry.source,"asset":entry.asset,"bytes_file":path,"length":bytes.len()}));
        }
        let mut files = vec![];
        for (i, entry) in job.input.files.iter().enumerate() {
            let bytes = job.input.read_file(&entry.file).unwrap();
            let path = format!("file-{i}.bin");
            std::fs::write(directory.join(&path), &bytes).unwrap();
            files.push(json!({"file":entry.file,"bytes_file":path,"length":bytes.len()}));
        }
        std::fs::write(directory.join("bytes.json"),serde_json::to_vec_pretty(&json!({"test":std::thread::current().name(),"assets":assets,"files":files,"evidence":"Actual public Host/FIFO/backend input and exact byte closure; controlled protocol PNG, not Native pixels"})).unwrap()).unwrap();
        jobs.push(job.clone());
        Ok(RenderedCapture {
            frame: job.frame,
            scene_id: id("f22-controlled-scene"),
            image_id: "f22-controlled-image".to_owned().try_into().unwrap(),
            mime_type: "image/png".into(),
            bytes: include_bytes!(
                "../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png"
            )
            .to_vec(),
            visible_objects: vec![],
            issues: vec![],
        })
    }
}

fn f22_mixed_port_host() -> Harness {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/production/p1-phase-calendar.json")).unwrap();
    value["scenario"]["window"]["end_ms"] = json!(160000);
    for calendar in value["project"]["calendars"].as_array_mut().unwrap() {
        calendar["intervals"] = json!([{"start_ms":0,"end_ms":160000}]);
    }
    let mut typ = value["project"]["item_types"][1].clone();
    typ["id"] = json!("alternate-part");
    value["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .push(typ);
    value["project"]["locations"][0]["capacity"]["value"]["coefficient"] = json!(10);
    value["project"]["locations"][1]["capacity"]["value"]["coefficient"] = json!(10);
    value["project"]["locations"][1]["accepted_item_types"]
        .as_array_mut()
        .unwrap()
        .push(json!("alternate-part"));
    let mut alternate = value["project"]["routes"][0].clone();
    alternate["id"] = json!("alternate-route");
    alternate["steps"][0]["id"] = json!("alternate-step");
    alternate["steps"][0]["item_bindings"]["product"] = json!("alternate-part");
    value["project"]["routes"]
        .as_array_mut()
        .unwrap()
        .push(alternate);
    let mut alternate = value["scenario"]["production_tasks"][0].clone();
    alternate["id"] = json!("TASK-ALTERNATE");
    alternate["routing_ref"]["id"] = json!("alternate-route");
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .push(alternate);
    let mut alternate = value["project"]["connections"][1].clone();
    alternate["data"]["id"] = json!("alternate-output");
    alternate["data"]["item_type"] = json!("alternate-part");
    value["project"]["connections"]
        .as_array_mut()
        .unwrap()
        .push(alternate);
    let mut normal = value["project"]["devices"][0].clone();
    normal["id"] = json!("CNC-NORMAL");
    normal["title"] = json!("Declared independent normal output");
    normal["pose"]["translation"]["x"] = json!(3);
    normal["role_bindings"]["resources"]["execution_capacity"] = json!("CAP-NORMAL");
    normal["role_bindings"]["resources"]["machine"] = json!("CAP-NORMAL");
    value["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .push(normal);
    let mut capacity = value["project"]["resources"][0].clone();
    capacity["id"] = json!("CAP-NORMAL");
    capacity["title"] = json!("CAP-NORMAL");
    value["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(capacity);
    value["project"]["resources"][1]["capacity"]["value"]["coefficient"] = json!(2);
    let mut route = value["project"]["routes"][0].clone();
    route["id"] = json!("normal-route");
    route["steps"][0]["id"] = json!("normal-step");
    route["steps"][0]["device_id"] = json!("CNC-NORMAL");
    value["project"]["routes"]
        .as_array_mut()
        .unwrap()
        .push(route);
    let mut task = value["scenario"]["production_tasks"][0].clone();
    task["id"] = json!("TASK-NORMAL");
    task["routing_ref"]["id"] = json!("normal-route");
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .push(task);
    let mut task = value["scenario"]["production_tasks"][0].clone();
    task["id"] = json!("TASK-CURRENT");
    task["release_at_ms"] = json!(80000);
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .push(task);
    value["scenario"]["scope"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"device","id":"CNC-NORMAL"}));
    for name in ["raw-alternate", "raw-normal", "raw-current"] {
        let mut raw = value["scenario"]["initial_conditions"]["items"][0].clone();
        raw["id"] = json!(name);
        value["scenario"]["initial_conditions"]["items"]
            .as_array_mut()
            .unwrap()
            .push(raw);
    }
    for index in [0, 1] {
        let mut connection = value["project"]["connections"][index].clone();
        connection["data"]["id"] = json!(format!("normal-connection-{index}"));
        let key = if index == 0 { "destination" } else { "source" };
        connection["data"][key]["data"]["instance_id"] = json!("CNC-NORMAL");
        value["project"]["connections"]
            .as_array_mut()
            .unwrap()
            .push(connection);
    }
    let mut h = Harness::from_value(value);
    for name in ["model-simple", "model"] {
        h.host
            .register_bundled_module_directory(&visual_supplement_directory(&format!(
                "modules/media/p302-controlled/installable/{name}"
            )))
            .unwrap();
    }
    let _:EditCommit=ok(h.apply(1,json!([{"kind":"adopt_module","data":{"module_ref":{"id":"p302.controlled.model","version":"0.1.0"},"targets":[{"kind":"device","id":"CNC"},{"kind":"device","id":"CNC-NORMAL"}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
    h
}

#[test]
fn f22_exact_unsupported_port_error_preserves_real_media_active_job_and_other_port_counts() {
    let mut h = f22_mixed_port_host();
    let handle = h.run("PHASE");
    h.finish_one();
    let run = h.completed_run(&handle);
    let project = h.context().project.unwrap();
    let reference:PortRef=serde_json::from_value(json!({"module_ref":{"id":"w5.station.CNC","version":"0.1.0"},"instance_id":"CNC","port_id":"output"})).unwrap();
    let normal: PortRef = serde_json::from_value(
        json!({"module_ref":reference.module_ref,"instance_id":"CNC-NORMAL","port_id":"output"}),
    )
    .unwrap();
    let at = SimTimeMs(100000u64.try_into().unwrap());
    let incompatible = h
        .host
        .read_run_port_output(
            &h.ui,
            &project.id,
            &ApiScope::Project,
            &project.revision,
            &run.id,
            &at,
            &reference,
        )
        .unwrap_err();
    assert!(matches!(
        incompatible.code,
        ErrorCode::UnsupportedCapability
    ));
    assert!(
        incompatible
            .message
            .contains("incompatible actual item types")
    );
    let normal_output = h
        .host
        .read_run_port_output(
            &h.ui,
            &project.id,
            &ApiScope::Project,
            &project.revision,
            &run.id,
            &at,
            &normal,
        )
        .unwrap();
    assert_eq!(normal_output.completed_output.value.coefficient.value(), 1);
    assert_eq!(normal_output.accepted_output.value.coefficient.value(), 1);
    let before = h.snapshot();
    let original = json!(h.full(&run.id));
    let backend = Arc::new(F22Backend {
        jobs: Mutex::new(vec![]),
    });
    h.host.set_render_backend(backend.clone());
    for (kind, time) in [
        ("line_canvas", 45000),
        ("line_canvas", 100000),
        ("multi_hall_canvas", 100000),
        ("device_detail", 100000),
        ("device_cycle_preview", 100000),
    ] {
        let mut request = params(&mut h, kind);
        request["view"]["project_revision"] = json!(run.input_ref.project_revision);
        request["view"]["run_id"] = json!(run.id);
        request["view"]["view_time_ms"] = json!(time);
        if kind == "line_canvas" {
            request["view"]["hall_id"] = json!("H-1");
        }
        if kind == "device_detail" || kind == "device_cycle_preview" {
            request["object_id"] = json!({"kind":"device","id":"CNC"});
        }
        let handle = submit(&mut h, request);
        h.finish_one();
        assert!(matches!(
            task(&mut h, &handle).task.state,
            ComputationStatus::Completed
        ));
        let frozen = backend.jobs.lock().unwrap().last().unwrap().clone();
        let actual = json!(frozen.input.as_ref());
        assert_eq!(json!(frozen.input.run), json!(Some(&run)));
        assert_eq!(
            frozen
                .frame
                .request
                .view
                .view_time_ms
                .as_ref()
                .unwrap()
                .0
                .value(),
            time
        );
        if time == 45000 {
            assert!(
                actual.get("port_output_failures").is_none(),
                "Empty failure Vec must retain original private JSON shape"
            );
            assert_eq!(frozen.input.port_output_summaries.len(), 2);
            continue;
        }
        assert_eq!(
            actual.get("port_output_failures"),
            Some(&json!([[reference, incompatible]])),
            "F22 requires complete original Unsupported with exact physical PortRef, never only an unlocated issue"
        );
        assert_eq!(
            json!(frozen.input.port_output_summaries),
            json!([normal_output])
        );
        assert!(
            frozen
                .input
                .issues
                .iter()
                .any(|issue| issue.code == "render.unavailable_port_output")
        );
        let device = frozen
            .input
            .project
            .devices
            .iter()
            .find(|d| d.id.as_str() == "CNC")
            .unwrap();
        assert!(device.presentation_override.is_some());
        let glb = frozen
            .input
            .assets
            .iter()
            .find(|entry| matches!(entry.asset.kind, AssetKind::Glb))
            .unwrap();
        assert!(
            frozen
                .input
                .read_asset(&glb.source, &glb.asset)
                .unwrap()
                .starts_with(b"glTF")
        );
        let active:Vec<_>=frozen.input.objects_at.iter().filter_map(|object|match object {DomainObject::Job(job) if matches!(&job.execution_ref,JobExecutionRef::ProjectDevice{device_id,..} if device_id.as_str()=="CNC")&&job.started_at_ms.as_ref().is_some_and(|v|v.0.value()<=time)&&job.finished_at_ms.as_ref().is_none_or(|v|v.0.value()>time)=>Some(job),_=>None}).collect();
        assert_eq!(
            active.len(),
            1,
            "A real unique current Job is required for detail/cycle, never fabricated"
        );
        if kind == "device_cycle_preview" {
            assert_eq!(
                json!(frozen.input.active_job_id),
                json!(Some(&active[0].id))
            );
        }
    }
    let mut request = params(&mut h, "line_canvas");
    request["view"]["hall_id"] = json!("H-1");
    request["view"]["project_revision"] = json!(run.input_ref.project_revision);
    request["view"]["run_id"] = json!(run.id);
    request["view"]["view_time_ms"] = json!(100000);
    let ui = h.ui.clone();
    let mut wrong = request.clone();
    wrong["revision"] = json!(999);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", wrong),
        ErrorCode::RevisionConflict,
    );
    let mut wrong = request.clone();
    wrong["view"]["run_id"] = json!("not-a-run");
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", wrong),
        ErrorCode::MissingReference,
    );
    let mut wrong = request.clone();
    wrong["view"]["view_time_ms"] = json!(160001);
    reject(
        call(&mut h, &ui, &ApiScope::Project, "render_view", wrong),
        ErrorCode::InvalidInput,
    );
    reject(
        call(
            &mut h,
            &id("unknown-f22-session"),
            &ApiScope::Project,
            "render_view",
            request,
        ),
        ErrorCode::PermissionDenied,
    );
    let mut wrong = reference.clone();
    wrong.module_ref.version = "99.0.0".to_owned().try_into().unwrap();
    assert!(matches!(
        h.host
            .read_run_port_output(
                &h.ui,
                &project.id,
                &ApiScope::Project,
                &project.revision,
                &run.id,
                &at,
                &wrong
            )
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    let mut wrong = reference;
    wrong.port_id = "unknown-output".to_owned().try_into().unwrap();
    assert!(matches!(
        h.host
            .read_run_port_output(
                &h.ui,
                &project.id,
                &ApiScope::Project,
                &project.revision,
                &run.id,
                &at,
                &wrong
            )
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    ));
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(h.snapshot()["project"], before["project"]);
    assert_eq!(h.snapshot()["scenarios"], before["scenarios"]);
    assert_eq!(json!(h.full(&run.id)), original);
}
