use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{Engine, EngineInput};
use std::collections::{BTreeMap, BTreeSet};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/multi_sink_goal.json")).unwrap()
}
fn input(value: Value) -> EngineInput {
    let scenario: ScenarioInput = serde_json::from_value(value["scenario"].clone()).unwrap();
    EngineInput {
        run_id: RunId::try_from("six-sink-unit-test".to_owned()).unwrap(),
        input_ref: serde_json::from_value(json!({"project_id":value["project"]["id"],"project_revision":value["project"]["revision"],"scenario_id":scenario.id,"scenario_revision":scenario.revision,
            "frozen_project_file":{"path":"unit/project.json","kind":"project"},"frozen_scenario_file":{"path":"unit/scenario.json","kind":"scenario"}})).unwrap(),
        project: serde_json::from_value(value["project"].clone()).unwrap(),
        definitions: serde_json::from_value(value["definitions"].clone()).unwrap(),
        scope: scenario.scope.clone(),
        scenario,
    }
}
fn execute(value: Value) -> RunResult {
    let mut engine = Engine::new(input(value)).unwrap_or_else(|issues| panic!("{issues:#?}"));
    engine.run_to_end();
    engine.snapshot()
}

#[test]
fn six_explicit_real_sinks_accept_same_twelve_identities_without_orders() {
    let value = fixture();
    let ids: BTreeSet<_> = value["scenario"]["initial_conditions"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_owned())
        .collect();
    let run = execute(value);
    assert!(matches!(
        run.computation_status,
        ComputationStatus::Completed
    ));
    assert_eq!(run.jobs.len(), 120);
    assert!(run.orders.is_empty());
    assert_eq!(run.goal_met, None);
    assert_eq!(run.final_items.len(), 12);
    assert_eq!(run.acceptance_events.len(), 12);
    let mut destinations = BTreeMap::new();
    let mut accepted = BTreeSet::new();
    for event in &run.acceptance_events {
        assert!(event.order_id.is_none());
        assert_eq!(event.quantity.value.coefficient.value(), 1);
        assert!(matches!(event.at_ms.0.value(), 20000 | 22000));
        *destinations.entry(event.destination.as_str()).or_insert(0) += 1;
        accepted.extend(event.item_refs.iter().map(|id| id.as_str().to_owned()));
    }
    assert_eq!(accepted, ids);
    assert_eq!(destinations.len(), 6);
    assert!(destinations.values().all(|n| *n == 2));
}

#[test]
fn primary_only_and_no_goal_never_infer_the_other_output_locations() {
    let mut value = fixture();
    value["scenario"]["acceptance_goal"]
        .as_object_mut()
        .unwrap()
        .remove("additional_acceptance_scopes");
    let run = execute(value.clone());
    assert_eq!(run.acceptance_events.len(), 2);
    assert!(
        run.acceptance_events
            .iter()
            .all(|e| e.destination.as_str() == "L1-SINK")
    );
    value["scenario"]["acceptance_goal"] = Value::Null;
    let run = execute(value);
    assert!(run.acceptance_events.is_empty());
    assert!(
        run.limitations
            .iter()
            .any(|i| i.code == "simulation.goal_acceptance_missing")
    );
}

#[test]
fn every_additional_scope_validates_exact_product_unit_destination_and_real_stage() {
    for field in ["product", "unit", "destination", "stage"] {
        let mut value = fixture();
        let scope = &mut value["scenario"]["acceptance_goal"]["additional_acceptance_scopes"][4];
        match field {
            "product" => scope["product_ref"]["revision"] = json!("absent"),
            "unit" => scope["unit"] = json!("box"),
            "destination" => scope["destination"] = json!("absent"),
            _ => scope["stage"] = json!("unloaded_and_released"),
        }
        let issues = Engine::new(input(value))
            .err()
            .expect("invalid explicit scope admitted");
        assert!(
            issues.iter().any(|i| i.field_path.as_deref().is_some_and(
                |path| path.starts_with("/acceptance_goal/additional_acceptance_scopes/4")
            )),
            "{field}: {issues:#?}"
        );
    }
}

#[test]
fn overlapping_explicit_scopes_cannot_count_the_same_quantity_twice() {
    let mut value = fixture();
    let primary = value["scenario"]["acceptance_goal"]["acceptance_scope"].clone();
    value["scenario"]["acceptance_goal"]["additional_acceptance_scopes"]
        .as_array_mut()
        .unwrap()
        .push(primary);
    let run = execute(value);
    assert_eq!(run.acceptance_events.len(), 12);
    assert_eq!(
        run.acceptance_events
            .iter()
            .flat_map(|e| e.item_refs.iter())
            .collect::<BTreeSet<_>>()
            .len(),
        12
    );
}

#[test]
fn old_goal_json_omits_empty_list_and_explicit_null_is_rejected() {
    let mut goal = fixture()["scenario"]["acceptance_goal"].clone();
    goal.as_object_mut()
        .unwrap()
        .remove("additional_acceptance_scopes");
    let typed: FactoryGenerationGoal = serde_json::from_value(goal.clone()).unwrap();
    assert_eq!(serde_json::to_value(&typed).unwrap(), goal);
    let mut empty = goal.clone();
    empty["additional_acceptance_scopes"] = json!([]);
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<FactoryGenerationGoal>(empty).unwrap())
            .unwrap(),
        goal
    );
    goal["additional_acceptance_scopes"] = Value::Null;
    assert!(serde_json::from_value::<FactoryGenerationGoal>(goal).is_err());
}

#[test]
fn differing_explicit_scope_sources_are_not_selected_by_precedence() {
    let value = fixture();
    let mut legacy = value["scenario"]["acceptance_goal"].clone();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("additional_acceptance_scopes");
    let goal = serde_json::from_value(legacy).unwrap();
    let issues = Engine::new_with_goal(input(value), goal)
        .err()
        .expect("conflicting goals admitted");
    assert!(
        issues
            .iter()
            .any(|i| i.code == "simulation.goal_source_conflict")
    );
}
