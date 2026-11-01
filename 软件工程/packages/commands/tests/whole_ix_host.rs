//! Full IX public Commands consumers. No direct Engine or professional solver call.
#[path = "fixtures/whole_ix/checks.rs"]
mod checks;
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use support::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn evidence(name: &str, value: &Value) {
    write(
        &root().join("scenarios/集成验收/runtime-host").join(name),
        value,
    );
}
fn preserve_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            preserve_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
fn source(name: &str) -> Value {
    serde_json::from_str(match name {
        "baseline" => include_str!("fixtures/production/p202-ix01-baseline.json"),
        "delayed" => include_str!("fixtures/production/p202-ix01-delayed-pcb.json"),
        "orders" => include_str!("fixtures/production/p104-A-first.json"),
        "phase" => include_str!("fixtures/production/p1-phase-calendar.json"),
        _ => panic!("fixed named input"),
    })
    .unwrap()
}
fn execute(h: &mut Harness, name: &str) -> RunResult {
    let task = h.run(name);
    assert!(matches!(task.state, ComputationStatus::Queued));
    assert!(h.task(&task, true).result.is_none());
    h.finish_one();
    h.completed_run(&task)
}
fn completed(h: &mut Harness, task: &TaskHandle) -> TaskOutput {
    let state = h.task(task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    state.result.unwrap()
}
fn events(h: &mut Harness, run: &RunId, limit: u64) -> Value {
    let mut after = Value::Null;
    let mut all = vec![];
    loop {
        let view:RunView=ok(h.call("get_run",json!({"run_id":run,"query":{"kind":"events","data":{"interval":null,"page":{"after_index":after,"limit":limit}}}})));
        let RunView::Events(page) = view else {
            panic!("actual events")
        };
        all.extend(page.events);
        let Some(next) = page.next_index else { break };
        after = serde_json::to_value(next).unwrap();
    }
    serde_json::to_value(all).unwrap()
}
fn objects(h: &mut Harness, run: &RunId, at: u64, selector: Value, limit: u64) -> Value {
    let mut after = Value::Null;
    let mut all = vec![];
    loop {
        let view:RunView=ok(h.call("get_run",json!({"run_id":run,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":selector,"page":{"after_index":after,"limit":limit}}}})));
        let RunView::Objects(page) = view else {
            panic!("actual ObjectsAt")
        };
        all.extend(page.objects);
        let Some(next) = page.next_index else { break };
        after = serde_json::to_value(next).unwrap();
    }
    serde_json::to_value(all).unwrap()
}
fn history(h: &mut Harness, run: &RunResult, moments: &[u64]) -> Value {
    let selector = json!(
        run.jobs
            .iter()
            .map(|j| json!({"kind":"job","id":j.id}))
            .chain(
                run.final_items
                    .iter()
                    .map(|i| json!({"kind":"item","id":i.id}))
            )
            .collect::<Vec<_>>()
    );
    let before = h.object("project", "P1");
    let mut states = serde_json::Map::new();
    for at in moments {
        let pages = objects(h, &run.id, *at, selector.clone(), 3);
        assert_eq!(pages, objects(h, &run.id, *at, selector.clone(), 10000));
        let empty = objects(h, &run.id, *at, json!([]), 3);
        assert!(
            empty
                .as_array()
                .unwrap()
                .iter()
                .all(|o| o["kind"] == "item")
        );
        assert_eq!(
            empty,
            json!(
                pages
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|o| o["kind"] == "item")
                    .cloned()
                    .collect::<Vec<_>>()
            )
        );
        states.insert(at.to_string(), pages);
    }
    assert_eq!(
        events(h, &run.id, 7),
        serde_json::to_value(&run.events).unwrap()
    );
    assert_eq!(h.object("project", "P1"), before);
    Value::Object(states)
}
fn frozen(h: &mut Harness, run: &RunResult, kind: &str, id: &str) -> Value {
    let source = ObjectSource::RunSnapshot {
        run_id: run.id.clone(),
        at_ms: run.window.end_ms.clone(),
    };
    let value: DomainObject = ok(h.call(
        "get_object",
        json!({"object_id":{"kind":kind,"id":id},"source":source,"node_index":null}),
    ));
    serde_json::to_value(value).unwrap()["data"].clone()
}
fn shell_buffer(h: &mut Harness, run: &RunResult, name: &str) -> Value {
    let expected = checks::expected("ix01", name);
    let mut states = serde_json::Map::new();
    for event in expected["buffer_events"].as_array().unwrap() {
        let at = event["at_ms"].as_u64().unwrap();
        let all = objects(h, &run.id, at, json!([]), 3);
        let shells: Vec<_> = all
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| {
                o["data"]["item_type_ref"]["id"] == "shell"
                    && o["data"]["holder"] == json!({"kind":"location","id":"shell-buffer"})
                    && o["data"]["lifecycle"] != "transformed"
                    && o["data"]["lifecycle"] != "scrapped"
            })
            .cloned()
            .collect();
        assert_eq!(
            shells.len() as u64,
            event["quantity"].as_u64().unwrap(),
            "original closed buffer quantity at {at}"
        );
        states.insert(at.to_string(), json!(shells));
    }
    Value::Object(states)
}
fn exact_modules(h: &mut Harness, run: &RunResult, input: &Value) {
    for definition in input["definitions"].as_array().unwrap() {
        let reference =
            serde_json::from_value(json!({"id":definition["id"],"version":definition["version"]}))
                .unwrap();
        let source = ObjectSource::RunModuleVersion {
            run_id: run.id.clone(),
            at_ms: run.window.end_ms.clone(),
            module_ref: reference,
        };
        let value:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"module","id":definition["id"]},"source":source,"node_index":null})));
        let canonical: ModuleDefinition = serde_json::from_value(definition.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(value).unwrap()["data"]["definition"],
            serde_json::to_value(canonical).unwrap()
        );
    }
}
fn relocate(h: &mut Harness, name: &str) -> Harness {
    let saved = h.tree.directory(&format!("{name}-save"));
    let base = h.context().project.unwrap().revision.0.value();
    let task = h.save(base, &saved);
    h.finish_one();
    completed(h, &task);
    let tree = Sandbox::new();
    let moved = tree.directory(&format!("{name}-moved"));
    copy(&saved, &moved);
    Harness::from_seed(tree, &moved)
}
fn compare(h: &mut Harness, runs: &[RunId]) -> Value {
    let output: RunComparison = ok(h.call(
        "compare_runs",
        json!({"run_ids":runs,"metric_scope":{"keys":[],"objects":[],"acceptance":[]}}),
    ));
    serde_json::to_value(output).unwrap()
}

#[test]
fn ix01_full_baseline_and_delayed_logistics_frozen_history_and_fresh_reopen() {
    let original = source("baseline");
    let late = source("delayed");
    let mut h = Harness::from_value(original.clone());
    let before = h.object("project", "P1");
    let scenario = h.object("scenario", "IX01-baseline");
    let baseline = execute(&mut h, "IX01-baseline");
    let base_value = serde_json::to_value(&baseline).unwrap();
    checks::ix01(&base_value, "baseline");
    checks::original_materials(
        &base_value,
        &original,
        &[("blank", 1), ("pcb", 1), ("fastener", 4), ("carton", 1)],
    );
    assert_eq!(frozen(&mut h, &baseline, "project", "P1"), before);
    assert_eq!(
        frozen(&mut h, &baseline, "scenario", "IX01-baseline"),
        scenario
    );
    exact_modules(&mut h, &baseline, &original);
    let mut next = late["scenario"].clone();
    next["project_revision"] = json!(1);
    let _: EditCommit = ok(h.apply(1, json!([put("scenario", next)])));
    let delayed = execute(&mut h, "IX01-delayed-pcb");
    let late_value = serde_json::to_value(&delayed).unwrap();
    checks::ix01(&late_value, "delayed_pcb");
    checks::original_materials(
        &late_value,
        &late,
        &[("blank", 1), ("pcb", 1), ("fastener", 4), ("carton", 1)],
    );
    let moments = [
        0, 60000, 70000, 75000, 240000, 260000, 369999, 370000, 419999, 420000, 425000, 440000,
        570000, 600000,
    ];
    let base_history = history(&mut h, &baseline, &moments);
    let late_history = history(&mut h, &delayed, &moments);
    let base_buffer = shell_buffer(&mut h, &baseline, "baseline");
    let late_buffer = shell_buffer(&mut h, &delayed, "delayed_pcb");
    let cargo = late_value["transport"]
        .as_array()
        .unwrap()
        .iter()
        .max_by_key(|t| t["departed_ms"].as_u64())
        .unwrap()["cargo"][0]
        .clone();
    for (at, holder) in [
        (419999, json!({"kind":"transport"})),
        (420000, json!({"kind":"location","id":"shell-buffer"})),
    ] {
        let item = late_history[at.to_string()]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["kind"] == "item" && o["data"]["id"] == cargo)
            .unwrap();
        assert_eq!(item["data"]["holder"]["kind"], holder["kind"]);
        if at == 420000 {
            assert_eq!(item["data"]["holder"], holder);
        }
    }
    let comparison = compare(&mut h, &[baseline.id.clone(), delayed.id.clone()]);
    assert!(
        !comparison["pairs"][0]["input_changes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_ne!(comparison["pairs"][0]["status"], "strict");
    assert_eq!(
        serde_json::to_value(h.full(&baseline.id)).unwrap(),
        base_value
    );
    let mut reopened = relocate(&mut h, "ix01");
    for (run, value, states) in [
        (&baseline, &base_value, &base_history),
        (&delayed, &late_value, &late_history),
    ] {
        assert_eq!(
            serde_json::to_value(reopened.full(&run.id)).unwrap(),
            *value
        );
        assert_eq!(history(&mut reopened, run, &moments), *states);
        exact_modules(&mut reopened, run, &original);
    }
    assert_eq!(
        compare(&mut reopened, &[baseline.id.clone(), delayed.id.clone()]),
        comparison
    );
    assert_eq!(
        shell_buffer(&mut reopened, &baseline, "baseline"),
        base_buffer
    );
    assert_eq!(
        shell_buffer(&mut reopened, &delayed, "delayed_pcb"),
        late_buffer
    );
    evidence(
        "ix01.json",
        &json!({"baseline":base_value,"delayed":late_value,"baseline_history":base_history,"delayed_history":late_history,"baseline_buffer":base_buffer,"delayed_buffer":late_buffer,"comparison":comparison,"fresh_reopen_full_equal":true}),
    );
}

#[test]
fn ix02_full_order_changeover_priority_variants_compare_and_reopen_unchanged() {
    let input = source("orders");
    let mut h = Harness::from_value(input.clone());
    let first = execute(&mut h, "ORDERS-A");
    let first_value = serde_json::to_value(&first).unwrap();
    checks::ix02(&first_value, "multi_a_first");
    checks::original_materials(
        &first_value,
        &input,
        &[("shell", 1), ("pcb", 1), ("fastener", 4), ("carton", 1)],
    );
    let frozen_first = frozen(&mut h, &first, "scenario", "ORDERS-A");
    let mut scenario = h.object("scenario", "ORDERS-A");
    for task in scenario["production_tasks"].as_array_mut().unwrap() {
        task["priority"] = json!(if task["id"] == "TASK-B" { 1 } else { 0 });
    }
    let _: EditCommit = ok(h.apply(1, json!([put("scenario", scenario)])));
    let second = execute(&mut h, "ORDERS-A");
    let second_value = serde_json::to_value(&second).unwrap();
    checks::ix02(&second_value, "multi_b_first");
    checks::original_materials(
        &second_value,
        &input,
        &[("shell", 1), ("pcb", 1), ("fastener", 4), ("carton", 1)],
    );
    assert_ne!(first.id, second.id);
    assert_eq!(frozen(&mut h, &first, "scenario", "ORDERS-A"), frozen_first);
    let moments = [
        0, 15000, 60000, 75000, 99999, 100000, 120000, 140000, 145000, 165000, 180000, 200000,
        205000, 225000,
    ];
    let a = history(&mut h, &first, &moments);
    let b = history(&mut h, &second, &moments);
    let comparison = compare(&mut h, &[first.id.clone(), second.id.clone()]);
    assert!(
        !comparison["pairs"][0]["input_changes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut reopened = relocate(&mut h, "ix02");
    for (run, value, states) in [(&first, &first_value, &a), (&second, &second_value, &b)] {
        assert_eq!(
            serde_json::to_value(reopened.full(&run.id)).unwrap(),
            *value
        );
        assert_eq!(history(&mut reopened, run, &moments), *states);
        exact_modules(&mut reopened, run, &input);
    }
    assert_eq!(
        compare(&mut reopened, &[first.id.clone(), second.id.clone()]),
        comparison
    );
    evidence(
        "ix02.json",
        &json!({"a_first":first_value,"b_first":second_value,"a_history":a,"b_history":b,"comparison":comparison,"fresh_reopen_full_equal":true}),
    );
}

#[test]
fn ix02_phase_shift_worker_history_preserves_actual_45_seconds_and_10_seconds_work() {
    let input = source("phase");
    let name = input["scenario"]["id"].as_str().unwrap();
    let mut h = Harness::from_value(input.clone());
    let run = execute(&mut h, name);
    let value = serde_json::to_value(&run).unwrap();
    checks::common(&value);
    let job = run
        .jobs
        .iter()
        .find(|j| j.operation_ref.id.as_str() == "w5.op.phased")
        .unwrap();
    assert_eq!(job.finished_at_ms.as_ref().unwrap().0.value(), 45000);
    assert_eq!(
        checks::intervals(&value, "W-C")
            .iter()
            .map(|(a, b)| b - a)
            .sum::<u64>(),
        10000
    );
    let moments = [0, 5000, 34999, 35000, 39999, 40000, 42500, 45000];
    let states = history(&mut h, &run, &moments);
    let wait = states["35000"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "job" && o["data"]["id"] == serde_json::to_value(&job.id).unwrap())
        .unwrap();
    assert_eq!(wait["data"]["state"], "waiting_resource");
    assert_eq!(wait["data"]["phase_ref"], "unload");
    assert_eq!(wait["data"]["effective_progress_ms"], 0);
    let mut reopened = relocate(&mut h, "ix02-phase");
    assert_eq!(history(&mut reopened, &run, &moments), states);
    assert_eq!(serde_json::to_value(reopened.full(&run.id)).unwrap(), value);
    evidence(
        "ix02-phase.json",
        &json!({"run":value,"history":states,"fresh_reopen_full_equal":true}),
    );
}

fn packaging_case(name: &str) -> Value {
    let cases: Value = serde_json::from_str(include_str!(
        "fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == name)
        .unwrap()["job"]["data"]
        .clone()
}
fn compute(h: &mut Harness, job: &Value) -> PackagingResult {
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([{"kind":"put_packaging_job","data":job}])));
    let base = h.context().project.unwrap().revision;
    let task: TaskHandle = ok(h.call(
        "compute_packaging",
        json!({"packaging_job":job,"input_revision":base,"computation_budget":job["budget"]}),
    ));
    assert!(matches!(task.state, ComputationStatus::Queued));
    assert!(h.task(&task, true).result.is_none());
    h.finish_one();
    let TaskOutput::Packaging(result) = completed(h, &task) else {
        panic!("real packaging result")
    };
    result
}
fn typed_job(value: &Value) -> Value {
    serde_json::to_value(serde_json::from_value::<PackageStructureJob>(value.clone()).unwrap())
        .unwrap()
}
fn packaging_input(job: &Value) -> Value {
    let mut input = fixture();
    let mut product = input["project"]["products"][0].clone();
    product["id"] = job["subject_ref"]["id"].clone();
    product["revision"] = job["subject_ref"]["revision"].clone();
    input["project"]["products"]
        .as_array_mut()
        .unwrap()
        .push(product);
    input
}

#[test]
fn ix03_real_host_three_mechanisms_three_changes_and_continuous_process_three_states() {
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/whole_ix/original-expectations.json")).unwrap();
    let mut outputs = serde_json::Map::new();
    let coverage = expected["ix03"]["coverage_cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| (v.as_str().unwrap(), None));
    let states = expected["ix03"]["forming_cases"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.as_str(), Some(v.as_str().unwrap())));
    for (name, purpose_outcome) in coverage.chain(states) {
        let job = packaging_case(name);
        let mut h = Harness::from_value(packaging_input(&job));
        let physical = h.object("scenario", "LOCAL")["initial_conditions"].clone();
        let result = compute(&mut h, &job);
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(value["evidence_origin"], "actual_software");
        assert!(!result.checks.is_empty());
        if let Some(expected) = purpose_outcome {
            assert!(
                value["checks"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["purpose"] == "forming" && c["outcome"] == expected)
            );
            if expected == "fail" {
                assert_eq!(value["outcome"], "fail");
                assert!(
                    value["checks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|c| !c["witnesses"].as_array().unwrap().is_empty())
                );
            }
            if expected == "undetermined" {
                assert_ne!(value["outcome"], "pass");
            }
        } else {
            assert_eq!(
                value["outcome"], "pass",
                "original complete coverage candidate {name}"
            );
            assert!(!result.manufacturing_files.is_empty());
            assert!(!result.process_poses.is_empty());
            for purpose in [
                "forming",
                "opening",
                "linear_extraction",
                "net_space",
                "manufacturing",
            ] {
                assert!(
                    value["checks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|c| c["purpose"] == purpose && c["outcome"] == "pass")
                );
            }
        }
        let project = h.object("project", "P1");
        assert_eq!(project["acknowledgments"], json!([]));
        assert_eq!(project["bindings"], json!([]));
        assert_eq!(
            h.object("scenario", "LOCAL")["initial_conditions"],
            physical
        );
        let revision = h.context().project.unwrap().revision;
        let (_, stored_result, stored_job) = h
            .host
            .read_stored_packaging_artifact(
                &h.ui,
                &"P1".to_owned().try_into().unwrap(),
                &ApiScope::Project,
                &revision,
                &result.artifact_id,
            )
            .unwrap();
        assert_eq!(serde_json::to_value(stored_result).unwrap(), value);
        assert_eq!(serde_json::to_value(stored_job).unwrap(), typed_job(&job));
        outputs.insert(name.to_owned(),json!({"input":job,"result":value,"inventory_unchanged":true,"no_implicit_approval_or_binding":true}));
    }
    evidence("ix03-mechanisms-and-states.json", &Value::Object(outputs));
}

fn professional_input() -> Value {
    let mut value = fixture();
    let declaration: Value =
        serde_json::from_str(include_str!("../../packaging/consumer-device.design.json")).unwrap();
    value["definitions"][5]["parameters"] = json!(
        declaration["data"]["definition"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["id"] == "packaging_job_id" || p["id"] == "packaging_job_revision")
            .cloned()
            .collect::<Vec<_>>()
    );
    value["project"]["devices"][1]["configuration"] = json!({"packaging_job_id":{"kind":"string","value":"IX03-JOB"},"packaging_job_revision":{"kind":"integer","value":1}});
    value
}
fn scope() -> Value {
    json!([
        "forming",
        "opening",
        "linear_extraction",
        "net_space",
        "manufacturing"
    ])
}
fn adoption(h: &mut Harness, result: &PackagingResult, ack: &str) -> Value {
    let base = h.context().project.unwrap().revision;
    json!({"artifact_id":result.artifact_id,"acceptance_ref":ack,"consumer_ref":{"kind":"device","id":"PACK-01"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},"requested_scope":scope(),"base_revision":base})
}
fn acknowledge(h: &mut Harness, result: &PackagingResult, name: &str, subject_revision: &str) {
    let base = h.context().project.unwrap().revision.0.value();
    let _:EditCommit=ok(h.apply(base,json!([{"kind":"record_artifact_acknowledgment","data":{"id":name,"artifact_ref":result.artifact_id,"accepted_subject":{"id":"PACKAGE","revision":subject_revision},"accepted_scope":scope(),"decision_source":{"kind":"human_statement","data":"Explicit simulated UI choice for automated Host integration; no claim of human acceptance"},"conditions":[],"recorded_at_iso8601":null}}])));
}
fn fonts() -> sf_reporting::EvaluationResources {
    let assets = root().join("tools/architecture-proof/assets");
    sf_reporting::EvaluationResources {
        cjk_font: Some(fs::read(assets.join("SourceHanSansCN-Regular.otf")).unwrap()),
        font_license: Some(fs::read(assets.join("LICENSE.SourceHanSans.txt")).unwrap()),
        files: BTreeMap::new(),
    }
}
fn report(h: &mut Harness, runs: &[RunId]) -> (ExportEvaluation, PathBuf) {
    h.host.set_evaluation_resources(fonts());
    let parent = h.tree.directory("ix03-evaluation");
    let token = h
        .host
        .select_directory(&h.ui, FilePurpose::ExportEvaluation, parent.clone())
        .unwrap();
    let base = h.context().project.unwrap().revision;
    let task:TaskHandle=ok(h.call("export_evaluation",json!({"revision":base,"report_mode":"simulation_evaluation","scenario_refs":[],"run_refs":runs,"metric_scope":{"keys":[],"objects":[],"acceptance":[]},"formats":["pdf","html","csv","json"],"selected_destination_ref":token})));
    h.finish_one();
    let TaskOutput::Evaluation(output) = completed(h, &task) else {
        panic!("actual Evaluation")
    };
    let file = output
        .files
        .iter()
        .find(|f| matches!(f.kind, DocumentKind::Evaluation))
        .unwrap();
    let dir = parent.join(file.path.as_str()).parent().unwrap().to_owned();
    (*output, dir)
}
fn zip(h: &mut Harness, runs: &[RunId], artifacts: &[ArtifactId]) -> PathBuf {
    zip_selected(h, runs, artifacts, "ix03-handoff.zip", false)
}
fn zip_selected(
    h: &mut Harness,
    runs: &[RunId],
    artifacts: &[ArtifactId],
    name: &str,
    include: bool,
) -> PathBuf {
    let path = h.tree.path.join(name);
    let token = h
        .host
        .select_file(&h.ui, FilePurpose::ExportPackage, path.clone())
        .unwrap();
    let base = h.context().project.unwrap().revision;
    let task:TaskHandle=ok(h.call("export_package",json!({"revision":base,"selected_destination_ref":token,"package_scope":{"purpose":"handoff_project","scenarios":[],"run_ids":runs,"artifact_ids":artifacts,"module_refs":[],"include_redistributable_assets":include}})));
    h.finish_one();
    completed(h, &task);
    assert!(path.is_file());
    path
}
fn import_owned_output_rights(h: &mut Harness, results: &[&PackagingResult]) -> Vec<AssetId> {
    // These are this test's controlled synthetic outputs. This declaration is not
    // an authorization for user/third-party files, nor a Host confirmation.
    let original = h.tree.directory("owned-output-save");
    let base = h.context().project.unwrap().revision.0.value();
    let task = h.save(base, &original);
    h.finish_one();
    completed(h, &task);
    let declared = h.tree.directory("owned-output-explicit-declaration");
    copy(&original, &declared);
    let mut manifest = read(&declared.join("project.json"));
    let mut assets = vec![];
    for result in results {
        for file in &result.manufacturing_files {
            let entry = manifest["data"]["documents"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|e| e["file_ref"]["path"] == file.file.path.as_str())
                .unwrap();
            assert_eq!(entry["object_ref"]["kind"], "asset");
            // Canonical rights declarations address included Asset entries.
            // Explicitly index these owned bytes; do not alter their content or refs.
            entry["role"] = json!("asset");
            let id = entry["object_ref"]["id"].clone();
            let asset: AssetId = serde_json::from_value(id.clone()).unwrap();
            assets.push(asset);
            manifest["data"]["assets"].as_array_mut().unwrap().push(json!({"asset_ref":id,"source":"This whole_ix_host test's controlled synthetic geometry and actual generated SVG; explicit fixture-owner dedication, not third-party permission","license":"CC0-1.0","redistribution_confirmed":true,"missing_reason":null}));
        }
    }
    write(&declared.join("project.json"), &manifest);
    let task = h.import(&declared);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    assets
}

#[test]
fn ix03_actual_compute_ui_choice_independent_binding_revision_review_report_and_professional_zip() {
    let mut h = Harness::from_value(professional_input());
    let physical = h.object("scenario", "LOCAL")["initial_conditions"].clone();
    let mut job = packaging_case("COVERAGE-restricted_folded_container");
    job["id"] = json!("IX03-JOB");
    job["subject_ref"] = json!({"id":"PACKAGE","revision":"r1"});
    job["conditions"]["product_envelope"]["subject"] = job["subject_ref"].clone();
    let first = compute(&mut h, &job);
    assert!(matches!(first.outcome, CheckOutcome::Pass));
    let original_artifact = h.object("artifact", first.artifact_id.as_str());
    let request = adoption(&mut h, &first, "IX03-ACK-R1");
    let before = h.snapshot();
    error(h.call("adopt_artifact", request));
    assert_eq!(h.snapshot(), before);
    acknowledge(&mut h, &first, "IX03-ACK-R1", "r1");
    assert_eq!(h.object("project", "P1")["bindings"], json!([]));
    assert_eq!(
        h.object("artifact", first.artifact_id.as_str()),
        original_artifact
    );
    let request = adoption(&mut h, &first, "IX03-ACK-R1");
    let binding: BindingCommit = ok(h.call("adopt_artifact", request));
    assert_eq!(
        serde_json::to_value(&binding.binding).unwrap()["mode"],
        "calculated_nominal"
    );
    let old = execute(&mut h, "LOCAL");
    let old_value = serde_json::to_value(&old).unwrap();
    assert_eq!(old.acceptance_events[0].at_ms.0.value(), 75000);
    let original_project = h.object("project", "P1");
    let mut product = h.object("product", "PACKAGE");
    product["revision"] = json!("r2");
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("product", product)])));
    assert!(
        h.context()
            .issues
            .iter()
            .any(|i| i.code == "professional_inputs.selected_subject_changed")
    );
    let base = h.context().project.unwrap().revision;
    let failure=error(h.call("run_scenario",json!({"scenario_id":"LOCAL","input_revision":base,"observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert!(h.host.drain_io_work().is_empty());
    let mut next = job.clone();
    next["revision"] = json!(2);
    next["subject_ref"]["revision"] = json!("r2");
    next["conditions"]["product_envelope"]["subject"] = next["subject_ref"].clone();
    let mut scenario = h.object("scenario", "LOCAL");
    scenario["orders"][0]["product_ref"]["revision"] = json!("r2");
    let base = h.context().project.unwrap().revision.0.value();
    let _:EditCommit=ok(h.apply(base,json!([put("scenario",scenario),{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":2},"basis":"user_provided"}}])));
    let second = compute(&mut h, &next);
    assert!(matches!(second.outcome, CheckOutcome::Pass));
    acknowledge(&mut h, &second, "IX03-ACK-R2", "r2");
    let request = adoption(&mut h, &second, "IX03-ACK-R2");
    let next_binding: BindingCommit = ok(h.call("adopt_artifact", request));
    assert_ne!(binding.binding.id, next_binding.binding.id);
    let new = execute(&mut h, "LOCAL");
    assert_eq!(new.acceptance_events[0].at_ms.0.value(), 75000);
    let new_value = serde_json::to_value(&new).unwrap();
    assert_eq!(
        h.object("artifact", first.artifact_id.as_str()),
        original_artifact
    );
    assert_eq!(serde_json::to_value(h.full(&old.id)).unwrap(), old_value);
    let current = h.object("project", "P1");
    assert_eq!(current["bindings"][0], original_project["bindings"][0]);
    assert_eq!(
        current["acknowledgments"][0],
        original_project["acknowledgments"][0]
    );
    assert_eq!(
        h.object("scenario", "LOCAL")["initial_conditions"],
        physical
    );
    let comparison = compare(&mut h, &[old.id.clone(), new.id.clone()]);
    let (output, dir) = report(&mut h, &[old.id.clone(), new.id.clone()]);
    assert!(
        fs::read(dir.join("report.pdf"))
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert!(dir.join("index.html").is_file());
    assert_eq!(
        read(&dir.join("report.json"))["data"],
        serde_json::to_value(&output.context).unwrap()
    );
    assert!(fs::read_dir(dir.join("csv")).unwrap().count() > 0);
    assert!(
        output
            .limitations
            .iter()
            .any(|i| i.code == "evaluation.actual_render_missing")
    );
    assert_eq!(h.object("project", "P1"), current);
    for result in [&first, &second] {
        for manufacturing in &result.manufacturing_files {
            let svg =
                fs::read_to_string(dir.join("project").join(manufacturing.file.path.as_str()))
                    .unwrap();
            for layer in ["cut", "fold", "glue"] {
                assert!(svg.contains(&format!("id=\"{layer}\"")));
            }
            assert!(svg.contains("mm\""));
        }
    }
    let archive = zip(
        &mut h,
        &[old.id.clone(), new.id.clone()],
        &[first.artifact_id.clone(), second.artifact_id.clone()],
    );
    let handoff_manifest =
        {
            let mut package = zip::ZipArchive::new(fs::File::open(&archive).unwrap()).unwrap();
            let manifest: Versioned<PackageManifest> =
                serde_json::from_reader(package.by_name("project.json").unwrap()).unwrap();
            for result in [&first, &second] {
                for manufacturing in &result.manufacturing_files {
                    assert!(package.by_name(manufacturing.file.path.as_str()).is_err());
                    assert!(
                        manifest.data.missing.iter().any(|i| i.code
                            == "handoff.asset_not_redistributed"
                            && i.field_path.as_deref() == Some(manufacturing.file.path.as_str()))
                    );
                }
            }
            serde_json::to_value(manifest).unwrap()
        };
    let report_tree = Sandbox::new();
    let report_project = report_tree.directory("report-project-moved");
    copy(&dir.join("project"), &report_project);
    let mut report_opened = Harness::from_seed(report_tree, &report_project);
    assert_eq!(
        serde_json::to_value(report_opened.full(&old.id)).unwrap(),
        old_value
    );
    assert_eq!(
        serde_json::to_value(report_opened.full(&new.id)).unwrap(),
        new_value
    );
    assert_eq!(report_opened.object("project", "P1"), current);
    for result in [&first, &second] {
        let revision = report_opened.context().project.unwrap().revision;
        let (_, actual, _) = report_opened
            .host
            .read_stored_packaging_artifact(
                &report_opened.ui,
                &"P1".to_owned().try_into().unwrap(),
                &ApiScope::Project,
                &revision,
                &result.artifact_id,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(result).unwrap()
        );
    }
    let tree = Sandbox::new();
    let moved = tree.path.join("moved-ix03.zip");
    fs::copy(&archive, &moved).unwrap();
    let ui = id("ix03-fresh-ui");
    let mut opened = Harness {
        host: ProjectHost::new(id("ix03-fresh-host"), ui.clone()),
        ui,
        tree,
    };
    let token = opened
        .host
        .select_file(&opened.ui, FilePurpose::ImportPackage, moved)
        .unwrap();
    let handle: TaskHandle = ok(opened.call_as(
        &opened.ui.clone(),
        false,
        "import_package",
        json!({"selected_file_ref":token,"import_policy":"open_project","base_revision":null}),
    ));
    opened.finish_one();
    let draft = opened.imported(&handle);
    assert!(
        draft
            .missing
            .iter()
            .any(|i| i.code == "handoff.asset_not_redistributed")
    );
    let denied = error(opened.adopt(&handle, &draft, false));
    assert!(matches!(denied.code, ErrorCode::InvalidInput));
    let _: EditCommit = ok(opened.adopt(&handle, &draft, true));
    assert_eq!(
        serde_json::to_value(opened.full(&old.id)).unwrap(),
        old_value
    );
    assert_eq!(
        serde_json::to_value(opened.full(&new.id)).unwrap(),
        new_value
    );
    assert_eq!(
        opened.object("project", "P1")["bindings"],
        current["bindings"]
    );
    assert_eq!(
        opened.object("artifact", first.artifact_id.as_str()),
        original_artifact
    );
    for result in [&first, &second] {
        let revision = opened.context().project.unwrap().revision;
        let (_, actual, _) = opened
            .host
            .read_stored_packaging_artifact(
                &opened.ui,
                &"P1".to_owned().try_into().unwrap(),
                &ApiScope::Project,
                &revision,
                &result.artifact_id,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(result).unwrap()
        );
    }
    let base = opened.context().project.unwrap().revision;
    let blocked=error(opened.call("run_scenario",json!({"scenario_id":"LOCAL","input_revision":base,"observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert!(matches!(
        blocked.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference | ErrorCode::PermissionDenied
    ));
    assert!(opened.host.drain_io_work().is_empty());
    let assets = import_owned_output_rights(&mut h, &[&first, &second]);
    let metadata_only = zip_selected(
        &mut h,
        &[old.id.clone(), new.id.clone()],
        &[first.artifact_id.clone(), second.artifact_id.clone()],
        "ix03-metadata-only.zip",
        true,
    );
    {
        let mut package = zip::ZipArchive::new(fs::File::open(metadata_only).unwrap()).unwrap();
        for result in [&first, &second] {
            for file in &result.manufacturing_files {
                assert!(
                    package.by_name(file.file.path.as_str()).is_err(),
                    "imported permission metadata alone must not grant current Host export authority"
                );
            }
        }
    }
    for asset in &assets {
        h.host.confirm_asset_redistribution(&h.ui, asset).unwrap();
    }
    let complete = zip_selected(
        &mut h,
        &[old.id.clone(), new.id.clone()],
        &[first.artifact_id.clone(), second.artifact_id.clone()],
        "ix03-complete-owned.zip",
        true,
    );
    let complete_manifest = {
        use std::io::Read;
        let mut package = zip::ZipArchive::new(fs::File::open(&complete).unwrap()).unwrap();
        let manifest: Versioned<PackageManifest> =
            serde_json::from_reader(package.by_name("project.json").unwrap()).unwrap();
        for result in [&first, &second] {
            for file in &result.manufacturing_files {
                let mut svg = String::new();
                package
                    .by_name(file.file.path.as_str())
                    .unwrap()
                    .read_to_string(&mut svg)
                    .unwrap();
                assert_eq!(
                    svg,
                    fs::read_to_string(dir.join("project").join(file.file.path.as_str())).unwrap()
                );
                assert!(
                    !manifest
                        .data
                        .missing
                        .iter()
                        .any(|i| i.field_path.as_deref() == Some(file.file.path.as_str()))
                );
            }
        }
        serde_json::to_value(manifest).unwrap()
    };
    let tree = Sandbox::new();
    let moved = tree.path.join("owned-complete-moved.zip");
    fs::copy(&complete, &moved).unwrap();
    let ui = id("owned-zip-fresh-ui");
    let mut portable = Harness {
        host: ProjectHost::new(id("owned-zip-fresh-host"), ui.clone()),
        ui,
        tree,
    };
    let token = portable
        .host
        .select_file(&portable.ui, FilePurpose::ImportPackage, moved)
        .unwrap();
    let task: TaskHandle = ok(portable.call_as(
        &portable.ui.clone(),
        false,
        "import_package",
        json!({"selected_file_ref":token,"import_policy":"open_project","base_revision":null}),
    ));
    portable.finish_one();
    let draft = portable.imported(&task);
    assert!(
        !draft
            .missing
            .iter()
            .any(|i| i.code == "handoff.asset_not_redistributed")
    );
    let _: EditCommit = ok(portable.adopt(&task, &draft, false));
    assert_eq!(
        serde_json::to_value(portable.full(&old.id)).unwrap(),
        old_value
    );
    assert_eq!(
        serde_json::to_value(portable.full(&new.id)).unwrap(),
        new_value
    );
    assert_eq!(
        portable.object("project", "P1")["bindings"],
        current["bindings"]
    );
    for result in [&first, &second] {
        let revision = portable.context().project.unwrap().revision;
        let (_, actual, _) = portable
            .host
            .read_stored_packaging_artifact(
                &portable.ui,
                &"P1".to_owned().try_into().unwrap(),
                &ApiScope::Project,
                &revision,
                &result.artifact_id,
            )
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(result).unwrap()
        );
    }
    let persisted = root().join("scenarios/集成验收/runtime-host/ix03-report");
    fs::create_dir_all(&persisted).unwrap();
    preserve_tree(&dir, &persisted);
    fs::copy(
        archive,
        root().join("scenarios/集成验收/runtime-host/ix03-handoff.zip"),
    )
    .unwrap();
    fs::copy(
        complete,
        root().join("scenarios/集成验收/runtime-host/ix03-complete-owned.zip"),
    )
    .unwrap();
    evidence(
        "ix03-lifecycle.json",
        &json!({"first_result":first,"second_result":second,"old_run":old_value,"new_run":new_value,"bindings":current["bindings"],"acknowledgments":current["acknowledgments"],"comparison":comparison,"report":output,"handoff_manifest":handoff_manifest,"owned_output_complete_manifest":complete_manifest,"report_project_fresh_reopen_complete":true,"zip_fresh_reopen_full_equal":true,"unconfirmed_zip_professional_complete":false,"explicit_owned_confirmed_zip_professional_complete":true,"actual_human_acceptance":false,"actual_renderer_frame":false}),
    );
}
