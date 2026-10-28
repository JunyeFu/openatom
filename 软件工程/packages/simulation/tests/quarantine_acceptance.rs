//! Independent full-original P1-05 consumer; expectations precede producer execution.
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use support::*;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/quality/quality-original.json"
    ))
    .unwrap()
}
fn assembly(cycle: usize) -> String {
    let job = format!("job:10:TASK-LOCAL1:{cycle}13:step-assembly");
    format!("output:{}:{job}9:assembled1:0", job.len())
}
fn at(e: &sf_simulation::Engine, ms: u64, id: &str) -> Value {
    serde_json::to_value(
        e.objects_at(serde_json::from_value(json!(ms)).unwrap())
            .unwrap(),
    )
    .unwrap()
    .as_array()
    .unwrap()
    .iter()
    .find(|v| v["id"] == id)
    .unwrap()
    .clone()
}
fn inspect_mut(v: &mut Value) -> &mut Value {
    &mut v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|x| x["id"] == "w5.op.inspect")
        .unwrap()["payload"]["data"]
}
fn accepted(r: &Value) -> u64 {
    r["acceptance_events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            assert!(v["order_id"].is_null());
            assert!(!v["item_refs"].as_array().unwrap().is_empty());
            v["quantity"]["value"]["coefficient"].as_u64().unwrap()
        })
        .sum()
}
#[test]
fn original_two_packages_one_physical_scrap_shortage_one_and_distinct_80s_states() {
    let v = fixture();
    assert_eq!(v["scenario"]["orders"], json!([]));
    assert!(v["project"]["generation_context"].is_null());
    let mut e = engine(&v);
    e.run_to_end();
    let r = snapshot(&e);
    assert_eq!(r["computation_status"], "completed");
    assert_eq!(r["whole_window_computed"], true);
    assert_eq!(physical(&r, "package").len(), 2);
    assert_eq!(physical(&r, "carton").len(), 1);
    assert_eq!(accepted(&r), 2);
    assert_eq!(3 - accepted(&r), 1);
    assert_eq!(r["goal_met"], false);
    assert_eq!(
        intervals(&r, "W-A"),
        vec![(0, 20000), (20000, 40000), (40000, 60000), (60000, 75000)]
    );
    assert_eq!(
        intervals(&r, "W-Q"),
        vec![
            (20000, 30000),
            (40000, 50000),
            (60000, 70000),
            (75000, 85000)
        ]
    );
    assert_eq!(intervals(&r, "W-P"), vec![(30000, 40000), (90000, 100000)]);
    let pending = at(&e, 80000, &assembly(2));
    assert_eq!(pending["quality"], "pending");
    assert_eq!(
        pending["holder"],
        json!({"kind":"location","id":"pending-store"})
    );
    let checking = at(&e, 80000, &assembly(1));
    assert_eq!(checking["lifecycle"], "in_process");
    assert_eq!(checking["holder"]["kind"], "job");
    let scrap = named(&r, &assembly(1));
    assert_eq!(scrap["lifecycle"], "scrapped");
    assert_eq!(
        scrap["holder"],
        json!({"kind":"location","id":"quarantine"})
    );
    assert_eq!(scrap["completed_reworks"], 1);
    assert!(ancestry(&r, &assembly(1), "shell02"));
    for ty in ["shell", "pcb", "fastener"] {
        assert!(physical(&r, ty).is_empty(), "{ty} returned after scrap");
    }
    let assemblies: Vec<_> = r["final_items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x["item_type_ref"]["id"] == "assembly")
        .collect();
    assert_eq!(
        assemblies.len(),
        3,
        "no replacement assembly or duplicate identity"
    );
    assert_eq!(
        assemblies
            .iter()
            .filter(|x| x["quality"] == "pending")
            .count(),
        0
    );
}
#[test]
fn unknown_without_explicit_decision_stays_in_real_pending_store_and_is_neither_passed_nor_scrapped()
 {
    let mut v = fixture();
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
    let r = run(&v);
    let third = named(&r, &assembly(2));
    assert_eq!(third["quality"], "pending");
    assert_eq!(third["lifecycle"], "available");
    assert_eq!(third["holder"]["id"], "pending-store");
    assert_eq!(accepted(&r), 1);
    assert_eq!(physical(&r, "package").len(), 1);
    assert_eq!(named(&r, &assembly(1))["holder"]["id"], "quarantine");
    assert_eq!(named(&r, &assembly(1))["lifecycle"], "available");
}
#[test]
fn hold_station_and_full_pending_capacity_keep_inspection_person_and_same_item() {
    for hold in [true, false] {
        let mut v = fixture();
        v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
        if hold {
            inspect_mut(&mut v)["quality_and_routes"]["pending_policy"] = json!("hold_station");
        } else {
            let mut blocker = v["scenario"]["initial_conditions"]["items"][0].clone();
            blocker["id"] = json!("existing-pending-slot");
            blocker["item_type_ref"]["id"] = json!("assembly");
            blocker["quality"] = json!("pending");
            blocker["holder"]["id"] = json!("pending-store");
            v["scenario"]["initial_conditions"]["items"]
                .as_array_mut()
                .unwrap()
                .push(blocker);
        }
        let mut e = engine(&v);
        e.run_to_end();
        let r = snapshot(&e);
        assert_eq!(accepted(&r), 1);
        assert_eq!(
            intervals(&r, "W-Q"),
            vec![(20000, 30000), (40000, 50000), (60000, 120000)]
        );
        let third = at(&e, 80000, &assembly(2));
        assert_eq!(third["quality"], "pending");
        assert_eq!(third["holder"]["kind"], "job");
        assert_eq!(named(&r, &assembly(1))["holder"]["id"], "inspect-in");
    }
}
#[test]
fn explicit_reinspect_is_a_new_real_attempt_on_same_item_without_rework_or_retroactive_completion()
{
    let mut v = fixture();
    v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["disposition"] =
        json!("reinspect");
    v["scenario"]["quality_presets"][2]["results"] = json!([null,{"kind":"string","value":"PASS"}]);
    let mut e = engine(&v);
    e.run_to_end();
    let r = snapshot(&e);
    assert_eq!(accepted(&r), 2);
    assert_eq!(named(&r, &assembly(2))["completed_reworks"], 0);
    assert_eq!(
        intervals(&r, "W-Q"),
        vec![
            (20000, 30000),
            (40000, 50000),
            (60000, 70000),
            (75000, 85000),
            (90000, 100000)
        ]
    );
    assert_eq!(intervals(&r, "W-P"), vec![(30000, 40000), (100000, 110000)]);
    let checks: Vec<_> = jobs(&r, "w5.op.inspect")
        .into_iter()
        .filter(|j| {
            j["input_refs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i == &assembly(2))
        })
        .collect();
    assert_eq!(checks.len(), 2);
    assert_ne!(checks[0]["id"], checks[1]["id"]);
    let mut finishes: Vec<_> = checks
        .iter()
        .map(|j| j["finished_at_ms"].as_u64().unwrap())
        .collect();
    finishes.sort();
    assert_eq!(finishes, vec![70000, 100000]);
    let during = at(&e, 95000, &assembly(2));
    assert_eq!(during["lifecycle"], "in_process");
    assert_eq!(during["holder"]["kind"], "job");
}
#[test]
fn explicit_reinspect_unknown_again_remains_pending_and_does_not_invent_pass_or_rework() {
    let mut v = fixture();
    v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["disposition"] =
        json!("reinspect");
    v["scenario"]["quality_presets"][2]["results"] = json!([null, null]);
    let r = run(&v);
    let third = named(&r, &assembly(2));
    assert_eq!(third["quality"], "pending");
    assert_eq!(third["completed_reworks"], 0);
    assert_eq!(third["holder"]["id"], "pending-store");
    assert_eq!(accepted(&r), 1);
}
#[test]
fn future_wrong_item_and_wrong_exact_inspection_decisions_never_become_cached_release() {
    for variant in ["early", "wrong-item", "wrong-version"] {
        let mut v = fixture();
        let d = &mut v["scenario"]["initial_conditions"]["declared_quality_decisions"][0];
        match variant {
            "early" => d["effective_at_ms"] = json!(10000),
            "wrong-item" => d["item_ref"] = json!("not-the-physical-item"),
            _ => d["inspection_ref"]["version"] = json!("0.2.0"),
        }
        let r = run(&v);
        assert_eq!(named(&r, &assembly(2))["quality"], "pending", "{variant}");
        assert_eq!(accepted(&r), 1, "{variant}");
        assert!(
            r["limitations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["code"] == "simulation.quality_decision_unapplied")
        );
    }
}
#[test]
fn pending_store_reinspection_waits_for_actual_person_before_removing_the_same_pending_item() {
    let mut v = fixture();
    v["scenario"]["window"]["end_ms"] = json!(130000);
    v["scenario"]["acceptance_goal"]["observation_window"] = v["scenario"]["window"].clone();
    v["project"]["calendars"][0]["intervals"] = json!([{"start_ms":0,"end_ms":130000}]);
    let mut calendar = v["project"]["calendars"][0].clone();
    calendar["id"] = json!("Q-LATE");
    calendar["intervals"] =
        json!([{"start_ms":0,"end_ms":85000},{"start_ms":105000,"end_ms":130000}]);
    v["project"]["calendars"]
        .as_array_mut()
        .unwrap()
        .push(calendar);
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "W-Q")
        .unwrap()["calendar_id"] = json!("Q-LATE");
    v["scenario"]["initial_conditions"]["declared_quality_decisions"][0]["disposition"] =
        json!("reinspect");
    v["scenario"]["quality_presets"][2]["results"] = json!([null,{"kind":"string","value":"PASS"}]);
    let mut e = engine(&v);
    e.run_to_end();
    let r = snapshot(&e);
    let waiting = at(&e, 100000, &assembly(2));
    assert_eq!(waiting["quality"], "pending");
    assert_eq!(
        waiting["holder"],
        json!({"kind":"location","id":"pending-store"})
    );
    assert_eq!(
        intervals(&r, "W-Q"),
        vec![
            (20000, 30000),
            (40000, 50000),
            (60000, 70000),
            (75000, 85000),
            (105000, 115000)
        ]
    );
    assert_eq!(intervals(&r, "W-P"), vec![(30000, 40000), (115000, 125000)]);
    assert_eq!(accepted(&r), 2);
    assert_eq!(named(&r, &assembly(2))["completed_reworks"], 0);
}
#[test]
fn scrapped_item_still_occupies_finite_isolation_and_does_not_free_it_for_another_reject() {
    let mut v = fixture();
    let scrap = v["scenario"]["initial_conditions"]["declared_quality_decisions"][1].clone();
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([scrap]);
    v["scenario"]["quality_presets"][2]["results"] =
        json!([{"kind":"string","value":"FAIL"},{"kind":"string","value":"FAIL"}]);
    let mut e = engine(&v);
    e.run_to_end();
    let r = snapshot(&e);
    assert_eq!(named(&r, &assembly(1))["lifecycle"], "scrapped");
    assert_eq!(named(&r, &assembly(1))["holder"]["id"], "quarantine");
    let third = named(&r, &assembly(2));
    assert_eq!(third["quality"], "rejected");
    assert_eq!(third["holder"]["kind"], "job");
    assert_eq!(
        intervals(&r, "W-Q"),
        vec![
            (20000, 30000),
            (40000, 50000),
            (60000, 70000),
            (75000, 85000),
            (90000, 120000)
        ]
    );
    assert_eq!(accepted(&r), 1);
}
