use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{
    Engine, EngineInput, check_inputs, check_inputs_with_goal, execute_engine,
    execute_engine_with_goal,
};
use std::sync::atomic::AtomicBool;

fn base() -> Value {
    serde_json::from_str(include_str!(
        "../../../packages/commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap()
}
fn business() -> Value {
    serde_json::from_str(include_str!(
        "../../../scenarios/真实业务设计/business-inputs.design.json"
    ))
    .unwrap()
}
fn expected(id: &str) -> Value {
    let j: Value = serde_json::from_str(include_str!(
        "../../../scenarios/真实业务设计/expected-business.design.json"
    ))
    .unwrap();
    j["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
        .clone()
}
fn boundary(id: &str) -> Value {
    let j: Value = serde_json::from_str(include_str!(
        "../../../scenarios/真实业务设计/business-rule-boundaries.design.json"
    ))
    .unwrap();
    j["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .unwrap()
        .clone()
}
fn q(n: u64) -> Value {
    json!({"value":{"coefficient":n,"decimal_places":0},"unit":"piece"})
}
fn duration(n: u64) -> Value {
    json!({"value":n,"unit":"millisecond","basis":"user_provided","source_ref":null})
}
fn t(n: u64) -> SimTimeMs {
    SimTimeMs(SafeUint::try_from(n).unwrap())
}
fn input(v: Value) -> EngineInput {
    let scenario: ScenarioInput = serde_json::from_value(v["scenario"].clone()).unwrap();
    EngineInput {
        run_id: RunId::try_from("P1-03-owner".to_owned()).unwrap(),
        input_ref: serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        scope: scenario.scope.clone(),
        scenario,
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
    }
}
fn engine(v: Value) -> Engine {
    Engine::new(input(v)).unwrap_or_else(|e| panic!("{e:#?}"))
}
fn run(v: Value) -> RunResult {
    let mut e = engine(v);
    e.run_to_end();
    e.snapshot()
}
fn op(v: &mut Value) -> &mut Value {
    &mut v["definitions"][0]["payload"]["data"]
}
fn worker() -> Value {
    json!({"role":"worker","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"person"},"qualifications":[]})
}
fn clock(v: &mut Value, name: &str, intervals: Value) {
    v["project"]["calendars"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["id"] == name)
        .unwrap()["intervals"] = intervals;
}
fn intervals(resource: &str, r: &RunResult) -> Vec<(u64, u64)> {
    let mut out: Vec<_> = r
        .resource_claims
        .iter()
        .filter(|c| c.resource_id.as_str() == resource)
        .flat_map(|c| {
            c.intervals
                .iter()
                .map(|i| (i.start_ms.0.value(), i.end_ms.0.value()))
        })
        .collect();
    out.sort();
    out
}
fn total(resource: &str, r: &RunResult) -> u64 {
    intervals(resource, r).iter().map(|(a, b)| b - a).sum()
}
// A formal DTO adapter of the original phase_calendar timings. One declared
// raw shell -> one finished part gives actual material ownership to the test;
// resource pools/calendars are explicit, with no simulator-private defaults.
fn phased() -> Value {
    let source = business();
    let source = source["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "phase_calendar")
        .unwrap();
    let mut v = base();
    let original = v.clone();
    let mut device = v["project"]["devices"][0].clone();
    device["id"] = json!("CNC");
    device["title"] = json!("CNC");
    device["role_bindings"]["resources"] = json!({"worker":"W-C","execution_capacity":"CNC-slot"});
    v["project"]["devices"] = json!([device]);
    let mut raw = v["project"]["locations"][0].clone();
    raw["capacity"] = q(2);
    let mut finished = v["project"]["locations"][2].clone();
    finished["id"] = json!("finished");
    finished["capacity"] = q(source["execution_design"]["output_capacity_piece"]
        .as_u64()
        .unwrap());
    v["project"]["locations"] = json!([raw, finished]);
    v["project"]["halls"][0]["input_locations"] = json!(["shells-A"]);
    v["project"]["halls"][0]["output_locations"] = json!(["finished"]);
    let mut w = v["project"]["resources"][0].clone();
    w["id"] = json!("W-C");
    w["calendar_id"] = json!("worker-calendar");
    let mut m = v["project"]["resources"][1].clone();
    m["id"] = json!("CNC-slot");
    m["calendar_id"] = json!("machine-calendar");
    v["project"]["resources"] = json!([w, m]);
    let calendar = |id: &str, rows: &Value| json!({"id":id,"title":id,"source":"user_provided","intervals":rows.as_array().unwrap().iter().map(|r|json!({"start_ms":r[0],"end_ms":r[1]})).collect::<Vec<_>>()});
    v["project"]["calendars"] = json!([
        calendar("worker-calendar", &source["worker_calendar_ms"]),
        calendar("machine-calendar", &source["machine_calendar_ms"])
    ]);
    let mut step = v["project"]["routes"][0]["steps"][0].clone();
    step["id"] = json!("step-cnc");
    step["device_id"] = json!("CNC");
    step["resource_bindings"] = json!({"worker":"W-C"});
    v["project"]["routes"][0]["steps"] = json!([step]);
    let mut row = v["project"]["boms"][0]["rows"][0].clone();
    row["consume_at_operation"] = json!("step-cnc");
    v["project"]["boms"][0]["rows"] = json!([row]);
    v["project"]["products"][0]["id"] = json!("PART");
    v["project"]["products"][0]["item_type_ref"]["id"] = json!("assembly");
    let mut inbound = original["project"]["connections"][0].clone();
    inbound["data"]["destination"]["data"]["instance_id"] = json!("CNC");
    let mut outbound = original["project"]["connections"][2].clone();
    outbound["data"]["source"]["data"]["instance_id"] = json!("CNC");
    outbound["data"]["destination"]["data"] = json!("finished");
    outbound["data"]["destination_store"] = json!("finished");
    v["project"]["connections"] = json!([inbound, outbound]);
    op(&mut v)["builtin"] = json!("transform");
    op(&mut v)["identity_policy"] = json!("transform_with_origin");
    op(&mut v)["inputs"] = json!([original["definitions"][0]["payload"]["data"]["inputs"][0]]);
    op(&mut v)["duration"] = Value::Null;
    op(&mut v)["resource_roles"] = json!([]);
    op(&mut v)["phases"]=Value::Array(source["phases"].as_array().unwrap().iter().map(|p|json!({"id":p["id"],"duration":duration(p["duration_ms"].as_u64().unwrap()),"phase_resources":if p["resources"].as_array().unwrap().iter().any(|r|r=="W-C"){json!([worker()])}else{json!([])},"interruption_policy":p["interruption_policy"],"work_kind":match p["id"].as_str().unwrap(){"load"=>"loading","unload"=>"unloading",_=>"processing"}})).collect());
    v["scenario"]["window"]["end_ms"] = source["time"]["observation_end_ms"].clone();
    v["scenario"]["orders"] = json!([]);
    v["scenario"]["production_tasks"][0]["order_allocations"] = json!([]);
    v["scenario"]["production_tasks"][0]["planned_quantity"] = q(1);
    v["scenario"]["scope"] = json!([{"kind":"device","id":"CNC"}]);
    v["scenario"]["initial_conditions"]["items"] =
        json!([original["scenario"]["initial_conditions"]["items"][0]]);
    v
}
fn set_single(v: &mut Value, work: u64, phase_worker: bool, policy: &str) {
    op(v)["duration"] = Value::Null;
    op(v)["resource_roles"] = json!([]);
    op(v)["phases"] = json!([{"id":"work","duration":duration(work),"phase_resources":if phase_worker{json!([worker()])}else{json!([])},"interruption_policy":policy,"work_kind":"processing"}]);
}
fn defer_raw(v: &mut Value, at: u64) {
    let raw = v["scenario"]["initial_conditions"]["items"][0].clone();
    v["scenario"]["initial_conditions"]["items"] = json!([]);
    v["scenario"]["arrivals"] = json!([{"id":"future-raw","at_ms":at,"destination":"shells-A","items":[raw],"release_policy":"unchecked_allowed"}]);
}
fn advance(e: &mut Engine, at: u64) {
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|t| t.0.value() < at)
    {
        assert!(!e.is_finished());
        e.step();
    }
}

#[test]
fn original_phase_calendar_finishes_at_45s_with_40s_effective_and_10s_person() {
    let r = run(phased());
    let job = &r.jobs[0];
    let expected = expected("phase_calendar");
    let effective: u64 = r
        .phase_work_intervals
        .iter()
        .map(|w| w.interval.end_ms.0.value() - w.interval.start_ms.0.value())
        .sum();
    let waits: u64 = r
        .waits
        .iter()
        .filter(|w| w.object_ref.id == job.id)
        .map(|w| w.interval.end_ms.0.value() - w.interval.start_ms.0.value())
        .sum();
    let actual = json!({"completion_ms":job.finished_at_ms.as_ref().unwrap().0.value(),"effective_ms":effective,"person_ms":total("W-C",&r),"wait_ms":waits});
    println!("P1_03_PHASE {}", actual);
    assert_eq!(actual["completion_ms"], expected["expected_completion_ms"]);
    assert_eq!(actual["effective_ms"], expected["expected_process_ms"]);
    assert_eq!(actual["person_ms"], expected["expected_worker_occupied_ms"]);
    assert_eq!(actual["wait_ms"], expected["expected_wait_ms"]);
    assert_eq!(intervals("W-C", &r), vec![(0, 5000), (40000, 45000)]);
    assert_eq!(intervals("CNC-slot", &r), vec![(0, 45000)]);
    let waits: Vec<_> = r
        .waits
        .iter()
        .filter(|w| w.object_ref.id == job.id)
        .collect();
    assert_eq!(waits.len(), 1);
    assert_eq!(waits[0].phase_ref.as_ref().unwrap().as_str(), "unload");
    assert!(matches!(waits[0].end_cause, WaitEndCause::Resolved));
    assert!(
        r.final_items
            .iter()
            .any(|i| i.item_type_ref.id.as_str() == "assembly"
                && matches!(&i.holder,HolderRef::Location(id)if id.as_str()=="finished"))
    );
}
#[test]
fn future_arrival_remains_external_until_real_receive_then_uses_same_inventory() {
    let mut v = phased();
    set_single(&mut v, 5000, false, "non_preemptive");
    defer_raw(&mut v, 10000);
    let mut e = engine(v);
    e.step();
    assert!(e.snapshot().final_items.is_empty());
    assert!(e.objects_at(t(10000)).is_none());
    assert!(e.item_history().is_empty());
    advance(&mut e, 10000);
    let r = e.snapshot();
    assert_eq!(r.jobs[0].started_at_ms.as_ref().unwrap().0.value(), 10000);
    assert!(
        r.final_items
            .iter()
            .any(|i| i.id.as_str() == "shell-001" && matches!(i.holder, HolderRef::Job(_)))
    );
    e.run_to_end();
    let r = e.snapshot();
    assert_eq!(r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(), 15000);
    assert_eq!(
        r.events
            .iter()
            .filter(|e| matches!(e.payload, SimulationEventPayload::ArrivalAccepted(_)))
            .count(),
        1
    );
}
#[test]
fn original_coarse_rule_cannot_stitch_worker_shifts_and_legacy_keeps_whole_job_person() {
    let mut v = phased();
    let source = business();
    let coarse = &source["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "refinement")
        .unwrap()["coarse"];
    op(&mut v)["phases"] = json!([]);
    op(&mut v)["duration"] = duration(coarse["single_phase_ms"].as_u64().unwrap());
    op(&mut v)["resource_roles"] = json!([worker()]);
    let r = run(v.clone());
    assert!(expected("coarse_full_worker")["expected_completion_ms"].is_null());
    assert!(r.jobs[0].started_at_ms.is_none());
    assert!(r.resource_claims.is_empty());
    assert!(
        r.final_items
            .iter()
            .all(|i| matches!(i.holder, HolderRef::Location(_)))
    );
    clock(
        &mut v,
        "worker-calendar",
        json!([{"start_ms":0,"end_ms":100000}]),
    );
    let r = run(v);
    assert_eq!(r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(), 40000);
    assert_eq!(total("W-C", &r), 40000);
    assert_eq!(total("CNC-slot", &r), 40000);
}
#[test]
fn output_capacity_holds_one_candidate_and_machine_after_phase_person_releases() {
    let mut v = phased();
    v["project"]["locations"][1]["capacity"] = q(0);
    let r = run(v);
    let job = &r.jobs[0];
    assert!(job.finished_at_ms.is_none());
    assert!(job.phase_ref.is_none());
    assert_eq!(job.pending_outputs.len(), 1);
    assert!(matches!(job.state, JobStatus::WaitingOutput));
    assert_eq!(total("W-C", &r), 10000);
    assert_eq!(intervals("CNC-slot", &r), vec![(0, 100000)]);
    assert_eq!(
        r.final_items
            .iter()
            .filter(|i| i.item_type_ref.id.as_str() == "assembly")
            .count(),
        1
    );
    assert!(
        r.final_items.iter().any(|i| i.id == job.pending_outputs[0]
            && matches!(&i.holder,HolderRef::Job(id)if *id==job.id))
    );
    assert!(r.waits.iter().any(|w| {
        w.interval.start_ms.0.value() == 45000
            && w.interval.end_ms.0.value() == 100000
            && w.phase_ref.is_none()
            && w.reasons
                .iter()
                .any(|r| matches!(r.kind, WaitReasonKind::OutputCapacity))
    }));
}
#[test]
fn explicit_last_phase_output_hold_is_a_separate_real_claim_not_more_work() {
    let mut v = phased();
    op(&mut v)["hold_on_output_wait"] = json!(["worker"]);
    v["project"]["locations"][1]["capacity"] = q(0);
    let r = run(v);
    let holds: Vec<_> = r
        .resource_claims
        .iter()
        .filter(|c| matches!(c.kind, ResourceClaimKind::OutputHold))
        .collect();
    assert_eq!(holds.len(), 1);
    assert!(holds[0].phase_ref.is_none());
    assert_eq!(holds[0].intervals[0].start_ms.0.value(), 45000);
    assert_eq!(holds[0].intervals[0].end_ms.0.value(), 100000);
    assert!(matches!(
        holds[0].interval_end_causes[0],
        ClaimIntervalEndCause::ObservationEnd
    ));
    assert_eq!(
        r.phase_work_intervals
            .iter()
            .map(|w| w.interval.end_ms.0.value() - w.interval.start_ms.0.value())
            .sum::<u64>(),
        40000
    );
}
#[test]
fn missing_phase_policy_duration_or_output_hold_source_is_located_without_guessing() {
    let mut missing = phased();
    op(&mut missing)["phases"][1]
        .as_object_mut()
        .unwrap()
        .remove("interruption_policy");
    let err =
        serde_json::from_value::<ModuleDefinition>(missing["definitions"][0].clone()).unwrap_err();
    assert!(err.to_string().contains("interruption_policy"));
    let mut missing = phased();
    op(&mut missing)["phases"][1]["duration"] = Value::Null;
    assert!(
        check_inputs(&input(missing))
            .unwrap_err()
            .iter()
            .any(|i| i.code == "simulation.duration_missing")
    );
    let mut missing = phased();
    op(&mut missing)["hold_on_output_wait"] = json!(["unknown-role"]);
    assert!(
        check_inputs(&input(missing))
            .unwrap_err()
            .iter()
            .any(|i| i.code == "simulation.output_hold_source")
    );
    let mut conflict = phased();
    op(&mut conflict)["hold_on_output_wait"] = json!(["worker"]);
    op(&mut conflict)["resource_roles"] = json!([worker()]);
    assert!(
        check_inputs(&input(conflict))
            .unwrap_err()
            .iter()
            .any(|i| i.code == "simulation.output_hold_conflict")
    );
}
#[test]
fn frozen_b09_multiple_wait_reasons_change_once_without_double_counting() {
    let b = boundary("B09");
    let mut v = phased();
    set_single(&mut v, 2000, true, "non_preemptive");
    defer_raw(&mut v, b["input"]["pcb_arrival_ms"].as_u64().unwrap());
    clock(
        &mut v,
        "worker-calendar",
        json!([{"start_ms":20000,"end_ms":40000}]),
    );
    let r = run(v);
    assert_eq!(
        r.jobs[0].started_at_ms.as_ref().unwrap().0.value(),
        b["expected"]["started_at_ms"].as_u64().unwrap()
    );
    let waits: Vec<_> = r
        .waits
        .iter()
        .filter(|w| w.object_ref.id == r.jobs[0].id)
        .collect();
    assert_eq!(waits.len(), 2);
    assert_eq!(waits[0].reasons.len(), 2);
    assert_eq!(waits[1].reasons.len(), 1);
    assert!(matches!(waits[0].end_cause, WaitEndCause::ConditionChanged));
    assert!(matches!(waits[1].end_cause, WaitEndCause::Resolved));
    assert_eq!(
        waits
            .iter()
            .map(|w| w.interval.end_ms.0.value() - w.interval.start_ms.0.value())
            .sum::<u64>(),
        20000
    );
}
#[test]
fn frozen_b10_pause_resume_records_only_effective_segments_and_keeps_machine() {
    let b = boundary("B10");
    let mut v = phased();
    set_single(
        &mut v,
        b["input"]["duration_ms"].as_u64().unwrap(),
        true,
        "pause_resume",
    );
    v["scenario"]["production_tasks"][0]["release_at_ms"] = b["input"]["start_ms"].clone();
    clock(
        &mut v,
        "worker-calendar",
        json!([{"start_ms":0,"end_ms":10000},{"start_ms":20000,"end_ms":30000}]),
    );
    let r = run(v);
    assert_eq!(
        r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(),
        b["expected"]["completion_ms"].as_u64().unwrap()
    );
    assert_eq!(intervals("W-C", &r), vec![(5000, 10000), (20000, 23000)]);
    assert_eq!(intervals("CNC-slot", &r), vec![(5000, 23000)]);
    let work = &r.phase_work_intervals;
    assert_eq!(work.len(), 2);
    assert_eq!(work[0].effective_progress_end_ms.0.value(), 5000);
    assert_eq!(work[1].effective_progress_start_ms.0.value(), 5000);
    assert_eq!(work[1].effective_progress_end_ms.0.value(), 8000);
    assert!(matches!(work[0].end_cause, PhaseWorkEndCause::Interrupted));
}
#[test]
fn zero_duration_resource_phase_cannot_steal_closed_worker_boundary() {
    let b = boundary("B11");
    for opens_again in [false, true] {
        let mut v = phased();
        set_single(&mut v, 0, true, "non_preemptive");
        v["scenario"]["production_tasks"][0]["release_at_ms"] = b["input"]["at_ms"].clone();
        if !opens_again {
            clock(
                &mut v,
                "worker-calendar",
                json!([{"start_ms":0,"end_ms":20000}]),
            )
        };
        let r = run(v);
        assert_eq!(
            r.jobs[0].started_at_ms.as_ref().map(|t| t.0.value()),
            opens_again.then_some(40000)
        );
        assert!(r.phase_work_intervals.is_empty());
        assert!(r.resource_claims.is_empty());
    }
}
#[test]
fn frozen_b12_prefix_uses_computed_boundary_and_never_claims_future_completion() {
    let b = boundary("B12");
    let mut v = phased();
    set_single(&mut v, 30000, false, "non_preemptive");
    v["scenario"]["production_tasks"][0]["release_at_ms"] = b["input"]["start_ms"].clone();
    let mut e = engine(v);
    advance(&mut e, 20000);
    let r = e.snapshot();
    let work = &r.phase_work_intervals[0];
    assert_eq!(work.interval.start_ms.0.value(), 5000);
    assert_eq!(work.interval.end_ms.0.value(), 20000);
    assert_eq!(work.effective_progress_end_ms.0.value(), 15000);
    assert!(matches!(
        work.end_cause,
        PhaseWorkEndCause::ComputedBoundary
    ));
    assert!(r.jobs[0].finished_at_ms.is_none());
    assert!(e.objects_at(t(35000)).is_none());
    assert!(
        !r.events
            .iter()
            .any(|e| matches!(e.payload, SimulationEventPayload::PhaseCompleted(_)))
    );
    advance(&mut e, 35000);
    assert_eq!(
        e.snapshot().jobs[0]
            .finished_at_ms
            .as_ref()
            .unwrap()
            .0
            .value(),
        35000
    );
}

fn goal(v: &Value, product: &str, destination: &str, target: Option<u64>) -> FactoryGenerationGoal {
    let template: Value = serde_json::from_str(include_str!(
        "inputs/check_factory_draft.request.json"
    ))
    .unwrap();
    let mut g = template["params"]["generation_goal"].clone();
    g["industry"] = json!("explicit finite production target");
    g["product_family"] = json!("declared product");
    g["constraints"] = json!([]);
    g["acceptance_scope"]["product_ref"] = json!({"id":product,"revision":"r1"});
    g["acceptance_scope"]["destination"] = json!(destination);
    g["observation_window"] = v["scenario"]["window"].clone();
    g["accepted_quantity_per_day"] = target.map(q).unwrap_or(Value::Null);
    serde_json::from_value(g).unwrap()
}
fn no_order() -> Value {
    let mut v = base();
    v["scenario"]["orders"] = json!([]);
    v["scenario"]["production_tasks"][0]["order_allocations"] = json!([]);
    let mut pcb = v["scenario"]["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("pcb-002");
    v["scenario"]["arrivals"] = json!([{"id":"later-pcb","at_ms":90000,"destination":"pcbs-A","items":[pcb],"release_policy":"unchecked_allowed"}]);
    v
}
#[test]
fn h03_explicit_goal_has_nonempty_cumulative_acceptance_without_orders_or_generation_context() {
    let v = no_order();
    assert!(v["project"]["generation_context"].is_null());
    let g = goal(&v, "PACKAGE", "packages-B", Some(1));
    let candidate = input(v);
    let before = serde_json::to_value(&candidate.project).unwrap();
    check_inputs_with_goal(&candidate, &g).unwrap();
    let e = execute_engine_with_goal(candidate.clone(), g, &AtomicBool::new(false)).unwrap();
    let r = e.snapshot();
    assert_eq!(r.acceptance_events.len(), 2);
    assert!(r.acceptance_events.iter().all(|a| a.order_id.is_none()));
    assert_eq!(
        r.acceptance_events
            .iter()
            .map(|a| a.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 165000]
    );
    assert_eq!(
        r.acceptance_events
            .iter()
            .map(|a| a.quantity.value.coefficient.value())
            .sum::<u64>(),
        2
    );
    assert!(r.orders.is_empty());
    assert_eq!(r.goal_met, Some(true));
    assert_eq!(serde_json::to_value(candidate.project).unwrap(), before);
    assert!(before["generation_context"].is_null());
    println!(
        "P1_03_H03 {}",
        json!({"acceptance_ms":[75000,165000],"accepted":2,"target":1,"order_ids":[null,null],"generation_context_created":false,"production_capped_by_goal":false})
    );
}
#[test]
fn h03_goal_resolves_product_version_unit_destination_and_real_stage() {
    let v = no_order();
    let g = goal(&v, "PACKAGE", "packages-B", Some(3));
    let mut e = Engine::new_with_goal(input(v.clone()), g.clone()).unwrap();
    e.run_to_end();
    assert_eq!(e.snapshot().goal_met, Some(false));
    assert_eq!(e.snapshot().acceptance_events.len(), 2);
    for variant in 0..4 {
        let mut invalid = g.clone();
        match variant {
            0 => {
                invalid.acceptance_scope.product_ref.revision =
                    SubjectRevisionId::try_from("r2".to_owned()).unwrap()
            }
            1 => invalid.acceptance_scope.unit = UnitId::try_from("box".to_owned()).unwrap(),
            2 => {
                invalid.acceptance_scope.destination =
                    ObjectId::try_from("missing".to_owned()).unwrap()
            }
            _ => invalid.acceptance_scope.stage = AcceptanceStage::UnloadedAndReleased,
        };
        assert!(check_inputs_with_goal(&input(v.clone()), &invalid).is_err());
    }
    let g = goal(&v, "PACKAGE", "pcbs-A", Some(1));
    let mut e = Engine::new_with_goal(input(v), g).unwrap();
    e.run_to_end();
    assert!(e.snapshot().acceptance_events.is_empty());
    assert_eq!(e.snapshot().goal_met, Some(false));
}
#[test]
fn h03_missing_scope_is_explicit_and_does_not_infer_acceptance_from_packaging() {
    let r = run(no_order());
    assert!(
        r.final_items
            .iter()
            .any(|i| i.item_type_ref.id.as_str() == "package")
    );
    assert!(r.acceptance_events.is_empty());
    assert!(r.orders.is_empty());
    assert!(r.goal_met.is_none());
    assert!(
        r.limitations
            .iter()
            .any(|i| i.code == "simulation.goal_acceptance_missing")
    );
}
#[test]
fn h03_container_contents_keep_their_product_unit_and_accept_once() {
    let mut v = no_order();
    let mut product = v["project"]["products"][0].clone();
    product["id"] = json!("ASSEMBLY");
    product["item_type_ref"]["id"] = json!("assembly");
    v["project"]["products"]
        .as_array_mut()
        .unwrap()
        .push(product);
    let g = goal(&v, "ASSEMBLY", "packages-B", Some(2));
    let mut e = Engine::new_with_goal(input(v), g).unwrap();
    e.run_to_end();
    let r = e.snapshot();
    assert_eq!(r.acceptance_events.len(), 2);
    for a in &r.acceptance_events {
        assert_eq!(a.quantity.unit.as_str(), "piece");
        let item = r
            .final_items
            .iter()
            .find(|i| i.id == a.item_refs[0])
            .unwrap();
        assert_eq!(item.item_type_ref.id.as_str(), "assembly");
        assert!(matches!(item.holder, HolderRef::Container(_)));
    }
    assert_eq!(r.goal_met, Some(true));
    let before = serde_json::to_value(r).unwrap();
    assert_eq!(e.step(), 0);
    assert_eq!(serde_json::to_value(e.snapshot()).unwrap(), before);
}
#[test]
fn h03_prefix_acceptance_is_real_but_goal_completion_waits_for_full_observation() {
    let v = no_order();
    let g = goal(&v, "PACKAGE", "packages-B", Some(1));
    let mut e = Engine::new_with_goal(input(v), g).unwrap();
    advance(&mut e, 75000);
    let r = e.snapshot();
    assert_eq!(r.acceptance_events.len(), 1);
    assert!(!r.whole_window_computed);
    assert!(r.goal_met.is_none());
    e.pause();
    assert_eq!(e.step(), 0);
    assert_eq!(e.snapshot().acceptance_events.len(), 1);
    e.resume();
    e.run_to_end();
    assert_eq!(e.snapshot().acceptance_events.len(), 2);
}

#[test]
fn inspection_required_arrival_does_not_become_ordinary_unchecked_material() {
    let mut v = phased();
    set_single(&mut v, 2000, false, "non_preemptive");
    op(&mut v)["inputs"][0]["required_quality"] = json!(["unchecked"]);
    v["scenario"]["initial_conditions"]["items"][0]["quality"] = json!("unchecked");
    defer_raw(&mut v, 10000);
    v["scenario"]["arrivals"][0]["release_policy"] = json!("inspection_required");
    let r = run(v.clone());
    assert!(r.jobs[0].started_at_ms.is_none());
    assert!(matches!(r.final_items[0].quality, QualityState::Pending));
    assert!(
        r.events
            .iter()
            .all(|e| !matches!(e.payload, SimulationEventPayload::QualityChanged(_)))
    );
    v["scenario"]["arrivals"][0]["release_policy"] = json!("unchecked_allowed");
    let r = run(v);
    assert_eq!(r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(), 12000);
}
#[test]
fn job_and_phase_alias_of_same_machine_pool_never_double_counts_capacity() {
    let mut v = phased();
    v["project"]["devices"][0]["role_bindings"]["resources"]["same-machine"] = json!("CNC-slot");
    op(&mut v)["phases"][0]["phase_resources"].as_array_mut().unwrap().push(json!({"role":"same-machine","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"slot"},"qualifications":[]}));
    let r = run(v);
    let claims: Vec<_> = r
        .resource_claims
        .iter()
        .filter(|c| c.resource_id.as_str() == "CNC-slot")
        .collect();
    assert_eq!(claims.len(), 1);
    assert!(matches!(claims[0].kind, ResourceClaimKind::Job));
    assert!(claims[0].phase_ref.is_none());
    assert_eq!(total("CNC-slot", &r), 45000);
}
fn auxiliary(v: &mut Value, work: u64, release: u64, exclusive: bool) {
    let mut device = v["project"]["devices"][0].clone();
    device["id"] = json!("BENCH");
    device["role_bindings"]["resources"]["execution_capacity"] = json!("BENCH-slot");
    device["role_bindings"]["targets"] = json!({"target":{"kind":"device","id":"BENCH"}});
    v["project"]["devices"].as_array_mut().unwrap().push(device);
    let mut resource = v["project"]["resources"][1].clone();
    resource["id"] = json!("BENCH-slot");
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(resource);
    let mut module = v["definitions"][0].clone();
    module["id"] = json!("fixture.auxiliary");
    module["payload"] = json!({"category":"tasks","data":{"kind":"manual_preparation","target_role":"target","prerequisites":[],"steps":[],"resource_roles":if work==0{json!([])}else{json!([worker()])},"duration":duration(work),"equipment_access":if exclusive{"exclusive"}else{"shared"},"completion_effect":{"target_role":"target","marker":"auxiliary-ready","produces_material":false}}});
    v["definitions"].as_array_mut().unwrap().push(module);
    v["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"fixture.auxiliary","version":"0.1.0"}));
    let mut route = v["project"]["routes"][0].clone();
    route["id"] = json!("ROUTE-BENCH");
    route["steps"] = json!([{"id":"step-bench","operation_ref":{"id":"fixture.auxiliary","version":"0.1.0"},"device_id":"BENCH","predecessors":[],"item_bindings":{},"resource_bindings":{"worker":"W-C"}}]);
    v["project"]["routes"].as_array_mut().unwrap().push(route);
    let mut task = v["scenario"]["production_tasks"][0].clone();
    task["id"] = json!("BENCH-TASK");
    task["routing_ref"]["id"] = json!("ROUTE-BENCH");
    task["release_at_ms"] = json!(release);
    v["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .push(task);
    v["scenario"]["scope"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"device","id":"BENCH"}));
}
#[test]
fn unload_waits_for_a_real_other_job_holding_worker_and_keeps_material_and_machine() {
    let mut v = phased();
    clock(
        &mut v,
        "worker-calendar",
        json!([{"start_ms":0,"end_ms":100000}]),
    );
    auxiliary(&mut v, 45000, 5000, true);
    let mut e = engine(v);
    advance(&mut e, 35000);
    let r = e.snapshot();
    let cnc=r.jobs.iter().find(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()=="CNC")).unwrap();
    assert!(matches!(cnc.state, JobStatus::WaitingResource));
    assert_eq!(cnc.phase_ref.as_ref().unwrap().as_str(), "unload");
    assert!(cnc.finished_at_ms.is_none());
    assert!(
        r.final_items
            .iter()
            .any(|i| matches!(&i.holder,HolderRef::Job(id)if *id==cnc.id)
                && matches!(i.lifecycle, Lifecycle::InProcess))
    );
    e.run_to_end();
    let r = e.snapshot();
    let cnc=r.jobs.iter().find(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()=="CNC")).unwrap();
    assert_eq!(cnc.finished_at_ms.as_ref().unwrap().0.value(), 55000);
    assert_eq!(intervals("CNC-slot", &r), vec![(0, 55000)]);
    assert_eq!(
        r.resource_claims
            .iter()
            .filter(|c| c.job_id == cnc.id && c.resource_id.as_str() == "W-C")
            .flat_map(|c| &c.intervals)
            .map(|i| i.end_ms.0.value() - i.start_ms.0.value())
            .sum::<u64>(),
        10000
    );
    assert!(r.waits.iter().any(|w| {
        w.object_ref.id == cnc.id
            && w.interval.start_ms.0.value() == 35000
            && w.interval.end_ms.0.value() == 50000
            && w.reasons
                .iter()
                .any(|r| matches!(r.kind, WaitReasonKind::ResourceBusy))
    }));
}
#[test]
fn resource_free_zero_task_closes_at_20s_even_while_machine_calendar_is_closed() {
    let mut v = phased();
    auxiliary(&mut v, 0, 20000, false);
    v["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .retain(|t| t["id"] == "BENCH-TASK");
    clock(
        &mut v,
        "machine-calendar",
        json!([{"start_ms":0,"end_ms":20000}]),
    );
    let r = run(v);
    assert_eq!(r.jobs.len(), 1);
    assert_eq!(r.jobs[0].started_at_ms.as_ref().unwrap().0.value(), 20000);
    assert_eq!(r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(), 20000);
    assert!(r.phase_work_intervals.is_empty());
    assert!(r.resource_claims.is_empty());
}
#[test]
fn b17_arrival_zero_work_transfer_and_goal_acceptance_close_at_same_instant() {
    let mut v = phased();
    set_single(&mut v, 0, false, "non_preemptive");
    v["scenario"]["production_tasks"][0]["release_at_ms"] = json!(10000);
    defer_raw(&mut v, 10000);
    let g = goal(&v, "PART", "finished", Some(1));
    let mut e = Engine::new_with_goal(input(v), g).unwrap();
    e.step();
    e.step();
    let r = e.snapshot();
    assert_eq!(r.computed_until_ms.unwrap().0.value(), 10000);
    assert_eq!(r.acceptance_events.len(), 1);
    assert_eq!(r.acceptance_events[0].at_ms.0.value(), 10000);
    assert_eq!(r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(), 10000);
    assert!(r.jobs[0].pending_outputs.is_empty());
    assert!(r.phase_work_intervals.is_empty());
    assert!(r.resource_claims.is_empty());
}

#[test]
fn saved_service_declaration_is_not_silently_executed_as_a_worker_pool() {
    let mut v = phased();
    v["project"]["resources"][0]["kind"] = json!("service_declaration");
    assert!(
        check_inputs(&input(v))
            .unwrap_err()
            .iter()
            .any(|i| i.code == "simulation.service_execution_deferred")
    );
    let mut v = phased();
    let mut declaration = v["project"]["resources"][0].clone();
    declaration["id"] = json!("declared-service");
    declaration["kind"] = json!("service_declaration");
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .push(declaration);
    let r = run(v);
    assert_eq!(r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(), 45000);
    assert!(
        !r.resource_claims
            .iter()
            .any(|c| c.resource_id.as_str() == "declared-service")
    );
}
#[test]
fn goal_unknown_quantity_and_exact_decimal_target_do_not_invent_orders_or_round() {
    let v = no_order();
    let mut g = goal(&v, "PACKAGE", "packages-B", None);
    let mut e = Engine::new_with_goal(input(v.clone()), g.clone()).unwrap();
    e.run_to_end();
    assert_eq!(e.snapshot().acceptance_events.len(), 2);
    assert!(e.snapshot().goal_met.is_none());
    assert!(e.snapshot().orders.is_empty());
    g.accepted_quantity_per_day = Some(
        serde_json::from_value(
            json!({"value":{"coefficient":15,"decimal_places":1},"unit":"piece"}),
        )
        .unwrap(),
    );
    let mut e = Engine::new_with_goal(input(v), g).unwrap();
    e.run_to_end();
    assert_eq!(e.snapshot().goal_met, Some(true));
    assert_eq!(e.snapshot().acceptance_events.len(), 2);
}

#[test]
fn frozen_scenario_goal_uses_the_normal_admission_engine_and_worker_without_context() {
    let mut v = no_order();
    let goal = goal(&v, "PACKAGE", "packages-B", Some(1));
    v["scenario"]["acceptance_goal"] = serde_json::to_value(&goal).unwrap();
    let candidate = input(v);
    let frozen = serde_json::to_value(&candidate.scenario).unwrap();
    assert!(candidate.project.generation_context.is_none());
    check_inputs(&candidate).unwrap();
    let restored: ScenarioInput = serde_json::from_value(frozen.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(restored.acceptance_goal).unwrap(),
        serde_json::to_value(&goal).unwrap()
    );
    let worker = execute_engine(candidate.clone(), &AtomicBool::new(false)).unwrap();
    let r = worker.snapshot();
    assert_eq!(r.acceptance_events.len(), 2);
    assert!(r.acceptance_events.iter().all(|e| e.order_id.is_none()));
    assert_eq!(r.goal_met, Some(true));
    assert_eq!(
        r.acceptance_events
            .iter()
            .map(|a| a.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 165000]
    );
    assert!(candidate.project.generation_context.is_none());
    assert!(r.orders.is_empty());
    assert_eq!(serde_json::to_value(&candidate.scenario).unwrap(), frozen);
    let mut direct = Engine::new(candidate.clone()).unwrap();
    direct.run_to_end();
    assert_eq!(
        serde_json::to_value(direct.snapshot()).unwrap(),
        serde_json::to_value(&r).unwrap()
    );
    let explicit =
        execute_engine_with_goal(candidate.clone(), goal, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        serde_json::to_value(explicit.snapshot()).unwrap(),
        serde_json::to_value(&r).unwrap()
    );
    let mut higher = candidate;
    higher
        .scenario
        .acceptance_goal
        .as_mut()
        .unwrap()
        .accepted_quantity_per_day = Some(serde_json::from_value(q(3)).unwrap());
    let revision = Revision(SafeUint::try_from(2).unwrap());
    higher.project.revision = revision.clone();
    higher.scenario.project_revision = revision.clone();
    higher.scenario.revision = revision.clone();
    higher.input_ref.project_revision = revision.clone();
    higher.input_ref.scenario_revision = revision;
    higher.run_id = RunId::try_from("P1-03-saved-goal-r2".to_owned()).unwrap();
    let higher = execute_engine(higher, &AtomicBool::new(false))
        .unwrap()
        .snapshot();
    assert_eq!(higher.goal_met, Some(false));
    assert_eq!(higher.acceptance_events.len(), 2);
    assert_eq!(worker.snapshot().goal_met, Some(true));
    println!(
        "P1_03_SCENARIO_GOAL {}",
        json!({"source":"frozen ScenarioInput.acceptance_goal","accepted":2,"goal1":true,"goal3":false,"acceptance_ms":[75000,165000],"order_ids":[null,null],"generation_context_created":false})
    );
}

// Pre-existing saved context fixture for comparison/order-mode tests only.
// The manual-goal success case above and the product Engine never create one.
fn existing_context(goal: &FactoryGenerationGoal) -> Value {
    json!({"goal":goal,"template_ref":null,"parameter_sources":[],"draft_check":null,"initial_run_id":null,"revised_run_ids":[],"changes":[],"stop_reason":null})
}
fn reject_goal(
    candidate: EngineInput,
    explicit: Option<&FactoryGenerationGoal>,
    code: &str,
) -> Vec<Issue> {
    let project = serde_json::to_value(&candidate.project).unwrap();
    let scenario = serde_json::to_value(&candidate.scenario).unwrap();
    let errors = if let Some(goal) = explicit {
        check_inputs_with_goal(&candidate, goal)
    } else {
        check_inputs(&candidate)
    }
    .unwrap_err();
    assert!(errors.iter().any(|i| i.code == code), "{errors:?}");
    let initialized = if let Some(goal) = explicit {
        Engine::new_with_goal(candidate.clone(), goal.clone())
    } else {
        Engine::new(candidate.clone())
    }
    .err()
    .unwrap();
    let executed = if let Some(goal) = explicit {
        execute_engine_with_goal(candidate.clone(), goal.clone(), &AtomicBool::new(false))
    } else {
        execute_engine(candidate.clone(), &AtomicBool::new(false))
    }
    .err()
    .unwrap();
    assert_eq!(
        serde_json::to_value(&errors).unwrap(),
        serde_json::to_value(initialized).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&errors).unwrap(),
        serde_json::to_value(executed).unwrap()
    );
    assert_eq!(serde_json::to_value(candidate.project).unwrap(), project);
    assert_eq!(serde_json::to_value(candidate.scenario).unwrap(), scenario);
    errors
}
#[test]
fn scenario_goal_sources_must_agree_in_every_field_without_precedence_override() {
    for variant in 0..6 {
        let mut v = no_order();
        let first = goal(&v, "PACKAGE", "packages-B", Some(1));
        let mut different = first.clone();
        different.accepted_quantity_per_day = Some(serde_json::from_value(q(3)).unwrap());
        v["scenario"]["acceptance_goal"] = serde_json::to_value(&first).unwrap();
        let mut explicit = None;
        match variant {
            0 => explicit = Some(different),
            1 => v["project"]["generation_context"] = existing_context(&different),
            2 => {
                v["project"]["generation_context"] = existing_context(&first);
                explicit = Some(different);
            }
            3 => {
                different = first.clone();
                different.industry = "different source metadata".into();
                v["project"]["generation_context"] = existing_context(&different);
            }
            4 => {
                different = first.clone();
                different.site_envelope.height_m = Some(FiniteNumber::try_from(5.0).unwrap());
                explicit = Some(different);
            }
            _ => {
                v["scenario"]["acceptance_goal"] = Value::Null;
                v["project"]["generation_context"] = existing_context(&first);
                explicit = Some(different);
            }
        }
        reject_goal(
            input(v),
            explicit.as_ref(),
            "simulation.goal_source_conflict",
        );
    }
}
#[test]
fn scenario_goal_agreeing_three_sources_use_one_acceptance_stream_and_stay_frozen() {
    let mut v = no_order();
    let goal = goal(&v, "PACKAGE", "packages-B", Some(1));
    v["scenario"]["acceptance_goal"] = serde_json::to_value(&goal).unwrap();
    v["project"]["generation_context"] = existing_context(&goal);
    let candidate = input(v);
    let frozen = serde_json::to_value(&candidate.project).unwrap();
    check_inputs(&candidate).unwrap();
    check_inputs_with_goal(&candidate, &goal).unwrap();
    let standard = execute_engine(candidate.clone(), &AtomicBool::new(false))
        .unwrap()
        .snapshot();
    let explicit = execute_engine_with_goal(candidate.clone(), goal, &AtomicBool::new(false))
        .unwrap()
        .snapshot();
    assert_eq!(standard.acceptance_events.len(), 2);
    assert_eq!(
        serde_json::to_value(standard).unwrap(),
        serde_json::to_value(explicit).unwrap()
    );
    assert_eq!(serde_json::to_value(candidate.project).unwrap(), frozen);
}
#[test]
fn scenario_goal_reference_unit_stage_and_window_fail_consistently_before_execution() {
    for variant in 0..7 {
        let mut v = no_order();
        let mut invalid = serde_json::to_value(goal(&v, "PACKAGE", "packages-B", Some(1))).unwrap();
        let code = match variant {
            0 => {
                invalid["acceptance_scope"]["product_ref"]["revision"] = json!("r2");
                "simulation.goal_product_reference"
            }
            1 => {
                invalid["acceptance_scope"]["unit"] = json!("box");
                "simulation.goal_unit"
            }
            2 => {
                invalid["acceptance_scope"]["destination"] = json!("missing");
                "simulation.goal_destination"
            }
            3 | 4 => {
                invalid["acceptance_scope"]["stage"] = json!(if variant == 3 {
                    "received"
                } else {
                    "unloaded_and_released"
                });
                "simulation.goal_acceptance_stage_scope"
            }
            5 => {
                invalid["observation_window"]["end_ms"] = json!(240001);
                "simulation.goal_window"
            }
            _ => {
                invalid["accepted_quantity_per_day"]["unit"] = json!("box");
                "simulation.goal_quantity"
            }
        };
        v["scenario"]["acceptance_goal"] = invalid;
        let errors = reject_goal(input(v), None, code);
        assert!(errors.iter().filter(|e| e.code == code).all(|e| {
            e.field_path
                .as_ref()
                .unwrap()
                .starts_with("/acceptance_goal/")
                && e.object_ref.as_ref().is_some_and(|o| {
                    matches!(o.kind, ObjectKind::Scenario) && o.id.as_str() == "LOCAL"
                })
        }));
    }
}
#[test]
fn scenario_goal_order_exclusion_and_absent_null_legacy_modes_keep_existing_rules() {
    let mut v = base();
    let goal = goal(&v, "PACKAGE", "packages-B", Some(1));
    v["scenario"]["acceptance_goal"] = serde_json::to_value(&goal).unwrap();
    reject_goal(input(v), None, "simulation.goal_order_mode_conflict");
    let mut v = base();
    v["project"]["generation_context"] = existing_context(&goal);
    let r = run(v);
    assert_eq!(r.orders.len(), 1);
    assert_eq!(r.acceptance_events.len(), 1);
    assert!(r.acceptance_events[0].order_id.is_some());
    assert_eq!(r.acceptance_events[0].at_ms.0.value(), 75000);
    assert_eq!(r.goal_met, Some(false));
    for null in [false, true] {
        let mut v = no_order();
        if null {
            v["scenario"]["acceptance_goal"] = Value::Null;
        } else {
            v["scenario"]
                .as_object_mut()
                .unwrap()
                .remove("acceptance_goal");
        }
        let candidate = input(v);
        assert!(candidate.scenario.acceptance_goal.is_none());
        check_inputs(&candidate).unwrap();
        let r = execute_engine(candidate.clone(), &AtomicBool::new(false))
            .unwrap()
            .snapshot();
        assert!(r.acceptance_events.is_empty());
        assert!(r.orders.is_empty());
        assert!(r.goal_met.is_none());
        assert!(candidate.project.generation_context.is_none());
        assert!(
            r.limitations
                .iter()
                .any(|e| e.code == "simulation.goal_acceptance_missing"
                    && e.field_path.as_deref() == Some("/acceptance_goal"))
        );
    }
}
