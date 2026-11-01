#![allow(dead_code)]
use serde_json::Value;
use sf_domain::*;
use sf_simulation::{Engine, EngineInput};
use std::collections::BTreeSet;

pub fn parallel(case: &str) -> Value {
    serde_json::from_str(match case {
        "A" => include_str!("p1-parallel-A.json"),
        "Q" => include_str!("p1-parallel-Q.json"),
        "U" => include_str!("p1-parallel-U.json"),
        _ => panic!("unknown original case {case}"),
    })
    .unwrap()
}
pub fn phased() -> Value {
    serde_json::from_str(include_str!("p1-phase-calendar.json")).unwrap()
}
pub fn input(v: &Value) -> EngineInput {
    EngineInput {
        run_id: RunId::try_from(format!("W5-P1-{}", v["scenario"]["id"].as_str().unwrap()))
            .unwrap(),
        input_ref: serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        scenario: serde_json::from_value(v["scenario"].clone()).unwrap(),
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope: serde_json::from_value(v["scenario"]["scope"].clone()).unwrap(),
    }
}
pub fn engine(v: &Value) -> Engine {
    Engine::new(input(v))
        .unwrap_or_else(|issues| panic!("formal independent input rejected: {issues:#?}"))
}
pub fn snapshot(e: &Engine) -> Value {
    serde_json::to_value(e.snapshot()).unwrap()
}
pub fn run(v: &Value) -> Value {
    let mut e = engine(v);
    e.run_to_end();
    let result = snapshot(&e);
    assert_eq!(result["evidence_origin"], "actual_software");
    assert_eq!(result["computation_status"], "completed");
    assert_eq!(result["whole_window_computed"], true);
    assert_eq!(
        result["computed_until_ms"],
        v["scenario"]["window"]["end_ms"]
    );
    assert_eq!(result["input_ref"], v["input_ref"]);
    assert_eq!(snapshot(&e), result);
    for pair in result["events"].as_array().unwrap().windows(2) {
        assert!(pair[0]["sequence"].as_u64().unwrap() < pair[1]["sequence"].as_u64().unwrap());
        assert!(pair[0]["at_ms"].as_u64().unwrap() <= pair[1]["at_ms"].as_u64().unwrap());
    }
    result
}
pub fn physical(result: &Value, kind: &str) -> Vec<Value> {
    result["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| {
            item["item_type_ref"]["id"] == kind
                && item["lifecycle"] != "transformed"
                && item["lifecycle"] != "scrapped"
        })
        .cloned()
        .collect()
}
pub fn named(result: &Value, id: &str) -> Value {
    result["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap_or_else(|| panic!("missing real material {id}"))
        .clone()
}
pub fn ancestry(result: &Value, id: &str, ancestor: &str) -> bool {
    let mut pending = vec![id.to_owned()];
    let mut seen = BTreeSet::new();
    while let Some(current) = pending.pop() {
        if current == ancestor {
            return true;
        }
        if !seen.insert(current.clone()) {
            continue;
        }
        let item = named(result, &current);
        pending.extend(
            item["composition_refs"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_owned()),
        );
        if let Some(parent) = item["split_from_ref"].as_str() {
            pending.push(parent.to_owned());
        }
    }
    false
}
pub fn jobs(result: &Value, operation: &str) -> Vec<Value> {
    result["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|job| job["operation_ref"]["id"] == operation)
        .cloned()
        .collect()
}
pub fn intervals(result: &Value, resource: &str) -> Vec<(u64, u64)> {
    let mut intervals: Vec<_> = result["resource_claims"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|claim| claim["resource_id"] == resource)
        .flat_map(|claim| {
            assert_eq!(
                claim["intervals"].as_array().unwrap().len(),
                claim["interval_end_causes"].as_array().unwrap().len()
            );
            assert_eq!(claim["units"], 1);
            claim["intervals"]
                .as_array()
                .unwrap()
                .iter()
                .map(|span| {
                    (
                        span["start_ms"].as_u64().unwrap(),
                        span["end_ms"].as_u64().unwrap(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect();
    intervals.sort();
    intervals
}
pub fn held_ms(result: &Value, resource: &str) -> u64 {
    intervals(result, resource).iter().map(|(a, b)| b - a).sum()
}
pub fn original_expectation(case: &str) -> Value {
    let v: Value = serde_json::from_str(include_str!("p1-full-expectations.json")).unwrap();
    v["parallel_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == case)
        .unwrap()["expected"]
        .clone()
}
