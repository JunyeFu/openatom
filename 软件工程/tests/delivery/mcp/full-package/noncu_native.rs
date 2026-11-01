//! Controlled hidden consumer of production Native state, OS pipe and renderer.
#![allow(dead_code)]
#[path = "../../../../apps/planner/src-tauri/src/agent_pipe.rs"] mod agent_pipe;
#[path = "../../../../apps/planner/src-tauri/src/project_api.rs"] mod project_api;
#[path = "../../../../apps/planner/src-tauri/src/native_render.rs"] mod native_render;
#[path = "../../../../apps/planner/src-tauri/src/native_render_capture.rs"] mod native_render_capture;
use serde::Deserialize;
use serde_json::{Value,json};
use sf_domain::*;
use std::{fs,io::{self,Read},os::windows::process::CommandExt,path::PathBuf,process::{Command,Stdio},time::Duration};
use tauri::{Manager,WebviewUrl,WebviewWindowBuilder};

#[derive(Clone,Deserialize)]
struct Case { root:PathBuf, seed:PathBuf, python:PathBuf, bridge:PathBuf, output:PathBuf, profiles:Vec<Value>, mode:String, drafts:Vec<Value>, #[serde(default)] renderer:bool, #[serde(default)] candidate_device_detail:bool }
fn request(id:&str,project:Value,operation:&str,params:Value)->Value {
    json!({"api_version":"0.1","request_id":id,"scope":{"kind":if project.is_null(){"workspace"}else{"project"}},"project_id":project,"operation":operation,"params":params})
}
fn ok(reply:Value)->io::Result<Value> {
    if reply["outcome"]["status"]!="ok" {return Err(io::Error::other(reply.to_string()));}
    Ok(reply["outcome"]["data"].clone())
}
fn import(state:&project_api::ProjectApiState,case:&Case)->io::Result<Value> {
    let ui=ObjectId::try_from("desktop-main-ui".to_owned()).map_err(io::Error::other)?;
    let selection=if case.seed.is_file() {
        project_api::noncu_select_file(state,&ui,sf_commands::FilePurpose::ImportPackage,case.seed.clone())
    }else{
        state.select_external_directory(&ui,sf_commands::FilePurpose::ImportPackage,case.seed.clone())
    }.map_err(|e|io::Error::other(e.message))?;
    let task=ok(state.dispatch_ui("main",request("noncu-import",Value::Null,"import_package",json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null}))))?;
    let draft=loop {
        let state=ok(state.dispatch_ui("main",request("noncu-import-state",Value::Null,"get_task",json!({"task_id":task["id"]}))))?;
        match state["task"]["state"].as_str() {
            Some("completed")=>break state["result"]["data"].clone(),
            Some("failed"|"cancelled")=>return Err(io::Error::other(state.to_string())),
            _=>std::thread::sleep(Duration::from_millis(5)),
        }
    };
    ok(state.dispatch_ui("main",request("noncu-adopt",Value::Null,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{
        "chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}))))?;
    let context=ok(state.dispatch_ui("main",request("noncu-initial-context",Value::Null,"get_context",json!({"selection":[]}))))?;
    Ok(context["project"]["id"].clone())
}
fn execute(app:tauri::AppHandle,case:Case,project:Value)->io::Result<bool> {
    let state=app.state::<agent_pipe::AgentPipeState>().inner().clone();
    let info=state.info()?;
    fs::write(case.output.join("endpoint.json"),serde_json::to_vec_pretty(&info)?)?;
    let initial=state.project.dispatch_ui("main",request("noncu-before",project.clone(),"get_object",json!({"object_id":{"kind":"project","id":project},"source":{"kind":"draft"},"node_index":null})));
    fs::write(case.output.join("Host-before.json"),serde_json::to_vec_pretty(&initial)?)?;
    let mut child=Command::new(&case.python).args(["-B","-X","utf8"])
        .arg(case.root.join("tests/delivery/mcp/full-package/noncu_client.py"))
        .arg(&case.bridge).arg(info["endpoint"].as_str().unwrap()).arg(info["host_instance_id"].as_str().unwrap())
        .arg(&case.output).arg(&case.mode).arg(&case.root)
        .current_dir(&case.root).stdout(Stdio::piped()).stderr(Stdio::piped()).creation_flags(0x08000000).spawn()?;
    let mut stdout=child.stdout.take().unwrap();let mut stderr=child.stderr.take().unwrap();
    let a=std::thread::spawn(move||{let mut s=String::new();stdout.read_to_string(&mut s).map(|_|s)});
    let b=std::thread::spawn(move||{let mut s=String::new();stderr.read_to_string(&mut s).map(|_|s)});
    let main=app.get_webview_window("main").unwrap();
    let secondary=app.get_webview_window("secondary").unwrap();
    let mut peers=Vec::new();let mut index=1u64;
    let exit=loop {
        for peer in state.info()?["connections"].as_array().unwrap() {
            if peer["authorized"]==true||peer["revoked"]==true {continue;}
            let profile=case.profiles.get(peers.len()).ok_or_else(||io::Error::other("Unexpected extra peer outside fixed case profiles"))?.clone();
            agent_pipe::agent_authorize_connection(main.clone(),app.state(),peer["session_id"].as_str().unwrap().to_owned(),serde_json::from_value(profile.clone()).map_err(io::Error::other)?).map_err(io::Error::other)?;
            peers.push(json!({"verified_OS_peer":peer,"authority":profile}));
            fs::write(case.output.join("authorizations.json"),serde_json::to_vec_pretty(&peers)?)?;
        }
        let path=case.output.join(format!("admin-{index}.json"));
        if path.exists() {
            let action:Value=serde_json::from_slice(&fs::read(&path)?)?;
            let kind=action["kind"].as_str().unwrap();
            let session=action["session"].as_str().unwrap_or("").to_owned();
            let result=match kind {
                "ui"=>state.project.dispatch_ui("main",action["request"].clone()),
                "probe_eof"=>{
                    let req=serde_json::from_value(request("noncu-eof-probe",project.clone(),"get_context",json!({"selection":[]}))).map_err(io::Error::other)?;
                    state.project.dispatch_session(&ObjectId::try_from(session).map_err(io::Error::other)?,req)
                },
                "task_read"|"task_control"|"secondary_task"=>{
                    let result=agent_pipe::agent_grant_task_access(if kind=="secondary_task"{secondary.clone()}else{main.clone()},app.state(),session,
                        serde_json::from_value(action["task"].clone()).map_err(io::Error::other)?,kind=="task_control");
                    match result {Ok(())=>json!({"ok":true}),Err(e)=>json!({"ok":false,"error":e})}
                },
                "revoke"=>{
                    let result=agent_pipe::agent_revoke_connection(main.clone(),app.state(),session);
                    match result {Ok(())=>json!({"ok":true}),Err(e)=>json!({"ok":false,"error":e})}
                },
                "select"=>{
                    let relative=PathBuf::from(action["relative"].as_str().unwrap());
                    if relative.is_absolute()||relative.components().any(|c|matches!(c,std::path::Component::ParentDir)) {return Err(io::Error::other("Controlled file selection must stay in case output"));}
                    let target=case.output.join(relative);
                    let purpose=serde_json::from_value(action["purpose"].clone()).map_err(io::Error::other)?;
                    let session=ObjectId::try_from(session).map_err(io::Error::other)?;
                    let token=if action["file"]==true {
                        fs::create_dir_all(target.parent().unwrap())?;
                        project_api::noncu_select_file(&state.project,&session,purpose,target.clone())
                    }else{
                        fs::create_dir_all(&target)?;
                        state.project.select_external_directory(&session,purpose,target.clone())
                    }.map_err(|e|io::Error::other(e.message))?;
                    json!({"token":token,"path":target})
                },
                "authority"|"secondary_authority"=>{
                    let mut choice=case.profiles[action["profile"].as_u64().unwrap() as usize].clone();
                    match action["variant"].as_str().unwrap_or("restore") {
                        "remove_fields"=>for grant in choice["edit"].as_array_mut().unwrap(){grant["objects"]=json!([]);},
                        "drop_operations"=>choice["operations"].as_array_mut().unwrap().retain(|x|!matches!(x.as_str(),Some("get_object"|"get_task"|"cancel_task"|"apply_commands"))),
                        "wrong_project"=>choice["project_id"]=json!("not-granted"),
                        "bind_single_definition"=>{
                            let caps=ok(state.project.dispatch_ui("main",request("noncu-actual-draft-scopes",Value::Null,"describe_capabilities",json!({}))))?;
                            let identities=caps["implemented_operations"].as_array().unwrap().iter().flat_map(|operation|operation["allowed_scopes"].as_array().unwrap()).filter(|scope|scope["kind"]=="definition_draft").filter_map(|scope|scope["data"].as_str()).collect::<std::collections::BTreeSet<_>>();
                            if identities.len()!=1{return Err(io::Error::other("Explicit generated-device case needs one actual UI-visible DefinitionDraft"));}
                            let actual=identities.into_iter().next().unwrap();
                            choice["definition_drafts"]=json!([actual]);
                            for grant in choice["edit"].as_array_mut().unwrap(){grant["scope"]["data"]["data"]["id"]=json!(actual);for object in grant["objects"].as_array_mut().unwrap(){object["target"]["id"]=json!(actual);}}
                        },
                        "restore"=>{},_=>return Err(io::Error::other("Unknown fixed authority variant")),
                    }
                    let result=agent_pipe::agent_update_connection_authority(if kind=="secondary_authority"{secondary.clone()}else{main.clone()},app.state(),session,
                        serde_json::from_value(choice).map_err(io::Error::other)?);
                    match result {Ok(())=>json!({"ok":true}),Err(e)=>json!({"ok":false,"error":e})}
                },
                _=>return Err(io::Error::other("Unknown controlled Native administration action")),
            };
            let temporary=case.output.join(format!("admin-{index}.response.tmp"));
            fs::write(&temporary,serde_json::to_vec_pretty(&result)?)?;
            fs::rename(temporary,case.output.join(format!("admin-{index}.response.json")))?;
            index+=1;
        }
        if let Some(exit)=child.try_wait()?{break exit;}
        std::thread::sleep(Duration::from_millis(5));
    };
    fs::write(case.output.join("client.stdout.txt"),a.join().unwrap()?)?;
    fs::write(case.output.join("client.stderr.txt"),b.join().unwrap()?)?;
    let context=state.project.dispatch_ui("main",request("noncu-final-context",Value::Null,"get_context",json!({"selection":[]})));
    let current=context["outcome"]["data"]["project"]["id"].clone();
    let final_project=state.project.dispatch_ui("main",request("noncu-after",current.clone(),"get_object",json!({"object_id":{"kind":"project","id":current},"source":{"kind":"draft"},"node_index":null})));
    fs::write(case.output.join("Host-after.json"),serde_json::to_vec_pretty(&final_project)?)?;
    // External EOF releases task authority, so the later UI context is not the
    // source for completed external image evidence. Use the recorded MCP replies.
    let client_result:Value=serde_json::from_slice(&fs::read(case.output.join("client-result.json"))?)?;
    let completed_render_tasks=client_result["render_images"].as_array().map(|images|images.iter().filter_map(|image|image["task"]["id"].as_str()).collect::<std::collections::BTreeSet<_>>().len()).unwrap_or(0);
    fs::write(case.output.join("native-receipt.json"),serde_json::to_vec_pretty(&json!({"client_exit":exit.code(),"actual_production_Native_state_and_pipe":true,
        "window_visible":false,"window_url":"about:blank","product_render_backend_registered":case.renderer,"candidate_device_detail_registration":case.candidate_device_detail,"completed_product_render_tasks":completed_render_tasks,"render_evidence_source":"client-result.json render_images plus raw completed get_task MCP image replies","product_frontend_or_renderer_executed":completed_render_tasks>0,"real_Codex":false,"human_acceptance":null,"whole_package_done":false}))?)?;
    Ok(exit.success())
}
fn main(){
    let mut case:Case=serde_json::from_slice(&fs::read(std::env::args().nth(1).expect("Explicit private case")).unwrap()).unwrap();
    fs::create_dir(&case.output).expect("New output directory required");
    fs::copy(case.output.parent().unwrap().join("input.json"),case.output.join("input.json")).expect("Preserve this attempt's exact frozen input");
    tauri::Builder::default().manage(project_api::ProjectApiState::new())
        .manage(project_api::native_capture_probe::NativeCaptureProbeState::default())
        .manage(project_api::public_render_smoke::PublicRenderSmokeState::default())
        .manage(native_render::NativeRenderState::default()).plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![native_render::native_render_input,native_render::native_render_asset,native_render::native_render_file,native_render::native_render_ready,native_render::native_render_failure])
        .setup(move|app|{
            for label in ["main","secondary"] {
                WebviewWindowBuilder::new(app,label,WebviewUrl::External("about:blank".parse().unwrap()))
                    .visible(false).focused(false).skip_taskbar(true).data_directory(case.output.join(format!("webview-{label}"))).build()?;
            }
            let state=app.state::<project_api::ProjectApiState>().inner().clone();
            if case.renderer {
                let mut supported=vec![RenderViewKind::LineCanvas,RenderViewKind::MultiHallCanvas,RenderViewKind::TimelineSnapshot];
                if case.candidate_device_detail { supported.push(RenderViewKind::DeviceDetail); }
                state.set_render_backend(std::sync::Arc::new(native_render::NativeRenderBackend::new(app.handle().clone(),app.state::<native_render::NativeRenderState>().inner().clone(),supported))).map_err(|error|io::Error::other(error.message))?;
            }
            state.register_bundled_library(&case.root.join("modules")).map_err(io::Error::other)?;
            state.register_evaluation_resources(&case.root.join("tools/architecture-proof/assets")).map_err(io::Error::other)?;
            let project=import(&state,&case)?;
            let mut created_drafts=Vec::new();
            for definition in &case.drafts {
                let commit=ok(state.dispatch_ui("main",request("noncu-controlled-draft-setup",project.clone(),"apply_commands",
                    json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":definition}]}))))?;
                // The actual Host allocates runtime draft identities separately
                // from the ModuleDefinition ID. Bind the fixed grants to that
                // returned identity before authorizing any external caller.
                let draft_id=commit["scope"]["data"]["id"].clone();
                for profile in &mut case.profiles {
                    for id in profile["definition_drafts"].as_array_mut().unwrap() {
                        if *id==definition["id"] {*id=draft_id.clone();}
                    }
                    for grant in profile["edit"].as_array_mut().unwrap() {
                        if grant["scope"]["data"]["kind"]=="definition_draft" && grant["scope"]["data"]["data"]["id"]==definition["id"] {
                            grant["scope"]["data"]["data"]["id"]=draft_id.clone();
                            for object in grant["objects"].as_array_mut().unwrap() {
                                if object["target"]["id"]==definition["id"] {object["target"]["id"]=draft_id.clone();}
                            }
                        }
                    }
                }
                created_drafts.push(commit);
            }
            fs::write(case.output.join("controlled-draft-setup.json"),serde_json::to_vec_pretty(&created_drafts)?)?;
            app.manage(agent_pipe::AgentPipeState::start(state)?);
            let handle=app.handle().clone();let case=case.clone();
            std::thread::spawn(move||{
                let result=execute(handle.clone(),case.clone(),project);
                if let Err(error)=&result{let _=fs::write(case.output.join("native-error.txt"),error.to_string());}
                handle.exit(if matches!(result,Ok(true)){0}else{1});
            });
            Ok(())
        }).run(tauri::generate_context!()).expect("Controlled hidden Native failed");
}
