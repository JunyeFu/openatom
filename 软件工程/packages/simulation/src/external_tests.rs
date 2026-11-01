use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../../commands/tests/fixtures/quality/quality-original.json")).unwrap()
}

fn engine(v: Value) -> Engine {
    let scenario: ScenarioInput = serde_json::from_value(v["scenario"].clone()).unwrap();
    let input = EngineInput {
        run_id: "external-quality".to_owned().try_into().unwrap(),
        input_ref: serde_json::from_value(json!({
            "project_id":v["project"]["id"],"project_revision":v["project"]["revision"],
            "scenario_id":scenario.id,"scenario_revision":scenario.revision,
            "frozen_project_file":{"path":"test/project.json","kind":"project"},
            "frozen_scenario_file":{"path":"test/scenario.json","kind":"scenario"}
        })).unwrap(),
        project: serde_json::from_value(v["project"].clone()).unwrap(),
        definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        scope: scenario.scope.clone(), scenario,
    };
    let bindings = Engine::required_external_bindings(&input).unwrap();
    let identity = serde_json::from_value(json!({"session_id":"session-quality","execution_epoch_id":"epoch-quality",
        "project_id":input.project.id,"host_context_session_id":"host-quality"})).unwrap();
    let policy = serde_json::from_value(json!({"clock_mode":"host_monotonic","report_cutoff_ms":input.scenario.window.end_ms,
        "stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish",
        "target_quantity":input.scenario.acceptance_goal.as_ref().and_then(|g| g.accepted_quantity_per_day.as_ref()).or_else(|| input.scenario.orders[0].quantity.as_ref()).unwrap(),
        "source_prediction_window":input.scenario.window,"observation_max_age_ms":1000})).unwrap();
    Engine::new_external(input, identity, policy, bindings).unwrap()
}

fn drive(e: &mut Engine) -> ExternalObservation {
    e.activate_external().unwrap();
    for ordinal in 1..100 {
        let job_count = e.jobs.len();
        let view = e.external_observation().unwrap();
        let mut command = None;
        for job in view.queued_logical_jobs {
            for binding in e.external_candidate_bindings(&job).unwrap() {
                let phase = e.external.as_ref().unwrap().candidates[&binding].binding.phase_id.clone();
                let attempt = wire::AttemptRef { logical_job_id: job.clone(), phase_id: phase,
                    attempt_ordinal: uint(ordinal), command_id: format!("command-{ordinal}").try_into().unwrap(),
                    selected_binding_id: binding, phase_token: format!("token-{ordinal}").try_into().unwrap() };
                match e.prepare_external(attempt, "boot-quality".to_owned().try_into().unwrap(), uint(e.now)) {
                    Ok(c) => { command = Some(c); break; }
                    Err(err) if matches!(err.code, "external.predecessor" | "external.resources" | "external.material" | "external.output_capacity" | "external.preparation") => {}
                    Err(err) => panic!("{err}"),
                }
            }
            if command.is_some() { break; }
        }
        let Some(c) = command else {
            if e.jobs.len() > job_count { continue; }
            if let Some(at) = e.jobs.iter().filter(|j| !j.dormant && j.state.started_at_ms.is_none() && j.spec.release > e.now)
                .map(|j| j.spec.release).min().filter(|at| *at < e.external.as_ref().unwrap().policy.report_cutoff_ms.value()) {
                e.advance_external(uint(at)).unwrap();
                continue;
            }
            return e.external_observation().unwrap();
        };
        e.mark_external_published(&c.attempt.command_id, uint(e.now)).unwrap();
        for (sequence, kind) in [(1, wire::DeviceReceiptKind::Started {}),
            (2, wire::DeviceReceiptKind::Completed { device_reported_duration_ms: None })] {
            let receipt = wire::DeviceReceipt {
                session_id: c.identity.session_id.clone(), execution_epoch_id: c.identity.execution_epoch_id.clone(),
                command_id: c.attempt.command_id.clone(), device_id: c.device_id.clone(), phase_id: c.attempt.phase_id.clone(),
                phase_token: c.attempt.phase_token.clone(), device_boot_id: c.target_device_boot_id.clone(),
                device_sequence: uint(ordinal * 2 + sequence), device_timestamp: "actual-device-receipt".to_owned().try_into().unwrap(), receipt: kind,
            };
            e.apply_external_receipt(receipt, uint(e.now + 1)).unwrap();
            if sequence == 1 && c.parameters.setup_transition.is_some() {
                e.advance_external(uint(e.now + c.parameters.nominal_work_ms.value() + 1)).unwrap();
                assert!(e.jobs.iter().filter(|j| j.spec.setup.is_none()).all(|j| j.state.started_at_ms.is_none()));
                assert!(e.jobs.iter().find(|j| j.spec.id == c.attempt.logical_job_id).unwrap().state.finished_at_ms.is_none());
            }
        }
    }
    panic!("external fixture did not settle within its finite declared work");
}

#[test]
fn actual_quality_routes_rework_and_unknown_without_synthetic_release() {
    for unknown in [false, true] {
        let mut v = fixture();
        v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
        v["scenario"]["quality_presets"][1]["results"] = json!([{"kind":"string","value":"FAIL"},{"kind":"string","value":"PASS"}]);
        v["scenario"]["quality_presets"][2]["results"] = if unknown { json!([null]) } else { json!([{"kind":"string","value":"PASS"}]) };
        let mut e = engine(v);
        let view = drive(&mut e);
        let accepted: u64 = view.acceptance_events.iter().map(|a| amount(&a.quantity)).sum();
        assert_eq!(accepted, if unknown { 2 } else { 3 });
        assert_eq!(view.goal_met, !unknown);
        assert!(view.items.iter().any(|item| item.completed_reworks.value() == 1));
        assert!(view.attempts.iter().any(|a| a.command.attempt.logical_job_id.as_str().starts_with("rework:")));
        if unknown {
            assert!(view.items.iter().any(|item| matches!(item.quality, QualityState::Pending)));
            assert!(!view.drained);
        } else { assert!(view.drained); }
    }
}

#[test]
fn family_changeover_uses_one_actual_setup_before_production() {
    for mode in ["internal", "external"] {
        let mut v = fixture();
        v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
        for preset in v["scenario"]["quality_presets"].as_array_mut().unwrap() {
            preset["results"] = json!([{"kind":"string","value":"PASS"}]);
        }
        let op = &mut v["definitions"].as_array_mut().unwrap().iter_mut()
            .find(|d| d["id"] == "w5.op.assembly").unwrap()["payload"]["data"];
        op["family"] = json!("new");
        let resources = op["resource_roles"].clone();
        v["project"]["setup_policies"] = json!([{"device_id":"Assembly01","initial_family":"old",
            "changeovers":[{"from_family":"old","to_family":"new",
                "duration":{"value":500,"unit":"millisecond","basis":"user_provided","source_ref":null},
                "resources":resources,"preparation_mode":mode}],"first_article":null}]);
        let mut e = engine(v);
        let view = drive(&mut e);
        assert!(view.goal_met && view.drained, "{mode}");
        let setup: Vec<_> = view.attempts.iter().filter(|a| a.command.parameters.setup_transition.is_some()).collect();
        assert_eq!(setup.len(), 1, "{mode}");
        assert!(matches!(setup[0].command.action, wire::DeviceAction::PrepareEquipment));
        assert_eq!(setup[0].command.parameters.nominal_work_ms.value(), 500);
        let setup_job = view.jobs.iter().find(|j| j.id == setup[0].command.attempt.logical_job_id).unwrap();
        assert!(setup_job.input_refs.is_empty());
        assert!(view.jobs.iter().filter(|j| !j.input_refs.is_empty()).all(|j| j.started_at_ms.as_ref().unwrap().0.value() >= setup_job.finished_at_ms.as_ref().unwrap().0.value()));
    }
}

#[test]
fn supply_transport_waits_for_each_actual_phase_and_preserves_cargo() {
    for (source, complete) in [(include_str!("../../commands/tests/fixtures/production/p202-halls-A.json"), false),
        (include_str!("../../commands/tests/fixtures/production/p202-halls-B.json"), true)] {
    let v: Value = serde_json::from_str(source).unwrap();
    let mut e = engine(v);
    let view = drive(&mut e);
    assert_eq!(view.goal_met, complete);
    assert_eq!(view.drained, complete);
    assert_eq!(view.acceptance_events.iter().map(|a| amount(&a.quantity)).sum::<u64>(), if complete { 2 } else { 1 });
    assert!(!e.trips.is_empty());
    for trip in e.trips.values() {
        assert!(matches!(trip.phase, TransportPhase::Returned));
        assert!(trip.departed_ms.as_ref().unwrap().0.value() < trip.arrived_ms.as_ref().unwrap().0.value());
        assert!(trip.arrived_ms.as_ref().unwrap().0.value() < trip.received_ms.as_ref().unwrap().0.value());
        assert!(trip.received_ms.as_ref().unwrap().0.value() < trip.returned_ms.as_ref().unwrap().0.value());
        assert_eq!(view.attempts.iter().filter(|a| a.command.attempt.logical_job_id == trip.id).count(), 4);
        assert!(view.attempts.iter().filter(|a| a.command.attempt.logical_job_id == trip.id).all(|a| matches!(a.command.action, wire::DeviceAction::TransportCargo)));
    }
    }
}
