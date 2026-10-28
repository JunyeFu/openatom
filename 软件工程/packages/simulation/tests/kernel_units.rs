use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{Engine, EngineInput, check_inputs, execute_engine};
use std::sync::atomic::AtomicBool;

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap()
}
fn input(v: Value) -> EngineInput {
    let scenario: ScenarioInput = serde_json::from_value(v["scenario"].clone()).unwrap();
    EngineInput {
        run_id: RunId::try_from("kernel-test-run".to_owned()).unwrap(),
        input_ref: serde_json::from_value(v["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        scope: scenario.scope.clone(),
        scenario,
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
    }
}
fn time(n: u64) -> SimTimeMs {
    SimTimeMs(SafeUint::try_from(n).unwrap())
}
fn engine(v: Value) -> Engine {
    Engine::new(input(v)).unwrap_or_else(|issues| panic!("{issues:#?}"))
}
fn run(v: Value) -> RunResult {
    let mut e = engine(v);
    e.run_to_end();
    e.snapshot()
}
fn one(v: &mut Value) {
    v["scenario"]["orders"][0]["quantity"]["value"]["coefficient"] = json!(1);
    v["scenario"]["production_tasks"][0]["planned_quantity"]["value"]["coefficient"] = json!(1);
    v["scenario"]["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(1);
}
fn add_pcb(v: &mut Value, at: u64) {
    let mut pcb = v["scenario"]["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("pcb-002");
    v["scenario"]["arrivals"] = json!([{"id":"later-pcb","at_ms":at,"destination":"pcbs-A","items":[pcb],"release_policy":"unchecked_allowed"}]);
}
fn operation<'a>(v: &'a mut Value, name: &str) -> &'a mut Value {
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == name)
        .map(|d| &mut d["payload"]["data"])
        .unwrap()
}

#[test]
fn original_local_a_waits_for_pcb_and_forms_one_assembly_and_one_box_once() {
    let r = run(fixture());
    assert!(r.whole_window_computed);
    assert_eq!(r.goal_met, Some(false));
    assert_eq!(r.orders[0].accepted.value.coefficient.value(), 1);
    assert_eq!(r.acceptance_events.len(), 1);
    assert_eq!(r.acceptance_events[0].at_ms.0.value(), 75000);
    let assemblies: Vec<_> = r
        .final_items
        .iter()
        .filter(|i| i.item_type_ref.id.as_str() == "assembly")
        .collect();
    assert_eq!(assemblies.len(), 1);
    let boxes: Vec<_> = r
        .final_items
        .iter()
        .filter(|i| i.item_type_ref.id.as_str() == "package")
        .collect();
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes[0].contents, vec![assemblies[0].id.clone()]);
    assert!(matches!(&assemblies[0].holder,HolderRef::Container(id) if *id==boxes[0].id));
    assert!(r.final_items.iter().any(|i| i.id.as_str() == "shell-002"
        && matches!(i.lifecycle, Lifecycle::Available)
        && matches!(i.holder, HolderRef::Location(_))));
    assert!(r.waits.iter().any(|w| {
        w.reasons
            .iter()
            .any(|reason| matches!(reason.kind, WaitReasonKind::MissingMaterial))
    }));
}
#[test]
fn later_pcb_uses_the_same_material_graph_and_actual_history() {
    let mut v = fixture();
    add_pcb(&mut v, 90000);
    let mut e = engine(v);
    assert!(e.objects_at(time(0)).is_none());
    e.run_to_end();
    let r = e.snapshot();
    assert_eq!(r.goal_met, Some(true));
    assert_eq!(
        r.acceptance_events
            .iter()
            .map(|a| a.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 165000]
    );
    assert_eq!(
        r.orders[0].completion_ms.as_ref().unwrap().0.value(),
        165000
    );
    assert!(
        !e.objects_at(time(59999))
            .unwrap()
            .iter()
            .any(|i| i.item_type_ref.id.as_str() == "assembly")
    );
    assert_eq!(
        e.objects_at(time(75000))
            .unwrap()
            .iter()
            .filter(|i| i.item_type_ref.id.as_str() == "package")
            .count(),
        1
    );
    assert!(
        e.objects_at(time(90000))
            .unwrap()
            .iter()
            .any(|i| i.id.as_str() == "pcb-002" && matches!(i.holder, HolderRef::Job(_)))
    );
    assert!(e.objects_at(time(240001)).is_none());
    assert_eq!(
        serde_json::to_value(e.snapshot()).unwrap(),
        serde_json::to_value(r).unwrap()
    );
}
#[test]
fn shared_worker_claims_do_not_overlap_or_double_count_capacity() {
    let mut v = fixture();
    add_pcb(&mut v, 0);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "assemblies-A")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(2);
    v["project"]["routes"][0]["steps"][1]["resource_bindings"]["worker"] = json!("worker-assembly");
    let r = run(v);
    let mut intervals: Vec<_> = r
        .resource_claims
        .iter()
        .filter(|c| c.resource_id.as_str() == "worker-assembly")
        .flat_map(|c| {
            c.intervals
                .iter()
                .map(|i| (i.start_ms.0.value(), i.end_ms.0.value()))
        })
        .collect();
    intervals.sort();
    assert_eq!(intervals.len(), 4);
    assert!(intervals.windows(2).all(|p| p[0].1 <= p[1].0));
    assert_eq!(r.orders[0].accepted.value.coefficient.value(), 2);
}
#[test]
fn full_output_retains_one_candidate_and_machine_claim_until_observation_end() {
    let mut v = fixture();
    one(&mut v);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "assemblies-A")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(0);
    let r = run(v);
    let job=r.jobs.iter().find(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()=="ASM-01")).unwrap();
    assert!(matches!(job.state, JobStatus::WaitingOutput));
    assert!(job.finished_at_ms.is_none());
    assert_eq!(job.pending_outputs.len(), 1);
    let item = r
        .final_items
        .iter()
        .find(|i| i.id == job.pending_outputs[0])
        .unwrap();
    assert!(matches!(&item.holder,HolderRef::Job(id) if *id==job.id));
    assert!(
        r.resource_claims
            .iter()
            .filter(|c| c.job_id == job.id)
            .all(|c| c.intervals[0].end_ms.0.value() == 240000
                && matches!(
                    c.interval_end_causes[0],
                    ClaimIntervalEndCause::ObservationEnd
                ))
    );
    assert_eq!(r.orders[0].accepted.value.coefficient.value(), 0);
}
#[test]
fn initial_partial_hard_reservation_is_split_once_and_failed_acquisition_is_atomic() {
    let mut v = fixture();
    one(&mut v);
    v["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "shell")
        .unwrap()["tracking"] = json!("batch");
    v["scenario"]["initial_conditions"]["items"][0]["id"] = json!("batch10");
    v["scenario"]["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"] = json!(10);
    v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "shells-A")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(10);
    operation(&mut v, "fixture.assemble")["inputs"][0]["quantity"]["value"]["coefficient"] =
        json!(4);
    v["project"]["boms"][0]["rows"][0]["quantity"]["value"]["coefficient"] = json!(4);
    v["scenario"]["initial_conditions"]["material_allocations"] = json!([{"id":"hard4","demand_ref":"TASK-LOCAL","item_refs":["batch10"],"quantity":{"value":{"coefficient":4,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"}]);
    v["project"]["calendars"][0]["intervals"][0]["start_ms"] = json!(5000);
    let before = v.clone();
    let mut e = engine(v);
    e.step();
    let r = e.snapshot();
    assert!(r.resource_claims.is_empty());
    assert!(r.jobs.iter().all(|j| j.started_at_ms.is_none()));
    let live: Vec<_> = r
        .final_items
        .iter()
        .filter(|i| {
            i.item_type_ref.id.as_str() == "shell" && !matches!(i.lifecycle, Lifecycle::Transformed)
        })
        .collect();
    assert_eq!(live.len(), 2);
    assert_eq!(
        live.iter()
            .map(|i| i.quantity.value.coefficient.value())
            .sum::<u64>(),
        10
    );
    assert_eq!(
        r.allocations
            .iter()
            .find(|a| a.id.as_str() == "hard4")
            .unwrap()
            .quantity
            .value
            .coefficient
            .value(),
        4
    );
    let frozen = serde_json::to_value(r).unwrap();
    assert_eq!(serde_json::to_value(e.snapshot()).unwrap(), frozen);
    e.step();
    let started = e.snapshot();
    assert_eq!(started.computed_until_ms.as_ref().unwrap().0.value(), 5000);
    let at_location: u64 = started
        .final_items
        .iter()
        .filter(|i| {
            i.item_type_ref.id.as_str() == "shell"
                && matches!(i.lifecycle, Lifecycle::Available)
                && matches!(i.holder, HolderRef::Location(_))
        })
        .map(|i| i.quantity.value.coefficient.value())
        .sum();
    let in_process: u64 = started
        .final_items
        .iter()
        .filter(|i| {
            i.item_type_ref.id.as_str() == "shell"
                && matches!(i.lifecycle, Lifecycle::InProcess)
                && matches!(i.holder, HolderRef::Job(_))
        })
        .map(|i| i.quantity.value.coefficient.value())
        .sum();
    assert_eq!((at_location, in_process), (6, 4));
    assert!(
        !started
            .allocations
            .iter()
            .any(|a| a.id.as_str() == "hard4" && matches!(a.mode, AllocationMode::HardReserved))
    );
    e.run_to_end();
    assert_eq!(e.snapshot().orders[0].accepted.value.coefficient.value(), 1);
    assert_eq!(
        before["scenario"]["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"],
        10
    );
}
#[test]
fn initial_composition_sources_become_transformed_and_cannot_be_freely_taken() {
    let mut v = fixture();
    v["scenario"]["initial_conditions"]["items"][0]["components"] = json!(["pcb-001"]);
    let mut e = engine(v);
    e.step();
    let r = e.snapshot();
    let pcb = r
        .final_items
        .iter()
        .find(|i| i.id.as_str() == "pcb-001")
        .unwrap();
    assert!(matches!(pcb.lifecycle, Lifecycle::Transformed));
    assert!(matches!(pcb.holder, HolderRef::Location(_)));
    assert!(r.jobs.iter().all(|j| j.started_at_ms.is_none()));
}
#[test]
fn phase_pause_keeps_machine_releases_phase_worker_and_records_only_effective_work() {
    let mut v = fixture();
    one(&mut v);
    let worker = json!([{"role":"worker","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"person"},"qualifications":[]}]);
    let duration =
        |ms| json!({"value":ms,"unit":"millisecond","basis":"user_provided","source_ref":null});
    let op = operation(&mut v, "fixture.assemble");
    op["duration"] = Value::Null;
    op["resource_roles"] = json!([]);
    op["phases"] = json!([
      {"id":"load","duration":duration(10000),"phase_resources":worker,"interruption_policy":"non_preemptive","work_kind":"loading"},
      {"id":"process","duration":duration(40000),"phase_resources":[],"interruption_policy":"pause_resume","work_kind":"processing"},
      {"id":"unload","duration":duration(5000),"phase_resources":worker,"interruption_policy":"non_preemptive","work_kind":"unloading"}]);
    let resource = v["project"]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "ASM-01")
        .unwrap()["role_bindings"]["resources"]["execution_capacity"]
        .clone();
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["id"] == resource)
        .unwrap()["calendar_id"] = json!("MACHINE-GAP");
    v["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"MACHINE-GAP","title":"declared gap","intervals":[{"start_ms":0,"end_ms":20000},{"start_ms":30000,"end_ms":240000}],"source":"user_provided"}));
    let r = run(v);
    let asm=r.jobs.iter().find(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()=="ASM-01")).unwrap();
    assert_eq!(asm.finished_at_ms.as_ref().unwrap().0.value(), 65000);
    let work: u64 = r
        .phase_work_intervals
        .iter()
        .filter(|p| p.job_id == asm.id)
        .map(|p| p.interval.end_ms.0.value() - p.interval.start_ms.0.value())
        .sum();
    assert_eq!(work, 55000);
    let worker_claims: Vec<_> = r
        .resource_claims
        .iter()
        .filter(|c| c.job_id == asm.id && c.resource_id.as_str() == "worker-assembly")
        .flat_map(|c| {
            c.intervals
                .iter()
                .map(|i| (i.start_ms.0.value(), i.end_ms.0.value()))
        })
        .collect();
    assert_eq!(worker_claims, vec![(0, 10000), (60000, 65000)]);
}
#[test]
fn zero_duration_outputs_and_acceptance_close_at_the_same_end_instant() {
    let mut v = fixture();
    one(&mut v);
    v["scenario"]["window"]["end_ms"] = json!(0);
    operation(&mut v, "fixture.assemble")["duration"]["value"] = json!(0);
    operation(&mut v, "fixture.pack")["duration"]["value"] = json!(0);
    let r = run(v);
    assert_eq!(r.goal_met, Some(true));
    assert!(
        r.jobs
            .iter()
            .all(|j| matches!(j.state, JobStatus::Completed))
    );
    assert!(r.phase_work_intervals.is_empty());
    assert!(r.resource_claims.is_empty());
    assert!(r.events.iter().all(|e| e.at_ms.0.value() == 0));
    assert_eq!(r.acceptance_events.len(), 1);
}
#[test]
fn due_unknown_does_not_block_execution_and_pack_mapping_is_required() {
    let mut v = fixture();
    v["scenario"]["orders"][0]["due_at_ms"] = Value::Null;
    let r = run(v.clone());
    assert!(r.orders[0].due_at_ms.is_none());
    assert!(r.orders[0].tardiness_ms.is_none());
    assert!(
        r.limitations
            .iter()
            .any(|i| i.code == "business_inputs.order_due_unknown")
    );
    operation(&mut v, "fixture.pack")["pack_identity"] = Value::Null;
    let issues = Engine::new(input(v)).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|i| i.code == "simulation.pack_identity_mapping")
    );
}
#[test]
fn cancellation_keeps_the_same_engine_prefix_and_snapshots_do_not_mutate() {
    let mut e = engine(fixture());
    e.step();
    let r = e.snapshot();
    let actual = serde_json::to_value(&r).unwrap();
    e.pause();
    assert_eq!(e.step(), 0);
    e.resume();
    e.cancel();
    let cancelled = e.snapshot();
    assert!(matches!(
        cancelled.computation_status,
        ComputationStatus::Cancelled
    ));
    assert!(!cancelled.whole_window_computed);
    assert!(cancelled.goal_met.is_none());
    assert_eq!(serde_json::to_value(r).unwrap(), actual);
    let e = execute_engine(input(fixture()), &AtomicBool::new(true))
        .unwrap_or_else(|i| panic!("{i:?}"));
    assert!(e.is_finished());
    assert!(e.snapshot().computed_until_ms.is_none());
    assert!(e.snapshot().orders.is_empty());
    assert!(e.snapshot().jobs.iter().all(|j| j.started_at_ms.is_none()));
}
#[test]
fn fractional_material_is_located_before_any_integer_normalization() {
    let mut v = fixture();
    v["scenario"]["initial_conditions"]["items"][0]["quantity"]["value"] =
        json!({"coefficient":15,"decimal_places":1});
    v["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "shell")
        .unwrap()["tracking"] = json!("batch");
    let errors = Engine::new(input(v)).err().unwrap();
    assert!(
        errors
            .iter()
            .any(|i| i.code == "simulation.fractional_stock_scope"
                && i.field_path.as_deref() == Some("/initial_conditions/items/0/quantity"))
    );
}

#[test]
fn actual_order_acceptance_cannot_take_the_unallocated_task_tail() {
    let mut v = fixture();
    add_pcb(&mut v, 0);
    v["scenario"]["production_tasks"][0]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
        json!(1);
    let r = run(v);
    assert_eq!(
        r.final_items
            .iter()
            .filter(|i| i.item_type_ref.id.as_str() == "package")
            .count(),
        2
    );
    assert_eq!(r.orders[0].accepted.value.coefficient.value(), 1);
    assert_eq!(r.orders[0].outstanding.value.coefficient.value(), 1);
}

#[test]
fn generated_output_collision_is_located_before_initialization_without_renaming() {
    let original = run(fixture());
    let generated = original
        .final_items
        .iter()
        .find(|i| i.item_type_ref.id.as_str() == "assembly")
        .unwrap()
        .id
        .clone();
    let mut v = fixture();
    let mut extra = v["scenario"]["initial_conditions"]["items"][3].clone();
    extra["id"] = json!(generated.as_str());
    v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "cartons-B")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(3);
    let frozen = v.clone();
    let errors = Engine::new(input(v)).err().unwrap();
    assert!(
        errors
            .iter()
            .any(|i| i.code == "simulation.generated_item_identity_collision")
    );
    assert_eq!(
        frozen["scenario"]["initial_conditions"]["items"][5]["id"],
        generated.as_str()
    );
}

#[test]
fn full_arrival_store_retries_after_actual_consumption_without_duplicating_items() {
    let mut v = fixture();
    add_pcb(&mut v, 0);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "pcbs-A")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(1);
    let r = run(v);
    assert_eq!(r.orders[0].accepted.value.coefficient.value(), 2);
    assert_eq!(
        r.final_items
            .iter()
            .filter(|i| i.id.as_str() == "pcb-002")
            .count(),
        1
    );
    assert_eq!(r.events.iter().filter(|e|matches!(&e.payload,SimulationEventPayload::ArrivalAccepted(id)if id.as_str()=="later-pcb")).count(),1);
}

fn batch_input() -> Value {
    let mut v = fixture();
    one(&mut v);
    v["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "shell")
        .unwrap()["tracking"] = json!("batch");
    v["scenario"]["initial_conditions"]["items"][0]["id"] = json!("BATCH10");
    v["scenario"]["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"] = json!(10);
    v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .remove(1);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "shells-A")
        .unwrap()["capacity"]["value"]["coefficient"] = json!(20);
    operation(&mut v, "fixture.assemble")["inputs"][0]["quantity"]["value"]["coefficient"] =
        json!(4);
    v["project"]["boms"][0]["rows"][0]["quantity"]["value"]["coefficient"] = json!(4);
    v
}
fn future_collision(v: &mut Value, key: &ObjectId) {
    let mut future = v["scenario"]["initial_conditions"]["items"][0].clone();
    future["id"] = json!(key.as_str());
    future["quantity"]["value"]["coefficient"] = json!(5);
    v["scenario"]["arrivals"] = json!([{"id":"future-identity-collision","at_ms":100000,"destination":"shells-A","items":[future],"release_policy":"unchecked_allowed"}]);
}
#[test]
fn initial_reservation_generation_checks_all_future_declared_identities() {
    let mut v = batch_input();
    v["scenario"]["initial_conditions"]["material_allocations"] = json!([{"id":"RESERVE4","demand_ref":"TASK-LOCAL","item_refs":["BATCH10"],"quantity":{"value":{"coefficient":4,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"}]);
    let normal = engine(v.clone()).snapshot();
    let child = normal
        .final_items
        .iter()
        .find(|i| {
            i.split_from_ref
                .as_ref()
                .is_some_and(|s| s.as_str() == "BATCH10")
                && !i.allocation_refs.is_empty()
        })
        .unwrap()
        .id
        .clone();
    future_collision(&mut v, &child);
    let frozen = v.clone();
    let errors = Engine::new(input(v)).err().unwrap();
    assert!(
        errors
            .iter()
            .any(|i| i.code == "simulation.generated_item_identity_collision")
    );
    assert_eq!(
        frozen["scenario"]["initial_conditions"]["items"][0]["quantity"]["value"]["coefficient"],
        10
    );
}
#[test]
fn runtime_split_collision_fails_before_source_allocation_or_resources_change() {
    let v = batch_input();
    let normal = run(v.clone());
    let child = normal
        .final_items
        .iter()
        .find(|i| {
            i.split_from_ref
                .as_ref()
                .is_some_and(|s| s.as_str() == "BATCH10")
                && i.quantity.value.coefficient.value() == 4
        })
        .unwrap()
        .id
        .clone();
    let mut collision = v;
    future_collision(&mut collision, &child);
    let mut e = engine(collision);
    let before = serde_json::to_value(e.snapshot().final_items).unwrap();
    e.step();
    let result = e.snapshot();
    assert!(e.is_finished());
    assert!(matches!(
        result.computation_status,
        ComputationStatus::Failed
    ));
    assert!(
        result
            .limitations
            .iter()
            .any(|i| i.code == "simulation.generated_item_identity_collision")
    );
    assert_eq!(serde_json::to_value(result.final_items).unwrap(), before);
    assert!(result.allocations.is_empty());
    assert!(result.resource_claims.is_empty());
    assert!(result.jobs.iter().all(|j| j.started_at_ms.is_none()));
}
#[test]
fn deterministic_dispatch_ignores_input_vector_and_definition_iteration_order() {
    let mut v = fixture();
    add_pcb(&mut v, 90000);
    let expected = run(v.clone());
    for field in [
        "item_types",
        "locations",
        "devices",
        "resources",
        "calendars",
        "connections",
    ] {
        v["project"][field].as_array_mut().unwrap().reverse();
    }
    v["project"]["routes"][0]["steps"]
        .as_array_mut()
        .unwrap()
        .reverse();
    v["definitions"].as_array_mut().unwrap().reverse();
    v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let actual = run(v);
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}

#[test]
fn explicit_supported_scope_does_not_execute_or_validate_an_excluded_pack_rule() {
    let mut v = fixture();
    v["scenario"]["scope"] = json!([{"kind":"device","id":"ASM-01"}]);
    operation(&mut v, "fixture.pack")["pack_identity"] = Value::Null;
    let r = run(v);
    assert!(r.whole_window_computed);
    assert!(r.goal_met.is_none());
    assert_eq!(r.jobs.len(), 2);
    assert!(r.jobs.iter().all(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()=="ASM-01")));
    assert!(
        r.limitations
            .iter()
            .any(|i| i.code == "simulation.partial_scope")
    );
}

#[test]
fn identical_source_preparation_is_one_real_job_reused_by_later_cycles() {
    let mut v = fixture();
    let mut definition = v["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "fixture.assemble")
        .unwrap()
        .clone();
    definition["id"] = json!("fixture.prepare");
    definition["payload"] = json!({"category":"tasks","data":{"kind":"manual_preparation","target_role":"target","prerequisites":[],"steps":[],"resource_roles":[{"role":"worker","quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"person"},"qualifications":[]}],"duration":{"value":10000,"unit":"millisecond","basis":"user_provided","source_ref":null},"equipment_access":"exclusive","completion_effect":{"target_role":"target","marker":"ready-fixture","produces_material":false}}});
    v["definitions"].as_array_mut().unwrap().push(definition);
    v["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"fixture.prepare","version":"0.1.0"}));
    v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "ASM-01")
        .unwrap()["role_bindings"]["targets"]["target"] = json!({"kind":"device","id":"ASM-01"});
    let steps = v["project"]["routes"][0]["steps"].as_array_mut().unwrap();
    steps[0]["predecessors"] = json!(["prepare"]);
    steps.push(json!({"id":"prepare","operation_ref":{"id":"fixture.prepare","version":"0.1.0"},"device_id":"ASM-01","predecessors":[],"item_bindings":{},"resource_bindings":{"worker":"worker-assembly"}}));
    let r = run(v);
    let preparation: Vec<_> = r
        .jobs
        .iter()
        .filter(|j| j.operation_ref.id.as_str() == "fixture.prepare")
        .collect();
    assert_eq!(preparation.len(), 1);
    assert!(matches!(preparation[0].state, JobStatus::Completed));
    assert_eq!(
        preparation[0].finished_at_ms.as_ref().unwrap().0.value(),
        10000
    );
    assert!(preparation[0].input_refs.is_empty());
    assert_eq!(r.acceptance_events[0].at_ms.0.value(), 85000);
}

fn declared_duration_input() -> Value {
    let mut v = fixture();
    one(&mut v);
    let op = operation(&mut v, "fixture.assemble").clone();
    let device = v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "ASM-01")
        .unwrap();
    device["operation_binding"] = Value::Null;
    device["configuration"]["unrelated_parameter_name"] = json!({"kind":"integer","value":20000});
    let module = device["module_ref"].clone();
    v["project"]["routes"][0]["steps"][0]["operation_ref"] = module.clone();
    let definition = v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == module["id"])
        .unwrap();
    definition["payload"]["data"]["behavior"] =
        json!({"inline":op,"operation_ref":null,"selected":"inline"});
    definition["parameters"] = json!([{"id":"unrelated_parameter_name","value_type":"integer","binding":"/payload/data/behavior/inline/duration/value","label":"Declared milliseconds, id is not the bound field name","unit":"millisecond","minimum":0,"maximum":240000,"integer":true,"options":[],"basis":"module_declared"}]);
    v["project"]["parameter_sources"] = json!([{"target":{"kind":"device","id":"ASM-01"},"field_path":"/configuration/unrelated_parameter_name/value","value":{"kind":"integer","value":20000},"basis":"user_provided","source_ref":null}]);
    v
}
#[test]
fn declared_parameter_binding_changes_effective_cycle_without_guessing_the_id() {
    let v = declared_duration_input();
    let r = run(v.clone());
    assert_eq!(r.acceptance_events[0].at_ms.0.value(), 35000);
    assert_eq!(r.jobs.iter().find(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()=="ASM-01")).unwrap().finished_at_ms.as_ref().unwrap().0.value(),20000);
    let mut invalid = v;
    invalid["project"]["parameter_sources"][0]["value"]["value"] = json!(10000);
    let issues = Engine::new(input(invalid)).err().unwrap();
    assert!(
        issues
            .iter()
            .any(|i| i.code == "simulation.parameter_source_mismatch")
    );
}

fn professional_selector_input() -> Value {
    let mut v = fixture();
    let consumer: Value =
        serde_json::from_str(include_str!("../../packaging/consumer-device.design.json")).unwrap();
    let selectors: Vec<_> = consumer["data"]["definition"]["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| {
            matches!(
                p["id"].as_str(),
                Some("packaging_job_id" | "packaging_job_revision")
            )
        })
        .cloned()
        .collect();
    v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "fixture.pack-station")
        .unwrap()["parameters"] = json!(selectors);
    v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "PACK-01")
        .unwrap()["configuration"] = json!({"packaging_job_id":{"kind":"string","value":"PACKJOB1"},"packaging_job_revision":{"kind":"integer","value":1}});
    v["project"]["parameter_sources"] = json!([
        {"target":{"kind":"device","id":"PACK-01"},"field_path":"/configuration/packaging_job_id/value","value":{"kind":"string","value":"PACKJOB1"},"basis":"user_provided","source_ref":null},
        {"target":{"kind":"device","id":"PACK-01"},"field_path":"/configuration/packaging_job_revision/value","value":{"kind":"integer","value":1},"basis":"user_provided","source_ref":null}
    ]);
    v
}
#[test]
fn professional_job_selectors_preserve_frozen_sources_and_the_actual_75000ms_run() {
    for inline in [false, true] {
        let mut v = professional_selector_input();
        if inline {
            let effective = operation(&mut v, "fixture.pack").clone();
            let device = v["project"]["devices"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|d| d["id"] == "PACK-01")
                .unwrap();
            device["operation_binding"] = Value::Null;
            let module = device["module_ref"].clone();
            let definition = v["definitions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|d| d["id"] == module["id"])
                .unwrap();
            definition["payload"]["data"]["behavior"] =
                json!({"inline":effective,"operation_ref":null,"selected":"inline"});
            v["project"]["routes"][0]["steps"][1]["operation_ref"] = module;
        }
        let candidate = input(v);
        let frozen_project = serde_json::to_value(&candidate.project).unwrap();
        let frozen_scenario = serde_json::to_value(&candidate.scenario).unwrap();
        check_inputs(&candidate).unwrap_or_else(|issues| panic!("{issues:?}"));
        let engine = execute_engine(candidate.clone(), &AtomicBool::new(false))
            .unwrap_or_else(|issues| panic!("{issues:?}"));
        let r = engine.snapshot();
        assert!(r.whole_window_computed);
        assert!(r.limitations.is_empty());
        assert_eq!(r.acceptance_events.len(), 1);
        assert_eq!(r.acceptance_events[0].at_ms.0.value(), 75000);
        let pack=r.jobs.iter().find(|j|matches!(&j.execution_ref,JobExecutionRef::ProjectDevice{device_id,..}if device_id.as_str()=="PACK-01")&&j.started_at_ms.is_some()).unwrap();
        assert_eq!(pack.started_at_ms.as_ref().unwrap().0.value(), 60000);
        assert_eq!(pack.finished_at_ms.as_ref().unwrap().0.value(), 75000);
        assert_eq!(
            serde_json::to_value(&candidate.project).unwrap(),
            frozen_project
        );
        assert_eq!(
            serde_json::to_value(&candidate.scenario).unwrap(),
            frozen_scenario
        );
        assert_eq!(
            frozen_project["parameter_sources"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
#[test]
fn professional_job_selectors_require_exact_declarations_types_and_recorded_sources() {
    for variant in 0..10 {
        let mut v = professional_selector_input();
        let definition = v["definitions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["id"] == "fixture.pack-station")
            .unwrap();
        match variant {
            0=>{definition["parameters"].as_array_mut().unwrap().remove(0);},
            1=>definition["parameters"][0]["binding"]=json!("/payload/data/configuration/not_the_declared_job/value"),
            2=>definition["parameters"][1]["integer"]=json!(false),
            3=>definition["parameters"][0]["value_type"]=json!("integer"),
            4|5=>{},
            6=>definition["parameters"].as_array_mut().unwrap().push(json!({"id":"unimplemented_selector","value_type":"integer","binding":"/payload/data/configuration/unimplemented_selector/value","label":null,"unit":null,"minimum":null,"maximum":null,"integer":true,"options":[],"basis":"user_provided"})),
            7=>definition["parameters"][1]["binding"]=json!("/payload/data/configuration/packaging_job_revision/value/extra"),
            8=>{},
            _=>{let duplicate=definition["parameters"][0].clone();definition["parameters"].as_array_mut().unwrap().push(duplicate);},
        }
        if matches!(variant, 4 | 5 | 6) {
            let device = v["project"]["devices"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|d| d["id"] == "PACK-01")
                .unwrap();
            if variant == 4 {
                device["configuration"]["packaging_job_id"] = json!({"kind":"integer","value":1});
                v["project"]["parameter_sources"][0]["value"] = json!({"kind":"integer","value":1});
            } else if variant == 5 {
                device["configuration"]["packaging_job_revision"] =
                    json!({"kind":"number","value":1.0});
                v["project"]["parameter_sources"][1]["value"] =
                    json!({"kind":"number","value":1.0});
            } else {
                device["configuration"]["unimplemented_selector"] =
                    json!({"kind":"integer","value":1});
            }
        }
        if variant == 8 {
            v["project"]["parameter_sources"][1]["value"]["value"] = json!(2);
        }
        let issues = check_inputs(&input(v)).unwrap_err();
        let expected = match variant {
            0 | 9 => "simulation.parameter_declaration",
            1 | 3 | 4 | 5 => "simulation.parameter_type",
            2 | 6 | 7 => "simulation.parameter_binding_scope",
            _ => "simulation.parameter_source_mismatch",
        };
        assert!(
            issues.iter().any(|i| i.code == expected),
            "variant {variant}: {issues:?}"
        );
    }
}

#[test]
fn operation_material_ratio_cannot_silently_override_the_versioned_product_bom() {
    let mut v = fixture();
    v["project"]["boms"][0]["rows"][0]["quantity"]["value"]["coefficient"] = json!(2);
    let errors = Engine::new(input(v)).err().unwrap();
    assert!(
        errors
            .iter()
            .any(|i| i.code == "simulation.bom_operation_quantity"
                && i.field_path.as_deref() == Some("/rows/0/quantity"))
    );
}

#[test]
fn admission_requires_actual_stage_capability_and_preserves_factory_release() {
    let good = input(fixture());
    check_inputs(&good).unwrap();
    let mut e = Engine::new(good).unwrap_or_else(|issues| panic!("{issues:?}"));
    e.run_to_end();
    let r = e.snapshot();
    assert_eq!(r.acceptance_events[0].at_ms.0.value(), 75000);
    assert!(matches!(
        r.acceptance_events[0].acceptance_stage,
        AcceptanceStage::FactoryReleased
    ));
    for stage in ["received", "unloaded_and_released"] {
        let mut v = fixture();
        v["scenario"]["orders"][0]["acceptance_stage"] = json!(stage);
        let candidate = input(v);
        let before = serde_json::to_value(&candidate.scenario).unwrap();
        let admission = check_inputs(&candidate).unwrap_err();
        assert!(admission.iter().any(|i| {
            i.code == "simulation.acceptance_stage_scope"
                && i.field_path.as_deref() == Some("/orders/0/acceptance_stage")
                && i.object_ref.as_ref().is_some_and(|o| {
                    matches!(o.kind, ObjectKind::Scenario) && o.id.as_str() == "LOCAL"
                })
                && i.blocks
                    .iter()
                    .any(|p| matches!(p, CapabilityPurpose::Execution))
        }));
        assert_eq!(serde_json::to_value(&candidate.scenario).unwrap(), before);
        let execution = Engine::new(candidate).err().unwrap();
        assert_eq!(
            serde_json::to_value(admission).unwrap(),
            serde_json::to_value(execution).unwrap()
        );
    }
}

#[test]
fn cancelled_empty_plan_keeps_valid_scope_and_has_no_unscheduled_material_changes() {
    let mut v = fixture();
    v["scenario"]["orders"] = json!([]);
    v["scenario"]["production_tasks"] = json!([]);
    let candidate = input(v);
    let scope = serde_json::to_value(&candidate.scope).unwrap();
    let initial = serde_json::to_value(&candidate.scenario.initial_conditions.items).unwrap();
    check_inputs(&candidate).unwrap();
    let mut e = Engine::new(candidate).unwrap_or_else(|issues| panic!("{issues:?}"));
    e.run_to_end();
    let result = e.snapshot();
    assert_eq!(serde_json::to_value(&result.scope).unwrap(), scope);
    assert!(result.whole_window_computed);
    assert!(result.jobs.is_empty());
    assert!(result.acceptance_events.is_empty());
    assert!(result.resource_claims.is_empty());
    assert!(result.phase_work_intervals.is_empty());
    assert!(result.events.is_empty());
    assert!(result.goal_met.is_none());
    assert_eq!(result.final_items.len(), initial.as_array().unwrap().len());
    for declared in initial.as_array().unwrap() {
        let actual = result
            .final_items
            .iter()
            .find(|i| i.id.as_str() == declared["id"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            serde_json::to_value(&actual.holder).unwrap(),
            declared["holder"]
        );
        assert_eq!(
            serde_json::to_value(&actual.quantity).unwrap(),
            declared["quantity"]
        );
        assert_eq!(
            serde_json::to_value(&actual.quality).unwrap(),
            declared["quality"]
        );
        assert!(matches!(actual.lifecycle, Lifecycle::Available));
    }
    assert_eq!(
        serde_json::to_value(e.objects_at(time(0)).unwrap()).unwrap(),
        serde_json::to_value(&result.final_items).unwrap()
    );
    assert_eq!(
        serde_json::to_value(e.objects_at(result.window.end_ms).unwrap()).unwrap(),
        serde_json::to_value(&result.final_items).unwrap()
    );
}

#[test]
fn empty_demand_does_not_admit_an_unknown_scope_object() {
    let mut v = fixture();
    v["scenario"]["orders"] = json!([]);
    v["scenario"]["production_tasks"] = json!([]);
    v["scenario"]["scope"]
        .as_array_mut()
        .unwrap()
        .push(json!({"kind":"device","id":"unknown-device"}));
    let candidate = input(v);
    let admission = check_inputs(&candidate).unwrap_err();
    assert!(admission.iter().any(|i| {
        i.code == "simulation.scope_uncompiled"
            && i.object_ref
                .as_ref()
                .is_some_and(|o| o.id.as_str() == "unknown-device")
    }));
    let execution = Engine::new(candidate).err().unwrap();
    assert_eq!(
        serde_json::to_value(admission).unwrap(),
        serde_json::to_value(execution).unwrap()
    );
}

#[test]
fn fixed_processing_converts_one_declared_blank_once_and_preserves_its_origin() {
    let mut v = fixture();
    one(&mut v);
    v["scenario"]["title"] = json!("fixed conversion without physical CNC");
    v["scenario"]["scope"] = json!([{"kind":"device","id":"ASM-01"}]);
    let mut raw_type = v["project"]["item_types"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "shell")
        .unwrap()
        .clone();
    raw_type["id"] = json!("raw-blank");
    v["project"]["item_types"]
        .as_array_mut()
        .unwrap()
        .push(raw_type);
    let mut blank = v["scenario"]["initial_conditions"]["items"][0].clone();
    blank["id"] = json!("blank-001");
    blank["item_type_ref"]["id"] = json!("raw-blank");
    v["scenario"]["initial_conditions"]["items"] = json!([blank]);
    for location in v["project"]["locations"].as_array_mut().unwrap() {
        if location["id"] == "shells-A" {
            location["accepted_item_types"] = json!(["raw-blank"]);
        }
        if location["id"] == "assemblies-A" {
            location["accepted_item_types"] = json!(["shell"]);
        }
    }
    let mut definition = v["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "fixture.assemble")
        .unwrap()
        .clone();
    definition["id"] = json!("fixture.transform");
    let operation = &mut definition["payload"]["data"];
    operation["builtin"] = json!("transform");
    operation["identity_policy"] = json!("transform_with_origin");
    operation["item_roles"] = json!(["blank", "shell"]);
    operation["duration"]["value"] = json!(12000);
    let mut input_row = operation["inputs"][0].clone();
    input_row["role"] = json!("blank");
    input_row["port"] = json!("blank");
    operation["inputs"] = json!([input_row]);
    operation["outputs"][0]["role"] = json!("shell");
    operation["outputs"][0]["port"] = json!("shell-out");
    v["definitions"].as_array_mut().unwrap().push(definition);
    let operation_ref = json!({"id":"fixture.transform","version":"0.1.0"});
    v["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(operation_ref.clone());
    let device = v["project"]["devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "ASM-01")
        .unwrap();
    device["operation_binding"] = operation_ref.clone();
    device["role_bindings"]["item_types"] = json!({"blank":"raw-blank","shell":"shell"});
    let module_ref = device["module_ref"].clone();
    let device_definition = v["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == module_ref["id"])
        .unwrap();
    let mut in_port = device_definition["payload"]["data"]["ports"][0].clone();
    in_port["id"] = json!("blank");
    in_port["payload"]["data"]["item_role"] = json!("blank");
    let mut out_port = device_definition["payload"]["data"]["ports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["direction"] == "output")
        .unwrap()
        .clone();
    out_port["id"] = json!("shell-out");
    out_port["payload"]["data"]["item_role"] = json!("shell");
    device_definition["payload"]["data"]["ports"] = json!([in_port, out_port]);
    device_definition["payload"]["data"]["behavior"]["operation_ref"] = operation_ref.clone();
    v["project"]["routes"] = json!([{"id":"ROUTE-PROCESS","revision":"r1","steps":[{"id":"process","operation_ref":operation_ref,"device_id":"ASM-01","predecessors":[],"item_bindings":{"blank":"raw-blank","shell":"shell"},"resource_bindings":{"worker":"worker-assembly"}}]}]);
    v["project"]["boms"] = json!([{"id":"BOM-PROCESS","revision":"r1","rows":[{"item_type_ref":{"id":"raw-blank","revision":"r1"},"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"consume_at_operation":"process"}]}]);
    v["project"]["products"] = json!([{"id":"PROCESSED-SHELL","revision":"r1","item_type_ref":{"id":"shell","revision":"r1"},"bom_ref":{"id":"BOM-PROCESS","revision":"r1"},"routing_ref":{"id":"ROUTE-PROCESS","revision":"r1"},"artifact_refs":[]}]);
    v["project"]["connections"] = json!([
      {"kind":"material","data":{"id":"blank-input","source":{"kind":"location","data":"shells-A"},"destination":{"kind":"port","data":{"module_ref":module_ref,"instance_id":"ASM-01","port_id":"blank"}},"item_type":"raw-blank","unit":"piece","quantity_rule":null,"destination_store":"shells-A","capacity_rule":null}},
      {"kind":"material","data":{"id":"shell-output","source":{"kind":"port","data":{"module_ref":module_ref,"instance_id":"ASM-01","port_id":"shell-out"}},"destination":{"kind":"location","data":"assemblies-A"},"item_type":"shell","unit":"piece","quantity_rule":null,"destination_store":"assemblies-A","capacity_rule":null}}]);
    v["scenario"]["orders"][0]["product_ref"] = json!({"id":"PROCESSED-SHELL","revision":"r1"});
    v["scenario"]["orders"][0]["destination"] = json!("assemblies-A");
    v["scenario"]["production_tasks"][0]["routing_ref"] =
        json!({"id":"ROUTE-PROCESS","revision":"r1"});
    let candidate = input(v);
    check_inputs(&candidate).unwrap();
    let mut e = Engine::new(candidate).unwrap_or_else(|issues| panic!("{issues:?}"));
    e.run_to_end();
    let r = e.snapshot();
    assert_eq!(r.jobs.len(), 1);
    assert_eq!(r.jobs[0].finished_at_ms.as_ref().unwrap().0.value(), 12000);
    assert_eq!(r.orders[0].accepted.value.coefficient.value(), 1);
    assert_eq!(r.acceptance_events.len(), 1);
    assert_eq!(r.acceptance_events[0].at_ms.0.value(), 12000);
    let source = r
        .final_items
        .iter()
        .find(|i| i.id.as_str() == "blank-001")
        .unwrap();
    assert!(matches!(source.lifecycle, Lifecycle::Transformed));
    let outputs: Vec<_> = r
        .final_items
        .iter()
        .filter(|i| i.item_type_ref.id.as_str() == "shell")
        .collect();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].composition_refs, vec![source.id.clone()]);
    assert!(matches!(&outputs[0].holder,HolderRef::Location(id)if id.as_str()=="assemblies-A"));
    assert!(
        !e.objects_at(time(11999))
            .unwrap()
            .iter()
            .any(|i| i.item_type_ref.id.as_str() == "shell")
    );
}
