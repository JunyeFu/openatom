//! Run the real external admission gate against the unchanged saved factory input.
use serde_json::{Value, json};
use sf_domain::{cloud_execution as wire, *};
use sf_simulation::{Engine, EngineInput};
use std::{fs, path::Path};
fn read(p: &Path) -> Value { serde_json::from_slice(&fs::read(p).unwrap()).unwrap() }
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let target: u64 = args.get(3).map(|v| v.parse().expect("explicit target quantity")).unwrap_or(100);
    let root = Path::new(&args[1]);
    let project = read(&root.join("actual-frozen-project.json"));
    let scenario: ScenarioInput = serde_json::from_value(read(&root.join("actual-scenario.json"))).unwrap();
    let run = read(&root.join("actual-connected-run.json"));
    let packages = read(&root.join("actual-frozen-modules.json"));
    let definitions: Vec<ModuleDefinition> = packages.as_array().unwrap().iter()
        .map(|p| serde_json::from_value(p["definition"].clone()).unwrap()).collect();
    let input = EngineInput { run_id: "factory-external-readiness".to_owned().try_into().unwrap(),
        input_ref: serde_json::from_value(run["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(project.clone()).unwrap(), scope: scenario.scope.clone(),
        scenario, definitions };
    let identity: wire::ExecutionIdentity = serde_json::from_value(json!({"session_id":"factory-readiness",
        "execution_epoch_id":"factory-readiness-epoch","project_id":project["id"],"host_context_session_id":"factory-readiness-host"})).unwrap();
    let policy: wire::ExecutionPolicy = serde_json::from_value(json!({"clock_mode":"host_monotonic",
        "report_cutoff_ms":7200000,"stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish",
        "target_quantity":{"value":{"coefficient":target,"decimal_places":0},"unit":"piece"},
        "source_prediction_window":input.scenario.window,"observation_max_age_ms":1000})).unwrap();
    let bindings = Engine::required_external_bindings(&input).unwrap();
    let binding_count = bindings.len();
    eprintln!("Required bindings: {binding_count}; checking unchanged factory admission");
    let output = Path::new(&args[2]);
    fs::write(output.with_extension("bindings.json"), serde_json::to_vec_pretty(&bindings).unwrap()).unwrap();
    let mut engine = match Engine::new_external(input, identity, policy, bindings.clone()) {
        Ok(engine) => engine,
        Err(error) => {
            let result = json!({"state":"actual_factory_external_admission_rejected", "source":root,
                "error_code":error.code,"message":error.message,"required_bindings":binding_count,
                "cloud_commands_published":0,"whole_factory_external_verified":false});
            fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
            println!("{result}");
            return;
        }
    };
    engine.activate_external().unwrap();
    eprintln!("Factory admitted; advancing only through component device receipts");
    let mut now = 0u64;
    let mut count = 0u64;
    let mut blocked = vec![];
    loop {
        let view = engine.external_observation().unwrap();
        if view.drained { break; }
        assert!(count < 5000, "Declared fixture work exceeded the driver bound");
        let mut selected = None;
        blocked.clear();
        for job in view.queued_logical_jobs {
            for binding_id in engine.external_candidate_bindings(&job).unwrap() {
                let binding = bindings.iter().find(|b| b.binding_id == binding_id).unwrap();
                let attempt = wire::AttemptRef { logical_job_id: job.clone(), phase_id: binding.phase_id.clone(),
                    attempt_ordinal: (count + 1).try_into().unwrap(), command_id: format!("factory-command-{count}").try_into().unwrap(),
                    selected_binding_id: binding_id, phase_token: format!("factory-phase-{count}").try_into().unwrap() };
                match engine.prepare_external(attempt, format!("factory-boot-{}", binding.device_id.as_str()).try_into().unwrap(), now.try_into().unwrap()) {
                    Ok(command) => { selected = Some(command); break; }
                    Err(error) => blocked.push(json!({"job":job,"code":error.code,"message":error.message})),
                }
            }
            if selected.is_some() { break; }
        }
        let Some(command) = selected else {
            if engine.external_observation().unwrap().logical_work.len() > view.logical_work.len() { continue; }
            if let Some(at) = engine.next_external_boundary_ms().unwrap().filter(|at| at.value() > now && at.value() < 7200000) {
                now = at.value(); engine.advance_external(at).unwrap(); continue;
            }
            break;
        };
        engine.mark_external_published(&command.attempt.command_id, now.try_into().unwrap()).unwrap();
        for kind in [wire::DeviceReceiptKind::Started {}, wire::DeviceReceiptKind::Completed { device_reported_duration_ms: None }] {
            now += 1;
            let receipt = wire::DeviceReceipt { session_id: command.identity.session_id.clone(), execution_epoch_id: command.identity.execution_epoch_id.clone(),
                command_id: command.attempt.command_id.clone(), device_id: command.device_id.clone(), phase_id: command.attempt.phase_id.clone(),
                phase_token: command.attempt.phase_token.clone(), device_boot_id: command.target_device_boot_id.clone(),
                device_sequence: now.try_into().unwrap(), device_timestamp: "component-receipt-driver".to_owned().try_into().unwrap(), receipt: kind };
            engine.apply_external_receipt(receipt, now.try_into().unwrap()).unwrap();
        }
        count += 1;
        if count % 100 == 0 { eprintln!("Completed {count} external phase commands"); }
    }
    let view = engine.external_observation().unwrap();
    fs::write(output.with_extension("observation.json"), serde_json::to_vec_pretty(&json!({"jobs":view.jobs,"items":view.items,
        "transport":view.transport,"events":view.events,"acceptance_events":view.acceptance_events,"orders":view.order_outcomes,
        "allocations":view.allocations,"resource_claims":view.resource_claims,"phase_work_intervals":view.phase_work_intervals})).unwrap()).unwrap();
    let result = json!({"state":if view.goal_met && view.drained {"engine_receipt_driver_passed"} else {"engine_receipt_driver_incomplete"},
        "source":root,"scope":"Unchanged actual frozen routes, Engine component receipts; no cloud or physical device claim",
        "required_bindings":binding_count,"commands_completed":count,"elapsed_host_ms":now,"goal_met":view.goal_met,"drained":view.drained,
        "accepted_quantity":view.acceptance_events.iter().map(|a| a.quantity.value.coefficient.value()).sum::<u64>(),
        "transport_trips":view.transport.len(),"blocked":blocked,"cloud_commands_published":0,"whole_factory_cloud_verified":false});
    fs::write(output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    println!("{}", json!({"state":result["state"],"commands":count,"goal_met":view.goal_met,"drained":view.drained,"blocked":result["blocked"].as_array().unwrap().len()}));
}
