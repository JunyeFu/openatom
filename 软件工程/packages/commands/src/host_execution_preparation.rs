//! H01 Task and frozen input ownership. Only the installed Native queue receives jobs.
use super::*;
use crate::execution_session::ExecutionSession;
use sf_domain::cloud_execution::*;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub struct ExecutionPreparationLease {
    host: ObjectId,
    epoch: u64,
    task: TaskId,
    token: Arc<()>,
}

#[cfg(test)]
mod tests {
    include!("host_execution_archive_tests.rs");
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicU64;
    static DIRECTORIES: AtomicU64 = AtomicU64::new(1);
    fn id(value: &str) -> ObjectId {
        value.to_owned().try_into().unwrap()
    }
    fn invoke(
        host: &mut ProjectHost,
        caller: &ObjectId,
        request: &str,
        operation: &str,
        params: Value,
        workspace: bool,
    ) -> Value {
        host.dispatch(caller, serde_json::from_value(json!({"api_version":"0.1","request_id":request,
            "project_id":if workspace { None } else { host.project.as_ref().map(|p| p.id.clone()) },
            "scope":{"kind":if workspace {"workspace"}else{"project"}},"operation":operation,"params":params})).unwrap())
    }
    fn setup(
        wrong_nominal: bool,
        with_runtime: bool,
    ) -> (
        ProjectHost,
        ObjectId,
        mpsc::Receiver<ExecutionPreparationJob>,
    ) {
        let ui = id("h01-ui");
        let mut host = ProjectHost::new(id("h01-native-host"), ui.clone());
        let actual_example =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../resources/v2-demo/project");
        let selected = host
            .select_directory(&ui, FilePurpose::ImportPackage, actual_example)
            .unwrap();
        let imported = invoke(
            &mut host,
            &ui,
            "import-example",
            "import_package",
            json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
            true,
        );
        assert_eq!(imported["outcome"]["status"], "ok", "{imported}");
        let task: TaskHandle = serde_json::from_value(imported["outcome"]["data"].clone()).unwrap();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let adopted = invoke(
            &mut host,
            &ui,
            "adopt-example",
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task.id,"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}),
            true,
        );
        assert_eq!(adopted["outcome"]["status"], "ok", "{adopted}");
        let mut candidates: Vec<CandidateBinding> = serde_json::from_str(include_str!(
            "../../../resources/v2-demo/candidate-bindings.json"
        ))
        .unwrap();
        if wrong_nominal {
            candidates[1].nominal_work_ms = 10000u64.try_into().unwrap();
        }
        let project = host.project.as_ref().unwrap().id.clone();
        let connection = CloudReadConnection::local_loopback(
            id("h01-private-connection"),
            project.clone(),
            "h01-cloud-project".to_owned().try_into().unwrap(),
            "http://127.0.0.1:9".into(),
            "finite-test-reader".into(),
            candidates,
        )
        .unwrap();
        host.install_cloud_read_connection(&ui, connection).unwrap();
        let (sender, receiver) = mpsc::channel();
        if with_runtime {
            host.install_execution_preparation_runtime(
                &ui,
                project,
                ExecutionPreparationRuntime::new(sender, id("h01-private-connection")),
            )
            .unwrap();
        }
        (host, ui, receiver)
    }
    fn start_params(host: &ProjectHost) -> Value {
        let project = host.project.as_ref().unwrap();
        let scenario = host
            .scenarios
            .values()
            .find(|s| s.id.as_str() == "Task8-six-units")
            .unwrap();
        let policy = json!({"clock_mode":"host_monotonic","report_cutoff_ms":120000,"stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish",
            "target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},"source_prediction_window":{"start_ms":0,"end_ms":scenario.window.end_ms},"observation_max_age_ms":3000});
        json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"base_revision":project.revision,"scenario_id":scenario.id,"scenario_revision":scenario.revision,
            "candidate_bindings":host.cloud_connections[&project.id].candidates,"policy":policy})
    }
    fn start(host: &mut ProjectHost, ui: &ObjectId, request: &str) -> StartupTaskHandle {
        let params = start_params(host);
        let response = invoke(host, ui, request, "start_execution_session", params, false);
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        serde_json::from_value(response["outcome"]["data"].clone()).unwrap()
    }
    fn task_state(host: &mut ProjectHost, ui: &ObjectId, task: &TaskId) -> Value {
        let response = invoke(
            host,
            ui,
            "inspect-original-task",
            "get_task",
            json!({"task_id":task}),
            false,
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        response["outcome"]["data"].clone()
    }
    fn cancel(host: &mut ProjectHost, ui: &ObjectId, task: &TaskId) -> Value {
        invoke(
            host,
            ui,
            "cancel-original-task",
            "cancel_task",
            json!({"task_id":task,"action":"cancel"}),
            false,
        )
    }
    fn persist(job: &ExecutionPreparationJob) -> PathBuf {
        let destination = std::env::temp_dir().join(format!(
            "sf-h01-frozen-{}-{}",
            std::process::id(),
            DIRECTORIES.fetch_add(1, Ordering::Relaxed)
        ));
        job.persist_frozen_inputs(&destination).unwrap();
        destination
    }
    /// Explicit finite callback input; this test does not claim C01/HTTP success.
    fn cloud_ack(job: &ExecutionPreparationJob) -> CloudResponseVariant0ResultSessionStoredData {
        serde_json::from_value(json!({"session_id":job.frozen.identity.session_id,"execution_epoch_id":job.frozen.identity.execution_epoch_id,"storage_status":"stored","dataset_id":"h01-component-dataset"})).unwrap()
    }
    fn acknowledge(host: &mut ProjectHost, gateway: &ExecutionGatewayLease) {
        for work in host.execution_gateway_cloud_work(gateway).unwrap().items {
            let CloudWorkItem::PublishSnapshot {
                work_id,
                publication,
            } = work
            else {
                panic!("Prepared execution has no production commands")
            };
            host.execution_gateway_acknowledge(
                gateway,
                CloudEffectAck::SnapshotPublished {
                    work_id,
                    ack: PublishAck {
                        session_id: publication.snapshot.identity.session_id,
                        execution_epoch_id: publication.snapshot.identity.execution_epoch_id,
                        execution_revision: publication.snapshot.execution_revision,
                        publication_status: PublishAckPublicationStatus::Stored,
                        observation_frontier: publication.snapshot.observation_frontier,
                    },
                },
                std::time::Instant::now(),
            )
            .unwrap();
        }
    }

    #[test]
    fn execution_media_reads_only_original_frozen_bytes_for_actual_ui_and_exact_identity_revision_asset()
     {
        let (mut host, ui, receiver) = setup(false, true);
        start(&mut host, &ui, "prepare-media");
        let job = receiver.try_recv().unwrap();
        host.execution_preparation_started(job.lease()).unwrap();
        job.prepare_engine().unwrap();
        persist(&job);
        host.execution_preparation_external_started(job.lease())
            .unwrap();
        let gateway = host
            .register_execution_preparation_core(job.lease(), cloud_ack(&job))
            .unwrap();
        let asset = &job.frozen.asset_bindings[0];
        let request = ExecutionMediaAssetReadRequest {
            project_id: job.frozen.identity.project_id.clone(),
            scope_revision: host.project.as_ref().unwrap().revision.clone(),
            identity: job.frozen.identity.clone(),
            execution_revision: 0u64.try_into().unwrap(),
            module_ref: asset.module_ref.clone(),
            asset: asset.asset.clone(),
        };
        let expected = job
            .files()
            .iter()
            .find(|f| exact_data(&f.file, &asset.archive_file))
            .unwrap()
            .bytes
            .clone();
        assert_eq!(
            host.read_execution_media_asset(&ui, &request).unwrap(),
            expected
        );
        host.modules.clear();
        assert_eq!(
            host.read_execution_media_asset(&ui, &request).unwrap(),
            expected,
            "Current draft modules do not own execution media"
        );
        assert!(
            host.read_execution_media_asset(&id("copied-ui"), &request)
                .is_err()
        );
        let mut bad = request.clone();
        bad.identity.execution_epoch_id = "other-epoch".to_owned().try_into().unwrap();
        assert!(host.read_execution_media_asset(&ui, &bad).is_err());
        let mut bad = request.clone();
        bad.execution_revision = 9999u64.try_into().unwrap();
        assert!(host.read_execution_media_asset(&ui, &bad).is_err());
        let mut bad = request.clone();
        bad.asset.path = "not-a-declared-asset.bin".to_owned().try_into().unwrap();
        assert!(host.read_execution_media_asset(&ui, &bad).is_err());
        let mut bad = request.clone();
        bad.module_ref.version = "99.0.0".to_owned().try_into().unwrap();
        assert!(host.read_execution_media_asset(&ui, &bad).is_err());
        let mut bad = request.clone();
        bad.scope_revision = Revision(9999u64.try_into().unwrap());
        assert!(host.read_execution_media_asset(&ui, &bad).is_err());
        assert!(
            !host
                .execution_gateway_snapshot(&gateway)
                .unwrap()
                .coverage
                .clock_started
        );
    }

    #[test]
    fn real_saved_demo_freezes_assets_and_transfers_one_engine_into_original_preparation_task() {
        let (mut host, ui, receiver) = setup(false, true);
        let params = start_params(&host);
        let task = start(&mut host, &ui, "prepare-once");
        let job = receiver.try_recv().unwrap();
        let same = invoke(
            &mut host,
            &ui,
            "prepare-once",
            "start_execution_session",
            params,
            false,
        );
        assert_eq!(same["outcome"]["data"]["id"], json!(task.id));
        assert!(receiver.try_recv().is_err());
        assert!(!job.frozen.asset_bindings.is_empty());
        let original_title = job.frozen.project.title.clone();
        host.execution_preparation_started(job.lease()).unwrap();
        assert!(
            host.execution_preparation_external_started(job.lease())
                .is_err()
        );
        job.prepare_engine().unwrap();
        job.prepare_engine().unwrap();
        assert!(
            host.execution_preparation_external_started(job.lease())
                .is_err()
        );
        let directory = persist(&job);
        job.persist_frozen_inputs(&directory).unwrap();
        for file in job.files() {
            assert_eq!(
                fs::read(directory.join(file.file.path.as_str())).unwrap(),
                file.bytes
            );
        }
        host.execution_preparation_external_started(job.lease())
            .unwrap();
        host.project.as_mut().unwrap().title = "draft-changed-after-freeze".into();
        let gateway = host
            .register_execution_preparation_core(job.lease(), cloud_ack(&job))
            .unwrap();
        assert!(
            job.prepare_engine().is_err(),
            "A transferred Engine cannot be built again"
        );
        assert_eq!(
            host.executions[&gateway.identity.session_id]
                .core
                .frozen
                .project
                .title,
            original_title
        );
        assert!(
            host.finish_execution_preparation(job.lease(), &gateway)
                .is_err()
        );
        for b in &job.frozen.candidate_bindings {
            host.execution_gateway_register_boot(
                &gateway,
                &b.device_id,
                format!("boot-{}", b.device_id.as_str()).try_into().unwrap(),
                std::time::Instant::now(),
            )
            .unwrap();
        }
        assert!(
            host.finish_execution_preparation(job.lease(), &gateway)
                .is_err()
        );
        acknowledge(&mut host, &gateway);
        host.finish_execution_preparation(job.lease(), &gateway)
            .unwrap();
        let state = task_state(&mut host, &ui, &task.id);
        let strict: ExecutionPreparationTaskState = serde_json::from_value(state.clone()).unwrap();
        assert!(matches!(strict.task.state, ComputationStatus::Completed));
        assert_eq!(strict.originating_request_id, id("prepare-once"));
        assert_eq!(state["result"]["kind"], "execution_session");
        let snapshot = host.execution_gateway_snapshot(&gateway).unwrap();
        assert!(!snapshot.coverage.clock_started);
        assert_eq!(snapshot.logical_work.len(), 6);
        assert!(snapshot.bound_jobs.is_empty());
        let cancelled = cancel(&mut host, &ui, &task.id);
        assert_eq!(cancelled["outcome"]["data"]["terminal_won"], true);
        assert!(!job.is_cancelled());
        let recovered = invoke(
            &mut host,
            &ui,
            "recover-prepare",
            "read_execution_view",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"request","originating_request_id":"prepare-once"}}),
            false,
        );
        assert_eq!(
            recovered["outcome"]["data"]["data"]["task"]["id"],
            json!(task.id)
        );
    }
    #[test]
    fn cancellation_before_external_admission_has_no_cloud_or_engine_effect() {
        let (mut host, ui, receiver) = setup(false, true);
        let task = start(&mut host, &ui, "cancel-before-remote");
        let job = receiver.try_recv().unwrap();
        cancel(&mut host, &ui, &task.id);
        assert!(job.is_cancelled());
        assert!(job.prepare_engine().is_err());
        host.finish_cancelled_execution_preparation(job.lease())
            .unwrap();
        let state = task_state(&mut host, &ui, &task.id);
        assert_eq!(state["task"]["state"], "cancelled");
        assert!(state["result"].is_null());
        assert!(host.executions.is_empty());
        assert!(host.desktop_close_state().active_tasks.is_empty());
    }
    #[test]
    fn unknown_cloud_admission_cannot_fail_or_discard_the_original_preparation() {
        let (mut host, ui, receiver) = setup(false, true);
        let params = start_params(&host);
        let task = start(&mut host, &ui, "unknown-c01");
        let job = receiver.try_recv().unwrap();
        host.execution_preparation_started(job.lease()).unwrap();
        job.prepare_engine().unwrap();
        persist(&job);
        host.execution_preparation_external_started(job.lease())
            .unwrap();
        cancel(&mut host, &ui, &task.id);
        assert!(
            host.finish_cancelled_execution_preparation(job.lease())
                .is_err()
        );
        assert!(
            host.fail_execution_preparation_before_remote(
                job.lease(),
                error(ErrorCode::IoFailure, "unknown network outcome", None)
            )
            .is_err()
        );
        let repeated = invoke(
            &mut host,
            &ui,
            "unknown-c01",
            "start_execution_session",
            params,
            false,
        );
        assert_eq!(repeated["outcome"]["data"]["id"], json!(task.id));
        assert!(receiver.try_recv().is_err());
        assert!(!host.desktop_close_state().active_tasks.is_empty());
        assert!(
            host.check_departure(&DepartureDecision::DiscardAfterTasksTerminal)
                .is_err()
        );
        let mut wrong = cloud_ack(&job);
        wrong.execution_epoch_id = "another-epoch".to_owned().try_into().unwrap();
        assert!(
            host.register_execution_preparation_core(job.lease(), wrong)
                .is_err()
        );
        let gateway = host
            .register_execution_preparation_core(job.lease(), cloud_ack(&job))
            .unwrap();
        host.stop_native_execution_preparation(&gateway, id("cleanup-original-preparation"))
            .unwrap();
        acknowledge(&mut host, &gateway);
        host.finish_native_execution_cleanup(&gateway).unwrap();
        host.finish_cancelled_execution_preparation(job.lease())
            .unwrap();
        assert_eq!(
            task_state(&mut host, &ui, &task.id)["task"]["state"],
            "cancelled"
        );
        assert!(host.desktop_close_state().active_tasks.is_empty());
        assert!(host.desktop_close_state().execution_blockers.is_empty());
    }
    #[test]
    fn actual_external_binding_mismatch_fails_before_the_cloud_boundary() {
        let (mut host, ui, receiver) = setup(true, true);
        let task = start(&mut host, &ui, "bad-native-nominal");
        let job = receiver.try_recv().unwrap();
        let failure = job
            .prepare_engine()
            .err()
            .expect("Actual B processing time is 15 seconds, not 10");
        assert!(
            host.execution_preparation_external_started(job.lease())
                .is_err()
        );
        host.fail_execution_preparation_before_remote(job.lease(), failure)
            .unwrap();
        let state = task_state(&mut host, &ui, &task.id);
        assert_eq!(state["task"]["state"], "failed");
        assert!(state["result"].is_null());
        assert!(host.executions.is_empty());
        serde_json::from_value::<ExecutionPreparationTaskState>(state).unwrap();
    }
    #[test]
    fn preparation_task_cannot_escape_strict_origin_kind_and_required_request_fields() {
        let (mut host, ui, _receiver) = setup(false, true);
        let task = start(&mut host, &ui, "strict-preparation");
        let state = task_state(&mut host, &ui, &task.id);
        serde_json::from_value::<TaskState>(state.clone()).unwrap();
        for field in ["originating_request_id", "result", "failure"] {
            let mut bad = state.clone();
            bad.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<TaskState>(bad).is_err(), "{field}");
        }
        let mut bad = state.clone();
        bad["originating_operation"] = json!("run_scenario");
        assert!(serde_json::from_value::<TaskState>(bad).is_err());
        let mut bad = state;
        bad["task"]["kind"] = json!("simulation");
        assert!(serde_json::from_value::<TaskState>(bad).is_err());
    }
    #[test]
    fn start_and_cloud_read_cannot_reuse_one_caller_request_id_for_two_tasks() {
        let (mut host, ui, receiver) = setup(false, true);
        let task = start(&mut host, &ui, "one-task-request");
        let _job = receiver.try_recv().unwrap();
        let second = invoke(
            &mut host,
            &ui,
            "one-task-request",
            "read_cloud_production",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"dataset_catalog"}}),
            false,
        );
        assert_eq!(second["outcome"]["data"]["code"], "InvalidInput");
        assert!(host.drain_io_work().is_empty());
        let mut changed = start_params(&host);
        changed["policy"]["report_cutoff_ms"] = json!(1);
        let conflict = invoke(
            &mut host,
            &ui,
            "one-task-request",
            "start_execution_session",
            changed,
            false,
        );
        assert_eq!(conflict["outcome"]["data"]["code"], "InvalidInput");
        assert_eq!(host.execution_preparation.requests.len(), 1);
        assert!(host.tasks.contains_key(&task.id));
    }
    #[test]
    fn standalone_host_does_not_advertise_or_fake_native_preparation() {
        let (mut host, ui, _receiver) = setup(false, false);
        assert!(!host.execution_preparation_available());
        let params = start_params(&host);
        let response = invoke(
            &mut host,
            &ui,
            "without-runtime",
            "start_execution_session",
            params,
            false,
        );
        assert_eq!(response["outcome"]["status"], "error");
        assert!(host.execution_preparation.requests.is_empty());
        assert!(host.executions.is_empty());
    }
    #[test]
    fn agent_creator_inherits_only_its_existing_h03_permission_for_its_own_session() {
        let (mut host, ui, receiver) = setup(false, true);
        let agent = id("h01-authorized-agent");
        host.register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: agent.clone(),
            granted_project: host.project.as_ref().map(|p| p.id.clone()),
            allowed_operations: vec![
                OperationName::StartExecutionSession,
                OperationName::ControlExecutionSession,
                OperationName::GetTask,
                OperationName::CancelTask,
                OperationName::GetExecutionSnapshot,
                OperationName::ReadExecutionView,
            ],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
        let task = start(&mut host, &agent, "agent-owned-preparation");
        let job = receiver.try_recv().unwrap();
        job.prepare_engine().unwrap();
        persist(&job);
        host.execution_preparation_external_started(job.lease())
            .unwrap();
        let gateway = host
            .register_execution_preparation_core(job.lease(), cloud_ack(&job))
            .unwrap();
        for b in &job.frozen.candidate_bindings {
            host.execution_gateway_register_boot(
                &gateway,
                &b.device_id,
                format!("agent-boot-{}", b.device_id.as_str())
                    .try_into()
                    .unwrap(),
                std::time::Instant::now(),
            )
            .unwrap();
        }
        acknowledge(&mut host, &gateway);
        host.finish_execution_preparation(job.lease(), &gateway)
            .unwrap();
        assert_eq!(
            task_state(&mut host, &agent, &task.id)["task"]["state"],
            "completed"
        );
        let view = invoke(
            &mut host,
            &agent,
            "read-own-session",
            "get_execution_snapshot",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":gateway.identity,"selection":{"kind":"latest"}}),
            false,
        );
        assert_eq!(view["outcome"]["status"], "ok", "{view}");
        assert!(
            view["outcome"]["data"]["access"]["allowed_session_controls"]
                .as_array()
                .unwrap()
                .iter()
                .any(|x| x == "run")
        );
        assert_eq!(
            view["outcome"]["data"]["access"]["device_acceptance_control_ids"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        let hidden = invoke(
            &mut host,
            &ui,
            "other-caller-recovery",
            "read_execution_view",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"request","originating_request_id":"agent-owned-preparation"}}),
            false,
        );
        assert_eq!(hidden["outcome"]["data"]["data"]["state"], "not_found");
    }
    #[test]
    fn actual_preparation_failure_is_terminal_only_after_external_cleanup_and_does_not_become_success()
     {
        let (mut host, ui, receiver) = setup(false, true);
        let task = start(&mut host, &ui, "native-bootstrap-failed");
        let job = receiver.try_recv().unwrap();
        job.prepare_engine().unwrap();
        persist(&job);
        host.execution_preparation_external_started(job.lease())
            .unwrap();
        let fault = || error(ErrorCode::IoFailure, "owned device bootstrap failed", None);
        assert!(
            host.finish_failed_execution_preparation(job.lease(), fault())
                .is_err()
        );
        let gateway = host
            .register_execution_preparation_core(job.lease(), cloud_ack(&job))
            .unwrap();
        assert!(
            host.finish_failed_execution_preparation(job.lease(), fault())
                .is_err()
        );
        host.stop_native_execution_preparation(&gateway, id("settle-bootstrap-fault"))
            .unwrap();
        acknowledge(&mut host, &gateway);
        host.finish_native_execution_cleanup(&gateway).unwrap();
        host.finish_failed_execution_preparation(job.lease(), fault())
            .unwrap();
        let result = task_state(&mut host, &ui, &task.id);
        assert_eq!(result["task"]["state"], "failed");
        assert!(result["result"].is_null());
        assert_eq!(result["failure"]["code"], "IoFailure");
        assert!(host.desktop_close_state().execution_blockers.is_empty());
    }
    #[test]
    fn selecting_one_demo_candidate_is_input_error_and_registers_no_task_or_native_job() {
        let (mut host, ui, receiver) = setup(false, true);
        let mut params = start_params(&host);
        params["candidate_bindings"].as_array_mut().unwrap().pop();
        let response = invoke(
            &mut host,
            &ui,
            "one-candidate",
            "start_execution_session",
            params,
            false,
        );
        assert_eq!(response["outcome"]["data"]["code"], "InvalidInput");
        assert!(receiver.try_recv().is_err());
        assert!(host.execution_preparation.requests.is_empty());
        assert!(host.executions.is_empty());
    }
    #[test]
    fn disconnected_native_queue_returns_the_original_failed_recoverable_task() {
        let (mut host, ui, receiver) = setup(false, true);
        drop(receiver);
        let task = start(&mut host, &ui, "disconnected-runtime");
        assert!(matches!(task.state, ComputationStatus::Failed));
        let state = task_state(&mut host, &ui, &task.id);
        serde_json::from_value::<ExecutionPreparationTaskState>(state).unwrap();
        let repeated = start(&mut host, &ui, "disconnected-runtime");
        assert_eq!(repeated.id, task.id);
        assert!(host.executions.is_empty());
    }
}
impl ExecutionPreparationLease {
    pub fn task_id(&self) -> &TaskId {
        &self.task
    }
}

pub struct FrozenExecutionFile {
    pub file: FileRef,
    pub bytes: Vec<u8>,
}

enum PreparationCore {
    Unbuilt,
    Ready(Box<ExecutionSession>),
    Transferred,
}

/// No serializer, clone or public constructor can create a second job/Engine.
pub struct ExecutionPreparationJob {
    lease: ExecutionPreparationLease,
    frozen: Arc<ExecutionFrozenInputs>,
    policy: ExecutionPolicy,
    connection_ref: ObjectId,
    files: Arc<Vec<FrozenExecutionFile>>,
    core: Arc<Mutex<PreparationCore>>,
    persisted_directory: Arc<Mutex<Option<PathBuf>>>,
    cancelled: Arc<AtomicBool>,
}
impl ExecutionPreparationJob {
    pub fn lease(&self) -> &ExecutionPreparationLease {
        &self.lease
    }
    pub fn frozen_inputs(&self) -> &ExecutionFrozenInputs {
        &self.frozen
    }
    pub fn files(&self) -> &[FrozenExecutionFile] {
        &self.files
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
    pub fn cloud_request(&self) -> CloudSessionCreate {
        CloudSessionCreate {
            contract_version: CloudSessionCreateContractVersion::SfExecution20Draft11,
            identity: self.frozen.identity.clone(),
            project_input_ref: self.frozen.project_input_ref.clone(),
            policy: self.policy.clone(),
            candidate_bindings: self.frozen.candidate_bindings.clone(),
            frozen_inputs: (*self.frozen).clone(),
        }
    }
    /// Native calls this outside the Host lock, before any external side effect.
    /// The retained slot survives a dropped worker and cannot rebuild after transfer.
    pub fn prepare_engine(&self) -> Result<(), ApiError> {
        if self.is_cancelled() {
            return Err(error(
                ErrorCode::Cancelled,
                "Execution preparation cancelled",
                None,
            ));
        }
        let mut slot = self.core.lock().expect("preparation Engine slot");
        match &*slot {
            PreparationCore::Ready(_) => Ok(()),
            PreparationCore::Transferred => Err(error(
                ErrorCode::InvalidInput,
                "Original preparation Engine was already transferred or retired",
                None,
            )),
            PreparationCore::Unbuilt => {
                let core = ExecutionSession::prepare_before_cloud(
                    (*self.frozen).clone(),
                    self.policy.clone(),
                    self.connection_ref.clone(),
                )?;
                *slot = PreparationCore::Ready(Box::new(core));
                Ok(())
            }
        }
    }
    /// Private Native destination only; no UI path or selected-file token enters.
    /// An incomplete directory stays incomplete and is never overwritten.
    pub fn persist_frozen_inputs(&self, destination: &Path) -> Result<(), ApiError> {
        let mut committed = self
            .persisted_directory
            .lock()
            .expect("preparation archive state");
        if let Some(original) = &*committed {
            return if original == destination {
                Ok(())
            } else {
                Err(error(
                    ErrorCode::InvalidInput,
                    "Frozen input directory is already bound",
                    None,
                ))
            };
        }
        if self.is_cancelled() {
            return Err(error(
                ErrorCode::Cancelled,
                "Execution preparation cancelled",
                None,
            ));
        }
        let io_failure = |_| {
            error(
                ErrorCode::IoFailure,
                "Could not commit the private frozen execution inputs",
                None,
            )
        };
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(io_failure)?;
        }
        fs::create_dir(destination).map_err(io_failure)?;
        for file in self.files.iter() {
            let path = destination.join(file.file.path.as_str());
            fs::create_dir_all(path.parent().expect("generated frozen file parent"))
                .map_err(io_failure)?;
            let mut writer = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(io_failure)?;
            writer.write_all(&file.bytes).map_err(io_failure)?;
            writer.sync_all().map_err(io_failure)?;
        }
        let mut marker = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination.join("execution-inputs.json"))
            .map_err(io_failure)?;
        marker
            .write_all(
                &serde_json::to_vec_pretty(&*self.frozen)
                    .expect("actual frozen execution serializes"),
            )
            .map_err(io_failure)?;
        marker.sync_all().map_err(io_failure)?;
        *committed = Some(destination.to_owned());
        Ok(())
    }
}

#[derive(Clone)]
pub struct ExecutionPreparationRuntime {
    sender: mpsc::Sender<ExecutionPreparationJob>,
    connection_ref: ObjectId,
}
impl ExecutionPreparationRuntime {
    /// Installed by the trusted Native adapter together with its real receiver.
    pub fn new(sender: mpsc::Sender<ExecutionPreparationJob>, connection_ref: ObjectId) -> Self {
        Self {
            sender,
            connection_ref,
        }
    }
}

struct RegisteredStart {
    payload: Value,
    task: TaskId,
}
struct PendingPreparation {
    token: Arc<()>,
    epoch: u64,
    owner: ObjectId,
    frozen: Arc<ExecutionFrozenInputs>,
    core: Arc<Mutex<PreparationCore>>,
    persisted_directory: Arc<Mutex<Option<PathBuf>>>,
    _files: Arc<Vec<FrozenExecutionFile>>,
    external_started: bool,
}
#[derive(Default)]
pub(super) struct PreparationRegistry {
    runtimes: BTreeMap<ProjectId, ExecutionPreparationRuntime>,
    requests: BTreeMap<(ObjectId, ObjectId), RegisteredStart>,
    pending: BTreeMap<TaskId, PendingPreparation>,
}

impl ProjectHost {
    /// Exact execution bytes are retained at H01 freeze. No draft, Run or path fallback.
    pub fn read_execution_media_asset(
        &self,
        ui: &ObjectId,
        request: &ExecutionMediaAssetReadRequest,
    ) -> Result<Vec<u8>, ApiError> {
        self.require_registered_ui(ui)?;
        let caller = &self.sessions[ui];
        self.admit(
            caller,
            &Some(request.project_id.clone()),
            &ApiScope::Project,
            &OperationName::ReadExecutionView,
        )?;
        check_revision(
            &request.scope_revision,
            &self.project.as_ref().unwrap().revision,
            "/scope_revision",
        )?;
        if request.identity.project_id != request.project_id {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Execution media belongs to another project",
                None,
            ));
        }
        let source=self.local_execution_source(&request.identity)?;
        source.snapshot(&SnapshotSelection::Revision {execution_revision:request.execution_revision.clone()})?;
        let binding=source.frozen().asset_bindings.iter().find(|binding|
            exact_data(&binding.module_ref,&request.module_ref)&&exact_data(&binding.asset,&request.asset))
            .ok_or_else(||error(ErrorCode::MissingReference,"Exact module and AssetRef are not declared in the execution freeze",None))?;
        let files=match source {
            host_execution::LocalExecutionSource::Live(_)=>self.execution_frozen_files(&request.identity)?,
            host_execution::LocalExecutionSource::Archive(source)=>source.files.clone(),
        };
        files.iter().find(|file|exact_data(&file.file,&binding.archive_file)).map(|file|file.bytes.clone())
            .ok_or_else(||error(ErrorCode::MissingReference,"Exact frozen execution asset bytes are missing",None))
    }

    pub(super) fn execution_frozen_files(&self,identity:&ExecutionIdentity)->Result<Arc<Vec<FrozenExecutionFile>>,ApiError> {
        self.execution_preparation.pending.values().find(|p|exact_data(&p.frozen.identity,identity)).map(|p|p._files.clone())
            .ok_or_else(||error(ErrorCode::MissingReference,"Original frozen execution files are unavailable",None))
    }


    pub(super) fn capture_execution_archives(&mut self)->Result<Vec<crate::execution_archive::ExecutionArchiveCapture>,ApiError> {
        use crate::execution_archive::{ExecutionArchiveCapture, same, fail, key};
        let project=self.project.as_ref().expect("admitted active project").id.clone();
        for entry in self.executions.values().filter(|e|e.core.identity.project_id==project) {
            self.execution_frozen_files(&entry.core.identity)?;
        }
        let tasks:Vec<_>=self.execution_preparation.pending.iter().filter(|(_,p)|p.frozen.identity.project_id==project).map(|(id,_)|id.clone()).collect();
        let mut captures=vec![];
        for task in tasks {
            let mut capture={
                let pending=&self.execution_preparation.pending[&task];
                let live=self.executions.get(&pending.frozen.identity.session_id).filter(|e|same(&e.core.identity,&pending.frozen.identity));
                let (snapshots,history,published)=if let Some(entry)=live {
                    (entry.core.projector.snapshots(),Some(entry.core.projector.history_document().map_err(|e|fail(&e.message))?),entry.core.cloud_published_revision.map(|r|vec![r.try_into().unwrap()]).unwrap_or_default())
                } else {
                    let core=pending.core.lock().expect("original preparation slot");
                    match &*core {
                        PreparationCore::Ready(core)=>(core.projector.snapshots(),Some(core.projector.history_document().map_err(|e|fail(&e.message))?),core.cloud_published_revision.map(|r|vec![r.try_into().unwrap()]).unwrap_or_default()),
                        PreparationCore::Unbuilt|PreparationCore::Transferred=>(vec![],None,vec![]),
                    }
                };
                ExecutionArchiveCapture {directory:String::new(),frozen:pending.frozen.clone(),snapshots,history,published,files:pending._files.clone(),inherited:None}
            };
            if let Some(old)=self.backing.as_ref().and_then(|a|a.execution_sources.get(&key(&capture.frozen.identity))) {
                if !same(old.record.frozen_inputs.as_ref(),capture.frozen.as_ref()) || old.files.len()!=capture.files.len()
                    || old.files.iter().any(|f|capture.files.iter().find(|n|same(&f.file,&n.file)).is_none_or(|n|n.bytes!=f.bytes)) {
                    return Err(fail("Original execution frozen bytes conflict with the saved source"));
                }
                // A retired preparation cannot erase history already saved as immutable data.
                if capture.snapshots.is_empty() && !old.record.snapshots.is_empty() {continue;}
                if !crate::execution_archive::snapshot_prefix(&old.record.snapshots,&capture.snapshots)
                    || old.record.history.as_ref().is_some_and(|h|capture.history.as_ref().is_none_or(|n|n.records.len()<h.records.len()||!same(h.records.as_slice(),&n.records[..h.records.len()]))) {
                    return Err(fail("Execution save would replace a different immutable observation prefix"));
                }
                capture.inherited=Some(old.record.files.clone());
                for revision in &old.record.published_revisions {if !capture.published.contains(revision){capture.published.push(revision.clone());}}
                capture.published.sort_by_key(|r|r.value());
                if old.record.snapshots.len()==capture.snapshots.len()&&crate::execution_archive::snapshot_prefix(&old.record.snapshots,&capture.snapshots)&&same(&old.record.history,&capture.history)&&same(&old.record.published_revisions,&capture.published){continue;}
            }
            capture.directory=self.next_document_directory("execution");
            captures.push(capture);
        }
        Ok(captures)
    }

    pub(super) fn validate_execution_archive_prefix(&self,archive:&ProjectArchive)->Result<(),ApiError> {
        use crate::execution_archive::{same,fail};
        for saved in archive.execution_sources.values() {
            let identity=&saved.record.frozen_inputs.identity;
            let Some(live)=self.executions.get(&identity.session_id).filter(|e|same(&e.core.identity,identity)) else {continue};
            let snapshots=live.core.projector.snapshots();
            let history=live.core.projector.history_document().map_err(|e|fail(&e.message))?;
            let bytes=self.execution_frozen_files(identity)?;
            if !same(saved.record.frozen_inputs.as_ref(),&live.core.frozen)
                || !crate::execution_archive::snapshot_prefix(&saved.record.snapshots,&snapshots)
                || saved.record.history.as_ref().is_some_and(|old|history.records.len()<old.records.len()||!same(old.records.as_slice(),&history.records[..old.records.len()]))
                || saved.files.len()!=bytes.len()||saved.files.iter().any(|old|bytes.iter().find(|n|same(&n.file,&old.file)).is_none_or(|n|n.bytes!=old.bytes))
                || saved.record.published_revisions.iter().any(|r|live.core.cloud_published_revision.is_none_or(|current|r.value()>current)) {
                return Err(fail("Saved execution is not a consistent prefix of the original live Engine"));
            }
        }
        Ok(())
    }

    pub fn install_execution_preparation_runtime(
        &mut self,
        ui: &ObjectId,
        project: ProjectId,
        runtime: ExecutionPreparationRuntime,
    ) -> Result<(), ApiError> {
        self.require_registered_ui(ui)?;
        let reader = self.cloud_connections.get(&project).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Install the matching private cloud configuration first",
                None,
            )
        })?;
        if self.project.as_ref().is_none_or(|p| p.id != project)
            || reader.connection_ref != runtime.connection_ref
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Native preparation runtime must match the active private cloud configuration",
                None,
            ));
        }
        self.execution_preparation.runtimes.insert(project, runtime);
        Ok(())
    }
    pub fn execution_preparation_available(&self) -> bool {
        self.project
            .as_ref()
            .is_some_and(|p| self.execution_preparation.runtimes.contains_key(&p.id))
    }
    pub(super) fn has_execution_start_request(&self, key: &(ObjectId, ObjectId)) -> bool {
        self.execution_preparation.requests.contains_key(key)
    }
    pub(super) fn registered_execution_start(
        &self,
        caller: &HostCallContext,
        request: &ObjectId,
    ) -> Result<Option<ExecutionRequestStatus>, ApiError> {
        let Some(registration) = self
            .execution_preparation
            .requests
            .get(&(caller.session_id.clone(), request.clone()))
        else {
            return Ok(None);
        };
        let task =
            current_task_state(self.owned_task(caller, &ApiScope::Project, &registration.task)?)
                .task;
        Ok(Some(ExecutionRequestStatus::RegisteredStart(
            RegisteredStartExecutionRequest {
                host_context_session_id: self.host_instance_id.clone(),
                state: RegisteredStartExecutionRequestState::Registered,
                operation: RegisteredStartExecutionRequestOperation::StartExecutionSession,
                task: StartupTaskHandle::try_from(task)
                    .expect("Actual preparation Task has its kind"),
                originating_request_id: request.clone(),
            },
        )))
    }
    pub(super) fn dispatch_execution_start(
        &mut self,
        caller_session: &ObjectId,
        request: HostStartRequest,
    ) -> Value {
        let request = request.into_inner();
        let request_id = request.request_id.clone();
        let scope = request.scope.clone();
        let result = (|| {
            let caller = self.sessions.get(caller_session).cloned().ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Unknown or revoked session",
                    None,
                )
            })?;
            self.admit(
                &caller,
                &request.project_id,
                &request.scope,
                &OperationName::StartExecutionSession,
            )?;
            let key = (caller_session.clone(), request.request_id.clone());
            let payload = serde_json::to_value(&request).expect("typed start request serializes");
            if let Some(original) = self.execution_preparation.requests.get(&key) {
                if original.payload != payload {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Original start request already has another payload",
                        None,
                    ));
                }
                return StartupTaskHandle::try_from(
                    current_task_state(self.owned_task(
                        &caller,
                        &ApiScope::Project,
                        &original.task,
                    )?)
                    .task,
                )
                .map_err(|_| {
                    error(
                        ErrorCode::InvalidInput,
                        "Original preparation Task kind differs",
                        None,
                    )
                });
            }
            if self.cloud_read_requests.contains_key(&key) {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Original request ID belongs to another Task operation",
                    None,
                ));
            }
            let project = self.project.as_ref().expect("admitted Project");
            check_revision(
                &Revision(request.params.base_revision.clone()),
                &project.revision,
                "/params/base_revision",
            )?;
            let scenario = self
                .scenarios
                .get(&request.params.scenario_id)
                .ok_or_else(|| {
                    error(ErrorCode::MissingReference, "Scenario is unavailable", None)
                })?;
            check_revision(
                &Revision(request.params.scenario_revision.clone()),
                &scenario.revision,
                "/params/scenario_revision",
            )?;
            if scenario.window.start_ms.0.value() != 0
                || request
                    .params
                    .policy
                    .source_prediction_window
                    .end_ms
                    .value()
                    != scenario.window.end_ms.0.value()
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Prediction window differs from the frozen Scenario",
                    None,
                ));
            }
            if !self.execution_blockers().is_empty()
                || self.execution_preparation.pending.keys().any(|id| {
                    self.tasks
                        .get(id)
                        .is_some_and(|t| !terminal(&t.state.task.state))
                })
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "An original preparation or execution still requires settlement",
                    None,
                ));
            }
            let runtime = self
                .execution_preparation
                .runtimes
                .get(&project.id)
                .ok_or_else(|| {
                    unsupported("Native execution preparation runtime is not installed")
                })?;
            let connection_ref = runtime.connection_ref.clone();
            let sender = runtime.sender.clone();
            let configured = &self.cloud_connections[&project.id];
            if configured.connection_ref != connection_ref {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Native execution runtime belongs to an older cloud configuration",
                    None,
                ));
            }
            if request.params.candidate_bindings.iter().map(|b| &b.device_id).collect::<std::collections::BTreeSet<_>>().len() < 2 {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Select at least two privately configured candidate devices covering the route operations",
                    None,
                ));
            }
            if request.params.candidate_bindings.iter().any(|b| {
                !configured
                    .candidates
                    .iter()
                    .any(|allowed| exact_data(allowed, b))
            }) {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Execution bindings must come from the exact private configuration",
                    None,
                ));
            }
            let (project, scenario, definitions) =
                self.frozen_project_execution(&request.params.scenario_id)?;
            let path = self.next_document_directory("execution");
            let task_id: TaskId =
                format!("{}:task:{}", self.host_instance_id.as_str(), self.next_task)
                    .try_into()
                    .unwrap();
            self.next_task += 1;
            let identity = ExecutionIdentity {
                session_id: format!(
                    "{}:execution:{}:{}",
                    self.host_instance_id.as_str(),
                    self.epoch,
                    self.next_task
                )
                .try_into()
                .unwrap(),
                execution_epoch_id: format!(
                    "{}:execution-epoch:{}:{}",
                    self.host_instance_id.as_str(),
                    self.epoch,
                    self.next_task
                )
                .try_into()
                .unwrap(),
                project_id: project.id.clone(),
                host_context_session_id: self.host_instance_id.clone(),
            };
            let input_ref = RunInputRef {
                project_id: project.id.clone(),
                project_revision: project.revision.clone(),
                scenario_id: scenario.id.clone(),
                scenario_revision: scenario.revision.clone(),
                frozen_project_file: file_ref(
                    &format!("{path}/project.json"),
                    DocumentKind::Project,
                ),
                frozen_scenario_file: file_ref(
                    &format!("{path}/scenario.json"),
                    DocumentKind::Scenario,
                ),
            };
            let mut files = vec![
                FrozenExecutionFile {
                    file: input_ref.frozen_project_file.clone(),
                    bytes: encode_document(&project),
                },
                FrozenExecutionFile {
                    file: input_ref.frozen_scenario_file.clone(),
                    bytes: encode_document(&scenario),
                },
            ];
            let mut modules = vec![];
            let mut assets = vec![];
            let mut references: Vec<_> = project
                .module_refs
                .iter()
                .chain(project.devices.iter().flat_map(|d| &d.route_operation_refs))
                .cloned()
                .collect();
            let mut seen = BTreeSet::new();
            while let Some(reference) = references.pop() {
                if !seen.insert(module_key(&reference)) {
                    continue;
                }
                let package = self.modules.get(&module_key(&reference)).ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Exact execution module dependency is not loaded",
                        None,
                    )
                })?;
                references.extend(package.dependencies.iter().cloned());
                let ordinal = modules.len();
                for (asset_ordinal, asset) in package.assets.iter().enumerate() {
                    let bytes = self.media_file_bytes(&reference, asset).ok_or_else(|| {
                        error(
                            ErrorCode::MissingReference,
                            "Declared execution asset bytes are missing",
                            None,
                        )
                    })?;
                    let file = file_ref(
                        &format!("{path}/assets/{ordinal:04}-{asset_ordinal:04}.bin"),
                        DocumentKind::Asset,
                    );
                    assets.push(ExecutionAssetBinding {
                        module_ref: reference.clone(),
                        asset: asset.clone(),
                        archive_file: file.clone(),
                    });
                    files.push(FrozenExecutionFile { file, bytes });
                }
                files.push(FrozenExecutionFile {
                    file: file_ref(
                        &format!("{path}/modules/{ordinal:04}.json"),
                        DocumentKind::Module,
                    ),
                    bytes: encode_document(package),
                });
                modules.push(package.clone());
            }
            let frozen = Arc::new(ExecutionFrozenInputs {
                document_kind: ExecutionFrozenInputsDocumentKind::ExecutionFrozenInputs,
                contract_version: ExecutionFrozenInputsContractVersion::SfExecution20Draft11,
                identity,
                project_input_ref: input_ref,
                scope: scenario.scope.clone(),
                project,
                scenario,
                modules,
                execution_definitions: definitions,
                candidate_bindings: request.params.candidate_bindings,
                asset_bindings: assets,
            });
            let token = Arc::new(());
            let core = Arc::new(Mutex::new(PreparationCore::Unbuilt));
            let files = Arc::new(files);
            let persisted_directory = Arc::new(Mutex::new(None));
            let cancelled = Arc::new(AtomicBool::new(false));
            let task = TaskHandle {
                id: task_id.clone(),
                scope: TaskScope::Edit(EditScope::Project(frozen.identity.project_id.clone())),
                kind: LongTaskKind::ExecutionPreparation,
                state: ComputationStatus::Queued,
                input_revision: frozen.project_input_ref.project_revision.clone(),
                progress: None,
                result_ref: None,
                failure: None,
            };
            self.tasks.insert(
                task_id.clone(),
                HostIoTask {
                    owner: caller_session.clone(),
                    epoch: self.epoch,
                    state: TaskState {
                        task: task.clone(),
                        originating_operation: OperationName::StartExecutionSession,
                        allowed_controls: vec![TaskControlAction::Cancel],
                        result: None,
                        failure: None,
                        originating_request_id: Some(request.request_id.clone()),
                    },
                    cancelled: cancelled.clone(),
                    started: Arc::new(AtomicBool::new(false)),
                    selection: None,
                    staged: None,
                    saved_content_generation: None,
                    progress: Arc::new(Mutex::new(None)),
                    packaging_input: None,
                },
            );
            self.execution_preparation.requests.insert(
                key,
                RegisteredStart {
                    payload,
                    task: task_id.clone(),
                },
            );
            self.execution_preparation.pending.insert(
                task_id.clone(),
                PendingPreparation {
                    token: token.clone(),
                    epoch: self.epoch,
                    owner: caller_session.clone(),
                    frozen: frozen.clone(),
                    core: core.clone(),
                    persisted_directory: persisted_directory.clone(),
                    _files: files.clone(),
                    external_started: false,
                },
            );
            let lease = ExecutionPreparationLease {
                host: self.host_instance_id.clone(),
                epoch: self.epoch,
                task: task_id.clone(),
                token,
            };
            self.mark_execution_content_changed(&frozen.identity);
            let job = ExecutionPreparationJob {
                lease,
                frozen,
                policy: request.params.policy,
                connection_ref,
                files,
                core,
                persisted_directory,
                cancelled,
            };
            if sender.send(job).is_err() {
                fail_task(
                    self.tasks.get_mut(&task_id).unwrap(),
                    error(
                        ErrorCode::IoFailure,
                        "The Native preparation queue is unavailable",
                        None,
                    ),
                );
            }
            StartupTaskHandle::try_from(current_task_state(&self.tasks[&task_id]).task).map_err(
                |_| {
                    error(
                        ErrorCode::InvalidInput,
                        "Preparation Task registration failed",
                        None,
                    )
                },
            )
        })();
        self.respond(request_id, caller_session, &scope, result)
    }
    fn preparation(
        &self,
        lease: &ExecutionPreparationLease,
    ) -> Result<&PendingPreparation, ApiError> {
        let pending = self
            .execution_preparation
            .pending
            .get(&lease.task)
            .ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Unknown Native preparation lease",
                    None,
                )
            })?;
        if lease.host != self.host_instance_id
            || lease.epoch != self.epoch
            || pending.epoch != self.epoch
            || !Arc::ptr_eq(&pending.token, &lease.token)
            || self
                .project
                .as_ref()
                .is_none_or(|p| p.id != pending.frozen.identity.project_id)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Native preparation lease no longer names its original Host/project",
                None,
            ));
        }
        Ok(pending)
    }
    pub fn execution_preparation_started(
        &mut self,
        lease: &ExecutionPreparationLease,
    ) -> Result<(), ApiError> {
        self.preparation(lease)?;
        let task = self.tasks.get_mut(&lease.task).unwrap();
        if terminal(&task.state.task.state) || task.cancelled.load(Ordering::Acquire) {
            return Err(error(
                ErrorCode::Cancelled,
                "Preparation is already terminal or cancelled",
                None,
            ));
        }
        task.started.store(true, Ordering::Release);
        task.state.task.state = ComputationStatus::Running;
        Ok(())
    }
    /// Native records the boundary before C01. Unknown results retain this Task.
    pub fn execution_preparation_external_started(
        &mut self,
        lease: &ExecutionPreparationLease,
    ) -> Result<(), ApiError> {
        let pending = self.preparation(lease)?;
        if pending
            .persisted_directory
            .lock()
            .expect("prepared archive")
            .is_none()
            || !matches!(
                *pending.core.lock().expect("prepared Engine"),
                PreparationCore::Ready(_)
            )
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Original inputs and Engine must be ready before Cloud C01",
                None,
            ));
        }
        let first=!pending.external_started;
        let identity=pending.frozen.identity.clone();
        let task = &self.tasks[&lease.task];
        if task.cancelled.load(Ordering::Acquire) || terminal(&task.state.task.state) {
            return Err(error(
                ErrorCode::Cancelled,
                "Preparation cancelled before external admission",
                None,
            ));
        }
        self.execution_preparation
            .pending
            .get_mut(&lease.task)
            .unwrap()
            .external_started = true;
        if first {self.mark_execution_content_changed(&identity);}
        Ok(())
    }
    /// C01 has returned the exact original typed identity/dataset. Transfer the
    /// already compiled Engine; changing the current draft cannot replace it.
    pub fn register_execution_preparation_core(
        &mut self,
        lease: &ExecutionPreparationLease,
        ack: CloudResponseVariant0ResultSessionStoredData,
    ) -> Result<ExecutionGatewayLease, ApiError> {
        let pending = self.preparation(lease)?;
        let identity = pending.frozen.identity.clone();
        if !pending.external_started
            || ack.session_id != identity.session_id
            || ack.execution_epoch_id != identity.execution_epoch_id
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Cloud admission does not match the original preparation",
                None,
            ));
        }
        if let Some(entry) = self.executions.get(&identity.session_id) {
            if !exact_data(&entry.core.identity, &identity)
                || entry.core.dataset_id.as_ref() != Some(&ack.dataset_id)
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Original cloud identity or dataset changed",
                    None,
                ));
            }
            return Ok(ExecutionGatewayLease {
                host_instance_id: self.host_instance_id.clone(),
                identity,
                token: entry.gateway_token.clone(),
            });
        }
        let slot = pending.core.clone();
        let mut prepared = slot.lock().expect("original prepared Engine");
        let PreparationCore::Ready(mut core) =
            std::mem::replace(&mut *prepared, PreparationCore::Transferred)
        else {
            return Err(error(
                ErrorCode::InvalidInput,
                "Original prepared Engine is not available",
                None,
            ));
        };
        core.dataset_id = Some(ack.dataset_id);
        let token = Arc::new(());
        let gateway = ExecutionGatewayLease {
            host_instance_id: self.host_instance_id.clone(),
            identity: identity.clone(),
            token: token.clone(),
        };
        self.executions.insert(
            identity.session_id,
            host_execution::HostedExecution {
                core: *core,
                gateway_token: token,
                cleanup_complete: false,
                preparation_ready: false,
                grants: BTreeMap::new(),
            },
        );
        self.mark_execution_content_changed(&gateway.identity);
        Ok(gateway)
    }
    pub fn finish_execution_preparation(
        &mut self,
        lease: &ExecutionPreparationLease,
        gateway: &ExecutionGatewayLease,
    ) -> Result<(), ApiError> {
        let pending = self.preparation(lease)?;
        if !exact_data(&pending.frozen.identity, &gateway.identity) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Preparation/Gateway identities differ",
                None,
            ));
        }
        let caller = self.sessions.get(&pending.owner).cloned().ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Original preparation caller was revoked",
                None,
            )
        })?;
        self.admit(
            &caller,
            &Some(pending.frozen.identity.project_id.clone()),
            &ApiScope::Project,
            &OperationName::StartExecutionSession,
        )?;
        if self.tasks[&lease.task].cancelled.load(Ordering::Acquire) {
            return Err(error(
                ErrorCode::Cancelled,
                "Original preparation was cancelled; settle its external work",
                None,
            ));
        }
        if terminal(&self.tasks[&lease.task].state.task.state) {
            return Ok(());
        }
        self.finish_native_execution_preparation(gateway)?;
        if matches!(caller.origin, CallerOrigin::ExternalAgent)
            && caller
                .allowed_operations
                .iter()
                .any(|op| matches!(op, OperationName::ControlExecutionSession))
        {
            let device_ids = self.executions[&gateway.identity.session_id]
                .core
                .devices
                .keys()
                .cloned()
                .collect();
            self.executions
                .get_mut(&gateway.identity.session_id)
                .unwrap()
                .grants
                .insert(
                    caller.session_id.clone(),
                    ExecutionControlGrant {
                        session_controls: vec![
                            ExecutionReadAccessAllowedSessionControlsItem::Run,
                            ExecutionReadAccessAllowedSessionControlsItem::PauseDispatch,
                            ExecutionReadAccessAllowedSessionControlsItem::ResumeDispatch,
                            ExecutionReadAccessAllowedSessionControlsItem::StopDispatch,
                            ExecutionReadAccessAllowedSessionControlsItem::RequestReconciliation,
                        ],
                        device_acceptance_control_ids: device_ids,
                    },
                );
        }
        let entry = &self.executions[&gateway.identity.session_id];
        let result = SessionAdmissionResult {
            identity: gateway.identity.clone(),
            state: SessionAdmissionResultState::Prepared,
            execution_revision: entry.core.revision.try_into().unwrap(),
        };
        let task = self.tasks.get_mut(&lease.task).unwrap();
        task.state.task.state = ComputationStatus::Completed;
        task.state.result = Some(TaskOutput::ExecutionSession(Box::new(result)));
        task.state.allowed_controls.clear();
        *task
            .progress
            .lock()
            .expect("preparation completion progress") = Some(TaskProgress {
            phase: "prepared".into(),
            processed: 1u64.try_into().unwrap(),
            total: Some(1u64.try_into().unwrap()),
            message: Some("冻结输入、设备boot和发布均已确认；尚未启动生产。".into()),
        });
        Ok(())
    }
    /// A failed/unknown C01 cannot use this pre-side-effect exit.
    pub fn fail_execution_preparation_before_remote(
        &mut self,
        lease: &ExecutionPreparationLease,
        failure: ApiError,
    ) -> Result<(), ApiError> {
        let pending = self.preparation(lease)?;
        if pending.external_started {
            return Err(error(
                ErrorCode::InvalidInput,
                "External preparation may exist; retain original identity and cleanup",
                None,
            ));
        }
        let slot = pending.core.clone();
        let task = self.tasks.get_mut(&lease.task).unwrap();
        if terminal(&task.state.task.state) {
            return Ok(());
        }
        fail_task(
            task,
            if task.cancelled.load(Ordering::Acquire) {
                error(
                    ErrorCode::Cancelled,
                    "Preparation cancelled before external admission",
                    None,
                )
            } else {
                failure
            },
        );
        *slot.lock().expect("original preparation slot") = PreparationCore::Transferred;
        Ok(())
    }
    pub fn finish_cancelled_execution_preparation(
        &mut self,
        lease: &ExecutionPreparationLease,
    ) -> Result<(), ApiError> {
        self.finish_settled_execution_preparation(
            lease,
            error(
                ErrorCode::Cancelled,
                "Original preparation cancelled and settled",
                None,
            ),
        )
    }
    pub fn finish_failed_execution_preparation(
        &mut self,
        lease: &ExecutionPreparationLease,
        failure: ApiError,
    ) -> Result<(), ApiError> {
        let failure = if self
            .tasks
            .get(&lease.task)
            .is_some_and(|t| t.cancelled.load(Ordering::Acquire))
        {
            error(
                ErrorCode::Cancelled,
                "Original preparation cancelled and settled",
                None,
            )
        } else {
            failure
        };
        self.finish_settled_execution_preparation(lease, failure)
    }
    fn finish_settled_execution_preparation(
        &mut self,
        lease: &ExecutionPreparationLease,
        failure: ApiError,
    ) -> Result<(), ApiError> {
        let pending = self.preparation(lease)?;
        if pending.external_started
            && self
                .executions
                .get(&pending.frozen.identity.session_id)
                .is_none_or(|e| !e.cleanup_complete)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Original external admission and cleanup are not settled",
                None,
            ));
        }
        let task = self.tasks.get_mut(&lease.task).unwrap();
        if terminal(&task.state.task.state) {
            return Ok(());
        }
        fail_task(task, failure);
        *task
            .progress
            .lock()
            .expect("preparation settlement progress") = Some(TaskProgress {
            phase: "settled".into(),
            processed: 1u64.try_into().unwrap(),
            total: Some(1u64.try_into().unwrap()),
            message: None,
        });
        Ok(())
    }
    pub fn execution_preparation_waiting(
        &mut self,
        lease: &ExecutionPreparationLease,
    ) -> Result<(), ApiError> {
        self.preparation(lease)?;
        let task = &self.tasks[&lease.task];
        if !terminal(&task.state.task.state) {
            *task.progress.lock().expect("original preparation progress") = Some(TaskProgress {
                phase: "awaiting_external_confirmation".into(),
                processed: 0u64.try_into().unwrap(),
                total: None,
                message: Some("原云端提交或设备结清结果尚未确认；保留原会话并继续核对。".into()),
            });
        }
        Ok(())
    }
}

impl ProjectHost {
    /// Native import checks the complete declared factory binding set before saving secrets.
    pub fn native_factory_bindings(&self, ui: &ObjectId, scenario_id: &ScenarioId) -> Result<Vec<CandidateBinding>, ApiError> {
        self.require_registered_ui(ui)?;
        if self.project.is_none() { return Err(error(ErrorCode::NoActiveProject, "请先打开工厂工程。", None)); }
        let (project, scenario, definitions) = self.frozen_project_execution(scenario_id)?;
        let input = sf_simulation::EngineInput {
            run_id: "native-binding-import".to_owned().try_into().unwrap(),
            input_ref: RunInputRef { project_id: project.id.clone(), project_revision: project.revision.clone(),
                scenario_id: scenario.id.clone(), scenario_revision: scenario.revision.clone(),
                frozen_project_file: file_ref("project.json", DocumentKind::Project),
                frozen_scenario_file: file_ref("scenario.json", DocumentKind::Scenario) },
            scope: scenario.scope.clone(), project, scenario, definitions,
        };
        sf_simulation::Engine::required_external_bindings(&input)
            .map_err(|e| error(ErrorCode::InvalidInput, &format!("工厂工艺尚不能绑定：{e}"), None))
    }
    pub fn validate_native_factory_bindings(&self, ui: &ObjectId, scenario_id: &ScenarioId,
        candidates: &[CandidateBinding]) -> Result<(), ApiError> {
        let expected = self.native_factory_bindings(ui, scenario_id)?;
        let normalized = |rows: &[CandidateBinding]| -> BTreeSet<String> { rows.iter().map(|row| {
            let mut value = serde_json::to_value(row).unwrap();
            value.as_object_mut().unwrap().remove("binding_id");
            serde_json::to_string(&value).unwrap()
        }).collect() };
        if normalized(candidates) != normalized(&expected) {
            return Err(error(ErrorCode::InvalidInput, "设备绑定与当前工厂方案不一致；请检查模型版本、工序阶段、端口、物料单位和资源，并补齐全部加工及运输绑定。", None));
        }
        Ok(())
    }

    pub(super) fn frozen_project_execution(
        &self,
        scenario_id: &ScenarioId,
    ) -> Result<(ProjectState, ScenarioInput, Vec<ModuleDefinition>), ApiError> {
        let project = self.project.as_ref().expect("admitted project");
        let scenario = self.scenarios.get(scenario_id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Current ScenarioInput does not exist",
                Some("/params/scenario_id"),
            )
        })?;
        let mut definitions = vec![];
        let mut definition_keys = BTreeSet::new();
        for reference in project.module_refs.iter().chain(
            project
                .devices
                .iter()
                .flat_map(|device| device.route_operation_refs.iter()),
        ) {
            if !definition_keys.insert(module_key(reference)) {
                continue;
            }
            let package = self.modules.get(&module_key(reference)).ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Exact module version is not loaded",
                    Some("/project/module_refs"),
                )
            })?;
            definitions.push(package.definition.clone());
        }
        let draft = self.execution_draft(scenario, &definitions);
        let blocking: Vec<_> = draft
            .project
            .issues
            .iter()
            .chain(scenario.issues.iter())
            .filter(|issue| {
                issue
                    .blocks
                    .iter()
                    .any(|purpose| matches!(purpose, CapabilityPurpose::Execution))
            })
            .cloned()
            .collect();
        if !blocking.is_empty() {
            return Err(engine_failure(blocking));
        }
        let packages: Vec<_> = self.modules.values().cloned().collect();
        for binding in current_consumer_bindings(&draft.project) {
            if !matches!(binding.mode, ProfessionalMode::CalculatedNominal) {
                continue;
            }
            let params = AdoptArtifactParams {
                artifact_id: binding.artifact_ref.clone(),
                acceptance_ref: binding.acceptance_ref.clone(),
                consumer_ref: binding.consumer_ref.clone(),
                operation_ref: binding.operation_ref.clone(),
                requested_scope: binding.requested_scope.clone(),
                base_revision: project.revision.clone(),
            };
            let trust = self.nominal_binding_trust(&draft, binding);
            let actual = prepare_artifact_binding(
                &draft,
                &packages,
                &params,
                binding.id.clone(),
                &trust,
                &self.providers,
            )?;
            if serde_json::to_value(&actual).unwrap() != serde_json::to_value(binding).unwrap() {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Stored nominal binding does not match the current full trusted consumer requirements",
                    Some("/project/bindings"),
                ));
            }
        }
        Ok((draft.project, scenario.clone(), definitions))
    }
}
