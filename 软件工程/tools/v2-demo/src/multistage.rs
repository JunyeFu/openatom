use super::*;

fn multistage_bindings(project: &ProjectState) -> Vec<wire::CandidateBinding> {
    let mut result = bindings(project);
    for binding in &mut result {
        if binding.operation_ref.id.as_str() == "p201.task8.finish" {
            binding.input_ports[0].location_id = id("L1-BUFFER");
        } else {
            binding.output_ports[0].location_id = id("L1-BUFFER");
        }
    }
    result
}

fn check(run: &RunResult) {
    assert!(matches!(run.computation_status, ComputationStatus::Completed));
    assert!(run.whole_window_computed);
    assert_eq!(run.jobs.len(), 24);
    assert_eq!(run.final_items.len(), 12);
    assert_eq!(run.acceptance_events.len(), 12);
    assert_eq!(run.acceptance_events.first().unwrap().at_ms.0.value(), 20000);
    assert_eq!(run.acceptance_events.last().unwrap().at_ms.0.value(), 130000);
    for item in &run.final_items {
        assert_eq!(value(&item.holder), json!({"kind":"location","id":"L1-SINK"}));
    }
}

pub(super) fn build(original: &Path, staged: &Path, output: &Path, receipt: &Path) {
    assert!(!output.exists(), "new output required");
    let before = read_project_archive(original).unwrap();
    let mut host = Host::new("multistage-build");
    host.open(staged);
    let edits = read(&staged.parent().unwrap().join("multistage-edit.json"));
    host.call(true, "apply_commands", json!({"base_revision":edits["base_revision"],"commands":edits["commands"]}));
    let project: ProjectState = typed(host.object("project", PROJECT));
    assert_eq!(project.devices.len(), 4);
    assert_eq!(project.routes[0].steps.len(), 2);
    let run: RunResult = typed(host.run());
    write(&receipt.with_file_name("new-prediction.json"), &value(&run));
    check(&run);
    host.save(output);
    let saved = read_project_archive(output).unwrap();
    geometry::retained(&before, &saved);
    let old = geometry::run_entries(&before);
    assert_eq!(geometry::run_entries(&saved).len(), old.len() + 1);
    let candidates = multistage_bindings(&saved.project);
    write(&output.parent().unwrap().join("candidate-bindings.json"), &value(&candidates));
    write(&output.parent().unwrap().join("profile-bindings.json"), &json!({"source_mode":"software_in_loop","local_project_id":PROJECT,"scenario_id":SCENARIO,"target_quantity":{"value":{"coefficient":12,"decimal_places":0},"unit":"piece"},"report_cutoff_ms":240000,"execution_profiles":candidates.iter().map(|b|json!({"profile_id":if b.nominal_work_ms.value()==10000{"processor-a"}else{"processor-b"},"device_id":b.device_id,"binding_id":b.binding_id,"processing_time_ms":b.nominal_work_ms})).collect::<Vec<_>>() }));
    write(receipt, &json!({"passed":true,"producer":"real Host apply_commands/run/save","pid":std::process::id(),"project_revision":saved.project.revision,"new_run_id":run.id,"old_run_ids":old.iter().map(|r|r.id.clone()).collect::<Vec<_>>(),"logical_jobs":24,"final_accepted":12,"first_final_ms":20000,"last_final_ms":130000,"old_run_frozen_module_asset_bytes_unchanged":true,"actual_cloud_acceptance":null}));
    println!("Real Host multi-operation build/save passed");
}

pub(super) fn verify(current: &Path, original: &Path, output: &Path) {
    let archive = read_project_archive(current).unwrap();
    let prior = read_project_archive(original).unwrap();
    geometry::retained(&prior, &archive);
    let old = geometry::run_entries(&prior);
    let all = geometry::run_entries(&archive);
    let run = all.iter().find(|r| !old.iter().any(|p| p.id == r.id)).unwrap();
    check(run);
    let mut host = Host::new("multistage-reopen");
    host.open(current);
    let project: ProjectState = typed(host.object("project", PROJECT));
    let scenario: ScenarioInput = typed(host.object("scenario", SCENARIO));
    assert_eq!(value(&project), value(&archive.project));
    for run in &all {
        let restored = host.call(true, "get_run", json!({"run_id":run.id,"query":{"kind":"full_result"}}));
        assert_eq!(restored["data"], value(run));
    }
    let packages = modules(&archive);
    let candidates = multistage_bindings(&project);
    let identity: wire::ExecutionIdentity = typed(json!({"session_id":"multistage-component-session","execution_epoch_id":"multistage-component-epoch","project_id":project.id,"host_context_session_id":"multistage-component-host"}));
    let policy: wire::ExecutionPolicy = typed(json!({"clock_mode":"host_monotonic","report_cutoff_ms":240000,"stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish","target_quantity":{"value":{"coefficient":12,"decimal_places":0},"unit":"piece"},"source_prediction_window":scenario.window,"observation_max_age_ms":1000}));
    let input = EngineInput {run_id:run.id.clone(),input_ref:run.input_ref.clone(),project:project.clone(),scenario:scenario.clone(),scope:scenario.scope.clone(),definitions:packages.iter().map(|p|p.definition.clone()).collect()};
    let engine = Engine::new_external(input.clone(), identity.clone(), policy.clone(), candidates.clone()).unwrap();
    let prepared = engine.external_observation().unwrap();
    assert_eq!(prepared.queued_logical_jobs.len(),24);
    assert_eq!(prepared.items.len(),12);
    assert!(prepared.jobs.is_empty() && engine.snapshot().acceptance_events.is_empty());
    write(&output.join("component-input.json"), &json!({"run_id":input.run_id,"input_ref":input.input_ref,"project":project,"scenario":scenario,"definitions":input.definitions,"scope":input.scope,"identity":identity,"policy":policy,"candidate_bindings":candidates}));
    write(&output.join("receipt.json"), &json!({"passed":true,"pid":std::process::id(),"new_process_reopen":true,"all_saved_runs_exactly_restored":all.len(),"old_bytes_unchanged":true,"prepared_logical_jobs":24,"initial_items":12,"prepared_accepted":0,"actual_cloud_acceptance":null}));
    println!("New Host multi-operation reopen and Engine preparation passed");
}
