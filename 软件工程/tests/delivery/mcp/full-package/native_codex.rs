//! Isolated acceptance app using the actual production Native modules and one
//! managed ProjectApiState. The external client is the installed Codex CLI.
#![allow(dead_code)]
#[path = "../../../../apps/planner/src-tauri/src/agent_pipe.rs"]
mod agent_pipe;
#[path = "../../../../apps/planner/src-tauri/src/project_api.rs"]
mod project_api;
#[path = "../../../../apps/planner/src-tauri/src/native_render.rs"]
mod native_render;
#[path = "../../../../apps/planner/src-tauri/src/native_render_capture.rs"]
mod native_render_capture;
use serde::Deserialize;
use serde_json::{Value, json};
use sf_domain::*;
use std::{fs, io, io::Read, os::windows::process::CommandExt, path::PathBuf,
    process::{Command, Stdio}, sync::Arc, time::{Duration, Instant}};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Client { Codex, Protocol, CreationProbe, RenderProof }
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    root: PathBuf,
    seed: PathBuf,
    bridge: PathBuf,
    python: PathBuf,
    prompt: PathBuf,
    output: PathBuf,
    authority: Value,
    render_kinds: Vec<RenderViewKind>,
    deadline_seconds: u64,
    client: Client,
    render_plan: Option<PathBuf>,
}
fn request(id: &str, project: Value, operation: &str, params: Value) -> Value {
    json!({"api_version":"0.1","request_id":id,"scope":{"kind":if project.is_null(){"workspace"}else{"project"}},"project_id":project,"operation":operation,"params":params})
}
fn ok(value: Value) -> io::Result<Value> {
    if value["outcome"]["status"] != "ok" { return Err(io::Error::other(format!("Actual Native response: {value}"))); }
    Ok(value["outcome"]["data"].clone())
}
fn import_actual(state: &project_api::ProjectApiState, case: &Case) -> io::Result<Value> {
    let ui = ObjectId::try_from("desktop-main-ui".to_owned()).map_err(io::Error::other)?;
    let token = state.select_external_directory(&ui, sf_commands::FilePurpose::ImportPackage, case.seed.clone())
        .map_err(|error| io::Error::other(error.message))?;
    let task = ok(state.dispatch_ui("main", request("P304-import", Value::Null, "import_package", json!({"selected_file_ref":token,"import_policy":"open_project","base_revision":null}))))?;
    let start = Instant::now();
    let draft = loop {
        let status = ok(state.dispatch_ui("main", request("P304-import-state", Value::Null, "get_task", json!({"task_id":task["id"]}))))?;
        match status["task"]["state"].as_str() {
            Some("completed") => break status["result"]["data"].clone(),
            Some("failed" | "cancelled") => return Err(io::Error::other(format!("Import failed: {status}"))),
            _ => {},
        }
        if start.elapsed() > Duration::from_secs(30) { return Err(io::Error::other("Actual import did not reach terminal within acceptance budget")); }
        std::thread::sleep(Duration::from_millis(10));
    };
    ok(state.dispatch_ui("main", request("P304-open", Value::Null, "apply_commands", json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{
        "chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}))))?;
    let context = ok(state.dispatch_ui("main", request("P304-open-context", Value::Null, "get_context", json!({"selection":[]}))))?;
    Ok(context["project"]["id"].clone())
}
fn execute(app: tauri::AppHandle, case: Case, project: Value) -> io::Result<bool> {
    let state = app.state::<agent_pipe::AgentPipeState>().inner().clone();
    let info = state.info()?;
    fs::write(case.output.join("actual-endpoint.json"), serde_json::to_vec_pretty(&info)?)?;
    let initial_context = state.project.dispatch_ui("main", request("P304-initial-context", Value::Null, "get_context", json!({"selection":[]})));
    fs::write(case.output.join("actual-initial-context.json"), serde_json::to_vec_pretty(&initial_context)?)?;
    let initial_project = state.project.dispatch_ui("main", request("P304-initial-project", project.clone(), "get_object", json!({"object_id":{"kind":"project","id":project},"source":{"kind":"draft"},"node_index":null})));
    fs::write(case.output.join("actual-initial-project.json"), serde_json::to_vec_pretty(&initial_project)?)?;
    let mut command = Command::new(&case.python);
    command.args(["-X","utf8"]);
    match case.client {
        Client::Codex => { command.arg(case.root.join("integrations/agent/clients/codex-session.py"))
            .arg("--bridge").arg(&case.bridge).arg("--host-pipe").arg(info["endpoint"].as_str().unwrap())
            .arg("--host-instance-id").arg(info["host_instance_id"].as_str().unwrap())
            .arg("--prompt-file").arg(&case.prompt).arg("--output").arg(case.output.join("codex")); },
        Client::Protocol | Client::CreationProbe => {
            let script = if matches!(case.client, Client::CreationProbe) {"creation_probe.py"} else {"concurrent_client.py"};
            command.arg(case.root.join("tests/delivery/mcp/full-package").join(script))
            .arg(&case.bridge).arg(info["endpoint"].as_str().unwrap())
            .arg(info["host_instance_id"].as_str().unwrap()).arg(project.as_str().unwrap()).arg(&case.output); },
        Client::RenderProof => {
            let plan = case.render_plan.as_ref().ok_or_else(|| io::Error::other("Explicit render plan required"))?;
            fs::copy(plan, case.output.join("plan.json"))?;
            command.arg(case.root.join("scenarios/Agent共建/runtime-public-native-render/process_render_client.py"))
                .arg(&case.bridge).arg(info["endpoint"].as_str().unwrap())
                .arg(info["host_instance_id"].as_str().unwrap()).arg(project.as_str().unwrap()).arg(&case.output);
        },
    }
    let mut child = command.current_dir(&case.root).stdout(Stdio::piped()).stderr(Stdio::piped()).creation_flags(0x08000000).spawn()?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let a = std::thread::spawn(move || { let mut text = String::new(); stdout.read_to_string(&mut text).map(|_| text) });
    let b = std::thread::spawn(move || { let mut text = String::new(); stderr.read_to_string(&mut text).map(|_| text) });
    let main = app.get_webview_window("main").ok_or_else(|| io::Error::other("Actual main window missing"))?;
    let mut authorized = Vec::new();
    let mut administration = 1;
    let start = Instant::now();
    let exit = loop {
        let current = state.info()?;
        for peer in current["connections"].as_array().unwrap() {
            if peer["authorized"] == true || peer["revoked"] == true { continue; }
            let mut choice = case.authority.clone();
            choice["project_id"] = project.clone();
            // The private, explicitly selected test case chooses authority;
            // neither model text nor an MCP request grants any capability.
            agent_pipe::agent_authorize_connection(main.clone(), app.state(), peer["session_id"].as_str().unwrap().to_owned(),
                serde_json::from_value(choice.clone()).map_err(io::Error::other)?)
                .map_err(io::Error::other)?;
            let session = ObjectId::try_from(peer["session_id"].as_str().unwrap().to_owned()).map_err(io::Error::other)?;
            let mut files = Vec::new();
            for (purpose, name) in [(sf_commands::FilePurpose::SaveProject, "saved"), (sf_commands::FilePurpose::ExportEvaluation, "reports"), (sf_commands::FilePurpose::ExportPackage, "module-export")] {
                let operation = serde_json::to_value(&purpose)?;
                if !choice["operations"].as_array().unwrap().contains(&operation) { continue; }
                let directory = case.output.join(name); fs::create_dir_all(&directory)?;
                let token = state.project.select_external_directory(&session, purpose, directory)
                    .map_err(|error| io::Error::other(error.message))?;
                files.push(token);
            }
            fs::write(case.output.join("actual-file-tokens.json"), serde_json::to_vec_pretty(&files)?)?;
            authorized.push(json!({"verified_OS_peer":peer,"choice":choice,"controlled_main_window_authorization":true}));
            fs::write(case.output.join("actual-authorizations.json"), serde_json::to_vec_pretty(&authorized)?)?;
        }
        let action = case.output.join(format!("ui-request-{administration}.json"));
        if action.exists() {
            let request: Value = serde_json::from_slice(&fs::read(action)?)?;
            let reply = state.project.dispatch_ui("main",request);
            let temporary = case.output.join(format!("ui-response-{administration}.tmp"));
            fs::write(&temporary,serde_json::to_vec_pretty(&reply)?)?;
            fs::rename(temporary,case.output.join(format!("ui-response-{administration}.json")))?;
            administration += 1;
        }
        if let Some(exit) = child.try_wait()? { break exit; }
        if start.elapsed() > Duration::from_secs(case.deadline_seconds) {
            let _ = Command::new("taskkill.exe").args(["/PID",&child.id().to_string(),"/T","/F"]).creation_flags(0x08000000).output();
            child.wait()?;
            return Err(io::Error::other("Actual Codex session exceeded the explicitly selected acceptance budget"));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    fs::write(case.output.join("launcher.stdout.txt"), a.join().unwrap()?)?;
    fs::write(case.output.join("launcher.stderr.txt"), b.join().unwrap()?)?;
    let context = state.project.dispatch_ui("main", request("P304-final-context", Value::Null, "get_context", json!({"selection":[]})));
    fs::write(case.output.join("actual-final-context.json"), serde_json::to_vec_pretty(&context)?)?;
    let active_project = context["outcome"]["data"]["project"]["id"].clone();
    let object = state.project.dispatch_ui("main", request("P304-final-project", active_project.clone(), "get_object", json!({"object_id":{"kind":"project","id":active_project},"source":{"kind":"draft"},"node_index":null})));
    fs::write(case.output.join("actual-final-project.json"), serde_json::to_vec_pretty(&object)?)?;
    fs::write(case.output.join("native-receipt.json"), serde_json::to_vec_pretty(&json!({"driver_exit_code":exit.code(),
        "actual_installed_Codex_client":matches!(case.client,Client::Codex),"actual_protocol_client":!matches!(case.client,Client::Codex),"single_actual_managed_ProjectApiState":true,"actual_production_Native_sources":true,
        "OS_verified_connections":authorized,"controlled_authorization_not_human_acceptance":true,
        "full_business_acceptance":"read actual Codex events, frozen Host outputs and files; this receipt alone is not a whole-task pass"}))?)?;
    Ok(exit.success())
}
#[tauri::command]
fn engineering_status() -> Value { json!({"phase":"S07","business_operations_ready":false}) }
#[tauri::command]
fn engineering_frontend_ready(phase: String, react_version: String, three_revision: String) {
    let _ = (phase,react_version,three_revision);
}
fn main() {
    let file = std::env::args().nth(1).expect("Explicit private Native acceptance case JSON required");
    let case: Case = serde_json::from_slice(&fs::read(file).unwrap()).unwrap();
    fs::create_dir(&case.output).expect("Acceptance output must be a new directory");
    tauri::Builder::default()
        .manage(project_api::ProjectApiState::new())
        .manage(project_api::native_capture_probe::NativeCaptureProbeState::default())
        .manage(project_api::public_render_smoke::PublicRenderSmokeState::default())
        .manage(native_render::NativeRenderState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![engineering_status, engineering_frontend_ready,
            native_render::native_render_input,native_render::native_render_asset,native_render::native_render_file,native_render::native_render_ready,native_render::native_render_failure,
            project_api::native_capture_probe::native_capture_probe_plan, project_api::project_api, project_api::project_close_state,
            project_api::project_select_directory,project_api::project_select_file,project_api::project_read_media_asset,
            project_api::project_read_stored_packaging,project_api::project_list_stored_packaging,project_api::project_read_stored_manufacturing_file,
            project_api::manufacturing_export::project_export_manufacturing_package,project_api::port_output::project_read_run_port_output,
            agent_pipe::agent_connection_info,agent_pipe::agent_authorize_connection,agent_pipe::agent_update_connection_authority,
            agent_pipe::agent_grant_task_access,agent_pipe::agent_revoke_connection,agent_pipe::agent_select_directory])
        .setup(move |app| {
            let state = app.state::<project_api::ProjectApiState>().inner().clone();
            state.register_bundled_library(&case.root.join("modules"))?;
            state.register_evaluation_resources(&case.root.join("tools/architecture-proof/assets"))?;
            state.set_render_backend(Arc::new(native_render::NativeRenderBackend::new(app.handle().clone(),
                app.state::<native_render::NativeRenderState>().inner().clone(),case.render_kinds.clone())))
                .map_err(|error| io::Error::other(error.message))?;
            WebviewWindowBuilder::new(app,"main",WebviewUrl::App("index.html".into()))
                .data_directory(case.output.join("webview-data")).visible(false).build()?;
            let project = import_actual(&state,&case)?;
            let pipe = agent_pipe::AgentPipeState::start(state)?; app.manage(pipe);
            let handle = app.handle().clone(); let case = case.clone();
            std::thread::spawn(move || {
                let result = execute(handle.clone(),case.clone(),project);
                if let Err(error) = &result { let _=fs::write(case.output.join("native-error.txt"),error.to_string()); }
                handle.exit(if matches!(result,Ok(true)){0}else{1});
            });
            Ok(())
        }).run(tauri::generate_context!()).expect("Actual Native acceptance app failed");
}
