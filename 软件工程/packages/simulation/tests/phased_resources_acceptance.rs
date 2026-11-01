//! Actual one-Engine consumers of frozen 5/30/5 calendars and explicit local variants.
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_domain::ModuleDefinition;
use sf_simulation::Engine;
use support::*;

fn op(v: &mut Value) -> &mut Value {
    &mut v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.phased")
        .unwrap()["payload"]["data"]
}
fn cnc(result: &Value) -> Value {
    let j = jobs(result, "w5.op.phased");
    assert_eq!(j.len(), 1);
    j[0].clone()
}
fn working(result: &Value) -> Vec<(String, u64, u64)> {
    let id = cnc(result)["id"].clone();
    let mut work: Vec<_> = result["phase_work_intervals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["job_id"] == id)
        .map(|w| {
            (
                w["phase_id"].as_str().unwrap().to_owned(),
                w["interval"]["start_ms"].as_u64().unwrap(),
                w["interval"]["end_ms"].as_u64().unwrap(),
            )
        })
        .collect();
    work.sort_by_key(|w| w.1);
    work
}
fn machine_gap(policy: &str) -> Value {
    let mut value = phased();
    value["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"MACHINE-GAP","title":"explicit off shift","source":"user_provided","intervals":[{"start_ms":0,"end_ms":20000},{"start_ms":40000,"end_ms":100000}]}));
    for r in value["project"]["resources"].as_array_mut().unwrap() {
        if r["id"] == "CAP-CNC" {
            r["calendar_id"] = json!("MACHINE-GAP");
        }
        if r["id"] == "W-C" {
            r["calendar_id"] = json!("ALL");
        }
    }
    op(&mut value)["phases"][1]["interruption_policy"] = json!(policy);
    value
}

#[test]
fn original_phase_calendar_finishes_at_45_seconds_with_40_work_10_worker_and_one_machine_claim() {
    let result = run(&phased());
    let job = cnc(&result);
    assert_eq!(job["started_at_ms"], 0);
    assert_eq!(job["finished_at_ms"], 45000);
    assert_eq!(
        working(&result),
        vec![
            ("load".into(), 0, 5000),
            ("auto".into(), 5000, 35000),
            ("unload".into(), 40000, 45000)
        ]
    );
    assert_eq!(held_ms(&result, "W-C"), 10000);
    assert_eq!(intervals(&result, "W-C"), vec![(0, 5000), (40000, 45000)]);
    assert_eq!(intervals(&result, "CAP-CNC"), vec![(0, 45000)]);
    let machine: Vec<_> = result["resource_claims"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["resource_id"] == "CAP-CNC")
        .collect();
    assert_eq!(machine.len(), 1);
    assert_eq!(machine[0]["kind"], "job");
    assert!(machine[0]["phase_ref"].is_null());
    assert_eq!(machine[0]["interval_end_causes"], json!(["released"]));
    assert!(
        result["waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["object_ref"]["id"] == job["id"]
                && w["phase_ref"] == "unload"
                && w["interval"] == json!({"start_ms":35000,"end_ms":40000})
                && w["end_cause"] == "resolved")
    );
    assert_eq!(physical(&result, "part").len(), 1);
}

#[test]
fn coarse_no_phase_40_second_worker_hold_cannot_fit_two_separate_20_second_shifts() {
    let mut value = phased();
    let operation = op(&mut value);
    operation["phases"] = json!([]);
    operation["duration"] =
        json!({"value":40000,"unit":"millisecond","basis":"user_provided","source_ref":null});
    operation["resource_roles"].as_array_mut().unwrap().push(json!({"role":"worker","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"person"},"qualifications":[]}));
    let result = run(&value);
    let job = cnc(&result);
    assert!(job["started_at_ms"].is_null());
    assert!(job["finished_at_ms"].is_null());
    assert!(working(&result).is_empty());
    assert!(physical(&result, "part").is_empty());
    assert_eq!(
        named(&result, "raw-1")["holder"],
        json!({"kind":"location","id":"raw-store"})
    );
    assert!(intervals(&result, "W-C").is_empty());
    assert!(intervals(&result, "CAP-CNC").is_empty());
}

#[test]
fn an_actual_competing_job_delays_unload_to_50_without_releasing_the_cnc_or_holding_its_worker_during_auto()
 {
    let value: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/p1-phase-busy.json"
    ))
    .unwrap();
    let result = run(&value);
    let job = cnc(&result);
    assert_eq!(job["finished_at_ms"], 55000);
    assert_eq!(
        working(&result),
        vec![
            ("load".into(), 0, 5000),
            ("auto".into(), 5000, 35000),
            ("unload".into(), 50000, 55000)
        ]
    );
    assert_eq!(intervals(&result, "CAP-CNC"), vec![(0, 55000)]);
    assert_eq!(
        intervals(&result, "W-C"),
        vec![(0, 5000), (30000, 50000), (50000, 55000)]
    );
    let blocker = jobs(&result, "w5.op.blocker");
    assert_eq!(blocker.len(), 1);
    assert_eq!(blocker[0]["started_at_ms"], 30000);
    assert_eq!(blocker[0]["finished_at_ms"], 50000);
    assert!(
        result["waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["object_ref"]["id"] == job["id"]
                && w["phase_ref"] == "unload"
                && w["interval"] == json!({"start_ms":35000,"end_ms":50000}))
    );
}

#[test]
fn output_capacity_zero_keeps_one_real_result_machine_and_unfinished_job_after_last_phase() {
    let mut value = phased();
    value["project"]["locations"][1]["capacity"]["value"]["coefficient"] = json!(0);
    let mut engine = engine(&value);
    engine.run_to_end();
    let result = snapshot(&engine);
    let job = cnc(&result);
    assert_eq!(job["state"], "waiting_output");
    assert!(job["finished_at_ms"].is_null());
    assert_eq!(working(&result).last().unwrap().2, 45000);
    assert_eq!(held_ms(&result, "W-C"), 10000);
    assert_eq!(intervals(&result, "CAP-CNC"), vec![(0, 100000)]);
    let parts = physical(&result, "part");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0]["holder"], json!({"kind":"job","id":job["id"]}));
    assert_eq!(job["pending_outputs"], json!([parts[0]["id"]]));
    assert!(
        result["waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["object_ref"]["id"] == job["id"]
                && w["phase_ref"].is_null()
                && w["interval"] == json!({"start_ms":45000,"end_ms":100000})
                && w["end_cause"] == "observation_end")
    );
    assert!(
        result["resource_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["resource_id"] == "CAP-CNC"
                && c["interval_end_causes"] == json!(["observation_end"]))
    );
    assert_eq!(snapshot(&engine), result);
}

#[test]
fn future_arrival_is_absent_before_10_seconds_and_shifts_the_actual_phase_boundaries_to_10_15_45_50()
 {
    let mut value = phased();
    let mut incoming = value["scenario"]["initial_conditions"]["items"][0].clone();
    incoming["id"] = json!("incoming-raw");
    value["scenario"]["initial_conditions"]["items"] = json!([]);
    value["scenario"]["arrivals"] = json!([{"id":"ARRIVAL-10","at_ms":10000,"destination":"raw-store","items":[incoming],"release_policy":"unchecked_allowed"}]);
    let mut engine = engine(&value);
    engine.run_to_end();
    let result = snapshot(&engine);
    let before = serde_json::to_value(
        engine
            .objects_at(serde_json::from_value(json!(9999)).unwrap())
            .unwrap(),
    )
    .unwrap();
    assert!(
        !before
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "incoming-raw")
    );
    assert_eq!(cnc(&result)["started_at_ms"], 10000);
    assert_eq!(cnc(&result)["finished_at_ms"], 50000);
    assert_eq!(
        working(&result),
        vec![
            ("load".into(), 10000, 15000),
            ("auto".into(), 15000, 45000),
            ("unload".into(), 45000, 50000)
        ]
    );
    assert_eq!(held_ms(&result, "W-C"), 10000);
}

#[test]
fn non_preemptive_machine_phase_waits_for_one_continuous_slot_and_keeps_job_capacity_across_the_gap()
 {
    let result = run(&machine_gap("non_preemptive"));
    assert_eq!(cnc(&result)["finished_at_ms"], 75000);
    assert_eq!(
        working(&result),
        vec![
            ("load".into(), 0, 5000),
            ("auto".into(), 40000, 70000),
            ("unload".into(), 70000, 75000)
        ]
    );
    assert_eq!(intervals(&result, "CAP-CNC"), vec![(0, 75000)]);
    assert_eq!(held_ms(&result, "W-C"), 10000);
}

#[test]
fn pause_resume_machine_phase_records_two_real_work_intervals_and_retains_its_item_and_capacity() {
    let result = run(&machine_gap("pause_resume"));
    assert_eq!(cnc(&result)["finished_at_ms"], 60000);
    assert_eq!(
        working(&result),
        vec![
            ("load".into(), 0, 5000),
            ("auto".into(), 5000, 20000),
            ("auto".into(), 40000, 55000),
            ("unload".into(), 55000, 60000)
        ]
    );
    assert_eq!(intervals(&result, "CAP-CNC"), vec![(0, 60000)]);
    assert_eq!(held_ms(&result, "W-C"), 10000);
    let work = result["phase_work_intervals"].as_array().unwrap();
    let auto: Vec<_> = work.iter().filter(|w| w["phase_id"] == "auto").collect();
    assert_eq!(auto[0]["end_cause"], "interrupted");
    assert_eq!(auto[0]["effective_progress_end_ms"], 15000);
    assert_eq!(auto[1]["effective_progress_start_ms"], 15000);
    assert_eq!(auto[1]["effective_progress_end_ms"], 30000);
}

#[test]
fn a_missing_interruption_policy_is_a_located_missing_input_instead_of_a_silent_default() {
    let mut value = phased();
    op(&mut value)["phases"][0]
        .as_object_mut()
        .unwrap()
        .remove("interruption_policy");
    let definition = value["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "w5.op.phased")
        .unwrap()
        .clone();
    match serde_json::from_value::<ModuleDefinition>(definition) {
        Err(error) => assert!(error.to_string().contains("interruption_policy")),
        Ok(_) => assert!(
            Engine::new(input(&value)).is_err(),
            "incomplete policy must not run with a guessed choice"
        ),
    }
}

fn legacy() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/p1-legacy-goal.json"
    ))
    .unwrap()
}
#[test]
fn explicit_saved_no_order_goal_accepts_real_nonempty_items_with_null_order_and_does_not_cap_production()
 {
    let value = legacy();
    assert!(value["scenario"]["orders"].as_array().unwrap().is_empty());
    assert!(value["project"]["generation_context"].is_null());
    let result = run(&value);
    assert_eq!(physical(&result, "part").len(), 3);
    assert!(result["orders"].as_array().unwrap().is_empty());
    assert_eq!(result["goal_met"], true);
    let accepts = result["acceptance_events"].as_array().unwrap();
    assert_eq!(accepts.len(), 3);
    let mut accepted = std::collections::BTreeSet::new();
    let mut times = Vec::new();
    for event in accepts {
        assert!(event["order_id"].is_null());
        assert_eq!(
            event["quantity"],
            json!({"value":{"coefficient":1,"decimal_places":0},"unit":"piece"})
        );
        assert_eq!(event["destination"], "finished");
        assert_eq!(event["acceptance_stage"], "factory_released");
        let items = event["item_refs"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert!(
            accepted.insert(items[0].as_str().unwrap().to_owned()),
            "one physical item accepted once"
        );
        let item = named(&result, items[0].as_str().unwrap());
        assert_eq!(item["holder"], json!({"kind":"location","id":"finished"}));
        assert_eq!(item["quality"], "released");
        times.push(event["at_ms"].as_u64().unwrap());
    }
    times.sort();
    assert_eq!(times, vec![1000, 2000, 3000]);
    assert_eq!(accepted.len(), 3);
    assert_eq!(
        value["scenario"]["acceptance_goal"]["accepted_quantity_per_day"]["value"]["coefficient"],
        1
    );
}

#[test]
fn legacy_goal_edits_change_only_goal_comparison_and_explicit_source_conflicts_are_rejected() {
    let mut value = legacy();
    let first = run(&value);
    value["scenario"]["acceptance_goal"]["accepted_quantity_per_day"]["value"]["coefficient"] =
        json!(4);
    let second = run(&value);
    assert_eq!(second["goal_met"], false);
    assert_eq!(first["final_items"], second["final_items"]);
    assert_eq!(first["acceptance_events"], second["acceptance_events"]);
    let mut alternate = value["scenario"]["acceptance_goal"].clone();
    alternate["accepted_quantity_per_day"]["value"]["coefficient"] = json!(5);
    let result = Engine::new_with_goal(input(&value), serde_json::from_value(alternate).unwrap());
    assert!(
        matches!(result,Err(ref issues) if issues.iter().any(|issue|issue.code=="simulation.goal_source_conflict"))
    );
}

#[test]
fn no_order_acceptance_scope_rejects_wrong_product_unit_destination_stage_and_window_before_execution()
 {
    for (pointer, replacement) in [
        ("/acceptance_scope/product_ref/revision", json!("r2")),
        ("/acceptance_scope/unit", json!("box")),
        ("/acceptance_scope/destination", json!("MISSING")),
        ("/acceptance_scope/stage", json!("unloaded_and_released")),
        ("/observation_window/end_ms", json!(86399999)),
        ("/accepted_quantity_per_day/value/coefficient", json!(0)),
    ] {
        let mut value = legacy();
        *value["scenario"]["acceptance_goal"]
            .pointer_mut(pointer)
            .unwrap() = replacement;
        let before = value.clone();
        assert!(
            Engine::new(input(&value)).is_err(),
            "invalid explicit scope {pointer}"
        );
        assert_eq!(value, before);
    }
}
