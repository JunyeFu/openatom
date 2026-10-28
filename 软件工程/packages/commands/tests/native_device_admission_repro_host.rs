//! Manual diagnosis of an actual saved Native project; this is not sample acceptance.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::json;
use sf_commands::*;
use sf_domain::*;
use support::*;

#[test]
#[ignore = "requires SF_NATIVE_DEVICE_REPRO pointing to the retained Native66 project"]
fn inspect_actual_native_device_sample_admission() {
    let directory = std::path::PathBuf::from(std::env::var_os("SF_NATIVE_DEVICE_REPRO").expect("actual saved Native directory"));
    let tree = Sandbox::new();
    let ui = id("native-device-diagnostic-ui");
    let host = ProjectHost::new(id("native-device-diagnostic-host"), ui.clone());
    let mut h = Harness { host, ui, tree };
    let imported = h.import(&directory);
    h.finish_one();
    let proposal = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &proposal, true));
    let module: DomainObject = ok(h.call("get_object", json!({
        "object_id":{"kind":"module","id":"native65.device.p301"},
        "source":{"kind":"module_version","data":{"module_ref":{"id":"native65.device.p301","version":"0.1.0"}}},"node_index":null
    })));
    let DomainObject::Module(module) = module else { panic!("actual saved device module") };
    let base = h.context().project.unwrap().revision.0.value();
    let commit: EditCommit = ok(h.apply(base, json!([{"kind":"create_definition_draft","data":module.definition}])));
    let EditScope::DefinitionDraft(draft) = commit.scope else { panic!("actual definition draft") };
    let input = h.object("scenario", "IX01-baseline");
    let request: ApiRequest = serde_json::from_value(json!({
        "api_version":"0.1","request_id":"actual-native-device-admission-diagnostic","project_id":"P1",
        "scope":{"kind":"definition_draft","data":draft.id},"operation":"run_scenario",
        "params":{"scenario_id":input["id"],"input_revision":commit.new_revision,"observation_window":input["window"],
            "intent":{"kind":"definition_cycle","data":{"definition_revision":commit.new_revision,"test_input":input}}}
    })).unwrap();
    let reply = h.host.dispatch(&h.ui, request);
    println!("{}", serde_json::to_string_pretty(&reply).unwrap());
    let detail = error(reply);
    assert!(matches!(detail.code, ErrorCode::InvalidInput));
    assert!(!detail.field_errors.is_empty(), "the actual rejection must explain its fields");

    // Replay the user's two explicit material-connection edits in this isolated
    // Host. The retained Native project on disk is never rewritten by this probe.
    let before = h.object("project", "P1");
    let device = before["devices"].as_array().unwrap().iter().find(|d| d["id"] == "CNC01").unwrap();
    let mut edits = vec![];
    for connection in before["connections"].as_array().unwrap() {
        let mut next = connection.clone();
        for endpoint in ["source", "destination"] {
            if next["kind"] == "material" && next["data"][endpoint]["kind"] == "port" && next["data"][endpoint]["data"]["instance_id"] == "CNC01" {
                let port = next["data"][endpoint]["data"]["port_id"].as_str().unwrap().to_owned();
                next["data"][endpoint]["data"] = device["role_bindings"]["ports"][&port].clone();
            }
        }
        if next != *connection { edits.push(put("connection", next)); }
    }
    assert_eq!(edits.len(), 2);
    let _: EditCommit = ok(h.apply(base, json!(edits)));
    let input = h.object("scenario", "IX01-baseline");
    let request: ApiRequest = serde_json::from_value(json!({
        "api_version":"0.1","request_id":"actual-native-device-after-explicit-connections","project_id":"P1",
        "scope":{"kind":"definition_draft","data":draft.id},"operation":"run_scenario",
        "params":{"scenario_id":input["id"],"input_revision":commit.new_revision,"observation_window":input["window"],
            "intent":{"kind":"definition_cycle","data":{"definition_revision":commit.new_revision,"test_input":input}}}
    })).unwrap();
    let task: TaskHandle = ok(h.host.dispatch(&h.ui, request));
    h.finish_one();
    let request: ApiRequest = serde_json::from_value(json!({
        "api_version":"0.1","request_id":"actual-native-device-result","project_id":"P1",
        "scope":{"kind":"definition_draft","data":draft.id},"operation":"get_task","params":{"task_id":task.id}
    })).unwrap();
    let state: TaskState = ok(h.host.dispatch(&h.ui, request));
    println!("After two explicit connection edits: {}", serde_json::to_string_pretty(&state).unwrap());
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    assert!(matches!(state.result, Some(TaskOutput::Run(_))));
}
