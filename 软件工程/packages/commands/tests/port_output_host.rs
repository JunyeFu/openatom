//! Actual Host private producer consumer; no Native or label renderer substitute.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;

fn p303() -> Value {
    let declared: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.input.json"
    ))
    .unwrap();
    let actual: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.bootstrap.actual.json"
    ))
    .unwrap();
    json!({"project":actual["snapshot"]["project"],"scenario":actual["snapshot"]["scenarios"][0],"definitions":declared["definitions"]})
}
fn p303_host(value: &Value) -> Harness {
    let tree = Sandbox::new();
    let directory = tree.directory("p303-seed");
    seed(&directory, value);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle = root.join("modules/devices/manual-assembly/presentation/p303-owned/phase");
    let package = read(&bundle.join("module.json"));
    let index = value["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|d| d["id"] == package["data"]["definition"]["id"])
        .unwrap();
    write(&directory.join(format!("module-{index}.json")), &package);
    let mut manifest = read(&directory.join("project.json"));
    manifest["data"]["assets"] = package["data"]["rights"].clone();
    for file in package["data"]["files"].as_array().unwrap() {
        let path = file["path"].as_str().unwrap();
        let target = directory.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(bundle.join(path), target).unwrap();
        let asset = package["data"]["assets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["path"] == path);
        manifest["data"]["documents"]
            .as_array_mut()
            .unwrap()
            .push(document(
                if asset.is_some() {
                    "asset"
                } else {
                    "supporting_file"
                },
                "asset",
                if asset.is_some() { "asset" } else { "module" },
                asset
                    .map(|a| a["id"].as_str().unwrap())
                    .unwrap_or(package["data"]["definition"]["id"].as_str().unwrap()),
                path,
                Value::Null,
            ));
    }
    write(&directory.join("project.json"), &manifest);
    Harness::from_seed(tree, &directory)
}
fn run(h: &mut Harness, scenario: &str) -> RunResult {
    let task = h.run(scenario);
    h.finish_one();
    h.completed_run(&task)
}
fn port(value: &Value, device: &str, port: &str) -> PortRef {
    let device = value["project"]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == device)
        .unwrap();
    serde_json::from_value(
        json!({"module_ref":device["module_ref"],"instance_id":device["id"],"port_id":port}),
    )
    .unwrap()
}
#[test]
fn actual_p303_port_done_75_and_transfer_85_are_distinct_readonly_counts() {
    let value = p303();
    let mut h = p303_host(&value);
    let result = run(&mut h, "P303-PHASE");
    let port = port(&value, "CNC", "output");
    let revision = h.context().project.unwrap().revision;
    let before = h.snapshot();
    let original = serde_json::to_value(&result).unwrap();
    let mut observations = vec![];
    for (at, done, transferred) in [(74999, 0, 0), (75000, 1, 0), (80000, 1, 0), (85000, 1, 1)] {
        let got = h
            .host
            .read_run_port_output(
                &h.ui,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &ApiScope::Project,
                &revision,
                &result.id,
                &SimTimeMs(at.try_into().unwrap()),
                &port,
            )
            .unwrap();
        let got = serde_json::to_value(got).unwrap();
        assert_eq!(got["completed_output"]["value"]["coefficient"], done);
        assert_eq!(got["accepted_output"]["value"]["coefficient"], transferred);
        assert_eq!(got["run_id"], json!(result.id));
        assert_eq!(got["port"], json!(port));
        assert_eq!(got["at_ms"], at);
        assert_eq!(got["completed_output"]["unit"], "piece");
        observations.push(got);
    }
    assert_eq!(h.snapshot(), before);
    assert_eq!(serde_json::to_value(h.full(&result.id)).unwrap(), original);
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/真实业务设计/runtime-port-output");
    write(
        &out.join("p303-host.actual.json"),
        &json!({"observations":observations,"Run":result,"readonly":true,"Host":true,"Native":false}),
    );
}

fn read_output(
    h: &Harness,
    run: &RunId,
    at: u64,
    port: &PortRef,
    revision: &Revision,
) -> Result<RunPortOutput, ApiError> {
    h.host.read_run_port_output(
        &h.ui,
        &ProjectId::try_from("P1".to_owned()).unwrap(),
        &ApiScope::Project,
        revision,
        run,
        &SimTimeMs(at.try_into().unwrap()),
        port,
    )
}
fn save(h: &mut Harness, name: &str) -> std::path::PathBuf {
    let path = h.tree.directory(name);
    let revision = h.context().project.unwrap().revision.0.value();
    let task = h.save(revision, &path);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    path
}

// Preserve an actual Save transaction for the subsequent desktop consumer.
// A unique generation avoids touching a previously opened/edited delivery.
fn preserve_native_input(saved: &std::path::Path, result: &RunResult, port: &PortRef) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let delivery = root.join("scenarios/真实业务设计/runtime-port-output/native-input");
    std::fs::create_dir_all(&delivery).unwrap();
    let delivery = delivery.canonicalize().unwrap();
    assert!(delivery.starts_with(&root));
    assert_ne!(delivery, root);
    let generation = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let staged = delivery.join(format!("saved-{generation}"));
    let moved = delivery.join(format!("project-{generation}"));
    std::fs::create_dir(&staged).unwrap();
    assert_eq!(
        staged.canonicalize().unwrap().parent(),
        Some(delivery.as_path())
    );
    copy(saved, &staged);
    std::fs::rename(&staged, &moved).unwrap();
    assert!(!staged.exists());
    let archive = read_project_archive(&moved).unwrap();
    assert_eq!(archive.project.id, result.input_ref.project_id);
    let tree = Sandbox::new();
    let mut fresh = Harness::from_seed(tree, &moved);
    let context = fresh.context();
    let revision = context.project.as_ref().unwrap().revision.clone();
    let job=result.jobs.iter().find(|job|matches!(&job.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id==&port.instance_id)).unwrap();
    let mut observations = vec![];
    let mut job_observations = vec![];
    for (at, done, transferred) in [(74999, 0, 0), (75000, 1, 0), (80000, 1, 0), (85000, 1, 1)] {
        let got = read_output(&fresh, &result.id, at, port, &revision).unwrap();
        assert_eq!(got.completed_output.value.coefficient.value(), done);
        assert_eq!(got.accepted_output.value.coefficient.value(), transferred);
        observations.push(serde_json::to_value(got).unwrap());
        let view:RunView=ok(fresh.call("get_run",json!({"run_id":result.id,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":[{"kind":"job","id":job.id}],"page":{"after_index":null,"limit":100}}}})));
        let RunView::Objects(page) = view else {
            panic!("actual Job history")
        };
        assert_eq!(page.objects.len(), 1);
        let DomainObject::Job(actual) = &page.objects[0] else {
            panic!("Job")
        };
        if at == 75000 {
            assert!(matches!(actual.state, JobStatus::WaitingOutput));
            assert!(actual.finished_at_ms.is_none());
            assert!(!actual.pending_outputs.is_empty());
        }
        if at == 85000 {
            assert!(matches!(actual.state, JobStatus::Completed));
            assert_eq!(actual.finished_at_ms.as_ref().unwrap().0.value(), 85000);
        }
        job_observations.push(json!({"at_ms":at,"job":actual}));
    }
    assert_eq!(
        serde_json::to_value(fresh.full(&result.id)).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    let ready: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.bootstrap.actual.json"
    ))
    .unwrap();
    let assets: Vec<AssetRef> = serde_json::from_value(ready["package"]["assets"].clone()).unwrap();
    assert_eq!(assets.len(), 4);
    let bundle = root.join("modules/devices/manual-assembly/presentation/p303-owned/phase");
    let source = ObjectSource::RunModuleVersion {
        run_id: result.id.clone(),
        at_ms: SimTimeMs(75000u64.try_into().unwrap()),
        module_ref: port.module_ref.clone(),
    };
    let mut asset_receipts = vec![];
    for asset in &assets {
        let actual = fresh
            .host
            .read_media_asset(
                &fresh.ui,
                &archive.project.id,
                &ApiScope::Project,
                &revision,
                &source,
                asset,
            )
            .unwrap();
        let original = std::fs::read(bundle.join(asset.path.as_str())).unwrap();
        assert_eq!(actual, original);
        asset_receipts.push(json!({"asset":asset,"actual_bytes":actual.len(),"same_as_original_bytes":true,"source":source}));
    }
    let history_entry = archive
        .manifest
        .documents
        .iter()
        .find(|entry| {
            matches!(entry.object_ref.kind, ObjectKind::Run)
                && entry.object_ref.id.as_str() == result.id.as_str()
                && entry.file_ref.path.as_str().ends_with("/item-history.json")
        })
        .unwrap();
    let history: Value =
        serde_json::from_slice(&archive.files[history_entry.file_ref.path.as_str()]).unwrap();
    assert!(
        history["data"]["job_records"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );
    assert!(
        history["data"]["port_outputs"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty())
    );
    let receipt = json!({
        "kind":"actual_Host_Save_move_freshHost_desktop_input_delivery",
        "producer":"existing port_output_host original P303 controlled input; actual same Host Run+Save, physical directory move, fresh Host OpenProject policy",
        "project_directory":moved,"project_relative_to_delivery":format!("project-{generation}"),"project_manifest":"project.json",
        "project_id":archive.project.id,"project_title":archive.project.title,"current_revision":revision,"scenario_id":result.input_ref.scenario_id,
        "run_id":result.id,"Job_id":job.id,"execution_ref":job.execution_ref,"port":port,"unit":"piece",
        "window":result.window,"computed_until_ms":result.computed_until_ms,"goal_met":result.goal_met,"orders":result.orders,
        "recommended_open_at_ms":75000,"counter_observations":observations,"Job_observations":job_observations,
        "assets":asset_receipts,"original_rights":ready["package"]["rights"],
        "history_document":history_entry.file_ref,"actual_Job_records":history["data"]["job_records"].as_array().unwrap().len(),"actual_port_records":history["data"]["port_outputs"].as_array().unwrap().len(),
        "actual_saved_file_count":archive.files.len(),"full_public_Run_unchanged":true,"fresh_Host_reads":true,"physical_directory_move":true,
        "all_files_under_project_root":true,"test_TempDir_required_for_retained_package":false,"Native_opened":false,"human_acceptance":null,"whole_release_passed":false
    });
    write(&delivery.join("current.json"), &receipt);
    write(
        &root.join("scenarios/真实业务设计/runtime-port-output/native-input-delivery.actual.json"),
        &receipt,
    );
    // One more full copy to a different location proves the retained directory,
    // not the initial seed/cache or former TempDir, is sufficient to open it.
    let copy_tree = Sandbox::new();
    let copy_path = copy_tree.directory("portable-copy");
    copy(&moved, &copy_path);
    let mut copied = Harness::from_seed(copy_tree, &copy_path);
    let copy_revision = copied.context().project.unwrap().revision;
    let got = read_output(&copied, &result.id, 75000, port, &copy_revision).unwrap();
    assert_eq!(got.completed_output.value.coefficient.value(), 1);
    assert_eq!(got.accepted_output.value.coefficient.value(), 0);
    assert_eq!(
        serde_json::to_value(copied.full(&result.id)).unwrap(),
        serde_json::to_value(result).unwrap()
    );
}

#[test]
fn actual_private_history_survives_save_move_fresh_host_and_old_missing_is_unavailable() {
    let value = p303();
    let mut h = p303_host(&value);
    let result = run(&mut h, "P303-PHASE");
    let port = port(&value, "CNC", "output");
    let revision = h.context().project.unwrap().revision;
    let expected: Vec<_> = [74999, 75000, 80000, 85000]
        .into_iter()
        .map(|at| {
            serde_json::to_value(read_output(&h, &result.id, at, &port, &revision).unwrap())
                .unwrap()
        })
        .collect();
    let saved = save(&mut h, "port-original");
    let tree = Sandbox::new();
    let moved = tree.directory("port-moved");
    copy(&saved, &moved);
    let mut fresh = Harness::from_seed(tree, &moved);
    let revision = fresh.context().project.unwrap().revision;
    for (at, want) in [74999, 75000, 80000, 85000].into_iter().zip(expected) {
        assert_eq!(
            serde_json::to_value(read_output(&fresh, &result.id, at, &port, &revision).unwrap())
                .unwrap(),
            want
        );
    }
    assert_eq!(
        serde_json::to_value(fresh.full(&result.id)).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    let legacy_tree = Sandbox::new();
    let legacy = legacy_tree.directory("legacy");
    copy(&saved, &legacy);
    let manifest = read(&legacy.join("project.json"));
    let path = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| {
            e["object_ref"]["id"] == json!(result.id)
                && e["file_ref"]["path"]
                    .as_str()
                    .is_some_and(|p| p.ends_with("/item-history.json"))
        })
        .unwrap()["file_ref"]["path"]
        .as_str()
        .unwrap();
    let path = legacy.join(path);
    let mut history = read(&path);
    let records = history["data"]
        .as_object_mut()
        .unwrap()
        .remove("port_outputs")
        .unwrap();
    assert!(records.as_array().unwrap().len() > 0);
    write(&path, &history);
    let mut old = Harness::from_seed(legacy_tree, &legacy);
    let revision = old.context().project.unwrap().revision;
    let missing = read_output(&old, &result.id, 80000, &port, &revision).unwrap_err();
    assert!(matches!(missing.code, ErrorCode::UnsupportedCapability));
    assert!(missing.message.contains("no actual port output history"));
    assert_eq!(
        serde_json::to_value(old.full(&result.id)).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    // The added private field is optional: original Item/Job history remains
    // usable, and no current or historical source is rerun to fill the gap.
    let view:RunView=ok(old.call("get_run",json!({"run_id":result.id,"query":{"kind":"objects_at","data":{"at_ms":80000,"objects":[{"kind":"job","id":result.jobs[0].id}],"page":{"after_index":null,"limit":100}}}})));
    assert!(matches!(view, RunView::Objects(_)));
    preserve_native_input(&saved, &result, &port);
}

#[test]
fn scope_revision_authority_frontier_and_exact_frozen_device_port_are_required() {
    let value: Value =
        serde_json::from_str(include_str!("fixtures/production/p1-phase-calendar.json")).unwrap();
    let mut h = Harness::from_value(value.clone());
    let result = run(&mut h, "PHASE");
    let port = port(&value, "CNC", "output");
    let revision = h.context().project.unwrap().revision;
    for at in [44999, 45000] {
        let got = read_output(&h, &result.id, at, &port, &revision).unwrap();
        assert_eq!(
            got.completed_output.value.coefficient.value(),
            u64::from(at == 45000)
        );
        assert_eq!(
            got.accepted_output.value.coefficient.value(),
            u64::from(at == 45000)
        );
    }
    assert!(matches!(
        read_output(&h, &result.id, 100001, &port, &revision)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    ));
    let mut wrong = port.clone();
    wrong.module_ref.version = "9.0.0".to_owned().try_into().unwrap();
    assert!(matches!(
        read_output(&h, &result.id, 45000, &wrong, &revision)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    wrong = port.clone();
    wrong.instance_id = id("w5.op.phased");
    assert!(matches!(
        read_output(&h, &result.id, 45000, &wrong, &revision)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    wrong = port.clone();
    wrong.port_id = "input".to_owned().try_into().unwrap();
    assert!(matches!(
        read_output(&h, &result.id, 45000, &wrong, &revision)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    ));
    let unknown = id("unknown-ui");
    assert!(matches!(
        h.host
            .read_run_port_output(
                &unknown,
                &"P1".to_owned().try_into().unwrap(),
                &ApiScope::Project,
                &revision,
                &result.id,
                &SimTimeMs(45000u64.try_into().unwrap()),
                &port
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        h.host
            .read_run_port_output(
                &h.ui,
                &"other-project".to_owned().try_into().unwrap(),
                &ApiScope::Project,
                &revision,
                &result.id,
                &SimTimeMs(45000u64.try_into().unwrap()),
                &port
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    let external = id("actual-agent");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: external.clone(),
            granted_project: Some("P1".to_owned().try_into().unwrap()),
            allowed_operations: vec![OperationName::GetRun],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    assert!(matches!(
        h.host
            .read_run_port_output(
                &external,
                &"P1".to_owned().try_into().unwrap(),
                &ApiScope::Project,
                &revision,
                &result.id,
                &SimTimeMs(45000u64.try_into().unwrap()),
                &port
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    let project = h.object("project", "P1");
    let _:EditCommit=ok(h.apply(1,json!([{"kind":"set_project_settings","data":{"title":"Edited current draft","time_reference":project["time_reference"],"generation_context":project["generation_context"],"sources":project["sources"]}}])));
    assert!(matches!(
        read_output(&h, &result.id, 45000, &port, &revision)
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    ));
    let current = h.context().project.unwrap().revision;
    assert_eq!(
        read_output(&h, &result.id, 45000, &port, &current)
            .unwrap()
            .completed_output
            .value
            .coefficient
            .value(),
        1
    );
    let definition = value["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "w5.op.phased")
        .unwrap()
        .clone();
    let created: EditCommit = ok(h.apply(
        current.0.value(),
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(identity) = created.scope else {
        panic!("draft")
    };
    assert!(matches!(
        h.host
            .read_run_port_output(
                &h.ui,
                &"P1".to_owned().try_into().unwrap(),
                &ApiScope::DefinitionDraft(identity.id),
                &revision,
                &result.id,
                &SimTimeMs(45000u64.try_into().unwrap()),
                &port
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
}

#[test]
fn same_physical_port_with_two_actual_item_types_cannot_be_silently_summed() {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/production/p1-phase-calendar.json")).unwrap();
    for calendar in value["project"]["calendars"].as_array_mut().unwrap() {
        calendar["intervals"] = json!([{"start_ms":0,"end_ms":100000}]);
    }
    let mut typ = value["project"]["item_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "part")
        .unwrap()
        .clone();
    typ["id"] = json!("alternate-part");
    value["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .push(typ);
    value["project"]["locations"][1]["accepted_item_types"]
        .as_array_mut()
        .unwrap()
        .push(json!("alternate-part"));
    value["project"]["locations"][1]["capacity"]["value"]["coefficient"] = json!(10);
    let mut route = value["project"]["routes"][0].clone();
    route["id"] = json!("alternate-route");
    route["steps"][0]["id"] = json!("alternate-step");
    route["steps"][0]["item_bindings"]["product"] = json!("alternate-part");
    value["project"]["routes"]
        .as_array_mut()
        .unwrap()
        .push(route);
    let mut task = value["scenario"]["production_tasks"][0].clone();
    task["id"] = json!("TASK-ALTERNATE");
    task["routing_ref"]["id"] = json!("alternate-route");
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .push(task);
    let mut raw = value["scenario"]["initial_conditions"]["items"][0].clone();
    raw["id"] = json!("raw-alternate");
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(raw);
    let mut connection = value["project"]["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["data"]["source"]["kind"] == "port")
        .unwrap()
        .clone();
    connection["data"]["id"] = json!("alternate-output");
    connection["data"]["item_type"] = json!("alternate-part");
    value["project"]["connections"]
        .as_array_mut()
        .unwrap()
        .push(connection);
    let mut h = Harness::from_value(value.clone());
    let result = run(&mut h, "PHASE");
    let port = port(&value, "CNC", "output");
    let revision = h.context().project.unwrap().revision;
    let incompatible = read_output(&h, &result.id, 100000, &port, &revision).unwrap_err();
    assert!(matches!(
        incompatible.code,
        ErrorCode::UnsupportedCapability
    ));
    assert!(
        incompatible
            .message
            .contains("incompatible actual item types")
    );
    assert_eq!(
        result
            .jobs
            .iter()
            .filter(|j| j.finished_at_ms.is_some())
            .count(),
        2
    );
}

#[test]
fn real_definition_cycle_retains_sample_read_but_instance_port_labels_are_not_applicable() {
    let mut h = Harness::new();
    let value = fixture();
    let definition = value["definitions"][0].clone();
    let created: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(identity) = created.scope else {
        panic!("draft")
    };
    let req:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"port-definition-cycle","project_id":"P1","scope":{"kind":"definition_draft","data":identity.id},"operation":"run_scenario","params":{"scenario_id":"LOCAL","input_revision":1,"observation_window":value["scenario"]["window"],"intent":{"kind":"definition_cycle","data":{"definition_revision":1,"test_input":value["scenario"]}}}})).unwrap();
    let handle: TaskHandle = ok(h.host.dispatch(&h.ui, req));
    h.finish_one();
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"port-definition-result","project_id":"P1","scope":{"kind":"definition_draft","data":identity.id},"operation":"get_task","params":{"task_id":handle.id}})).unwrap();
    let state: TaskState = ok(h.host.dispatch(&h.ui, request));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    let Some(TaskOutput::Run(summary)) = state.result else {
        panic!("Run")
    };
    let port = port(&value, "ASM-01", "product");
    let unavailable = h
        .host
        .read_run_port_output(
            &h.ui,
            &"P1".to_owned().try_into().unwrap(),
            &ApiScope::DefinitionDraft(identity.id.clone()),
            &identity.revision,
            &summary.run_id,
            &SimTimeMs(60000u64.try_into().unwrap()),
            &port,
        )
        .unwrap_err();
    assert!(matches!(unavailable.code, ErrorCode::UnsupportedCapability));
    assert!(
        unavailable
            .message
            .contains("no formal Project device instance port")
    );
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"port-definition-full","project_id":"P1","scope":{"kind":"definition_draft","data":identity.id},"operation":"get_run","params":{"run_id":summary.run_id,"query":{"kind":"full_result"}}})).unwrap();
    assert!(matches!(
        ok::<RunView>(h.host.dispatch(&h.ui, request)),
        RunView::Full(_)
    ));
}
