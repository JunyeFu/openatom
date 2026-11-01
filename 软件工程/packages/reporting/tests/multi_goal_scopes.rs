//! Finite reporting counterexamples derived from frozen actual Engine inputs.
//! These mutations are not additional Engine executions or NFR measurements.
use serde_json::{Value, json};
use sf_domain::*;
use sf_reporting::compare_runs;

type Frozen = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);

fn inputs() -> Vec<Frozen> {
    let evidence: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/acceptance-comparison/actual-engine-inputs.json"
    ))
    .unwrap();
    ["quality_a", "quality_same"]
        .iter()
        .map(|key| serde_json::from_value(evidence[*key].clone()).unwrap())
        .collect()
}

fn two_sinks() -> Vec<Frozen> {
    let mut inputs = inputs();
    for input in &mut inputs {
        let goal = input.1.acceptance_goal.as_mut().unwrap();
        let mut extra = goal.acceptance_scope.clone();
        let mut location = input
            .0
            .locations
            .iter()
            .find(|l| l.id == extra.destination)
            .unwrap()
            .clone();
        location.id = ObjectId::try_from("extra-sink".to_owned()).unwrap();
        extra.destination = location.id.clone();
        input.0.locations.push(location);
        let event = &mut input.3.acceptance_events[1];
        event.destination = extra.destination.clone();
        for item in &mut input.3.final_items {
            if event.item_refs.contains(&item.id) {
                item.holder = HolderRef::Location(extra.destination.clone());
            }
        }
        goal.additional_acceptance_scopes.push(extra);
    }
    inputs
}

fn compare(inputs: &[Frozen], metric: MetricKey) -> RunComparison {
    compare_runs(inputs, &[metric], &[], &[]).unwrap()
}

#[test]
fn each_distinct_sink_uses_only_its_actual_acceptance_events() {
    let inputs = two_sinks();
    let result = compare(&inputs, MetricKey::AcceptedQuantity);
    assert_eq!(result.metrics.len(), 2);
    for metric in &result.metrics {
        assert!(metric.acceptance_scope.is_some());
        for value in &metric.values {
            assert_eq!(
                serde_json::to_value(&value.value).unwrap(),
                json!({"kind":"quantity","data":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}})
            );
        }
    }
    assert!(inputs.iter().all(|i| i.1.orders.is_empty()));
}

#[test]
fn changing_an_extra_scope_prevents_a_comparable_improvement_claim() {
    let mut inputs = two_sinks();
    let mut other = inputs[0].0.locations.last().unwrap().clone();
    other.id = ObjectId::try_from("other-sink".to_owned()).unwrap();
    for input in &mut inputs {
        input.0.locations.push(other.clone());
    }
    inputs[1]
        .1
        .acceptance_goal
        .as_mut()
        .unwrap()
        .additional_acceptance_scopes[0]
        .destination = other.id;
    let result = compare(&inputs, MetricKey::AcceptedQuantity);
    assert!(matches!(
        result.pairs[0].status,
        ComparisonStatus::SideBySideOnly
    ));
    assert!(result.pairs[0].conditions.iter().any(|condition| matches!(
        condition.key,
        ComparisonConditionKey::AcceptanceScope
    ) && !condition.matches));
    assert!(
        result
            .metrics
            .iter()
            .flat_map(|m| &m.pair_differences)
            .all(|d| !d.comparable && d.difference.is_none())
    );
}

#[test]
fn a_shared_target_does_not_invent_per_sink_completion_quotas() {
    let mut inputs = two_sinks();
    for input in &mut inputs {
        input
            .1
            .acceptance_goal
            .as_mut()
            .unwrap()
            .accepted_quantity_per_day
            .as_mut()
            .unwrap()
            .value
            .coefficient = SafeUint::try_from(1u64).unwrap();
    }
    let result = compare(&inputs, MetricKey::CompletionTime);
    assert_eq!(result.metrics.len(), 2);
    for value in result.metrics.iter().flat_map(|m| &m.values) {
        let MetricValue::Unknown(issue) = &value.value else {
            panic!("invented per-sink quota")
        };
        assert!(issue.message.contains("per-scope allocation"));
    }
}

#[test]
fn overlapping_products_without_event_product_identity_are_unknown() {
    let mut inputs = inputs();
    for input in &mut inputs {
        let goal = input.1.acceptance_goal.as_mut().unwrap();
        let mut extra = goal.acceptance_scope.clone();
        let mut product = input
            .0
            .products
            .iter()
            .find(|p| p.id == extra.product_ref.id)
            .unwrap()
            .clone();
        product.id = ObjectId::try_from("other-product".to_owned()).unwrap();
        extra.product_ref.id = product.id.clone();
        input.0.products.push(product);
        goal.additional_acceptance_scopes.push(extra);
    }
    let result = compare(&inputs, MetricKey::AcceptedQuantity);
    assert_eq!(result.metrics.len(), 2);
    for value in result.metrics.iter().flat_map(|m| &m.values) {
        let MetricValue::Unknown(issue) = &value.value else {
            panic!("ambiguous product attribution")
        };
        assert!(issue.message.contains("no product identity"));
    }
}

#[test]
fn duplicate_scopes_preserve_single_scope_quantity_and_completion() {
    let mut inputs = inputs();
    for input in &mut inputs {
        let goal = input.1.acceptance_goal.as_mut().unwrap();
        goal.accepted_quantity_per_day
            .as_mut()
            .unwrap()
            .value
            .coefficient = SafeUint::try_from(1u64).unwrap();
        goal.additional_acceptance_scopes
            .push(goal.acceptance_scope.clone());
    }
    let quantity = compare(&inputs, MetricKey::AcceptedQuantity);
    assert_eq!(quantity.metrics.len(), 1);
    assert_eq!(
        serde_json::to_value(&quantity.metrics[0].values[0].value).unwrap(),
        json!({"kind":"quantity","data":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"}})
    );
    let time = compare(&inputs, MetricKey::CompletionTime);
    assert_eq!(time.metrics.len(), 1);
    assert_eq!(
        serde_json::to_value(&time.metrics[0].values[0].value).unwrap(),
        json!({"kind":"duration","data":40000})
    );
}
