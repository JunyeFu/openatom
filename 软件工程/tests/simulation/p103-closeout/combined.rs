#[path = "support.rs"]
mod noncu_resources_support;
use noncu_resources_support::*;
use original::*;
use serde_json::{Value, json};
use sf_domain::*;

fn time(t: u64) -> SimTimeMs {
    SimTimeMs(t.try_into().unwrap())
}

#[test]
fn future_arrival_shift_interruption_unload_and_output_wait_form_one_owned_timeline() {
    let mut cases = vec![];
    for policy in ["pause_resume", "non_preemptive"] {
        for capacity in [0, 1] {
            let v = combined(policy, capacity);
            let before = v.clone();
            let mut e = engine(&v);
            e.run_to_end();
            let result = snapshot(&e);
            assert_eq!(v, before);
            assert_eq!(work(&result), expected_work(policy, 80000));
            assert_eq!(held_ms(&result, "W-C"), 10000);
            assert_eq!(
                intervals(&result, "W-C"),
                vec![(10000, 15000), (80000, 85000)]
            );
            assert_eq!(
                intervals(&result, "CAP-CNC"),
                vec![(10000, if capacity == 0 { 100000 } else { 85000 })]
            );
            let job = &result["jobs"][0];
            assert_eq!(
                job["finished_at_ms"],
                if capacity == 0 {
                    Value::Null
                } else {
                    json!(85000)
                }
            );
            assert_eq!(physical(&result, "part").len(), 1);
            assert_eq!(arrival_count(&result), 1);
            assert!(
                !e.objects_at(time(9999))
                    .unwrap()
                    .iter()
                    .any(|i| i.id.as_str() == "noncu-future-raw")
            );
            let received = e.objects_at(time(10000)).unwrap();
            assert_eq!(
                received
                    .iter()
                    .filter(|i| i.id.as_str() == "noncu-future-raw")
                    .count(),
                1
            );
            for at in [20000, 30000, 39999, 65000, 70000, 79999] {
                let jobs = e.jobs_at(time(at)).unwrap();
                let j = &jobs[0];
                assert!(j.finished_at_ms.is_none());
                assert!(j.job_resources.iter().any(|r| r.as_str() == "CAP-CNC"));
                assert!(!j.phase_resources.iter().any(|r| r.as_str() == "W-C"));
            }
            let auto: Vec<_> = result["phase_work_intervals"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|w| w["phase_id"] == "auto")
                .collect();
            if policy == "pause_resume" {
                assert_eq!(auto[0]["end_cause"], "interrupted");
                assert_eq!(auto[0]["effective_progress_end_ms"], 5000);
                assert_eq!(auto[1]["effective_progress_start_ms"], 5000);
            }
            let claim = result["resource_claims"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["resource_id"] == "CAP-CNC")
                .unwrap();
            assert_eq!(
                claim["interval_end_causes"],
                if capacity == 0 {
                    json!(["observation_end"])
                } else {
                    json!(["released"])
                }
            );
            if capacity == 0 {
                assert_eq!(job["state"], "waiting_output");
                assert!(
                    result["waits"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(
                            |w| w["interval"] == json!({"start_ms":85000,"end_ms":100000})
                                && w["end_cause"] == "observation_end"
                        )
                );
                assert_eq!(physical(&result, "part")[0]["holder"]["kind"], "job");
            }
            cases.push(json!({"policy":policy,"capacity":capacity,"input":v,"run":result,"history":[e.jobs_at(time(19999)),e.jobs_at(time(20000)),e.jobs_at(time(40000)),e.jobs_at(time(85000))]}));
        }
    }
    evidence("combined.json", &json!({"passed":true,"cases":cases}));
}

#[test]
fn pause_resume_and_cancel_keep_the_same_prefix_claims_and_never_receive_arrival_twice() {
    let v = combined("pause_resume", 0);
    let uninterrupted = run(&v);
    let mut e = engine(&v);
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|t| t.0.value() < 20000)
    {
        e.step();
    }
    e.pause();
    let prefix = snapshot(&e);
    assert_eq!(prefix["computed_until_ms"], 20000);
    assert_eq!(prefix["whole_window_computed"], false);
    assert_eq!(prefix["jobs"][0]["effective_progress_ms"], 5000);
    assert_eq!(intervals(&prefix, "CAP-CNC"), vec![(10000, 20000)]);
    assert_eq!(
        prefix["resource_claims"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["resource_id"] == "CAP-CNC")
            .unwrap()["interval_end_causes"],
        json!(["computed_boundary"])
    );
    assert!(e.objects_at(time(20001)).is_none());
    e.resume();
    e.run_to_end();
    assert_eq!(snapshot(&e), uninterrupted);
    assert_eq!(arrival_count(&snapshot(&e)), 1);
    let mut cancelled = engine(&v);
    while cancelled
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|t| t.0.value() < 20000)
    {
        cancelled.step();
    }
    cancelled.cancel();
    let frozen = snapshot(&cancelled);
    cancelled.run_to_end();
    assert_eq!(snapshot(&cancelled), frozen);
    assert_eq!(frozen["computation_status"], "cancelled");
    assert_eq!(frozen["computed_until_ms"], 20000);
    assert_eq!(arrival_count(&frozen), 1);
    assert!(frozen["jobs"][0]["finished_at_ms"].is_null());
    evidence(
        "prefix.json",
        &json!({"passed":true,"input":v,"paused":prefix,"cancelled":frozen,"resumed":uninterrupted}),
    );
}

#[test]
fn arrival_at_end_uses_closed_shift_or_adjacent_open_shift_before_zero_work_closure() {
    let mut rows = vec![];
    for adjacent in [false, true] {
        let mut v = combined("non_preemptive", 1);
        v["scenario"]["window"]["end_ms"] = json!(10000);
        for c in v["project"]["calendars"].as_array_mut().unwrap() {
            c["intervals"] = if adjacent {
                json!([{"start_ms":0,"end_ms":10000},{"start_ms":10000,"end_ms":20000}])
            } else {
                json!([{"start_ms":0,"end_ms":10000}])
            };
        }
        for p in operation(&mut v)["phases"].as_array_mut().unwrap() {
            p["duration"]["value"] = json!(0);
        }
        let result = run(&v);
        assert_eq!(arrival_count(&result), 1);
        assert_eq!(held_ms(&result, "CAP-CNC"), 0);
        assert_eq!(held_ms(&result, "W-C"), 0);
        assert_eq!(physical(&result, "part").len(), usize::from(adjacent));
        if adjacent {
            assert_eq!(result["jobs"][0]["finished_at_ms"], 10000);
        } else {
            assert!(result["jobs"][0]["started_at_ms"].is_null());
            assert_eq!(
                named(&result, "noncu-future-raw")["holder"],
                json!({"kind":"location","id":"raw-store"})
            );
        }
        rows.push(json!({"adjacent":adjacent,"input":v,"run":result}));
    }
    evidence("same-instant.json", &json!({"passed":true,"cases":rows}));
}

#[test]
fn explicit_phase_qualification_unit_and_capacity_are_checked_before_future_material_is_taken() {
    let mut qualified = combined("pause_resume", 1);
    operation(&mut qualified)["phases"][0]["phase_resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["role"] == "worker")
        .unwrap()["qualifications"] = json!(["cnc-trained"]);
    qualified["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == "W-C")
        .unwrap()["qualifications"] = json!(["cnc-trained"]);
    assert_eq!(work(&run(&qualified)), expected_work("pause_resume", 80000));
    let mut rejected = vec![];
    for kind in ["qualification", "unit", "capacity"] {
        let mut candidate = qualified.clone();
        let worker = candidate["project"]["resources"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == "W-C")
            .unwrap();
        match kind {
            "qualification" => worker["qualifications"] = json!([]),
            "unit" => worker["capacity"]["unit"] = json!("slot"),
            "capacity" => worker["capacity"]["value"]["coefficient"] = json!(0),
            _ => unreachable!(),
        }
        let before = candidate.clone();
        let issues = sf_simulation::check_inputs(&input(&candidate)).unwrap_err();
        assert!(
            issues
                .iter()
                .any(|i| i.code == "simulation.resource_qualification"),
            "{kind}: {issues:?}"
        );
        assert_eq!(candidate, before);
        assert!(sf_simulation::Engine::new(input(&candidate)).is_err());
        rejected.push(json!({"kind":kind,"input":candidate,"issues":issues}));
    }
    evidence(
        "qualification.json",
        &json!({"passed":true,"qualified_input":qualified,"rejections":rejected}),
    );
}
