//! Real saved ProjectHost/Engine/files with the production decoder and MCP result
//! encoder. Publication acknowledgements are finite component inputs, not HTTP,
//! stdio/pipe IPC, Native UI or actual cloud/process acceptance.
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::{cloud_execution::*, *};
use sf_mcp_bridge::{decode::decode, execution_identity::ExecutionObservations};
use std::{path::PathBuf, time::Instant};

fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn request(op: &str, name: &str, params: Value, project: bool) -> Value {
    json!({"api_version":"0.1","request_id":name,"project_id":if project {json!("Task8-two-device-six-units")} else {Value::Null},
        "scope":{"kind":if project {"project"} else {"workspace"}},"operation":op,"params":params})
}
fn ok(reply: &Value) -> Value {
    assert_eq!(reply["outcome"]["status"], "ok", "{reply:#}");
    reply["outcome"]["data"].clone()
}
fn ui_call(host: &mut ProjectHost, req: Value) -> Value {
    ok(&host.dispatch(&id("ui"), serde_json::from_value(req).unwrap()))
}
struct Consumer {
    observations: ExecutionObservations,
    trace: Vec<Value>,
}
impl Consumer {
    fn call(&mut self, host: &mut ProjectHost, req: Value) -> Value {
        let reply = host.dispatch(&id("caller"), serde_json::from_value(req.clone()).unwrap());
        let decoded=decode(json!({"wire_version":"1","host_instance_id":"consumer-real-host","client_session_id":"caller",
            "originating_operation":req["operation"],"reply":reply,"managed_results":null,"managed_assets":null}),
            &req,&json!("consumer-real-host"),&json!("caller"),|_,_|panic!("No fabricated leases")).unwrap();
        self.observations.observe(&req, &decoded.original).unwrap();
        let original = decoded.original.clone();
        let mcp = decoded.mcp().unwrap();
        assert_eq!(mcp.is_error, Some(original["outcome"]["status"] == "error"));
        self.trace
            .push(json!({"request":req,"response":original,"mcp":mcp}));
        original
    }
}
fn acknowledge_component_publications(host: &mut ProjectHost, lease: &ExecutionGatewayLease) {
    for item in host.execution_gateway_cloud_work(lease).unwrap().items {
        match item {
            CloudWorkItem::PublishSnapshot {
                work_id,
                publication,
            } => {
                let ack = CloudEffectAck::SnapshotPublished {
                    work_id,
                    ack: PublishAck {
                        session_id: publication.snapshot.identity.session_id,
                        execution_epoch_id: publication.snapshot.identity.execution_epoch_id,
                        execution_revision: publication.snapshot.execution_revision,
                        publication_status: PublishAckPublicationStatus::Stored,
                        observation_frontier: publication.snapshot.observation_frontier,
                    },
                };
                host.execution_gateway_acknowledge(lease, ack, Instant::now())
                    .unwrap();
            }
            _ => panic!("No component command publication is needed for prepared readiness"),
        }
    }
}
fn setup() -> (
    ProjectHost,
    ExecutionGatewayLease,
    ExecutionIdentity,
    Consumer,
    PathBuf,
) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("resources/v2-demo/geometry-v4/project");
    let archive = read_project_archive(&source).unwrap();
    let mut host = ProjectHost::new(id("consumer-real-host"), id("ui"));
    let selected = host
        .select_directory(&id("ui"), FilePurpose::ImportPackage, source)
        .unwrap();
    let task = ui_call(
        &mut host,
        request(
            "import_package",
            "open",
            json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
            false,
        ),
    );
    for work in host.drain_io_work() {
        host.finish_io(work.run());
    }
    ui_call(
        &mut host,
        request(
            "apply_commands",
            "adopt",
            json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{
        "task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,
        "departure":"discard_after_tasks_terminal"}}}]}),
            false,
        ),
    );
    let manifest = serde_json::to_value(&archive.manifest).unwrap();
    let modules: Vec<ModulePackage> = manifest["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["role"] == "module_definition")
        .map(|d| {
            serde_json::from_slice::<Versioned<ModulePackage>>(
                &archive.files[d["file_ref"]["path"].as_str().unwrap()],
            )
            .unwrap()
            .data
        })
        .collect();
    let run: RunResult = manifest["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["role"] == "run_result")
        .map(|d| {
            serde_json::from_slice::<Versioned<RunResult>>(
                &archive.files[d["file_ref"]["path"].as_str().unwrap()],
            )
            .unwrap()
            .data
        })
        .find(|r| r.input_ref.project_revision.0.value() == 4)
        .unwrap();
    let scenario = archive
        .scenarios
        .values()
        .find(|s| s.revision.0.value() == 4)
        .unwrap();
    let bindings: Vec<CandidateBinding> = serde_json::from_slice(
        &std::fs::read(root.join("resources/v2-demo/geometry-v4/candidate-bindings.json")).unwrap(),
    )
    .unwrap();
    let identity: ExecutionIdentity = serde_json::from_value(
        json!({"session_id":"consumer-execution","execution_epoch_id":"consumer-epoch",
        "project_id":archive.project.id,"host_context_session_id":"consumer-real-host"}),
    )
    .unwrap();
    let frozen:ExecutionFrozenInputs=serde_json::from_value(json!({"document_kind":"execution_frozen_inputs","contract_version":EXECUTION_CONTRACT_VERSION,
        "identity":identity,"project_input_ref":run.input_ref,"project":archive.project,"scenario":scenario,"modules":modules,
        "execution_definitions":modules.iter().map(|m|m.definition.clone()).collect::<Vec<_>>(),"scope":scenario.scope,"candidate_bindings":bindings,"asset_bindings":[]})).unwrap();
    let policy=serde_json::from_value(json!({"clock_mode":"host_monotonic","report_cutoff_ms":120000,"stop_new_dispatch_on_cutoff":true,
        "finish_policy":"drain_admitted_then_finish","target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},
        "source_prediction_window":{"start_ms":0,"end_ms":scenario.window.end_ms},"observation_max_age_ms":2000})).unwrap();
    let lease = host
        .register_native_execution(
            &id("ui"),
            frozen,
            policy,
            "consumer-local-dataset".to_owned().try_into().unwrap(),
            id("component-connection"),
        )
        .unwrap();
    for device in ["L1-D01", "L1-D02"] {
        host.execution_gateway_register_boot(
            &lease,
            &id(device),
            format!("component-{device}").try_into().unwrap(),
            Instant::now(),
        )
        .unwrap();
    }
    acknowledge_component_publications(&mut host, &lease);
    host.finish_native_execution_preparation(&lease).unwrap();
    host.register_external_session(HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        session_id: id("caller"),
        granted_project: Some(identity.project_id.clone()),
        allowed_operations: vec![
            OperationName::DescribeCapabilities,
            OperationName::GetContext,
            OperationName::GetTask,
            OperationName::CancelTask,
            OperationName::ReadExecutionView,
            OperationName::GetExecutionSnapshot,
            OperationName::ControlExecutionSession,
            OperationName::ExportExecutionReport,
        ],
        allowed_definition_drafts: vec![],
        allow_view_control: false,
        selected_file_tokens: vec![],
        edit_grants: vec![],
    })
    .unwrap();
    (
        host,
        lease,
        identity,
        Consumer {
            observations: Default::default(),
            trace: vec![],
        },
        root,
    )
}

#[test]
fn real_host_read_control_export_and_authority_identity_idempotency_boundaries() {
    let (mut host, lease, identity, mut consumer, root) = setup();
    let read = |name: &str| {
        request(
            "get_execution_snapshot",
            name,
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,
        "identity":identity,"selection":{"kind":"latest"}}),
            true,
        )
    };
    let initial = ok(&consumer.call(&mut host, read("initial")));
    let revision = initial["snapshot"]["execution_revision"].clone();
    let run = request(
        "control_execution_session",
        "run-once",
        json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,
        "identity":identity,"expected_execution_revision":revision,"action":{"kind":"run"}}),
        true,
    );
    let before = serde_json::to_value(host.execution_gateway_snapshot(&lease).unwrap()).unwrap();
    let denied = consumer.call(&mut host, run.clone());
    assert_eq!(denied["outcome"]["data"]["code"], "PermissionDenied");
    assert_eq!(
        before,
        serde_json::to_value(host.execution_gateway_snapshot(&lease).unwrap()).unwrap()
    );
    host.grant_execution_controls(
        &id("ui"),
        &id("caller"),
        &identity,
        ExecutionControlGrant {
            session_controls: vec![
                ExecutionReadAccessAllowedSessionControlsItem::Run,
                ExecutionReadAccessAllowedSessionControlsItem::PauseDispatch,
                ExecutionReadAccessAllowedSessionControlsItem::StopDispatch,
            ],
            device_acceptance_control_ids: vec![],
        },
    )
    .unwrap();
    let first = ok(&consumer.call(&mut host, run.clone()));
    let replay = ok(&consumer.call(&mut host, run));
    assert_eq!(
        first, replay,
        "Identical control request must return original acknowledgement"
    );
    let controlled =
        serde_json::to_value(host.execution_gateway_snapshot(&lease).unwrap()).unwrap();
    for kind in ["epoch", "project"] {
        let mut wrong = read(&format!("foreign-{kind}"));
        if kind == "epoch" {
            wrong["params"]["identity"]["execution_epoch_id"] = json!("foreign-epoch");
        } else {
            wrong["project_id"] = json!("foreign-project");
        }
        assert_eq!(
            consumer.call(&mut host, wrong)["outcome"]["status"],
            "error"
        );
        assert_eq!(
            controlled,
            serde_json::to_value(host.execution_gateway_snapshot(&lease).unwrap()).unwrap()
        );
    }
    let current = ok(&consumer.call(&mut host, read("after-control")));
    let out = root
        .join(".work/v2-agent-consumer/exports")
        .join(format!("actual-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap();
    let selection = host
        .select_directory(
            &id("caller"),
            FilePurpose::ExportExecutionReport,
            out.clone(),
        )
        .unwrap();
    let export = request(
        "export_execution_report",
        "export-once",
        json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"revision":4,
        "sources":[{"read_origin":"host","identity":identity,"execution_revision":current["snapshot"]["execution_revision"]}],
        "formats":["json","csv"],"prediction":null,"selected_destination_ref":selection}),
        true,
    );
    let admitted = ok(&consumer.call(&mut host, export.clone()));
    assert_eq!(
        admitted["id"],
        ok(&consumer.call(&mut host, export.clone()))["id"]
    );
    let recovered = ok(&consumer.call(
        &mut host,
        request(
            "read_execution_view",
            "recover-export",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,
        "selection":{"kind":"request","originating_request_id":"export-once"}}),
            true,
        ),
    ));
    assert_eq!(recovered["data"]["task"]["id"], admitted["id"]);
    let jobs = host.drain_io_work();
    assert_eq!(
        jobs.len(),
        1,
        "Duplicate export must not enqueue a second writer"
    );
    for work in jobs {
        host.finish_io(work.run());
    }
    let completed = ok(&consumer.call(
        &mut host,
        request(
            "get_task",
            "read-export",
            json!({"task_id":admitted["id"]}),
            true,
        ),
    ));
    assert_eq!(completed["task"]["state"], "completed", "{completed}");
    let result = &completed["result"]["data"];
    let file = out.join(result["report_file"]["path"].as_str().unwrap());
    let document: ExecutionReport = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(document.project_id, identity.project_id);
    assert!(host.drain_io_work().is_empty());
    let mut changed = export;
    changed["params"]["formats"] = json!(["json"]);
    assert_eq!(
        consumer.call(&mut host, changed)["outcome"]["data"]["code"],
        "InvalidInput"
    );
    let before_revoke =
        serde_json::to_value(host.execution_gateway_snapshot(&lease).unwrap()).unwrap();
    host.revoke_external_session(&id("caller")).unwrap();
    let rejected = host.dispatch(
        &id("caller"),
        serde_json::from_value(read("revoked")).unwrap(),
    );
    assert_eq!(rejected["outcome"]["data"]["code"], "PermissionDenied");
    assert_eq!(
        before_revoke,
        serde_json::to_value(host.execution_gateway_snapshot(&lease).unwrap()).unwrap()
    );
    std::fs::create_dir_all(root.join(".work/v2-agent-consumer")).unwrap();
    std::fs::write(root.join(".work/v2-agent-consumer/actual-host-workflow.json"),serde_json::to_vec_pretty(&json!({
        "scope":"Real saved ProjectHost/Engine/files + production MCP decoder/result encoder; no pipe/stdIO, Native or HTTP",
        "readiness_boundary":"Finite component boot/publication acknowledgement inputs; not actual cloud results",
        "trace":consumer.trace,"export_directory":out,"revoked_response":rejected})).unwrap()).unwrap();
}
