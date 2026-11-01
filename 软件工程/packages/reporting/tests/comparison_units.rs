//! Numeric/condition counterexamples derived from the actual Host fixture.
//! Mutated cases below are finite type fixtures, not additional real executions.
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
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/comparison/actual-frozen-inputs.json"
    ))
    .unwrap()
}
fn compare(v: &[Frozen], keys: &[MetricKey]) -> RunComparison {
    compare_runs(v, keys, &[], &[]).unwrap()
}
fn accepted(c: &RunComparison) -> &MetricComparison {
    c.metrics
        .iter()
        .find(|m| matches!(m.metric, MetricKey::AcceptedQuantity))
        .unwrap()
}
fn v<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap()
}
#[test]
fn actual_fixture_is_controlled_change_and_unfinished_completion_is_unknown() {
    let c = compare(
        &inputs(),
        &[MetricKey::AcceptedQuantity, MetricKey::CompletionTime],
    );
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::ControlledChanges
    ));
    assert!(
        c.pairs[0]
            .input_changes
            .iter()
            .any(|change| change.path.contains("arrivals"))
    );
    assert_eq!(
        v(&accepted(&c).pair_differences[0].difference),
        json!({"kind":"quantity","data":{"negative":false,"magnitude":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}}})
    );
    let time = c
        .metrics
        .iter()
        .find(|m| matches!(m.metric, MetricKey::CompletionTime))
        .unwrap();
    assert!(matches!(time.values[0].value, MetricValue::Unknown(_)));
    assert_eq!(
        v(&time.values[1].value),
        json!({"kind":"duration","data":165000})
    );
}
#[test]
fn title_camera_and_run_identity_do_not_create_fake_business_changes() {
    let mut i = inputs();
    i[1] = i[0].clone();
    i[1].3.id = RunId::try_from("title-only-run".to_owned()).unwrap();
    i[1].0.title = "different display label".into();
    i[1].1.title = "renamed scenario".into();
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(matches!(c.pairs[0].status, ComparisonStatus::Strict));
    assert!(c.pairs[0].input_changes.is_empty());
}
#[test]
fn different_windows_are_not_an_improvement_claim() {
    let mut i = inputs();
    i[1].3.window.end_ms = SimTimeMs(SafeUint::try_from(180000u64).unwrap());
    i[1].3.computed_until_ms = Some(i[1].3.window.end_ms.clone());
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::SideBySideOnly
    ));
    assert!(!accepted(&c).pair_differences[0].comparable);
    assert!(accepted(&c).pair_differences[0].difference.is_none());
}
#[test]
fn different_demand_is_side_by_side_and_lists_the_actual_change() {
    let mut i = inputs();
    i[1].1.orders[0]
        .quantity
        .as_mut()
        .unwrap()
        .value
        .coefficient = SafeUint::try_from(3u64).unwrap();
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::SideBySideOnly
    ));
    assert!(
        c.pairs[0]
            .input_changes
            .iter()
            .any(|x| x.path.contains("orders"))
    );
}
#[test]
fn prefix_is_known_but_never_promoted_to_a_full_window_comparison() {
    let mut i = inputs();
    i[1].3.computed_until_ms = Some(SimTimeMs(SafeUint::try_from(75000u64).unwrap()));
    i[1].3.whole_window_computed = false;
    i[1].3.computation_status = ComputationStatus::Paused;
    i[1].3.acceptance_events.truncate(1);
    i[1].3.orders[0].completion_ms = None;
    i[1].3.orders[0].outstanding.value.coefficient = SafeUint::try_from(1u64).unwrap();
    let c = compare(
        &i,
        &[MetricKey::AcceptedQuantity, MetricKey::CompletionTime],
    );
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::SideBySideOnly
    ));
    assert_eq!(
        v(&accepted(&c).values[1].value),
        json!({"kind":"quantity","data":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}})
    );
    assert!(
        c.metrics
            .iter()
            .find(|m| matches!(m.metric, MetricKey::CompletionTime))
            .unwrap()
            .values
            .iter()
            .all(|m| matches!(m.value, MetricValue::Unknown(_)))
    );
}
#[test]
fn price_or_missing_cost_inputs_do_not_invent_actual_unit_cost() {
    let c = compare(&inputs(), &[MetricKey::UnitCost]);
    assert!(
        c.metrics[0]
            .values
            .iter()
            .all(|m| matches!(m.value, MetricValue::Unknown(_)))
    );
    assert!(c.metrics[0].pair_differences[0].difference.is_none());
}
#[test]
fn zero_baseline_has_exact_delta_but_no_invented_percent() {
    let mut i = inputs();
    i[0].3.acceptance_events.clear();
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    let d = &accepted(&c).pair_differences[0];
    assert!(d.comparable);
    assert!(d.change_percent.is_none());
    assert_eq!(
        v(&d.difference),
        json!({"kind":"quantity","data":{"negative":false,"magnitude":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"}}})
    );
}
#[test]
fn decimal_quantities_keep_exact_base_units_and_sign() {
    let mut i = inputs();
    i[0].3.acceptance_events[0].quantity.value = DecimalAmount {
        coefficient: SafeUint::try_from(23u64).unwrap(),
        decimal_places: DecimalPlaces::try_from(1).unwrap(),
    };
    i[1].3.acceptance_events.truncate(1);
    i[1].3.acceptance_events[0].quantity.value = DecimalAmount {
        coefficient: SafeUint::try_from(1u64).unwrap(),
        decimal_places: DecimalPlaces::try_from(1).unwrap(),
    };
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    let d = &accepted(&c).pair_differences[0];
    assert_eq!(
        v(&d.difference),
        json!({"kind":"quantity","data":{"negative":true,"magnitude":{"value":{"coefficient":22,"decimal_places":1},"unit":"piece"}}})
    );
}
#[test]
fn duplicate_run_and_wrong_frozen_identity_are_rejected() {
    let i = inputs();
    let mut bad = i.clone();
    bad[1].3.id = bad[0].3.id.clone();
    assert!(compare_runs(&bad, &[], &[], &[]).is_err());
    let mut bad = i;
    bad[1].3.input_ref.project_revision = Revision(SafeUint::try_from(900u64).unwrap());
    assert!(compare_runs(&bad, &[], &[], &[]).is_err());
}
#[test]
fn same_id_version_conflict_is_not_hidden_by_another_version() {
    let mut i = inputs();
    let mut unused = i[0].2[0].clone();
    unused.version = VersionId::try_from("9.0.0".to_owned()).unwrap();
    i[0].2.push(unused.clone());
    i[1].2.push(unused);
    i[1].2[0].title = "conflicting unchanged version".into();
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::SideBySideOnly
    ));
    assert!(
        c.pairs[0]
            .input_changes
            .iter()
            .any(|x| x.path.contains("definitions"))
    );
}
#[test]
fn unknown_object_wait_is_unknown_instead_of_zero() {
    let object = ObjectRef {
        kind: ObjectKind::Device,
        id: ObjectId::try_from("not-in-factory".to_owned()).unwrap(),
    };
    let c = compare_runs(&inputs(), &[MetricKey::WaitingTime], &[object], &[]).unwrap();
    assert!(
        c.metrics[0]
            .values
            .iter()
            .all(|m| matches!(m.value, MetricValue::Unknown(_)))
    );
}
#[test]
fn overlapping_reasons_do_not_double_count_one_objects_wait() {
    let mut i = inputs();
    let mut wait = i[0].3.waits[0].clone();
    let object = wait.object_ref.clone();
    wait.interval = IntervalMs {
        start_ms: SimTimeMs(SafeUint::try_from(0u64).unwrap()),
        end_ms: SimTimeMs(SafeUint::try_from(10000u64).unwrap()),
    };
    let mut overlapping = wait.clone();
    overlapping.interval.start_ms = SimTimeMs(SafeUint::try_from(5000u64).unwrap());
    overlapping.interval.end_ms = SimTimeMs(SafeUint::try_from(15000u64).unwrap());
    i[0].3.waits = vec![wait, overlapping];
    let c = compare_runs(&i, &[MetricKey::WaitingTime], &[object], &[]).unwrap();
    assert_eq!(
        v(&c.metrics[0].values[0].value),
        json!({"kind":"duration","data":15000})
    );
}

#[test]
fn nonzero_window_start_does_not_reset_completion_time_to_zero() {
    let mut i = inputs();
    for input in &mut i {
        input.3.window.start_ms = SimTimeMs(SafeUint::try_from(1000u64).unwrap());
    }
    let c = compare(&i, &[MetricKey::CompletionTime]);
    assert_eq!(
        v(&c.metrics[0].values[1].value),
        json!({"kind":"duration","data":165000})
    );
}
#[test]
fn missing_price_does_not_block_known_production_comparison() {
    let mut i = inputs();
    i[1].1.issues.push(Issue {
        code: "price_missing".into(),
        object_ref: None,
        field_path: Some("/cost_inputs/0/rate".into()),
        message: "Price was not supplied".into(),
        blocks: vec![CapabilityPurpose::CompleteConclusion],
    });
    let c = compare(&i, &[MetricKey::AcceptedQuantity, MetricKey::UnitCost]);
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::ControlledChanges
    ));
    assert!(accepted(&c).pair_differences[0].comparable);
    assert!(c.limitations.iter().any(|e| e.code == "price_missing"));
}
#[test]
fn absent_acceptance_scope_is_unknown_instead_of_zero() {
    let mut scope:AcceptanceScope=serde_json::from_value(json!({"product_ref":inputs()[0].1.orders[0].product_ref,"unit":"piece","destination":"not-declared","stage":"factory_released"})).unwrap();
    let c = compare_runs(
        &inputs(),
        &[MetricKey::AcceptedQuantity],
        &[],
        std::slice::from_ref(&scope),
    )
    .unwrap();
    assert!(
        c.metrics[0]
            .values
            .iter()
            .all(|m| matches!(m.value, MetricValue::Unknown(_)))
    );
    scope.unit = UnitId::try_from("kg".to_owned()).unwrap();
    let c = compare_runs(&inputs(), &[MetricKey::AcceptedQuantity], &[], &[scope]).unwrap();
    assert!(
        c.metrics[0]
            .values
            .iter()
            .all(|m| matches!(m.value, MetricValue::Unknown(_)))
    );
}
#[test]
fn utilization_uses_unique_claims_and_open_calendar_intersections() {
    let mut i = inputs();
    let claim = i[0].3.resource_claims[0].clone();
    let resource_id = claim.resource_id.clone();
    let resource = i[0]
        .0
        .resources
        .iter_mut()
        .find(|r| r.id == resource_id)
        .unwrap();
    resource.capacity.value = DecimalAmount {
        coefficient: SafeUint::try_from(1u64).unwrap(),
        decimal_places: DecimalPlaces::try_from(0).unwrap(),
    };
    let calendar_id = resource.calendar_id.clone();
    let interval = |a, b| IntervalMs {
        start_ms: SimTimeMs(SafeUint::try_from(a).unwrap()),
        end_ms: SimTimeMs(SafeUint::try_from(b).unwrap()),
    };
    i[0].0
        .calendars
        .iter_mut()
        .find(|c| c.id == calendar_id)
        .unwrap()
        .intervals = vec![interval(0u64, 10000u64), interval(20000u64, 30000u64)];
    let mut claim = claim;
    claim.units = SafeUint::try_from(1u64).unwrap();
    claim.intervals = vec![interval(0u64, 30000u64)];
    claim.interval_end_causes = vec![ClaimIntervalEndCause::Released];
    i[0].3.resource_claims = vec![claim.clone(), claim];
    let object = ObjectRef {
        kind: ObjectKind::Resource,
        id: resource_id,
    };
    let c = compare_runs(&i, &[MetricKey::ResourceUtilization], &[object], &[]).unwrap();
    match &c.metrics[0].values[0].value {
        MetricValue::Scalar { value, .. } => assert_eq!(value.value(), 1.0),
        other => panic!("{other:?}"),
    }
}
#[test]
fn quantity_overflow_is_unknown_and_never_wrapped_or_rounded() {
    let mut i = inputs();
    for e in &mut i[1].3.acceptance_events {
        e.quantity.value = DecimalAmount {
            coefficient: SafeUint::try_from(MAX_SAFE_UINT).unwrap(),
            decimal_places: DecimalPlaces::try_from(0).unwrap(),
        };
    }
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(matches!(
        accepted(&c).values[1].value,
        MetricValue::Unknown(_)
    ));
    assert!(accepted(&c).pair_differences[0].difference.is_none());
}

#[test]
fn change_links_refer_to_real_owners_not_synthetic_collection_ids() {
    let i = inputs();
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    let arrival = c.pairs[0]
        .input_changes
        .iter()
        .find(|c| c.path.contains("arrivals"))
        .unwrap();
    assert!(matches!(arrival.target.kind, ObjectKind::Scenario));
    assert_eq!(arrival.target.id.as_str(), i[1].1.id.as_str());
}
#[test]
fn multiple_candidates_preserve_order_and_baseline_pair_identity() {
    let mut i = inputs();
    let mut third = i[0].clone();
    third.3.id = RunId::try_from("third-candidate".to_owned()).unwrap();
    i.push(third);
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert_eq!(c.run_ids.len(), 3);
    assert_eq!(c.pairs.len(), 2);
    assert_eq!(c.pairs[1].candidate_run_id.as_str(), "third-candidate");
    assert_eq!(accepted(&c).values.len(), 3);
    assert_eq!(accepted(&c).pair_differences.len(), 2);
}

fn manual_goal_inputs() -> Vec<Frozen> {
    let evidence: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/acceptance-comparison/actual-engine-inputs.json"
    ))
    .unwrap();
    ["quality_a", "quality_same"]
        .iter()
        .map(|key| serde_json::from_value(evidence[*key].clone()).unwrap())
        .collect()
}

#[test]
fn manual_goal_completion_uses_first_threshold_event_without_capping_real_acceptance() {
    // Finite counterexample: change only the saved target in real event inputs.
    let mut i = manual_goal_inputs();
    for input in &mut i {
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
    let c = compare(
        &i,
        &[MetricKey::AcceptedQuantity, MetricKey::CompletionTime],
    );
    for value in &accepted(&c).values {
        assert_eq!(
            v(&value.value),
            json!({"kind":"quantity","data":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"}})
        );
    }
    let time = c
        .metrics
        .iter()
        .find(|m| matches!(m.metric, MetricKey::CompletionTime))
        .unwrap();
    assert_eq!(
        v(&time.values[0].value),
        json!({"kind":"duration","data":40000})
    );
    assert!(i.iter().all(|input| input.1.orders.is_empty()));
}

#[test]
fn changed_manual_quantity_is_different_demand_even_when_acceptance_scope_matches() {
    let mut i = manual_goal_inputs();
    i[1].1
        .acceptance_goal
        .as_mut()
        .unwrap()
        .accepted_quantity_per_day
        .as_mut()
        .unwrap()
        .value
        .coefficient = SafeUint::try_from(2u64).unwrap();
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(matches!(
        c.pairs[0].status,
        ComparisonStatus::SideBySideOnly
    ));
    assert!(
        c.pairs[0]
            .conditions
            .iter()
            .any(|c| matches!(c.key, ComparisonConditionKey::AcceptanceScope) && c.matches)
    );
    assert!(
        c.pairs[0]
            .conditions
            .iter()
            .any(|c| matches!(c.key, ComparisonConditionKey::Demand) && !c.matches)
    );
    assert!(!accepted(&c).pair_differences[0].comparable);
    assert!(accepted(&c).pair_differences[0].difference.is_none());
}

#[test]
fn actual_output_without_a_declared_goal_does_not_invent_an_acceptance_target() {
    let mut i = manual_goal_inputs();
    for input in &mut i {
        input.1.acceptance_goal = None;
        input.0.generation_context = None;
    }
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(accepted(&c).acceptance_scope.is_none());
    assert!(
        accepted(&c)
            .values
            .iter()
            .all(|m| matches!(m.value, MetricValue::Unknown(_)))
    );
}

#[test]
fn real_generation_context_goal_keeps_no_order_acceptance_semantics() {
    // Existing Engine source selection also accepts a real stored generation context.
    let mut i = manual_goal_inputs();
    for input in &mut i {
        let goal = input.1.acceptance_goal.take().unwrap();
        input.0.generation_context = Some(GenerationContext {
            goal,
            template_ref: None,
            parameter_sources: vec![],
            draft_check: None,
            initial_run_id: None,
            revised_run_ids: vec![],
            changes: vec![],
            stop_reason: None,
        });
    }
    let c = compare(&i, &[MetricKey::AcceptedQuantity]);
    assert!(matches!(c.pairs[0].status, ComparisonStatus::Strict));
    assert_eq!(
        v(&accepted(&c).values[0].value),
        json!({"kind":"quantity","data":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"}})
    );
    assert!(i.iter().all(|input| input.1.orders.is_empty()));
}
