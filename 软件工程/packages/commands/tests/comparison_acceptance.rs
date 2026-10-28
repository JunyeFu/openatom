//! Independent P2-03 Host, actual Engine inputs and actual directory/ZIP consumers.
#[path = "fixtures/production/p1_engine_support.rs"]
mod engine_support;
#[path = "fixtures/production/host_support.rs"]
mod host;
#[path = "../../../tests/planner/acceptance/read_host.rs"]
mod planner_read_acceptance;
use host::*;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;

fn quality() -> Value {
    serde_json::from_str(include_str!("fixtures/quality/quality-original.json")).unwrap()
}
fn frozen(mut v: Value, suffix: &str, partial: bool) -> Value {
    v["scenario"]["id"] = json!(format!(
        "{}-{suffix}",
        v["scenario"]["id"].as_str().unwrap()
    ));
    v["input_ref"]["scenario_id"] = v["scenario"]["id"].clone();
    v["project"]["scenario_refs"] = json!([v["scenario"]["id"]]);
    let mut e = engine_support::engine(&v);
    if partial {
        for _ in 0..3 {
            e.step();
        }
        e.cancel();
    } else {
        e.run_to_end();
    }
    json!([v["project"], v["scenario"], v["definitions"], e.snapshot()])
}
#[test]
fn record_real_engine_frozen_inputs_for_the_reporting_consumer() {
    let halls_a: Value =
        serde_json::from_str(include_str!("fixtures/production/p202-halls-A.json")).unwrap();
    let halls_b: Value =
        serde_json::from_str(include_str!("fixtures/production/p202-halls-B.json")).unwrap();
    let original = quality();
    let mut changed = quality();
    changed["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["effective_at_ms"] =
        json!(95000);
    let mut window = quality();
    window["scenario"]["window"]["end_ms"] = json!(110000);
    window["scenario"]["acceptance_goal"]["observation_window"] =
        window["scenario"]["window"].clone();
    let mut destination = quality();
    destination["scenario"]["acceptance_goal"]["acceptance_scope"]["destination"] =
        json!("pending-store");
    let values = json!({"halls_a":frozen(halls_a,"record",false),"halls_b":frozen(halls_b,"record",false),
        "quality_a":frozen(original.clone(),"A",false),"quality_same":frozen(original.clone(),"same",false),
        "quality_changed":frozen(changed,"changed",false),"quality_window":frozen(window,"window",false),
        "quality_scope":frozen(destination,"scope",false),"quality_partial":frozen(original,"partial",true)});
    assert_eq!(values["quality_a"][3]["evidence_origin"], "actual_software");
    assert_eq!(
        values["quality_a"][3]["acceptance_events"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        values["halls_a"][3]["acceptance_events"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        values["halls_b"][3]["acceptance_events"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    if let Some(path) = std::env::var_os("W5_RECORD_COMPARISON_INPUTS") {
        write(std::path::Path::new(&path), &values);
    }
}
fn compare(h: &mut Harness, runs: &[RunId]) -> Value {
    let c: RunComparison = ok(h.call(
        "compare_runs",
        json!({"run_ids":runs,"metric_scope":{"keys":[],"objects":[],"acceptance":[]}}),
    ));
    serde_json::to_value(c).unwrap()
}
fn two_runs(h: &mut Harness) -> (RunResult, RunResult) {
    let task = h.run("QUALITY-ORIGINAL");
    h.finish_one();
    let old = h.completed_run(&task);
    let mut scenario = h.object("scenario", "QUALITY-ORIGINAL");
    let base = h.context().project.unwrap().revision;
    scenario["id"] = json!("QUALITY-CHANGED");
    scenario["revision"] = json!(1);
    scenario["project_revision"] = serde_json::to_value(&base).unwrap();
    scenario["initial_conditions"]["declared_quality_decisions"][0]["effective_at_ms"] =
        json!(95000);
    let _: EditCommit = ok(h.apply(base.0.value(), json!([put("scenario", scenario)])));
    let task = h.run("QUALITY-CHANGED");
    h.finish_one();
    let new = h.completed_run(&task);
    assert_eq!(
        serde_json::to_value(h.full(&old.id)).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    assert_ne!(old.id, new.id);
    (old, new)
}
#[test]
fn actual_host_compares_frozen_runs_after_new_scenario_without_reinterpreting_old_run() {
    let mut h = Harness::from_value(quality());
    let (old, new) = two_runs(&mut h);
    let c = compare(&mut h, &[old.id.clone(), new.id.clone()]);
    assert!(
        !c["pairs"][0]["input_changes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        c["pairs"][0]["conditions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["key"] == "initial_conditions" && x["matches"] == false)
    );
    assert_ne!(c["pairs"][0]["status"], "strict");
    assert_eq!(
        serde_json::to_value(h.full(&old.id)).unwrap(),
        serde_json::to_value(old).unwrap()
    );
}
fn export(h: &mut Harness, zip: bool, runs: &[RunId], scenarios: Value) -> std::path::PathBuf {
    let path = if zip {
        h.tree.path.join("selected.zip")
    } else {
        h.tree.directory("selected-directory")
    };
    let selected = if zip {
        h.host
            .select_file(&h.ui, FilePurpose::ExportPackage, path.clone())
            .unwrap()
    } else {
        h.host
            .select_directory(&h.ui, FilePurpose::ExportPackage, path.clone())
            .unwrap()
    };
    let revision = h.context().project.unwrap().revision;
    let task:TaskHandle=ok(h.call("export_package",json!({"revision":revision,"selected_destination_ref":selected,
        "package_scope":{"purpose":"handoff_project","scenarios":scenarios,"run_ids":runs,"artifact_ids":[],"module_refs":[],"include_redistributable_assets":false}})));
    assert!(
        !path.exists() || !zip,
        "Task admission is not ZIP completion"
    );
    h.finish_one();
    let state = h.task(&task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:#?}"
    );
    assert!(path.exists());
    path
}
fn reopen(source: &mut Harness, path: &std::path::Path, zip: bool) -> Harness {
    let tree = Sandbox::new();
    let destination = if zip {
        tree.path.join("moved.zip")
    } else {
        tree.directory("moved-directory")
    };
    if zip {
        std::fs::copy(path, &destination).unwrap();
    } else {
        copy(path, &destination);
    }
    let ui = id("reopen-comparison-ui");
    let host = ProjectHost::new(id("reopen-comparison-host"), ui.clone());
    let mut h = Harness { host, ui, tree };
    let task = if zip {
        let token = h
            .host
            .select_file(&h.ui, FilePurpose::ImportPackage, destination)
            .unwrap();
        ok(h.call_as(
            &h.ui.clone(),
            false,
            "import_package",
            json!({"selected_file_ref":token,"import_policy":"open_project","base_revision":null}),
        ))
    } else {
        h.import(&destination)
    };
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    assert_eq!(
        source.object("project", "P1")["module_refs"],
        h.object("project", "P1")["module_refs"]
    );
    h
}
#[test]
fn selected_directory_and_zip_reopen_move_edit_and_rerun_with_old_results_unchanged() {
    for zip in [false, true] {
        let mut source = Harness::from_value(quality());
        let (old, new) = two_runs(&mut source);
        let path = export(
            &mut source,
            zip,
            &[old.id.clone(), new.id.clone()],
            json!(["QUALITY-ORIGINAL", "QUALITY-CHANGED"]),
        );
        let mut opened = reopen(&mut source, &path, zip);
        assert_eq!(
            serde_json::to_value(opened.full(&old.id)).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        assert_eq!(
            serde_json::to_value(opened.full(&new.id)).unwrap(),
            serde_json::to_value(&new).unwrap()
        );
        let c = compare(&mut opened, &[old.id.clone(), new.id.clone()]);
        assert_eq!(c["run_ids"].as_array().unwrap().len(), 2);
        let mut scenario = opened.object("scenario", "QUALITY-CHANGED");
        let base = opened.context().project.unwrap().revision;
        scenario["initial_conditions"]["declared_quality_decisions"][0]["effective_at_ms"] =
            json!(100000);
        let _: EditCommit = ok(opened.apply(base.0.value(), json!([put("scenario", scenario)])));
        let task = opened.run("QUALITY-CHANGED");
        opened.finish_one();
        let rerun = opened.completed_run(&task);
        assert_ne!(rerun.id, new.id);
        assert_eq!(rerun.window.start_ms.0.value(), 0);
        assert_eq!(
            serde_json::to_value(opened.full(&old.id)).unwrap(),
            serde_json::to_value(&old).unwrap()
        );
        assert!(
            rerun.events.iter().any(|e| e.at_ms.0.value() == 0),
            "reopen restart is real initial execution, not snapshot continuation"
        );
    }
}
#[test]
fn design_only_export_does_not_invent_run_and_rejects_unknown_selected_run_without_commit() {
    let mut h = Harness::from_value(quality());
    let path = export(&mut h, false, &[], json!(["QUALITY-ORIGINAL"]));
    let manifest = read(&path.join("project.json"));
    assert_eq!(
        manifest["data"]["restart_policy"],
        "restart_from_initial_input"
    );
    assert!(
        !manifest["data"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["role"] == "run_result")
    );
    let mut opened = reopen(&mut h, &path, false);
    let missing = error(opened.call(
        "get_run",
        json!({"run_id":"never-produced","query":{"kind":"full_result"}}),
    ));
    assert!(matches!(missing.code, ErrorCode::MissingReference));
    let before = std::fs::read(path.join("project.json")).unwrap();
    let token = h
        .host
        .select_directory(&h.ui, FilePurpose::ExportPackage, path.clone())
        .unwrap();
    let rev = h.context().project.unwrap().revision;
    let task:TaskHandle=ok(h.call("export_package",json!({"revision":rev,"selected_destination_ref":token,"package_scope":{
        "purpose":"handoff_project","scenarios":["QUALITY-ORIGINAL"],"run_ids":["never-produced"],"artifact_ids":[],"module_refs":[],"include_redistributable_assets":false}})));
    h.finish_one();
    let state = h.task(&task, true);
    assert!(matches!(state.task.state, ComputationStatus::Failed));
    assert_eq!(std::fs::read(path.join("project.json")).unwrap(), before);
}

#[test]
fn explicit_selected_old_run_and_scenario_exclude_unselected_new_history_after_real_reopen() {
    let mut h = Harness::from_value(quality());
    let (old, new) = two_runs(&mut h);
    let path = export(
        &mut h,
        false,
        std::slice::from_ref(&old.id),
        json!(["QUALITY-ORIGINAL"]),
    );
    let mut opened = reopen(&mut h, &path, false);
    assert_eq!(
        opened.object("project", "P1")["scenario_refs"],
        json!(["QUALITY-ORIGINAL"])
    );
    assert_eq!(
        serde_json::to_value(opened.full(&old.id)).unwrap(),
        serde_json::to_value(old).unwrap()
    );
    let failure = error(opened.call(
        "get_run",
        json!({"run_id":new.id,"query":{"kind":"full_result"}}),
    ));
    assert!(matches!(failure.code, ErrorCode::MissingReference));
}

#[test]
fn cancellation_before_actual_export_io_keeps_previous_valid_destination() {
    let mut h = Harness::from_value(quality());
    let path = export(&mut h, false, &[], json!(["QUALITY-ORIGINAL"]));
    let before = std::fs::read(path.join("project.json")).unwrap();
    let token = h
        .host
        .select_directory(&h.ui, FilePurpose::ExportPackage, path.clone())
        .unwrap();
    let revision = h.context().project.unwrap().revision;
    let task:TaskHandle=ok(h.call("export_package",json!({"revision":revision,"selected_destination_ref":token,
        "package_scope":{"purpose":"handoff_project","scenarios":["QUALITY-ORIGINAL"],"run_ids":[],"artifact_ids":[],"module_refs":[],"include_redistributable_assets":false}})));
    let _: CancellationState =
        ok(h.call("cancel_task", json!({"task_id":task.id,"action":"cancel"})));
    h.finish_one();
    let state = h.task(&task, true);
    assert!(matches!(state.task.state, ComputationStatus::Cancelled));
    assert_eq!(std::fs::read(path.join("project.json")).unwrap(), before);
    let mut reopened = reopen(&mut h, &path, false);
    assert_eq!(
        reopened.object("scenario", "QUALITY-ORIGINAL")["window"]["start_ms"],
        0
    );
}
