//! Actual Job-history consumers; original fixtures and expectations stay read-only.
#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{Engine, EngineInput, jobs_at_from_history};
use support::{engine, phased};

fn t(value: u64) -> SimTimeMs {
    SimTimeMs(value.try_into().unwrap())
}
fn at(engine: &Engine, time: u64) -> JobState {
    engine
        .jobs_at(t(time))
        .unwrap()
        .into_iter()
        .find(|j| matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..} if device_id.as_str()=="CNC"))
        .unwrap()
}
fn sorted(mut jobs: Vec<JobState>) -> Value {
    jobs.sort_by(|a, b| a.id.cmp(&b.id));
    serde_json::to_value(jobs).unwrap()
}
fn op(input: &mut Value) -> &mut Value {
    &mut input["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.phased")
        .unwrap()["payload"]["data"]
}
fn gap(policy: &str) -> Value {
    let mut v = phased();
    v["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"MACHINE-GAP","title":"explicit gap","source":"user_provided","intervals":[{"start_ms":0,"end_ms":20000},{"start_ms":40000,"end_ms":100000}]}));
    for r in v["project"]["resources"].as_array_mut().unwrap() {
        if r["id"] == "CAP-CNC" {
            r["calendar_id"] = json!("MACHINE-GAP");
        }
        if r["id"] == "W-C" {
            r["calendar_id"] = json!("ALL");
        }
    }
    op(&mut v)["phases"][1]["interruption_policy"] = json!(policy);
    v
}
fn runtime() -> std::path::PathBuf {
    let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/真实业务设计/runtime-job-history");
    std::fs::create_dir_all(&here).unwrap();
    here
}

#[test]
fn actual_closed_instants_and_between_event_progress_keep_45s_10s_worker_expectation() {
    let mut e = engine(&phased());
    assert!(e.jobs_at(t(0)).is_none());
    assert!(e.job_history().is_empty());
    while !e.is_finished() {
        e.step();
        let run = e.snapshot();
        let until = run.computed_until_ms.clone().unwrap();
        assert_eq!(sorted(e.jobs_at(until).unwrap()), sorted(run.jobs));
    }
    for (time, phase, progress, status) in [
        (0, Some("load"), 0, "working"),
        (2500, Some("load"), 2500, "working"),
        (5000, Some("auto"), 0, "working"),
        (20000, Some("auto"), 15000, "working"),
        (34999, Some("auto"), 29999, "working"),
        (35000, Some("unload"), 0, "waiting_resource"),
        (39999, Some("unload"), 0, "waiting_resource"),
        (40000, Some("unload"), 0, "working"),
        (42500, Some("unload"), 2500, "working"),
        (45000, None, 0, "completed"),
    ] {
        let job = at(&e, time);
        assert_eq!(job.phase_ref.as_ref().map(|p| p.as_str()), phase, "{time}");
        assert_eq!(job.effective_progress_ms.0.value(), progress, "{time}");
        assert_eq!(serde_json::to_value(&job.state).unwrap(), status, "{time}");
        if time < 45000 {
            assert!(job.finished_at_ms.is_none());
            assert!(job.job_resources.iter().any(|r| r.as_str() == "CAP-CNC"));
        }
        if (5000..40000).contains(&time) {
            assert!(!job.phase_resources.iter().any(|r| r.as_str() == "W-C"));
        }
    }
    let run = e.snapshot();
    assert_eq!(at(&e, 45000).finished_at_ms.unwrap().0.value(), 45000);
    let worker: u64 = run
        .resource_claims
        .iter()
        .filter(|c| c.resource_id.as_str() == "W-C")
        .flat_map(|c| &c.intervals)
        .map(|i| i.end_ms.0.value() - i.start_ms.0.value())
        .sum();
    assert_eq!(worker, 10000);
    std::fs::write(runtime().join("phase-calendar.actual.json"),serde_json::to_vec_pretty(&json!({"run":run,"job_history":e.job_history(),"observations":[at(&e,2500),at(&e,35000),at(&e,42500)]})).unwrap()).unwrap();
}

#[test]
fn calendar_interruption_freezes_effective_work_and_nonpreemptive_wait_never_invents_progress() {
    let mut e = engine(&gap("pause_resume"));
    e.run_to_end();
    for (time, progress, status) in [
        (20000, 15000, "waiting_resource"),
        (30000, 15000, "waiting_resource"),
        (39999, 15000, "waiting_resource"),
        (40000, 15000, "working"),
        (50000, 25000, "working"),
    ] {
        let job = at(&e, time);
        assert_eq!(job.effective_progress_ms.0.value(), progress);
        assert_eq!(serde_json::to_value(job.state).unwrap(), status);
        assert_eq!(job.phase_ref.unwrap().as_str(), "auto");
        assert!(job.job_resources.iter().any(|r| r.as_str() == "CAP-CNC"));
    }
    assert_eq!(at(&e, 60000).finished_at_ms.unwrap().0.value(), 60000);
    let mut nonpreemptive = engine(&gap("non_preemptive"));
    nonpreemptive.run_to_end();
    for time in [5000, 20000, 39999] {
        let job = at(&nonpreemptive, time);
        assert!(matches!(job.state, JobStatus::WaitingResource));
        assert_eq!(job.effective_progress_ms.0.value(), 0);
    }
    assert_eq!(
        at(&nonpreemptive, 75000).finished_at_ms.unwrap().0.value(),
        75000
    );
}

#[test]
fn future_arrival_and_unreleased_job_have_no_future_inputs_or_completion() {
    let mut v = phased();
    let mut item = v["scenario"]["initial_conditions"]["items"][0].clone();
    item["id"] = json!("history-incoming");
    v["scenario"]["initial_conditions"]["items"] = json!([]);
    v["scenario"]["arrivals"] = json!([{"id":"history-arrival","at_ms":10000,"destination":"raw-store","items":[item],"release_policy":"unchecked_allowed"}]);
    v["scenario"]["production_tasks"][0]["release_at_ms"] = json!(10000);
    let mut e = engine(&v);
    e.run_to_end();
    let before = at(&e, 9999);
    assert!(before.started_at_ms.is_none());
    assert!(before.input_refs.is_empty());
    assert!(before.pending_outputs.is_empty());
    assert_eq!(before.effective_progress_ms.0.value(), 0);
    assert!(
        !e.objects_at(t(9999))
            .unwrap()
            .iter()
            .any(|i| i.id.as_str() == "history-incoming")
    );
    let start = at(&e, 10000);
    assert_eq!(start.started_at_ms.unwrap().0.value(), 10000);
    assert!(!start.input_refs.is_empty());
    assert!(start.finished_at_ms.is_none());
    assert_eq!(at(&e, 50000).finished_at_ms.unwrap().0.value(), 50000);
}

#[test]
fn pending_output_keeps_exact_item_ids_machine_and_wait_without_work_progress() {
    let mut v = phased();
    v["project"]["locations"][1]["capacity"]["value"]["coefficient"] = json!(0);
    let mut e = engine(&v);
    e.run_to_end();
    for time in [45000, 70000, 100000] {
        let job = at(&e, time);
        assert!(matches!(job.state, JobStatus::WaitingOutput));
        assert!(job.finished_at_ms.is_none());
        assert!(job.phase_ref.is_none());
        assert!(job.phase_resources.is_empty());
        assert_eq!(job.effective_progress_ms.0.value(), 0);
        assert_eq!(job.pending_outputs.len(), 1);
        assert!(job.job_resources.iter().any(|r| r.as_str() == "CAP-CNC"));
        let items = e.objects_at(t(time)).unwrap();
        assert!(items.iter().any(|i| job.pending_outputs.contains(&i.id)
            && matches!(&i.holder,HolderRef::Job(id) if id==&job.id)));
    }
}

#[test]
fn paused_cancelled_frontier_and_persisted_history_are_readonly_without_final_job_fallback() {
    let mut e = engine(&phased());
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|x| x.0.value() < 20000)
    {
        e.step();
    }
    e.pause();
    let before = serde_json::to_value(e.snapshot()).unwrap();
    let history = e.job_history();
    assert!(e.jobs_at(t(20001)).is_none());
    assert_eq!(e.step(), 0);
    assert_eq!(serde_json::to_value(e.snapshot()).unwrap(), before);
    assert_eq!(at(&e, 17500).effective_progress_ms.0.value(), 12500);
    assert_eq!(at(&e, 20000).effective_progress_ms.0.value(), 15000);
    let encoded = serde_json::to_vec(&history).unwrap();
    let restored: Vec<(SimTimeMs, JobState)> = serde_json::from_slice(&encoded).unwrap();
    for time in [0, 2500, 5000, 17500, 20000] {
        let run = e.snapshot();
        assert_eq!(
            sorted(
                jobs_at_from_history(
                    &restored,
                    &run.phase_work_intervals,
                    run.window.start_ms.clone(),
                    run.computed_until_ms.clone(),
                    t(time)
                )
                .unwrap()
            ),
            sorted(e.jobs_at(t(time)).unwrap())
        );
    }
    assert!(jobs_at_from_history(&restored, &[], t(0), None, t(0)).is_none());
    assert!(jobs_at_from_history(&restored, &[], t(5), Some(t(20000)), t(4)).is_none());
    let mut returned = e.jobs_at(t(0)).unwrap();
    returned[0].input_refs.clear();
    returned[0]
        .pending_outputs
        .push("not-an-actual-item".to_owned().try_into().unwrap());
    assert_eq!(serde_json::to_value(e.snapshot()).unwrap(), before);
    assert_eq!(
        serde_json::to_value(e.job_history()).unwrap(),
        serde_json::to_value(&history).unwrap()
    );
    e.cancel();
    assert!(e.jobs_at(t(20001)).is_none());
    assert_eq!(e.step(), 0);
    assert_eq!(at(&e, 20000).effective_progress_ms.0.value(), 15000);
    let mut completed = engine(&phased());
    completed.run_to_end();
    assert!(completed.jobs_at(t(100001)).is_none());
    assert_eq!(
        sorted(completed.jobs_at(t(17500)).unwrap()),
        sorted(
            jobs_at_from_history(
                &restored,
                &e.snapshot().phase_work_intervals,
                t(0),
                Some(t(20000)),
                t(17500)
            )
            .unwrap()
        )
    );
    std::fs::write(
        runtime().join("paused-history.roundtrip.json"),
        serde_json::to_vec_pretty(
            &json!({"run":e.snapshot(),"job_history":restored,"at17500":e.jobs_at(t(17500))}),
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn zero_work_closure_has_only_final_state_at_the_actual_instant_and_no_phantom_duration() {
    for release in [0, 10000] {
        let mut v = phased();
        for phase in op(&mut v)["phases"].as_array_mut().unwrap() {
            phase["duration"]["value"] = json!(0);
            phase["phase_resources"] = json!([]);
        }
        v["scenario"]["production_tasks"][0]["release_at_ms"] = json!(release);
        let mut e = engine(&v);
        e.run_to_end();
        if release > 0 {
            assert!(at(&e, release - 1).started_at_ms.is_none());
        }
        let job = at(&e, release);
        assert!(matches!(job.state, JobStatus::Completed));
        assert_eq!(job.started_at_ms.unwrap().0.value(), release);
        assert_eq!(job.finished_at_ms.unwrap().0.value(), release);
        assert!(job.pending_outputs.is_empty());
        assert_eq!(job.effective_progress_ms.0.value(), 0);
        assert!(e.snapshot().phase_work_intervals.is_empty());
        let records: Vec<_> = e
            .job_history()
            .into_iter()
            .filter(|(when, _)| when.0.value() == release)
            .collect();
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].1.state, JobStatus::Completed));
    }
}

#[test]
fn original_p303_55_to70_resource_and75_to85_output_waits_have_actual_history() {
    let declared: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.input.json"
    ))
    .unwrap();
    // The actual Host bootstrap contains the frozen identities and the adopted
    // exact device version. The earlier preparation input_ref still names PHASE,
    // not P303-PHASE; never rewrite that old input to make it look captured.
    let ready: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.bootstrap.actual.json"
    ))
    .unwrap();
    let formal = &ready["formal_run"];
    let scenario: ScenarioInput =
        serde_json::from_value(ready["snapshot"]["scenarios"][0].clone()).unwrap();
    let mut e = Engine::new(EngineInput {
        run_id: serde_json::from_value(formal["id"].clone()).unwrap(),
        input_ref: serde_json::from_value(formal["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(ready["snapshot"]["project"].clone()).unwrap(),
        scope: scenario.scope.clone(),
        scenario,
        definitions: serde_json::from_value(declared["definitions"].clone()).unwrap(),
    })
    .unwrap();
    e.run_to_end();
    for (time, status, phase, progress) in [
        (54999, "working", Some("auto"), 44999),
        (55000, "waiting_resource", Some("unload"), 0),
        (60000, "waiting_resource", Some("unload"), 0),
        (70000, "working", Some("unload"), 0),
        (72500, "working", Some("unload"), 2500),
        (75000, "waiting_output", None, 0),
        (80000, "waiting_output", None, 0),
        (85000, "completed", None, 0),
    ] {
        let job = at(&e, time);
        assert_eq!(serde_json::to_value(&job.state).unwrap(), status, "{time}");
        assert_eq!(job.phase_ref.as_ref().map(|p| p.as_str()), phase, "{time}");
        assert_eq!(job.effective_progress_ms.0.value(), progress, "{time}");
    }
    let result = e.snapshot();
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        *formal,
        "unchanged entire actual P303 Host Run"
    );
    assert!(result.waits.iter().any(|w|w.interval.start_ms.0.value()==55000&&w.interval.end_ms.0.value()==70000));
    assert!(result.waits.iter().any(|w|w.interval.start_ms.0.value()==75000&&w.interval.end_ms.0.value()==85000));
    std::fs::write(runtime().join("p303-actual-history.json"),serde_json::to_vec_pretty(&json!({"run":result,"job_history":e.job_history(),"observations":[at(&e,60000),at(&e,80000)]})).unwrap()).unwrap();
}

#[test]
fn original_actual_root_w5_seed_and_policy_runs_remain_entirely_equal() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baseline = root.join("scenarios/模板与交接/runtime/source-clock");
    let read = |path: std::path::PathBuf| -> Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    };
    let bundled =
        read(root.join("modules/templates/purchased-assembly/candidate/definitions.json"));
    for name in [
        "root_original",
        "root_sdk_mapped",
        "w5_original_source",
        "w5_original_target",
        "existing_seed",
        "internal-pass",
        "external-pass",
        "internal-pending",
    ] {
        let original = read(baseline.join(format!("{name}.actual.json")));
        let project = read(baseline.join(format!("{name}.project.json")));
        let scenario: ScenarioInput =
            serde_json::from_value(read(baseline.join(format!("{name}.scenario.json")))).unwrap();
        let definitions = if name.starts_with("root_") || name.starts_with("w5_") {
            bundled.clone()
        } else if name == "existing_seed" {
            read(baseline.join("../seed-composition/engine-definitions.json"))
        } else {
            read(baseline.join(format!("../setup-composition/{name}.input.json")))["definitions"]
                .clone()
        };
        let mut e = Engine::new(EngineInput {
            run_id: serde_json::from_value(original["id"].clone()).unwrap(),
            input_ref: serde_json::from_value(original["input_ref"].clone()).unwrap(),
            project: serde_json::from_value(project).unwrap(),
            scope: scenario.scope.clone(),
            scenario,
            definitions: serde_json::from_value(definitions).unwrap(),
        })
        .unwrap();
        while !e.is_finished() {
            e.step();
            let run = e.snapshot();
            assert_eq!(
                sorted(e.jobs_at(run.computed_until_ms.clone().unwrap()).unwrap()),
                sorted(run.jobs),
                "actual closed instant: {name}"
            );
        }
        assert_eq!(
            serde_json::to_value(e.snapshot()).unwrap(),
            original,
            "entire original Run unchanged: {name}"
        );
        std::fs::write(runtime().join(format!("{name}.actual.json")),serde_json::to_vec_pretty(&json!({"unchanged_entire_Run":true,"run":e.snapshot(),"job_history":e.job_history()})).unwrap()).unwrap();
    }
}

#[test]
fn legacy_single_phase_history_keeps_whole_job_worker_claim_and_actual_progress() {
    let value: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap();
    let mut e = engine(&value);
    e.run_to_end();
    let final_run = e.snapshot();
    let assembly = final_run
        .jobs
        .iter()
        .find(|j| j.operation_ref.id.as_str() == "fixture.assemble")
        .unwrap();
    let job = e
        .jobs_at(t(22500))
        .unwrap()
        .into_iter()
        .find(|j| j.id == assembly.id)
        .unwrap();
    assert!(matches!(job.state, JobStatus::Working));
    assert_eq!(job.effective_progress_ms.0.value(), 22500);
    assert!(job.finished_at_ms.is_none());
    assert!(!job.input_refs.is_empty());
    let workers: Vec<_> = value["project"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "worker")
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(
        job.job_resources
            .iter()
            .chain(&job.phase_resources)
            .any(|id| workers.contains(&id.as_str()))
    );
    assert!(final_run.resource_claims.iter().any(|claim| {
        claim.job_id == assembly.id
            && workers.contains(&claim.resource_id.as_str())
            && claim
                .intervals
                .iter()
                .any(|i| i.start_ms.0.value() == 0 && i.end_ms.0.value() == 60000)
    }));
}

#[test]
fn history_saved_moved_and_reopened_without_engine_still_reads_the_same_jobs() {
    let mut e = engine(&gap("pause_resume"));
    e.step();
    e.pause();
    let before = sorted(e.jobs_at(t(0)).unwrap());
    assert_eq!(e.step(), 0);
    assert_eq!(sorted(e.jobs_at(t(0)).unwrap()), before);
    e.resume();
    e.run_to_end();
    let points = [0, 2500, 5000, 20000, 30000, 40000, 50000, 55000, 60000];
    let expected: Vec<_> = points
        .iter()
        .map(|time| sorted(e.jobs_at(t(*time)).unwrap()))
        .collect();
    let dir = runtime().join(format!("actual-history-move-{}", std::process::id()));
    let source = dir.join("original");
    let moved = dir.join("moved");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("history.json"),
        serde_json::to_vec_pretty(&json!({"records":e.job_history(),"run":e.snapshot()})).unwrap(),
    )
    .unwrap();
    drop(e);
    std::fs::rename(&source, &moved).unwrap();
    let reopened: Value =
        serde_json::from_slice(&std::fs::read(moved.join("history.json")).unwrap()).unwrap();
    let records: Vec<(SimTimeMs, JobState)> =
        serde_json::from_value(reopened["records"].clone()).unwrap();
    let run: RunResult = serde_json::from_value(reopened["run"].clone()).unwrap();
    for (time, want) in points.iter().zip(expected) {
        let found = jobs_at_from_history(
            &records,
            &run.phase_work_intervals,
            run.window.start_ms.clone(),
            run.computed_until_ms.clone(),
            t(*time),
        )
        .unwrap();
        assert_eq!(sorted(found), want, "{time}");
    }
    assert!(!source.exists());
    assert!(moved.join("history.json").is_file());
}
