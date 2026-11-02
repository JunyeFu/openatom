use serde_json::{Value, json};
use sf_domain::{cloud_execution as wire, *};
use sf_simulation::{Engine, EngineInput, ExternalAttemptState, ExternalReceiptStatus};

fn uint(n: u64) -> SafeUint {
    n.try_into().unwrap()
}
fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn piece(n: u64) -> Value {
    json!({"value":{"coefficient":n,"decimal_places":0},"unit":"piece"})
}

// Reduce an existing real material/acceptance fixture to six original cycles
// with two interchangeable devices. It still uses the production compiler,
// Stock, resource capacity, transfer and acceptance consumers.
fn fixture(stock_count: u64) -> Value {
    let mut v: Value = serde_json::from_str(include_str!("fixtures/multi_sink_goal.json")).unwrap();
    for field in ["halls", "lines", "routes", "products", "boms"] {
        v["project"][field].as_array_mut().unwrap().truncate(1);
    }
    v["project"]["halls"][0]["input_locations"] = json!(["L1-SOURCE"]);
    v["project"]["halls"][0]["output_locations"] = json!(["L1-SINK"]);
    v["project"]["lines"][0]["members"] = json!(["L1-D01", "L1-D02"]);
    v["project"]["lines"][0]["boundary_ports"] = json!([]);
    v["project"]["devices"].as_array_mut().unwrap().truncate(2);
    v["project"]["resources"]
        .as_array_mut()
        .unwrap()
        .truncate(2);
    v["project"]["routes"][0]["steps"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .retain(|l| matches!(l["id"].as_str(), Some("L1-SOURCE" | "L1-SINK")));
    v["project"]["connections"]
        .as_array_mut()
        .unwrap()
        .truncate(4);
    for c in v["project"]["connections"].as_array_mut().unwrap() {
        let d = &mut c["data"];
        if d["destination"]["kind"] == "port" {
            d["source"]["data"] = json!("L1-SOURCE");
            d["destination_store"] = json!("L1-SOURCE");
        } else {
            d["destination"]["data"] = json!("L1-SINK");
            d["destination_store"] = json!("L1-SINK");
        }
    }
    v["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    v["scenario"]["production_tasks"][0]["planned_quantity"] = piece(6);
    v["scenario"]["acceptance_goal"]["additional_acceptance_scopes"] = json!([]);
    let seed = v["scenario"]["initial_conditions"]["items"][0].clone();
    v["scenario"]["initial_conditions"]["items"] = Value::Array(
        (0..stock_count)
            .map(|n| {
                let mut item = seed.clone();
                item["id"] = json!(format!("blank-{n}"));
                item
            })
            .collect(),
    );
    v
}
fn input(v: Value) -> EngineInput {
    let scenario: ScenarioInput = serde_json::from_value(v["scenario"].clone()).unwrap();
    EngineInput {
        run_id: "external-original-run".to_owned().try_into().unwrap(),
        input_ref: serde_json::from_value(
            json!({"project_id":v["project"]["id"],"project_revision":v["project"]["revision"],
            "scenario_id":scenario.id,"scenario_revision":scenario.revision,
            "frozen_project_file":{"path":"unit/project.json","kind":"project"},
            "frozen_scenario_file":{"path":"unit/scenario.json","kind":"scenario"}}),
        )
        .unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope: scenario.scope.clone(),
        scenario,
    }
}
fn identity() -> wire::ExecutionIdentity {
    serde_json::from_value(
        json!({"session_id":"session-one","execution_epoch_id":"epoch-one",
        "project_id":"NFR-B60-balanced","host_context_session_id":"host-one"}),
    )
    .unwrap()
}
fn policy() -> wire::ExecutionPolicy {
    serde_json::from_value(
        json!({"clock_mode":"host_monotonic","report_cutoff_ms":10000,
        "stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish",
        "target_quantity":piece(6),"source_prediction_window":{"start_ms":0,"end_ms":1200000},
        "observation_max_age_ms":1000}),
    )
    .unwrap()
}
fn binding(device: &str) -> wire::CandidateBinding {
    serde_json::from_value(json!({"binding_id":format!("binding-{device}"),"device_id":device,
        "device_module_ref":{"id":"p201.b60.station","version":"1.0.0"},
        "operation_ref":{"id":"p201.b60.station","version":"1.0.0"},
        "execution_capacity_ref":{"kind":"resource","id":format!("{device}-CAP")},
        "input_ports":[{"role_id":"workpiece","port_id":"input","location_id":"L1-SOURCE","item_type_id":"benchmark-item-v1","unit_id":"piece"}],
        "output_ports":[{"role_id":"workpiece","port_id":"output","location_id":"L1-SINK","item_type_id":"benchmark-item-v1","unit_id":"piece"}],
        "resources":[],"phase_id":"cycle","nominal_work_ms":2000})).unwrap()
}
fn engine(v: Value) -> Engine {
    Engine::new_external(
        input(v),
        identity(),
        policy(),
        vec![binding("L1-D01"), binding("L1-D02")],
    )
    .unwrap()
}
fn attempt(job: ObjectId, device: &str, name: &str, ordinal: u64) -> wire::AttemptRef {
    serde_json::from_value(json!({"logical_job_id":job,"phase_id":"cycle","attempt_ordinal":ordinal,
        "command_id":name,"selected_binding_id":format!("binding-{device}"),"phase_token":format!("token-{name}")})).unwrap()
}
fn prepare(e: &mut Engine, index: usize, device: &str, name: &str, at: u64) -> wire::Command {
    let job = e.external_observation().unwrap().queued_logical_jobs[index].clone();
    e.prepare_external(
        attempt(job, device, name, 1),
        "boot-one".to_owned().try_into().unwrap(),
        uint(at),
    )
    .unwrap()
}
fn publish(e: &mut Engine, c: &wire::Command, at: u64) {
    e.mark_external_published(&c.attempt.command_id, uint(at))
        .unwrap();
}
fn receipt(c: &wire::Command, sequence: u64, kind: wire::DeviceReceiptKind) -> wire::DeviceReceipt {
    wire::DeviceReceipt {
        session_id: c.identity.session_id.clone(),
        execution_epoch_id: c.identity.execution_epoch_id.clone(),
        command_id: c.attempt.command_id.clone(),
        device_id: c.device_id.clone(),
        phase_id: c.attempt.phase_id.clone(),
        phase_token: c.attempt.phase_token.clone(),
        device_boot_id: c.target_device_boot_id.clone(),
        device_sequence: uint(sequence),
        device_timestamp: "device-clock-is-not-business-time"
            .to_owned()
            .try_into()
            .unwrap(),
        receipt: kind,
    }
}
fn started(c: &wire::Command) -> wire::DeviceReceipt {
    receipt(c, 1, wire::DeviceReceiptKind::Started {})
}
fn completed(c: &wire::Command) -> wire::DeviceReceipt {
    receipt(
        c,
        2,
        wire::DeviceReceiptKind::Completed {
            device_reported_duration_ms: Some(uint(999)),
        },
    )
}
fn states(e: &Engine) -> Value {
    serde_json::to_value(e.snapshot()).unwrap()
}

fn two_stage_fixture() -> Value {
    let mut v = fixture(12);
    v["scenario"]["production_tasks"][0]["planned_quantity"] = piece(12);
    let mut operation = v["definitions"][0].clone();
    operation["id"] = json!("task8.finish");
    v["definitions"].as_array_mut().unwrap().push(operation);
    v["project"]["module_refs"].as_array_mut().unwrap().push(json!({"id":"task8.finish","version":"1.0.0"}));
    let mut buffer = v["project"]["locations"][1].clone();
    buffer["id"] = json!("L1-BUFFER");
    buffer["purpose"] = json!("buffer");
    v["project"]["locations"].as_array_mut().unwrap().push(buffer);
    for c in v["project"]["connections"].as_array_mut().unwrap() {
        if c["data"]["destination"]["kind"] != "port" {
            c["data"]["destination"]["data"] = json!("L1-BUFFER");
            c["data"]["destination_store"] = json!("L1-BUFFER");
        }
    }
    for (old, new) in [("L1-D01", "L1-D03"), ("L1-D02", "L1-D04")] {
        let index = if old == "L1-D01" { 0 } else { 1 };
        let mut device: Value = serde_json::from_str(&v["project"]["devices"][index].to_string().replace(old, new)).unwrap();
        device["module_ref"] = json!({"id":"task8.finish","version":"1.0.0"});
        device["route_operation_refs"] = json!([{"id":"task8.finish","version":"1.0.0"}]);
        device["pose"]["translation"]["x"] = json!(20 + index * 4);
        v["project"]["devices"].as_array_mut().unwrap().push(device);
        let resource: Value = serde_json::from_str(&v["project"]["resources"][index].to_string().replace(old, new)).unwrap();
        v["project"]["resources"].as_array_mut().unwrap().push(resource);
        v["project"]["lines"][0]["members"].as_array_mut().unwrap().push(json!(new));
        let connections: Vec<Value> = v["project"]["connections"].as_array().unwrap().iter().filter(|c| c.to_string().contains(old)).cloned().collect();
        for c in connections {
            let mut c: Value = serde_json::from_str(&c.to_string().replace(old, new)).unwrap();
            let data = &mut c["data"];
            if data["destination"]["kind"] == "port" {
                data["source"]["data"] = json!("L1-BUFFER");
                data["destination_store"] = json!("L1-BUFFER");
                data["destination"]["data"]["module_ref"] = json!({"id":"task8.finish","version":"1.0.0"});
            } else {
                data["destination"]["data"] = json!("L1-SINK");
                data["destination_store"] = json!("L1-SINK");
                data["source"]["data"]["module_ref"] = json!({"id":"task8.finish","version":"1.0.0"});
            }
            v["project"]["connections"].as_array_mut().unwrap().push(c);
        }
    }
    let mut step = v["project"]["routes"][0]["steps"][0].clone();
    step["id"] = json!("L1-FINISH-step");
    step["device_id"] = json!("L1-D03");
    step["operation_ref"] = json!({"id":"task8.finish","version":"1.0.0"});
    step["predecessors"] = json!(["L1-D01-step"]);
    v["project"]["routes"][0]["steps"].as_array_mut().unwrap().push(step);
    v
}
fn two_stage_bindings() -> Vec<wire::CandidateBinding> {
    ["L1-D01", "L1-D02", "L1-D03", "L1-D04"].into_iter().map(|device| {
        let mut b = binding(device);
        if matches!(device, "L1-D03" | "L1-D04") {
            b.device_module_ref.id = "task8.finish".to_owned().try_into().unwrap();
            b.operation_ref.id = "task8.finish".to_owned().try_into().unwrap();
            b.input_ports[0].location_id = id("L1-BUFFER");
        } else { b.output_ports[0].location_id = id("L1-BUFFER"); }
        b
    }).collect()
}
fn two_stage_policy() -> wire::ExecutionPolicy {
    let mut p = policy();p.target_quantity = serde_json::from_value(piece(12)).unwrap();p
}

#[test]
fn two_operations_gate_predecessors_preserve_wip_and_count_only_twelve_final_units() {
    let mut e = Engine::new_external(input(two_stage_fixture()), identity(), two_stage_policy(), two_stage_bindings()).unwrap();
    e.activate_external().unwrap();
    let work = e.external_observation().unwrap().logical_work;
    assert_eq!(work.len(), 24);
    let first = work.iter().find(|w| w.cycle == 0 && w.route_step_id == Some(id("L1-D01-step"))).unwrap();
    let second = work.iter().find(|w| w.cycle == 0 && w.route_step_id == Some(id("L1-FINISH-step"))).unwrap();
    assert_eq!(first.allowed_binding_ids, vec![id("binding-L1-D01"), id("binding-L1-D02")]);
    assert_eq!(second.allowed_binding_ids, vec![id("binding-L1-D03"), id("binding-L1-D04")]);
    let boot = "boot-one".to_owned().try_into().unwrap();
    assert_eq!(e.prepare_external(attempt(first.logical_job_id.clone(), "L1-D03", "wrong-operation", 1), boot, uint(0)).unwrap_err().code, "external.binding");
    assert_eq!(e.prepare_external(attempt(second.logical_job_id.clone(), "L1-D03", "before-predecessor", 1), "boot-one".to_owned().try_into().unwrap(), uint(0)).unwrap_err().code, "external.predecessor");
    e.advance_external(uint(5000)).unwrap();
    assert!(e.external_observation().unwrap().acceptance_events.is_empty());
    let mut at = 5000;
    let mut last = None;
    for cycle in 0..12 {
        for (step, device) in [("L1-D01-step", if cycle%2==0 {"L1-D01"} else {"L1-D02"}), ("L1-FINISH-step", if cycle%2==0 {"L1-D03"} else {"L1-D04"})] {
            let job = work.iter().find(|w| w.cycle == cycle && w.route_step_id == Some(id(step))).unwrap();
            let c = e.prepare_external(attempt(job.logical_job_id.clone(), device, &format!("cycle-{cycle}-{step}"), 1), "boot-one".to_owned().try_into().unwrap(), uint(at)).unwrap();
            publish(&mut e, &c, at);
            at += 1;e.apply_external_receipt(started(&c), uint(at)).unwrap();
            at += 1;e.apply_external_receipt(completed(&c), uint(at)).unwrap();
            if step == "L1-D01-step" { assert_eq!(e.external_observation().unwrap().acceptance_events.len(), cycle as usize); }
            last = Some(c);
        }
    }
    let result = e.external_observation().unwrap();
    assert_eq!(result.attempts.len(),24);assert_eq!(result.acceptance_events.len(),12);
    assert!(result.goal_met && result.drained);
    assert!(result.resource_claims.iter().all(|claim| claim.interval_end_causes.iter().all(|end| matches!(end, ClaimIntervalEndCause::Released))));
    assert_eq!(result.items.len(),12);
    assert!(result.items.iter().all(|item| matches!(&item.holder, HolderRef::Location(location) if location==&id("L1-SINK"))));
    for n in 0..12 { assert!(result.items.iter().any(|item| item.id==id(&format!("blank-{n}")))); }
    e.apply_external_receipt(completed(&last.unwrap()), uint(at)).unwrap();
    assert_eq!(e.external_observation().unwrap().acceptance_events.len(),12);
}

#[test]
fn chained_work_requires_each_operation_binding_and_terminal_target_not_command_count() {
    let b=two_stage_bindings();
    let no_finish=Engine::new_external(input(two_stage_fixture()), identity(), two_stage_policy(), b[..2].to_vec());
    assert!(matches!(no_finish, Err(e) if e.code=="external.binding"));
    let mut wrong=two_stage_policy();wrong.target_quantity=serde_json::from_value(piece(24)).unwrap();
    let result=Engine::new_external(input(two_stage_fixture()),identity(),wrong,b);
    assert!(matches!(result,Err(e) if e.code=="external.target"));
}

#[test]
fn prepare_reserves_same_stock_and_capacity_without_physical_or_bound_job_commit() {
    let mut e = engine(fixture(1));
    assert_eq!(
        e.external_observation().unwrap().queued_logical_jobs.len(),
        6
    );
    assert!(e.external_observation().unwrap().jobs.is_empty());
    assert!(
        e.prepare_external(
            attempt(id("no-work"), "L1-D01", "before-activation", 1),
            "boot-one".to_owned().try_into().unwrap(),
            uint(0)
        )
        .is_err()
    );
    e.activate_external().unwrap();
    let before = states(&e);
    let c = prepare(&mut e, 0, "L1-D01", "command-one", 0);
    let view = e.external_observation().unwrap();
    assert_eq!(
        serde_json::to_value(&view.items).unwrap(),
        before["final_items"]
    );
    assert!(view.jobs.is_empty());
    assert!(view.events.is_empty());
    assert!(view.resource_claims.is_empty());
    assert_eq!(view.attempts[0].pending_items, vec![(id("blank-0"), 1)]);
    let job = view.queued_logical_jobs[1].clone();
    assert_eq!(
        e.prepare_external(
            attempt(job.clone(), "L1-D01", "busy", 1),
            c.target_device_boot_id.clone(),
            uint(0)
        )
        .unwrap_err()
        .code,
        "external.resources"
    );
    assert_eq!(
        e.prepare_external(
            attempt(job, "L1-D02", "empty", 1),
            c.target_device_boot_id.clone(),
            uint(0)
        )
        .unwrap_err()
        .code,
        "external.material"
    );
    assert_eq!(e.external_observation().unwrap().attempts.len(), 1);
    e.withdraw_external_unpublished(&c.attempt.command_id)
        .unwrap();
    let replacement = e
        .prepare_external(
            attempt(c.attempt.logical_job_id.clone(), "L1-D02", "replacement", 2),
            c.target_device_boot_id.clone(),
            uint(0),
        )
        .unwrap();
    assert_eq!(replacement.device_id, id("L1-D02"));
}

#[test]
fn valid_start_commits_once_and_only_receipt_completes_the_original_production_path() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D02", "command-one", 0);
    publish(&mut e, &c, 0);
    let r = started(&c);
    assert_eq!(
        e.apply_external_receipt(r.clone(), uint(10))
            .unwrap()
            .status,
        ExternalReceiptStatus::Started
    );
    let at_start = e.external_observation().unwrap();
    assert_eq!(at_start.jobs.len(), 1);
    assert_eq!(
        at_start
            .items
            .iter()
            .filter(|i| matches!(i.lifecycle, Lifecycle::InProcess))
            .count(),
        1
    );
    assert!(
        matches!(&at_start.jobs[0].execution_ref,JobExecutionRef::ProjectDevice{device_id,..} if *device_id==id("L1-D02"))
    );
    let allocations = states(&e)["allocations"].clone();
    assert_eq!(allocations.as_array().unwrap().len(), 1);
    assert!(e.apply_external_receipt(r, uint(10)).unwrap().duplicate);
    assert_eq!(states(&e)["allocations"], allocations);
    e.advance_external(uint(3000)).unwrap();
    assert_eq!(e.step(), 0);
    e.run_to_end();
    assert_eq!(e.external_observation().unwrap().elapsed_ms, 3000);
    assert!(
        e.external_observation()
            .unwrap()
            .acceptance_events
            .is_empty()
    );
    let result = e.apply_external_receipt(completed(&c), uint(3100)).unwrap();
    assert_eq!(result.status, ExternalReceiptStatus::Completed);
    assert_eq!(result.device_reported_duration_ms, Some(999));
    let done = e.external_observation().unwrap();
    assert_eq!(done.acceptance_events.len(), 1);
    assert_eq!(done.acceptance_events[0].at_ms.0.value(), 3100);
    assert!(
        matches!(&done.items.iter().find(|i| i.id==id("blank-0")).unwrap().holder,HolderRef::Location(l) if *l==id("L1-SINK"))
    );
    assert_eq!(done.phase_work_intervals[0].interval.start_ms.0.value(), 10);
    assert_eq!(done.phase_work_intervals[0].interval.end_ms.0.value(), 3100);
    assert!(
        e.apply_external_receipt(completed(&c), uint(3100))
            .unwrap()
            .duplicate
    );
    assert_eq!(e.external_observation().unwrap().acceptance_events.len(), 1);
}

#[test]
fn mismatched_identity_and_sequence_conflict_cannot_change_ownership() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "identity-command", 0);
    publish(&mut e, &c, 0);
    let base = serde_json::to_value(started(&c)).unwrap();
    let before = states(&e);
    for key in [
        "session_id",
        "execution_epoch_id",
        "command_id",
        "device_id",
        "device_boot_id",
        "phase_id",
        "phase_token",
    ] {
        let mut bad = base.clone();
        bad[key] = json!("wrong-identity");
        assert!(
            e.apply_external_receipt(serde_json::from_value(bad).unwrap(), uint(5))
                .is_err(),
            "{key}"
        );
        assert_eq!(states(&e), before, "{key}");
    }
    e.apply_external_receipt(started(&c), uint(5)).unwrap();
    let before = states(&e);
    assert_eq!(
        e.apply_external_receipt(
            receipt(
                &c,
                1,
                wire::DeviceReceiptKind::Completed {
                    device_reported_duration_ms: None
                }
            ),
            uint(5)
        )
        .unwrap_err()
        .code,
        "external.sequence"
    );
    assert!(
        e.apply_external_receipt(receipt(&c, 2, wire::DeviceReceiptKind::Started {}), uint(5))
            .is_err()
    );
    assert_eq!(states(&e), before);
}

#[test]
fn early_complete_is_retained_without_synthetic_start_then_reconciled_exactly_once() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "early-command", 0);
    publish(&mut e, &c, 0);
    let out = e.apply_external_receipt(completed(&c), uint(20)).unwrap();
    assert_eq!(out.status, ExternalReceiptStatus::RetainedUnapplied);
    let pending = e.external_observation().unwrap();
    assert!(pending.jobs.is_empty());
    assert!(pending.acceptance_events.is_empty());
    assert_eq!(pending.attempts[0].unapplied_receipts.len(), 1);
    assert!(
        e.apply_external_receipt(completed(&c), uint(20))
            .unwrap()
            .duplicate
    );
    let mut conflict = completed(&c);
    conflict.receipt = wire::DeviceReceiptKind::Completed {
        device_reported_duration_ms: None,
    };
    assert_eq!(
        e.apply_external_receipt(conflict, uint(20))
            .unwrap_err()
            .code,
        "external.sequence"
    );
    assert!(
        e.apply_external_receipt(
            receipt(&c, 3, wire::DeviceReceiptKind::CancelledNotStarted {}),
            uint(20)
        )
        .is_err()
    );
    e.apply_external_receipt(started(&c), uint(21)).unwrap();
    e.apply_external_receipt(completed(&c), uint(22)).unwrap();
    let done = e.external_observation().unwrap();
    assert_eq!(done.acceptance_events.len(), 1);
    assert_eq!(done.jobs.len(), 1);
    assert!(done.attempts[0].unapplied_receipts.is_empty());
}

#[test]
fn published_unknown_cannot_reassign_but_proven_not_started_cancellation_can() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "cancel-command", 0);
    publish(&mut e, &c, 0);
    assert!(
        e.withdraw_external_unpublished(&c.attempt.command_id)
            .is_err()
    );
    let second = attempt(
        c.attempt.logical_job_id.clone(),
        "L1-D02",
        "retry-command",
        2,
    );
    assert_eq!(
        e.prepare_external(second.clone(), c.target_device_boot_id.clone(), uint(0))
            .unwrap_err()
            .code,
        "external.live_attempt"
    );
    e.apply_external_receipt(
        receipt(&c, 1, wire::DeviceReceiptKind::Accepted {}),
        uint(1),
    )
    .unwrap();
    e.apply_external_receipt(
        receipt(&c, 2, wire::DeviceReceiptKind::CancelledNotStarted {}),
        uint(2),
    )
    .unwrap();
    assert!(e.external_observation().unwrap().jobs.is_empty());
    let retry = e
        .prepare_external(second, c.target_device_boot_id.clone(), uint(3))
        .unwrap();
    publish(&mut e, &retry, 3);
    e.apply_external_receipt(started(&retry), uint(4)).unwrap();
    let before = states(&e);
    assert!(
        e.apply_external_receipt(
            receipt(&retry, 2, wire::DeviceReceiptKind::CancelledNotStarted {}),
            uint(4)
        )
        .is_err()
    );
    assert_eq!(states(&e), before);
    assert!(
        e.prepare_external(
            attempt(
                retry.attempt.logical_job_id.clone(),
                "L1-D01",
                "illegal-third",
                3
            ),
            retry.target_device_boot_id.clone(),
            uint(4)
        )
        .is_err()
    );
}

#[test]
fn cutoff_stops_dispatch_and_keeps_an_immutable_prefix_while_admitted_work_drains() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "late-command", 0);
    publish(&mut e, &c, 0);
    let unpublished = prepare(&mut e, 1, "L1-D02", "unpublished", 1);
    e.advance_external(uint(10000)).unwrap();
    let cutoff = e.external_cutoff_observation().unwrap().clone();
    assert_eq!(cutoff.elapsed_ms, 10000);
    assert!(cutoff.dispatch_stopped);
    assert!(!cutoff.drained);
    assert!(cutoff.jobs.is_empty());
    assert!(cutoff.acceptance_events.is_empty());
    assert!(cutoff.attempts.iter().any(|a| a.command.attempt.command_id
        == unpublished.attempt.command_id
        && a.state == ExternalAttemptState::WithdrawnUnpublished));
    let job = e.external_observation().unwrap().queued_logical_jobs[2].clone();
    assert_eq!(
        e.prepare_external(
            attempt(job, "L1-D02", "after-cutoff", 1),
            c.target_device_boot_id.clone(),
            uint(10001)
        )
        .unwrap_err()
        .code,
        "external.cutoff"
    );
    e.apply_external_receipt(started(&c), uint(1200010))
        .unwrap();
    e.apply_external_receipt(completed(&c), uint(1200050))
        .unwrap();
    let live = e.external_observation().unwrap();
    assert_eq!(live.elapsed_ms, 1200050);
    assert!(live.drained);
    assert!(!live.goal_met);
    assert_eq!(live.acceptance_events[0].at_ms.0.value(), 1200050);
    assert_eq!(
        e.external_cutoff_observation().unwrap().elapsed_ms,
        cutoff.elapsed_ms
    );
    assert!(e.external_cutoff_observation().unwrap().jobs.is_empty());
    assert!(
        e.external_cutoff_observation()
            .unwrap()
            .acceptance_events
            .is_empty()
    );
    assert_eq!(
        e.advance_external(uint(1200049)).unwrap_err().code,
        "external.clock"
    );
}

#[test]
fn six_logical_jobs_choose_one_of_two_devices_without_duplicate_outputs() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    for n in 0..6 {
        let device = if n % 2 == 0 { "L1-D01" } else { "L1-D02" };
        let at = n * 100;
        let c = prepare(&mut e, 0, device, &format!("six-{n}"), at);
        publish(&mut e, &c, at);
        e.apply_external_receipt(started(&c), uint(at + 1)).unwrap();
        e.apply_external_receipt(completed(&c), uint(at + 80))
            .unwrap();
    }
    let done = e.external_observation().unwrap();
    assert_eq!(done.jobs.len(), 6);
    assert_eq!(done.items.len(), 6);
    assert_eq!(done.acceptance_events.len(), 6);
    assert!(done.queued_logical_jobs.is_empty());
    assert!(done.goal_met);
    assert!(done.drained);
    assert_eq!(done.attempts.len(), 6);
    assert_eq!(done.elapsed_ms, 580);
    e.advance_external(uint(10000)).unwrap();
    assert_eq!(e.external_observation().unwrap().elapsed_ms, 580);
    assert!(e.external_cutoff_observation().is_none());
}

#[test]
fn batch_reservations_follow_the_actual_remaining_child_after_partial_take() {
    let mut v = fixture(1);
    v["project"]["item_types"][0]["tracking"] = json!("batch");
    v["definitions"][3]["payload"]["data"]["tracking"] = json!("batch");
    v["scenario"]["initial_conditions"]["items"][0]["quantity"] = piece(6);
    let mut e = engine(v);
    e.activate_external().unwrap();
    let a = prepare(&mut e, 0, "L1-D01", "batch-a", 0);
    publish(&mut e, &a, 0);
    let b = prepare(&mut e, 1, "L1-D02", "batch-b", 0);
    publish(&mut e, &b, 0);
    assert!(
        e.external_observation()
            .unwrap()
            .attempts
            .iter()
            .all(|a| a.pending_items == vec![(id("blank-0"), 1)])
    );
    e.apply_external_receipt(started(&a), uint(1)).unwrap();
    let view = e.external_observation().unwrap();
    let remaining = view
        .items
        .iter()
        .find(|i| {
            matches!(i.lifecycle, Lifecycle::Available) && i.quantity.value.coefficient.value() == 5
        })
        .unwrap();
    assert!(
        view.attempts
            .iter()
            .find(|a| a.command.attempt.command_id == b.attempt.command_id)
            .unwrap()
            .pending_items
            == vec![(remaining.id.clone(), 1)]
    );
    e.apply_external_receipt(started(&b), uint(2)).unwrap();
    e.apply_external_receipt(completed(&a), uint(3)).unwrap();
    e.apply_external_receipt(completed(&b), uint(4)).unwrap();
    let view = e.external_observation().unwrap();
    assert_eq!(
        view.acceptance_events
            .iter()
            .map(|a| a.quantity.value.coefficient.value())
            .sum::<u64>(),
        2
    );
    assert_eq!(
        view.items
            .iter()
            .filter(|i| matches!(i.lifecycle, Lifecycle::Available))
            .map(|i| i.quantity.value.coefficient.value())
            .sum::<u64>(),
        6
    );
}

#[test]
fn completed_command_with_full_sink_retains_original_pending_output_and_claim() {
    let mut v = fixture(6);
    v["project"]["locations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|l| l["id"] == "L1-SINK")
        .unwrap()["capacity"] = piece(1);
    let mut blocker = v["scenario"]["initial_conditions"]["items"][0].clone();
    blocker["id"] = json!("preexisting-sink-item");
    blocker["holder"]["id"] = json!("L1-SINK");
    v["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(blocker);
    let mut e = engine(v);
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "blocked-output", 0);
    publish(&mut e, &c, 0);
    e.apply_external_receipt(started(&c), uint(1)).unwrap();
    assert_eq!(
        e.apply_external_receipt(completed(&c), uint(50))
            .unwrap()
            .status,
        ExternalReceiptStatus::Completed
    );
    let view = e.external_observation().unwrap();
    assert!(view.acceptance_events.is_empty());
    assert!(!view.goal_met);
    assert!(!view.drained);
    assert!(matches!(view.jobs[0].state, JobStatus::WaitingOutput));
    assert!(!view.jobs[0].pending_outputs.is_empty());
    assert!(
        view.items
            .iter()
            .any(|i| matches!(i.holder, HolderRef::Job(_)))
    );
    assert!(!view.jobs[0].job_resources.is_empty());
    let next = view.queued_logical_jobs[0].clone();
    assert_eq!(
        e.prepare_external(
            attempt(next, "L1-D01", "still-busy", 1),
            c.target_device_boot_id.clone(),
            uint(51)
        )
        .unwrap_err()
        .code,
        "external.resources"
    );
    e.advance_external(uint(10000)).unwrap();
    assert!(!e.external_observation().unwrap().drained);
}

#[test]
fn started_failure_preserves_wip_and_resources_without_rollback() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "failed-work", 0);
    publish(&mut e, &c, 0);
    e.apply_external_receipt(started(&c), uint(1)).unwrap();
    let failed = receipt(
        &c,
        2,
        wire::DeviceReceiptKind::Failed {
            code: "fault".to_owned().try_into().unwrap(),
            message: "stopped with work in fixture"
                .to_owned()
                .try_into()
                .unwrap(),
        },
    );
    assert_eq!(
        e.apply_external_receipt(failed.clone(), uint(10))
            .unwrap()
            .status,
        ExternalReceiptStatus::Failed
    );
    assert!(
        e.apply_external_receipt(failed, uint(10))
            .unwrap()
            .duplicate
    );
    e.advance_external(uint(10000)).unwrap();
    let view = e.external_observation().unwrap();
    assert!(!view.drained);
    assert!(view.acceptance_events.is_empty());
    assert!(
        view.items
            .iter()
            .any(|i| matches!(i.lifecycle, Lifecycle::InProcess)
                && matches!(i.holder, HolderRef::Job(_)))
    );
    assert!(!view.jobs[0].job_resources.is_empty());
    assert!(matches!(view.jobs[0].state, JobStatus::Paused));
    assert_eq!(view.phase_work_intervals[0].interval.end_ms.0.value(), 10);
    assert!(
        e.apply_external_receipt(completed(&c), uint(10001))
            .is_err()
    );
}

#[test]
fn completion_observed_at_cutoff_is_part_of_its_frozen_prefix() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "at-cutoff", 0);
    publish(&mut e, &c, 0);
    e.apply_external_receipt(started(&c), uint(1)).unwrap();
    e.apply_external_receipt(completed(&c), uint(10000))
        .unwrap();
    let cutoff = e.external_cutoff_observation().unwrap();
    assert_eq!(cutoff.acceptance_events.len(), 1);
    assert_eq!(cutoff.acceptance_events[0].at_ms.0.value(), 10000);
    assert!(cutoff.drained);
}

#[test]
fn stop_dispatch_withdraws_only_unpublished_work_and_does_not_cancel_started_work() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let a = prepare(&mut e, 0, "L1-D01", "safe-unpublished", 0);
    let b = prepare(&mut e, 1, "L1-D02", "active-after-stop", 0);
    publish(&mut e, &b, 0);
    e.apply_external_receipt(started(&b), uint(1)).unwrap();
    e.cancel();
    let view = e.external_observation().unwrap();
    assert!(view.dispatch_stopped);
    assert!(!view.drained);
    assert_eq!(
        view.attempts
            .iter()
            .find(|x| x.command.attempt.command_id == a.attempt.command_id)
            .unwrap()
            .state,
        ExternalAttemptState::WithdrawnUnpublished
    );
    e.apply_external_receipt(completed(&b), uint(2)).unwrap();
    assert!(e.external_observation().unwrap().drained);
}

#[test]
fn candidate_mappings_are_checked_against_the_original_compiler() {
    let base = serde_json::to_value(binding("L1-D01")).unwrap();
    for (pointer, bad) in [
        ("/device_module_ref/version", json!("9.0.0")),
        ("/input_ports/0/unit_id", json!("kg")),
        ("/input_ports/0/port_id", json!("absent")),
        ("/output_ports/0/location_id", json!("L1-SOURCE")),
        ("/execution_capacity_ref/id", json!("L1-D02-CAP")),
        ("/nominal_work_ms", json!(1)),
        ("/phase_id", json!("other")),
    ] {
        let mut candidate = base.clone();
        *candidate.pointer_mut(pointer).unwrap() = bad;
        assert!(
            Engine::new_external(
                input(fixture(6)),
                identity(),
                policy(),
                vec![serde_json::from_value(candidate).unwrap()]
            )
            .is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn original_local_mode_still_uses_nominal_completion_and_same_stock() {
    let mut e = Engine::new(input(fixture(6))).unwrap();
    e.run_to_end();
    let run = e.snapshot();
    assert!(run.whole_window_computed);
    assert_eq!(run.jobs.len(), 6);
    assert_eq!(run.acceptance_events.len(), 6);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 2000);
    assert_eq!(run.acceptance_events[5].at_ms.0.value(), 12000);
}

fn multiphase_fixture() -> Value {
    let mut v = fixture(6);
    let phase = v["definitions"][0]["payload"]["data"]["behavior"]["inline"]["phases"][0].clone();
    v["definitions"][0]["payload"]["data"]["behavior"]["inline"]["phases"] =
        Value::Array(["load", "auto", "unload"].iter().map(|name| {
            let mut p = phase.clone(); p["id"] = json!(name); p
        }).collect());
    v
}
fn phase_bindings() -> Vec<wire::CandidateBinding> {
    ["L1-D01", "L1-D02"].iter().flat_map(|device|
        ["load", "auto", "unload"].iter().map(move |phase| {
            let mut b = binding(device);
            b.binding_id = id(&format!("{device}-{phase}")); b.phase_id = (*phase).to_owned().try_into().unwrap(); b
        })).collect()
}
#[test]
fn external_pause_resume_retains_material_and_excludes_paused_time() {
    let mut v = multiphase_fixture();
    v["definitions"][0]["payload"]["data"]["behavior"]["inline"]["phases"][0]["interruption_policy"] = json!("pause_resume");
    let mut e = Engine::new_external(input(v), identity(), policy(), phase_bindings()).unwrap();
    e.activate_external().unwrap();
    let job = e.external_observation().unwrap().queued_logical_jobs[0].clone();
    let c = prepare_phase(&mut e, &job, "L1-D01", "load", 1, 0).unwrap();
    publish(&mut e, &c, 0);
    assert!(e.apply_external_receipt(receipt(&c, 3, wire::DeviceReceiptKind::Resumed {}), uint(1)).is_err());
    e.apply_external_receipt(started(&c), uint(2)).unwrap();
    let pause = receipt(&c, 4, wire::DeviceReceiptKind::Paused {});
    e.apply_external_receipt(pause.clone(), uint(10)).unwrap();
    e.advance_external(uint(100)).unwrap();
    let view = e.external_observation().unwrap();
    assert!(view.items.iter().any(|i| matches!(&i.holder, HolderRef::Job(id) if id == &job)));
    assert!(view.acceptance_events.is_empty());
    assert_eq!(view.phase_work_intervals.len(), 1);
    assert_eq!(view.phase_work_intervals[0].effective_progress_end_ms.0.value(), 8);
    assert!(e.apply_external_receipt(pause, uint(100)).unwrap().duplicate);
    assert!(e.apply_external_receipt(receipt(&c, 5, wire::DeviceReceiptKind::Completed { device_reported_duration_ms: None }), uint(100)).is_err());
    e.apply_external_receipt(receipt(&c, 6, wire::DeviceReceiptKind::Resumed {}), uint(101)).unwrap();
    e.apply_external_receipt(receipt(&c, 7, wire::DeviceReceiptKind::Completed { device_reported_duration_ms: None }), uint(110)).unwrap();
    let view = e.external_observation().unwrap();
    assert_eq!(view.phase_work_intervals.len(), 2);
    assert_eq!(view.phase_work_intervals[1].effective_progress_start_ms.0.value(), 8);
    assert_eq!(view.phase_work_intervals[1].effective_progress_end_ms.0.value(), 17);
    assert!(view.acceptance_events.is_empty());
}
#[test]
fn multiphase_requires_every_phase_on_each_selectable_device() {
    let mut bindings = phase_bindings(); bindings.pop();
    let Err(error) = Engine::new_external(input(multiphase_fixture()), identity(), policy(), bindings)
        else { panic!("A load-only candidate must not strand admitted work"); };
    assert_eq!(error.code, "external.binding");
    assert!(error.message.contains("all operation phases"));
}
#[test]
fn external_three_phases_keep_one_input_and_drain_after_cutoff() {
    let mut e = Engine::new_external(input(multiphase_fixture()), identity(), policy(), phase_bindings()).unwrap();
    e.activate_external().unwrap();
    let jobs = e.external_observation().unwrap().queued_logical_jobs;
    let job = jobs[0].clone();
    let mut first = None;
    for (n, phase) in ["load", "auto", "unload"].iter().enumerate() {
        let at = if n == 0 { 0 } else { 10001 + n as u64 * 10 };
        let mut a = attempt(job.clone(), "L1-D01", &format!("phase-{n}"), n as u64 + 1);
        a.phase_id = (*phase).to_owned().try_into().unwrap(); a.selected_binding_id = id(&format!("L1-D01-{phase}"));
        if n > 0 {
            let mut wrong = a.clone(); wrong.selected_binding_id = id(&format!("L1-D02-{phase}"));
            assert_eq!(e.prepare_external(wrong, "boot-one".to_owned().try_into().unwrap(), uint(at)).unwrap_err().code, "external.binding");
        }
        let c = e.prepare_external(a, "boot-one".to_owned().try_into().unwrap(), uint(at)).unwrap();
        publish(&mut e, &c, at);
        if n == 0 { first = Some(c.clone()); }
        e.apply_external_receipt(started(&c), uint(at + 1)).unwrap();
        if n == 1 {
            let old = first.as_ref().unwrap();
            assert!(e.apply_external_receipt(completed(old), uint(at + 2)).unwrap().duplicate);
            assert!(e.external_observation().unwrap().acceptance_events.is_empty());
        }
        e.apply_external_receipt(completed(&c), uint(at + 3)).unwrap();
        let view = e.external_observation().unwrap();
        assert_eq!(view.items.len(), 6);
        assert_eq!(view.phase_work_intervals.len(), n + 1);
        assert_eq!(view.phase_work_intervals[n].phase_id.as_str(), *phase);
        if n < 2 {
            assert!(view.acceptance_events.is_empty());
            assert!(view.items.iter().any(|item| item.id == id("blank-0") && matches!(&item.holder, HolderRef::Job(j) if j == &job)));
            assert!(view.queued_logical_jobs.contains(&job));
        } else {
            assert_eq!(view.acceptance_events.len(), 1);
            assert!(view.drained);
            assert!(view.items.iter().any(|item| item.id == id("blank-0") && matches!(&item.holder, HolderRef::Location(l) if l == &id("L1-SINK"))));
        }
    }
    assert_eq!(e.external_observation().unwrap().attempts.len(), 3);
}

#[test]
fn phase_gap_holds_device_and_cancellation_retries_without_new_material() {
    let mut e = Engine::new_external(input(multiphase_fixture()), identity(), policy(), phase_bindings()).unwrap();
    e.activate_external().unwrap();
    let jobs = e.external_observation().unwrap().queued_logical_jobs;
    let make = |job: ObjectId, phase: &str, name: &str, ordinal| {
        let mut a = attempt(job, "L1-D01", name, ordinal);
        a.phase_id = phase.to_owned().try_into().unwrap();
        a.selected_binding_id = id(&format!("L1-D01-{phase}")); a
    };
    let boot: DeviceBootId = "boot-one".to_owned().try_into().unwrap();
    assert_eq!(e.prepare_external(make(jobs[0].clone(), "auto", "skip-load", 1), boot.clone(), uint(0)).unwrap_err().code, "external.phase");
    let load = e.prepare_external(make(jobs[0].clone(), "load", "load", 1), boot.clone(), uint(0)).unwrap();
    publish(&mut e, &load, 0);
    e.apply_external_receipt(started(&load), uint(1)).unwrap();
    e.apply_external_receipt(completed(&load), uint(2)).unwrap();
    assert_eq!(e.prepare_external(make(jobs[1].clone(), "load", "other-job", 1), boot.clone(), uint(3)).unwrap_err().code, "external.resources");
    let auto = e.prepare_external(make(jobs[0].clone(), "auto", "auto-cancel", 2), boot.clone(), uint(4)).unwrap();
    publish(&mut e, &auto, 4);
    e.apply_external_receipt(receipt(&auto, 1, wire::DeviceReceiptKind::CancelledNotStarted {}), uint(5)).unwrap();
    let retry = e.prepare_external(make(jobs[0].clone(), "auto", "auto-retry", 3), boot, uint(6)).unwrap();
    publish(&mut e, &retry, 6);
    e.advance_external(uint(9000)).unwrap();
    let before = e.external_observation().unwrap();
    assert!(before.acceptance_events.is_empty());
    assert_eq!(before.phase_work_intervals.len(), 1);
    assert!(before.attempts.iter().find(|a| a.command.attempt.command_id == retry.attempt.command_id).unwrap().pending_items.is_empty());
    e.apply_external_receipt(started(&retry), uint(9001)).unwrap();
    e.apply_external_receipt(completed(&retry), uint(9002)).unwrap();
    let after = e.external_observation().unwrap();
    assert_eq!(after.items.len(), 6);
    assert!(after.acceptance_events.is_empty());
    assert_eq!(after.items.iter().filter(|item| matches!(&item.holder, HolderRef::Job(j) if j == &jobs[0])).count(), 1);
}

#[test]
fn cancellation_rejected_as_started_requires_original_start_reconciliation() {
    let mut e = engine(fixture(6));
    e.activate_external().unwrap();
    let c = prepare(&mut e, 0, "L1-D01", "reconcile-start", 0);
    publish(&mut e, &c, 0);
    let rejected = receipt(&c, 2, wire::DeviceReceiptKind::CancelRejectedStarted {});
    assert_eq!(
        e.apply_external_receipt(rejected.clone(), uint(5))
            .unwrap()
            .status,
        ExternalReceiptStatus::RetainedUnapplied
    );
    e.apply_external_receipt(
        receipt(&c, 0, wire::DeviceReceiptKind::Accepted {}),
        uint(6),
    )
    .unwrap();
    let pending = e.external_observation().unwrap();
    assert!(pending.jobs.is_empty());
    assert_eq!(
        pending.attempts[0].state,
        ExternalAttemptState::ReconciliationRequired
    );
    e.apply_external_receipt(started(&c), uint(7)).unwrap();
    let allocations = states(&e)["allocations"].clone();
    assert_eq!(
        e.apply_external_receipt(rejected, uint(8)).unwrap().status,
        ExternalReceiptStatus::Started
    );
    assert_eq!(states(&e)["allocations"], allocations);
    e.apply_external_receipt(
        receipt(
            &c,
            3,
            wire::DeviceReceiptKind::Completed {
                device_reported_duration_ms: None,
            },
        ),
        uint(9),
    )
    .unwrap();
    let done = e.external_observation().unwrap();
    assert_eq!(done.acceptance_events.len(), 1);
    assert!(done.attempts[0].unapplied_receipts.is_empty());
}


fn tasks_for_ordering(v: &mut Value, priorities: [i64; 3], releases: [u64; 3]) {
    let original = v["scenario"]["production_tasks"][0].clone();
    v["scenario"]["production_tasks"] = Value::Array((0..3).map(|i| {
        let mut task = original.clone();
        task["id"] = json!(["A", "B", "C"][i]);
        task["planned_quantity"] = piece(2);
        task["priority"] = json!(priorities[i]);
        task["release_at_ms"] = json!(releases[i]);
        task
    }).collect());
}
fn queued_tasks(e: &Engine) -> Vec<String> {
    let view = e.external_observation().unwrap();
    view.queued_logical_jobs.iter().map(|id| view.logical_work.iter()
        .find(|work| work.logical_job_id == *id).unwrap().production_task_id.as_str().to_owned()).collect()
}

#[test]
fn external_queue_uses_declared_priority_before_stable_task_order() {
    let mut v = fixture(6);
    tasks_for_ordering(&mut v, [0, 5, 2], [0, 0, 0]);
    let mut e = engine(v);
    assert_eq!(queued_tasks(&e), ["B", "B", "C", "C", "A", "A"]);
    e.activate_external().unwrap();
    let command = prepare(&mut e, 0, "L1-D01", "priority-command", 0);
    assert!(command.attempt.logical_job_id.as_str().contains("B"));
    assert!(e.external_observation().unwrap().acceptance_events.is_empty());
}

#[test]
fn external_queue_uses_due_date_before_stable_task_order() {
    let mut v = fixture(6);
    tasks_for_ordering(&mut v, [0, 0, 0], [0, 0, 0]);
    v["scenario"]["acceptance_goal"] = Value::Null;
    v["scenario"]["orders"] = Value::Array((0..3).map(|i| json!({
        "id":format!("order-{i}"),"product_ref":{"id":"L1-PRODUCT","revision":"r1"},
        "quantity":piece(2),"release_at_ms":0,"due_at_ms":([9000,4000,7000][i]),
        "destination":"L1-SINK","acceptance_stage":"factory_released","allow_partial":true,"priority":0
    })).collect());
    for i in 0..3 {v["scenario"]["production_tasks"][i]["order_allocations"] = json!([
        {"order_id":format!("order-{i}"),"quantity":piece(2)}]);}
    assert_eq!(queued_tasks(&engine(v)), ["B", "B", "C", "C", "A", "A"]);
}

#[test]
fn external_queue_uses_declared_release_before_stable_task_order() {
    let mut v = fixture(6);
    tasks_for_ordering(&mut v, [0, 0, 0], [20, 0, 10]);
    let mut e = engine(v);
    assert_eq!(queued_tasks(&e), ["B", "B", "C", "C", "A", "A"]);
    e.activate_external().unwrap();
    let future = e.external_observation().unwrap().queued_logical_jobs[2].clone();
    assert!(e.prepare_external(attempt(future, "L1-D01", "not-released", 1),
        "boot-one".to_owned().try_into().unwrap(), uint(0)).is_err());
}

#[test]
fn external_queue_prioritizes_all_released_work_after_device_wait() {
    let mut v = fixture(6);
    tasks_for_ordering(&mut v, [0, 5, 2], [0, 10, 5]);
    let mut e = engine(v);
    e.activate_external().unwrap();
    // Device admission can arrive after every task's declared release. An
    // earlier release must no longer outrank a higher current priority.
    e.advance_external(uint(20)).unwrap();
    assert_eq!(queued_tasks(&e), ["B", "B", "C", "C", "A", "A"]);
    let command = prepare(&mut e, 0, "L1-D01", "late-admission-priority", 20);
    let view = e.external_observation().unwrap();
    let work = view.logical_work.iter().find(|w| w.logical_job_id == command.attempt.logical_job_id).unwrap();
    assert_eq!(work.production_task_id.as_str(), "B");
    assert!(view.acceptance_events.is_empty());
}

#[test]
fn external_queue_prioritizes_due_dates_after_all_work_is_released() {
    let mut v = fixture(6);
    tasks_for_ordering(&mut v, [0, 0, 0], [0, 10, 5]);
    v["scenario"]["acceptance_goal"] = Value::Null;
    v["scenario"]["orders"] = Value::Array((0..3).map(|i| json!({
        "id":format!("order-{i}"),"product_ref":{"id":"L1-PRODUCT","revision":"r1"},
        "quantity":piece(2),"release_at_ms":0,"due_at_ms":([9000,4000,7000][i]),
        "destination":"L1-SINK","acceptance_stage":"factory_released","allow_partial":true,"priority":0
    })).collect());
    for i in 0..3 {v["scenario"]["production_tasks"][i]["order_allocations"] = json!([
        {"order_id":format!("order-{i}"),"quantity":piece(2)}]);}
    let mut e = engine(v);
    e.activate_external().unwrap();
    e.advance_external(uint(20)).unwrap();
    assert_eq!(queued_tasks(&e), ["B", "B", "C", "C", "A", "A"]);
}

fn flow_fixture() -> Value {
    let mut v = multiphase_fixture();
    let op = &mut v["definitions"][0]["payload"]["data"]["behavior"]["inline"];
    let mut tail = op["phases"][2].clone(); tail["id"] = json!("return");
    op["phases"].as_array_mut().unwrap().push(tail);
    op["material_flow"] = json!({"primary_role":"workpiece","auxiliary_roles":[],
        "acquire_after_phase":"load","consume_after_phase":"auto",
        "reserve_before_phase":"unload","deliver_after_phase":"unload",
        "required_primary_attributes":{},"attributes_after_consume":{}});
    v
}
fn flow_bindings() -> Vec<wire::CandidateBinding> {
    let mut bindings = phase_bindings();
    for device in ["L1-D01", "L1-D02"] {
        let mut b = binding(device); b.binding_id = id(&format!("{device}-return"));
        b.phase_id = "return".to_owned().try_into().unwrap(); bindings.push(b);
    }
    bindings
}
fn prepare_phase(e: &mut Engine, job: &ObjectId, device: &str, phase: &str, ordinal: u64, at: u64) -> Result<wire::Command, sf_simulation::ExternalError> {
    let mut a = attempt(job.clone(), device, &format!("{device}-{phase}-{ordinal}"), ordinal);
    a.phase_id = phase.to_owned().try_into().unwrap(); a.selected_binding_id = id(&format!("{device}-{phase}"));
    e.prepare_external(a, "boot-one".to_owned().try_into().unwrap(), uint(at))
}
#[test]
fn external_flow_acquires_after_load_delivers_after_unload_and_waits_for_return() {
    let mut e = Engine::new_external(input(flow_fixture()), identity(), policy(), flow_bindings()).unwrap();
    e.activate_external().unwrap(); let job=e.external_observation().unwrap().queued_logical_jobs[0].clone();
    for (n, phase) in ["load","auto","unload","return"].iter().enumerate() {
        let t=n as u64*10;
        let c=prepare_phase(&mut e,&job,"L1-D01",phase,n as u64+1,t).unwrap();publish(&mut e,&c,t);
        e.apply_external_receipt(started(&c),uint(t+1)).unwrap();
        if n==0 {
            let view=e.external_observation().unwrap();
            assert!(matches!(&view.items.iter().find(|i|i.id==id("blank-0")).unwrap().holder,HolderRef::Location(l) if l==&id("L1-SOURCE")));
        }
        e.apply_external_receipt(completed(&c),uint(t+2)).unwrap();
        let view=e.external_observation().unwrap();let item=view.items.iter().find(|i|i.id==id("blank-0")).unwrap();
        if n<2 {assert!(matches!(&item.holder,HolderRef::Job(j) if j==&job));assert!(view.acceptance_events.is_empty());}
        else {assert!(matches!(&item.holder,HolderRef::Location(l) if l==&id("L1-SINK")));}
        if n==2 {assert!(view.queued_logical_jobs.contains(&job));assert!(!matches!(view.jobs[0].state,JobStatus::Completed));}
        if n==3 {assert!(matches!(view.jobs[0].state,JobStatus::Completed));}
    }
}
#[test]
fn external_flow_reserves_destination_before_dispatch_and_releases_on_cancel() {
    let mut v=flow_fixture();
    v["project"]["locations"].as_array_mut().unwrap().iter_mut().find(|l|l["id"]=="L1-SINK").unwrap()["capacity"]=piece(1);
    let mut e=Engine::new_external(input(v),identity(),policy(),flow_bindings()).unwrap();e.activate_external().unwrap();
    let jobs=e.external_observation().unwrap().queued_logical_jobs;
    for (n,phase) in ["load","auto"].iter().enumerate() {
        for (k,device) in ["L1-D01","L1-D02"].iter().enumerate() {
            let t=n as u64*10+k as u64*3;
            let c=prepare_phase(&mut e,&jobs[k],device,phase,n as u64+1,t).unwrap();publish(&mut e,&c,t);
            e.apply_external_receipt(started(&c),uint(t+1)).unwrap();e.apply_external_receipt(completed(&c),uint(t+2)).unwrap();
        }
    }
    let first=prepare_phase(&mut e,&jobs[0],"L1-D01","unload",3,20).unwrap();publish(&mut e,&first,20);
    assert_eq!(prepare_phase(&mut e,&jobs[1],"L1-D02","unload",3,21).unwrap_err().code,"external.output_capacity");
    assert!(e.external_output_capacity_waits().any(|(job, location)| job == &jobs[1] && location.as_str() == "L1-SINK"));
    e.apply_external_receipt(receipt(&first,3,wire::DeviceReceiptKind::CancelledNotStarted {}),uint(22)).unwrap();
    let next=prepare_phase(&mut e,&jobs[1],"L1-D02","unload",3,23).unwrap();publish(&mut e,&next,23);
    e.apply_external_receipt(started(&next),uint(24)).unwrap();e.apply_external_receipt(completed(&next),uint(25)).unwrap();
    assert_eq!(prepare_phase(&mut e,&jobs[0],"L1-D01","unload",4,26).unwrap_err().code,"external.output_capacity");
}

#[test]
fn external_multi_piece_yield_uses_declared_quantity_and_never_duplicates_output() {
    for second_output in [false,true] {
    let mut v=fixture(6);
    let op=&mut v["definitions"][0]["payload"]["data"]["behavior"]["inline"];
    op["identity_policy"]=json!("transform_with_origin");op["outputs"][0]["quantity"]=piece(2);
    let mut bindings=vec![binding("L1-D01"),binding("L1-D02")];
    if second_output {
        let mut output=op["outputs"][0].clone();output["role"]=json!("secondary");output["port"]=json!("secondary");output["quantity"]=piece(1);
        op["outputs"].as_array_mut().unwrap().push(output);op["item_roles"].as_array_mut().unwrap().push(json!("secondary"));
        let mut port=v["definitions"][0]["payload"]["data"]["ports"][1].clone();port["id"]=json!("secondary");port["payload"]["data"]["item_role"]=json!("secondary");
        v["definitions"][0]["payload"]["data"]["ports"].as_array_mut().unwrap().push(port);
        let extra:Vec<_>=v["project"]["connections"].as_array().unwrap().iter().filter(|c|c["data"]["source"]["kind"]=="port").map(|c| {
            let mut c=c.clone();c["data"]["id"]=json!(format!("{}-secondary",c["data"]["id"].as_str().unwrap()));c["data"]["source"]["data"]["port_id"]=json!("secondary");c
        }).collect();v["project"]["connections"].as_array_mut().unwrap().extend(extra);
        for device in v["project"]["devices"].as_array_mut().unwrap(){device["role_bindings"]["item_types"]["secondary"]=json!("benchmark-item-v1");}
        for binding in &mut bindings {let mut output=binding.output_ports[0].clone();output.role_id="secondary".to_owned().try_into().unwrap();output.port_id="secondary".to_owned().try_into().unwrap();binding.output_ports.push(output);}
    }
    let per_cycle=if second_output {3}else{2};
    let mut p=policy();p.target_quantity=serde_json::from_value(piece(per_cycle*6)).unwrap();
    assert_eq!(Engine::new_external(input(v.clone()),identity(),policy(),bindings.clone()).err().unwrap().code,"external.target");
    let mut e=Engine::new_external(input(v),identity(),p,bindings).unwrap();e.activate_external().unwrap();
    for n in 0..6 {
        let t=n*10;let c=prepare(&mut e,0,"L1-D01",&format!("batch-{n}"),t);assert_eq!(serde_json::to_value(&c.parameters.quantity).unwrap(),piece(2));publish(&mut e,&c,t);
        e.apply_external_receipt(started(&c),uint(t+1)).unwrap();
        assert_eq!(e.external_observation().unwrap().acceptance_events.iter().map(|a|a.quantity.value.coefficient.value()).sum::<u64>(),n*per_cycle);
        e.apply_external_receipt(completed(&c),uint(t+2)).unwrap();assert!(e.apply_external_receipt(completed(&c),uint(t+3)).unwrap().duplicate);
        let observed=e.external_observation().unwrap();assert_eq!(observed.acceptance_events.iter().map(|a|a.quantity.value.coefficient.value()).sum::<u64>(),(n+1)*per_cycle);assert_eq!(observed.goal_met,n==5);
    }
    assert!(e.external_observation().unwrap().drained);
    }
}

fn preparation_fixture() -> (Value, Vec<wire::CandidateBinding>) {
    let mut v=fixture(6);let mut bindings=vec![binding("L1-D01"),binding("L1-D02")];
    v["project"]["routes"][0]["steps"][0]["predecessors"]=json!(["prepare-two"]);
    for (name,marker,prerequisites) in [("prepare-one","ready",json!([])),("prepare-two","calibrated",json!([{"target_role":"device","marker":"ready"}]))] {
        let reference=json!({"id":format!("test.{name}"),"version":"1.0.0"});
        let mut definition=v["definitions"][0].clone();definition["id"]=reference["id"].clone();
        definition["payload"]=json!({"category":"tasks","data":{"kind":"supported_automatic","target_role":"device","prerequisites":prerequisites,"steps":[],"resource_roles":[],"duration":{"value":500,"unit":"millisecond","basis":"user_provided","source_ref":"explicit-test"},"equipment_access":"exclusive","completion_effect":{"target_role":"device","marker":marker,"produces_material":false}}});
        v["definitions"].as_array_mut().unwrap().push(definition);v["project"]["module_refs"].as_array_mut().unwrap().push(reference.clone());
        v["project"]["routes"][0]["steps"].as_array_mut().unwrap().push(json!({"id":name,"operation_ref":reference,"device_id":"L1-D01","predecessors":[],"item_bindings":{},"resource_bindings":{}}));
        for device in v["project"]["devices"].as_array_mut().unwrap() {
            device["role_bindings"]["targets"]["device"]=json!({"kind":"device","id":device["id"]});
            device["route_operation_refs"].as_array_mut().unwrap().push(reference.clone());
            let mut b=binding(device["id"].as_str().unwrap());b.binding_id=id(&format!("{}-{name}",device["id"].as_str().unwrap()));b.operation_ref=serde_json::from_value(reference.clone()).unwrap();b.input_ports.clear();b.output_ports.clear();b.nominal_work_ms=uint(500);bindings.push(b);
        }
    }
    (v,bindings)
}
#[test]
fn external_preparation_markers_need_actual_completion_and_do_not_manufacture_items() {
    let (v,bindings)=preparation_fixture();let mut e=Engine::new_external(input(v),identity(),policy(),bindings).unwrap();e.activate_external().unwrap();
    let view=e.external_observation().unwrap();assert_eq!(view.logical_work.len(),8);
    let first=view.logical_work.iter().find(|j|j.route_step_id == Some(id("prepare-one"))).unwrap().logical_job_id.clone();
    let second=view.logical_work.iter().find(|j|j.route_step_id == Some(id("prepare-two"))).unwrap().logical_job_id.clone();
    let mut a=attempt(second.clone(),"L1-D01","prepare-second",1);a.selected_binding_id=id("L1-D01-prepare-two");
    assert_eq!(e.prepare_external(a.clone(),"boot-one".to_owned().try_into().unwrap(),uint(0)).unwrap_err().code,"external.preparation");
    let mut begin=attempt(first,"L1-D01","prepare-first",1);begin.selected_binding_id=id("L1-D01-prepare-one");
    let c=e.prepare_external(begin,"boot-one".to_owned().try_into().unwrap(),uint(0)).unwrap();assert!(matches!(c.action,wire::DeviceAction::PrepareEquipment));assert_eq!(c.parameters.quantity.unit.as_str(),"cycle");publish(&mut e,&c,0);e.apply_external_receipt(started(&c),uint(1)).unwrap();
    e.advance_external(uint(600)).unwrap();assert_eq!(e.prepare_external(a.clone(),"boot-one".to_owned().try_into().unwrap(),uint(600)).unwrap_err().code,"external.preparation");
    e.apply_external_receipt(completed(&c),uint(601)).unwrap();
    let c=e.prepare_external(a,"boot-one".to_owned().try_into().unwrap(),uint(602)).unwrap();publish(&mut e,&c,602);e.apply_external_receipt(started(&c),uint(603)).unwrap();e.apply_external_receipt(completed(&c),uint(604)).unwrap();
    let view=e.external_observation().unwrap();assert_eq!(view.items.len(),6);assert!(view.acceptance_events.is_empty());assert_eq!(view.jobs.iter().filter(|j|matches!(j.state,JobStatus::Completed)).count(),2);
    let c=prepare(&mut e,0,"L1-D01","after-ready",605);publish(&mut e,&c,605);e.apply_external_receipt(started(&c),uint(606)).unwrap();e.apply_external_receipt(completed(&c),uint(607)).unwrap();assert_eq!(e.external_observation().unwrap().acceptance_events.len(),1);
}


#[test]
fn external_forecast_keeps_published_and_overdue_work_occupied() {
    let mut e=engine(fixture(6));e.activate_external().unwrap();
    let jobs=e.external_observation().unwrap().queued_logical_jobs;
    let forecast=|e:&Engine|e.external_candidate_plan(&jobs[1],&id("binding-L1-D01")).unwrap().estimated_completion_ms;
    assert_eq!(forecast(&e),Some(2000));
    let c=prepare(&mut e,0,"L1-D01","forecast-live",0);publish(&mut e,&c,0);
    assert_eq!(forecast(&e),None);
    e.apply_external_receipt(started(&c),uint(1)).unwrap();assert_eq!(forecast(&e),Some(4001));
    e.advance_external(uint(2002)).unwrap();assert_eq!(forecast(&e),None);
    assert!(e.external_observation().unwrap().acceptance_events.is_empty());
    e.apply_external_receipt(completed(&c),uint(2003)).unwrap();assert_eq!(forecast(&e),Some(4003));
}
#[test]
fn external_forecast_uses_current_phase_and_preserves_real_progress() {
    let mut v=multiphase_fixture();
    v["definitions"][0]["payload"]["data"]["behavior"]["inline"]["phases"][1]["duration"]["value"]=json!(500);
    let mut bindings=phase_bindings();for b in &mut bindings {if b.phase_id.as_str()=="auto" {b.nominal_work_ms=uint(500);}}
    let mut e=Engine::new_external(input(v),identity(),policy(),bindings).unwrap();e.activate_external().unwrap();
    let job=e.external_observation().unwrap().queued_logical_jobs[0].clone();
    let c=prepare_phase(&mut e,&job,"L1-D01","load",1,0).unwrap();publish(&mut e,&c,0);
    e.apply_external_receipt(started(&c),uint(1)).unwrap();e.apply_external_receipt(completed(&c),uint(2)).unwrap();
    let before=states(&e);let next=e.external_candidate_plan(&job,&id("L1-D01-auto")).unwrap();
    assert_eq!(next.estimated_completion_ms,Some(502));assert_eq!(states(&e),before);
}
