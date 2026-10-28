//! Actual external Engine/ExecutionProjector geometry evidence; finite receipt fixtures, no cloud.
use super::*;
use sf_commands::execution_projection::*;
use sf_simulation::ExternalReceiptStatus;
fn n(v:u64)->SafeUint{v.try_into().unwrap()}
struct Driver { engine:Engine, frozen:wire::ExecutionFrozenInputs, ledger:HostProjectionLedger, store:ExecutionProjector, revision:u64 }
impl Driver {
 fn observe(&mut self)->wire::ExecutionSnapshot {
  let observed=self.engine.external_observation().unwrap();self.ledger.execution_revision=n(self.revision);
  self.ledger.lifecycle_state=if !observed.clock_started{wire::ExecutionSnapshotLifecycleState::Prepared}else if observed.drained&&observed.goal_met{wire::ExecutionSnapshotLifecycleState::Completed}else{wire::ExecutionSnapshotLifecycleState::Running};
  self.ledger.new_dispatch_allowed=matches!(self.ledger.lifecycle_state,wire::ExecutionSnapshotLifecycleState::Running);
  let batch=self.store.append(ProjectionInput{observation:&observed,frozen:&self.frozen,ledger:&self.ledger,as_cutoff_snapshot:false,history_contiguous:true}).unwrap();self.revision+=1;batch.snapshot
 }
 fn receipt(&mut self,command:&wire::Command,at:u64,started:bool){
  let r:wire::DeviceReceipt=typed(json!({"session_id":command.identity.session_id,"execution_epoch_id":command.identity.execution_epoch_id,"command_id":command.attempt.command_id,
   "device_id":command.device_id,"phase_id":command.attempt.phase_id,"phase_token":command.attempt.phase_token,"device_boot_id":command.target_device_boot_id,
   "device_sequence":if started{1}else{2},"device_timestamp":"finite-component-receipt-fixture","receipt":if started{json!({"kind":"started"})}else{json!({"kind":"completed","device_reported_duration_ms":null})}}));
  let out=self.engine.apply_external_receipt(r.clone(),n(at)).unwrap();assert!(!out.duplicate&&out.status!=ExternalReceiptStatus::RetainedUnapplied);
  let seq=self.ledger.host_admission_sequence.value()+1;
  self.ledger.receipts.push(HostReceiptEvidence{receipt:r,admission:typed(json!({"receipt_id":format!("geometry-receipt-{seq}"),"decision":"applied","execution_revision":self.revision,
   "host_admission_sequence":seq,"host_received_elapsed_ms":at,"business_effect_committed":true,"reason":null})),committed_at_host_ms:Some(n(out.committed_at_ms))});
  self.ledger.host_admission_sequence=n(seq);self.observe();
 }
}
pub(super) fn produce(project:&ProjectState,scenario:&ScenarioInput,run:&RunResult,packages:&[ModulePackage],asset_reads:&Value,output:&Path){
 let candidates=bindings(project);
 let identity:wire::ExecutionIdentity=typed(json!({"session_id":format!("geometry-engine-session-{}",std::process::id()),"execution_epoch_id":format!("geometry-engine-epoch-{}",std::process::id()),
  "project_id":project.id,"host_context_session_id":format!("geometry-engine-host-{}",std::process::id())}));
 let policy:wire::ExecutionPolicy=typed(json!({"clock_mode":"host_monotonic","report_cutoff_ms":120000,"stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish",
  "target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},"source_prediction_window":{"start_ms":0,"end_ms":scenario.window.end_ms},"observation_max_age_ms":1000}));
 let definitions:Vec<_>=packages.iter().map(|p|p.definition.clone()).collect();
 let input=EngineInput{run_id:run.id.clone(),input_ref:run.input_ref.clone(),project:project.clone(),scenario:scenario.clone(),scope:scenario.scope.clone(),definitions:definitions.clone()};
 let frozen:wire::ExecutionFrozenInputs=typed(json!({"document_kind":"execution_frozen_inputs","contract_version":"sf.execution/2.0-draft11","identity":identity,
  "project_input_ref":run.input_ref,"project":project,"scenario":scenario,"modules":packages,"execution_definitions":definitions,"scope":scenario.scope,"candidate_bindings":candidates,
  "asset_bindings":asset_reads.as_array().unwrap().iter().map(|r|json!({"module_ref":r["module_ref"],"asset":r["asset"],"archive_file":{"path":r["asset"]["path"],"kind":"asset"}})).collect::<Vec<_>>()}));
 let devices=candidates.iter().map(|binding|typed(json!({"binding":binding,"operator_accept_new_work":true,"latest_observation":null,"freshness":"unknown","effective_accept_new_work":null,
  "blocking_reasons":["Finite receipt component fixture; live Host qualification is not asserted"],"registered_device_boot_id":"geometry-component-boot","pending_observation_probe":null}))).collect();
 let ledger=HostProjectionLedger{execution_revision:n(0),host_admission_sequence:n(0),policy:policy.clone(),provenance:typed(json!({"mode":"software_in_loop","cloud_connection_ref":"geometry-component-only",
  "device_binding_refs":candidates.iter().map(|b|b.binding_id.clone()).collect::<Vec<_>>(),"producer":"host"})),lifecycle_state:wire::ExecutionSnapshotLifecycleState::Prepared,lifecycle_reason:None,new_dispatch_allowed:false,
  terminal_disposition_recorded:false,attempts:vec![],receipts:vec![],observation_admissions:vec![],devices,queued_dispatch_decisions:Default::default(),missing_conditions:vec![]};
 let engine=Engine::new_external(input,identity.clone(),policy,candidates.clone()).unwrap();
 let mut d=Driver{engine,frozen,ledger,store:ExecutionProjector::new(),revision:0};let prepared=d.observe();
 assert!(!prepared.coverage.clock_started);assert_eq!(prepared.business_facts.items.len(),6);d.engine.activate_external().unwrap();d.observe();
 for k in 0..6 {
  let binding=&candidates[k%2];let at=k as u64*20000;let work=d.engine.external_observation().unwrap().queued_logical_jobs[0].clone();
  let attempt:wire::AttemptRef=typed(json!({"logical_job_id":work,"phase_id":"cycle","attempt_ordinal":1,"command_id":format!("geometry-command-{k}"),"selected_binding_id":binding.binding_id,"phase_token":format!("geometry-token-{k}")}));
  let command=d.engine.prepare_external(attempt,"geometry-component-boot".to_owned().try_into().unwrap(),n(at)).unwrap();
  d.ledger.attempts.push(AttemptMetadata{command_id:command.attempt.command_id.clone(),reservation_id:id(&format!("geometry-reservation-{k}")),reserved_at_host_ms:Some(n(at)),published_at_host_ms:None,withdrawn_unpublished_at_host_ms:None,
   reserved_resource_units:vec![typed(json!({"resource_id":binding.execution_capacity_ref.id,"units":1}))],pending_input_ordinals:None,
   dispatch_decision:Some(typed(json!({"based_on_host_admission_sequence":d.ledger.host_admission_sequence,"decision_host_ms":at,"reason":"earliest_estimated_completion","selected_binding_id":binding.binding_id,
    "candidates":[{"binding_id":binding.binding_id,"eligible":true,"blocking_reasons":[],"estimated_completion_host_ms":at+binding.nominal_work_ms.value(),"rank":1}]})))});
  d.observe();d.engine.mark_external_published(&command.attempt.command_id,n(at)).unwrap();d.ledger.attempts.last_mut().unwrap().published_at_host_ms=Some(n(at));d.observe();
  d.receipt(&command,at+1,true);d.receipt(&command,at+1+binding.nominal_work_ms.value(),false);
 }
 let final_snapshot=d.store.snapshot(&wire::SnapshotSelection::Latest{}).unwrap().clone();
 assert_eq!(final_snapshot.business_facts.items.len(),6);assert_eq!(final_snapshot.bound_jobs.len(),6);assert_eq!(final_snapshot.business_facts.acceptance_events.len(),6);
 assert_eq!(final_snapshot.business_facts.acceptance_events.iter().map(|a|a.quantity.value.coefficient.value()).sum::<u64>(),6);
 let mut pages=vec![];
 for (revision,at) in [(0,0),(final_snapshot.execution_revision.value(),0),(final_snapshot.execution_revision.value(),1),(final_snapshot.execution_revision.value(),20001),(final_snapshot.execution_revision.value(),115001)]{
  let q:wire::ExecutionObservationQuery=typed(json!({"contract_version":"sf.execution/2.0-draft11","identity":identity,"execution_revision":revision,"at_host_ms":at,"objects":[],"page":{"limit":200,"cursor":null}}));
  let page=d.store.query_observations(&q).unwrap();assert!(page.next_cursor.is_none());pages.push(page);
 }
 write(&output.join("execution-projection.json"),&json!({"inputs":d.frozen,"prepared":prepared,"snapshot":final_snapshot,"snapshots":d.store.snapshots(),"history":d.store.history_document().unwrap(),"observation_pages":pages,
  "actual_engine_and_projector":true,"receipt_transport":"explicit finite typed callback fixtures; no live cloud/device or wall-clock production claim","input_run_ref_is_actual_saved_prediction":run.id}));
}
