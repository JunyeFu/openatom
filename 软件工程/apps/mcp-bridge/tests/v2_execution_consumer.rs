//! Component consumers plus a real saved-r4 ProjectHost read/denial boundary.
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::ObjectId;
use sf_mcp_bridge::{
    catalog,
    decode::decode,
    execution_identity::{ExecutionObservations, validate_reply},
};

const OPS: [&str; 7] = [
    "start_execution_session",
    "get_execution_snapshot",
    "control_execution_session",
    "read_cloud_production",
    "read_execution_view",
    "export_execution_report",
    "export_cloud_data",
];
const PRIVATE: [&str; 4] = [
    "apply_device_receipt",
    "apply_device_observation",
    "get_execution_cloud_work",
    "acknowledge_execution_cloud_work",
];

#[test]
fn v2_effect_hints_match_the_actual_task_and_directory_effects() {
    let operations=OPS.map(|name|json!({"operation":name,
        "input_schema_ref":format!("{name}.request.schema.json"),"output_schema_ref":format!("{name}.response.schema.json"),
        "allowed_scopes":[{"kind":"project"}],"available_controls":[]}));
    let source:CapabilityCatalog=serde_json::from_value(json!({"api_version":"0.1","implemented_operations":operations,
        "available_command_kinds":[],"providers":[],"category_ids":[],"media_kinds":[],"native_primitives":[],
        "max_ipc_frame_bytes":1048576,"limitations":[]})).unwrap();
    for tool in catalog::tools(&source).unwrap() {
        let value = serde_json::to_value(tool).unwrap();
        let name = value["name"].as_str().unwrap();
        let read = matches!(
            name,
            "superfactory_get_execution_snapshot" | "superfactory_read_execution_view"
        );
        let destructive = matches!(
            name,
            "superfactory_control_execution_session" | "superfactory_export_execution_report"
        );
        assert_eq!(value["annotations"]["readOnlyHint"], read, "{name}");
        assert_eq!(value["annotations"]["idempotentHint"], read, "{name}");
        assert_eq!(
            value["annotations"]["destructiveHint"], destructive,
            "{name}"
        );
    }
}

fn telemetry_query(cursor: Value) -> Value {
    json!({"dataset":"telemetry","contract_version":"sf.execution/2.0-draft11","cloud_project_id":"cloud-project",
        "dataset_id":"dataset","device_ids":["A"],"received_from":"2026-10-28T00:00:00.000000Z",
        "received_until":"2026-10-28T00:02:00.000000Z","aggregation":"count_by_device_minute","page":{"limit":200,"cursor":cursor}})
}
fn telemetry_page(frontier: u64, next: Value) -> Value {
    json!({"dataset":"telemetry","contract_version":"sf.execution/2.0-draft11","selected_epoch":null,
        "selected_execution_revision":null,"rows":[{"kind":"count_by_device_minute","device_id":"A",
        "source_mode":"software_in_loop","bucket_start":"2026-10-28T00:00:00.000000Z","count":2,"row_index":0}],
        "next_cursor":next,"data_quality_issues":[],"dataset_id":"dataset","selected_visible_frontier":frontier,"selected_device_ids":["A"]})
}
fn observe_query(
    observations: &mut ExecutionObservations,
    name: &str,
    query: Value,
    page: Value,
) -> std::io::Result<()> {
    let admission = request(
        "read_cloud_production",
        name,
        json!({"execution_contract_version":"sf.execution/2.0-draft11",
        "selection":{"kind":"data_query","query":query}}),
    );
    serde_json::from_value::<ApiRequest>(admission.clone()).unwrap();
    let mut task = handle("cloud_read");
    task["id"] = json!(name);
    let admitted = response(&admission, task.clone());
    decoded(&admission, admitted.clone()).unwrap();
    observations.observe(&admission, &admitted)?;
    task["state"] = json!("completed");
    let get = request("get_task", &format!("get-{name}"), json!({"task_id":name}));
    let reply = response(
        &get,
        json!({"task":task,"originating_operation":"read_cloud_production","originating_request_id":name,
        "allowed_controls":[],"result":{"kind":"cloud_read","data":{"kind":"query_result","data":page}},"failure":null}),
    );
    let output = decoded(&get, reply)?;
    observations.observe(&get, &output.original)
}

#[test]
fn cloud_query_consumer_rejects_a_different_selected_device_set_or_aggregation() {
    observe_query(
        &mut ExecutionObservations::default(),
        "valid-page",
        telemetry_query(Value::Null),
        telemetry_page(8, Value::Null),
    )
    .unwrap();
    let mut foreign = telemetry_page(8, Value::Null);
    foreign["selected_device_ids"] = json!(["B"]);
    assert!(
        observe_query(
            &mut ExecutionObservations::default(),
            "foreign-devices",
            telemetry_query(Value::Null),
            foreign
        )
        .is_err()
    );
    let mut query = telemetry_query(Value::Null);
    query["aggregation"] = json!("rows");
    assert!(
        observe_query(
            &mut ExecutionObservations::default(),
            "wrong-aggregation",
            query,
            telemetry_page(8, Value::Null)
        )
        .is_err()
    );
}

#[test]
fn cloud_query_pages_keep_the_learned_frontier_and_original_filter() {
    for changed in ["frontier", "filter"] {
        let mut observations = ExecutionObservations::default();
        observe_query(
            &mut observations,
            "first",
            telemetry_query(Value::Null),
            telemetry_page(8, json!("next-cursor")),
        )
        .unwrap();
        let mut query = telemetry_query(json!("next-cursor"));
        let page = if changed == "frontier" {
            telemetry_page(9, Value::Null)
        } else {
            query["received_until"] = json!("2026-10-28T00:03:00.000000Z");
            telemetry_page(8, Value::Null)
        };
        assert!(
            observe_query(&mut observations, "second", query, page).is_err(),
            "{changed}"
        );
    }
    let mut observations = ExecutionObservations::default();
    observe_query(
        &mut observations,
        "first",
        telemetry_query(Value::Null),
        telemetry_page(8, json!("next-cursor")),
    )
    .unwrap();
    observe_query(
        &mut observations,
        "second",
        telemetry_query(json!("next-cursor")),
        telemetry_page(8, Value::Null),
    )
    .unwrap();
}

#[test]
fn cloud_query_rows_and_repeated_admissions_cannot_change_the_bound_identity() {
    let mut observations = ExecutionObservations::default();
    observe_query(
        &mut observations,
        "same-admission",
        telemetry_query(Value::Null),
        telemetry_page(8, Value::Null),
    )
    .unwrap();
    let mut changed = telemetry_query(Value::Null);
    changed["received_until"] = json!("2026-10-28T00:03:00.000000Z");
    assert!(
        observe_query(
            &mut observations,
            "same-admission",
            changed,
            telemetry_page(8, Value::Null)
        )
        .is_err()
    );
    let identity = json!({"session_id":"execution","execution_epoch_id":"epoch","project_id":"Task8-two-device-six-units","host_context_session_id":"host"});
    let query = json!({"dataset":"execution","contract_version":"sf.execution/2.0-draft11","identity":identity,
        "selection":{"kind":"revision","execution_revision":8},"host_from_ms":0,"host_until_ms":20000,
        "aggregation":"command_counts","page":{"limit":200,"cursor":null}});
    let page = json!({"dataset":"execution","contract_version":"sf.execution/2.0-draft11","selected_epoch":"epoch",
        "selected_execution_revision":8,"rows":[{"kind":"command_count","identity":identity,"execution_revision":8,
        "state":"device_completed","count":1,"row_index":0}],"next_cursor":null,"data_quality_issues":[]});
    observe_query(
        &mut ExecutionObservations::default(),
        "valid-execution",
        query.clone(),
        page.clone(),
    )
    .unwrap();
    for key in ["identity", "execution_revision"] {
        let mut foreign = page.clone();
        if key == "identity" {
            foreign["rows"][0]["identity"]["execution_epoch_id"] = json!("another-epoch");
        } else {
            foreign["rows"][0]["execution_revision"] = json!(9);
        }
        assert!(
            observe_query(
                &mut ExecutionObservations::default(),
                "foreign-row",
                query.clone(),
                foreign
            )
            .is_err(),
            "{key}"
        );
    }
}

#[test]
fn telemetry_raw_rows_preserve_microsecond_filter_and_frontier_boundaries() {
    let mut query = telemetry_query(Value::Null);
    query["aggregation"] = json!("rows");
    query["received_from"] = json!("2026-10-28T00:00:00.000001Z");
    query["received_until"] = json!("2026-10-28T00:00:00.000003Z");
    let mut page = telemetry_page(8, Value::Null);
    page["rows"] = json!([{"kind":"generated_load","ingress_id":1,"dataset_id":"dataset",
        "cloud_received_at":"2026-10-28T00:00:00.000002Z","event":{"device_ref":"A","device_sequence":1,
        "source_mode":"generated_load","observed_at":"2026-10-28T00:00:00.000000Z","metric":"status_sample","value":1}}]);
    observe_query(
        &mut ExecutionObservations::default(),
        "valid-microsecond-row",
        query.clone(),
        page.clone(),
    )
    .unwrap();
    for (pointer, value) in [
        (
            "/rows/0/cloud_received_at",
            json!("2026-10-28T00:00:00.000000Z"),
        ),
        (
            "/rows/0/cloud_received_at",
            json!("2026-10-28T00:00:00.000003Z"),
        ),
        ("/rows/0/ingress_id", json!(9)),
        ("/rows/0/event/device_ref", json!("B")),
        ("/rows/0/dataset_id", json!("another-dataset")),
    ] {
        let mut foreign = page.clone();
        *foreign.pointer_mut(pointer).unwrap() = value;
        assert!(
            observe_query(
                &mut ExecutionObservations::default(),
                "foreign-raw-row",
                query.clone(),
                foreign
            )
            .is_err(),
            "{pointer}"
        );
    }
}
fn context() -> Value {
    json!({"host_instance_id":"host", "session_id":"caller", "project_id":"Task8-two-device-six-units",
        "project_revision":4, "scope":{"kind":"project","data":"Task8-two-device-six-units"}, "scope_revision":4})
}
fn request(operation: &str, request_id: &str, params: Value) -> Value {
    json!({"api_version":"0.1", "request_id":request_id, "scope":{"kind":"project"},
        "project_id":"Task8-two-device-six-units", "operation":operation, "params":params})
}
fn response(request: &Value, data: Value) -> Value {
    json!({"api_version":"0.1","request_id":request["request_id"],"context":context(),"outcome":{"status":"ok","data":data}})
}
fn decoded(request: &Value, reply: Value) -> std::io::Result<sf_mcp_bridge::decode::Decoded> {
    decode(
        json!({"wire_version":"1","host_instance_id":"host","client_session_id":"caller",
        "originating_operation":request["operation"], "reply":reply,"managed_results":null,"managed_assets":null}),
        request,
        &json!("host"),
        &json!("caller"),
        |_, _| panic!("No fixture leases"),
    )
}
fn handle(kind: &str) -> Value {
    json!({"id":"task", "kind":kind,"scope":{"kind":"edit","data":{"kind":"project","data":"Task8-two-device-six-units"}},
        "state":"queued","input_revision":4,"progress":null,"result_ref":null,"failure":null})
}

#[test]
fn actual_schemas_and_ordinary_agent_catalog_never_expose_gateway() {
    let operations = OPS.into_iter().chain(PRIVATE).map(|name|json!({"operation":name,
        "input_schema_ref":format!("{name}.request.schema.json"),"output_schema_ref":format!("{name}.response.schema.json"),
        "allowed_scopes":[{"kind":"project"}],"available_controls":[]})).collect::<Vec<_>>();
    let catalog: CapabilityCatalog=serde_json::from_value(json!({"api_version":"0.1","implemented_operations":operations,
        "available_command_kinds":[],"providers":[],"category_ids":[],"media_kinds":[],"native_primitives":[],
        "max_ipc_frame_bytes":1048576,"limitations":[]})).unwrap();
    let tools = catalog::tools(&catalog).unwrap();
    assert_eq!(tools.len(), 7);
    for op in OPS {
        let op: OperationName = serde_json::from_value(json!(op)).unwrap();
        let schema = catalog::tool_schema(&op).unwrap();
        assert_eq!(schema["additionalProperties"], false);
        for forbidden in ["caller", "grants", "url", "path", "script", "token"] {
            assert!(schema["properties"].get(forbidden).is_none());
        }
    }
    for op in PRIVATE {
        assert!(!catalog::agent_operation(
            &serde_json::from_value(json!(op)).unwrap()
        ));
    }
    let export = catalog::tool_schema(&OperationName::ExportExecutionReport)
        .unwrap()
        .to_string();
    assert!(export.contains("export_execution_report"));
    assert!(!export.contains("xlsx"));
}

#[test]
fn executable_python_requests_match_actual_rust_wire_without_handmade_schema() {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../.work/v2-implementation-20261026/agent-execution-consumer/workflow-requests.json",
    );
    // Optional cross-language fixture: the product tests remain standalone.
    if !file.exists() {
        return;
    }
    let requests: Vec<Value> = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert!(!requests.is_empty());
    for request in requests {
        assert!(
            serde_json::from_value::<ApiRequest>(request.clone()).is_ok(),
            "{request:#}"
        );
    }
}

#[test]
fn new_task_types_keep_original_kind_operation_request_and_frozen_input() {
    for (op, kind) in [
        ("start_execution_session", "execution_preparation"),
        ("read_cloud_production", "cloud_read"),
        ("export_execution_report", "exporting"),
        ("export_cloud_data", "exporting"),
    ] {
        let admission = request(op, "original", json!({}));
        let admitted = response(&admission, handle(kind));
        decoded(&admission, admitted.clone()).unwrap();
        let mut observations = ExecutionObservations::default();
        observations.observe(&admission, &admitted).unwrap();
        let read = request("get_task", "read", json!({"task_id":"task"}));
        let state = json!({"task":handle(kind),"originating_operation":op,"originating_request_id":"original",
            "allowed_controls":["cancel"],"result":null,"failure":null});
        let actual = response(&read, state);
        decoded(&read, actual.clone()).unwrap();
        observations.observe(&read, &actual).unwrap();
        let mut foreign = actual.clone();
        foreign["outcome"]["data"]["originating_request_id"] = json!("different-original");
        assert!(observations.observe(&read, &foreign).is_err());
        let mut wrong = actual.clone();
        wrong["outcome"]["data"]["task"]["kind"] = json!("simulation");
        assert!(decoded(&read, wrong).is_err());
        let mut wrong = actual.clone();
        wrong["outcome"]["data"]["task"]["id"] = json!("another");
        assert!(decoded(&read, wrong).is_err());
        let mut wrong = actual.clone();
        wrong["outcome"]["data"]["originating_operation"] = json!("run_scenario");
        assert!(
            decoded(&read, wrong.clone()).is_err() || observations.observe(&read, &wrong).is_err()
        );
    }
}

#[test]
fn h09_recovers_only_original_task_without_submitting_any_work() {
    let read = request(
        "read_execution_view",
        "lookup",
        json!({"selection":{"kind":"request","originating_request_id":"original"}}),
    );
    let actual = response(
        &read,
        json!({"kind":"request","data":{"host_context_session_id":"host","state":"registered",
        "operation":"start_execution_session","task":handle("execution_preparation"),"originating_request_id":"original"}}),
    );
    decoded(&read, actual.clone()).unwrap();
    let mut observations = ExecutionObservations::default();
    observations.observe(&read, &actual).unwrap();
    let mut wrong = actual.clone();
    wrong["outcome"]["data"]["data"]["host_context_session_id"] = json!("other-host");
    assert!(decoded(&read, wrong).is_err());
    let mut wrong = actual.clone();
    wrong["outcome"]["data"]["data"]["originating_request_id"] = json!("other-request");
    assert!(decoded(&read, wrong).is_err());
}

#[test]
fn snapshot_control_and_pages_bind_original_identity_revision_and_time() {
    let identity = json!({"session_id":"execution", "execution_epoch_id":"epoch", "project_id":"Task8-two-device-six-units", "host_context_session_id":"host"});
    let read = request(
        "get_execution_snapshot",
        "exact",
        json!({"identity":identity,"selection":{"kind":"revision","execution_revision":8}}),
    );
    let mut reply = response(
        &read,
        json!({"snapshot":{"identity":identity,"execution_revision":8}}),
    );
    validate_reply(&read, &reply).unwrap();
    reply["outcome"]["data"]["snapshot"]["execution_revision"] = json!(9);
    assert!(validate_reply(&read, &reply).is_err());
    let read = request(
        "read_execution_view",
        "page",
        json!({"selection":{"kind":"observations","query":{
        "identity":identity,"execution_revision":8,"at_host_ms":20001}}}),
    );
    let reply = response(
        &read,
        json!({"kind":"observations","data":{"identity":identity,"execution_revision":8,"at_host_ms":20001,"observation_frontier":12}}),
    );
    validate_reply(&read, &reply).unwrap();
    for (key, other) in [
        ("execution_revision", json!(9)),
        ("at_host_ms", json!(20002)),
        ("identity", json!({"session_id":"other"})),
    ] {
        let mut wrong = reply.clone();
        wrong["outcome"]["data"]["data"][key] = other;
        assert!(validate_reply(&read, &wrong).is_err());
    }
    let control = request(
        "control_execution_session",
        "control",
        json!({"identity":identity,"action":{"kind":"pause_dispatch"},"expected_execution_revision":8}),
    );
    let ack = response(
        &control,
        json!({"identity":identity,"execution_revision":9,"action":{"kind":"run"},"admission":"applied"}),
    );
    assert!(decoded(&control, ack).is_err());
}

fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn actual(
    host: &mut ProjectHost,
    session: &ObjectId,
    project: bool,
    op: &str,
    params: Value,
) -> Value {
    let mut req = request(op, &format!("real-{op}"), params);
    if !project {
        req["project_id"] = Value::Null;
        req["scope"] = json!({"kind":"workspace"});
    }
    host.dispatch(session, serde_json::from_value(req).unwrap())
}
fn ok(reply: Value) -> Value {
    assert_eq!(reply["outcome"]["status"], "ok", "{reply:#}");
    reply["outcome"]["data"].clone()
}

#[test]
fn actual_saved_r4_host_grants_catalog_configuration_and_denies_forged_exports() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut host = ProjectHost::new(id("host"), id("ui"));
    let ui = id("ui");
    let caller = id("caller");
    let selection = host
        .select_directory(
            &ui,
            FilePurpose::ImportPackage,
            root.join("resources/v2-demo/geometry-v4/project"),
        )
        .unwrap();
    let handle = ok(actual(
        &mut host,
        &ui,
        false,
        "import_package",
        json!({"selected_file_ref":selection,"import_policy":"open_project","base_revision":null}),
    ));
    for work in host.drain_io_work() {
        host.finish_io(work.run());
    }
    let state = ok(actual(
        &mut host,
        &ui,
        false,
        "get_task",
        json!({"task_id":handle["id"]}),
    ));
    assert_eq!(state["task"]["state"], "completed");
    let imported = &state["result"]["data"];
    ok(actual(
        &mut host,
        &ui,
        false,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{
        "task_id":handle["id"],"choices":{"chosen_policy":"open_project","objects_to_create":imported["proposed_objects"],
        "module_versions":imported["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}),
    ));
    host.register_external_session(HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        session_id: caller.clone(),
        granted_project: Some("Task8-two-device-six-units".to_owned().try_into().unwrap()),
        allowed_operations: OPS
            .into_iter()
            .chain(PRIVATE)
            .chain(["describe_capabilities", "get_context", "get_task"])
            .map(|x| serde_json::from_value(json!(x)).unwrap())
            .collect(),
        allowed_definition_drafts: vec![],
        allow_view_control: false,
        selected_file_tokens: vec![],
        edit_grants: vec![],
    })
    .unwrap();
    let catalog: CapabilityCatalog = serde_json::from_value(ok(actual(
        &mut host,
        &caller,
        false,
        "describe_capabilities",
        json!({}),
    )))
    .unwrap();
    let tools = catalog::tools(&catalog).unwrap();
    let names = tools.iter().map(|t| t.name.as_ref()).collect::<Vec<_>>();
    for op in PRIVATE {
        assert!(!names.contains(&format!("superfactory_{op}").as_str()));
    }
    assert!(!names.contains(&"superfactory_start_execution_session")); // No actual preparation runtime installed.
    let denied_gateway = actual(
        &mut host,
        &caller,
        true,
        "get_execution_cloud_work",
        json!({"execution_contract_version":"sf.execution/2.0-draft11",
        "identity":{"session_id":"missing","execution_epoch_id":"missing","project_id":"Task8-two-device-six-units","host_context_session_id":"host"}}),
    );
    assert_eq!(
        denied_gateway["outcome"]["data"]["code"],
        "PermissionDenied"
    );
    let config = ok(actual(
        &mut host,
        &caller,
        true,
        "read_cloud_production",
        json!({"execution_contract_version":"sf.execution/2.0-draft11",
        "selection":{"kind":"configuration","base_revision":4,"scenario_id":"Task8-six-units","scenario_revision":4}}),
    ));
    for work in host.drain_io_work() {
        host.finish_io(work.run());
    }
    let state = ok(actual(
        &mut host,
        &caller,
        true,
        "get_task",
        json!({"task_id":config["id"]}),
    ));
    assert_eq!(state["originating_operation"], "read_cloud_production");
    assert_eq!(state["result"]["data"]["kind"], "configuration");
    assert_eq!(state["result"]["data"]["data"]["project_revision"], 4);
    assert!(
        !state["result"]["data"]["data"]["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let req = request(
        "export_execution_report",
        "forged",
        json!({"execution_contract_version":"sf.execution/2.0-draft11","revision":4,
        "sources":[{"read_origin":"host","identity":{"session_id":"fake","execution_epoch_id":"fake","project_id":"Task8-two-device-six-units","host_context_session_id":"host"},"execution_revision":0}],
        "formats":["pdf","html","csv","json"],"prediction":null,"selected_destination_ref":{"selection_id":"forged-selection","purpose":"export_execution_report"}}),
    );
    let denied = host.dispatch(&caller, serde_json::from_value(req.clone()).unwrap());
    assert_eq!(denied["outcome"]["status"], "error");
    assert!(
        matches!(
            denied["outcome"]["data"]["code"].as_str(),
            Some("PermissionDenied" | "UnsupportedCapability")
        ),
        "{denied}"
    );
    let mut wrong = req;
    wrong["params"]["selected_destination_ref"]["purpose"] = json!("export_cloud_data");
    assert!(serde_json::from_value::<ApiRequest>(wrong).is_err());
    let audit = root.join(".work/v2-implementation-20261026/agent-execution-consumer");
    std::fs::create_dir_all(&audit).unwrap();
    std::fs::write(audit.join("actual-host-r4.json"),serde_json::to_vec_pretty(&json!({"scope":"actual ProjectHost; no Native/cloud runtime",
        "catalog":serde_json::to_value(catalog).unwrap(),"configuration_task":state,"forged_export_rejection":denied,"gateway_rejection":denied_gateway})).unwrap()).unwrap();
}
