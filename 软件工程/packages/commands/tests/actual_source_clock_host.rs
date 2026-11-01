//! Replay the original Root Native declared source, through the real Host.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;

fn call(
    host: &mut ProjectHost,
    ui: &ObjectId,
    project: Option<&str>,
    scope: Value,
    operation: &str,
    params: Value,
) -> Value {
    host.dispatch(ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"actual-root-source","project_id":project,"scope":scope,"operation":operation,"params":params})).unwrap())
}
fn project_call(host: &mut ProjectHost, ui: &ObjectId, operation: &str, params: Value) -> Value {
    call(
        host,
        ui,
        Some("desktop-host-72052:project:1"),
        json!({"kind":"project"}),
        operation,
        params,
    )
}
fn object(host: &mut ProjectHost, ui: &ObjectId, kind: &str, id: &str) -> Value {
    let value: DomainObject = ok(project_call(
        host,
        ui,
        "get_object",
        json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"draft"},"node_index":null}),
    ));
    serde_json::to_value(value).unwrap()["data"].clone()
}
fn preview(
    host: &mut ProjectHost,
    ui: &ObjectId,
    scope: Value,
    reference: &ModuleRef,
    base: u64,
    choices: Value,
    seed: Value,
) -> Value {
    call(
        host,
        ui,
        Some("desktop-host-72052:project:1"),
        scope,
        "get_object",
        json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":base,"choices":choices,"seed":seed}},"node_index":null}),
    )
}
fn unwrap_preview(response: Value) -> TemplatePreview {
    let value: DomainObject = ok(response);
    let DomainObject::TemplatePreview(value) = value else {
        panic!("actual template preview")
    };
    *value
}
fn finish_run(host: &mut ProjectHost, ui: &ObjectId, scenario: &Value, base: u64) -> RunResult {
    let task: TaskHandle = ok(project_call(
        host,
        ui,
        "run_scenario",
        json!({"scenario_id":scenario["id"],"input_revision":base,"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}),
    ));
    let mut work = host.drain_io_work();
    assert_eq!(work.len(), 1);
    host.finish_io(work.pop().unwrap().run());
    let state: TaskState = ok(project_call(
        host,
        ui,
        "get_task",
        json!({"task_id":task.id}),
    ));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Run(summary)) = state.result else {
        panic!("actual run")
    };
    full(host, ui, &summary.run_id)
}
fn full(host: &mut ProjectHost, ui: &ObjectId, run: &RunId) -> RunResult {
    let view: RunView = ok(project_call(
        host,
        ui,
        "get_run",
        json!({"run_id":run,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(value) = view else {
        panic!("actual result")
    };
    *value
}
fn accepted(run: &RunResult) -> Vec<u64> {
    run.acceptance_events
        .iter()
        .map(|event| event.at_ms.0.value())
        .collect()
}

#[test]
fn original_root_source_labels_stay_exact_while_full_draft_local_seed_runs_without_implicit_adoption()
 {
    let root: Value = serde_json::from_str(include_str!(
        "../../../scenarios/自定义模块/runtime-local-library-source/root-original-source.json"
    ))
    .unwrap();
    let ui = id("actual-root-source-ui");
    let mut host = ProjectHost::new(id("desktop-host-72052"), ui.clone());
    host.register_bundled_template_directory(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../modules/templates/purchased-assembly/candidate"),
    )
    .unwrap();
    let created: EditCommit = ok(call(
        &mut host,
        &ui,
        None,
        json!({"kind":"workspace"}),
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":root["project"]["title"],"time_reference":root["project"]["time_reference"],"departure":"stay"}}]}),
    ));
    assert_eq!(json!(created.project_id), root["project"]["id"]);
    let reference: ModuleRef = serde_json::from_value(
        json!({"id":"community.template-purchased-assembly","version":"1.0.0"}),
    )
    .unwrap();
    let original = unwrap_preview(preview(
        &mut host,
        &ui,
        json!({"kind":"project"}),
        &reference,
        1,
        Value::Null,
        Value::Null,
    ));
    let resources: serde_json::Map<_, _> = original
        .resources
        .iter()
        .map(|r| (r.id.as_str().to_owned(), Value::Null))
        .collect();
    let seed = original
        .subset
        .adoption
        .as_ref()
        .unwrap()
        .deferred_seed
        .as_ref()
        .unwrap();
    let qualities: serde_json::Map<_, _> = seed
        .initial_stock
        .iter()
        .map(|item| (item.location_id.as_str().to_owned(), json!("released")))
        .collect();
    let prepared = unwrap_preview(preview(
        &mut host,
        &ui,
        json!({"kind":"project"}),
        &reference,
        1,
        json!({"source_project":original.source_project,"namespace":"root-native-context","resources":resources,"reuse_items":{},"include_example_product":true,"site_envelope":null}),
        json!({"scenario_id":"ROOT-NATIVE-CONTEXT","supply":true,"demand":true,"initial_quality":qualities,"adopt_declared_inspection_results":true}),
    ));
    let _: EditCommit = ok(project_call(
        &mut host,
        &ui,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"instantiate_template","data":prepared.command.unwrap()}]}),
    ));
    let source_project = object(&mut host, &ui, "project", "desktop-host-72052:project:1");
    let source_scenario = object(&mut host, &ui, "scenario", "ROOT-NATIVE-CONTEXT");
    let expected_project: ProjectState = serde_json::from_value(root["project"].clone()).unwrap();
    let expected_scenario: ScenarioInput =
        serde_json::from_value(root["scenario"].clone()).unwrap();
    assert_eq!(
        source_project,
        serde_json::to_value(expected_project).unwrap()
    );
    assert_eq!(
        source_scenario,
        serde_json::to_value(expected_scenario).unwrap()
    );
    assert_eq!(source_project["time_reference"]["label"], "工程相对时间");
    assert_eq!(source_scenario["time_reference"]["label"], "scenario_start");
    let old = finish_run(&mut host, &ui, &source_scenario, 2);
    assert_eq!(accepted(&old), vec![70000, 110000]);
    let created: EditCommit = ok(project_call(
        &mut host,
        &ui,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"create_definition_draft","data":root["definition"]}]}),
    ));
    let EditScope::DefinitionDraft(identity) = created.scope else {
        panic!("draft")
    };
    let scope = json!({"kind":"definition_draft","data":identity.id});
    // Captured adoption metadata is generated by Host; a fresh composition
    // submits the original selected objects/seed, not a forged prior receipt.
    let mut selected = root["subset"].clone();
    selected["adoption"] = Value::Null;
    let wrong_capture = error(call(
        &mut host,
        &ui,
        Some("desktop-host-72052:project:1"),
        scope.clone(),
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":2,"definition":root["definition"],"subset":selected,"source_scenario":{"scenario_id":"ROOT-NATIVE-CONTEXT","revision":2}}}]}),
    ));
    assert!(matches!(wrong_capture.code, ErrorCode::RevisionConflict));
    assert_eq!(wrong_capture.expected_revision.unwrap().0.value(), 2);
    assert_eq!(wrong_capture.actual_revision.unwrap().0.value(), 1);
    assert_eq!(
        object(&mut host, &ui, "project", "desktop-host-72052:project:1"),
        source_project
    );
    assert_eq!(
        object(&mut host, &ui, "scenario", "ROOT-NATIVE-CONTEXT"),
        source_scenario
    );
    let _: EditCommit = ok(call(
        &mut host,
        &ui,
        Some("desktop-host-72052:project:1"),
        scope.clone(),
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":2,"definition":root["definition"],"subset":selected,"source_scenario":{"scenario_id":"ROOT-NATIVE-CONTEXT","revision":1}}}]}),
    ));
    let local_ref: ModuleRef = serde_json::from_value(
        json!({"id":root["definition"]["id"],"version":root["definition"]["version"]}),
    )
    .unwrap();
    let captured = unwrap_preview(preview(
        &mut host,
        &ui,
        scope.clone(),
        &local_ref,
        2,
        Value::Null,
        Value::Null,
    ));
    assert_eq!(
        captured
            .source_scenario
            .as_ref()
            .unwrap()
            .revision
            .0
            .value(),
        1
    );
    assert_eq!(
        serde_json::to_value(captured.subset.seed.as_ref().unwrap()).unwrap(),
        root["subset"]["seed"]
    );
    let resources: serde_json::Map<_, _> = captured
        .resources
        .iter()
        .map(|r| (r.id.as_str().to_owned(), json!(r.id)))
        .collect();
    let qualities: serde_json::Map<_, _> = source_scenario["initial_conditions"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["id"].as_str().unwrap().to_owned(),
                item["quality"].clone(),
            )
        })
        .collect();
    let choices = json!({"source_project":captured.source_project,"namespace":"root-clock-consumer","resources":resources,"reuse_items":{},"include_example_product":true,"site_envelope":null});
    let seed = json!({"scenario_id":"ROOT-CLOCK-ADOPTED","supply":true,"demand":true,"initial_quality":qualities,"adopt_declared_inspection_results":true});
    let draft_prepared = unwrap_preview(preview(
        &mut host,
        &ui,
        scope.clone(),
        &local_ref,
        2,
        choices.clone(),
        seed.clone(),
    ));
    assert_eq!(
        serde_json::to_value(
            &draft_prepared
                .command
                .as_ref()
                .unwrap()
                .explicit_new_scenario_seed
                .as_ref()
                .unwrap()
                .time_reference
        )
        .unwrap(),
        source_scenario["time_reference"]
    );
    let _: EditCommit = ok(call(
        &mut host,
        &ui,
        Some("desktop-host-72052:project:1"),
        scope,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"commit_definition_version","data":{"version":"0.2.0"}}]}),
    ));
    let reference: ModuleRef =
        serde_json::from_value(json!({"id":local_ref.id,"version":"0.2.0"})).unwrap();
    let captured = unwrap_preview(preview(
        &mut host,
        &ui,
        json!({"kind":"project"}),
        &reference,
        2,
        Value::Null,
        Value::Null,
    ));
    let mut choices = choices;
    choices["source_project"] = json!(captured.source_project);
    let prepared = unwrap_preview(preview(
        &mut host,
        &ui,
        json!({"kind":"project"}),
        &reference,
        2,
        choices.clone(),
        seed.clone(),
    ));
    assert_eq!(
        object(&mut host, &ui, "project", "desktop-host-72052:project:1"),
        source_project
    );
    let command = prepared.command.unwrap();
    for (field, value) in [
        ("origin_iso8601", json!("2026-10-16T00:00:00Z")),
        ("timezone", json!("UTC")),
    ] {
        let mut forged = serde_json::to_value(&command).unwrap();
        forged["explicit_new_scenario_seed"]["time_reference"][field] = value;
        let failure = error(project_call(
            &mut host,
            &ui,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"instantiate_template","data":forged}]}),
        ));
        assert!(matches!(failure.code, ErrorCode::InvalidInput));
        assert_eq!(
            object(&mut host, &ui, "project", "desktop-host-72052:project:1"),
            source_project
        );
    }
    let _: EditCommit = ok(project_call(
        &mut host,
        &ui,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"instantiate_template","data":command}]}),
    ));
    let adopted = object(&mut host, &ui, "scenario", "ROOT-CLOCK-ADOPTED");
    assert_eq!(adopted["time_reference"], source_scenario["time_reference"]);
    assert_eq!(
        accepted(&finish_run(&mut host, &ui, &adopted, 3)),
        vec![70000, 110000]
    );
    assert_eq!(
        serde_json::to_value(full(&mut host, &ui, &old.id)).unwrap(),
        serde_json::to_value(old).unwrap()
    );
    let retained = object(&mut host, &ui, "scenario", "ROOT-NATIVE-CONTEXT");
    // The current Scenario is rebound by the established Project revision
    // policy. The committed template and old Run keep their frozen Scenario R1.
    assert_eq!(retained["revision"], 2);
    assert_eq!(retained["project_revision"], 3);
    for field in [
        "id",
        "time_reference",
        "initial_conditions",
        "orders",
        "arrivals",
        "production_tasks",
        "quality_presets",
        "window",
        "scope",
        "supplies",
    ] {
        assert_eq!(retained[field], source_scenario[field], "source {field}");
    }
    let frozen_source = unwrap_preview(preview(
        &mut host,
        &ui,
        json!({"kind":"project"}),
        &reference,
        3,
        Value::Null,
        Value::Null,
    ));
    assert_eq!(
        json!(frozen_source.source_scenario),
        json!({"scenario_id":"ROOT-NATIVE-CONTEXT","revision":1})
    );
    assert_eq!(
        serde_json::to_value(frozen_source.subset.seed.unwrap()).unwrap(),
        root["subset"]["seed"]
    );
    assert_eq!(
        object(&mut host, &ui, "project", "desktop-host-72052:project:1")["time_reference"],
        source_project["time_reference"]
    );
}
