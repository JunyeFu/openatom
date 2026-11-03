//! Host-owned external execution. This owns the original Engine, never a second
//! stock. Native supplies authenticated messages and the one monotonic clock.
use crate::execution_api::*;
use crate::execution_projection::{
    AttemptMetadata, ExecutionProjector, HostProjectionLedger, HostReceiptEvidence, ProjectionInput,
};
use crate::{ApiError, ErrorCode};
use serde_json::Value;
use sf_domain::{cloud_execution::*, *};
use sf_simulation::{
    Engine, EngineInput, ExternalAttemptState, ExternalObservation, ExternalReceiptStatus,
};
use std::{collections::BTreeMap, time::Instant};

type Result<T> = std::result::Result<T, ApiError>;
fn uint(n: u64) -> SafeUint {
    SafeUint::try_from(n).expect("checked Host execution counter")
}
fn object(value: String) -> ObjectId {
    ObjectId::try_from(value).expect("Host-generated identity")
}
fn fault(code: ErrorCode, message: impl Into<String>) -> ApiError {
    ApiError {
        code,
        message: message.into(),
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    fn id(s: &str) -> ObjectId {
        object(s.to_owned())
    }
    fn at(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }
    fn stamp() -> CloudTimestamp {
        "2026-10-27T00:00:00.123456Z".to_owned().try_into().unwrap()
    }
    fn session_with(cutoff: u64, arrival: bool) -> ExecutionSession {
        // Same original compiler/Stock/acceptance fixture as the Engine batch,
        // with six units and real model parameters fixed to 10/15 seconds.
        let mut v: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/cloud_execution/six_units.json"
        ))
        .unwrap();
        if arrival {
            let item = v["scenario"]["initial_conditions"]["items"]
                .as_array_mut()
                .unwrap()
                .pop()
                .unwrap();
            v["scenario"]["arrivals"] = json!([{"id":"arriving-original-sixth-unit","at_ms":5,"destination":"L1-SOURCE","items":[item],"release_policy":"unchecked_allowed"}]);
        }
        let scenario: ScenarioInput = serde_json::from_value(v["scenario"].clone()).unwrap();
        let input_ref:RunInputRef=serde_json::from_value(json!({"project_id":v["project"]["id"],"project_revision":v["project"]["revision"],
            "scenario_id":scenario.id,"scenario_revision":scenario.revision,"frozen_project_file":{"path":"execution/project.json","kind":"project"},
            "frozen_scenario_file":{"path":"execution/scenario.json","kind":"scenario"}})).unwrap();
        let input = EngineInput {
            run_id: "private-execution-engine".to_owned().try_into().unwrap(),
            input_ref,
            project: serde_json::from_value(v["project"].clone()).unwrap(),
            scope: scenario.scope.clone(),
            scenario,
            definitions: serde_json::from_value(v["definitions"].clone()).unwrap(),
        };
        let identity: ExecutionIdentity = serde_json::from_value(
            json!({"session_id":"owner-session","execution_epoch_id":"owner-epoch",
            "project_id":v["project"]["id"],"host_context_session_id":"native-host"}),
        )
        .unwrap();
        let policy:ExecutionPolicy=serde_json::from_value(json!({"clock_mode":"host_monotonic","report_cutoff_ms":cutoff,
            "stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish","target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},
            "source_prediction_window":{"start_ms":0,"end_ms":1200000},"observation_max_age_ms":2000})).unwrap();
        let candidates:Vec<CandidateBinding>=[("L1-D01",10000),("L1-D02",15000)].into_iter().map(|(device,ms)|serde_json::from_value(json!({
            "binding_id":format!("binding-{device}"),"device_id":device,"device_module_ref":{"id":"p201.b60.station","version":"1.0.0"},
            "operation_ref":{"id":"p201.b60.station","version":"1.0.0"},"execution_capacity_ref":{"kind":"resource","id":format!("{device}-CAP")},
            "input_ports":[{"role_id":"workpiece","port_id":"input","location_id":"L1-SOURCE","item_type_id":"benchmark-item-v1","unit_id":"piece"}],
            "output_ports":[{"role_id":"workpiece","port_id":"output","location_id":"L1-SINK","item_type_id":"benchmark-item-v1","unit_id":"piece"}],
            "resources":[],"phase_id":"cycle","nominal_work_ms":ms})).unwrap()).collect();
        let frozen:ExecutionFrozenInputs=serde_json::from_value(json!({"document_kind":"execution_frozen_inputs",
            "contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"project_input_ref":input.input_ref,
            "project":input.project,"scenario":input.scenario,"modules":[],"execution_definitions":input.definitions,
            "scope":input.scope,"candidate_bindings":candidates,"asset_bindings":[]})).unwrap();
        ExecutionSession::prepare(
            frozen,
            policy,
            "execution-owner-session".to_owned().try_into().unwrap(),
            id("private-cloud-config"),
        )
        .unwrap()
    }
    pub(crate) fn session() -> ExecutionSession {
        session_with(120000, false)
    }

    #[test]
    fn restored_history_pages_match_original_filtered_records_and_validate_middle_frames() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 500));
        ready(&mut s, "L1-D02", 1, at(start, 1000));
        let history = s.projector.history_document().unwrap();
        let frames = s.projector.snapshots();
        let reader = ExecutionProjector::from_archive(history.clone(), frames.clone()).unwrap();
        let mut query: ExecutionHistoryQuery = serde_json::from_value(json!({
            "contract_version":"sf.execution/2.0-draft11", "identity":s.identity,
            "execution_revision":s.revision, "host_from_ms":500, "host_until_ms":1000,
            "objects":[], "page":{"limit":1,"cursor":null}
        })).unwrap();
        let expected: Vec<_> = history.records.iter().filter(|r| r.at_host_ms.value() >= 500).cloned().collect();
        assert!(expected.len() > 1);
        let mut actual = Vec::new();
        loop {
            let page = reader.query_history(&query).unwrap();
            assert!(page.records.len() <= 1);
            actual.extend(page.records);
            query.page.cursor = page.next_cursor;
            if query.page.cursor.is_none() { break; }
        }
        assert!(equal(&actual, &expected));
        // Incremental reconstruction must still inspect an intermediate frame,
        // even when the latest frame and every record remain unchanged.
        let mut changed = frames;
        let middle = changed.len() / 2;
        let mut altered = changed[middle].decode().unwrap();
        altered.devices[0].operator_accept_new_work = !altered.devices[0].operator_accept_new_work;
        changed[middle] = std::sync::Arc::new(crate::execution_projection::StoredExecutionSnapshot::capture(&altered).unwrap());
        assert!(ExecutionProjector::from_archive(history.clone(), changed).is_err());
        let mut broken = history;
        let middle = broken.records.len() / 2;
        broken.records[middle].record_sequence = 1_u64.try_into().unwrap();
        assert!(ExecutionProjector::from_archive(broken, s.projector.snapshots()).is_err());
    }

    #[test]
    fn selected_binding_views_preserve_the_full_view_filter() {
        let s = session();
        let observation = s.observe().unwrap();
        let full = s.binding_views(0, &observation);
        let selected = vec![full[0].binding.binding_id.clone()];
        let expected: Vec<_> = full.into_iter().filter(|view| selected.contains(&view.binding.binding_id)).collect();
        assert!(equal(&expected, &s.selected_binding_views(0, &observation, Some(&selected))));
        assert!(s.selected_binding_views(0, &observation, Some(&[])).is_empty());
    }
    #[test]
    fn coalesced_unsent_publications_keep_records_history_and_cutoff_boundary() {
        let mut s = session();
        let now = Instant::now();
        run(&mut s, now);
        s.tick(at(now, 10)).unwrap();
        let original = s.cloud_work().items;
        assert!(original.len() >= 3);
        assert!(original.iter().all(|w| matches!(w, CloudWorkItem::PublishSnapshot { .. })));
        let frames = s.projector.snapshots();
        // A cutoff must be published exactly, even with later frames queued.
        if let CloudWorkItem::PublishSnapshot { publication, .. } = &mut s.pending_cloud_work[1] {
            publication.as_cutoff_snapshot = true;
        }
        let records: Vec<_> = original[..2].iter().flat_map(|w| match w {
            CloudWorkItem::PublishSnapshot { publication, .. } => publication.observation_records.clone(),
            _ => unreachable!(),
        }).collect();
        s.coalesce_pending_publications();
        assert_eq!(s.pending_cloud_work.len(), original.len() - 1);
        let CloudWorkItem::PublishSnapshot { work_id, publication } = &s.pending_cloud_work[0] else { panic!() };
        let CloudWorkItem::PublishSnapshot { publication: first, .. } = &original[0] else { panic!() };
        let CloudWorkItem::PublishSnapshot { publication: second, .. } = &original[1] else { panic!() };
        assert_eq!(work_id, original[0].work_id());
        assert_eq!(publication.expected_published_revision, first.expected_published_revision);
        assert!(equal(&publication.snapshot, &second.snapshot));
        assert!(equal(&publication.observation_records, &records));
        assert!(publication.as_cutoff_snapshot);
        let leased = s.cloud_work();
        s.coalesce_pending_publications();
        assert!(equal(&leased, &s.cloud_work()));
        assert!(equal(&frames, &s.projector.snapshots()));
    }
    #[test]
    fn coalescing_does_not_cross_command_work() {
        let mut s = session();
        let now = Instant::now();
        run(&mut s, now);
        ready(&mut s, "L1-D01", 1, at(now, 1));
        let before = s.cloud_work().items;
        let boundary = before.iter().position(|w| matches!(w, CloudWorkItem::PublishCommand { .. })).unwrap();
        assert!(boundary > 0);
        let tail = before[boundary..].to_vec();
        s.coalesce_pending_publications();
        assert!(equal(&tail, &&s.pending_cloud_work[1..]));
        s.pending_cloud_work.remove(0);
        s.coalesce_pending_publications();
        assert!(equal(&tail, &s.pending_cloud_work));
    }
    #[test]
    fn confirmed_publications_release_duplicate_frames_but_keep_query_history() {
        let mut s = session();
        let now = Instant::now();
        run(&mut s, now);
        let frames = s.projector.snapshots();
        assert!(frames.len() > 1);
        for work in s.cloud_work().items {
            if let CloudWorkItem::PublishSnapshot { work_id, publication } = work {
                let ack = CloudEffectAck::SnapshotPublished { work_id, ack: PublishAck {
                    session_id: publication.snapshot.identity.session_id,
                    execution_epoch_id: publication.snapshot.identity.execution_epoch_id,
                    execution_revision: publication.snapshot.execution_revision,
                    publication_status: PublishAckPublicationStatus::Stored,
                    observation_frontier: publication.snapshot.observation_frontier,
                }};
                s.acknowledge_cloud_work(ack.clone(), now).unwrap();
                s.acknowledge_cloud_work(ack, now).unwrap();
            }
        }
        assert_eq!(s.publications.len(), 1);
        assert!(s.publications.contains_key(&s.revision));
        assert!(equal(&frames, &s.projector.snapshots()));
        let captured = s.projector.snapshots();
        assert!(std::sync::Arc::ptr_eq(&frames[0], &captured[0]));
        let plain: Vec<ExecutionSnapshot> = serde_json::from_value(serde_json::to_value(&captured).unwrap()).unwrap();
        assert!(equal(&plain, &captured));
    }

    fn dispatch_order_session(priorities: [i64; 3], releases: [u64; 3], dues: Option<[u64; 3]>) -> ExecutionSession {
        let seed = session();
        let mut frozen = serde_json::to_value(&seed.frozen).unwrap();
        let original = frozen["scenario"]["production_tasks"][0].clone();
        frozen["scenario"]["production_tasks"] = Value::Array((0..3).map(|i| {
            let mut task = original.clone();
            task["id"] = json!(["A", "B", "C"][i]);
            task["planned_quantity"] = json!({"value":{"coefficient":2,"decimal_places":0},"unit":"piece"});
            task["priority"] = json!(priorities[i]);
            task["release_at_ms"] = json!(releases[i]);
            task
        }).collect());
        if let Some(due) = dues {
            frozen["scenario"]["acceptance_goal"] = Value::Null;
            frozen["scenario"]["orders"] = Value::Array((0..3).map(|i| json!({
                "id":format!("order-{i}"),"product_ref":{"id":"L1-PRODUCT","revision":"r1"},
                "quantity":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"},
                "release_at_ms":0,"due_at_ms":due[i],"destination":"L1-SINK",
                "acceptance_stage":"factory_released","allow_partial":true,"priority":0
            })).collect());
            for i in 0..3 {
                frozen["scenario"]["production_tasks"][i]["order_allocations"] = json!([
                    {"order_id":format!("order-{i}"),"quantity":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"}}
                ]);
            }
        }
        ExecutionSession::prepare(serde_json::from_value(frozen).unwrap(), seed.policy,
            seed.dataset_id.unwrap(), seed.provenance.cloud_connection_ref).unwrap()
    }
    fn dispatched_task(s: &ExecutionSession, command: &Command) -> String {
        s.observe().unwrap().logical_work.iter()
            .find(|w| w.logical_job_id == command.attempt.logical_job_id).unwrap()
            .production_task_id.as_str().to_owned()
    }
    #[test]
    fn dispatch_order_host_prioritizes_released_work_after_device_wait() {
        let mut s = dispatch_order_session([0, 5, 2], [0, 10, 5], None);
        let start = Instant::now(); run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 20));
        let c = commands(&s); assert_eq!(c.len(), 1);
        assert_eq!(dispatched_task(&s, &c[0]), "B");
        assert_eq!(c[0].device_id, id("L1-D01"));
        assert_eq!(s.observe().unwrap().items.len(), 6);
        assert!(s.observe().unwrap().acceptance_events.is_empty());
    }
    #[test]
    fn dispatch_order_host_uses_due_date_after_device_wait() {
        let mut s = dispatch_order_session([0, 0, 0], [0, 10, 5], Some([9000, 4000, 7000]));
        let start = Instant::now(); run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 20));
        let c = commands(&s); assert_eq!(c.len(), 1);
        assert_eq!(dispatched_task(&s, &c[0]), "B");
        assert!(s.observe().unwrap().acceptance_events.is_empty());
    }
    #[test]
    fn dispatch_order_host_skips_future_work_then_dispatches_it_when_released() {
        let mut s = dispatch_order_session([0, 5, 2], [0, 50, 5], None);
        let start = Instant::now(); run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 20));
        let first = commands(&s); assert_eq!(first.len(), 1);
        assert_eq!(dispatched_task(&s, &first[0]), "C");
        ready(&mut s, "L1-D02", 1, at(start, 60));
        let all = commands(&s); assert_eq!(all.len(), 2);
        let second = all.iter().find(|c| c.device_id == id("L1-D02")).unwrap();
        assert_eq!(dispatched_task(&s, second), "B");
        assert_eq!(s.observe().unwrap().items.len(), 6);
        assert!(s.observe().unwrap().acceptance_events.is_empty());
    }
    fn run(s: &mut ExecutionSession, start: Instant) {
        for dev in ["L1-D01", "L1-D02"] {
            s.register_boot(&id(dev), format!("boot-{dev}").try_into().unwrap(), start)
                .unwrap();
        }
        s.control(
            id("operator"),
            id("run"),
            s.revision,
            ControlAction::Run {},
            start,
        )
        .unwrap();
    }
    fn observation(s: &ExecutionSession, dev: &str, seq: u64) -> ExecutionTelemetryObservation {
        let slot = &s.devices[&id(dev)];
        serde_json::from_value(json!({"contract_version":EXECUTION_CONTRACT_VERSION,"session_id":s.identity.session_id,
            "execution_epoch_id":s.identity.execution_epoch_id,"device_id":dev,"device_boot_id":slot.boot,
            "device_sequence":seq,"device_timestamp":"device-private-clock","accept_new_work":true,"device_state":"idle","active_command_id":null,
            "dataset_id":s.dataset_id,"source_purpose":"execution","observation_probe_id":slot.probe.as_ref().unwrap().probe_id})).unwrap()
    }
    fn ready(s: &mut ExecutionSession, dev: &str, seq: u64, now: Instant) {
        s.tick(now).unwrap();
        let o = observation(s, dev, seq);
        let a = s
            .apply_observation(id(&format!("observation-{dev}-{seq}")), stamp(), o, now)
            .unwrap();
        assert!(matches!(a.decision, ReceiptAdmissionDecision::Applied));
    }
    fn receipt(command: &Command, seq: u64, kind: Value) -> DeviceReceipt {
        serde_json::from_value(json!({"session_id":command.identity.session_id,"execution_epoch_id":command.identity.execution_epoch_id,
            "command_id":command.attempt.command_id,"device_id":command.device_id,"phase_id":command.attempt.phase_id,
            "phase_token":command.attempt.phase_token,"device_boot_id":command.target_device_boot_id,"device_sequence":seq,
            "device_timestamp":"independent-device-time","receipt":kind})).unwrap()
    }
    fn commands(s: &ExecutionSession) -> Vec<Command> {
        s.cloud_work()
            .items
            .into_iter()
            .filter_map(|w| match w {
                CloudWorkItem::PublishCommand { command, .. } => Some(command),
                _ => None,
            })
            .collect()
    }

    fn semantic_session(v: Value) -> ExecutionSession {
        let seed = session();
        let mut frozen = seed.frozen;
        frozen.project = serde_json::from_value(v["project"].clone()).unwrap();
        frozen.scenario = serde_json::from_value(v["scenario"].clone()).unwrap();
        frozen.execution_definitions = serde_json::from_value(v["definitions"].clone()).unwrap();
        frozen.modules = if let Some(packages) = v.get("modules") {
            serde_json::from_value(packages.clone()).unwrap()
        } else {
            frozen.execution_definitions.iter().map(|definition| ModulePackage {
                definition: definition.clone(), assets: vec![], files: vec![], dependencies: vec![], rights: vec![], issues: vec![],
            }).collect()
        };
        frozen.identity.project_id = frozen.project.id.clone();
        frozen.project_input_ref.project_id = frozen.project.id.clone();
        frozen.project_input_ref.project_revision = frozen.project.revision.clone();
        frozen.project_input_ref.scenario_id = frozen.scenario.id.clone();
        frozen.project_input_ref.scenario_revision = frozen.scenario.revision.clone();
        frozen.scope = frozen.scenario.scope.clone();
        let input = EngineInput { run_id: "semantic-host".to_owned().try_into().unwrap(),
            input_ref: frozen.project_input_ref.clone(), project: frozen.project.clone(), scenario: frozen.scenario.clone(),
            definitions: frozen.execution_definitions.clone(), scope: frozen.scope.clone() };
        frozen.candidate_bindings = Engine::required_external_bindings(&input).unwrap();
        let mut policy = seed.policy;
        policy.source_prediction_window = serde_json::from_value(serde_json::to_value(&input.scenario.window).unwrap()).unwrap();
        policy.report_cutoff_ms = input.scenario.window.end_ms.0.clone();
        policy.target_quantity = input.scenario.acceptance_goal.as_ref().and_then(|g| g.accepted_quantity_per_day.clone())
            .or_else(|| input.scenario.orders[0].quantity.clone()).unwrap();
        if !input.scenario.orders.is_empty() {
            let quantities: Vec<_> = input.scenario.orders.iter().filter_map(|o| o.quantity.as_ref()).collect();
            assert!(quantities.iter().all(|q| q.unit == policy.target_quantity.unit && q.value.decimal_places.value() == 0));
            policy.target_quantity.value.coefficient = uint(quantities.iter().map(|q| q.value.coefficient.value()).sum());
        }
        ExecutionSession::prepare(frozen, policy, seed.dataset_id.unwrap(), seed.provenance.cloud_connection_ref).unwrap()
    }

    fn drive_semantic_session(s: &mut ExecutionSession) -> ExecutionSnapshot {
        let snapshot = drive_semantic_prefix(s, usize::MAX);
        assert_semantic_report(s, &snapshot);
        snapshot
    }
    fn assert_semantic_report(s: &ExecutionSession, snapshot: &ExecutionSnapshot) {
        let source = serde_json::from_value(json!({"read_origin":"host","identity":snapshot.identity,
            "execution_revision":snapshot.execution_revision,"observation_frontier":snapshot.observation_frontier,
            "delivery":{"read_origin":"host","local_execution_revision":snapshot.execution_revision,
                "cloud_published_revision":null,"relation":"publication_pending"}})).unwrap();
        let report = sf_reporting::build_execution_report(sf_reporting::ExecutionReportInput {
            report_id: id("semantic-report"), project_id: s.frozen.project.id.clone(),
            export_project_revision: s.frozen.project.revision.0.clone(), generated_at_utc: stamp(), prediction: None,
            sessions: vec![sf_reporting::ExecutionReportInputSession { source, inputs: s.frozen.clone(),
                snapshot: snapshot.clone(), history: s.projector.history_document().unwrap() }],
        }).unwrap();
        assert_eq!(serde_json::to_value(&report.sessions[0].snapshot.business_facts).unwrap(),
            serde_json::to_value(&snapshot.business_facts).unwrap());
    }
    fn drive_semantic_prefix(s: &mut ExecutionSession, limit: usize) -> ExecutionSnapshot {
        let start = Instant::now();
        let devices: Vec<_> = s.devices.keys().cloned().collect();
        for dev in &devices {
            s.register_boot(dev, format!("boot-{}", dev.as_str()).try_into().unwrap(), start).unwrap();
        }
        s.control(id("operator"), id("run-semantics"), s.revision, ControlAction::Run {}, start).unwrap();
        let mut sequences: BTreeMap<_, u64> = devices.iter().map(|dev| (dev.clone(), 1)).collect();
        for dev in &devices { ready(s, dev.as_str(), 1, start); }
        let mut handled = std::collections::BTreeSet::new();
        let mut now = 0;
        let mut idle = 0;
        for _ in 0..100 {
            now += 1;
            s.tick(at(start, now)).unwrap();
            let next: Vec<_> = commands(s).into_iter().filter(|c| !handled.contains(&c.attempt.command_id)).collect();
            if next.is_empty() {
                idle += 1;
                if idle >= 3 { return s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap().into_owned(); }
                continue;
            }
            idle = 0;
            for command in next {
                handled.insert(command.attempt.command_id.clone());
                for kind in [json!({"kind":"started"}), json!({"kind":"completed","device_reported_duration_ms":null})] {
                    now += 1;
                    let seq = sequences.get_mut(&command.device_id).unwrap(); *seq += 1;
                    let receipt = receipt(&command, *seq, kind);
                    let admission = s.apply_receipt(id(&format!("semantic-{}-{seq}", command.device_id.as_str())), stamp(), receipt, at(start, now)).unwrap();
                    assert!(matches!(admission.decision, ReceiptAdmissionDecision::Applied), "{admission:?}");
                }
                if handled.len() >= limit { return s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap().into_owned(); }
            }
        }
        panic!("semantic Host did not settle");
    }

    #[test]
    #[ignore = "Requires SF_FACTORY_FROZEN_DIR pointing to the saved actual factory evidence"]
    fn actual_frozen_factory_host_admits_and_advances_twenty_commands() {
        let root = std::path::PathBuf::from(std::env::var("SF_FACTORY_FROZEN_DIR").unwrap());
        let read = |name: &str| -> Value { serde_json::from_slice(&std::fs::read(root.join(name)).unwrap()).unwrap() };
        let packages = read("actual-frozen-modules.json");
        let v = json!({"project":read("actual-frozen-project.json"),"scenario":read("actual-scenario.json"), "modules":packages,
            "definitions":packages.as_array().unwrap().iter().map(|p| p["definition"].clone()).collect::<Vec<_>>()});
        let mut s = semantic_session(v);
        assert_eq!(s.frozen.candidate_bindings.len(), 456);
        assert_eq!(s.devices.len(), 93);
        let snapshot = drive_semantic_prefix(&mut s, 20);
        assert_eq!(snapshot.attempts.iter().filter(|a| a.completed_at_host_ms.is_some()).count(), 20);
        assert!(!snapshot.business_facts.transport.is_empty());
        assert!(snapshot.coverage.clock_started);
        assert_semantic_report(&s, &snapshot);
    }

    #[test]
    fn actual_quality_setup_and_transport_reach_host_projection() {
        for mode in [None, Some("internal"), Some("external")] {
            let mut v: Value = serde_json::from_str(include_str!("../tests/fixtures/quality/quality-original.json")).unwrap();
            v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!([]);
            for preset in v["scenario"]["quality_presets"].as_array_mut().unwrap() {
                preset["results"] = json!([{"kind":"string","value":"PASS"}]);
            }
            v["scenario"]["quality_presets"][1]["results"] = json!([{"kind":"string","value":"FAIL"},{"kind":"string","value":"PASS"}]);
            if let Some(mode) = mode {
                let op = &mut v["definitions"].as_array_mut().unwrap().iter_mut().find(|d| d["id"] == "w5.op.assembly").unwrap()["payload"]["data"];
                op["family"] = json!("new");
                let resources = op["resource_roles"].clone();
                v["project"]["setup_policies"] = json!([{"device_id":"Assembly01","initial_family":"old","changeovers":[{
                    "from_family":"old","to_family":"new","duration":{"value":500,"unit":"millisecond","basis":"user_provided","source_ref":null},
                    "resources":resources,"preparation_mode":mode}],"first_article":null}]);
            }
            let mut s = semantic_session(v);
            let snapshot = drive_semantic_session(&mut s);
            assert!(matches!(snapshot.goal_at_snapshot, ExecutionSnapshotGoalAtSnapshot::Satisfied), "{mode:?}");
            assert!(snapshot.business_facts.items.iter().any(|i| i.completed_reworks.value() == 1));
        }
        for (source, satisfied) in [(include_str!("../tests/fixtures/production/p202-halls-A.json"), false),
            (include_str!("../tests/fixtures/production/p202-halls-B.json"), true)] {
            let mut s = semantic_session(serde_json::from_str(source).unwrap());
            let snapshot = drive_semantic_session(&mut s);
            assert_eq!(matches!(snapshot.goal_at_snapshot, ExecutionSnapshotGoalAtSnapshot::Satisfied), satisfied);
            assert!(!snapshot.business_facts.transport.is_empty());
            assert!(snapshot.business_facts.transport.iter().all(|trip| matches!(trip.phase, TransportPhase::Returned)));
        }
    }


    #[test]
    fn stopping_an_arrived_transport_names_the_full_destination() {
        let mut input: Value = serde_json::from_str(include_str!("../tests/fixtures/production/p202-halls-B.json")).unwrap();
        let sink = input["scenario"]["supplies"][0]["destination"].clone();
        input["project"]["locations"].as_array_mut().unwrap().iter_mut()
            .find(|l| l["id"] == sink).unwrap()["capacity"]["value"]["coefficient"] = json!(0);
        let mut session = semantic_session(input);
        let before = drive_semantic_prefix(&mut session, usize::MAX);
        let trip = before.business_facts.transport.iter()
            .find(|t| t.arrived_ms.is_some() && t.received_ms.is_none()).unwrap();
        let now = session.clock_anchor.unwrap() + std::time::Duration::from_millis(before.coverage.observed_until_host_ms.value()+1);
        session.control(id("operator"), id("stop-arrived"), session.revision, ControlAction::StopDispatch {}, now).unwrap();
        let after = session.projector.snapshot(&SnapshotSelection::Latest {}).unwrap();
        assert!(matches!(after.lifecycle.state, ExecutionSnapshotLifecycleState::Draining));
        assert!(!after.lifecycle.new_dispatch_allowed);
        let reason = after.lifecycle.reason.as_ref().unwrap().as_str();
        assert!(reason.contains(trip.id.as_str()) && reason.contains(sink.as_str().unwrap()));
        assert_eq!(before.attempts.len(), after.attempts.len());
        assert!(equal(&before.business_facts.transport, &after.business_facts.transport));
        assert!(equal(&before.business_facts.acceptance_events, &after.business_facts.acceptance_events));
    }

    fn multiphase_session(cutoff: u64) -> ExecutionSession {
        let seed = session_with(cutoff, false);
        let mut frozen = serde_json::to_value(&seed.frozen).unwrap();
        let phases = &mut frozen["execution_definitions"][0]["payload"]["data"]["behavior"]["inline"]["phases"];
        let original = phases[0].clone();
        *phases = Value::Array(["load", "auto", "unload"].iter().map(|name| {
            let mut phase = original.clone(); phase["id"] = json!(name); phase
        }).collect());
        let original = frozen["candidate_bindings"].as_array().unwrap().clone();
        frozen["candidate_bindings"] = Value::Array(original.iter().flat_map(|binding|
            ["load", "auto", "unload"].iter().map(move |phase| {
                let mut b = binding.clone();
                b["binding_id"] = json!(format!("{}-{phase}", binding["device_id"].as_str().unwrap()));
                b["phase_id"] = json!(phase);
                if *phase != "load" { b["nominal_work_ms"] = json!(2000); } b
            })).collect());
        ExecutionSession::prepare(serde_json::from_value(frozen).unwrap(), seed.policy,
            seed.dataset_id.unwrap(), seed.provenance.cloud_connection_ref).unwrap()
    }
    #[test]
    fn preparation_command_finishes_readiness_before_production() {
        let seed=session();let mut frozen=serde_json::to_value(&seed.frozen).unwrap();
        let reference=json!({"id":"test.prepare","version":"1.0.0"});
        let mut definition=frozen["execution_definitions"][0].clone();definition["id"]=reference["id"].clone();
        definition["payload"]=json!({"category":"tasks","data":{"kind":"supported_automatic","target_role":"device","prerequisites":[],"steps":[],"resource_roles":[],"duration":{"value":500,"unit":"millisecond","basis":"user_provided","source_ref":"explicit-test"},"equipment_access":"exclusive","completion_effect":{"target_role":"device","marker":"ready","produces_material":false}}});
        frozen["execution_definitions"].as_array_mut().unwrap().push(definition);frozen["project"]["module_refs"].as_array_mut().unwrap().push(reference.clone());
        frozen["project"]["routes"][0]["steps"][0]["predecessors"]=json!(["prepare"]);
        frozen["project"]["routes"][0]["steps"].as_array_mut().unwrap().push(json!({"id":"prepare","operation_ref":reference,"device_id":"L1-D01","predecessors":[],"item_bindings":{},"resource_bindings":{}}));
        let original=frozen["candidate_bindings"].as_array().unwrap().clone();
        for (i,binding) in original.iter().enumerate() {
            let device=&mut frozen["project"]["devices"][i];device["role_bindings"]["targets"]["device"]=json!({"kind":"device","id":device["id"]});device["route_operation_refs"].as_array_mut().unwrap().push(reference.clone());
            let mut b=binding.clone();b["binding_id"]=json!(format!("{}-prepare",binding["device_id"].as_str().unwrap()));b["operation_ref"]=reference.clone();b["input_ports"]=json!([]);b["output_ports"]=json!([]);b["nominal_work_ms"]=json!(500);frozen["candidate_bindings"].as_array_mut().unwrap().push(b);
        }
        let mut s=ExecutionSession::prepare(serde_json::from_value(frozen).unwrap(),seed.policy,seed.dataset_id.unwrap(),seed.provenance.cloud_connection_ref).unwrap();
        let start=Instant::now();run(&mut s,start);ready(&mut s,"L1-D01",1,at(start,1));
        let command=commands(&s)[0].clone();assert!(matches!(command.action,sf_domain::cloud_execution::DeviceAction::PrepareEquipment));
        s.apply_receipt(id("prep-start"),stamp(),receipt(&command,2,json!({"kind":"started"})),at(start,2)).unwrap();
        s.tick(at(start,600)).unwrap();assert_eq!(commands(&s).len(),1);
        s.apply_receipt(id("prep-done"),stamp(),receipt(&command,3,json!({"kind":"completed","device_reported_duration_ms":598})),at(start,601)).unwrap();
        assert!(s.observe().unwrap().acceptance_events.is_empty());assert_eq!(s.observe().unwrap().items.len(),6);
        assert!(commands(&s).iter().any(|c| matches!(c.action,sf_domain::cloud_execution::DeviceAction::ProcessUnit)));
    }
    #[test]
    fn multi_piece_command_commits_declared_output_quantity_once() {
        let seed=session();let mut frozen=serde_json::to_value(&seed.frozen).unwrap();
        let op=&mut frozen["execution_definitions"][0]["payload"]["data"]["behavior"]["inline"];
        op["identity_policy"]=json!("transform_with_origin");op["outputs"][0]["quantity"]["value"]["coefficient"]=json!(2);
        let mut policy=seed.policy;policy.target_quantity.value.coefficient=uint(12);
        let mut s=ExecutionSession::prepare(serde_json::from_value(frozen).unwrap(),policy,
            seed.dataset_id.unwrap(),seed.provenance.cloud_connection_ref).unwrap();
        let start=Instant::now();run(&mut s,start);ready(&mut s,"L1-D01",1,at(start,1));
        let command=commands(&s)[0].clone();assert_eq!(command.parameters.quantity.value.coefficient.value(),2);
        s.apply_receipt(id("yield-start"),stamp(),receipt(&command,2,json!({"kind":"started"})),at(start,2)).unwrap();
        assert!(s.observe().unwrap().acceptance_events.is_empty());
        let done=receipt(&command,3,json!({"kind":"completed","device_reported_duration_ms":1}));
        s.apply_receipt(id("yield-done"),stamp(),done.clone(),at(start,3)).unwrap();
        assert!(s.apply_receipt(id("yield-done-retry"),stamp(),done.clone(),at(start,4)).is_err());
        s.apply_receipt(id("yield-done"),stamp(),done,at(start,4)).unwrap();
        assert_eq!(s.observe().unwrap().acceptance_events.iter().map(|a|a.quantity.value.coefficient.value()).sum::<u64>(),2);
    }
    #[test]
    fn phase_material_flow_uses_real_host_receipts_and_delayed_item_transfer() {
        let seed = multiphase_session(10000);
        let mut frozen = serde_json::to_value(&seed.frozen).unwrap();
        frozen["execution_definitions"][0]["payload"]["data"]["behavior"]["inline"]["material_flow"] = json!({
            "primary_role":"workpiece","auxiliary_roles":[],"acquire_after_phase":"load",
            "consume_after_phase":"auto","reserve_before_phase":"unload","deliver_after_phase":"unload",
            "required_primary_attributes":{},"attributes_after_consume":{}});
        let mut s = ExecutionSession::prepare(serde_json::from_value(frozen).unwrap(),seed.policy,
            seed.dataset_id.unwrap(),seed.provenance.cloud_connection_ref).unwrap();
        let start=Instant::now();run(&mut s,start);ready(&mut s,"L1-D01",1,at(start,1));
        let job=commands(&s)[0].attempt.logical_job_id.clone();
        for (n,phase) in ["load","auto","unload"].iter().enumerate() {
            let command=commands(&s).into_iter().find(|c|c.attempt.logical_job_id==job&&c.attempt.phase_id.as_str()==*phase).unwrap();
            let t=2+n as u64*10;
            s.apply_receipt(id(&format!("flow-start-{n}")),stamp(),receipt(&command,2+n as u64*2,json!({"kind":"started"})),at(start,t)).unwrap();
            if n==0 {
                let observed=s.observe().unwrap();
                let input=&observed.jobs.iter().find(|j|j.id==job).unwrap().input_refs[0];
                assert!(matches!(observed.items.iter().find(|i|&i.id==input).unwrap().holder,HolderRef::Location(_)));
            }
            s.apply_receipt(id(&format!("flow-done-{n}")),stamp(),receipt(&command,3+n as u64*2,json!({"kind":"completed","device_reported_duration_ms":1})),at(start,t+1)).unwrap();
            assert_eq!(s.observe().unwrap().acceptance_events.len(),if n==2 {1}else{0});
        }
    }
    #[test]
    fn stopping_at_full_sink_exposes_blocked_job_and_location() {
        let seed = session();
        let mut frozen = serde_json::to_value(&seed.frozen).unwrap();
        let sink = frozen["project"]["locations"].as_array_mut().unwrap().iter_mut()
            .find(|location| location["id"] == "L1-SINK").unwrap();
        sink["capacity"]["value"]["coefficient"] = json!(1);
        let items = frozen["scenario"]["initial_conditions"]["items"].as_array_mut().unwrap();
        let mut blocker = items[0].clone();
        blocker["id"] = json!("occupied-sink");
        blocker["holder"]["id"] = json!("L1-SINK");
        items.push(blocker);
        let mut s = ExecutionSession::prepare(serde_json::from_value(frozen).unwrap(),
            seed.policy, seed.dataset_id.unwrap(), seed.provenance.cloud_connection_ref).unwrap();
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let cmd = commands(&s)[0].clone();
        s.apply_receipt(id("start"), stamp(), receipt(&cmd, 2, json!({"kind":"started"})), at(start, 2)).unwrap();
        s.control(id("operator"), id("stop"), s.revision, ControlAction::StopDispatch {}, at(start, 3)).unwrap();
        let before = s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap();
        assert!(before.lifecycle.reason.is_none());
        s.apply_receipt(id("done"), stamp(), receipt(&cmd, 3, json!({"kind":"completed","device_reported_duration_ms":10})), at(start, 12)).unwrap();
        let after = s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap();
        assert!(matches!(after.lifecycle.state, ExecutionSnapshotLifecycleState::Draining));
        let reason = after.lifecycle.reason.as_ref().unwrap().as_str();
        assert!(reason.contains(cmd.attempt.logical_job_id.as_str()) && reason.contains("L1-SINK"));
        assert!(!after.lifecycle.new_dispatch_allowed);
        assert_eq!(after.business_facts.pending_output_refs.len(), 1);
        assert!(after.business_facts.acceptance_events.is_empty());
        assert_eq!(after.attempts.len(), 1);
    }

    #[test]
    fn shared_device_bindings_dispatch_all_phases_and_drain_started_work() {
        let mut s = multiphase_session(20);
        assert_eq!(s.devices.len(), 2);
        assert_eq!(s.device_views(0).len(), 2);
        assert_eq!(s.binding_views(0, &s.observe().unwrap()).len(), 6);
        assert_eq!(s.devices[&id("L1-D01")].bindings.len(), 3);
        let start = Instant::now(); run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let first = commands(&s)[0].clone();
        assert_eq!(first.attempt.phase_id.as_str(), "load");
        for (n, phase) in ["load", "auto", "unload"].iter().enumerate() {
            let command = commands(&s).into_iter().find(|c| c.attempt.phase_id.as_str() == *phase).unwrap();
            assert_eq!(command.device_id, first.device_id);
            assert_eq!(command.attempt.logical_job_id, first.attempt.logical_job_id);
            assert_eq!(command.target_device_boot_id, first.target_device_boot_id);
            let t = if n == 0 { 2 } else { 30 + n as u64 * 10 };
            s.apply_receipt(id(&format!("start-{n}")), stamp(), receipt(&command, 2+n as u64*2,
                json!({"kind":"started"})), at(start,t)).unwrap();
            s.apply_receipt(id(&format!("done-{n}")), stamp(), receipt(&command, 3+n as u64*2,
                json!({"kind":"completed","device_reported_duration_ms":1})), at(start,t+1)).unwrap();
            let observed = s.observe().unwrap();
            assert_eq!(observed.items.len(), 6);
            assert_eq!(observed.phase_work_intervals.len(), n+1);
            assert_eq!(observed.acceptance_events.len(), if n==2 {1} else {0});
            let snapshot = s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap();
            assert_eq!(snapshot.bound_jobs[0].started_at_host_ms.value(), 2);
            let attempt = snapshot.attempts.iter()
                .find(|a| a.attempt.command_id == command.attempt.command_id).unwrap();
            assert_eq!(attempt.started_at_host_ms.as_ref().unwrap().value(), t);
            if n < 2 {
                let next = snapshot.attempts.iter()
                    .max_by_key(|a| a.attempt.attempt_ordinal.value()).unwrap();
                assert!(next.started_at_host_ms.is_none());
            }
            if n==0 {
                let auto = commands(&s).into_iter().find(|c| c.attempt.phase_id.as_str()=="auto").unwrap();
                let mut wrong = observation(&s,"L1-D01",20);
                wrong.device_state=ExecutionTelemetryObservationDeviceState::Busy;
                wrong.active_command_id=Some(auto.attempt.command_id.clone());
                wrong.phase_progress=Some(serde_json::from_value(json!({"phase_id":"load","effective_work_ms":1,"duration_ms":10})).unwrap());
                assert!(s.apply_observation(id("wrong-command-phase"),stamp(),wrong,at(start,4)).is_err());
                s.tick(at(start,21)).unwrap();
                assert!(matches!(s.phase,ExecutionSnapshotLifecycleState::Draining));
                assert!(!s.cloud_work().items.iter().any(|work| matches!(work,CloudWorkItem::WithdrawCommand {command_id,..} if command_id==&auto.attempt.command_id)));
            }
        }
        assert_eq!(s.attempts.len(),3);
        assert!(s.observe().unwrap().drained);
        assert!(matches!(s.phase,ExecutionSnapshotLifecycleState::Stopped));
    }
    #[test]
    fn actual_device_pause_retains_host_job_and_resumes_same_phase() {
        let seed = multiphase_session(120000);
        let mut frozen = serde_json::to_value(&seed.frozen).unwrap();
        frozen["execution_definitions"][0]["payload"]["data"]["behavior"]["inline"]["phases"][0]["interruption_policy"] = json!("pause_resume");
        let mut s = ExecutionSession::prepare(serde_json::from_value(frozen).unwrap(), seed.policy, seed.dataset_id.unwrap(), seed.provenance.cloud_connection_ref).unwrap();
        let start = Instant::now(); run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let command = commands(&s)[0].clone();
        for (seq, at_ms, kind) in [(2, 2, "started"), (3, 5, "paused"), (4, 25, "resumed")] {
            let admission = s.apply_receipt(id(&format!("physical-{kind}")), stamp(), receipt(&command, seq, json!({"kind":kind})), at(start, at_ms)).unwrap();
            assert!(matches!(admission.decision, ReceiptAdmissionDecision::Applied));
            let snapshot = s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap();
            assert_eq!(matches!(snapshot.bound_jobs[0].state, BoundExecutionJobSnapshotState::Paused), kind == "paused");
            assert_eq!(snapshot.bound_jobs[0].input_refs.len(), 1);
            assert!(snapshot.business_facts.acceptance_events.is_empty());
        }
        s.apply_receipt(id("physical-completed"), stamp(), receipt(&command, 5, json!({"kind":"completed","device_reported_duration_ms":8})), at(start, 30)).unwrap();
        let view = s.observe().unwrap();
        assert_eq!(view.phase_work_intervals.len(), 2);
        assert_eq!(view.phase_work_intervals[1].effective_progress_end_ms.0.value(), 8);
    }

    #[test]
    fn phase_reports_cannot_start_jobs_and_wrong_bound_phases_are_rejected() {
        let mut s=session();let start=Instant::now();run(&mut s,start);s.tick(at(start,1)).unwrap();
        let mut report=observation(&s,"L1-D01",1);
        report.device_state=ExecutionTelemetryObservationDeviceState::Busy;
        report.active_command_id=Some("reported-but-not-started".to_owned().try_into().unwrap());
        report.phase_progress=Some(serde_json::from_value(json!({"phase_id":"cycle","effective_work_ms":500,"duration_ms":1800})).unwrap());
        let before=s.revision;
        let mut invalid=report.clone();invalid.phase_progress.as_mut().unwrap().phase_id="another-phase".to_owned().try_into().unwrap();
        assert!(s.apply_observation(id("invalid-phase"),stamp(),invalid,at(start,2)).is_err());
        assert_eq!(s.revision,before);
        let admission=s.apply_observation(id("measured-not-started"),stamp(),report,at(start,2)).unwrap();
        assert!(matches!(admission.decision,ReceiptAdmissionDecision::Applied));
        let observed=s.observe().unwrap();
        assert!(observed.jobs.iter().all(|job|job.started_at_ms.is_none()));
        assert!(observed.allocations.is_empty());
    }

    #[test]
    fn preparation_has_one_original_six_job_engine_and_no_clock_or_dispatch() {
        let s = session();
        let o = s.observe().unwrap();
        assert!(!o.clock_started);
        assert_eq!(o.logical_work.len(), 6);
        assert_eq!(o.items.len(), 6);
        assert!(o.jobs.iter().all(|j| j.started_at_ms.is_none()));
        assert!(o.allocations.is_empty());
        assert!(commands(&s).is_empty());
        assert_eq!(s.publications.len(), 1);
        assert!(!s.publications[&0].snapshot.coverage.clock_started);
        assert_eq!(s.elapsed(Instant::now()).unwrap(), 0);
        assert_eq!(
            s.devices[&id("L1-D01")].bindings[0].nominal_work_ms.value(),
            10000
        );
        assert_eq!(
            s.devices[&id("L1-D02")].bindings[0].nominal_work_ms.value(),
            15000
        );
    }
    #[test]
    fn unrelated_idle_report_does_not_rewrite_unchanged_waiting_decisions() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        for device in ["L1-D01", "L1-D02"] {
            s.control(id("operator"), id(&format!("disable-{device}")), s.revision,
                ControlAction::SetDeviceAcceptance { device_id: id(device), accept_new_work: false }, start).unwrap();
        }
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let decisions = s.dispatch_decisions.clone();
        let before = s.projector.history_document().unwrap().records.len();
        ready(&mut s, "L1-D01", 2, at(start, 1));
        assert!(equal(&decisions, &s.dispatch_decisions));
        let history = s.projector.history_document().unwrap();
        assert!(history.records[before..].iter().all(|r| !matches!(r.object, ExecutionObservedObject::LogicalWork { .. })));
        assert!(commands(&s).is_empty());
        assert_eq!(s.admissions.len(), 2);
    }
    #[test]
    fn same_inlet_time_reports_retain_all_admissions_and_dispatch_on_readiness_change() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        for sequence in [1, 2] {
            let mut report = observation(&s, "L1-D01", sequence);
            report.accept_new_work = false;
            s.apply_observation(id(&format!("not-ready-{sequence}")), stamp(), report, at(start, 1)).unwrap();
            assert!(commands(&s).is_empty());
        }
        ready(&mut s, "L1-D01", 3, at(start, 1));
        assert_eq!(commands(&s).len(), 1);
        assert_eq!(s.admissions.len(), 3);
        assert!(s.admissions.iter().all(|a| matches!(a.decision, ReceiptAdmissionDecision::Applied)));
    }
    #[test]
    fn original_probe_expiry_and_exact_retry_do_not_refresh_eligibility() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        let old = observation(&s, "L1-D01", 1);
        let revision = s.revision;
        s.tick(at(start, 2001)).unwrap();
        assert!(s.revision > revision);
        let a = s
            .apply_observation(id("late"), stamp(), old.clone(), at(start, 2001))
            .unwrap();
        assert!(matches!(a.decision, ReceiptAdmissionDecision::Rejected));
        assert!(commands(&s).is_empty());
        let r = s.revision;
        let count = s.admissions.len();
        let repeated = s
            .apply_observation(id("late"), stamp(), old, at(start, 2050))
            .unwrap();
        assert!(matches!(
            repeated.decision,
            ReceiptAdmissionDecision::Duplicate
        ));
        assert_eq!(s.revision, r);
        assert_eq!(s.admissions.len(), count);
        assert!(s.devices[&id("L1-D01")].latest.is_none());
        ready(&mut s, "L1-D01", 2, at(start, 2100));
        assert_eq!(commands(&s).len(), 1);
    }
    #[test]
    fn operator_disable_survives_fresh_device_acceptance_and_rank_uses_eligible_set() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        s.control(
            id("operator"),
            id("disable-A"),
            s.revision,
            ControlAction::SetDeviceAcceptance {
                device_id: id("L1-D01"),
                accept_new_work: false,
            },
            start,
        )
        .unwrap();
        ready(&mut s, "L1-D01", 1, at(start, 1));
        assert!(commands(&s).is_empty());
        ready(&mut s, "L1-D02", 1, at(start, 2));
        let c = commands(&s);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].device_id, id("L1-D02"));
        let explanation = &s.attempts[&c[0].attempt.command_id].dispatch;
        assert_eq!(
            explanation
                .candidates
                .iter()
                .find(|v| v.eligible)
                .unwrap()
                .rank
                .as_ref()
                .unwrap()
                .value(),
            1
        );
        assert!(
            !s.device_views(2)
                .iter()
                .find(|v| v.binding.device_id == id("L1-D01"))
                .unwrap()
                .operator_accept_new_work
        );
    }
    #[test]
    fn publication_barrier_preserves_unknown_work_until_positive_cloud_withdrawal() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let original = commands(&s)[0].clone();
        ready(&mut s, "L1-D02", 1, at(start, 2));
        s.control(
            id("operator"),
            id("disable-A"),
            s.revision,
            ControlAction::SetDeviceAcceptance {
                device_id: id("L1-D01"),
                accept_new_work: false,
            },
            at(start, 3),
        )
        .unwrap();
        let work = s.cloud_work();
        let (work_id, cid) = work
            .items
            .iter()
            .find_map(|w| match w {
                CloudWorkItem::WithdrawCommand {
                    work_id,
                    command_id,
                    ..
                } => Some((work_id.clone(), command_id.clone())),
                _ => None,
            })
            .unwrap();
        assert_eq!(cid, original.attempt.command_id);
        let still = s.observe().unwrap();
        assert_eq!(
            still
                .attempts
                .iter()
                .filter(|a| a.command.attempt.logical_job_id == original.attempt.logical_job_id)
                .count(),
            1
        );
        let mut wrong = serde_json::to_value(CloudEffectAck::WithdrawalResult {
            work_id: work_id.clone(),
            command_id: cid.clone(),
            state: CloudResponseVariant0ResultWithdrawalDataState::WithdrawnBeforeDelivery,
        })
        .unwrap();
        wrong["command_id"] = json!("foreign");
        assert!(
            s.acknowledge_cloud_work(serde_json::from_value(wrong).unwrap(), at(start, 4))
                .is_err()
        );
        s.acknowledge_cloud_work(
            CloudEffectAck::WithdrawalResult {
                work_id,
                command_id: cid,
                state: CloudResponseVariant0ResultWithdrawalDataState::WithdrawnBeforeDelivery,
            },
            at(start, 4),
        )
        .unwrap();
        let a = s.observe().unwrap();
        assert!(a.attempts.iter().any(|a| a.command.attempt.command_id
            == original.attempt.command_id
            && a.state == ExternalAttemptState::CancelledNotStarted));
        assert!(a.allocations.is_empty());
        assert!(a.acceptance_events.is_empty());
        // B still owns another published attempt. Only its own positive proof
        // frees it; then the original A work may acquire B with a new attempt.
        s.control(
            id("operator"),
            id("disable-B"),
            s.revision,
            ControlAction::SetDeviceAcceptance {
                device_id: id("L1-D02"),
                accept_new_work: false,
            },
            at(start, 5),
        )
        .unwrap();
        let (work_id, command_id) = s
            .cloud_work()
            .items
            .iter()
            .find_map(|w| match w {
                CloudWorkItem::WithdrawCommand {
                    work_id,
                    command_id,
                    ..
                } if *command_id != original.attempt.command_id => {
                    Some((work_id.clone(), command_id.clone()))
                }
                _ => None,
            })
            .unwrap();
        s.acknowledge_cloud_work(
            CloudEffectAck::WithdrawalResult {
                work_id,
                command_id,
                state: CloudResponseVariant0ResultWithdrawalDataState::WithdrawnBeforeDelivery,
            },
            at(start, 6),
        )
        .unwrap();
        s.control(
            id("operator"),
            id("enable-B"),
            s.revision,
            ControlAction::SetDeviceAcceptance {
                device_id: id("L1-D02"),
                accept_new_work: true,
            },
            at(start, 7),
        )
        .unwrap();
        let current = s.observe().unwrap();
        let reassigned = current
            .attempts
            .iter()
            .find(|a| {
                a.command.attempt.logical_job_id == original.attempt.logical_job_id
                    && a.state == ExternalAttemptState::Published
            })
            .unwrap();
        assert_eq!(reassigned.command.device_id, id("L1-D02"));
        assert_eq!(reassigned.command.attempt.attempt_ordinal.value(), 2);
        assert!(current.acceptance_events.is_empty());
        assert_eq!(current.items.len(), 6);
    }
    #[test]
    fn acknowledged_command_retry_accepts_the_servers_same_payload_storage_status() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let CloudWorkItem::PublishCommand { work_id, command } = s
            .cloud_work()
            .items
            .into_iter()
            .find(|w| matches!(w, CloudWorkItem::PublishCommand { .. }))
            .unwrap()
        else {
            panic!("command work")
        };
        let ack = CloudEffectAck::CommandStored {
            work_id: work_id.clone(),
            command_id: command.attempt.command_id.clone(),
            storage_status: CloudResponseVariant0ResultCommandStoredDataStorageStatus::Stored,
        };
        s.acknowledge_cloud_work(ack, start).unwrap();
        let revision = s.revision;
        s.acknowledge_cloud_work(CloudEffectAck::CommandStored{work_id,command_id:command.attempt.command_id,
            storage_status:CloudResponseVariant0ResultCommandStoredDataStorageStatus::AlreadyStoredSamePayload},start).unwrap();
        assert_eq!(s.revision, revision);
        assert!(commands(&s).is_empty());
        assert!(s.observe().unwrap().acceptance_events.is_empty());
    }
    #[test]
    fn completed_before_started_is_retained_then_applied_once_despite_newer_telemetry() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let command = commands(&s)[0].clone();
        let completed = receipt(
            &command,
            3,
            json!({"kind":"completed","device_reported_duration_ms":10000}),
        );
        let first = s
            .apply_receipt(id("done"), stamp(), completed.clone(), at(start, 10))
            .unwrap();
        assert!(matches!(
            first.decision,
            ReceiptAdmissionDecision::PendingDependency
        ));
        assert!(s.observe().unwrap().allocations.is_empty());
        let mut newer = observation(&s, "L1-D01", 20);
        newer.device_state = ExecutionTelemetryObservationDeviceState::Busy;
        newer.active_command_id = Some(command.attempt.command_id.clone());
        s.apply_observation(id("newer-telemetry"), stamp(), newer, at(start, 11))
            .unwrap();
        s.apply_receipt(
            id("started"),
            stamp(),
            receipt(&command, 2, json!({"kind":"started"})),
            at(start, 12),
        )
        .unwrap();
        let after = s.observe().unwrap();
        assert!(
            after.allocations.is_empty(),
            "Completed released the original active allocation"
        );
        assert_eq!(after.acceptance_events.len(), 1);
        assert!(matches!(
            s.messages[&id("done")].latest.decision,
            ReceiptAdmissionDecision::Applied
        ));
        assert_eq!(
            s.admissions
                .iter()
                .filter(|a| a.receipt_id == id("done"))
                .count(),
            2,
            "pending and later applied stay immutable"
        );
        s.apply_receipt(id("done"), stamp(), completed, at(start, 13))
            .unwrap();
        assert_eq!(s.observe().unwrap().acceptance_events.len(), 1);
        assert!(s.observe().unwrap().allocations.is_empty());
        assert!(
            matches!(s.phase, ExecutionSnapshotLifecycleState::Running),
            "resolved dependency restores original dispatch mode"
        );
        ready(&mut s, "L1-D01", 21, at(start, 14));
        assert_eq!(s.observe().unwrap().attempts.len(), 2);
    }
    #[test]
    fn pause_resume_preserves_the_original_clock_and_control_request_identity() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        let rev = s.revision;
        let ack = s
            .control(
                id("operator"),
                id("pause"),
                rev,
                ControlAction::PauseDispatch {},
                at(start, 100),
            )
            .unwrap();
        let repeated = s
            .control(
                id("operator"),
                id("pause"),
                rev,
                ControlAction::PauseDispatch {},
                at(start, 500),
            )
            .unwrap();
        assert!(equal(&ack, &repeated));
        assert!(
            s.control(
                id("another-operator"),
                id("pause"),
                rev,
                ControlAction::PauseDispatch {},
                at(start, 500)
            )
            .is_err(),
            "another caller cannot reuse the original request record"
        );
        assert!(
            s.control(
                id("operator"),
                id("pause"),
                rev,
                ControlAction::StopDispatch {},
                at(start, 500)
            )
            .is_err()
        );
        s.control(
            id("operator"),
            id("resume"),
            s.revision,
            ControlAction::ResumeDispatch {},
            at(start, 700),
        )
        .unwrap();
        assert_eq!(s.elapsed(at(start, 1200)).unwrap(), 1200);
        assert!(s.clock_anchor == Some(start));
        assert!(
            s.control(
                id("operator"),
                id("stale-revision"),
                0,
                ControlAction::StopDispatch {},
                at(start, 1200)
            )
            .is_err()
        );
    }
    #[test]
    fn actual_same_stock_accepts_exactly_six_units_and_never_uses_nominal_completion() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        for n in 0..6u64 {
            let time = n * 10001 + 1;
            ready(&mut s, "L1-D01", n * 3 + 1, at(start, time));
            let command = commands(&s).last().unwrap().clone();
            s.apply_receipt(
                id(&format!("start-{n}")),
                stamp(),
                receipt(&command, n * 3 + 2, json!({"kind":"started"})),
                at(start, time + 1),
            )
            .unwrap();
            assert_eq!(s.observe().unwrap().allocations.len(), 1);
            s.tick(at(start, time + 9999)).unwrap();
            assert_eq!(s.observe().unwrap().acceptance_events.len(), n as usize);
            s.apply_receipt(
                id(&format!("complete-{n}")),
                stamp(),
                receipt(
                    &command,
                    n * 3 + 3,
                    json!({"kind":"completed","device_reported_duration_ms":10000}),
                ),
                at(start, time + 10000),
            )
            .unwrap();
        }
        let result = s.observe().unwrap();
        assert_eq!(result.logical_work.len(), 6);
        assert_eq!(result.jobs.len(), 6);
        assert_eq!(result.acceptance_events.len(), 6);
        assert!(result.allocations.is_empty());
        assert!(result.goal_met && result.drained);
        assert_eq!(result.items.len(), 6);
        assert_eq!(
            result
                .items
                .iter()
                .map(|item| item.id.as_str().to_owned())
                .collect::<std::collections::BTreeSet<_>>(),
            (0..6).map(|n| format!("blank-{n}")).collect()
        );
        assert!(matches!(
            s.phase,
            ExecutionSnapshotLifecycleState::Completed
        ));
        let final_revision = s.revision;
        assert!(
            s.control(
                id("operator"),
                id("after-completed"),
                final_revision,
                ControlAction::SetDeviceAcceptance {
                    device_id: id("L1-D01"),
                    accept_new_work: false
                },
                at(start, 60008)
            )
            .is_err()
        );
        assert_eq!(s.revision, final_revision);
        let history = s.projector.history_document().unwrap();
        assert!(
            history
                .records
                .iter()
                .enumerate()
                .all(|(i, r)| r.record_sequence.value() == i as u64 + 1)
        );
        assert!(s.publications.keys().copied().eq(0..=s.revision));
        let frozen =
            serde_json::to_value(s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap())
                .unwrap();
        let extra = observation(&s, "L1-D01", 99);
        assert!(
            s.apply_observation(
                id("after-terminal-telemetry"),
                stamp(),
                extra,
                at(start, 60010)
            )
            .is_err()
        );
        assert_eq!(
            serde_json::to_value(s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap())
                .unwrap(),
            frozen
        );
        assert_eq!(
            result
                .acceptance_events
                .iter()
                .map(|a| a.quantity.value.coefficient.value())
                .sum::<u64>(),
            6
        );
    }
    #[test]
    fn same_millisecond_cutoff_keeps_started_prefix_before_pending_completion_resolution() {
        let mut s = session_with(12, false);
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let command = commands(&s)[0].clone();
        s.apply_receipt(
            id("pending-done"),
            stamp(),
            receipt(
                &command,
                3,
                json!({"kind":"completed","device_reported_duration_ms":10000}),
            ),
            at(start, 10),
        )
        .unwrap();
        let started = s
            .apply_receipt(
                id("late-start"),
                stamp(),
                receipt(&command, 2, json!({"kind":"started"})),
                at(start, 12),
            )
            .unwrap();
        let cutoff = s
            .projector
            .snapshot(&SnapshotSelection::Cutoff {})
            .unwrap()
            .into_owned();
        let latest = s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap();
        assert_eq!(
            cutoff.execution_revision.value(),
            started.execution_revision.value()
        );
        assert_eq!(cutoff.coverage.observed_until_host_ms.value(), 12);
        assert_eq!(cutoff.business_facts.acceptance_events.len(), 0);
        assert_eq!(latest.business_facts.acceptance_events.len(), 1);
        assert!(latest.execution_revision.value() > cutoff.execution_revision.value());
        let evidence = s
            .receipt_evidence
            .iter()
            .find(|e| {
                e.admission.receipt_id == id("pending-done")
                    && matches!(e.admission.decision, ReceiptAdmissionDecision::Applied)
            })
            .unwrap();
        assert_eq!(evidence.admission.host_received_elapsed_ms.value(), 10);
        assert_eq!(evidence.committed_at_host_ms.as_ref().unwrap().value(), 12);
        assert_eq!(
            cutoff
                .receipt_admissions
                .iter()
                .filter(|a| a.receipt_id == id("pending-done"))
                .count(),
            1
        );
        assert_eq!(
            latest
                .receipt_admissions
                .iter()
                .filter(|a| a.receipt_id == id("pending-done"))
                .count(),
            2
        );
        s.tick(at(start, 30)).unwrap();
        assert!(equal(
            &s.projector.snapshot(&SnapshotSelection::Cutoff {}).unwrap(),
            &cutoff
        ));
        assert_eq!(
            s.publications
                .values()
                .filter(|p| p.as_cutoff_snapshot)
                .count(),
            1
        );
    }
    #[test]
    fn later_receipt_never_revises_the_original_clock_cutoff_or_closes_its_open_claim() {
        let mut s = session_with(10, false);
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let command = commands(&s)[0].clone();
        s.apply_receipt(
            id("start"),
            stamp(),
            receipt(&command, 2, json!({"kind":"started"})),
            at(start, 2),
        )
        .unwrap();
        s.tick(at(start, 20)).unwrap();
        let cutoff = s
            .projector
            .snapshot(&SnapshotSelection::Cutoff {})
            .unwrap()
            .into_owned();
        assert_eq!(cutoff.coverage.observed_until_host_ms.value(), 10);
        assert!(cutoff.business_facts.acceptance_events.is_empty());
        assert!(cutoff.business_facts.resource_claims.iter().any(
            |c| c.open_interval_index.is_some()
                && matches!(
                    c.view_boundary,
                    ProjectedResourceClaimViewBoundary::ReportCutoff
                )
        ));
        s.apply_receipt(
            id("done"),
            stamp(),
            receipt(
                &command,
                3,
                json!({"kind":"completed","device_reported_duration_ms":10000}),
            ),
            at(start, 30),
        )
        .unwrap();
        assert_eq!(
            s.projector
                .snapshot(&SnapshotSelection::Latest {})
                .unwrap()
                .business_facts
                .acceptance_events
                .len(),
            1
        );
        assert!(equal(
            &s.projector.snapshot(&SnapshotSelection::Cutoff {}).unwrap(),
            &cutoff
        ));
    }
    #[test]
    fn original_arrival_and_expiry_are_observed_without_backdating_new_probe_issuance() {
        let mut s = session_with(120000, true);
        let start = Instant::now();
        assert_eq!(s.publications[&0].snapshot.business_facts.items.len(), 5);
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        s.tick(at(start, 2500)).unwrap();
        let arrival = s
            .publications
            .values()
            .find(|p| p.snapshot.coverage.observed_until_host_ms.value() == 5)
            .unwrap();
        assert_eq!(arrival.snapshot.business_facts.items.len(), 6);
        let expired = s
            .publications
            .values()
            .find(|p| p.snapshot.coverage.observed_until_host_ms.value() == 2001)
            .unwrap();
        let device = expired
            .snapshot
            .devices
            .iter()
            .find(|d| d.binding.device_id == id("L1-D01"))
            .unwrap();
        assert!(matches!(
            device.freshness,
            ExecutionDeviceViewFreshness::Stale
        ));
        assert_eq!(
            device
                .pending_observation_probe
                .as_ref()
                .unwrap()
                .issued_at_host_ms
                .value(),
            1
        );
        let latest = s.projector.snapshot(&SnapshotSelection::Latest {}).unwrap();
        let device = latest
            .devices
            .iter()
            .find(|d| d.binding.device_id == id("L1-D01"))
            .unwrap();
        assert_eq!(
            device
                .pending_observation_probe
                .as_ref()
                .unwrap()
                .issued_at_host_ms
                .value(),
            2500
        );
        assert_eq!(latest.pending_reservations.len(), 1);
        let history = s.projector.history_document().unwrap();
        assert!(history.records.iter().any(|r| r.at_host_ms.value() == 5
            && matches!(&r.object,ExecutionObservedObject::Item{data} if data.id==id("blank-5"))));
    }
    #[test]
    fn failed_after_started_keeps_its_actual_item_and_resource_and_cannot_be_reassigned() {
        let mut s = session();
        let start = Instant::now();
        run(&mut s, start);
        ready(&mut s, "L1-D01", 1, at(start, 1));
        let command = commands(&s)[0].clone();
        s.apply_receipt(
            id("started"),
            stamp(),
            receipt(&command, 2, json!({"kind":"started"})),
            at(start, 2),
        )
        .unwrap();
        s.apply_receipt(
            id("failed"),
            stamp(),
            receipt(
                &command,
                3,
                json!({"kind":"failed","code":"motor","message":"actual fault"}),
            ),
            at(start, 3),
        )
        .unwrap();
        let original = s.observe().unwrap();
        assert!(!original.drained);
        assert!(!original.resource_claims.is_empty());
        assert!(matches!(
            s.phase,
            ExecutionSnapshotLifecycleState::NeedsResolution
        ));
        assert!(
            s.engine
                .confirm_external_withdrawal(&command.attempt.command_id, uint(4))
                .is_err()
        );
        assert!(s.observe().unwrap().acceptance_events.is_empty());
        assert!(
            s.device_views(3)
                .iter()
                .find(|v| v.binding.device_id == id("L1-D01"))
                .unwrap()
                .blocking_reasons
                .iter()
                .any(|r| r.as_str() == "original_attempt_unsettled")
        );
    }
}
fn engine_error(e: sf_simulation::ExternalError) -> ApiError {
    fault(ErrorCode::InvalidInput, e.to_string())
}
fn equal(a: &impl serde::Serialize, b: &impl serde::Serialize) -> bool {
    serde_json::to_value(a).unwrap() == serde_json::to_value(b).unwrap()
}

pub(crate) struct DeviceSlot {
    pub bindings: Vec<CandidateBinding>,
    pub operator_accept: bool,
    pub boot: Option<DeviceBootId>,
    pub probe: Option<ObservationProbe>,
    pub latest: Option<HostObservedDevice>,
}
#[derive(Clone)]
pub(crate) struct AttemptLedger {
    pub command: Command,
    pub reserved_at_host_ms: u64,
    pub published_at_host_ms: u64,
    pub dispatch: DispatchExplanation,
    pub reservation_id: ObjectId,
    pub resource_plan: Vec<ResourceUnits>,
}
#[derive(Clone)]
pub(crate) struct AdmittedMessage {
    pub payload: Value,
    pub production_receipt: Option<DeviceReceipt>,
    pub latest: ReceiptAdmission,
    pub first_received_elapsed_ms: u64,
}

/// Preparation transfers this single owner into ProjectHost after its actual
/// initial cloud publication. No public request can supply an Engine or clock.
#[derive(Clone, Copy)]
enum DispatchMode {
    Prepared,
    Running,
    Paused,
    Stopped,
    Reconciliation,
}

pub(crate) struct ExecutionSession {
    dispatch_mode: DispatchMode,
    pub engine: Engine,
    pub frozen: ExecutionFrozenInputs,
    pub provenance: ExecutionSnapshotProvenance,
    pub projector: ExecutionProjector,
    pub receipt_evidence: Vec<HostReceiptEvidence>,
    pub identity: ExecutionIdentity,
    pub policy: ExecutionPolicy,
    pub dataset_id: Option<CloudDatasetId>,
    pub revision: u64,
    pub phase: ExecutionSnapshotLifecycleState,
    pub clock_anchor: Option<Instant>,
    pub devices: BTreeMap<ObjectId, DeviceSlot>,
    pub attempts: BTreeMap<DispatchCommandId, AttemptLedger>,
    pub dispatch_decisions: BTreeMap<ObjectId, DispatchExplanation>,
    pub admissions: Vec<ReceiptAdmission>,
    pub messages: BTreeMap<ObjectId, AdmittedMessage>,
    pub cloud_published_revision: Option<u64>,
    pub pending_cloud_work: Vec<CloudWorkItem>,
    pub publications: BTreeMap<u64, SnapshotPublication>,
    control_requests: BTreeMap<(ObjectId, ObjectId), (Value, ControlAck)>,
    acknowledged_work: BTreeMap<ObjectId, Value>,
    device_sequences: BTreeMap<(ObjectId, DeviceBootId, u64), ObjectId>,
    next_identity: u64,
    admission_sequence: u64,
}

impl ExecutionSession {
    pub fn prepare(
        frozen: ExecutionFrozenInputs,
        policy: ExecutionPolicy,
        dataset_id: CloudDatasetId,
        connection_ref: ObjectId,
    ) -> Result<Self> {
        Self::prepare_inner(frozen, policy, Some(dataset_id), connection_ref)
    }
    pub fn prepare_before_cloud(
        frozen: ExecutionFrozenInputs,
        policy: ExecutionPolicy,
        connection_ref: ObjectId,
    ) -> Result<Self> {
        Self::prepare_inner(frozen, policy, None, connection_ref)
    }
    fn prepare_inner(
        frozen: ExecutionFrozenInputs,
        policy: ExecutionPolicy,
        dataset_id: Option<CloudDatasetId>,
        connection_ref: ObjectId,
    ) -> Result<Self> {
        let identity = frozen.identity.clone();
        let candidates = frozen.candidate_bindings.clone();
        let input = EngineInput {
            run_id: RunId::try_from(format!("{}:engine", identity.session_id.as_str())).unwrap(),
            input_ref: frozen.project_input_ref.clone(),
            project: frozen.project.clone(),
            scenario: frozen.scenario.clone(),
            definitions: frozen.execution_definitions.clone(),
            scope: frozen.scope.clone(),
        };
        let engine =
            Engine::new_external(input, identity.clone(), policy.clone(), candidates.clone())
                .map_err(engine_error)?;
        let provenance = ExecutionSnapshotProvenance {
            mode: ExecutionSnapshotProvenanceMode::SoftwareInLoop,
            cloud_connection_ref: connection_ref,
            device_binding_refs: candidates.iter().map(|b| b.binding_id.clone()).collect(),
            producer: ExecutionSnapshotProvenanceProducer::Host,
        };
        let mut devices: BTreeMap<ObjectId, DeviceSlot> = BTreeMap::new();
        for binding in candidates {
            devices.entry(binding.device_id.clone()).or_insert_with(|| DeviceSlot {
                bindings: vec![], operator_accept: true, boot: None, probe: None, latest: None,
            }).bindings.push(binding);
        }
        let mut session = Self {
            engine,
            frozen,
            provenance,
            projector: ExecutionProjector::new(),
            receipt_evidence: vec![],
            identity,
            policy,
            devices,
            dataset_id,
            revision: 0,
            phase: ExecutionSnapshotLifecycleState::Prepared,
            dispatch_mode: DispatchMode::Prepared,
            clock_anchor: None,
            attempts: BTreeMap::new(),
            dispatch_decisions: BTreeMap::new(),
            admissions: vec![],
            messages: BTreeMap::new(),
            cloud_published_revision: None,
            pending_cloud_work: vec![],
            publications: BTreeMap::new(),
            control_requests: BTreeMap::new(),
            acknowledged_work: BTreeMap::new(),
            device_sequences: BTreeMap::new(),
            next_identity: 1,
            admission_sequence: 0,
        };
        session.capture_frame()?;
        Ok(session)
    }
    fn is_terminal(&self) -> bool {
        matches!(
            self.phase,
            ExecutionSnapshotLifecycleState::Completed
                | ExecutionSnapshotLifecycleState::Stopped
                | ExecutionSnapshotLifecycleState::Failed
        )
    }
    fn has_cutoff_frame(&self) -> bool {
        self.publications.values().any(|p| p.as_cutoff_snapshot)
    }
    fn capture_frame(&mut self) -> Result<()> {
        let observation = self.observe()?;
        let as_cutoff =
            self.engine.external_cutoff_observation().is_some() && !self.has_cutoff_frame();
        if as_cutoff && observation.elapsed_ms != self.policy.report_cutoff_ms.value() {
            return Err(fault(
                ErrorCode::ComputationFailed,
                "Original Host cutoff boundary was not captured; later state cannot backfill it",
            ));
        }
        let attempts = self
            .attempts
            .values()
            .map(|m| {
                let actual = observation
                    .attempts
                    .iter()
                    .find(|a| a.command.attempt.command_id == m.command.attempt.command_id)
                    .expect("same original Engine attempt ledger");
                AttemptMetadata {
                    command_id: m.command.attempt.command_id.clone(),
                    reservation_id: m.reservation_id.clone(),
                    reserved_at_host_ms: Some(uint(m.reserved_at_host_ms)),
                    published_at_host_ms: Some(uint(m.published_at_host_ms)),
                    withdrawn_unpublished_at_host_ms: None,
                    reserved_resource_units: m.resource_plan.clone(),
                    pending_input_ordinals: Some(actual.pending_input_ordinals.clone()),
                    dispatch_decision: Some(m.dispatch.clone()),
                }
            })
            .collect();
        let ledger = HostProjectionLedger {
            execution_revision: uint(self.revision),
            host_admission_sequence: uint(self.admission_sequence),
            policy: self.policy.clone(),
            provenance: self.provenance.clone(),
            lifecycle_state: self.phase.clone(),
            lifecycle_reason: match self.phase {
                ExecutionSnapshotLifecycleState::NeedsResolution => Some(
                    DeviceText::try_from("Original execution facts require reconciliation".to_owned()).unwrap()
                ),
                ExecutionSnapshotLifecycleState::Draining => {
                    let waits: Vec<_> = self.engine.external_output_capacity_waits()
                        .map(|(job, location)| format!("作业 {} 等待库位 {} 腾出容量", job.as_str(), location.as_str()))
                        .collect();
                    (!waits.is_empty()).then(|| DeviceText::try_from(waits.join("；")).unwrap())
                }
                _ => None,
            },
            new_dispatch_allowed: matches!(self.phase, ExecutionSnapshotLifecycleState::Running)
                && !observation.dispatch_stopped
                && !observation.drained,
            terminal_disposition_recorded: false,
            attempts,
            receipts: self.receipt_evidence.clone(),
            observation_admissions: self
                .admissions
                .iter()
                .filter(|a| {
                    self.messages
                        .get(&a.receipt_id)
                        .is_some_and(|m| m.production_receipt.is_none())
                })
                .cloned()
                .collect(),
            devices: self.device_views(observation.elapsed_ms),
            queued_dispatch_decisions: self.dispatch_decisions.clone(),
            missing_conditions: vec![],
        };
        let batch = self
            .projector
            .append(ProjectionInput {
                observation: &observation,
                frozen: &self.frozen,
                ledger: &ledger,
                as_cutoff_snapshot: as_cutoff,
                history_contiguous: true,
            })
            .map_err(|e| fault(ErrorCode::ComputationFailed, e.to_string()))?;
        self.queue_publication(SnapshotPublication {
            contract_version: SnapshotPublicationContractVersion::SfExecution20Draft11,
            expected_published_revision: if self.revision == 0 {
                None
            } else {
                Some(uint(self.revision - 1))
            },
            snapshot: batch.snapshot,
            as_cutoff_snapshot: as_cutoff,
            observation_records: batch.observation_records,
        })
    }
    /// Observe deterministic original model/expiry/cutoff boundaries before a
    /// later input. New probes/dispatch still use the actual input time.
    fn advance_boundaries(&mut self, elapsed: u64, receipt_at_exact_cutoff: bool) -> Result<()> {
        if self.clock_anchor.is_none() || self.is_terminal() {
            return Ok(());
        }
        loop {
            let current = self.observe()?.elapsed_ms;
            let mut boundaries = vec![];
            if let Some(next) = self
                .engine
                .next_external_boundary_ms()
                .map_err(engine_error)?
            {
                if next.value() > current && next.value() < elapsed {
                    boundaries.push(next.value());
                }
            }
            let cutoff = self.policy.report_cutoff_ms.value();
            if !self.has_cutoff_frame()
                && cutoff >= current
                && (cutoff < elapsed || cutoff == elapsed && !receipt_at_exact_cutoff)
            {
                boundaries.push(cutoff);
            }
            for slot in self.devices.values() {
                if let Some(latest) = &slot.latest {
                    if let Some(expiry) = latest
                        .observation_probe
                        .expires_at_host_ms
                        .value()
                        .checked_add(1)
                    {
                        if current < expiry && expiry < elapsed {
                            boundaries.push(expiry);
                        }
                    }
                }
            }
            let Some(next) = boundaries.into_iter().min() else {
                break;
            };
            let revision = self.next_revision()?;
            self.engine
                .advance_external(uint(next))
                .map_err(engine_error)?;
            self.revision = revision;
            self.reconcile_lifecycle()?;
            self.capture_frame()?;
            if self.is_terminal() {
                break;
            }
        }
        Ok(())
    }
    pub fn observe(&self) -> Result<ExternalObservation> {
        self.engine.external_observation().map_err(engine_error)
    }
    pub fn elapsed(&self, now: Instant) -> Result<u64> {
        let Some(anchor) = self.clock_anchor else {
            return Ok(0);
        };
        let millis = now
            .checked_duration_since(anchor)
            .ok_or_else(|| {
                fault(
                    ErrorCode::InvalidInput,
                    "Host monotonic time cannot precede its original run anchor",
                )
            })?
            .as_millis();
        if millis > MAX_SAFE_UINT as u128 {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Host elapsed exceeds the execution wire range",
            ));
        }
        Ok(millis as u64)
    }
    fn next_revision(&self) -> Result<u64> {
        self.revision
            .checked_add(1)
            .filter(|n| *n <= MAX_SAFE_UINT)
            .ok_or_else(|| fault(ErrorCode::InvalidInput, "Execution revision is exhausted"))
    }
    fn next_id(&mut self, purpose: &str) -> ObjectId {
        let id = object(format!(
            "{}:{purpose}:{}",
            self.identity.session_id.as_str(),
            self.next_identity
        ));
        self.next_identity += 1;
        id
    }
    pub fn register_boot(
        &mut self,
        device: &ObjectId,
        boot: DeviceBootId,
        now: Instant,
    ) -> Result<()> {
        let slot = self.devices.get(device).ok_or_else(|| {
            fault(
                ErrorCode::PermissionDenied,
                "Device is outside this execution",
            )
        })?;
        if let Some(original) = &slot.boot {
            return if original == &boot {
                Ok(())
            } else {
                Err(fault(
                    ErrorCode::InvalidInput,
                    "The original device boot must be settled and retired before replacement",
                ))
            };
        }
        if self.is_terminal() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Settled execution cannot admit a new device boot",
            ));
        }
        let elapsed = self.elapsed(now)?;
        self.advance_boundaries(elapsed, false)?;
        if self.is_terminal() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Execution settled before boot admission",
            ));
        }
        let next = self.next_revision()?;
        if self.clock_anchor.is_some() {
            self.engine
                .advance_external(uint(elapsed))
                .map_err(engine_error)?;
        }
        self.devices.get_mut(device).unwrap().boot = Some(boot);
        self.revision = next;
        self.refresh_probes(elapsed)?;
        self.reconcile_lifecycle()?;
        self.capture_frame()
    }
    fn refresh_probes(&mut self, elapsed: u64) -> Result<bool> {
        if self.clock_anchor.is_none()
            || matches!(
                self.phase,
                ExecutionSnapshotLifecycleState::Completed
                    | ExecutionSnapshotLifecycleState::Stopped
                    | ExecutionSnapshotLifecycleState::Failed
            )
        {
            return Ok(false);
        }
        let expires = elapsed
            .checked_add(self.policy.observation_max_age_ms.value())
            .filter(|n| *n <= MAX_SAFE_UINT)
            .ok_or_else(|| {
                fault(
                    ErrorCode::InvalidInput,
                    "Probe deadline exceeds the execution wire range",
                )
            })?;
        let devices: Vec<_> = self
            .devices
            .iter()
            .filter_map(|(id, s)| {
                (s.boot.is_some()
                    && s.probe
                        .as_ref()
                        .is_none_or(|p| p.expires_at_host_ms.value() < elapsed))
                .then_some(id.clone())
            })
            .collect();
        for device in &devices {
            let probe_id =
                ObservationProbeId::try_from(self.next_id("probe").as_str().to_owned()).unwrap();
            let slot = self.devices.get_mut(device).unwrap();
            slot.probe = Some(ObservationProbe {
                probe_id,
                device_id: device.clone(),
                device_boot_id: slot.boot.clone().unwrap(),
                issued_execution_revision: uint(self.revision),
                issued_at_host_ms: uint(elapsed),
                expires_at_host_ms: uint(expires),
            });
        }
        Ok(!devices.is_empty())
    }
    pub fn device_views(&self, elapsed: u64) -> Vec<ExecutionDeviceView> {
        let observation = self.engine.external_observation().expect("owned external Engine");
        let mut views = self.binding_views(elapsed, &observation);
        self.devices.iter().map(|(device_id, slot)| {
            let latest = observation.attempts.iter().filter(|a| &a.command.device_id == device_id)
                .max_by_key(|a| self.attempts.get(&a.command.attempt.command_id)
                    .map(|ledger| ledger.published_at_host_ms).unwrap_or(0));
            let binding_id = latest.map(|attempt| {
                let next = self.engine.external_candidate_bindings(&attempt.command.attempt.logical_job_id)
                    .expect("original logical work");
                slot.bindings.iter().find(|b| next.contains(&b.binding_id))
                    .map(|b| b.binding_id.clone()).unwrap_or_else(|| attempt.command.attempt.selected_binding_id.clone())
            }).unwrap_or_else(|| slot.bindings[0].binding_id.clone());
            let index = views.iter().position(|view| view.binding.binding_id == binding_id).expect("frozen device binding");
            views.swap_remove(index)
        }).collect()
    }
    fn binding_views(&self, elapsed: u64, observation: &ExternalObservation) -> Vec<ExecutionDeviceView> {
        self.selected_binding_views(elapsed, observation, None)
    }
    fn selected_binding_views(&self, elapsed: u64, observation: &ExternalObservation, selected: Option<&[ObjectId]>) -> Vec<ExecutionDeviceView> {
        self.devices
            .iter()
            .filter(|(_, slot)| selected.is_none_or(|ids| slot.bindings.iter().any(|binding| ids.contains(&binding.binding_id))))
            .flat_map(|(device_id, slot)| {
                let fresh = slot
                    .latest
                    .as_ref()
                    .is_some_and(|v| elapsed <= v.observation_probe.expires_at_host_ms.value());
                let mut reasons = vec![];
                if !slot.operator_accept {
                    reasons.push("operator_disabled");
                }
                if slot.boot.is_none() {
                    reasons.push("device_boot_not_registered");
                }
                if !fresh {
                    reasons.push("fresh_observation_required");
                }
                if let Some(view) = &slot.latest {
                    if !view.observation.accept_new_work {
                        reasons.push("device_disabled_acceptance");
                    }
                    if !matches!(
                        view.observation.device_state,
                        ExecutionTelemetryObservationDeviceState::Idle
                    ) || view.observation.active_command_id.is_some()
                    {
                        reasons.push("device_not_idle");
                    }
                }
                let busy = observation.attempts
                    .iter()
                    .any(|a| {
                        &a.command.device_id == device_id
                            && matches!(
                                a.state,
                                ExternalAttemptState::Reserved
                                    | ExternalAttemptState::Published
                                    | ExternalAttemptState::Accepted
                                    | ExternalAttemptState::Started
                                    | ExternalAttemptState::ReconciliationRequired
                                    | ExternalAttemptState::Failed
                            )
                    });
                if busy {
                    reasons.push("original_attempt_unsettled");
                }
                slot.bindings.iter().filter(|binding| selected.is_none_or(|ids| ids.contains(&binding.binding_id))).map(|binding| ExecutionDeviceView {
                    binding: binding.clone(),
                    operator_accept_new_work: slot.operator_accept,
                    latest_observation: slot.latest.clone(),
                    freshness: if slot.latest.is_none() {
                        ExecutionDeviceViewFreshness::Unknown
                    } else if fresh {
                        ExecutionDeviceViewFreshness::Fresh
                    } else {
                        ExecutionDeviceViewFreshness::Stale
                    },
                    effective_accept_new_work: if fresh {
                        Some(reasons.is_empty())
                    } else {
                        None
                    },
                    blocking_reasons: reasons
                        .iter()
                        .map(|s| DeviceText::try_from((*s).to_owned()).unwrap())
                        .collect(),
                    registered_device_boot_id: slot.boot.clone(),
                    pending_observation_probe: slot.probe.clone(),
                }).collect::<Vec<_>>()
            })
            .collect()
    }
    fn reconcile_lifecycle(&mut self) -> Result<()> {
        let mut obs = self.observe()?;
        if obs.goal_met && !obs.dispatch_stopped {
            self.engine.cancel();
            obs = self.observe()?;
        }
        if obs.attempts.iter().any(|a| {
            matches!(
                a.state,
                ExternalAttemptState::Failed | ExternalAttemptState::ReconciliationRequired
            ) || !a.unapplied_receipts.is_empty()
        }) {
            self.phase = ExecutionSnapshotLifecycleState::NeedsResolution;
        } else if obs.drained {
            self.phase = if obs.goal_met {
                ExecutionSnapshotLifecycleState::Completed
            } else {
                ExecutionSnapshotLifecycleState::Stopped
            };
        } else if obs.dispatch_stopped {
            self.phase = ExecutionSnapshotLifecycleState::Draining;
        } else {
            self.phase = match self.dispatch_mode {
                DispatchMode::Prepared => ExecutionSnapshotLifecycleState::Prepared,
                DispatchMode::Running => ExecutionSnapshotLifecycleState::Running,
                DispatchMode::Paused => ExecutionSnapshotLifecycleState::DispatchPaused,
                DispatchMode::Stopped => ExecutionSnapshotLifecycleState::Draining,
                DispatchMode::Reconciliation => ExecutionSnapshotLifecycleState::NeedsResolution,
            };
        }
        Ok(())
    }
    pub fn control(
        &mut self,
        caller_session: ObjectId,
        request_id: ObjectId,
        expected_revision: u64,
        action: ControlAction,
        now: Instant,
    ) -> Result<ControlAck> {
        let request_key = (caller_session, request_id);
        let payload =
            serde_json::json!({"expected_execution_revision":expected_revision,"action":action});
        if let Some((old, ack)) = self.control_requests.get(&request_key) {
            return if old == &payload {
                Ok(ack.clone())
            } else {
                Err(fault(
                    ErrorCode::InvalidInput,
                    "Original control request has another payload",
                ))
            };
        }
        if expected_revision != self.revision {
            let mut error = fault(
                ErrorCode::RevisionConflict,
                "Execution revision changed before control admission",
            );
            error.expected_revision = Some(Revision(uint(expected_revision)));
            error.actual_revision = Some(Revision(uint(self.revision)));
            return Err(error);
        }
        if matches!(
            self.phase,
            ExecutionSnapshotLifecycleState::Completed
                | ExecutionSnapshotLifecycleState::Stopped
                | ExecutionSnapshotLifecycleState::Failed
        ) {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Settled executions have no production controls",
            ));
        }
        let elapsed = self.elapsed(now)?;
        self.advance_boundaries(elapsed, false)?;
        if self.is_terminal() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Execution reached its terminal boundary before control",
            ));
        }
        let next = self.next_revision()?;
        if self.clock_anchor.is_some() {
            self.engine
                .advance_external(uint(elapsed))
                .map_err(engine_error)?;
        }
        let applied = match &action {
            ControlAction::Run {} => {
                if !matches!(self.dispatch_mode, DispatchMode::Prepared) {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Run requires the prepared original session",
                    ));
                }
                self.engine.activate_external().map_err(engine_error)?;
                self.clock_anchor = Some(now);
                self.dispatch_mode = DispatchMode::Running;
                self.phase = ExecutionSnapshotLifecycleState::Running;
                true
            }
            ControlAction::PauseDispatch {} => {
                if matches!(self.dispatch_mode, DispatchMode::Paused) {
                    false
                } else if matches!(self.dispatch_mode, DispatchMode::Running) {
                    self.engine.pause();
                    self.dispatch_mode = DispatchMode::Paused;
                    self.phase = ExecutionSnapshotLifecycleState::DispatchPaused;
                    true
                } else {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Pause requires a running execution",
                    ));
                }
            }
            ControlAction::ResumeDispatch {} => {
                if matches!(self.dispatch_mode, DispatchMode::Running) {
                    false
                } else if matches!(
                    self.dispatch_mode,
                    DispatchMode::Paused | DispatchMode::Reconciliation
                ) {
                    if self.observe()?.attempts.iter().any(|a| {
                        matches!(
                            a.state,
                            ExternalAttemptState::Failed
                                | ExternalAttemptState::ReconciliationRequired
                        )
                    }) {
                        return Err(fault(
                            ErrorCode::InvalidInput,
                            "Original unresolved execution facts must be reconciled before resume",
                        ));
                    }
                    self.engine.resume();
                    self.dispatch_mode = DispatchMode::Running;
                    self.phase = ExecutionSnapshotLifecycleState::Running;
                    true
                } else {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Resume requires dispatch_paused",
                    ));
                }
            }
            ControlAction::StopDispatch {} => {
                if self.clock_anchor.is_none() {
                    self.engine.cancel();
                    self.dispatch_mode = DispatchMode::Stopped;
                    self.phase = ExecutionSnapshotLifecycleState::Stopped;
                    true
                } else {
                    self.engine.cancel();
                    self.dispatch_mode = DispatchMode::Stopped;
                    self.phase = ExecutionSnapshotLifecycleState::Draining;
                    true
                }
            }
            ControlAction::RequestReconciliation {} => {
                if self.clock_anchor.is_none() {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Reconciliation requires an activated execution",
                    ));
                }
                self.engine.pause();
                self.dispatch_mode = DispatchMode::Reconciliation;
                self.phase = ExecutionSnapshotLifecycleState::NeedsResolution;
                true
            }
            ControlAction::SetDeviceAcceptance {
                device_id,
                accept_new_work,
            } => {
                let slot = self.devices.get_mut(device_id).ok_or_else(|| {
                    fault(
                        ErrorCode::PermissionDenied,
                        "Device is outside this execution",
                    )
                })?;
                let changed = slot.operator_accept != *accept_new_work;
                slot.operator_accept = *accept_new_work;
                changed
            }
        };
        self.revision = next;
        if self.clock_anchor.is_some() {
            self.engine
                .advance_external(uint(elapsed))
                .map_err(engine_error)?;
            self.reconcile_lifecycle()?;
            self.refresh_probes(elapsed)?;
            self.withdraw_disabled_commands()?;
            self.schedule(elapsed)?;
            self.reconcile_lifecycle()?;
        }
        self.capture_frame()?;
        let ack = ControlAck {
            identity: self.identity.clone(),
            execution_revision: uint(self.revision),
            action,
            admission: if applied {
                ControlAdmission::Applied
            } else {
                ControlAdmission::AlreadyApplied
            },
        };
        self.control_requests
            .insert(request_key, (payload, ack.clone()));
        Ok(ack)
    }
    fn withdraw_disabled_commands(&mut self) -> Result<()> {
        let observation = self.observe()?;
        for attempt in &observation.attempts {
            let continuing = observation.jobs.iter().any(|job| job.id == attempt.command.attempt.logical_job_id && job.started_at_ms.is_some());
            let disabled = !self.devices[&attempt.command.device_id].operator_accept
                || matches!(self.phase, ExecutionSnapshotLifecycleState::Draining) && !continuing;
            let command_id = &attempt.command.attempt.command_id;
            if disabled && matches!(attempt.state, ExternalAttemptState::Published | ExternalAttemptState::Accepted)
                && !self.pending_cloud_work.iter().any(|w| matches!(w,CloudWorkItem::WithdrawCommand { command_id:id,.. } if id==command_id)) {
                let work_id = self.next_id("withdraw");
                self.pending_cloud_work.push(CloudWorkItem::WithdrawCommand { work_id,
                    identity: self.identity.clone(), command_id: command_id.clone() });
            }
        }
        Ok(())
    }
    fn schedule(&mut self, elapsed: u64) -> Result<()> {
        if !matches!(self.phase, ExecutionSnapshotLifecycleState::Running | ExecutionSnapshotLifecycleState::Draining) {
            return Ok(());
        }
        let original = self.observe()?;
        if original.drained {
            return Ok(());
        }
        let queued = original.queued_logical_jobs.clone();
        let mut current = original;
        for logical_job_id in queued {
            let continuing = current.jobs.iter().any(|job| job.id == logical_job_id && job.started_at_ms.is_some());
            if current.dispatch_stopped && !continuing { continue; }
            if current.attempts.iter().any(|a| {
                a.command.attempt.logical_job_id == logical_job_id
                    && !matches!(
                        a.state,
                        ExternalAttemptState::CancelledNotStarted
                            | ExternalAttemptState::WithdrawnUnpublished
                            | ExternalAttemptState::Completed
                    )
            }) {
                continue;
            }
            let compatible = self.engine.external_candidate_bindings(&logical_job_id).map_err(engine_error)?;
            let mut views = self.selected_binding_views(elapsed, &current, Some(&compatible));
            let plans = views.iter().map(|v| self.engine.external_candidate_plan(&logical_job_id, &v.binding.binding_id)
                .map(|p| (v.binding.binding_id.clone(), p)).map_err(engine_error)).collect::<Result<BTreeMap<_, _>>>()?;
            views.sort_by(|a,b| plans[&a.binding.binding_id].estimated_completion_ms.unwrap_or(MAX_SAFE_UINT)
                .cmp(&plans[&b.binding.binding_id].estimated_completion_ms.unwrap_or(MAX_SAFE_UINT))
                .then(a.binding.device_id.cmp(&b.binding.device_id))
                .then(a.binding.binding_id.cmp(&b.binding.binding_id)));
            let mut candidates = vec![];
            let mut selected = None;
            let ordinal = current
                .attempts
                .iter()
                .filter(|a| a.command.attempt.logical_job_id == logical_job_id)
                .map(|a| a.command.attempt.attempt_ordinal.value())
                .max()
                .unwrap_or(0)
                + 1;
            let mut eligible_rank = 0;
            for view in &views {
                let eligible = view.effective_accept_new_work == Some(true);
                if eligible {
                    eligible_rank += 1;
                }
                let mut candidate = DispatchCandidateView {
                    binding_id: view.binding.binding_id.clone(),
                    eligible,
                    blocking_reasons: view.blocking_reasons.iter().cloned().chain(plans[&view.binding.binding_id].blocking_reasons.iter().map(|s| DeviceText::try_from(s.clone()).unwrap())).collect(),
                    estimated_completion_host_ms: if eligible { plans[&view.binding.binding_id].estimated_completion_ms.map(uint) } else { None },
                    rank: if eligible {
                        Some(PositiveSafeUint::try_from(eligible_rank as u64).unwrap())
                    } else {
                        None
                    },
                };
                if eligible && selected.is_none() {
                    let command_id =
                        DispatchCommandId::try_from(self.next_id("command").as_str().to_owned())
                            .unwrap();
                    let attempt = AttemptRef {
                        logical_job_id: logical_job_id.clone(),
                        phase_id: view.binding.phase_id.clone(),
                        attempt_ordinal: SafeUint::try_from(ordinal)
                            .map_err(|s| fault(ErrorCode::InvalidInput, format!("{s:?}")))?,
                        command_id: command_id.clone(),
                        selected_binding_id: view.binding.binding_id.clone(),
                        phase_token: self.next_id("phase"),
                    };
                    match self.engine.prepare_external(
                        attempt,
                        view.registered_device_boot_id.clone().unwrap(),
                        uint(elapsed),
                    ) {
                        Ok(command) => {
                            // The command becomes non-reassignable before it can leave this owner.
                            self.engine
                                .mark_external_published(&command_id, uint(elapsed))
                                .map_err(engine_error)?;
                            selected = Some(command);
                        }
                        Err(e)
                            if matches!(
                                e.code,
                                "external.resources" | "external.material" | "external.phase" | "external.predecessor" | "external.output_capacity" | "external.preparation"
                            ) =>
                        {
                            eligible_rank -= 1;
                            candidate.eligible = false;
                            candidate.rank = None;
                            candidate.estimated_completion_host_ms = None;
                            candidate
                                .blocking_reasons
                                .push(DeviceText::try_from(e.code.to_owned()).unwrap());
                        }
                        Err(e) => return Err(engine_error(e)),
                    }
                }
                candidates.push(candidate);
            }
            let explanation = DispatchExplanation {
                based_on_host_admission_sequence: uint(self.admission_sequence),
                decision_host_ms: uint(elapsed),
                reason: if selected.is_none() {
                    DispatchExplanationReason::WaitingForEligibleBinding
                } else if ordinal > 1 && !continuing {
                    DispatchExplanationReason::ReselectAfterConfirmedWithdrawal
                } else if views.len() > 1
                    && plans[&views[0].binding.binding_id].estimated_completion_ms.is_some()
                    && plans[&views[0].binding.binding_id].estimated_completion_ms == plans[&views[1].binding.binding_id].estimated_completion_ms
                {
                    DispatchExplanationReason::StableBindingTiebreak
                } else {
                    DispatchExplanationReason::EarliestEstimatedCompletion
                },
                selected_binding_id: selected
                    .as_ref()
                    .map(|c| c.attempt.selected_binding_id.clone()),
                candidates,
            };
            // An unrelated idle report is not a new dispatch decision. Retain
            // the original waiting cause until eligibility/ranking changes;
            // otherwise the global admission sequence rewrites every queued
            // job and turns one device report into a factory-sized history.
            if selected.is_some() || self.dispatch_decisions.get(&logical_job_id).is_none_or(|old|
                old.selected_binding_id.is_some() || !equal(&old.reason, &explanation.reason)
                    || !equal(&old.candidates, &explanation.candidates)) {
                self.dispatch_decisions.insert(logical_job_id, explanation.clone());
            }
            if let Some(command) = selected {
                current = self.observe()?;
                let resource_plan = current
                    .attempts
                    .iter()
                    .find(|a| a.command.attempt.command_id == command.attempt.command_id)
                    .expect("just prepared original Engine attempt")
                    .reserved_resource_units
                    .clone();
                let reservation_id = self.next_id("reservation");
                self.attempts.insert(
                    command.attempt.command_id.clone(),
                    AttemptLedger {
                        reservation_id,
                        resource_plan,
                        command: command.clone(),
                        reserved_at_host_ms: elapsed,
                        published_at_host_ms: elapsed,
                        dispatch: explanation,
                    },
                );
                let work_id = self.next_id("publish-command");
                self.pending_cloud_work
                    .push(CloudWorkItem::PublishCommand { work_id, command });
            }
        }
        Ok(())
    }
    pub fn tick(&mut self, now: Instant) -> Result<()> {
        if self.clock_anchor.is_none() || self.is_terminal() {
            return Ok(());
        }
        let elapsed = self.elapsed(now)?;
        if elapsed == self.observe()?.elapsed_ms {
            return Ok(());
        }
        self.advance_boundaries(elapsed, false)?;
        if self.is_terminal() {
            return Ok(());
        }
        let next = self.next_revision()?;
        self.engine
            .advance_external(uint(elapsed))
            .map_err(engine_error)?;
        self.revision = next;
        self.reconcile_lifecycle()?;
        self.refresh_probes(elapsed)?;
        self.withdraw_disabled_commands()?;
        self.schedule(elapsed)?;
        self.capture_frame()
    }
    fn message_replay(
        &self,
        receipt_id: &ObjectId,
        payload: &Value,
    ) -> Result<Option<ReceiptAdmission>> {
        if let Some(old) = self.messages.get(receipt_id) {
            if &old.payload != payload {
                return Err(fault(
                    ErrorCode::InvalidInput,
                    "Receipt identity has another payload",
                ));
            }
            let mut duplicate = old.latest.clone();
            duplicate.decision = ReceiptAdmissionDecision::Duplicate;
            duplicate.business_effect_committed = false;
            return Ok(Some(duplicate));
        }
        Ok(None)
    }
    fn admit_message(
        &mut self,
        receipt_id: ObjectId,
        payload: Value,
        production: Option<DeviceReceipt>,
        decision: ReceiptAdmissionDecision,
        effect: bool,
        reason: Option<String>,
        elapsed: u64,
    ) -> ReceiptAdmission {
        self.admission_sequence += 1;
        let received = self
            .messages
            .get(&receipt_id)
            .map_or(elapsed, |m| m.first_received_elapsed_ms);
        let admission = ReceiptAdmission {
            receipt_id: receipt_id.clone(),
            decision,
            execution_revision: uint(self.revision),
            host_admission_sequence: uint(self.admission_sequence),
            host_received_elapsed_ms: uint(received),
            business_effect_committed: effect,
            reason: reason.map(|s| DeviceText::try_from(s).unwrap()),
        };
        self.admissions.push(admission.clone());
        self.messages.insert(
            receipt_id,
            AdmittedMessage {
                payload,
                production_receipt: production,
                latest: admission.clone(),
                first_received_elapsed_ms: received,
            },
        );
        admission
    }
    fn check_device_message(
        &self,
        session: &ExecutionSessionId,
        epoch: &ExecutionEpochId,
        device: &ObjectId,
        boot: &DeviceBootId,
        sequence: u64,
    ) -> Result<()> {
        if session != &self.identity.session_id
            || epoch != &self.identity.execution_epoch_id
            || self
                .devices
                .get(device)
                .is_none_or(|s| s.boot.as_ref() != Some(boot))
        {
            return Err(fault(
                ErrorCode::PermissionDenied,
                "Message is outside the original session/device/boot",
            ));
        }
        if self
            .device_sequences
            .contains_key(&(device.clone(), boot.clone(), sequence))
        {
            return Err(fault(
                ErrorCode::InvalidInput,
                "A boot sequence already belongs to another original cloud receipt",
            ));
        }
        Ok(())
    }
    pub fn apply_observation(
        &mut self,
        receipt_id: ObjectId,
        cloud_received_at: CloudTimestamp,
        observation: ExecutionTelemetryObservation,
        now: Instant,
    ) -> Result<ReceiptAdmission> {
        let payload = serde_json::json!({"kind":"telemetry","cloud_received_at":cloud_received_at,"observation":observation});
        if let Some(old) = self.message_replay(&receipt_id, &payload)? {
            return Ok(old);
        }
        self.check_device_message(
            &observation.session_id,
            &observation.execution_epoch_id,
            &observation.device_id,
            &observation.device_boot_id,
            observation.device_sequence.value(),
        )?;
        if let Some(progress) = &observation.phase_progress {
            if !matches!(observation.device_state, ExecutionTelemetryObservationDeviceState::Busy)
                || observation.active_command_id.is_none()
                || !self.devices[&observation.device_id].bindings.iter().any(|binding| binding.phase_id == progress.phase_id)
                || observation.active_command_id.as_ref().and_then(|id| self.attempts.get(id))
                    .is_some_and(|attempt| attempt.command.device_id != observation.device_id
                        || attempt.command.attempt.phase_id != progress.phase_id)
            {
                return Err(fault(ErrorCode::InvalidInput, "Reported phase requires a busy command and the bound device phase"));
            }
        }
        if self.clock_anchor.is_none() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Prepared observations cannot start production freshness",
            ));
        }
        if self.dataset_id.as_ref() != Some(&observation.dataset_id) {
            return Err(fault(
                ErrorCode::PermissionDenied,
                "Telemetry belongs to another dataset",
            ));
        }
        if self.is_terminal() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Execution observation window is settled; retain later raw telemetry in cloud",
            ));
        }
        let elapsed = self.elapsed(now)?;
        let previous_elapsed = self.projector.snapshot(&SnapshotSelection::Latest {})
            .expect("prepared execution projector has its latest frame")
            .coverage.observed_until_host_ms.value();
        let previous_revision = self.revision;
        self.advance_boundaries(elapsed, false)?;
        if self.is_terminal() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Execution observation window ended before this telemetry",
            ));
        }
        // One ingress page has one actual Host time. Unchanged readiness at that
        // same time cannot unlock work already scanned by its preceding admission.
        let schedule_needed = previous_elapsed != elapsed || previous_revision != self.revision
            || self.devices[&observation.device_id].latest.as_ref().is_none_or(|old| {
                elapsed > old.observation_probe.expires_at_host_ms.value()
                    || old.observation.accept_new_work != observation.accept_new_work
                    || std::mem::discriminant(&old.observation.device_state) != std::mem::discriminant(&observation.device_state)
                    || old.observation.active_command_id != observation.active_command_id
            });
        let probe = self.devices[&observation.device_id]
            .probe
            .clone()
            .filter(|p| {
                observation.observation_probe_id.as_ref() == Some(&p.probe_id)
                    && p.issued_at_host_ms.value() <= elapsed
                    && elapsed <= p.expires_at_host_ms.value()
            });
        let next = self.next_revision()?;
        self.engine
            .advance_external(uint(elapsed))
            .map_err(engine_error)?;
        self.revision = next;
        let admission = self.admit_message(
            receipt_id.clone(),
            payload,
            None,
            if probe.is_some() {
                ReceiptAdmissionDecision::Applied
            } else {
                ReceiptAdmissionDecision::Rejected
            },
            false,
            probe
                .is_none()
                .then(|| "Original post-run probe is missing or expired".to_owned()),
            elapsed,
        );
        self.device_sequences.insert(
            (
                observation.device_id.clone(),
                observation.device_boot_id.clone(),
                observation.device_sequence.value(),
            ),
            receipt_id.clone(),
        );
        if let Some(probe) = probe {
            let slot = self.devices.get_mut(&observation.device_id).unwrap();
            slot.latest = Some(HostObservedDevice {
                receipt_id,
                host_admission_sequence: admission.host_admission_sequence.clone(),
                host_received_elapsed_ms: uint(elapsed),
                observation,
                observation_probe: probe,
            });
            slot.probe = None;
        }
        self.reconcile_lifecycle()?;
        self.refresh_probes(elapsed)?;
        if schedule_needed {
            self.schedule(elapsed)?;
        }
        self.capture_frame()?;
        Ok(admission)
    }
    pub fn apply_receipt(
        &mut self,
        receipt_id: ObjectId,
        cloud_received_at: CloudTimestamp,
        receipt: DeviceReceipt,
        now: Instant,
    ) -> Result<ReceiptAdmission> {
        let payload = serde_json::json!({"kind":"receipt","cloud_received_at":cloud_received_at,"observation":receipt});
        if let Some(old) = self.message_replay(&receipt_id, &payload)? {
            return Ok(old);
        }
        self.check_device_message(
            &receipt.session_id,
            &receipt.execution_epoch_id,
            &receipt.device_id,
            &receipt.device_boot_id,
            receipt.device_sequence.value(),
        )?;
        if self.clock_anchor.is_none() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Prepared session cannot accept production receipts",
            ));
        }
        if self.is_terminal() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Settled execution cannot admit a new production receipt",
            ));
        }
        let elapsed = self.elapsed(now)?;
        self.advance_boundaries(elapsed, true)?;
        if self.is_terminal() {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Execution settled before this new production receipt",
            ));
        }
        let next = self.next_revision()?;
        let outcome = self
            .engine
            .apply_external_receipt(receipt.clone(), uint(elapsed));
        // A rejected message still has its real Host receive point. It cannot
        // place an admission beyond an older Engine observation frontier.
        if self.observe()?.elapsed_ms < elapsed {
            self.engine
                .advance_external(uint(elapsed))
                .map_err(engine_error)?;
        }
        self.revision = next;
        let (decision, effect, reason, committed) = match outcome {
            Ok(result) => {
                let pending = matches!(result.status, ExternalReceiptStatus::RetainedUnapplied);
                (
                    if result.duplicate {
                        ReceiptAdmissionDecision::Duplicate
                    } else if pending {
                        ReceiptAdmissionDecision::PendingDependency
                    } else {
                        ReceiptAdmissionDecision::Applied
                    },
                    !result.duplicate
                        && matches!(
                            result.status,
                            ExternalReceiptStatus::Started
                                | ExternalReceiptStatus::Completed
                                | ExternalReceiptStatus::Failed
                        ),
                    pending.then(|| "Original Started receipt is still required".to_owned()),
                    (!pending).then(|| uint(result.committed_at_ms)),
                )
            }
            Err(e) if e.code == "external.material_changed" => (
                ReceiptAdmissionDecision::PendingDependency,
                false,
                Some(e.to_string()),
                None,
            ),
            Err(e) => (
                ReceiptAdmissionDecision::Rejected,
                false,
                Some(e.to_string()),
                None,
            ),
        };
        let admission = self.admit_message(
            receipt_id.clone(),
            payload,
            Some(receipt.clone()),
            decision,
            effect,
            reason,
            elapsed,
        );
        self.receipt_evidence.push(HostReceiptEvidence {
            receipt: receipt.clone(),
            admission: admission.clone(),
            committed_at_host_ms: committed,
        });
        self.device_sequences.insert(
            (
                receipt.device_id.clone(),
                receipt.device_boot_id.clone(),
                receipt.device_sequence.value(),
            ),
            receipt_id,
        );
        self.reconcile_lifecycle()?;
        self.schedule(elapsed)?;
        self.capture_frame()?;
        // Each later dependency resolution is its own immutable revision, even
        // when it shares a millisecond with the newly admitted Started.
        let pending: Vec<_> = self
            .messages
            .iter()
            .filter_map(|(id, m)| {
                (matches!(admission.decision, ReceiptAdmissionDecision::Applied)
                    && matches!(receipt.receipt, DeviceReceiptKind::Started {})
                    && matches!(
                        m.latest.decision,
                        ReceiptAdmissionDecision::PendingDependency
                    )
                    && m.production_receipt
                        .as_ref()
                        .is_some_and(|r| r.command_id == receipt.command_id))
                .then_some((id.clone(), m.clone()))
            })
            .collect();
        for (id, m) in pending {
            let original = m.production_receipt.clone().unwrap();
            let next = self.next_revision()?;
            match self
                .engine
                .apply_external_receipt(original.clone(), uint(elapsed))
            {
                Ok(result)
                    if !matches!(result.status, ExternalReceiptStatus::RetainedUnapplied) =>
                {
                    self.revision = next;
                    let resolved = self.admit_message(
                        id,
                        m.payload,
                        Some(original.clone()),
                        ReceiptAdmissionDecision::Applied,
                        matches!(
                            result.status,
                            ExternalReceiptStatus::Started
                                | ExternalReceiptStatus::Completed
                                | ExternalReceiptStatus::Failed
                        ),
                        None,
                        elapsed,
                    );
                    self.receipt_evidence.push(HostReceiptEvidence {
                        receipt: original,
                        admission: resolved,
                        committed_at_host_ms: Some(uint(result.committed_at_ms)),
                    });
                    self.reconcile_lifecycle()?;
                    self.schedule(elapsed)?;
                    self.capture_frame()?;
                }
                Ok(_) => {}
                Err(e)
                    if matches!(
                        e.code,
                        "external.material_changed" | "external.sequence" | "external.state"
                    ) => {}
                Err(e) => return Err(engine_error(e)),
            }
        }
        Ok(admission)
    }
    /// Called only before Native leases its next request. Never change a request
    /// with an unknown network outcome; command and cutoff boundaries stay ordered.
    pub fn coalesce_pending_publications(&mut self) {
        let mut count = 0;
        for work in &self.pending_cloud_work {
            let CloudWorkItem::PublishSnapshot { publication, .. } = work else { break };
            count += 1;
            if publication.as_cutoff_snapshot { break; }
        }
        if count < 2 { return; }
        let mut group = self.pending_cloud_work.drain(..count);
        let CloudWorkItem::PublishSnapshot { work_id, mut publication } = group.next().unwrap() else { unreachable!() };
        for work in group {
            let CloudWorkItem::PublishSnapshot { publication: next, .. } = work else { unreachable!() };
            publication.snapshot = next.snapshot;
            publication.as_cutoff_snapshot = next.as_cutoff_snapshot;
            publication.observation_records.extend(next.observation_records);
        }
        self.pending_cloud_work.insert(0, CloudWorkItem::PublishSnapshot { work_id, publication });
    }
    pub fn cloud_work(&self) -> CloudWorkBatch {
        CloudWorkBatch {
            identity: self.identity.clone(),
            items: self.pending_cloud_work.clone(),
        }
    }
    pub fn acknowledge_cloud_work(&mut self, ack: CloudEffectAck, now: Instant) -> Result<()> {
        let id = ack.work_id().clone();
        let value = serde_json::to_value(&ack).unwrap();
        if let Some(old) = self.acknowledged_work.get(&id) {
            let canonical = |v: &Value| {
                let mut v = v.clone();
                match v["kind"].as_str() {
                    Some("command_stored") => {
                        v.as_object_mut().unwrap().remove("storage_status");
                    }
                    Some("snapshot_published") => {
                        v["ack"]
                            .as_object_mut()
                            .unwrap()
                            .remove("publication_status");
                    }
                    _ => {}
                }
                v
            };
            let already_settled = value["kind"] == "withdrawal_result"
                && value["state"] == "already_settled"
                && old["kind"] == value["kind"]
                && old["command_id"] == value["command_id"]
                && old["work_id"] == value["work_id"];
            return if canonical(old) == canonical(&value) || already_settled {
                Ok(())
            } else {
                Err(fault(
                    ErrorCode::InvalidInput,
                    "Cloud work acknowledgement payload changed",
                ))
            };
        }
        let index = self
            .pending_cloud_work
            .iter()
            .position(|w| w.work_id() == &id)
            .ok_or_else(|| {
                fault(
                    ErrorCode::MissingReference,
                    "Cloud work identity is not registered",
                )
            })?;
        let original = self.pending_cloud_work[index].clone();
        match (&original, &ack) {
            (
                CloudWorkItem::PublishCommand { command, .. },
                CloudEffectAck::CommandStored { command_id, .. },
            ) if command_id == &command.attempt.command_id => {}
            (
                CloudWorkItem::PublishSnapshot { publication, .. },
                CloudEffectAck::SnapshotPublished { ack, .. },
            ) if ack.session_id == self.identity.session_id
                && ack.execution_epoch_id == self.identity.execution_epoch_id
                && ack.execution_revision.value()
                    == publication.snapshot.execution_revision.value()
                && ack.observation_frontier.value()
                    == publication.snapshot.observation_frontier.value() =>
            {
                self.cloud_published_revision = Some(
                    self.cloud_published_revision
                        .unwrap_or(0)
                        .max(ack.execution_revision.value()),
                );
                // The projector retains the complete immutable history. Once cloud
                // storage is confirmed, keep only the current/cutoff publication
                // here rather than a second full copy of every factory frame.
                let confirmed = ack.execution_revision.value();
                let current = self.revision;
                self.publications.retain(|revision, publication| {
                    *revision > confirmed || *revision == current || publication.as_cutoff_snapshot
                });
            }
            (
                CloudWorkItem::WithdrawCommand { command_id, .. },
                CloudEffectAck::WithdrawalResult {
                    command_id: confirmed,
                    state,
                    ..
                },
            ) if confirmed == command_id => {
                if matches!(
                    state,
                    CloudResponseVariant0ResultWithdrawalDataState::WithdrawnBeforeDelivery
                ) {
                    let elapsed = self.elapsed(now)?;
                    self.advance_boundaries(elapsed, false)?;
                    let next = self.next_revision()?;
                    self.engine
                        .confirm_external_withdrawal(command_id, uint(elapsed))
                        .map_err(engine_error)?;
                    self.revision = next;
                    self.reconcile_lifecycle()?;
                    self.schedule(elapsed)?;
                    self.capture_frame()?;
                }
            }
            _ => {
                return Err(fault(
                    ErrorCode::InvalidInput,
                    "Cloud acknowledgement does not match the original work",
                ));
            }
        }
        self.pending_cloud_work.remove(index);
        self.acknowledged_work.insert(id, value);
        Ok(())
    }
    pub fn queue_publication(&mut self, publication: SnapshotPublication) -> Result<()> {
        if !equal(&publication.snapshot.identity, &self.identity)
            || publication.snapshot.execution_revision.value() != self.revision
        {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Publication must be the original current execution projection",
            ));
        }
        if let Some(old) = self.publications.get(&self.revision) {
            return if equal(old, &publication) {
                Ok(())
            } else {
                Err(fault(
                    ErrorCode::InvalidInput,
                    "A revision already has another publication",
                ))
            };
        }
        self.publications.insert(self.revision, publication.clone());
        let work_id = self.next_id("publish-snapshot");
        self.pending_cloud_work
            .push(CloudWorkItem::PublishSnapshot {
                work_id,
                publication,
            });
        Ok(())
    }
}
