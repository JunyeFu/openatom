//! Frozen real Engine evidence recorded by commands/comparison_acceptance, never hand-built Runs.
use serde_json::{Value, json};
use sf_domain::*;
type Frozen = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);
fn all() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/acceptance-comparison/actual-engine-inputs.json"
    ))
    .unwrap()
}
fn input(name: &str) -> Frozen {
    serde_json::from_value(all()[name].clone()).unwrap()
}
fn comparison(a: &str, b: &str) -> Value {
    let c = sf_reporting::compare_runs(&[input(a), input(b)], &[], &[], &[]).unwrap();
    serde_json::to_value(c).unwrap()
}
fn condition(c: &Value, key: &str) -> Value {
    c["pairs"][0]["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["key"] == key)
        .unwrap()
        .clone()
}
fn metric(c: &Value, key: &str) -> Value {
    c["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["metric"] == key)
        .unwrap()
        .clone()
}
#[test]
fn identical_business_rerun_is_strict_and_manual_no_order_goal_uses_real_cumulative_acceptance() {
    let c = comparison("quality_a", "quality_same");
    assert_eq!(c["pairs"][0]["status"], "strict");
    for key in [
        "time_reference",
        "observation_window",
        "acceptance_scope",
        "demand",
        "selected_scope",
        "initial_conditions",
        "supported_capabilities",
    ] {
        assert_eq!(condition(&c, key)["matches"], true, "{key}");
    }
    let m = metric(&c, "accepted_quantity");
    assert_eq!(
        m["acceptance_scope"],
        json!({"product_ref":{"id":"PACKAGE","revision":"r1"},"unit":"piece","destination":"finished","stage":"factory_released"})
    );
    for value in m["values"].as_array().unwrap() {
        assert_eq!(value["value"]["kind"], "quantity");
        assert_eq!(value["value"]["data"]["value"]["coefficient"], 2);
    }
    for value in metric(&c, "completion_time")["values"].as_array().unwrap() {
        assert_eq!(
            value["value"]["kind"], "unknown",
            "demand3 remains unfinished despite last package at100s"
        );
    }
}
#[test]
fn changed_saved_manual_goal_destination_is_a_real_acceptance_scope_change_and_never_strict() {
    let c = comparison("quality_a", "quality_scope");
    assert_eq!(condition(&c, "acceptance_scope")["matches"], false);
    assert_eq!(c["pairs"][0]["status"], "side_by_side_only");
    assert!(
        c["pairs"][0]["input_changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["path"].as_str().unwrap().contains("acceptance_goal"))
    );
    for m in c["metrics"].as_array().unwrap() {
        assert_eq!(m["pair_differences"][0]["comparable"], false);
        assert!(m["pair_differences"][0]["difference"].is_null());
    }
}
#[test]
fn different_actual_windows_and_cancelled_prefix_are_side_by_side_without_improvement_claim() {
    for other in ["quality_window", "quality_partial"] {
        let c = comparison("quality_a", other);
        assert_eq!(c["pairs"][0]["status"], "side_by_side_only");
        let key = if other == "quality_window" {
            "observation_window"
        } else {
            "supported_capabilities"
        };
        assert_eq!(condition(&c, key)["matches"], false);
        for m in c["metrics"].as_array().unwrap() {
            assert_eq!(m["pair_differences"][0]["comparable"], false);
            assert!(m["pair_differences"][0]["difference"].is_null());
        }
    }
}
#[test]
fn original_two_halls_show_changed_start_stock_before_125_205_results_and_unknown_unfinished_completion()
 {
    let c = comparison("halls_a", "halls_b");
    assert_ne!(c["pairs"][0]["status"], "strict");
    assert_eq!(condition(&c, "initial_conditions")["matches"], false);
    assert!(
        !c["pairs"][0]["input_changes"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let m = metric(&c, "accepted_quantity");
    assert_eq!(m["values"][0]["value"]["data"]["value"]["coefficient"], 1);
    assert_eq!(m["values"][1]["value"]["data"]["value"]["coefficient"], 2);
    let m = metric(&c, "completion_time");
    assert_eq!(m["values"][0]["value"]["kind"], "unknown");
    assert_eq!(
        m["values"][1]["value"],
        json!({"kind":"duration","data":205000})
    );
}
#[test]
fn actual_declared_decision_change_is_listed_while_original_frozen_result_remains_exact() {
    let a = input("quality_a");
    let saved = serde_json::to_value(&a.3).unwrap();
    let c =
        sf_reporting::compare_runs(&[a.clone(), input("quality_changed")], &[], &[], &[]).unwrap();
    let c = serde_json::to_value(c).unwrap();
    assert_eq!(condition(&c, "initial_conditions")["matches"], false);
    assert!(
        c["pairs"][0]["input_changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["path"]
                .as_str()
                .unwrap()
                .contains("declared_quality_decisions"))
    );
    assert_eq!(serde_json::to_value(a.3).unwrap(), saved);
}
#[test]
fn duplicate_or_mismatched_frozen_identity_is_rejected_and_missing_exact_definitions_prevent_strict_comparison()
 {
    let a = input("quality_a");
    assert!(sf_reporting::compare_runs(&[a.clone(), a.clone()], &[], &[], &[]).is_err());
    let mut mismatched = input("quality_same");
    mismatched.0.revision = serde_json::from_value(json!(999)).unwrap();
    assert!(sf_reporting::compare_runs(&[a.clone(), mismatched], &[], &[], &[]).is_err());
    let mut missing = input("quality_same");
    missing.2.clear();
    let c = serde_json::to_value(sf_reporting::compare_runs(&[a, missing], &[], &[], &[]).unwrap())
        .unwrap();
    assert_eq!(condition(&c, "supported_capabilities")["matches"], false);
    assert_eq!(c["pairs"][0]["status"], "side_by_side_only");
}
