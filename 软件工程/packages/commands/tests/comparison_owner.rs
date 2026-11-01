//! Actual Host/sole Engine/File worker consumers for all three P2-03 steps.
#[path = "fixtures/comparison/professional.rs"]
mod professional_fixture;
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{fs, path::Path};
use support::*;

fn complete(h: &mut Harness, scenario: &str) -> RunResult {
    let task = h.run(scenario);
    h.finish_one();
    h.completed_run(&task)
}
fn add_pcb_arrival(h: &mut Harness, scenario: &str, at: u64) {
    let mut s = h.object("scenario", "LOCAL");
    s["id"] = json!(scenario);
    s["revision"] = json!(1);
    s["project_revision"] = json!(h.context().project.unwrap().revision);
    let mut pcb = s["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("pcb-002");
    s["arrivals"] = json!([{"id":"PCB-ARRIVAL","at_ms":at,"destination":"pcbs-A","items":[pcb],"release_policy":"unchecked_allowed"}]);
    let revision = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(revision, json!([put("scenario", s)])));
}
fn comparison(h: &mut Harness, a: &RunId, b: &RunId) -> RunComparison {
    ok(h.call("compare_runs",json!({"run_ids":[a,b],"metric_scope":{"keys":["accepted_quantity","completion_time","waiting_time","unit_cost"],"objects":[],"acceptance":[]}})))
}
fn export(
    h: &mut Harness,
    path: &Path,
    runs: &[RunId],
    scenarios: &[&str],
    file: bool,
) -> TaskHandle {
    let selected = if file {
        h.host
            .select_file(&h.ui, FilePurpose::ExportPackage, path.to_owned())
            .unwrap()
    } else {
        h.host
            .select_directory(&h.ui, FilePurpose::ExportPackage, path.to_owned())
            .unwrap()
    };
    let revision = h.context().project.unwrap().revision;
    ok(h.call("export_package",json!({"revision":revision,"package_scope":{"purpose":"handoff_project","scenarios":scenarios,"run_ids":runs,"artifact_ids":[],"module_refs":[],"include_redistributable_assets":true},"selected_destination_ref":selected})))
}
fn exported(h: &mut Harness, task: &TaskHandle) -> ExportPackage {
    h.finish_one();
    let state = h.task(task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    match state.result.unwrap() {
        TaskOutput::Package(package) => package,
        other => panic!("not package {other:?}"),
    }
}
fn import_zip(h: &mut Harness, path: &Path) -> TaskHandle {
    let selected = h
        .host
        .select_file(&h.ui, FilePurpose::ImportPackage, path.to_owned())
        .unwrap();
    ok(h.call_as(
        &h.ui.clone(),
        false,
        "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
    ))
}
fn frozen_inputs(directory: &Path) -> Vec<Value> {
    let root = read(&directory.join("project.json"))["data"].clone();
    let entries = root["documents"].as_array().unwrap();
    let definitions: Vec<_> = entries
        .iter()
        .filter(|e| e["file_ref"]["kind"] == "module")
        .map(|e| {
            read(&directory.join(e["file_ref"]["path"].as_str().unwrap()))["data"]["definition"]
                .clone()
        })
        .collect();
    entries
        .iter()
        .filter(|e| e["file_ref"]["kind"] == "run_result")
        .map(|entry| {
            let run =
                read(&directory.join(entry["file_ref"]["path"].as_str().unwrap()))["data"].clone();
            let p = read(
                &directory.join(
                    run["input_ref"]["frozen_project_file"]["path"]
                        .as_str()
                        .unwrap(),
                ),
            )["data"]
                .clone();
            let s = read(
                &directory.join(
                    run["input_ref"]["frozen_scenario_file"]["path"]
                        .as_str()
                        .unwrap(),
                ),
            )["data"]
                .clone();
            json!([p, s, definitions, run])
        })
        .collect()
}

#[test]
fn actual_comparison_directory_zip_reopen_edit_and_initial_rerun() {
    let mut h = Harness::new();
    let a = complete(&mut h, "LOCAL");
    let a_bytes = serde_json::to_vec(&a).unwrap();
    assert_eq!(
        a.acceptance_events
            .iter()
            .map(|e| e.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000]
    );
    add_pcb_arrival(&mut h, "WITH-PCB", 90000);
    let b = complete(&mut h, "WITH-PCB");
    assert_ne!(a.id, b.id);
    assert_eq!(
        b.acceptance_events
            .iter()
            .map(|e| e.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 165000]
    );
    let compared = comparison(&mut h, &a.id, &b.id);
    assert!(matches!(
        compared.pairs[0].status,
        ComparisonStatus::ControlledChanges
    ));
    assert!(
        compared.pairs[0]
            .input_changes
            .iter()
            .any(|c| c.path.contains("arrivals"))
    );
    let accepted = compared
        .metrics
        .iter()
        .find(|m| matches!(m.metric, MetricKey::AcceptedQuantity))
        .unwrap();
    assert_eq!(
        serde_json::to_value(&accepted.values[0].value).unwrap(),
        json!({"kind":"quantity","data":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}})
    );
    assert_eq!(
        serde_json::to_value(&accepted.values[1].value).unwrap(),
        json!({"kind":"quantity","data":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"}})
    );
    assert_eq!(
        accepted.pair_differences[0]
            .change_percent
            .as_ref()
            .unwrap()
            .value(),
        100.0
    );
    let time = compared
        .metrics
        .iter()
        .find(|m| matches!(m.metric, MetricKey::CompletionTime))
        .unwrap();
    assert!(matches!(time.values[0].value, MetricValue::Unknown(_)));
    assert_eq!(
        serde_json::to_value(&time.values[1].value).unwrap(),
        json!({"kind":"duration","data":165000})
    );
    assert!(!time.pair_differences[0].comparable);
    let directory = h.tree.directory("exported");
    let task = export(
        &mut h,
        &directory,
        &[a.id.clone(), b.id.clone()],
        &["LOCAL", "WITH-PCB"],
        false,
    );
    let package = exported(&mut h, &task);
    assert!(package.missing.is_empty(), "{:?}", package.missing);
    assert!(!package.included_files.is_empty());
    let mut inputs = frozen_inputs(&directory);
    inputs.sort_by_key(|i| i[1]["id"].as_str().unwrap().to_owned());
    assert_eq!(inputs.len(), 2);
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/方案比较/runtime");
    fs::create_dir_all(&output).unwrap();
    write(&output.join("actual-frozen-inputs.json"), &json!(inputs));
    write(
        &output.join("actual-comparison.json"),
        &serde_json::to_value(&compared).unwrap(),
    );
    let zip = h.tree.path.join("handoff.sfproj");
    let task = export(
        &mut h,
        &zip,
        &[a.id.clone(), b.id.clone()],
        &["LOCAL", "WITH-PCB"],
        true,
    );
    exported(&mut h, &task);
    let moved = h.tree.directory("another-directory");
    let moved_zip = moved.join("renamed.sfproj");
    fs::copy(&zip, &moved_zip).unwrap();
    let task = import_zip(&mut h, &moved_zip);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    assert_eq!(serde_json::to_vec(&h.full(&a.id)).unwrap(), a_bytes);
    let reopened = comparison(&mut h, &a.id, &b.id);
    assert_eq!(
        serde_json::to_value(reopened).unwrap(),
        serde_json::to_value(compared).unwrap()
    );
    let mut modified = h.object("scenario", "WITH-PCB");
    modified["arrivals"][0]["at_ms"] = json!(120000);
    let revision = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(revision, json!([put("scenario", modified)])));
    let c = complete(&mut h, "WITH-PCB");
    assert_ne!(c.id, b.id);
    assert_ne!(c.id, a.id);
    assert_eq!(
        c.acceptance_events
            .iter()
            .map(|e| e.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 195000]
    );
    assert_eq!(serde_json::to_vec(&h.full(&a.id)).unwrap(), a_bytes);
    assert_eq!(h.full(&b.id).acceptance_events[1].at_ms.0.value(), 165000);
    let changed = h.tree.directory("changed");
    let revision = h.context().project.unwrap().revision.0.value();
    let task = h.save(revision, &changed);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let task = h.import(&changed);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    assert_eq!(
        h.object("scenario", "WITH-PCB")["arrivals"][0]["at_ms"],
        120000
    );
    assert_eq!(h.full(&b.id).acceptance_events[1].at_ms.0.value(), 165000);
    assert_eq!(h.full(&c.id).acceptance_events[1].at_ms.0.value(), 195000);
}

#[test]
fn design_only_and_selected_history_have_distinct_capabilities() {
    let mut h = Harness::new();
    let a = complete(&mut h, "LOCAL");
    add_pcb_arrival(&mut h, "WITH-PCB", 90000);
    let b = complete(&mut h, "WITH-PCB");
    let design = h.tree.directory("design");
    let task = export(&mut h, &design, &[], &["LOCAL"], false);
    exported(&mut h, &task);
    let root = read(&design.join("project.json"))["data"].clone();
    assert_eq!(root["restart_policy"], "restart_from_initial_input");
    assert!(
        !root["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["file_ref"]["kind"] == "run_result")
    );
    let selected = h.tree.directory("only-a");
    let task = export(
        &mut h,
        &selected,
        std::slice::from_ref(&a.id),
        &["LOCAL"],
        false,
    );
    exported(&mut h, &task);
    let inputs = frozen_inputs(&selected);
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0][3]["id"], json!(a.id));
    let task = h.import(&selected);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    assert_eq!(h.full(&a.id).acceptance_events[0].at_ms.0.value(), 75000);
    assert!(matches!(
        error(h.call(
            "get_run",
            json!({"run_id":b.id,"query":{"kind":"full_result"}})
        ))
        .code,
        ErrorCode::MissingReference
    ));
    let fresh = complete(&mut h, "LOCAL");
    assert_ne!(fresh.id, a.id);
    assert_eq!(
        fresh
            .acceptance_events
            .iter()
            .map(|e| e.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000]
    );
}

#[test]
fn unsafe_zip_is_rejected_without_replacing_active_project() {
    let mut h = Harness::new();
    let before = h.snapshot();
    for (index, name) in [
        "../outside.json",
        "C:/outside.json",
        "AUX.json",
        "assets./image.png",
    ]
    .iter()
    .enumerate()
    {
        let path = h.tree.path.join(format!("unsafe-{index}.sfproj"));
        let mut zip = zip::ZipWriter::new(fs::File::create(&path).unwrap());
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        use std::io::Write;
        zip.write_all(b"not an executable member").unwrap();
        zip.finish().unwrap();
        let task = import_zip(&mut h, &path);
        h.finish_one();
        let state = h.task(&task, false);
        assert!(
            matches!(state.task.state, ComputationStatus::Failed),
            "{state:?}"
        );
        assert_eq!(before["project"], h.snapshot()["project"]);
        assert!(!h.tree.path.join("outside.json").exists());
    }
}

#[test]
fn exact_old_module_version_is_preserved_even_when_newer_source_is_available() {
    let mut value = fixture();
    let mut newer = value["definitions"][0].clone();
    newer["version"] = json!("9.0.0");
    newer["title"] = json!("new version must not replace exact old version");
    value["definitions"].as_array_mut().unwrap().push(newer);
    let tree = Sandbox::new();
    let seed_dir = tree.directory("old-and-new-modules");
    seed(&seed_dir, &value);
    let mut manifest = read(&seed_dir.join("project.json"));
    let newer = &value["definitions"][value["definitions"].as_array().unwrap().len() - 1];
    manifest["data"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":newer["id"],"version":newer["version"]}));
    write(&seed_dir.join("project.json"), &manifest);
    let mut h = Harness::from_seed(tree, &seed_dir);
    let before = complete(&mut h, "LOCAL");
    let directory = h.tree.directory("exact-version");
    let task = export(&mut h, &directory, &[], &["LOCAL"], false);
    exported(&mut h, &task);
    let root = read(&directory.join("project.json"))["data"].clone();
    assert!(
        !root["module_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["version"] == "9.0.0")
    );
    let task = h.import(&directory);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    let after = complete(&mut h, "LOCAL");
    assert_eq!(
        before.acceptance_events[0].at_ms.0.value(),
        after.acceptance_events[0].at_ms.0.value()
    );
    assert!(
        after
            .jobs
            .iter()
            .all(|j| String::from(j.operation_ref.version.clone()) != "9.0.0")
    );
}

#[test]
fn imported_asset_permission_label_does_not_grant_handoff_but_current_ui_confirmation_does() {
    let tree = Sandbox::new();
    let seed_dir = tree.directory("asset-source");
    seed(&seed_dir, &fixture());
    let right = json!({"asset_ref":"ASSET-OWN","source":"self-created vector illustration","license":"CC0-1.0","redistribution_confirmed":true,"missing_reason":null});
    let mut module = read(&seed_dir.join("module-0.json"));
    module["data"]["definition"]["contribution"]["assets"] = json!([right.clone()]);
    module["data"]["assets"] = json!([{"id":"ASSET-OWN","path":"assets/own.svg","kind":"svg"}]);
    module["data"]["rights"] = json!([right.clone()]);
    write(&seed_dir.join("module-0.json"), &module);
    fs::create_dir(seed_dir.join("assets")).unwrap();
    let svg=b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"12\" height=\"12\"><rect width=\"12\" height=\"12\"/></svg>";
    fs::write(seed_dir.join("assets/own.svg"), svg).unwrap();
    let mut root = read(&seed_dir.join("project.json"));
    root["data"]["assets"] = json!([right]);
    root["data"]["documents"]
        .as_array_mut()
        .unwrap()
        .push(document(
            "asset",
            "asset",
            "asset",
            "ASSET-OWN",
            "assets/own.svg",
            Value::Null,
        ));
    write(&seed_dir.join("project.json"), &root);
    let mut h = Harness::from_seed(tree, &seed_dir);
    let denied = h.tree.directory("permission-not-granted");
    let task = export(&mut h, &denied, &[], &["LOCAL"], false);
    let result = exported(&mut h, &task);
    assert!(!denied.join("assets/own.svg").exists());
    assert!(result.missing.iter().any(|i| {
        i.code == "handoff.asset_not_redistributed"
            && i.blocks
                .iter()
                .any(|b| matches!(b, CapabilityPurpose::Preview))
            && !i
                .blocks
                .iter()
                .any(|b| matches!(b, CapabilityPurpose::Execution))
    }));
    h.host
        .confirm_asset_redistribution(&h.ui, &AssetId::try_from("ASSET-OWN".to_owned()).unwrap())
        .unwrap();
    let approved = h.tree.directory("permission-granted");
    let task = export(&mut h, &approved, &[], &["LOCAL"], false);
    let result = exported(&mut h, &task);
    assert!(result.missing.is_empty(), "{:?}", result.missing);
    let manifest = read(&approved.join("project.json"));
    let asset = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["object_ref"]["kind"] == "asset" && e["object_ref"]["id"] == "ASSET-OWN")
        .unwrap();
    assert_eq!(
        fs::read(approved.join(asset["file_ref"]["path"].as_str().unwrap())).unwrap(),
        svg
    );
}

#[test]
fn cancellation_preserves_previous_zip_and_missing_selected_run_fails_before_output() {
    let mut h = Harness::new();
    let zip = h.tree.path.join("existing.sfproj");
    let task = export(&mut h, &zip, &[], &["LOCAL"], true);
    exported(&mut h, &task);
    let old = fs::read(&zip).unwrap();
    let task = export(&mut h, &zip, &[], &["LOCAL"], true);
    let _: CancellationState =
        ok(h.call("cancel_task", json!({"task_id":task.id,"action":"cancel"})));
    h.finish_one();
    let state = h.task(&task, true);
    assert!(matches!(state.task.state, ComputationStatus::Cancelled));
    assert_eq!(fs::read(&zip).unwrap(), old);
    let missing = RunId::try_from("absent-run".to_owned()).unwrap();
    let task = export(&mut h, &zip, &[missing], &["LOCAL"], true);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Failed
    ));
    assert_eq!(fs::read(&zip).unwrap(), old);
}

#[cfg(windows)]
#[test]
fn actual_windows_locked_zip_commit_preserves_previous_package_and_current_edits() {
    use std::os::windows::fs::OpenOptionsExt;
    let mut h = Harness::new();
    let zip = h.tree.path.join("locked.sfproj");
    let task = export(&mut h, &zip, &[], &["LOCAL"], true);
    exported(&mut h, &task);
    let previous = fs::read(&zip).unwrap();
    add_pcb_arrival(&mut h, "UNSAVED-ARRIVAL", 120000);
    let before = h.object("scenario", "UNSAVED-ARRIVAL");
    let revision = h.context().project.unwrap().revision;
    assert!(h.context().project.unwrap().dirty);
    // A real Windows handle permits reads while denying replacement/deletion.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&zip)
        .unwrap();
    let task = export(&mut h, &zip, &[], &["LOCAL", "UNSAVED-ARRIVAL"], true);
    h.finish_one();
    let state = h.task(&task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Failed),
        "{state:?}"
    );
    assert!(state.result.is_none());
    assert!(
        matches!(
            state.failure.as_ref().map(|e| &e.code),
            Some(ErrorCode::IoFailure)
        ),
        "{state:?}"
    );
    assert_eq!(fs::read(&zip).unwrap(), previous);
    assert_eq!(h.object("scenario", "UNSAVED-ARRIVAL"), before);
    assert_eq!(h.context().project.unwrap().revision.0, revision.0);
    assert!(h.context().project.unwrap().dirty);
    drop(held);
    let task = import_zip(&mut h, &zip);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, true));
    let restored = h.object("project", "P1");
    assert!(
        !restored["scenario_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s == "UNSAVED-ARRIVAL")
    );
    assert_eq!(h.object("scenario", "LOCAL")["arrivals"], json!([]));
}

#[test]
fn professional_jobs_results_acknowledgments_and_bindings_survive_zip_without_promoting_claims() {
    let mut h = professional_fixture::professional(true);
    let before = h.object("project", "P1");
    let artifact = h.object("artifact", "ART-U");
    let zip = h.tree.path.join("professional.sfproj");
    let task = export(&mut h, &zip, &[], &["LOCAL"], true);
    exported(&mut h, &task);
    let target = h.tree.directory("professional-recipient");
    let copy = target.join("professional.sfproj");
    fs::copy(&zip, &copy).unwrap();
    let task = import_zip(&mut h, &copy);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, true));
    let after = h.object("project", "P1");
    assert_eq!(before["bindings"], after["bindings"]);
    assert_eq!(before["acknowledgments"], after["acknowledgments"]);
    assert_eq!(h.object("artifact", "ART-U"), artifact);
    assert!(
        !after["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == "ART-F")
    );
    assert!(
        after["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == "ART-OTHER")
    );
    let directory = h.tree.directory("professional-reopened-save");
    let revision = h.context().project.unwrap().revision.0.value();
    let task = h.save(revision, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let manifest = read(&directory.join("project.json"));
    let documents = manifest["data"]["documents"].as_array().unwrap();
    assert!(
        documents
            .iter()
            .any(|e| e["file_ref"]["kind"] == "packaging_job" && e["object_ref"]["id"] == "JOB-U")
    );
    assert!(
        documents.iter().any(
            |e| e["file_ref"]["kind"] == "packaging_result" && e["object_ref"]["id"] == "ART-U"
        )
    );
    assert!(
        !documents
            .iter()
            .any(|e| e["file_ref"]["kind"] == "run_result")
    );
}

#[test]
fn one_initial_pcb_change_is_controlled_but_never_marked_strict_initial_equality() {
    let mut h = Harness::new();
    let a = complete(&mut h, "LOCAL");
    let mut scenario = h.object("scenario", "LOCAL");
    scenario["id"] = json!("INITIAL-PCB");
    scenario["revision"] = json!(1);
    let mut pcb = scenario["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("pcb-002");
    scenario["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(pcb);
    let rev = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(rev, json!([put("scenario", scenario)])));
    let b = complete(&mut h, "INITIAL-PCB");
    assert_eq!(a.acceptance_events.len(), 1);
    assert_eq!(b.acceptance_events.len(), 2);
    let c = comparison(&mut h, &a.id, &b.id);
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::ControlledChanges
    ));
    assert!(
        c.pairs[0]
            .conditions
            .iter()
            .any(|c| matches!(c.key, ComparisonConditionKey::InitialConditions) && !c.matches)
    );
    assert!(
        c.pairs[0]
            .input_changes
            .iter()
            .any(|c| c.path.contains("initial_conditions/items/pcb-002")
                && matches!(c.kind, InputChangeKind::Added))
    );
}
#[test]
fn missing_behavior_roundtrips_as_an_explicit_non_executable_draft() {
    let mut value = fixture();
    let missing_ref = value["project"]["routes"][0]["steps"][0]["operation_ref"].clone();
    value["definitions"]
        .as_array_mut()
        .unwrap()
        .retain(|d| d["id"] != missing_ref["id"] || d["version"] != missing_ref["version"]);
    let issue = json!({"code":"business_inputs.missing_module","object_ref":{"kind":"route","id":value["project"]["routes"][0]["id"]},"field_path":"/steps/0/operation_ref","message":"Required exact behavior definition absent; this is a layout-only draft","blocks":["execution","complete_conclusion"]});
    value["project"]["issues"]
        .as_array_mut()
        .unwrap()
        .push(issue);
    let tree = Sandbox::new();
    let seed_dir = tree.directory("missing-behavior-source");
    seed(&seed_dir, &value);
    let ui = id("missing-behavior-ui");
    let mut h = Harness {
        host: ProjectHost::new(id("missing-behavior-host"), ui.clone()),
        ui,
        tree,
    };
    let task = h.import(&seed_dir);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, true));
    let active = h.object("project", "P1");
    let debug = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/方案比较/runtime/missing-behavior-input.json");
    write(
        &debug,
        &json!({"active":active,"manifest":read(&seed_dir.join("project.json"))}),
    );
    let target = h.tree.directory("missing-behavior-export");
    let task = export(&mut h, &target, &[], &["LOCAL"], false);
    let exported = exported(&mut h, &task);
    assert!(exported.missing.iter().any(|e| {
        e.blocks
            .iter()
            .any(|b| matches!(b, CapabilityPurpose::Execution))
    }));
    let manifest = read(&target.join("project.json"));
    assert!(
        !manifest["data"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["file_ref"]["kind"] == "run_result")
    );
    let task = h.import(&target);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, true));
    let rev = h.context().project.unwrap().revision;
    let scenario = h.object("scenario", "LOCAL");
    let reply=h.call("run_scenario",json!({"scenario_id":"LOCAL","input_revision":rev,"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}));
    let response: Response<TaskHandle> = serde_json::from_value(reply).unwrap();
    match response.outcome {
        ApiOutcome::Error(e) => assert!(!e.message.is_empty()),
        ApiOutcome::Ok(task) => {
            h.finish_one();
            let state = h.task(&task, true);
            assert!(matches!(state.task.state, ComputationStatus::Failed));
            assert!(state.result.is_none());
        }
        other => panic!("unexpected outcome {other:?}"),
    }
}
