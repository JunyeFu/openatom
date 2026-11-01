//! Expectations originate in the frozen parallel/rework design, before runtime observation.
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_simulation::Engine;
use support::*;

fn inspection_starts(result: &Value) -> usize {
    let ids: Vec<_> = jobs(result, "w5.op.inspect")
        .into_iter()
        .map(|job| job["id"].clone())
        .collect();
    result["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| {
            event["payload"]["kind"] == "job_started" && ids.contains(&event["payload"]["data"])
        })
        .count()
}
fn decisions(result: &Value) -> Vec<Value> {
    let mut values: Vec<Value> = result["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["payload"]["kind"] == "quality_changed")
        .map(|e| e["payload"]["data"].clone())
        .collect();
    values.sort_by_key(|d| d["effective_at_ms"].as_u64().unwrap());
    values
}

fn assembly(result: &Value, raw: &str) -> Value {
    let found: Vec<_> = physical(result, "assembly")
        .into_iter()
        .filter(|item| ancestry(result, item["id"].as_str().unwrap(), raw))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "one raw shell has exactly one physical assembly"
    );
    found[0].clone()
}
fn check_original(case: &str, result: &Value) {
    let expected = original_expectation(case);
    let mut finished: Vec<_> = jobs(result, "w5.op.pack")
        .iter()
        .filter_map(|job| job["finished_at_ms"].as_u64())
        .collect();
    finished.sort();
    let times: Vec<_> = expected["package_finish_seconds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_u64().unwrap() * 1000)
        .collect();
    assert_eq!(finished, times);
    assert_eq!(
        physical(result, "package").len(),
        expected["packages"].as_u64().unwrap() as usize
    );
    assert_eq!(
        physical(result, "assembly")
            .iter()
            .filter(|item| item["quality"] == "rejected"
                && item["holder"] == json!({"kind":"location","id":"quarantine"}))
            .count(),
        expected["quarantined"].as_u64().unwrap() as usize
    );
    assert_eq!(
        physical(result, "assembly")
            .iter()
            .filter(|item| item["quality"] == "pending")
            .count(),
        expected["awaiting_result"].as_u64().unwrap() as usize
    );
    assert_eq!(
        inspection_starts(result),
        expected["inspection_started_count"].as_u64().unwrap() as usize
    );
    assert_eq!(
        decisions(&result).len(),
        expected["quality_decisions"].as_u64().unwrap() as usize
    );
    assert_eq!(
        jobs(result, "w5.op.rework")
            .iter()
            .filter(|job| job["state"] == "completed")
            .count(),
        expected["rework_cycles"].as_u64().unwrap() as usize
    );
    assert_eq!(
        physical(result, "assembly").len(),
        2,
        "inspection/rework alternatives never duplicate an assembly"
    );
    for package in physical(result, "package") {
        assert_eq!(
            package["holder"],
            json!({"kind":"location","id":"finished"})
        );
        let contents = package["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        let contained = named(result, contents[0].as_str().unwrap());
        assert_eq!(contained["quality"], "released");
        assert_eq!(
            contained["holder"],
            json!({"kind":"container","id":package["id"]})
        );
    }
    for worker in ["W-A", "W-Q", "W-R", "W-P"] {
        for pair in intervals(result, worker).windows(2) {
            assert!(pair[0].1 <= pair[1].0, "actual worker {worker} overlaps");
        }
    }
}

#[test]
fn original_a_parallel_join_rework_and_reinspection_finish_at_145_and_175_seconds() {
    let result = run(&parallel("A"));
    check_original("A", &result);
    assert_eq!(
        intervals(&result, "W-A"),
        vec![(30000, 90000), (90000, 150000)]
    );
    assert_eq!(
        intervals(&result, "W-Q"),
        vec![(90000, 100000), (120000, 130000), (150000, 160000)]
    );
    assert_eq!(intervals(&result, "W-R"), vec![(100000, 120000)]);
    assert_eq!(
        intervals(&result, "W-P"),
        vec![(130000, 145000), (160000, 175000)]
    );
    let first = assembly(&result, "raw_shell-001");
    let second = assembly(&result, "raw_shell-002");
    assert_eq!(first["completed_reworks"], 1);
    assert_eq!(second["completed_reworks"], 0);
    let decisions = decisions(&result);
    assert_eq!(
        decisions
            .iter()
            .map(|d| d["effective_at_ms"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![100000, 130000, 160000]
    );
    assert_eq!(decisions[0]["item_ref"], first["id"]);
    assert_eq!(decisions[1]["item_ref"], first["id"]);
    assert_eq!(decisions[2]["item_ref"], second["id"]);
    assert_eq!(
        decisions
            .iter()
            .map(|d| d["disposition"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["rework", "release", "release"]
    );
}

#[test]
fn original_q_keeps_the_twice_failed_identity_in_isolation_and_only_second_product_finishes_at_175()
{
    let result = run(&parallel("Q"));
    check_original("Q", &result);
    let first = assembly(&result, "raw_shell-001");
    assert_eq!(first["completed_reworks"], 1);
    assert_eq!(first["quality"], "rejected");
    assert_eq!(
        first["holder"],
        json!({"kind":"location","id":"quarantine"})
    );
    assert_eq!(first["quality"], "rejected");
    assert_eq!(
        first["holder"],
        json!({"kind":"location","id":"quarantine"})
    );
    assert!(
        !result["final_items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["lifecycle"] == "scrapped")
    );
    let d = decisions(&result);
    assert_eq!(d[0]["item_ref"], d[1]["item_ref"]);
    assert_eq!(d[1]["disposition"], "isolate");
    assert_eq!(physical(&result, "carton").len(), 1);
}

#[test]
fn original_u_null_is_pending_without_a_quality_decision_and_keeps_the_inspector_until_the_window_end()
 {
    let result = run(&parallel("U"));
    check_original("U", &result);
    let first = assembly(&result, "raw_shell-001");
    assert_eq!(first["quality"], "pending");
    assert_eq!(first["holder"]["kind"], "job");
    assert_eq!(intervals(&result, "W-Q"), vec![(90000, 180000)]);
    let inspect = jobs(&result, "w5.op.inspect");
    assert_eq!(
        inspect
            .iter()
            .filter(|j| j["started_at_ms"].is_null())
            .count(),
        1
    );
    let held = inspect
        .iter()
        .find(|j| j["id"] == first["holder"]["id"])
        .unwrap();
    assert!(held["finished_at_ms"].is_null());
    let physical_work: u64 = result["phase_work_intervals"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["job_id"] == held["id"])
        .map(|w| {
            w["interval"]["end_ms"].as_u64().unwrap() - w["interval"]["start_ms"].as_u64().unwrap()
        })
        .sum();
    assert_eq!(
        physical_work, 10000,
        "finished inspection work is distinct from waiting for a result"
    );
    assert_eq!(physical(&result, "carton").len(), 2);
    assert!(
        result["resource_claims"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["resource_id"] == "W-Q"
                && c["interval_end_causes"] == json!(["observation_end"]))
    );
}

#[test]
fn a_missing_parallel_pcb_branch_never_partially_acquires_the_assembly_kit_or_assembler() {
    let mut value = parallel("A");
    value["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .retain(|item| item["item_type_ref"]["id"] != "raw_pcb");
    let result = run(&value);
    assert_eq!(physical(&result, "shell").len(), 2);
    assert!(physical(&result, "assembly").is_empty());
    assert!(physical(&result, "package").is_empty());
    assert!(
        jobs(&result, "w5.op.assembly")
            .iter()
            .all(|job| job["started_at_ms"].is_null())
    );
    assert!(intervals(&result, "W-A").is_empty());
    assert!(intervals(&result, "CAP-ASM-01").is_empty());
    assert!(
        physical(&result, "shell")
            .iter()
            .all(|item| item["holder"] == json!({"kind":"location","id":"shell-ready"}))
    );
}

#[test]
fn rework_completion_keeps_the_same_identity_and_requires_reinspection_instead_of_auto_release() {
    let value = parallel("A");
    let mut engine = engine(&value);
    engine.run_to_end();
    let result = snapshot(&engine);
    let first = assembly(&result, "raw_shell-001");
    let id = first["id"].as_str().unwrap();
    let at = |t| {
        serde_json::to_value(
            engine
                .objects_at(serde_json::from_value(json!(t)).unwrap())
                .unwrap(),
        )
        .unwrap()
    };
    let before = at(119999);
    let before = before
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap();
    assert_eq!(before["completed_reworks"], 0);
    assert_ne!(before["quality"], "released");
    let after = at(120000);
    let after = after
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == id)
        .unwrap();
    assert_eq!(after["completed_reworks"], 1);
    assert_ne!(after["quality"], "released");
    assert_eq!(after["holder"]["kind"], "job");
    assert!(
        !at(129999)
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["item_type_ref"]["id"] == "package")
    );
}

#[test]
fn exhausted_result_sequence_is_unknown_on_reinspection_and_does_not_advance_the_other_inspection()
{
    let mut value = parallel("A");
    value["scenario"]["quality_presets"][0]["results"] = json!([{"kind":"string","value":"FAIL"}]);
    let result = run(&value);
    assert!(physical(&result, "package").is_empty());
    let first = assembly(&result, "raw_shell-001");
    assert_eq!(first["completed_reworks"], 1);
    assert_eq!(first["quality"], "pending");
    assert_eq!(first["holder"]["kind"], "job");
    assert_eq!(decisions(&result).len(), 1);
    assert_eq!(inspection_starts(&result), 2);
    assert_eq!(held_ms(&result, "W-Q"), 70000);
}

#[test]
fn a_full_pass_destination_holds_one_reinspected_item_and_the_inspector_without_duplicate_decisions()
 {
    let mut value = parallel("A");
    value["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "passed-in")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(0);
    let result = run(&value);
    let first = assembly(&result, "raw_shell-001");
    assert_eq!(first["quality"], "released");
    assert_eq!(first["holder"]["kind"], "job");
    assert!(physical(&result, "package").is_empty());
    assert_eq!(decisions(&result).len(), 2);
    let job = jobs(&result, "w5.op.inspect")
        .into_iter()
        .find(|j| j["id"] == first["holder"]["id"])
        .unwrap();
    assert_eq!(job["state"], "waiting_output");
    assert!(job["finished_at_ms"].is_null());
    assert_eq!(job["pending_outputs"], json!([first["id"]]));
    assert_eq!(
        intervals(&result, "W-Q"),
        vec![(90000, 100000), (120000, 180000)]
    );
}

#[test]
fn zero_rework_limit_isolates_the_first_failure_and_does_not_consume_the_unused_pass_result() {
    let mut value = parallel("A");
    value["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.inspect")
        .unwrap()["payload"]["data"]["quality_and_routes"]["max_completed_reworks"] = json!(0);
    let result = run(&value);
    let first = assembly(&result, "raw_shell-001");
    assert_eq!(first["quality"], "rejected");
    assert_eq!(
        first["holder"],
        json!({"kind":"location","id":"quarantine"})
    );
    assert_eq!(first["completed_reworks"], 0);
    assert!(
        jobs(&result, "w5.op.rework")
            .iter()
            .all(|j| j["started_at_ms"].is_null())
    );
    assert_eq!(decisions(&result).len(), 2);
    assert_eq!(physical(&result, "package").len(), 1);
}

#[test]
fn wrong_inspection_version_and_ambiguous_ancestor_presets_are_located_before_any_execution() {
    for duplicate in [false, true] {
        let mut value = parallel("A");
        if duplicate {
            let mut extra = value["scenario"]["quality_presets"][0].clone();
            extra["id"] = json!("AMBIGUOUS");
            value["scenario"]["quality_presets"]
                .as_array_mut()
                .unwrap()
                .push(extra);
        } else {
            value["scenario"]["quality_presets"][0]["inspection_module"]["version"] =
                json!("0.2.0");
        }
        let before = value.clone();
        let issues = match Engine::new(input(&value)) {
            Err(issues) => issues,
            Ok(_) => panic!("ambiguous/wrong-version quality source admitted"),
        };
        assert!(issues.iter().any(|i| {
            i.field_path
                .as_deref()
                .is_some_and(|p| p.contains("quality_presets"))
        }));
        assert_eq!(value, before);
    }
}
