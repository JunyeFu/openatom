#![allow(dead_code)]
#[path = "../../../packages/commands/tests/fixtures/whole_ix/checks.rs"]
pub mod checks;
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
pub mod host;
use host::{Harness, ok};
use serde_json::{Value, json};
use sf_commands::RunView;
use sf_domain::{RunComparison, RunResult};
use std::{collections::BTreeSet, path::PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap()
        .to_owned()
}
pub fn source(case: &str) -> Value {
    let mut value: Value = serde_json::from_str(if case == "B" {
        include_str!("../../../packages/commands/tests/fixtures/production/p104-B-first.json")
    } else {
        include_str!("../../../packages/commands/tests/fixtures/production/p104-A-first.json")
    })
    .unwrap();
    if case == "missing" {
        value["scenario"]["arrivals"] = json!([]);
    }
    value["project"]["views"]["selected_hall"] = json!("H1");
    value
}
pub fn execute(h: &mut Harness, scenario: &str) -> RunResult {
    let task = h.run(scenario);
    h.finish_one();
    h.completed_run(&task)
}
pub fn check(case: &str, run: &RunResult) {
    let value = json!(run);
    checks::common(&value);
    assert!(run.limitations.is_empty());
    assert_eq!(
        run.final_items
            .iter()
            .map(|i| &i.id)
            .collect::<BTreeSet<_>>()
            .len(),
        run.final_items.len()
    );
    if case != "missing" {
        checks::ix02(
            &value,
            if case == "A" {
                "multi_a_first"
            } else {
                "multi_b_first"
            },
        );
        checks::original_materials(
            &value,
            &source(case),
            &[("shell", 1), ("pcb", 1), ("fastener", 4), ("carton", 1)],
        );
    } else {
        assert_eq!(run.goal_met, Some(false));
        assert_eq!(checks::physical(&value, "package").len(), 2);
        let b = run
            .orders
            .iter()
            .find(|o| o.order_id.as_str() == "B")
            .unwrap();
        assert!(b.completion_ms.is_none());
        assert!(b.tardiness_ms.is_none());
        assert_eq!(b.accepted.value.coefficient.value(), 0);
        assert_eq!(b.outstanding.value.coefficient.value(), 2);
        assert!(
            !run.final_items
                .iter()
                .any(|i| ["pcb3", "pcb4"].contains(&i.id.as_str()))
        );
        assert!(
            run.jobs
                .iter()
                .filter(|j| j.operation_ref.id.as_str() == "w5.op.assembly-B")
                .all(|j| j.input_refs.is_empty())
        );
    }
}
pub fn business(mut run: Value) -> Value {
    // Only a fresh Run's identity and exact frozen file refs differ.
    run.as_object_mut().unwrap().remove("id");
    run.as_object_mut().unwrap().remove("input_ref");
    run
}
pub fn frozen(h: &mut Harness, run: &RunResult, kind: &str, id: &str) -> Value {
    let value: sf_commands::DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":0}},"node_index":null})));
    json!(value)["data"].clone()
}
pub fn history(h: &mut Harness, run: &RunResult) -> Value {
    let selected = json!(
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
    let mut rows = vec![];
    for at in [
        0, 59999, 60000, 74999, 75000, 99999, 100000, 119999, 120000, 140000, 144999, 145000,
        164999, 165000, 180000, 200000, 205000, 225000,
    ] {
        let mut cursor: Option<sf_domain::SafeUint> = None;
        let mut objects = vec![];
        loop {
            let view:RunView=ok(h.call("get_run",json!({"run_id":run.id,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":selected,"page":{"after_index":cursor,"limit":7}}}})));
            let RunView::Objects(page) = view else {
                panic!("actual historical objects")
            };
            objects.extend(page.objects);
            cursor = page.next_index;
            if cursor.is_none() {
                break;
            }
        }
        rows.push(json!({"at_ms":at,"objects":objects}));
    }
    let mut cursor: Option<sf_domain::SafeUint> = None;
    let mut events = vec![];
    loop {
        let view:RunView=ok(h.call("get_run",json!({"run_id":run.id,"query":{"kind":"events","data":{"interval":null,"page":{"after_index":cursor,"limit":7}}}})));
        let RunView::Events(page) = view else {
            panic!("actual paged events")
        };
        events.extend(page.events);
        cursor = page.next_index;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(json!(events), json!(run.events));
    json!({"objects":rows,"events":events})
}
pub fn compare_completion(
    h: &mut Harness,
    case: &str,
    first: &RunResult,
    second: &RunResult,
) -> Value {
    let input = source(case);
    let scopes:Vec<_>=input["scenario"]["orders"].as_array().unwrap().iter().map(|o|json!({
        "product_ref":o["product_ref"],"unit":o["quantity"]["unit"],"destination":o["destination"],"stage":o["acceptance_stage"]
    })).collect();
    let comparison:RunComparison=ok(h.call("compare_runs",json!({"run_ids":[first.id,second.id],"metric_scope":{"keys":["completion_time"],"objects":[],"acceptance":scopes}})));
    let value = json!(comparison);
    for (order, scope) in input["scenario"]["orders"]
        .as_array()
        .unwrap()
        .iter()
        .zip(scopes)
    {
        let metric = value["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["metric"] == "completion_time" && m["acceptance_scope"] == scope)
            .unwrap();
        assert_eq!(metric["values"].as_array().unwrap().len(), 2);
        for actual in metric["values"].as_array().unwrap() {
            if case == "missing" && order["id"] == "B" {
                assert_eq!(actual["value"]["kind"], "unknown");
            } else {
                let early = if case == "B" { "B" } else { "A" };
                assert_eq!(actual["value"]["kind"], "duration");
                assert_eq!(
                    actual["value"]["data"],
                    if order["id"] == early { 145000 } else { 205000 }
                );
            }
        }
    }
    value
}
