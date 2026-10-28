//! Whole P2-02 independent consumers, expectations fixed from original designs before runtime.
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_simulation::Engine;
use support::*;
fn fixture(name: &str) -> Value {
    serde_json::from_str(match name {
        "A" => include_str!("../../commands/tests/fixtures/production/p202-halls-A.json"),
        "B" => include_str!("../../commands/tests/fixtures/production/p202-halls-B.json"),
        "short-stock" => {
            include_str!("../../commands/tests/fixtures/production/p202-short-stock.json")
        }
        "whole-batch-short" => {
            include_str!("../../commands/tests/fixtures/production/p202-whole-batch-short.json")
        }
        "identity" => include_str!("../../commands/tests/fixtures/production/p202-identity.json"),
        "blocked" => include_str!("../../commands/tests/fixtures/production/p202-blocked.json"),
        "departure-missed" => {
            include_str!("../../commands/tests/fixtures/production/p202-departure-missed.json")
        }
        "B13" => include_str!("../../commands/tests/fixtures/production/p202-B13.json"),
        "B14" => include_str!("../../commands/tests/fixtures/production/p202-B14.json"),
        "B16" => include_str!("../../commands/tests/fixtures/production/p202-B16.json"),
        "phase-people" => {
            include_str!("../../commands/tests/fixtures/production/p202-phase-people.json")
        }
        "baseline" => {
            include_str!("../../commands/tests/fixtures/production/p202-ix01-baseline.json")
        }
        "delayed-pcb" => {
            include_str!("../../commands/tests/fixtures/production/p202-ix01-delayed-pcb.json")
        }
        _ => panic!("unknown explicit case"),
    })
    .unwrap()
}
fn transfers(result: &Value) -> Vec<Value> {
    let mut t = result["transport"].as_array().unwrap().clone();
    t.sort_by_key(|x| x["departed_ms"].as_u64());
    t
}
fn at(e: &Engine, t: u64) -> Value {
    serde_json::to_value(
        e.objects_at(serde_json::from_value(json!(t)).unwrap())
            .unwrap(),
    )
    .unwrap()
}
fn cargo_at(e: &Engine, t: u64, id: &str) -> Value {
    at(e, t)
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == id)
        .unwrap()
        .clone()
}
#[test]
fn original_two_halls_a_b_keep_same_cargo_identity_110_receive_140_return_and_125_205_product_completion()
 {
    for (case, times, received) in [("A", vec![125000], 1), ("B", vec![125000, 205000], 2)] {
        let mut e = engine(&fixture(case));
        e.run_to_end();
        let result = snapshot(&e);
        let t = transfers(&result);
        assert_eq!(t.len(), received);
        let mut actual: Vec<_> = jobs(&result, "fixture.pack")
            .iter()
            .filter_map(|j| j["finished_at_ms"].as_u64())
            .collect();
        actual.sort();
        assert_eq!(actual, times);
        assert_eq!(t[0]["departed_ms"], 70000);
        assert_eq!(t[0]["arrived_ms"], 100000);
        assert_eq!(t[0]["received_ms"], 110000);
        assert_eq!(t[0]["returned_ms"], 140000);
        assert_eq!(intervals(&result, "worker-transport")[0], (60000, 140000));
        let id = t[0]["cargo"][0].as_str().unwrap();
        assert_eq!(
            cargo_at(&e, 65000, id)["holder"],
            json!({"kind":"location","id":"assemblies-A"})
        );
        assert_eq!(cargo_at(&e, 70000, id)["holder"]["kind"], "transport");
        assert_eq!(cargo_at(&e, 109999, id)["holder"]["kind"], "transport");
        assert_eq!(cargo_at(&e, 110000, id)["holder"]["kind"], "job");
        assert_eq!(physical(&result, "package").len(), received);
        if case == "B" {
            assert_eq!(t[1]["departed_ms"], 150000);
            assert_eq!(t[1]["received_ms"], 190000);
            assert_eq!(t[1]["returned_ms"], 220000);
        }
        let order = &result["orders"][0];
        assert_eq!(
            order["outstanding"]["value"]["coefficient"],
            if case == "A" { 1 } else { 0 }
        );
    }
}
#[test]
fn declared_target_200_never_creates_more_than_150_real_source_items_and_partial_load_is_independent()
 {
    let result = run(&fixture("short-stock"));
    let t = transfers(&result);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0]["cargo"].as_array().unwrap().len(), 150);
    assert_eq!(physical(&result, "assembly").len(), 150);
    assert!(
        physical(&result, "assembly")
            .iter()
            .all(|i| i["holder"] == json!({"kind":"location","id":"cargo-destination"}))
    );
    let result = run(&fixture("whole-batch-short"));
    assert!(transfers(&result).is_empty());
    assert!(intervals(&result, "T1").is_empty());
    assert_eq!(physical(&result, "assembly").len(), 150);
}
#[test]
fn loading_reservation_depart_arrive_receive_and_return_are_distinct_and_cargo_never_exists_at_two_holders()
 {
    let mut e = engine(&fixture("identity"));
    e.run_to_end();
    let result = snapshot(&e);
    let t = transfers(&result);
    assert_eq!(t.len(), 2);
    let id = t[0]["cargo"][0].as_str().unwrap();
    assert_eq!(
        cargo_at(&e, 9999, id)["holder"],
        json!({"kind":"location","id":"cargo-source"})
    );
    for time in [10000, 40000, 49999] {
        assert_eq!(cargo_at(&e, time, id)["holder"]["kind"], "transport");
    }
    assert_eq!(
        cargo_at(&e, 50000, id)["holder"],
        json!({"kind":"location","id":"cargo-destination"})
    );
    assert_eq!(t[0]["returned_ms"], 80000);
    assert_eq!(t[1]["departed_ms"], 90000);
    assert_eq!(intervals(&result, "T1"), vec![(0, 80000), (80000, 160000)]);
    assert_eq!(physical(&result, "assembly").len(), 2);
}
#[test]
fn full_receiver_keeps_real_cargo_on_transport_and_the_carrier_until_observation_end_without_phantom_receipt()
 {
    let result = run(&fixture("blocked"));
    let t = transfers(&result);
    assert_eq!(t.len(), 1);
    assert_eq!(t[0]["arrived_ms"], 40000);
    assert!(t[0]["received_ms"].is_null());
    assert!(t[0]["returned_ms"].is_null());
    assert_eq!(
        named(&result, t[0]["cargo"][0].as_str().unwrap())["holder"]["kind"],
        "transport"
    );
    assert_eq!(intervals(&result, "T1"), vec![(0, 240000)]);
    assert!(result["resource_claims"].as_array().unwrap().iter().any(
        |c| c["resource_id"] == "T1" && c["interval_end_causes"] == json!(["observation_end"])
    ));
    assert!(
        result["waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |w| w["interval"] == json!({"start_ms":40000,"end_ms":240000})
                    && w["end_cause"] == "observation_end"
            )
    );
    assert_eq!(physical(&result, "assembly").len(), 2);
    assert!(result["acceptance_events"].as_array().unwrap().is_empty());
}
#[test]
fn a_missed_one_millisecond_departure_opportunity_cannot_expand_to_an_all_day_window_or_ship_late_arrival()
 {
    let result = run(&fixture("departure-missed"));
    assert!(transfers(&result).is_empty());
    assert!(intervals(&result, "T1").is_empty());
    assert_eq!(
        named(&result, "cargo-0")["holder"],
        json!({"kind":"location","id":"cargo-source"})
    );
}
#[test]
fn incompatible_destination_type_or_quantity_unit_is_rejected_before_any_engine_state_exists() {
    for bad_unit in [false, true] {
        let mut value = fixture("identity");
        if bad_unit {
            value["scenario"]["supplies"][0]["target_quantity"]["unit"] = json!("box");
        } else {
            value["project"]["locations"][1]["accepted_item_types"] = json!(["pcb"]);
        }
        let before = value.clone();
        assert!(Engine::new(input(&value)).is_err());
        assert_eq!(value, before);
    }
}
fn verify_ix01(case: &str, last: u64) {
    let mut e = engine(&fixture(case));
    e.run_to_end();
    let result = snapshot(&e);
    let frozen: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/p401-p104-p202-expectations.json"
    ))
    .unwrap();
    let source = frozen["original"]["IX01_expected"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["scenario"]
                == if case == "baseline" {
                    "baseline"
                } else {
                    "delayed_pcb"
                }
        })
        .unwrap();
    let mut actual: Vec<_> = jobs(&result, "w5.op.pack")
        .iter()
        .filter_map(|j| j["finished_at_ms"].as_u64())
        .collect();
    actual.sort();
    let expected: Vec<_> = source["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["completed_ms"].as_u64().unwrap())
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(*actual.last().unwrap(), last);
    assert_eq!(physical(&result, "package").len(), 6);
    for (operation, kind) in [
        ("w5.op.assembly", "assembly"),
        ("w5.op.rework", "rework"),
        ("w5.op.pack", "packing"),
    ] {
        let mut a: Vec<_> = jobs(&result, operation)
            .iter()
            .filter_map(|j| Some((j["started_at_ms"].as_u64()?, j["finished_at_ms"].as_u64()?)))
            .collect();
        a.sort();
        let mut b: Vec<_> = source["jobs"]
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
    let expected_intervals = |kinds: &[&str]| {
        let mut spans: Vec<_> = source["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| kinds.iter().any(|kind| j["kind"] == *kind))
            .map(|j| {
                (
                    j["start_ms"].as_u64().unwrap(),
                    j["end_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        spans.sort();
        spans
    };
    let actual_work = |operation: &str, phase: Option<&str>| {
        let ids: Vec<_> = jobs(&result, operation)
            .iter()
            .map(|j| j["id"].clone())
            .collect();
        let mut spans: Vec<_> = result["phase_work_intervals"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|w| ids.contains(&w["job_id"]) && phase.is_none_or(|p| w["phase_id"] == p))
            .map(|w| {
                (
                    w["interval"]["start_ms"].as_u64().unwrap(),
                    w["interval"]["end_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        spans.sort();
        spans
    };
    for (phase, kind) in [
        ("load", "cnc_load"),
        ("auto", "cnc_auto"),
        ("unload", "cnc_unload"),
    ] {
        assert_eq!(
            actual_work("w5.op.shell", Some(phase)),
            expected_intervals(&[kind])
        );
    }
    assert_eq!(
        actual_work("w5.op.inspect", None),
        expected_intervals(&["inspection", "reinspection"])
    );
    assert_eq!(
        intervals(&result, "CAP-CNC01"),
        expected_intervals(&["cnc_machine"])
    );
    for worker in ["W-A", "W-B", "W-C"] {
        let mut expected: Vec<_> = source["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|j| {
                j["resources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|r| r == worker)
            })
            .map(|j| {
                (
                    j["start_ms"].as_u64().unwrap(),
                    j["end_ms"].as_u64().unwrap(),
                )
            })
            .collect();
        expected.sort();
        assert_eq!(intervals(&result, worker), expected);
    }
    let t = transfers(&result);
    assert_eq!(t.len(), 6);
    for (a, b) in t.iter().zip(source["transfers"].as_array().unwrap()) {
        assert_eq!(a["departed_ms"], b["depart_ms"]);
        assert_eq!(a["arrived_ms"], b["arrival_ms"]);
        assert_eq!(a["received_ms"], b["accepted_ms"]);
        assert_eq!(a["returned_ms"], b["resource_available_ms"]);
        assert_eq!(a["cargo"].as_array().unwrap().len(), 1);
    }
    let assemblies: Vec<_> = physical(&result, "assembly")
        .into_iter()
        .filter(|i| ancestry(&result, i["id"].as_str().unwrap(), "blank003"))
        .collect();
    assert_eq!(assemblies.len(), 1);
    assert_eq!(assemblies[0]["completed_reworks"], 1);
    assert_eq!(assemblies[0]["quality"], "released");
    if case == "delayed-pcb" {
        let id = t[5]["cargo"][0].as_str().unwrap();
        assert_eq!(cargo_at(&e, 419999, id)["holder"]["kind"], "transport");
        assert_eq!(
            cargo_at(&e, 420000, id)["holder"],
            json!({"kind":"location","id":"shell-buffer"})
        );
        assert_eq!(t[5]["returned_ms"], 425000);
        assert!(
            result["waits"]
                .as_array()
                .unwrap()
                .iter()
                .any(
                    |w| w["interval"] == json!({"start_ms":370000,"end_ms":420000})
                        && w["end_cause"] == "resolved"
                )
        );
    }
}
#[test]
fn full_original_ix01_baseline_keeps_all_six_packages_and_finishes_at_440_seconds() {
    verify_ix01("baseline", 440000);
}
#[test]
fn full_original_ix01_delayed_pcb_finishes_at_570_seconds_with_cart_wait_370_to_420_and_return_425()
{
    verify_ix01("delayed-pcb", 570000);
}

#[test]
fn frozen_b14_waits_on_the_actual_unloader_calendar_until_120_then_receives_125_and_returns_135() {
    let mut e = engine(&fixture("B14"));
    e.run_to_end();
    let result = snapshot(&e);
    let t = transfers(&result);
    assert_eq!(t.len(), 1);
    for (key, n) in [
        ("departed_ms", 100000),
        ("arrived_ms", 110000),
        ("received_ms", 125000),
        ("returned_ms", 135000),
    ] {
        assert_eq!(t[0][key], n);
    }
    assert_eq!(intervals(&result, "T1"), vec![(100000, 135000)]);
    assert_eq!(intervals(&result, "W-DOCK"), vec![(120000, 125000)]);
    assert_eq!(
        cargo_at(&e, 124999, t[0]["cargo"][0].as_str().unwrap())["holder"]["kind"],
        "transport"
    );
    assert!(result["waits"].as_array().unwrap().iter().any(|w| {
        w["interval"] == json!({"start_ms":110000,"end_ms":120000})
            && w["end_cause"] == "resolved"
            && w["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["kind"] == "calendar")
    }));
}
#[test]
fn actual_loading_and_unloading_fields_use_one_generic_role_worker_only_in_the_two_declared_phases()
{
    let result = run(&fixture("phase-people"));
    assert_eq!(
        intervals(&result, "W-DOCK"),
        vec![(0, 10000), (40000, 50000), (80000, 90000), (120000, 130000)]
    );
    assert_eq!(held_ms(&result, "W-DOCK"), 40000);
    assert_eq!(intervals(&result, "T1"), vec![(0, 80000), (80000, 160000)]);
    assert!(
        result["resource_claims"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["resource_id"] == "W-DOCK")
            .all(|c| c["kind"] == "phase")
    );
}
#[test]
fn frozen_b13_cross_midnight_keeps_one_real_order_allocation_and_counts_acceptance_on_the_second_day()
 {
    let mut e = engine(&fixture("B13"));
    e.run_to_end();
    let result = snapshot(&e);
    let t = transfers(&result);
    assert_eq!(t.len(), 1);
    let t = &t[0];
    for (key, n) in [
        ("departed_ms", 86390000),
        ("arrived_ms", 86410000),
        ("received_ms", 86425000),
        ("returned_ms", 86435000),
    ] {
        assert_eq!(t[key], n);
    }
    assert_eq!(intervals(&result, "T1"), vec![(86390000, 86435000)]);
    assert_eq!(intervals(&result, "W-DOCK"), vec![(86420000, 86425000)]);
    let mut days = [0u64; 2];
    for a in result["acceptance_events"].as_array().unwrap() {
        let time = a["at_ms"].as_u64().unwrap();
        days[(time / 86400000) as usize] += a["quantity"]["value"]["coefficient"].as_u64().unwrap();
        assert_eq!(a["order_id"], "O-day1");
        assert_eq!(time, 86425000);
    }
    assert_eq!(days, [0, 2]);
    assert_eq!(result["orders"][0]["completion_ms"], 86425000);
    assert_eq!(result["orders"][0]["tardiness_ms"], 0);
    assert_eq!(physical(&result, "package").len(), 2);
    for cargo in t["cargo"].as_array().unwrap() {
        let id = cargo.as_str().unwrap();
        assert_eq!(cargo_at(&e, 86400000, id)["holder"]["kind"], "transport");
        assert_eq!(cargo_at(&e, 86424999, id)["holder"]["kind"], "transport");
        assert_eq!(
            cargo_at(&e, 86425000, id)["holder"],
            json!({"kind":"location","id":"customer"})
        );
    }
}
#[test]
fn frozen_b16_nonpartial_order_does_not_free_capacity_on_acceptance_or_invent_an_unlimited_receiver()
 {
    let result = run(&fixture("B16"));
    let t = transfers(&result);
    assert_eq!(t.len(), 2);
    assert_eq!(t[0]["received_ms"], 10000);
    assert_eq!(t[1]["arrived_ms"], 20000);
    assert!(t[1]["received_ms"].is_null());
    assert!(result["acceptance_events"].as_array().unwrap().is_empty());
    assert_eq!(
        result["orders"][0]["outstanding"]["value"]["coefficient"],
        2
    );
    assert!(result["orders"][0]["completion_ms"].is_null());
    assert_eq!(result["goal_met"], false);
    assert_eq!(
        named(&result, t[1]["cargo"][0].as_str().unwrap())["holder"]["kind"],
        "transport"
    );
    assert_eq!(
        physical(&result, "package")
            .iter()
            .filter(|i| i["holder"] == json!({"kind":"location","id":"customer"}))
            .count(),
        1
    );
    assert!(
        result["waits"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |w| w["interval"] == json!({"start_ms":20000,"end_ms":30000})
                    && w["end_cause"] == "observation_end"
            )
    );
}
#[test]
fn same_actual_carrier_in_job_and_both_phase_lists_is_held_once_and_missing_phase_role_is_not_guessed()
 {
    let mut value = fixture("identity");
    let tr = value["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["payload"]["category"] == "transport")
        .unwrap();
    tr["payload"]["data"]["loading_resources"] = tr["payload"]["data"]["resources"].clone();
    tr["payload"]["data"]["unloading_resources"] = tr["payload"]["data"]["resources"].clone();
    let result = run(&value);
    assert_eq!(intervals(&result, "T1"), vec![(0, 80000), (80000, 160000)]);
    assert_eq!(held_ms(&result, "T1"), 160000);
    let mut missing = fixture("B14");
    missing["scenario"]["supplies"][0]["resources"]
        .as_object_mut()
        .unwrap()
        .remove("dock_hand");
    let before = missing.clone();
    assert!(Engine::new(input(&missing)).is_err());
    assert_eq!(missing, before);
}
