//! Actual public Host/worker/store consumers over original P1 input.
use super::{host, price};
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn fixture(demand: u64) -> Value {
    let mut v = host::fixture();
    v["scenario"]["orders"][0]["quantity"]["value"]["coefficient"] = json!(demand);
    v["scenario"]["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(1);
    v["scenario"]["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(1);
    v["scenario"]["cost_inputs"] = json!([price(
        "frozen-carton",
        "item_type",
        "carton",
        "consumed_material",
        135000,
        7,
        "piece"
    )]);
    v
}
fn fonts() -> sf_reporting::EvaluationResources {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/architecture-proof/assets");
    sf_reporting::EvaluationResources {
        cjk_font: Some(fs::read(root.join("SourceHanSansCN-Regular.otf")).unwrap()),
        font_license: Some(fs::read(root.join("LICENSE.SourceHanSans.txt")).unwrap()),
        files: BTreeMap::new(),
    }
}
fn single() -> (host::Harness, RunResult) {
    let mut h = host::Harness::from_value(fixture(1));
    h.host.set_evaluation_resources(fonts());
    let task = h.run("LOCAL");
    assert!(matches!(task.state, ComputationStatus::Queued));
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(
        run.acceptance_events
            .iter()
            .map(|e| e.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000]
    );
    (h, run)
}
fn params(
    h: &mut host::Harness,
    selection: &SelectedFileRef,
    mode: &str,
    runs: Value,
    scenarios: Value,
    formats: Value,
) -> Value {
    json!({"revision":h.context().project.unwrap().revision,"report_mode":mode,"scenario_refs":scenarios,"run_refs":runs,"metric_scope":{"keys":[],"objects":[],"acceptance":[]},"formats":formats,"selected_destination_ref":selection})
}
fn selected(h: &mut host::Harness, path: &Path) -> SelectedFileRef {
    h.host
        .select_directory(&h.ui, FilePurpose::ExportEvaluation, path.to_owned())
        .unwrap()
}
fn admit(
    h: &mut host::Harness,
    path: &Path,
    mode: &str,
    runs: Value,
    scenarios: Value,
    formats: Value,
) -> TaskHandle {
    let selection = selected(h, path);
    let input = params(h, &selection, mode, runs, scenarios, formats);
    host::ok(h.call("export_evaluation", input))
}
fn completed(h: &mut host::Harness, task: &TaskHandle) -> ExportEvaluation {
    let state = h.task(task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    assert!(matches!(
        state.originating_operation,
        OperationName::ExportEvaluation
    ));
    let TaskOutput::Evaluation(output) = state.result.unwrap() else {
        panic!("typed actual Evaluation TaskOutput required")
    };
    *output
}
fn output_folder(parent: &Path, output: &ExportEvaluation) -> PathBuf {
    let file = output
        .files
        .iter()
        .find(|file| matches!(&file.kind, DocumentKind::Evaluation))
        .unwrap();
    parent.join(file.path.as_str()).parent().unwrap().to_owned()
}
fn preserve(name: &str, parent: &Path, output: &ExportEvaluation) {
    if let Some(base) = std::env::var_os("W5_HOST_EVALUATION_OUTPUT") {
        let target = PathBuf::from(base).join(name);
        fs::create_dir_all(&target).unwrap();
        host::copy(&output_folder(parent, output), &target);
        host::write(
            &target.join("actual-task-output.json"),
            &serde_json::to_value(output).unwrap(),
        );
    }
}
fn bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                visit(root, &p, out)
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_owned(),
                    fs::read(p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}
fn rate(h: &mut host::Harness, n: Option<u64>) {
    let mut scenario = h.object("scenario", "LOCAL");
    scenario["cost_inputs"] = n
        .map(|n| {
            json!([price(
                "frozen-carton",
                "item_type",
                "carton",
                "consumed_material",
                n,
                7,
                "piece"
            )])
        })
        .unwrap_or_else(|| json!([]));
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = host::ok(h.apply(base, json!([host::put("scenario", scenario)])));
}
#[test]
fn actual_host_single_four_formats_freeze_admission_inputs_resources_and_portable_project_without_saving_current()
 {
    let (mut h, run) = single();
    let original = serde_json::to_value(&run).unwrap();
    let parent = h.tree.directory("actual-report");
    let task = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["pdf", "html", "csv", "json"]),
    );
    assert!(matches!(task.state, ComputationStatus::Queued));
    assert!(h.task(&task, true).result.is_none());
    h.host
        .set_evaluation_resources(sf_reporting::EvaluationResources::default());
    rate(&mut h, Some(270000));
    let before = h.object("project", "P1");
    let scenario = h.object("scenario", "LOCAL");
    let dirty = h.context().project.unwrap().dirty;
    let backing = bytes(&h.tree.path.join("seed"));
    h.finish_one();
    let output = completed(&mut h, &task);
    assert_eq!(output.context.project_revision.0.value(), 1);
    assert_eq!(output.context.run_refs.len(), 1);
    assert!(output.context.scenario_comparison.is_none());
    assert_eq!(
        output.context.run_evaluations[0].costs.lines[0]
            .amount
            .as_ref()
            .unwrap()
            .value(),
        135000.0 / 7.0
    );
    assert_eq!(
        output.context.run_evaluations[0].order_metrics[0]
            .completion_ms
            .as_ref()
            .unwrap()
            .0
            .value(),
        75000
    );
    let folder = output_folder(&parent, &output);
    assert!(
        fs::read(folder.join("report.pdf"))
            .unwrap()
            .starts_with(b"%PDF-")
    );
    let json = host::read(&folder.join("report.json"));
    assert_eq!(json["data"], serde_json::to_value(&output.context).unwrap());
    assert_eq!(fs::read_dir(folder.join("csv")).unwrap().count(), 15);
    assert!(folder.join("licenses/SourceHanSans-LICENSE.txt").is_file());
    assert!(folder.join("project/project.json").is_file());
    assert!(
        output
            .limitations
            .iter()
            .any(|i| i.code == "evaluation.actual_render_missing")
    );
    assert_eq!(h.object("project", "P1"), before);
    assert_eq!(h.object("scenario", "LOCAL"), scenario);
    assert_eq!(h.context().project.unwrap().dirty, dirty);
    assert_eq!(bytes(&h.tree.path.join("seed")), backing);
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), original);
    let _: EditCommit = host::ok(h.apply(2, json!([{"kind":"undo"}])));
    assert_eq!(
        h.object("scenario", "LOCAL")["cost_inputs"][0]["rate"]["coefficient"],
        135000
    );
    let _: EditCommit = host::ok(h.apply(3, json!([{"kind":"redo"}])));
    assert_eq!(
        h.object("scenario", "LOCAL")["cost_inputs"][0]["rate"]["coefficient"],
        270000
    );
    preserve("single", &parent, &output);
    let moved = h.tree.directory("moved-project");
    host::copy(&folder.join("project"), &moved);
    let tree = host::Sandbox::new();
    let mut reopened = host::Harness::from_seed(tree, &moved);
    assert_eq!(
        serde_json::to_value(reopened.full(&run.id)).unwrap(),
        original
    );
    assert_eq!(
        reopened.object("scenario", "LOCAL")["cost_inputs"][0]["rate"]["coefficient"],
        135000
    );
    let before = h.object("project", "P1");
    let invalid = h.import(&folder);
    h.finish_one();
    assert!(matches!(
        h.task(&invalid, false).task.state,
        ComputationStatus::Failed
    ));
    assert_eq!(
        h.object("project", "P1"),
        before,
        "report container/JSON must not become an active Project"
    );
}
#[test]
fn actual_host_comparison_keeps_each_frozen_rate_and_unknown_new_rate_then_reopens_exact_current_and_historical_scenarios()
 {
    let (mut h, a) = single();
    rate(&mut h, Some(270000));
    let task = h.run("LOCAL");
    h.finish_one();
    let b = h.completed_run(&task);
    rate(&mut h, None);
    let task = h.run("LOCAL");
    h.finish_one();
    let c = h.completed_run(&task);
    let originals = [&a, &b, &c].map(|r| serde_json::to_value(r).unwrap());
    let parent = h.tree.directory("comparison-report");
    let task = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([a.id, b.id, c.id]),
        json!([{"scenario_id":"LOCAL","revision":1},{"scenario_id":"LOCAL","revision":2},{"scenario_id":"LOCAL","revision":3}]),
        json!(["json", "csv", "html"]),
    );
    h.finish_one();
    let output = completed(&mut h, &task);
    assert_eq!(output.context.run_evaluations.len(), 3);
    for (id, expected) in [(&a.id, 135000.0 / 7.0), (&b.id, 270000.0 / 7.0)] {
        let r = output
            .context
            .run_evaluations
            .iter()
            .find(|r| &r.run_id == id)
            .unwrap();
        assert_eq!(r.costs.lines[0].amount.as_ref().unwrap().value(), expected);
    }
    let missing = output
        .context
        .run_evaluations
        .iter()
        .find(|r| r.run_id == c.id)
        .unwrap();
    assert!(missing.costs.inputs.is_empty());
    assert!(
        missing
            .costs
            .currencies
            .iter()
            .all(|v| v.complete_total.is_none() && v.unit_cost.is_none())
    );
    assert!(!missing.costs.missing_conditions.is_empty());
    preserve("comparison", &parent, &output);
    let folder = output_folder(&parent, &output);
    let moved = h.tree.directory("comparison-moved");
    host::copy(&folder.join("project"), &moved);
    let mut opened = host::Harness::from_seed(host::Sandbox::new(), &moved);
    assert!(
        opened.object("scenario", "LOCAL")["cost_inputs"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    for (i, run) in [&a, &b, &c].iter().enumerate() {
        assert_eq!(
            serde_json::to_value(opened.full(&run.id)).unwrap(),
            originals[i]
        );
    }
    let old:DomainObject=host::ok(opened.call("get_object",json!({"object_id":{"kind":"scenario","id":"LOCAL"},"source":{"kind":"run_snapshot","data":{"run_id":a.id,"at_ms":0}},"node_index":null})));
    assert_eq!(
        serde_json::to_value(old).unwrap()["data"]["cost_inputs"][0]["rate"]["coefficient"],
        135000
    );
}
#[test]
fn actual_host_incomplete_order_keeps_unfinished_completion_and_tardiness_unknown() {
    let mut h = host::Harness::from_value(fixture(2));
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events.len(), 1);
    let parent = h.tree.directory("incomplete-report");
    let task = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json", "csv", "html"]),
    );
    h.finish_one();
    let output = completed(&mut h, &task);
    let order = &output.context.run_evaluations[0].order_metrics[0];
    assert!(order.completion_ms.is_none());
    assert!(order.tardiness_ms.is_none());
    assert_eq!(order.outstanding.value.coefficient.value(), 1);
    preserve("incomplete", &parent, &output);
}
#[test]
fn actual_host_missing_font_early_cancel_and_late_cancel_preserve_existing_successful_directory() {
    let (mut h, run) = single();
    let parent = h.tree.directory("retained-report");
    let first = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json", "html", "csv"]),
    );
    h.finish_one();
    let _first = completed(&mut h, &first);
    let before = bytes(&parent);
    h.host
        .set_evaluation_resources(sf_reporting::EvaluationResources::default());
    let missing = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["pdf"]),
    );
    h.finish_one();
    let failed = h.task(&missing, true);
    assert!(matches!(failed.task.state, ComputationStatus::Failed));
    assert!(matches!(
        failed.failure.unwrap().code,
        ErrorCode::MissingReference
    ));
    assert_eq!(bytes(&parent), before);
    let cancelled = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    let _: CancellationState = host::ok(h.call(
        "cancel_task",
        json!({"task_id":cancelled.id,"action":"cancel"}),
    ));
    h.finish_one();
    assert!(matches!(
        h.task(&cancelled, true).task.state,
        ComputationStatus::Cancelled
    ));
    assert_eq!(bytes(&parent), before);
    let winner = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    let mut work = h.host.drain_io_work();
    assert_eq!(work.len(), 1);
    let completion = work.pop().unwrap().run();
    let _: CancellationState = host::ok(h.call(
        "cancel_task",
        json!({"task_id":winner.id,"action":"cancel"}),
    ));
    h.host.finish_io(completion);
    let winner = completed(&mut h, &winner);
    assert!(
        output_folder(&parent, &winner)
            .join("report.json")
            .is_file()
    );
    for (path, value) in before {
        assert_eq!(fs::read(parent.join(path)).unwrap(), value);
    }
}
#[test]
fn actual_host_existing_target_is_not_replaced_when_atomic_publication_loses() {
    let (mut h, run) = single();
    let parent = h.tree.directory("collision-parent");
    let task = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    // Fault injection follows this frozen worker's public Task ID ordinal.
    let ordinal: u64 = task
        .id
        .as_str()
        .rsplit(':')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let target = parent.join(format!("evaluation-{ordinal:08}"));
    fs::create_dir(&target).unwrap();
    fs::write(
        target.join("old-valid.txt"),
        b"existing user directory must survive",
    )
    .unwrap();
    let before = bytes(&parent);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Failed
    ));
    assert_eq!(bytes(&parent), before);
}
fn caller(session: &ObjectId, operations: Vec<OperationName>) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: session.clone(),
        granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
        allowed_operations: operations,
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![],
    }
}
#[test]
fn actual_host_rejects_stale_wrong_purpose_stolen_token_and_nonproject_scope_before_enqueuing() {
    let (mut h, run) = single();
    let parent = h.tree.directory("guarded-report");
    let selection = selected(&mut h, &parent);
    let original = h.object("project", "P1");
    let no_operation = host::id("w5-no-export-operation");
    h.host
        .register_external_session(caller(&no_operation, vec![OperationName::GetContext]))
        .unwrap();
    let mut denied = params(
        &mut h,
        &selection,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    denied["revision"] = json!(0);
    assert!(
        matches!(
            host::error(h.call_as(&no_operation, true, "export_evaluation", denied)).code,
            ErrorCode::PermissionDenied
        ),
        "operation authority must precede stale input details"
    );
    let mut input = params(
        &mut h,
        &selection,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    input["revision"] = json!(0);
    assert!(matches!(
        host::error(h.call("export_evaluation", input)).code,
        ErrorCode::RevisionConflict
    ));
    let wrong = h
        .host
        .select_directory(&h.ui, FilePurpose::SaveProject, parent.clone())
        .unwrap();
    let input = params(
        &mut h,
        &wrong,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    assert!(matches!(
        host::error(h.call("export_evaluation", input)).code,
        ErrorCode::PermissionDenied
    ));
    let agent = host::id("w5-report-agent");
    h.host
        .register_external_session(caller(&agent, vec![OperationName::ExportEvaluation]))
        .unwrap();
    let input = params(
        &mut h,
        &selection,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    assert!(matches!(
        host::error(h.call_as(&agent, true, "export_evaluation", input)).code,
        ErrorCode::PermissionDenied
    ));
    let input = params(
        &mut h,
        &selection,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"w5-definition-report-scope","project_id":null,"scope":{"kind":"definition_draft","data":"fixture.worker"},"operation":"export_evaluation","params":input})).unwrap();
    let _ = host::error(h.host.dispatch(&h.ui, request));
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(h.object("project", "P1"), original);
    assert!(bytes(&parent).is_empty());
}
#[test]
fn actual_host_external_token_is_owned_and_revocation_does_not_cancel_admitted_publication() {
    let (mut h, run) = single();
    let agent = host::id("w5-report-own-token");
    h.host
        .register_external_session(caller(
            &agent,
            vec![OperationName::ExportEvaluation, OperationName::GetTask],
        ))
        .unwrap();
    let parent = h.tree.directory("agent-report");
    let selection = h
        .host
        .select_directory(&agent, FilePurpose::ExportEvaluation, parent.clone())
        .unwrap();
    let input = params(
        &mut h,
        &selection,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    let task: TaskHandle = host::ok(h.call_as(&agent, true, "export_evaluation", input));
    let queued: TaskState =
        host::ok(h.call_as(&agent, true, "get_task", json!({"task_id":task.id})));
    assert!(matches!(queued.task.state, ComputationStatus::Queued));
    h.host.revoke_external_session(&agent).unwrap();
    h.finish_one();
    let denied = host::error(h.call_as(&agent, true, "get_task", json!({"task_id":task.id})));
    assert!(matches!(denied.code, ErrorCode::PermissionDenied));
    let ordinal: u64 = task
        .id
        .as_str()
        .rsplit(':')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let report = host::read(&parent.join(format!("evaluation-{ordinal:08}/report.json")));
    assert_eq!(report["data"]["run_refs"], json!([run.id]));
}
#[test]
fn actual_host_export_queue_is_fifo_and_each_admitted_draft_is_frozen() {
    let (mut h, run) = single();
    let parent = h.tree.directory("FIFO-reports");
    let first = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([{"scenario_id":"LOCAL","revision":1}]),
        json!(["json"]),
    );
    rate(&mut h, Some(270000));
    let second = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([{"scenario_id":"LOCAL","revision":2}]),
        json!(["json"]),
    );
    let mut work = h.host.drain_io_work();
    assert_eq!(work.len(), 2);
    h.host.finish_io(work.remove(0).run());
    let a = completed(&mut h, &first);
    assert!(matches!(
        h.task(&second, true).task.state,
        ComputationStatus::Queued
    ));
    h.host.finish_io(work.remove(0).run());
    let b = completed(&mut h, &second);
    assert_eq!(a.context.project_revision.0.value(), 1);
    assert_eq!(b.context.project_revision.0.value(), 2);
    for report in [&a, &b] {
        assert_eq!(
            report.context.run_evaluations[0].costs.lines[0]
                .amount
                .as_ref()
                .unwrap()
                .value(),
            135000.0 / 7.0
        );
    }
}
#[test]
fn actual_host_historical_design_scenario_is_reported_without_activating_it_in_portable_current_project()
 {
    let (mut h, run) = single();
    rate(&mut h, Some(270000));
    let parent = h.tree.directory("historical-design");
    let task = admit(
        &mut h,
        &parent,
        "design_description",
        json!([]),
        json!([{"scenario_id":"LOCAL","revision":1}]),
        json!(["json", "html", "csv"]),
    );
    h.finish_one();
    let output = completed(&mut h, &task);
    assert!(output.context.run_evaluations.is_empty());
    assert!(output.context.run_refs.is_empty());
    assert!(
        output
            .context
            .scenario_refs
            .iter()
            .any(|s| s.revision.0.value() == 1)
    );
    let copied = output_folder(&parent, &output).join("project");
    let mut opened = host::Harness::from_seed(host::Sandbox::new(), &copied);
    assert_eq!(
        opened.object("scenario", "LOCAL")["cost_inputs"][0]["rate"]["coefficient"],
        270000
    );
    assert_eq!(
        serde_json::to_value(opened.full(&run.id)).unwrap(),
        serde_json::to_value(run).unwrap()
    );
    preserve("historical-design", &parent, &output);
}
#[test]
fn actual_host_old_project_selection_cannot_authorize_export_in_a_real_new_project() {
    let (mut h, _) = single();
    let parent = h.tree.directory("old-project-token");
    let selection = selected(&mut h, &parent);
    let _:EditCommit=host::ok(h.call_as(&h.ui.clone(),false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"Different actual project","time_reference":{"label":"relative ms"},"departure":"discard_after_tasks_terminal"}}]})));
    let context: ProjectContext =
        host::ok(h.call_as(&h.ui.clone(), false, "get_context", json!({"selection":[]})));
    let project = context.project.unwrap();
    assert_ne!(project.id.as_str(), "P1");
    let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"w5-real-project-token-isolation","project_id":project.id,"scope":{"kind":"project"},"operation":"export_evaluation","params":{"revision":project.revision,"report_mode":"design_description","scenario_refs":[],"run_refs":[],"metric_scope":{"keys":[],"objects":[],"acceptance":[]},"formats":["json"],"selected_destination_ref":selection}})).unwrap();
    assert!(matches!(
        host::error(h.host.dispatch(&h.ui, request)).code,
        ErrorCode::PermissionDenied
    ));
    assert!(h.host.drain_io_work().is_empty());
    assert!(bytes(&parent).is_empty());
}

#[test]
fn actual_published_project_with_a_missing_real_module_is_rejected_before_another_report_can_publish()
 {
    let (mut h, run) = single();
    let parent = h.tree.directory("source-file-proof");
    let task = admit(
        &mut h,
        &parent,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    h.finish_one();
    let valid = completed(&mut h, &task);
    let bad = h.tree.directory("missing-module-project");
    host::copy(&output_folder(&parent, &valid).join("project"), &bad);
    let manifest = host::read(&bad.join("project.json"));
    let entry = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["role"] == "module_definition")
        .unwrap();
    let target = bad
        .join(entry["file_ref"]["path"].as_str().unwrap())
        .canonicalize()
        .unwrap();
    assert!(target.starts_with(bad.canonicalize().unwrap()));
    fs::remove_file(target).unwrap();
    let before = h.object("project", "P1");
    let imported = h.import(&bad);
    h.finish_one();
    let state = h.task(&imported, false);
    if matches!(state.task.state, ComputationStatus::Failed) {
        assert!(matches!(
            state.failure.unwrap().code,
            ErrorCode::MissingReference | ErrorCode::InvalidInput | ErrorCode::IoFailure
        ));
        assert_eq!(h.object("project", "P1"), before);
        return;
    }
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    let TaskOutput::Import(draft) = state.result.unwrap() else {
        panic!("actual partial import")
    };
    let response = h.adopt(&imported, &draft, true);
    if response["outcome"]["status"] == "error" {
        let failure = host::error(response);
        assert!(matches!(
            failure.code,
            ErrorCode::MissingReference | ErrorCode::InvalidInput
        ));
        assert_eq!(h.object("project", "P1"), before);
        return;
    }
    let _: EditCommit = host::ok(response);
    let destination = h.tree.directory("must-not-publish-missing-source");
    let selection = selected(&mut h, &destination);
    let input = params(
        &mut h,
        &selection,
        "simulation_evaluation",
        json!([run.id]),
        json!([]),
        json!(["json"]),
    );
    let failure = host::error(h.call("export_evaluation", input));
    assert!(matches!(
        failure.code,
        ErrorCode::MissingReference | ErrorCode::InvalidInput
    ));
    assert!(h.host.drain_io_work().is_empty());
    assert!(bytes(&destination).is_empty());
}

#[test]
fn actual_professional_report_summarizes_pose_history_and_preserves_full_referenced_results() {
    let seed = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/evaluation/r12-publication/saved-project");
    let original = bytes(&seed);
    let mut h = host::Harness::from_seed(host::Sandbox::new(), &seed);
    h.host.set_evaluation_resources(fonts());
    let run_id: RunId = serde_json::from_value(json!("whole-w5-host:run:1:5")).unwrap();
    let old_run = serde_json::to_value(h.full(&run_id)).unwrap();
    let acknowledgments = h.object("project", "P1")["acknowledgments"].clone();
    let parent = h.tree.directory("professional-readable-report");
    let task = admit(&mut h, &parent, "simulation_evaluation", json!([run_id]),
        json!([]), json!(["pdf", "html", "csv", "json"]));
    h.finish_one();
    let output = completed(&mut h, &task);
    let report_root = output_folder(&parent, &output);
    let conclusions = output.context.conclusions.join("\n");
    assert!(!conclusions.contains("\"process_poses\""),
        "The reading report must not dump all panel transforms into its conclusions");
    let professional: Value = serde_json::from_str(output.context.conclusions.iter()
        .find(|line| line.starts_with("保存的专业成果/"))
        .unwrap().split_once('：').unwrap().1).unwrap();
    for path in ["documents/packaging-result-00000002/result.json",
        "documents/packaging-result-00000005/result.json"] {
        let full = host::read(&seed.join(path));
        let result = &full["data"];
        assert!(!result["process_poses"].as_array().unwrap().is_empty());
        assert!(conclusions.contains(path), "Missing exact full-result reference: {path}");
        assert!(conclusions.contains(result["artifact_id"].as_str().unwrap()));
        let mut summary = professional["results"].as_array().unwrap().iter()
            .find(|summary| summary["artifact_id"] == result["artifact_id"])
            .unwrap().clone();
        let mut expected = result.clone();
        let poses = expected.as_object_mut().unwrap().remove("process_poses").unwrap();
        assert_eq!(summary.as_object_mut().unwrap().remove("process_pose_sample_count"),
            Some(json!(poses.as_array().unwrap().len())));
        assert_eq!(summary.as_object_mut().unwrap().remove("result_ref").unwrap()["path"], path);
        assert_eq!(summary, expected, "All professional facts except full pose history must remain exact");
        assert_eq!(fs::read(report_root.join("project").join(path)).unwrap(), original[Path::new(path)]);
    }
    assert_eq!(serde_json::to_value(h.full(&run_id)).unwrap(), old_run);
    assert_eq!(h.object("project", "P1")["acknowledgments"], acknowledgments);
    assert_eq!(bytes(&seed), original);
    preserve("professional-readable", &parent, &output);
}

// Controlled protocol pixels verify Host freezing/atomicity, not Native visuals.
struct ReportCaptureBackend {
    jobs: std::sync::Mutex<Vec<RenderJob>>,
    mode: &'static str,
}
impl RenderBackend for ReportCaptureBackend {
    fn supported_kinds(&self) -> Vec<RenderViewKind> {
        vec![RenderViewKind::MultiHallCanvas, RenderViewKind::TimelineSnapshot]
    }
    fn render(&self, job: RenderJob, cancelled: &std::sync::atomic::AtomicBool) -> Result<RenderedCapture, ApiError> {
        let mut jobs = self.jobs.lock().unwrap();
        let ordinal = jobs.len();
        jobs.push(job.clone());
        if self.mode == "fail" || self.mode == "second-fails" && ordinal == 1 {
            return Err(ApiError { code: ErrorCode::ComputationFailed, message: "controlled capture failed".into(), object_refs:vec![],field_errors:vec![],expected_revision:None,actual_revision:None });
        }
        if self.mode == "cancel" { cancelled.store(true,std::sync::atomic::Ordering::Release); }
        let mut visible_objects = vec![];
        if self.mode == "layout-schedule-object" {
            visible_objects.push(ObjectRef { kind: ObjectKind::Order, id: host::id("ORDER-01") });
        }
        if matches!(job.frame.request.view.kind, RenderViewKind::TimelineSnapshot) {
            if self.mode == "timeline-members" {
                for scenario in &job.input.scenarios {
                    visible_objects.extend(scenario.orders.iter().map(|v| ObjectRef { kind: ObjectKind::Order, id: v.id.clone() }));
                    visible_objects.extend(scenario.production_tasks.iter().map(|v| ObjectRef { kind: ObjectKind::Task, id: v.id.clone() }));
                }
                if let Some(run) = &job.input.run {
                    visible_objects.extend(run.events.iter().map(|event| event.subject.clone()));
                }
            } else if self.mode == "foreign-timeline" {
                visible_objects.push(ObjectRef { kind: ObjectKind::Order, id: host::id("foreign-order") });
            }
        }
        let mut frame = job.frame;
        if self.mode == "wrong-frame" { frame.request.view.camera.zoom = FiniteNumber::try_from(9.).unwrap(); }
        Ok(RenderedCapture { frame,scene_id:host::id(&format!("controlled-scene-{ordinal}")),image_id:AssetId::try_from(format!("controlled-image-{ordinal}")).unwrap(),mime_type:"image/png".into(),bytes:include_bytes!("../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png").to_vec(),visible_objects,issues:vec![] })
    }
}
fn capture_report_host(mode: &'static str) -> (host::Harness,std::sync::Arc<ReportCaptureBackend>) {
    let seed = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/evaluation/r12-publication/saved-project");
    let mut h = host::Harness::from_seed(host::Sandbox::new(), &seed);
    h.host.set_evaluation_resources(fonts());
    let backend=std::sync::Arc::new(ReportCaptureBackend{jobs:std::sync::Mutex::new(vec![]),mode});
    h.host.set_render_backend(backend.clone()); (h,backend)
}
#[test]
fn actual_report_captures_current_layout_and_each_exact_old_run_layout_and_timeline() {
    let (mut h,backend)=capture_report_host("timeline-members");
    let before=h.object("project","P1");
    let parent=h.tree.directory("report-captures");
    let task=admit(&mut h,&parent,"simulation_evaluation",json!(["whole-w5-host:run:1:3","whole-w5-host:run:1:5"]),json!([]),json!(["pdf","html","csv","json"]));
    let _:EditCommit=host::ok(h.apply(10,json!([{"kind":"set_project_settings","data":{
        "title":"edited after capture admission","time_reference":before["time_reference"],
        "generation_context":before["generation_context"],"sources":before["sources"]}}])));
    h.finish_one();let output=completed(&mut h,&task);
    assert_eq!(output.context.layout_and_schedule.len(),5);
    assert!(!output.limitations.iter().any(|i|i.code=="evaluation.actual_render_missing"));
    let jobs=backend.jobs.lock().unwrap();assert_eq!(jobs.len(),5);
    for job in jobs.iter() {
        assert_eq!(job.frame.task_id,task.id);
        assert_eq!(job.frame.scope_revision.0.value(),10);
        assert!(matches!(job.frame.request.delivery,RenderDelivery::CaptureOnly));
        assert_eq!(job.input.project.title,before["title"].as_str().unwrap());
        assert_eq!(job.frame.request.view.project_revision.0.value(),job.input.project.revision.0.value());
        assert_eq!(job.viewport_css,if matches!(job.frame.request.view.kind,RenderViewKind::TimelineSnapshot){(720,320)}else{(720,450)});
    }
    for (index,revision) in [(1,5),(3,10)] {
        assert_eq!(jobs[index].input.project.revision.0.value(),revision);
        assert_eq!(jobs[index].frame.request.view.view_time_ms.as_ref().unwrap().0.value(),240000);
        assert!(matches!(jobs[index+1].frame.request.view.kind,RenderViewKind::TimelineSnapshot));
        assert_eq!(jobs[index].frame.request.view.run_id,jobs[index+1].frame.request.view.run_id);
    }
    let root=output_folder(&parent,&output);
    for image in &output.context.layout_and_schedule {
        assert_eq!(image.scene_ref.scene_revision.0.value(),10);
        assert_eq!(fs::read(root.join(image.scene_ref.file.path.as_str())).unwrap(),include_bytes!("../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png"));
    }
    assert_eq!(output.context.layout_and_schedule[1].scene_ref.view.project_revision.0.value(),5);
    preserve("controlled-render-report",&parent,&output);
}
#[test]
fn actual_report_capture_wrong_frame_cancel_and_second_failure_never_publish_partial_directory() {
    for mode in ["wrong-frame","cancel","second-fails","foreign-timeline","layout-schedule-object"] {
        let (mut h,_)=capture_report_host(mode);
        let before=h.object("project","P1");
        let parent=h.tree.directory(mode);
        let task=admit(&mut h,&parent,"simulation_evaluation",json!(["whole-w5-host:run:1:5"]),json!([]),json!(["pdf","html","csv","json"]));
        h.finish_one();let state=h.task(&task,true);
        assert!(matches!(state.task.state,ComputationStatus::Failed|ComputationStatus::Cancelled),"{mode}: {:?}",state.task.state);
        assert!(state.result.is_none());assert!(bytes(&parent).is_empty());
        assert_eq!(h.object("project","P1"),before);
    }
}

#[test]
fn actual_design_report_captures_saved_input_without_inventing_a_run() {
    let (mut h,backend)=capture_report_host("timeline-members");
    let parent=h.tree.directory("design-captures");
    let task=admit(&mut h,&parent,"design_description",json!([]),json!([{"scenario_id":"LOCAL","revision":10}]),json!(["pdf","html","csv","json"]));
    h.finish_one();let output=completed(&mut h,&task);
    assert_eq!(output.context.layout_and_schedule.len(),2);
    assert!(output.context.run_refs.is_empty() && output.context.run_evaluations.is_empty());
    for job in backend.jobs.lock().unwrap().iter() {
        assert!(job.frame.request.view.run_id.is_none());
        assert!(job.frame.request.view.view_time_ms.is_none());
        assert!(job.input.run.is_none());
    }
    assert!(output.context.layout_and_schedule.iter().all(|image|matches!(image.evidence_kind,EvidenceKind::ActualPresentation)));
    preserve("controlled-design-captures",&parent,&output);
}

#[test]
fn report_only_agent_captures_privately_without_acquiring_public_render_or_view_control_rights() {
    let (mut h,backend)=capture_report_host("ok");
    let agent=host::id("report-only-agent");
    h.host.register_external_session(caller(&agent,vec![OperationName::ExportEvaluation,OperationName::GetTask])).unwrap();
    let parent=h.tree.directory("report-only-agent");
    let selection=h.host.select_directory(&agent,FilePurpose::ExportEvaluation,parent.clone()).unwrap();
    let input=params(&mut h,&selection,"simulation_evaluation",json!(["whole-w5-host:run:1:5"]),json!([]),json!(["json"]));
    let task:TaskHandle=host::ok(h.call_as(&agent,true,"export_evaluation",input));
    h.finish_one();let state:TaskState=host::ok(h.call_as(&agent,true,"get_task",json!({"task_id":task.id})));
    assert!(matches!(state.task.state,ComputationStatus::Completed));
    let public_request={let jobs=backend.jobs.lock().unwrap();
        assert_eq!(jobs.len(),3);
        assert!(jobs.iter().all(|job|!job.show_authorized.load(std::sync::atomic::Ordering::Acquire)));
        json!(jobs[0].frame.request)};
    assert!(matches!(host::error(h.call_as(&agent,true,"render_view",public_request)).code,ErrorCode::PermissionDenied));
}
