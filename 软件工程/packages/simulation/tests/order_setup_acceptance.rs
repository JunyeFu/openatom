//! Whole P1-04/IX-02 independent expectations, fixed before producer runtime.
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_simulation::Engine;
use support::*;
fn fixture(first: &str) -> Value {
    serde_json::from_str(if first == "A" {
        include_str!("../../commands/tests/fixtures/production/p104-A-first.json")
    } else {
        include_str!("../../commands/tests/fixtures/production/p104-B-first.json")
    })
    .unwrap()
}
fn expected(first: &str) -> Value {
    let v: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/p401-p104-p202-expectations.json"
    ))
    .unwrap();
    v["original"]["business_expected"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["id"]
                == if first == "A" {
                    "multi_a_first"
                } else {
                    "multi_b_first"
                }
        })
        .unwrap()
        .clone()
}
fn production(result: &Value, op: &str) -> Vec<Value> {
    jobs(result, op)
        .into_iter()
        .filter(|j| !j["input_refs"].as_array().unwrap().is_empty())
        .collect()
}
fn check_full(first: &str, result: &Value) {
    let expected = expected(first);
    assert_eq!(result["whole_window_computed"], true);
    assert_eq!(result["goal_met"], true);
    assert_eq!(
        held_ms(result, "W-A"),
        expected["expected_worker_a_occupied_ms"].as_u64().unwrap()
    );
    let last = result["acceptance_events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["at_ms"].as_u64().unwrap())
        .max()
        .unwrap();
    assert_eq!(last, 205000);
    assert_eq!(physical(result, "package").len(), 4);
    let mut ids = std::collections::BTreeSet::new();
    for e in result["acceptance_events"].as_array().unwrap() {
        assert!(!e["item_refs"].as_array().unwrap().is_empty());
        for id in e["item_refs"].as_array().unwrap() {
            assert!(ids.insert(id.as_str().unwrap()));
        }
    }
    for order in expected["expected_orders"].as_array().unwrap() {
        let actual = result["orders"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["order_id"] == order["order"])
            .unwrap();
        assert_eq!(actual["completion_ms"], order["completion_ms"]);
        assert_eq!(actual["tardiness_ms"], order["tardiness_ms"]);
        assert_eq!(actual["accepted"]["value"]["coefficient"], 2);
        assert_eq!(actual["outstanding"]["value"]["coefficient"], 0);
    }
    for (ops, kind) in [
        (vec!["w5.op.assembly-A", "w5.op.assembly-B"], "assembly"),
        (vec!["w5.op.inspect"], "inspection"),
        (vec!["w5.op.pack"], "packing"),
    ] {
        let mut a: Vec<_> = ops
            .into_iter()
            .flat_map(|op| production(result, op))
            .map(|j| {
                (
                    j["started_at_ms"].as_u64().unwrap(),
                    j["finished_at_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        a.sort();
        let mut b: Vec<_> = expected["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| j["kind"] == kind)
            .map(|j| {
                (
                    j["start_ms"].as_u64().unwrap(),
                    j["end_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        b.sort();
        assert_eq!(a, b);
    }
    let mut shipments = result["transport"].as_array().unwrap().clone();
    shipments.sort_by_key(|t| t["departed_ms"].as_u64());
    assert_eq!(shipments.len(), 2);
    for (t, depart, received, returned) in [
        (&shipments[0], 120000, 145000, 165000),
        (&shipments[1], 180000, 205000, 225000),
    ] {
        assert_eq!(t["departed_ms"], depart);
        assert_eq!(t["received_ms"], received);
        assert_eq!(t["returned_ms"], returned);
        assert_eq!(t["cargo"].as_array().unwrap().len(), 2);
    }
    for worker in ["W-A", "W-B", "T1"] {
        for pair in intervals(result, worker).windows(2) {
            assert!(pair[0].1 <= pair[1].0);
        }
    }
}
#[test]
fn original_a_before_b_completes_at_205_with_b_late_5_and_worker_a_135_seconds() {
    check_full("A", &run(&fixture("A")));
}
#[test]
fn original_b_before_a_completes_at_205_with_a_late_45_and_worker_a_160_seconds() {
    check_full("B", &run(&fixture("B")));
}
#[test]
fn local_atomic_material_and_setup_consumer_retains_early_preparation_60_to_75_then_waits_for_pcb_100()
 {
    let mut value = fixture("A");
    value["scenario"]["supplies"] = json!([]);
    for o in value["scenario"]["orders"].as_array_mut().unwrap() {
        o["destination"] = json!("dispatch-store");
        o["acceptance_stage"] = json!("factory_released");
    }
    let mut e = engine(&value);
    e.run_to_end();
    let result = snapshot(&e);
    assert_eq!(held_ms(&result, "W-A"), 135000);
    assert!(intervals(&result, "W-A").contains(&(60000, 75000)));
    let b = production(&result, "w5.op.assembly-B");
    assert_eq!(b[0]["started_at_ms"], 100000);
    let before = serde_json::to_value(
        e.objects_at(serde_json::from_value(json!(99999)).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(
        !before
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == "pcb3" || i["id"] == "pcb4")
    );
    for i in before.as_array().unwrap().iter().filter(|i| {
        i["item_type_ref"]["id"] == "shell"
            && i["id"].as_str().unwrap() != "shell1"
            && i["id"].as_str().unwrap() != "shell2"
    }) {
        assert_eq!(i["holder"], json!({"kind":"location","id":"shell-store"}));
    }
}
#[test]
fn absent_second_pcb_arrival_leaves_the_unfinished_order_completion_and_tardiness_null_without_inventory_or_false_optimality()
 {
    let mut value = fixture("A");
    value["scenario"]["arrivals"] = json!([]);
    let result = run(&value);
    assert_eq!(physical(&result, "package").len(), 2);
    assert_eq!(result["goal_met"], false);
    let b = result["orders"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["order_id"] == "B")
        .unwrap();
    assert!(b["completion_ms"].is_null());
    assert!(b["tardiness_ms"].is_null());
    assert_eq!(b["outstanding"]["value"]["coefficient"], 2);
    assert!(
        jobs(&result, "w5.op.assembly-B")
            .iter()
            .all(|j| j["input_refs"].as_array().unwrap().is_empty())
    );
}
#[test]
fn explicit_release_blocks_early_material_acquisition_and_resource_claims() {
    let mut value = fixture("A");
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .retain(|t| t["id"] == "TASK-A");
    value["scenario"]["orders"]
        .as_array_mut()
        .unwrap()
        .retain(|o| o["id"] == "A");
    value["scenario"]["production_tasks"][0]["release_at_ms"] = json!(50000);
    value["scenario"]["orders"][0]["release_at_ms"] = json!(50000);
    let result = run(&value);
    let a = production(&result, "w5.op.assembly-A");
    assert_eq!(a[0]["started_at_ms"], 50000);
    assert!(
        intervals(&result, "W-A")
            .iter()
            .all(|(start, _)| *start >= 50000)
    );
}
#[test]
fn frozen_task_priorities_and_stable_ties_ignore_source_vector_order() {
    let original = fixture("B");
    let mut reversed = original.clone();
    for field in ["production_tasks", "orders"] {
        reversed["scenario"][field]
            .as_array_mut()
            .unwrap()
            .reverse();
    }
    reversed["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    reversed["definitions"].as_array_mut().unwrap().reverse();
    let a = run(&original);
    let b = run(&reversed);
    assert_eq!(a, b);
}
fn first_article(pending: bool) -> Value {
    let mut value = fixture("B");
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .retain(|t| t["id"] == "TASK-B");
    value["scenario"]["orders"]
        .as_array_mut()
        .unwrap()
        .retain(|o| o["id"] == "B");
    value["project"]["setup_policies"][0]["first_article"] = json!({"route_ref":{"id":"ROUTE-B","revision":"r1"},"family":"B","inspection_module":{"id":"w5.op.inspect","version":"0.1.0"},"gate_id":"B-FIRST-ARTICLE"});
    if pending {
        value["scenario"]["quality_presets"][0]["results"] = json!([null]);
    }
    value
}
#[test]
fn the_same_preparation_epoch_releases_remaining_family_work_only_after_its_actual_first_article_pass()
 {
    let result = run(&first_article(false));
    let b = production(&result, "w5.op.assembly-B");
    assert_eq!(b.len(), 2);
    assert_eq!(b[0]["started_at_ms"], 15000);
    assert_eq!(b[1]["started_at_ms"], 55000);
}
#[test]
fn unknown_first_article_holds_inspector_and_never_releases_the_remaining_batch_by_elapsed_time() {
    let result = run(&first_article(true));
    assert!(physical(&result, "package").is_empty());
    let b = production(&result, "w5.op.assembly-B");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0]["started_at_ms"], 15000);
    assert!(
        jobs(&result, "w5.op.assembly-B")
            .iter()
            .any(|j| j["started_at_ms"].is_null())
    );
    assert_eq!(intervals(&result, "W-B"), vec![(45000, 240000)]);

    // A real B -> A -> B sequence must create a fresh B preparation epoch.
    // The old B first-article PASS cannot release the new epoch's remaining B work.
    let mut epoch = fixture("B");
    epoch["project"]["setup_policies"][0]["first_article"] =
        first_article(false)["project"]["setup_policies"][0]["first_article"].clone();
    epoch["scenario"]["window"]["end_ms"] = json!(600000);
    epoch["scenario"]["supplies"] = json!([]);
    for calendar in epoch["project"]["calendars"].as_array_mut().unwrap() {
        calendar["intervals"] = json!([{"start_ms":0,"end_ms":600000}]);
    }
    let tasks = epoch["scenario"]["production_tasks"]
        .as_array()
        .unwrap()
        .clone();
    let base_b = tasks.iter().find(|t| t["id"] == "TASK-B").unwrap();
    let base_a = tasks.iter().find(|t| t["id"] == "TASK-A").unwrap();
    let mut staged = vec![];
    for (template, id, release, priority) in [
        (base_b, "RB-1", 0, 4),
        (base_a, "RA-1", 80000, 3),
        (base_b, "RB-2", 200000, 2),
        (base_b, "RB-3", 200000, 1),
    ] {
        let mut task = template.clone();
        task["id"] = json!(id);
        task["release_at_ms"] = json!(release);
        task["priority"] = json!(priority);
        task["planned_quantity"]["value"]["coefficient"] = json!(1);
        task["order_allocations"][0]["quantity"]["value"]["coefficient"] = json!(1);
        staged.push(task);
    }
    epoch["scenario"]["production_tasks"] = json!(staged);
    for order in epoch["scenario"]["orders"].as_array_mut().unwrap() {
        order["quantity"]["value"]["coefficient"] = json!(if order["id"] == "B" { 3 } else { 1 });
        order["destination"] = json!("dispatch-store");
        order["acceptance_stage"] = json!("factory_released");
        order["allow_partial"] = json!(true);
    }
    epoch["scenario"]["quality_presets"][2]["results"] = json!([null]);
    let result = run(&epoch);
    let mut b = production(&result, "w5.op.assembly-B");
    b.sort_by_key(|j| j["started_at_ms"].as_u64().unwrap());
    let a = production(&result, "w5.op.assembly-A");
    assert_eq!(b.len(), 2);
    assert_eq!(a.len(), 1);
    assert!(b[0]["finished_at_ms"].as_u64().unwrap() <= a[0]["started_at_ms"].as_u64().unwrap());
    assert!(a[0]["finished_at_ms"].as_u64().unwrap() <= b[1]["started_at_ms"].as_u64().unwrap());
    assert!(
        jobs(&result, "w5.op.assembly-B")
            .iter()
            .any(|j| j["task_ref"] == "RB-3" && j["started_at_ms"].is_null())
    );
    assert_eq!(physical(&result, "package").len(), 2);
    assert_eq!(
        named(&result, "shell4")["holder"],
        json!({"kind":"location","id":"shell-store"})
    );
}
#[test]
fn unknown_initial_family_without_a_declared_cold_start_is_missing_input_instead_of_zero_time_preparation()
 {
    let mut value = fixture("B");
    value["project"]["setup_policies"][0]["initial_family"] = Value::Null;
    value["scenario"]["initial_conditions"]["initial_setup_families"] = json!({});
    let before = value.clone();
    assert!(Engine::new(input(&value)).is_err());
    assert_eq!(value, before);
}
