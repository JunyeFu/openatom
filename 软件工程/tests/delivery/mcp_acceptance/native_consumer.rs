//! Independent actual native producer consumer, without modifying its implementation.
#![allow(dead_code)]
#[path = "../../../apps/planner/src-tauri/src/agent_pipe.rs"]
mod agent_pipe;
#[path = "../../../apps/planner/src-tauri/src/project_api.rs"]
mod project_api;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::os::windows::process::CommandExt;
use std::{
    io,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
fn request(id: &str, project: Value, op: &str, params: Value) -> Value {
    json!({"api_version":"0.1","request_id":id,"project_id":project,"scope":{"kind":if project.is_null(){"workspace"}else{"project"}},"operation":op,"params":params})
}
fn checked(v: Value) -> Value {
    assert_eq!(v["outcome"]["status"], "ok", "{v}");
    v["outcome"]["data"].clone()
}
fn load_actual(state: &project_api::ProjectApiState, seed: &Path) -> Value {
    let ui = ObjectId::try_from("desktop-main-ui".to_owned()).unwrap();
    let token = state
        .select_external_directory(&ui, FilePurpose::ImportPackage, seed.to_owned())
        .unwrap();
    let task = checked(state.dispatch_ui(
        "main",
        request(
            "independent-import",
            Value::Null,
            "import_package",
            json!({"selected_file_ref":token,"import_policy":"open_project","base_revision":null}),
        ),
    ));
    let start = Instant::now();
    let draft = loop {
        let status = checked(state.dispatch_ui(
            "main",
            request(
                "independent-import-state",
                Value::Null,
                "get_task",
                json!({"task_id":task["id"]}),
            ),
        ));
        if status["task"]["state"] == "completed" {
            break status["result"]["data"].clone();
        }
        assert_ne!(status["task"]["state"], "failed", "{status}");
        assert!(start.elapsed() < Duration::from_secs(10));
        std::thread::sleep(Duration::from_millis(10));
    };
    checked(state.dispatch_ui("main",request("independent-open",Value::Null,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{
        "chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}))));
    json!("P1")
}
fn execute(
    state: agent_pipe::AgentPipeState,
    app: tauri::AppHandle,
    project: Value,
) -> io::Result<bool> {
    use std::io::Read;
    let root = std::env::current_dir()?;
    let out = std::path::PathBuf::from(std::env::var_os("W5_NATIVE_OUTPUT").unwrap());
    std::fs::create_dir_all(&out)?;
    let info = state.info()?;
    std::fs::write(out.join("endpoint.json"), serde_json::to_vec(&info)?)?;
    let python = std::env::var_os("W5_TEST_PYTHON").unwrap();
    let mut child = Command::new(python)
        .arg("-X")
        .arg("utf8")
        .arg("-B")
        .arg(root.join("tests/delivery/mcp_acceptance/process_consumer.py"))
        .arg(std::env::var_os("W5_NATIVE_BRIDGE").unwrap())
        .arg(&info["endpoint"].as_str().unwrap())
        .arg(info["host_instance_id"].as_str().unwrap())
        .arg(&out)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let a = std::thread::spawn(move || {
        let mut s = String::new();
        stdout.read_to_string(&mut s).map(|_| s)
    });
    let b = std::thread::spawn(move || {
        let mut s = String::new();
        stderr.read_to_string(&mut s).map(|_| s)
    });
    let start = Instant::now();
    let mut admitted = 0;
    let mut sessions = vec![];
    let mut revoked = false;
    let exit = loop {
        let pending = state.info()?["connections"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["authorized"] == false && c["revoked"] == false)
            .cloned()
            .collect::<Vec<_>>();
        for peer in pending {
            assert!(peer["pid"].as_u64().unwrap() > 0);
            let editable = admitted == 1;
            let grant = if editable {
                json!([{"scope":{"kind":"edit","data":{"kind":"project","data":project}},"branches":["set_project_settings","put_object"],
                "objects":[{"target":{"kind":"project","id":project},"fields":[{"pointer":"/title","element_id":null}],"can_remove":false}],
                "creates":[{"kind":"item_type","max_new_objects":1,"fields":[{"pointer":"/revision","element_id":null},{"pointer":"/title","element_id":null},{"pointer":"/unit","element_id":null},{"pointer":"/tracking","element_id":null},{"pointer":"/spec_ref","element_id":null},{"pointer":"/attributes","element_id":null},{"pointer":"/attribute_schema","element_id":null}]}],"source_versions":[],"reusable_resources":[]}])
            } else {
                json!([])
            };
            let choice=serde_json::from_value(json!({"project_id":project,"operations":["describe_capabilities","get_context","get_object","apply_commands","run_scenario","get_run","save_project","get_task","cancel_task","query_module_library","check_factory_draft","render_view","export_evaluation"],"definition_drafts":[],"edit":grant})).unwrap();
            let session = peer["session_id"].as_str().unwrap().to_owned();
            state.authorize(&session, choice)?;
            if editable {
                std::fs::create_dir_all(out.join("saved"))?;
                let token = state
                    .project
                    .select_external_directory(
                        &ObjectId::try_from(session.clone()).unwrap(),
                        FilePurpose::SaveProject,
                        out.join("saved"),
                    )
                    .map_err(|e| io::Error::other(e.message))?;
                std::fs::write(
                    out.join("selection.json"),
                    serde_json::to_vec(&json!({"session_id":session,"selection":token}))?,
                )?;
            }
            sessions.push(json!({"verified_peer":peer,"editable":editable}));
            admitted += 1;
        }
        if !revoked && out.join("revoke.json").exists() {
            let command: Value = serde_json::from_slice(&std::fs::read(out.join("revoke.json"))?)?;
            agent_pipe::agent_revoke_connection(
                app.get_webview_window("main").unwrap(),
                app.state::<agent_pipe::AgentPipeState>(),
                command["session_id"].as_str().unwrap().to_owned(),
            )
            .map_err(io::Error::other)?;
            std::fs::write(out.join("revoked.json"), b"true")?;
            revoked = true;
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(55) {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::other("Independent MCP process exceeded55s"));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    std::fs::write(out.join("client.stdout.txt"), a.join().unwrap()?)?;
    std::fs::write(out.join("client.stderr.txt"), b.join().unwrap()?)?;
    let read=checked(state.project.dispatch_ui("main",request("independent-ui-readback",project.clone(),"get_object",json!({"object_id":{"kind":"project","id":project},"source":{"kind":"draft"},"node_index":null}))));
    let context = checked(state.project.dispatch_ui(
        "main",
        request(
            "independent-ui-context",
            project,
            "get_context",
            json!({"selection":[]}),
        ),
    ));
    let shared = read["data"]["title"] == "Edited by independent native MCP";
    let report = json!({"producer":"actual production AgentPipeState and ProjectApiState managed together by Tauri; actual bridge executable; controlled JSON-RPC consumer","exit":exit.code(),"shared_native_ui_readback":shared,"context":context,"actual_os_peers":sessions,"real_codex_business_verified":false,"human_acceptance":false});
    std::fs::write(
        out.join("native-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "{}",
        json!({"independent_native_mcp_exit":exit.code(),"shared_ui_readback":shared,"report":out.join("native-report.json")})
    );
    Ok(exit.success() && shared)
}
fn main() {
    tauri::Builder::default()
        .manage(project_api::ProjectApiState::new())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .visible(false)
                .build()?;
            let state = app.state::<project_api::ProjectApiState>().inner().clone();
            let seed = std::env::var_os("W5_NATIVE_SEED").unwrap();
            let project = load_actual(&state, Path::new(&seed));
            let pipe = agent_pipe::AgentPipeState::start(state)?;
            app.manage(pipe.clone());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let ok = match execute(pipe, handle.clone(), project) {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Independent MCP consumer: {e}");
                        false
                    }
                };
                handle.exit(if ok { 0 } else { 1 });
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("independent native Tauri runner failed");
}
