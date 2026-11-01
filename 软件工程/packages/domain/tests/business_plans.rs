use sf_domain::{
    AllocationMode, ObjectId, Revision, SafeUint, ScenarioId, ScenarioInput,
    fork_without_unstarted_demands,
};

fn id(value: &str) -> ObjectId {
    ObjectId::try_from(value.to_owned()).unwrap()
}

fn scenario() -> ScenarioInput {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/business-inputs/initial-stock.scenario.json"
    ))
    .unwrap()
}

fn fork(
    source: &ScenarioInput,
    orders: &[&str],
    tasks: &[&str],
) -> Result<ScenarioInput, Vec<sf_domain::Issue>> {
    fork_without_unstarted_demands(
        source,
        ScenarioId::try_from("REPLANNED".to_owned()).unwrap(),
        Revision(SafeUint::try_from(17).unwrap()),
        &orders.iter().map(|x| id(x)).collect::<Vec<_>>(),
        &tasks.iter().map(|x| id(x)).collect::<Vec<_>>(),
    )
}

#[test]
fn order_cancel_forks_from_initial_input_and_releases_only_its_own_plans() {
    let mut source = scenario();
    source.revision = Revision(SafeUint::try_from(9).unwrap());
    let before = serde_json::to_value(&source).unwrap();
    let next = fork(&source, &["ORDER-A"], &[]).unwrap();
    let value = serde_json::to_value(&next).unwrap();
    let mut expected = before.clone();
    expected["id"] = serde_json::json!("REPLANNED");
    expected["revision"] = serde_json::json!(1);
    expected["project_revision"] = serde_json::json!(17);
    expected["orders"] = serde_json::json!([before["orders"][1]]);
    expected["production_tasks"] = serde_json::json!([before["production_tasks"][1]]);
    expected["initial_conditions"]["material_allocations"] = serde_json::json!([
        before["initial_conditions"]["material_allocations"][1],
        before["initial_conditions"]["material_allocations"][3]
    ]);
    assert_eq!(value, expected);
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
    // All ten physical units remain initial input; six remain reserved for B.
    assert_eq!(
        next.initial_conditions.items[0]
            .quantity
            .value
            .coefficient
            .value(),
        10
    );
    assert_eq!(
        next.initial_conditions.material_allocations[0]
            .quantity
            .value
            .coefficient
            .value(),
        6
    );
    assert_eq!(
        next.orders[0].product_ref.revision,
        source.orders[1].product_ref.revision
    );
}

#[test]
fn explicit_task_cancel_keeps_order_demand_and_other_task_reservations() {
    let source = scenario();
    let before = serde_json::to_value(&source).unwrap();
    let next = fork(&source, &[], &["TASK-A"]).unwrap();
    assert_eq!(
        serde_json::to_value(&next.orders).unwrap(),
        before["orders"]
    );
    assert_eq!(next.production_tasks.len(), 1);
    assert_eq!(next.production_tasks[0].id, id("TASK-B"));
    assert_eq!(next.initial_conditions.material_allocations.len(), 3);
    assert!(
        next.initial_conditions
            .material_allocations
            .iter()
            .all(|a| a.id != id("HARD-A"))
    );
    assert!(
        next.initial_conditions
            .material_allocations
            .iter()
            .any(|a| a.id == id("SOFT-FUTURE"))
    );
    assert_eq!(
        serde_json::to_value(&next.initial_conditions.items).unwrap(),
        before["initial_conditions"]["items"]
    );
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
}

#[test]
fn partially_cancelled_shared_task_requires_explicit_material_replan() {
    let mut source = scenario();
    source.orders[1].product_ref = source.orders[0].product_ref.clone();
    source.production_tasks[0]
        .planned_quantity
        .value
        .coefficient = SafeUint::try_from(8).unwrap();
    source.orders[1]
        .quantity
        .as_mut()
        .unwrap()
        .value
        .coefficient = SafeUint::try_from(8).unwrap();
    let allocation = source.production_tasks[1].order_allocations[0].clone();
    source.production_tasks[0]
        .order_allocations
        .push(allocation);
    let before = serde_json::to_value(&source).unwrap();
    let errors = fork(&source, &["ORDER-A"], &[]).unwrap_err();
    let error = errors
        .iter()
        .find(|e| e.code == "business_plans.shared_task_requires_replan")
        .unwrap();
    assert_eq!(
        error.field_path.as_deref(),
        Some("/production_tasks/0/order_allocations")
    );
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
    // Cancelling both orders removes the complete shared task; no ratio is inferred.
    let next = fork(&source, &["ORDER-A", "ORDER-B"], &[]).unwrap();
    assert!(next.orders.is_empty() && next.production_tasks.is_empty());
    assert!(next.initial_conditions.material_allocations.is_empty());
    assert_eq!(
        serde_json::to_value(&next.initial_conditions.items).unwrap(),
        before["initial_conditions"]["items"]
    );

    let mut source = scenario();
    source.production_tasks[0]
        .planned_quantity
        .value
        .coefficient = SafeUint::try_from(5).unwrap();
    let before = serde_json::to_value(&source).unwrap();
    let errors = fork(&source, &["ORDER-A"], &[]).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.unassigned_task_requires_replan")
    );
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
}

#[test]
fn committed_material_and_ambiguous_soft_demand_are_not_silently_released() {
    let mut source = scenario();
    source.initial_conditions.material_allocations[0].mode = AllocationMode::Committed;
    let before = serde_json::to_value(&source).unwrap();
    let errors = fork(&source, &[], &["TASK-A"]).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.committed_not_cancellable")
    );
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
    source = scenario();
    source.production_tasks[0].id = id("ORDER-A");
    let before = serde_json::to_value(&source).unwrap();
    let errors = fork(&source, &["ORDER-A"], &[]).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.ambiguous_soft_demand")
    );
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
}

#[test]
fn source_identity_and_unknown_or_ambiguous_targets_do_not_produce_a_candidate() {
    let mut source = scenario();
    let errors = fork_without_unstarted_demands(
        &source,
        source.id.clone(),
        source.project_revision.clone(),
        &[],
        &[],
    )
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.new_scenario_required")
    );
    let errors = fork(&source, &["missing"], &["missing-task"]).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.order_not_unique")
    );
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.task_not_unique")
    );
    source.orders.push(source.orders[0].clone());
    source
        .production_tasks
        .push(source.production_tasks[0].clone());
    let errors = fork(&source, &["ORDER-A"], &["TASK-A"]).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.order_not_unique")
    );
    assert!(
        errors
            .iter()
            .any(|e| e.code == "business_plans.task_not_unique")
    );
    assert!(
        errors
            .iter()
            .all(|e| e.object_ref.as_ref().unwrap().id.as_str() == source.id.as_str())
    );
}

#[test]
fn legacy_target_demand_is_preserved_without_fabricating_an_order() {
    let mut source = scenario();
    source.orders.clear();
    source.production_tasks.clear();
    source.initial_conditions.material_allocations.clear();
    let before = serde_json::to_value(&source).unwrap();
    let next = fork(&source, &[], &[]).unwrap();
    let mut expected = before.clone();
    expected["id"] = serde_json::json!("REPLANNED");
    expected["revision"] = serde_json::json!(1);
    expected["project_revision"] = serde_json::json!(17);
    assert_eq!(serde_json::to_value(&next).unwrap(), expected);
    assert_eq!(serde_json::to_value(&source).unwrap(), before);
}
