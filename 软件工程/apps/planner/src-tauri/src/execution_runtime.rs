//! Native-owned execution preparation and continuous gateway actor.
//! No IPC configuration, second Engine, child restart, PID reopening or inventory.
use crate::device_process::{
    self, ChildLifecycle, DeviceBinding, DeviceError, DeviceProfile, DeviceToken, LocalDeviceState,
    ManagedDevice, NativeDeviceRuntime, ProcessFacts,
};
use sf_commands::execution_cloud::{CloudIngressPage, ExecutionCloudConnection};
use sf_commands::{
    ApiError, CloudEffectAck, CloudWorkItem, ErrorCode, ExecutionGatewayLease,
    ExecutionPreparationJob, ExecutionPreparationRuntime, ProjectHost,
};
use sf_domain::cloud_execution::*;
use sf_domain::{DeviceBootId, ObjectId, SafeUint, TaskId};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const CADENCE: Duration = Duration::from_millis(100);
const UNKNOWN_DELAY: Duration = Duration::from_secs(5);
static IDS: AtomicU64 = AtomicU64::new(1);

/// Native secret storage supplies these values. Deliberately no Debug/Serialize.
pub struct NativeExecutionDevice {
    pub profile: DeviceProfile,
    pub token: DeviceToken,
}
/// Construct only in trusted Native setup, never from Renderer, Agent or project JSON.
pub struct NativeExecutionConfig {
    pub cloud: Arc<ExecutionCloudConnection>,
    pub devices: Arc<NativeDeviceRuntime>,
    pub connection_ref: ObjectId,
    pub bindings: BTreeMap<ObjectId, NativeExecutionDevice>,
    pub archive_root: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeExecutionPhase {
    Idle,
    Starting,
    LocalPreparation,
    CreatingSession,
    RegisteringCore,
    SpawningDevices,
    ConfirmingBoots,
    PublishingPrepared,
    Live,
    StoppingPreparation,
    Settling,
    ShuttingDown,
    RetiringBoots,
    ReleasingSlot,
    Finishing,
}
/// Native-only status. Facts are observations, never control authority or business receipts.
#[derive(Debug, Clone)]
pub struct NativeExecutionStatus {
    pub phase: NativeExecutionPhase,
    pub task_id: Option<TaskId>,
    pub identity: Option<ExecutionIdentity>,
    pub has_obligations: bool,
    pub last_error: Option<ApiError>,
    pub children: Vec<ProcessFacts>,
}
pub struct NativeExecutionActor {
    status: Arc<Mutex<NativeExecutionStatus>>,
    simulation: Arc<Mutex<Option<DeviceSimulationOverrides>>>,
    worker: JoinHandle<()>,
}
struct DeviceSimulationOverrides {
    identity: ExecutionIdentity,
    maintenance: BTreeMap<ObjectId, bool>,
}
impl NativeExecutionActor {
    pub fn status(&self) -> NativeExecutionStatus {
        self.status.lock().expect("Native execution status").clone()
    }
    pub fn is_finished(&self) -> bool {
        self.worker.is_finished()
    }
    pub fn request_device_maintenance(&self, identity: &ExecutionIdentity, device: &ObjectId, maintenance: bool) -> Result<(), ApiError> {
        let status = self.status();
        if status.phase != NativeExecutionPhase::Live
            || status.identity.as_ref().is_none_or(|active| !same_identity(active, identity))
            || !status.children.iter().any(|child| child.device_id == device.as_str() && child.lifecycle == ChildLifecycle::Running)
        {
            return Err(fault(ErrorCode::PermissionDenied, "Only a device owned by the current live Native software execution can be disturbed"));
        }
        let mut current = self.simulation.lock().expect("Native device simulation state");
        if current.as_ref().is_none_or(|value| !same_identity(&value.identity, identity)) {
            *current = Some(DeviceSimulationOverrides { identity: identity.clone(), maintenance: BTreeMap::new() });
        }
        current.as_mut().unwrap().maintenance.insert(device.clone(), maintenance);
        Ok(()) // A request; only a subsequent cloud observation changes Host eligibility.
    }
}

/// Return the real Host queue to install and the Native lifetime/status owner.
/// Dropping the status handle never cancels a Task, stops production or closes stdin.
pub fn start_execution_runtime(
    host: Arc<Mutex<ProjectHost>>,
    config: NativeExecutionConfig,
) -> Result<(ExecutionPreparationRuntime, NativeExecutionActor), ApiError> {
    if !config.archive_root.is_absolute()
        || config.bindings.len() < 2
        || (!config.bindings.values().all(|b| b.profile == DeviceProfile::FactoryCommands) && (!config
            .bindings
            .values()
            .any(|b| b.profile == DeviceProfile::ProcessorA)
        || !config
            .bindings
            .values()
            .any(|b| b.profile == DeviceProfile::ProcessorB)))
    {
        return Err(fault(
            ErrorCode::InvalidInput,
            "Native execution needs an absolute private archive and distinct trusted devices using factory or paired processor profiles",
        ));
    }
    let (sender, receiver) = mpsc::channel();
    let runtime = ExecutionPreparationRuntime::new(sender, config.connection_ref.clone());
    let status = Arc::new(Mutex::new(NativeExecutionStatus {
        phase: NativeExecutionPhase::Idle,
        task_id: None,
        identity: None,
        has_obligations: false,
        last_error: None,
        children: vec![],
    }));
    let shared = status.clone();
    let simulation = Arc::new(Mutex::new(None));
    let simulation_worker = simulation.clone();
    // Do not form an idle Arc cycle: Host owns the queue sender. An active job
    // retains the real Host strongly until every external obligation is settled.
    let idle_host = Arc::downgrade(&host);
    let worker = thread::Builder::new()
        .name("execution-native-actor".into())
        .spawn(move || {
            let mut io = NativeIo {
                config,
                children: BTreeMap::new(),
                simulation: simulation_worker,
                active_identity: None,
            };
            while let Ok(job) = receiver.recv() {
                let Some(host) = idle_host.upgrade() else {
                    break;
                };
                *io.simulation.lock().expect("Native device simulation state") = None;
                let mut current = ActiveJob::new(job, &io.config.archive_root);
                loop {
                    let started = Instant::now();
                    let result = current.step(&host, &mut io, started);
                    current.children = io.children.values_mut().map(ManagedDevice::poll).collect();
                    let (done, delay, error) = match result {
                        Ok(done) => (done, CADENCE.saturating_sub(started.elapsed()), None),
                        Err(error) => {
                            // After C01 starts, the original Task and all Child handles stay owned.
                            let error = if current.external_started && !current.prepared {
                                match with_host(&host, |h| {
                                    h.execution_preparation_waiting(current.job.lease())
                                }) {
                                    Ok(()) => error,
                                    Err(wait_error) => wait_error,
                                }
                            } else {
                                error
                            };
                            (false, UNKNOWN_DELAY, Some(error))
                        }
                    };
                    current.publish_status(&shared, error);
                    if done {
                        break;
                    }
                    // No Host/Engine guard survives step or this sleep.
                    thread::sleep(delay);
                }
                io.children.clear(); // step finishes only after observed clean exits, or before spawning.
                *shared.lock().expect("Native execution status") = NativeExecutionStatus {
                    phase: NativeExecutionPhase::Idle,
                    task_id: None,
                    identity: None,
                    has_obligations: false,
                    last_error: None,
                    children: vec![],
                };
            }
        })
        .map_err(|_| {
            fault(
                ErrorCode::IoFailure,
                "Could not start Native execution queue",
            )
        })?;
    Ok((runtime, NativeExecutionActor { status, simulation, worker }))
}

fn fault(code: ErrorCode, message: &str) -> ApiError {
    ApiError {
        code,
        message: message.into(),
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}
fn device_fault(error: DeviceError) -> ApiError {
    fault(
        ErrorCode::IoFailure,
        &format!("Owned device requires confirmation: {error}"),
    )
}
fn with_host<T>(
    host: &Arc<Mutex<ProjectHost>>,
    f: impl FnOnce(&mut ProjectHost) -> Result<T, ApiError>,
) -> Result<T, ApiError> {
    let mut host = host.lock().map_err(|_| {
        fault(
            ErrorCode::IoFailure,
            "Original Host lock is poisoned; retain execution ownership",
        )
    })?;
    f(&mut host)
}
fn same_identity(a: &ExecutionIdentity, b: &ExecutionIdentity) -> bool {
    a.session_id == b.session_id
        && a.execution_epoch_id == b.execution_epoch_id
        && a.project_id == b.project_id
        && a.host_context_session_id == b.host_context_session_id
}
fn unique_id(prefix: &str) -> String {
    format!(
        "{prefix}-{:x}-{:x}-{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system epoch")
            .as_nanos(),
        std::process::id(),
        IDS.fetch_add(1, Ordering::Relaxed)
    )
}
fn terminal(snapshot: &ExecutionSnapshot) -> bool {
    matches!(
        snapshot.lifecycle.state,
        ExecutionSnapshotLifecycleState::Completed
            | ExecutionSnapshotLifecycleState::Stopped
            | ExecutionSnapshotLifecycleState::Failed
    )
}

// Narrow private side-effect seam for finite component checks. The public
// constructor above always installs the real HTTPS/ManagedDevice implementation.
trait ExternalIo {
    fn verify(&mut self, bindings: &[CandidateBinding]) -> Result<(), ApiError>;
    fn create(
        &mut self,
        input: &CloudSessionCreate,
    ) -> Result<CloudResponseVariant0ResultSessionStoredData, ApiError>;
    fn active(&mut self) -> Result<ActiveExecutionSlot, ApiError>;
    fn spawn(
        &mut self,
        identity: &ExecutionIdentity,
        dataset: &sf_domain::CloudDatasetId,
        device: &ObjectId,
        boot: &DeviceBootId,
    ) -> Result<(), ApiError>;
    fn children(
        &mut self,
        desired: &BTreeMap<ObjectId, bool>,
        closing: bool,
    ) -> Result<Vec<ProcessFacts>, ApiError>;
    fn publish(&mut self, work: &CloudWorkItem) -> Result<CloudEffectAck, ApiError>;
    fn events(
        &mut self,
        identity: &ExecutionIdentity,
        after: SafeUint,
        frontier: Option<SafeUint>,
    ) -> Result<CloudIngressPage, ApiError>;
    fn retire(
        &mut self,
        identity: &ExecutionIdentity,
        device: &ObjectId,
        boot: &DeviceBootId,
        revision: SafeUint,
    ) -> Result<(), ApiError>;
    fn release(&mut self, input: &CloudReleaseRequest) -> Result<(), ApiError>;
}
struct NativeIo {
    config: NativeExecutionConfig,
    children: BTreeMap<ObjectId, ManagedDevice>,
    simulation: Arc<Mutex<Option<DeviceSimulationOverrides>>>,
    active_identity: Option<ExecutionIdentity>,
}
impl ExternalIo for NativeIo {
    fn verify(&mut self, bindings: &[CandidateBinding]) -> Result<(), ApiError> {
        self.config.devices.verify_imports().map_err(device_fault)?;
        if bindings.len() < 2
            || bindings
                .iter()
                .map(|b| &b.device_id)
                .collect::<BTreeSet<_>>()
                .len()
                < 2
        {
            return Err(fault(
                ErrorCode::InvalidInput,
                "Execution must freeze at least two trusted devices covering its route",
            ));
        }
        for binding in bindings {
            let profile = self
                .config
                .bindings
                .get(&binding.device_id)
                .ok_or_else(|| {
                    fault(
                        ErrorCode::InvalidInput,
                        "Frozen device has no Native token/profile",
                    )
                })?
                .profile;
            let expected = match profile {
                DeviceProfile::ProcessorA => 10000,
                DeviceProfile::ProcessorB => 15000,
                DeviceProfile::FactoryCommands => continue,
                DeviceProfile::TelemetryOnly => {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Telemetry-only profile cannot execute production",
                    ));
                }
            };
            if binding.nominal_work_ms.value() != expected {
                return Err(fault(
                    ErrorCode::InvalidInput,
                    "Frozen nominal duration differs from its trusted processor profile",
                ));
            }
        }
        Ok(())
    }
    fn create(
        &mut self,
        input: &CloudSessionCreate,
    ) -> Result<CloudResponseVariant0ResultSessionStoredData, ApiError> {
        self.config.cloud.create_session(input)
    }
    fn active(&mut self) -> Result<ActiveExecutionSlot, ApiError> {
        self.config.cloud.active_execution()
    }
    fn spawn(
        &mut self,
        identity: &ExecutionIdentity,
        dataset: &sf_domain::CloudDatasetId,
        device: &ObjectId,
        boot: &DeviceBootId,
    ) -> Result<(), ApiError> {
        self.active_identity = Some(identity.clone());
        let binding = &self.config.bindings[device];
        let child = self
            .config
            .devices
            .spawn(
                DeviceBinding {
                    host_context_session_id: identity.host_context_session_id.as_str().into(),
                    device_id: device.as_str().into(),
                    device_boot_id: boot.as_str().into(),
                    dataset_id: dataset.as_str().into(),
                    identity: Some(device_process::ExecutionIdentity {
                        session_id: identity.session_id.as_str().into(),
                        execution_epoch_id: identity.execution_epoch_id.as_str().into(),
                        project_id: identity.project_id.as_str().into(),
                        host_context_session_id: identity.host_context_session_id.as_str().into(),
                    }),
                },
                binding.profile,
                &binding.token,
                false,
            )
            .map_err(device_fault)?;
        // spawn returns ownership even if bootstrap fails after CreateProcess.
        self.children.insert(device.clone(), child);
        Ok(())
    }
    fn children(
        &mut self,
        desired: &BTreeMap<ObjectId, bool>,
        closing: bool,
    ) -> Result<Vec<ProcessFacts>, ApiError> {
        let mut facts = Vec::new();
        let maintenance = self.simulation.lock().expect("Native device simulation state")
            .as_ref().filter(|value| self.active_identity.as_ref().is_some_and(|active| same_identity(active, &value.identity)))
            .map(|value| value.maintenance.clone()).unwrap_or_default();
        for (id, child) in &mut self.children {
            let current = child.poll();
            if let Some(error) = current.fault {
                return Err(device_fault(error));
            }
            if matches!(
                current.lifecycle,
                ChildLifecycle::ExitedUnreconciled | ChildLifecycle::ChannelUncertain
            ) {
                return Err(device_fault(DeviceError::UnexpectedExit));
            }
            if current.lifecycle == ChildLifecycle::Running && !current.shutdown_acknowledged {
                let accept = desired.get(id).copied().unwrap_or(false);
                let result = if current
                    .status
                    .as_ref()
                    .is_some_and(|s| s.accept_commands != accept)
                {
                    child.set_accept_commands(accept)
                } else if !closing && current.status.as_ref().is_some_and(|s| s.maintenance != maintenance.get(id).copied().unwrap_or(false)) {
                    child.set_maintenance(maintenance.get(id).copied().unwrap_or(false))
                } else if closing
                    && current.status.as_ref().is_some_and(|s| {
                        s.registered
                            && s.state == LocalDeviceState::Idle
                            && !s.accept_commands
                            && s.shutdown_ready
                            && !s.pending_poll
                            && s.pending_events == 0
                            && !s.transport_unknown
                    })
                {
                    child.request_shutdown()
                } else {
                    child.request_status()
                };
                match result {
                    Ok(()) | Err(DeviceError::ControlPending) | Err(DeviceError::NotIdle) => {}
                    Err(error) => return Err(device_fault(error)),
                }
            }
            facts.push(current);
        }
        Ok(facts)
    }
    fn publish(&mut self, work: &CloudWorkItem) -> Result<CloudEffectAck, ApiError> {
        self.config.cloud.publish_work(work)
    }
    fn events(
        &mut self,
        identity: &ExecutionIdentity,
        after: SafeUint,
        frontier: Option<SafeUint>,
    ) -> Result<CloudIngressPage, ApiError> {
        self.config
            .cloud
            .events(identity, after, 200u64.try_into().unwrap(), frontier)
    }
    fn retire(
        &mut self,
        identity: &ExecutionIdentity,
        device: &ObjectId,
        boot: &DeviceBootId,
        revision: SafeUint,
    ) -> Result<(), ApiError> {
        self.config
            .cloud
            .retire_boot(identity, device, boot, revision)
            .map(|_| ())
    }
    fn release(&mut self, input: &CloudReleaseRequest) -> Result<(), ApiError> {
        self.config.cloud.release_slot(input).map(|_| ())
    }
}

struct PendingWork {
    work: CloudWorkItem,
    acknowledgement: Option<CloudEffectAck>,
}
#[derive(Default)]
struct Ingress {
    after: u64,
    frontier: Option<SafeUint>,
    rows: VecDeque<CloudEventRow>,
    page_end: Option<Option<SafeUint>>,
}
struct ActiveJob {
    job: ExecutionPreparationJob,
    identity: ExecutionIdentity,
    phase: NativeExecutionPhase,
    archive: PathBuf,
    cloud_input: CloudSessionCreate,
    cloud_ack: Option<CloudResponseVariant0ResultSessionStoredData>,
    gateway: Option<ExecutionGatewayLease>,
    boots: BTreeMap<ObjectId, DeviceBootId>,
    spawned: BTreeSet<ObjectId>,
    registered: BTreeSet<ObjectId>,
    external_started: bool,
    prepared: bool,
    failure: Option<ApiError>,
    pending_work: Option<PendingWork>,
    ingress: Ingress,
    published_revision: Option<SafeUint>,
    cleanup_revision: Option<SafeUint>,
    retire_queue: Option<VecDeque<(ObjectId, DeviceBootId)>>,
    children: Vec<ProcessFacts>,
    stop_request: ObjectId,
    cleanup_committed: bool,
}
impl ActiveJob {
    fn new(job: ExecutionPreparationJob, root: &std::path::Path) -> Self {
        let cloud_input = job.cloud_request();
        let identity = cloud_input.identity.clone();
        let boots = cloud_input
            .candidate_bindings
            .iter()
            .map(|b| (b.device_id.clone(), unique_id("boot").try_into().unwrap()))
            .collect();
        let stop_request = format!("native-stop:{}", job.lease().task_id().as_str())
            .try_into()
            .unwrap();
        Self {
            job,
            identity,
            phase: NativeExecutionPhase::Starting,
            archive: root.join(unique_id("execution")),
            cloud_input,
            cloud_ack: None,
            gateway: None,
            boots,
            spawned: BTreeSet::new(),
            registered: BTreeSet::new(),
            external_started: false,
            prepared: false,
            failure: None,
            pending_work: None,
            ingress: Ingress::default(),
            published_revision: None,
            cleanup_revision: None,
            retire_queue: None,
            children: vec![],
            stop_request,
            cleanup_committed: false,
        }
    }
    fn publish_status(&self, status: &Mutex<NativeExecutionStatus>, error: Option<ApiError>) {
        let mut status = status.lock().expect("Native execution status");
        let last_error = error.or_else(|| self.failure.clone());
        if self.prepared && (status.last_error.as_ref().map(|e| format!("{:?}:{}", e.code, e.message))
            != last_error.as_ref().map(|e| format!("{:?}:{}", e.code, e.message))) {
            // Keep one private diagnostic for the observed stalled gateway. Never
            // include credentials, URLs, request bodies, or a growing per-poll log.
            let diagnostic = serde_json::json!({"phase": format!("{:?}", self.phase), "error": last_error});
            if let Err(error) = std::fs::write(self.archive.join("native-runtime-status.json"), diagnostic.to_string()) {
                eprintln!("Cannot persist Native gateway diagnostic: {error}");
            }
        }
        *status = NativeExecutionStatus {
            phase: self.phase,
            task_id: Some(self.job.lease().task_id().clone()),
            identity: Some(self.identity.clone()),
            has_obligations: true,
            last_error,
            children: self.children.clone(),
        };
    }
    fn before_remote_failure(
        &mut self,
        host: &Arc<Mutex<ProjectHost>>,
        error: ApiError,
    ) -> Result<bool, ApiError> {
        with_host(host, |h| {
            h.fail_execution_preparation_before_remote(self.job.lease(), error)
        })?;
        Ok(true)
    }
    fn stop_for(&mut self, error: ApiError) {
        self.failure.get_or_insert(error);
        self.phase = NativeExecutionPhase::StoppingPreparation;
    }
    fn gateway(&self) -> &ExecutionGatewayLease {
        self.gateway
            .as_ref()
            .expect("original transferred Gateway lease")
    }
    fn snapshot(&self, host: &Arc<Mutex<ProjectHost>>) -> Result<ExecutionSnapshot, ApiError> {
        with_host(host, |h| h.execution_gateway_snapshot(self.gateway()))
    }
    fn confirm_slot(&self, slot: &ActiveExecutionSlot) -> Result<(), ApiError> {
        let slot = slot.active_slot.as_ref().ok_or_else(|| {
            fault(
                ErrorCode::InvalidInput,
                "Original cloud execution slot is not confirmed",
            )
        })?;
        if slot.session_id != self.identity.session_id
            || slot.execution_epoch_id != self.identity.execution_epoch_id
            || slot.owner_host_context != self.identity.host_context_session_id
        {
            return Err(fault(
                ErrorCode::PermissionDenied,
                "Cloud slot belongs to another original session/epoch/Host",
            ));
        }
        if slot.device_boots.iter().any(|b| {
            matches!(b.state, DeviceBootState::Active)
                && (self.boots.get(&b.device_id) != Some(&b.device_boot_id)
                    || !self.spawned.contains(&b.device_id)
                    || b.session_id != self.identity.session_id
                    || b.execution_epoch_id != self.identity.execution_epoch_id)
        }) {
            return Err(fault(
                ErrorCode::PermissionDenied,
                "Cloud slot has an unowned active boot; keep original ownership",
            ));
        }
        Ok(())
    }
    fn slot_ready(&self, slot: &ActiveExecutionSlot) -> bool {
        slot.active_slot.as_ref().is_some_and(|active| {
            matches!(active.state, ActiveExecutionSlotActiveSlotState::Active)
                && self.boots.iter().all(|(id, boot)| {
                    active.device_boots.iter().any(|b| {
                        b.device_id == *id
                            && b.device_boot_id == *boot
                            && matches!(b.state, DeviceBootState::Active)
                    })
                })
        })
    }
    fn flush_one(
        &mut self,
        host: &Arc<Mutex<ProjectHost>>,
        io: &mut impl ExternalIo,
        _now: Instant,
    ) -> Result<bool, ApiError> {
        if self.pending_work.is_none() {
            if let Some(work) = with_host(host, |h| h.execution_gateway_next_cloud_work(self.gateway()))? {
                self.pending_work = Some(PendingWork {
                    work,
                    acknowledgement: None,
                });
            } else {
                return Ok(true);
            }
        }
        let pending = self.pending_work.as_mut().unwrap();
        if pending.acknowledgement.is_none() {
            pending.acknowledgement = Some(io.publish(&pending.work)?);
        }
        let ack = pending.acknowledgement.as_ref().unwrap().clone();
        with_host(host, |h| {
            h.execution_gateway_acknowledge(self.gateway(), ack.clone(), Instant::now())
        })?;
        if let CloudEffectAck::SnapshotPublished { ack, .. } = ack {
            if self
                .published_revision
                .as_ref()
                .is_none_or(|r| r.value() < ack.execution_revision.value())
            {
                self.published_revision = Some(ack.execution_revision);
            }
        }
        self.pending_work = None;
        Ok(false)
    }
    fn receive_frontier(
        &mut self,
        host: &Arc<Mutex<ProjectHost>>,
        io: &mut impl ExternalIo,
        now: Instant,
    ) -> Result<(), ApiError> {
        loop {
            self.receive_one(host, io, now)?;
            if self.ingress.frontier.is_none() {
                return Ok(());
            }
        }
    }
    fn receive_one(
        &mut self,
        host: &Arc<Mutex<ProjectHost>>,
        io: &mut impl ExternalIo,
        _now: Instant,
    ) -> Result<(), ApiError> {
        if self.ingress.rows.is_empty() {
            if let Some(next) = self.ingress.page_end.take() {
                if let Some(next) = next {
                    if next.value() != self.ingress.after {
                        return Err(fault(
                            ErrorCode::IoFailure,
                            "Original event page cursor was not fully admitted",
                        ));
                    }
                } else {
                    self.ingress.after = self
                        .ingress
                        .frontier
                        .take()
                        .expect("fixed event frontier")
                        .value();
                    return Ok(());
                }
            }
            let page = io.events(
                &self.identity,
                self.ingress.after.try_into().unwrap(),
                self.ingress.frontier.clone(),
            )?;
            if !same_identity(&page.page.identity, &self.identity) {
                return Err(fault(
                    ErrorCode::PermissionDenied,
                    "Events belong to another original execution",
                ));
            }
            self.ingress.frontier = Some(page.visible_frontier);
            self.ingress.page_end = Some(page.page.next_ingress_id);
            self.ingress.rows = page.page.rows.into();
        }
        let Some(row) = self.ingress.rows.front().cloned() else {
            return Ok(());
        };
        let position = match row {
            CloudEventRow::Receipt {
                receipt_id,
                ingress_id,
                cloud_received_at,
                observation,
            } => {
                let result = with_host(host, |h| {
                    h.execution_gateway_receipt(
                        self.gateway(),
                        receipt_id,
                        cloud_received_at,
                        observation,
                        Instant::now(),
                    )
                })?;
                if matches!(result.decision, ReceiptAdmissionDecision::Rejected) {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Host rejected original production receipt; retain cursor for reconciliation",
                    ));
                }
                ingress_id
            }
            CloudEventRow::Telemetry {
                receipt_id,
                ingress_id,
                cloud_received_at,
                observation,
            } => {
                let snapshot = self.snapshot(host)?;
                if snapshot.coverage.clock_started && !terminal(&snapshot) {
                    match with_host(host, |h| {
                        h.execution_gateway_observation(
                            self.gateway(),
                            receipt_id,
                            cloud_received_at,
                            observation,
                            Instant::now(),
                        )
                    }) {
                        Ok(_) => {}
                        Err(error) => {
                            let latest = self.snapshot(host)?;
                            if latest.coverage.clock_started && !terminal(&latest) {
                                return Err(error);
                            }
                        }
                    }
                } // prepared/terminal telemetry stays raw in cloud; no fake freshness.
                ingress_id
            }
        };
        self.ingress.after = position.value();
        self.ingress.rows.pop_front();
        Ok(())
    }
    fn step(
        &mut self,
        host: &Arc<Mutex<ProjectHost>>,
        io: &mut impl ExternalIo,
        now: Instant,
    ) -> Result<bool, ApiError> {
        use NativeExecutionPhase::*;
        if !self.prepared
            && self.job.is_cancelled()
            && matches!(
                self.phase,
                SpawningDevices | ConfirmingBoots | PublishingPrepared
            )
        {
            self.stop_for(fault(
                ErrorCode::Cancelled,
                "Original preparation was cancelled",
            ));
        }
        match self.phase {
            Starting => {
                if let Err(error) =
                    with_host(host, |h| h.execution_preparation_started(self.job.lease()))
                {
                    return self.before_remote_failure(host, error);
                }
                self.phase = LocalPreparation;
            }
            LocalPreparation => {
                let result = io
                    .verify(&self.cloud_input.candidate_bindings)
                    .and_then(|_| self.job.prepare_engine())
                    .and_then(|_| self.job.persist_frozen_inputs(&self.archive));
                if let Err(error) = result {
                    return self.before_remote_failure(host, error);
                }
                if let Err(error) = with_host(host, |h| {
                    h.execution_preparation_external_started(self.job.lease())
                }) {
                    return self.before_remote_failure(host, error);
                }
                self.external_started = true;
                self.phase = CreatingSession;
            }
            CreatingSession => {
                self.cloud_ack = Some(io.create(&self.cloud_input)?);
                self.phase = RegisteringCore;
            }
            RegisteringCore => {
                self.gateway = Some(with_host(host, |h| {
                    h.register_execution_preparation_core(
                        self.job.lease(),
                        self.cloud_ack.as_ref().unwrap().clone(),
                    )
                })?);
                self.phase = SpawningDevices;
            }
            SpawningDevices => {
                if let Some((id, boot)) = self
                    .boots
                    .iter()
                    .find(|(id, _)| !self.spawned.contains(*id))
                {
                    let id = id.clone();
                    let boot = boot.clone();
                    match io.spawn(
                        &self.identity,
                        &self.cloud_ack.as_ref().unwrap().dataset_id,
                        &id,
                        &boot,
                    ) {
                        Ok(()) => {
                            self.spawned.insert(id);
                        }
                        Err(error) => self.stop_for(error),
                    }
                } else {
                    self.phase = ConfirmingBoots;
                }
            }
            ConfirmingBoots => {
                match io.children(&BTreeMap::new(), false) {
                    Ok(children) => self.children = children,
                    Err(error) => {
                        self.stop_for(error);
                        return Ok(false);
                    }
                }
                let slot = io.active()?;
                self.confirm_slot(&slot)?;
                let active = slot.active_slot.as_ref().unwrap();
                if !matches!(active.state, ActiveExecutionSlotActiveSlotState::Active) {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Original active slot needs reconciliation",
                    ));
                }
                if self.children.len() != self.boots.len()
                    || self.children.iter().any(|c| {
                        c.lifecycle != ChildLifecycle::Running
                            || !c.status.as_ref().is_some_and(|s| s.registered)
                    })
                {
                    return Ok(false);
                }
                for (id, boot) in &self.boots {
                    if !self.children.iter().any(|c| {
                        c.device_id == id.as_str()
                            && c.device_boot_id == boot.as_str()
                            && c.lifecycle == ChildLifecycle::Running
                    }) || !active.device_boots.iter().any(|b| {
                        b.device_id == *id
                            && b.device_boot_id == *boot
                            && matches!(b.state, DeviceBootState::Active)
                    }) {
                        return Ok(false);
                    }
                    if !self.registered.contains(id) {
                        with_host(host, |h| {
                            h.execution_gateway_register_boot(
                                self.gateway(),
                                id,
                                boot.clone(),
                                Instant::now(),
                            )
                        })?;
                        self.registered.insert(id.clone());
                    }
                }
                self.phase = PublishingPrepared;
            }
            PublishingPrepared => {
                self.children = match io.children(&BTreeMap::new(), false) {
                    Ok(v) => v,
                    Err(error) => {
                        self.stop_for(error);
                        return Ok(false);
                    }
                };
                if !self.flush_one(host, io, now)? {
                    return Ok(false);
                }
                let slot = io.active()?;
                self.confirm_slot(&slot)?;
                if !self.slot_ready(&slot) {
                    return Err(fault(
                        ErrorCode::InvalidInput,
                        "Exact active boots are no longer confirmed",
                    ));
                }
                // C06/C13 may have blocked. Re-poll the original OS handles
                // after those calls, immediately before committing prepared.
                self.children = match io.children(&BTreeMap::new(), false) {
                    Ok(children) => children,
                    Err(error) => {
                        self.stop_for(error);
                        return Ok(false);
                    }
                };
                if self.children.len() != self.boots.len()
                    || self
                        .children
                        .iter()
                        .any(|c| c.lifecycle != ChildLifecycle::Running)
                {
                    self.stop_for(fault(
                        ErrorCode::IoFailure,
                        "Owned child exited before prepared",
                    ));
                    return Ok(false);
                }
                match with_host(host, |h| {
                    h.finish_execution_preparation(self.job.lease(), self.gateway())
                }) {
                    Ok(()) => {
                        self.prepared = true;
                        self.phase = Live;
                    }
                    Err(error) => self.stop_for(error),
                }
            }
            Live | Settling => {
                // Settle the original publications before advancing another frame.
                // One acknowledgement per new tick let probes expire behind old frames.
                while !self.flush_one(host, io, now)? {}
                with_host(host, |h| h.execution_gateway_tick(self.gateway(), Instant::now()))?;
                let snapshot = self.snapshot(host)?;
                let desired = snapshot
                    .devices
                    .iter()
                    .map(|d| {
                        (
                            d.binding.device_id.clone(),
                            snapshot.coverage.clock_started
                                && matches!(
                                    snapshot.lifecycle.state,
                                    ExecutionSnapshotLifecycleState::Running
                                        | ExecutionSnapshotLifecycleState::DispatchPaused
                                )
                                && d.operator_accept_new_work,
                        )
                    })
                    .collect();
                // A child fault does not fabricate Failed/Completed. Continue Host/cloud
                // reconciliation; preserve the error and all owned handles.
                let children_error = match io.children(&desired, false) {
                    Ok(v) => {
                        self.children = v;
                        None
                    }
                    Err(e) => Some(e),
                };
                while !self.flush_one(host, io, now)? {}
                // Admit the fixed cloud frontier before another tick/publication cycle.
                // One row per cycle made a factory's observations expire in our queue.
                self.receive_frontier(host, io, now)?;
                let snapshot = self.snapshot(host)?;
                let empty = self.pending_work.is_none()
                    && with_host(host, |h| h.execution_gateway_next_cloud_work(self.gateway()))?
                        .is_none();
                if let Some(error) = children_error {
                    return Err(error);
                }
                if terminal(&snapshot)
                    && empty
                    && self.ingress.rows.is_empty()
                    && self.ingress.page_end.is_none()
                    && self.ingress.frontier.is_none()
                    && snapshot.lifecycle.unresolved_command_ids.is_empty()
                    && snapshot.pending_reservations.is_empty()
                    && self
                        .published_revision
                        .as_ref()
                        .is_some_and(|r| r.value() == snapshot.execution_revision.value())
                {
                    self.cleanup_revision = Some(snapshot.execution_revision);
                    self.phase = ShuttingDown;
                }
            }
            StoppingPreparation => {
                with_host(host, |h| {
                    h.stop_native_execution_preparation(self.gateway(), self.stop_request.clone())
                })?;
                self.phase = Settling;
            }
            ShuttingDown => {
                self.children = io.children(&BTreeMap::new(), true)?;
                if self
                    .children
                    .iter()
                    .all(|c| c.lifecycle == ChildLifecycle::ExitedClean)
                {
                    self.phase = RetiringBoots;
                }
            }
            RetiringBoots => {
                if self.retire_queue.is_none() {
                    let slot = io.active()?;
                    self.confirm_slot(&slot)?;
                    self.retire_queue = Some(
                        slot.active_slot
                            .unwrap()
                            .device_boots
                            .into_iter()
                            .filter(|b| matches!(b.state, DeviceBootState::Active))
                            .map(|b| (b.device_id, b.device_boot_id))
                            .collect(),
                    );
                }
                if let Some((id, boot)) = self.retire_queue.as_ref().unwrap().front().cloned() {
                    io.retire(
                        &self.identity,
                        &id,
                        &boot,
                        self.cleanup_revision.clone().unwrap(),
                    )?;
                    self.retire_queue.as_mut().unwrap().pop_front();
                } else {
                    self.phase = ReleasingSlot;
                }
            }
            ReleasingSlot => {
                io.release(&CloudReleaseRequest {
                    contract_version: CloudReleaseRequestContractVersion::SfExecution20Draft11,
                    identity: self.identity.clone(),
                    expected_published_revision: self.cleanup_revision.clone().unwrap(),
                })?;
                self.phase = Finishing;
            }
            Finishing => {
                if !self.cleanup_committed {
                    with_host(host, |h| h.finish_native_execution_cleanup(self.gateway()))?;
                    self.cleanup_committed = true;
                }
                if !self.prepared {
                    if self.job.is_cancelled() {
                        with_host(host, |h| {
                            h.finish_cancelled_execution_preparation(self.job.lease())
                        })?;
                    } else {
                        with_host(host, |h| {
                            h.finish_failed_execution_preparation(
                                self.job.lease(),
                                self.failure.clone().expect("failed preparation reason"),
                            )
                        })?;
                    }
                }
                return Ok(true);
            }
            Idle => unreachable!("queue receiver owns idle state"),
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use sf_commands::{CloudReadConnection, FilePurpose, StartupTaskHandle};
    use sf_domain::{CloudDatasetId, ProjectId};

    fn id(s: &str) -> ObjectId {
        s.to_owned().try_into().unwrap()
    }
    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .find(|p| {
                p.join("resources/v2-demo/candidate-bindings.json")
                    .is_file()
            })
            .unwrap()
            .to_path_buf()
    }
    fn request(
        host: &mut ProjectHost,
        ui: &ObjectId,
        project: Option<&ProjectId>,
        name: &str,
        op: &str,
        params: Value,
    ) -> Value {
        host.dispatch(ui,serde_json::from_value(json!({"api_version":"0.1","request_id":name,"project_id":project,
            "scope":{"kind":if project.is_some(){"project"}else{"workspace"}},"operation":op,"params":params})).unwrap())
    }
    struct Fixture {
        host: Arc<Mutex<ProjectHost>>,
        ui: ObjectId,
        project: ProjectId,
        current: ActiveJob,
        io: FixtureIo,
    }
    fn fixture() -> Fixture {
        let root = root();
        let ui = id("actor-fixture-ui");
        let mut host = ProjectHost::new(id(&unique_id("actor-host")), ui.clone());
        let selected = host
            .select_directory(
                &ui,
                FilePurpose::ImportPackage,
                root.join("resources/v2-demo/project"),
            )
            .unwrap();
        let response = request(
            &mut host,
            &ui,
            None,
            "import",
            "import_package",
            json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let work = host.drain_io_work().pop().unwrap();
        host.finish_io(work.run());
        let response = request(
            &mut host,
            &ui,
            None,
            "adopt",
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":response["outcome"]["data"]["id"],"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let context = request(
            &mut host,
            &ui,
            None,
            "context",
            "get_context",
            json!({"selection":[]}),
        );
        let project: ProjectId =
            serde_json::from_value(context["outcome"]["data"]["project"]["id"].clone()).unwrap();
        let revision = context["outcome"]["data"]["project"]["revision"].clone();
        let scenario: Value = serde_json::from_slice(
            &std::fs::read(root.join("resources/v2-demo/project/data/g000001/scenario-0001.json"))
                .unwrap(),
        )
        .unwrap();
        let candidates: Vec<CandidateBinding> = serde_json::from_slice(
            &std::fs::read(root.join("resources/v2-demo/candidate-bindings.json")).unwrap(),
        )
        .unwrap();
        host.install_cloud_read_connection(
            &ui,
            CloudReadConnection::local_loopback(
                id("actor-test-connection"),
                project.clone(),
                "actor-cloud-project".to_owned().try_into().unwrap(),
                "http://127.0.0.1:9".into(),
                "component-reader-never-used".into(),
                candidates.clone(),
            )
            .unwrap(),
        )
        .unwrap();
        let (sender, receiver) = mpsc::channel();
        host.install_execution_preparation_runtime(
            &ui,
            project.clone(),
            ExecutionPreparationRuntime::new(sender, id("actor-test-connection")),
        )
        .unwrap();
        let response = request(
            &mut host,
            &ui,
            Some(&project),
            "start",
            "start_execution_session",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"base_revision":revision,
            "scenario_id":scenario["data"]["id"],"scenario_revision":scenario["data"]["revision"],"candidate_bindings":candidates,
            "policy":{"clock_mode":"host_monotonic","report_cutoff_ms":120000,"stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish",
                "target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},"source_prediction_window":{"start_ms":0,"end_ms":scenario["data"]["window"]["end_ms"]},"observation_max_age_ms":3000}}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let _: StartupTaskHandle =
            serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let current = ActiveJob::new(
            receiver.try_recv().unwrap(),
            &std::env::temp_dir().join("sf-native-actor-components"),
        );
        let host = Arc::new(Mutex::new(host));
        let io = FixtureIo::new(host.clone(), current.identity.clone());
        Fixture {
            host,
            ui,
            project,
            current,
            io,
        }
    }
    impl Fixture {
        fn step(&mut self) -> Result<bool, ApiError> {
            self.current.step(&self.host, &mut self.io, Instant::now())
        }
        fn until(&mut self, phase: NativeExecutionPhase) {
            for _ in 0..100 {
                if self.current.phase == phase {
                    return;
                }
                assert!(!self.step().unwrap());
            }
            panic!("phase {:?} not reached: {:?}", phase, self.current.phase);
        }
        fn finish(&mut self) {
            for _ in 0..100 {
                if self.step().unwrap() {
                    return;
                }
            }
            panic!("not finished: {:?}", self.current.phase);
        }
        fn task(&mut self) -> Value {
            request(
                &mut self.host.lock().unwrap(),
                &self.ui,
                Some(&self.project),
                "task",
                "get_task",
                json!({"task_id":self.current.job.lease().task_id()}),
            )["outcome"]["data"]
                .clone()
        }
        fn cancel(&mut self) {
            let response = request(
                &mut self.host.lock().unwrap(),
                &self.ui,
                Some(&self.project),
                "cancel",
                "cancel_task",
                json!({"task_id":self.current.job.lease().task_id(),"action":"cancel"}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
        }
        fn run(&mut self) {
            let snapshot = self.current.snapshot(&self.host).unwrap();
            let response = request(
                &mut self.host.lock().unwrap(),
                &self.ui,
                Some(&self.project),
                "run",
                "control_execution_session",
                json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":self.current.identity,"expected_execution_revision":snapshot.execution_revision,"action":{"kind":"run"}}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
        }
        fn stop(&mut self) {
            let snapshot = self.current.snapshot(&self.host).unwrap();
            let response = request(
                &mut self.host.lock().unwrap(),
                &self.ui,
                Some(&self.project),
                "stop",
                "control_execution_session",
                json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":self.current.identity,"expected_execution_revision":snapshot.execution_revision,"action":{"kind":"stop_dispatch"}}),
            );
            assert_eq!(response["outcome"]["status"], "ok", "{response}");
        }
    }
    /// Explicit component side effects only: no socket, process, TLS bypass or credentials.
    struct FixtureIo {
        host: Arc<Mutex<ProjectHost>>,
        identity: ExecutionIdentity,
        boots: BTreeMap<ObjectId, DeviceBootId>,
        facts: BTreeMap<ObjectId, ProcessFacts>,
        retired: BTreeSet<ObjectId>,
        log: Vec<String>,
        creates: Vec<Value>,
        published: Vec<CloudWorkItem>,
        events: Vec<CloudEventRow>,
        fail_create: usize,
        fail_publish: usize,
        fail_retire: usize,
        fail_release: usize,
        retire_payloads: Vec<Value>,
        release_payloads: Vec<Value>,
        fail_spawn_at: Option<usize>,
        fail_probe: bool,
        unexpected_exit: bool,
        slot_owner_mismatch: bool,
        exit_during_active: bool,
        released: bool,
    }
    impl FixtureIo {
        fn new(host: Arc<Mutex<ProjectHost>>, identity: ExecutionIdentity) -> Self {
            Self {
                host,
                identity,
                boots: BTreeMap::new(),
                facts: BTreeMap::new(),
                retired: BTreeSet::new(),
                log: vec![],
                creates: vec![],
                published: vec![],
                events: vec![],
                fail_create: 0,
                fail_publish: 0,
                fail_retire: 0,
                fail_release: 0,
                retire_payloads: vec![],
                release_payloads: vec![],
                fail_spawn_at: None,
                fail_probe: false,
                unexpected_exit: false,
                slot_owner_mismatch: false,
                exit_during_active: false,
                released: false,
            }
        }
        fn unlocked(&mut self, event: &str) {
            assert!(
                self.host.try_lock().is_ok(),
                "side effect under Host lock: {event}"
            );
            self.log.push(event.into());
        }
        fn push_telemetry(&mut self, snapshot: &ExecutionSnapshot, device: usize, seq: u64) {
            let d = &snapshot.devices[device];
            let ingress = (self.events.len() + 1) as u64;
            self.events.push(CloudEventRow::Telemetry {
                receipt_id: id(&format!("telemetry-{ingress}")),
                ingress_id: ingress.try_into().unwrap(),
                cloud_received_at: "2026-10-27T00:00:00Z".to_owned().try_into().unwrap(),
                observation: ExecutionTelemetryObservation {
                    contract_version:
                        ExecutionTelemetryObservationContractVersion::SfExecution20Draft11,
                    session_id: self.identity.session_id.clone(),
                    execution_epoch_id: self.identity.execution_epoch_id.clone(),
                    device_id: d.binding.device_id.clone(),
                    device_boot_id: d.registered_device_boot_id.clone().unwrap(),
                    device_sequence: seq.try_into().unwrap(),
                    device_timestamp: "component".to_owned().try_into().unwrap(),
                    accept_new_work: true,
                    device_state: ExecutionTelemetryObservationDeviceState::Idle,
                    active_command_id: None,
                    phase_progress: None,
                    dataset_id: "actor-dataset".to_owned().try_into().unwrap(),
                    source_purpose: ExecutionTelemetryObservationSourcePurpose::Execution,
                    observation_probe_id: d
                        .pending_observation_probe
                        .as_ref()
                        .map(|p| p.probe_id.clone()),
                },
            });
        }
        fn push_receipt(&mut self, command: &Command, seq: u64, kind: DeviceReceiptKind) {
            let ingress = (self.events.len() + 1) as u64;
            self.events.push(CloudEventRow::Receipt {
                receipt_id: id(&format!("production-{seq}")),
                ingress_id: ingress.try_into().unwrap(),
                cloud_received_at: "2026-10-27T00:00:00Z".to_owned().try_into().unwrap(),
                observation: DeviceReceipt {
                    session_id: self.identity.session_id.clone(),
                    execution_epoch_id: self.identity.execution_epoch_id.clone(),
                    command_id: command.attempt.command_id.clone(),
                    device_id: command.device_id.clone(),
                    phase_id: command.attempt.phase_id.clone(),
                    phase_token: command.attempt.phase_token.clone(),
                    device_boot_id: command.target_device_boot_id.clone(),
                    device_sequence: seq.try_into().unwrap(),
                    device_timestamp: "component".to_owned().try_into().unwrap(),
                    receipt: kind,
                },
            });
        }
    }
    impl ExternalIo for FixtureIo {
        fn verify(&mut self, _: &[CandidateBinding]) -> Result<(), ApiError> {
            self.unlocked("probe");
            if self.fail_probe {
                Err(fault(ErrorCode::IoFailure, "component probe failure"))
            } else {
                Ok(())
            }
        }
        fn create(
            &mut self,
            input: &CloudSessionCreate,
        ) -> Result<CloudResponseVariant0ResultSessionStoredData, ApiError> {
            self.unlocked("C01");
            self.creates.push(serde_json::to_value(input).unwrap());
            if self.fail_create > 0 {
                self.fail_create -= 1;
                return Err(fault(ErrorCode::IoFailure, "component unknown C01"));
            }
            Ok(CloudResponseVariant0ResultSessionStoredData {
                session_id: input.identity.session_id.clone(),
                execution_epoch_id: input.identity.execution_epoch_id.clone(),
                storage_status: CloudResponseVariant0ResultSessionStoredDataStorageStatus::Stored,
                dataset_id: "actor-dataset".to_owned().try_into().unwrap(),
            })
        }
        fn active(&mut self) -> Result<ActiveExecutionSlot, ApiError> {
            self.unlocked("C13");
            if self.exit_during_active {
                self.unexpected_exit = true;
            }
            Ok(ActiveExecutionSlot {
                cloud_project_id: "actor-cloud-project".to_owned().try_into().unwrap(),
                active_slot: Some(ActiveExecutionSlotActiveSlot {
                    session_id: self.identity.session_id.clone(),
                    execution_epoch_id: self.identity.execution_epoch_id.clone(),
                    owner_host_context: if self.slot_owner_mismatch {
                        id("foreign-host")
                    } else {
                        self.identity.host_context_session_id.clone()
                    },
                    state: ActiveExecutionSlotActiveSlotState::Active,
                    published_revision: None,
                    device_boots: self
                        .boots
                        .iter()
                        .map(|(id, boot)| DeviceBoot {
                            device_id: id.clone(),
                            device_boot_id: boot.clone(),
                            session_id: self.identity.session_id.clone(),
                            execution_epoch_id: self.identity.execution_epoch_id.clone(),
                            state: if self.retired.contains(id) {
                                DeviceBootState::Retired
                            } else {
                                DeviceBootState::Active
                            },
                            retired_at_published_revision: None,
                        })
                        .collect(),
                }),
            })
        }
        fn spawn(
            &mut self,
            _: &ExecutionIdentity,
            _: &CloudDatasetId,
            device: &ObjectId,
            boot: &DeviceBootId,
        ) -> Result<(), ApiError> {
            self.unlocked("spawn");
            if self.fail_spawn_at == Some(self.boots.len()) {
                return Err(fault(
                    ErrorCode::IoFailure,
                    "component partial spawn failure",
                ));
            }
            self.boots.insert(device.clone(), boot.clone());
            self.facts.insert(
                device.clone(),
                ProcessFacts {
                    creation: device_process::CreationIdentity {
                        process_id: 0,
                        windows_creation_time_100ns: None,
                        spawn_sequence: 0,
                    },
                    device_id: device.as_str().into(),
                    device_boot_id: boot.as_str().into(),
                    journal_path: PathBuf::from("component-not-real-child"),
                    lifecycle: ChildLifecycle::Running,
                    status: Some(device_process::DeviceStatus {
                        registered: true,
                        state: LocalDeviceState::Idle,
                        accept_commands: false,
                        maintenance: false,
                        shutdown_ready: true,
                        pending_poll: false,
                        pending_events: 0,
                        sampled_slots: 0,
                        missed_slots: 0,
                        transport_unknown: false,
                    }),
                    shutdown_acknowledged: false,
                    shutdown_blocked: false,
                    exit_code: None,
                    stderr_seen: false,
                    fault: None,
                },
            );
            Ok(())
        }
        fn children(
            &mut self,
            desired: &BTreeMap<ObjectId, bool>,
            closing: bool,
        ) -> Result<Vec<ProcessFacts>, ApiError> {
            self.unlocked("children");
            if self.unexpected_exit {
                self.facts.values_mut().next().unwrap().lifecycle =
                    ChildLifecycle::ExitedUnreconciled;
                return Err(device_fault(DeviceError::UnexpectedExit));
            }
            for (id, f) in &mut self.facts {
                f.status.as_mut().unwrap().accept_commands =
                    desired.get(id).copied().unwrap_or(false);
                if closing {
                    self.log.push(format!("shutdown:{}", id.as_str()));
                    f.lifecycle = ChildLifecycle::ExitedClean;
                    f.shutdown_acknowledged = true;
                    f.exit_code = Some(0);
                }
            }
            Ok(self.facts.values().cloned().collect())
        }
        fn publish(&mut self, work: &CloudWorkItem) -> Result<CloudEffectAck, ApiError> {
            self.unlocked("publish");
            self.published.push(work.clone());
            if self.fail_publish > 0 {
                self.fail_publish -= 1;
                return Err(fault(ErrorCode::IoFailure, "component unknown publication"));
            }
            Ok(match work {
                CloudWorkItem::PublishSnapshot {
                    work_id,
                    publication,
                } => CloudEffectAck::SnapshotPublished {
                    work_id: work_id.clone(),
                    ack: PublishAck {
                        session_id: publication.snapshot.identity.session_id.clone(),
                        execution_epoch_id: publication
                            .snapshot
                            .identity
                            .execution_epoch_id
                            .clone(),
                        execution_revision: publication.snapshot.execution_revision.clone(),
                        publication_status: PublishAckPublicationStatus::Stored,
                        observation_frontier: publication.snapshot.observation_frontier.clone(),
                    },
                },
                CloudWorkItem::PublishCommand { work_id, command } => {
                    CloudEffectAck::CommandStored {
                        work_id: work_id.clone(),
                        command_id: command.attempt.command_id.clone(),
                        storage_status:
                            CloudResponseVariant0ResultCommandStoredDataStorageStatus::Stored,
                    }
                }
                CloudWorkItem::WithdrawCommand {
                    work_id,
                    command_id,
                    ..
                } => CloudEffectAck::WithdrawalResult {
                    work_id: work_id.clone(),
                    command_id: command_id.clone(),
                    state: CloudResponseVariant0ResultWithdrawalDataState::WithdrawnBeforeDelivery,
                },
            })
        }
        fn events(
            &mut self,
            identity: &ExecutionIdentity,
            after: SafeUint,
            frontier: Option<SafeUint>,
        ) -> Result<CloudIngressPage, ApiError> {
            self.unlocked("C05");
            let frontier =
                frontier.unwrap_or_else(|| (self.events.len() as u64).try_into().unwrap());
            let rows = self
                .events
                .iter()
                .filter(|r| {
                    let v = match r {
                        CloudEventRow::Receipt { ingress_id, .. }
                        | CloudEventRow::Telemetry { ingress_id, .. } => ingress_id.value(),
                    };
                    v > after.value() && v <= frontier.value()
                })
                .cloned()
                .collect();
            Ok(CloudIngressPage {
                visible_frontier: frontier,
                page: CloudEventsPage {
                    identity: identity.clone(),
                    rows,
                    next_ingress_id: None,
                },
            })
        }
        fn retire(
            &mut self,
            identity: &ExecutionIdentity,
            device: &ObjectId,
            boot: &DeviceBootId,
            revision: SafeUint,
        ) -> Result<(), ApiError> {
            self.unlocked("C20");
            self.retire_payloads
                .push(json!({"identity":identity,"device":device,"boot":boot,"revision":revision}));
            if self.fail_retire > 0 {
                self.fail_retire -= 1;
                return Err(fault(ErrorCode::IoFailure, "component unknown C20"));
            }
            assert_eq!(self.facts[device].lifecycle, ChildLifecycle::ExitedClean);
            self.retired.insert(device.clone());
            Ok(())
        }
        fn release(&mut self, input: &CloudReleaseRequest) -> Result<(), ApiError> {
            self.unlocked("C10");
            self.release_payloads
                .push(serde_json::to_value(input).unwrap());
            if self.fail_release > 0 {
                self.fail_release -= 1;
                return Err(fault(ErrorCode::IoFailure, "component unknown C10"));
            }
            assert_eq!(self.retired.len(), self.boots.len());
            self.released = true;
            Ok(())
        }
    }

    #[test]
    fn actual_host_prepares_without_run_and_persists_frozen_assets_outside_lock() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        let s = f.current.snapshot(&f.host).unwrap();
        assert!(!s.coverage.clock_started);
        assert!(matches!(
            s.lifecycle.state,
            ExecutionSnapshotLifecycleState::Prepared
        ));
        assert!(
            f.io.published
                .iter()
                .all(|w| matches!(w, CloudWorkItem::PublishSnapshot { .. }))
        );
        assert_eq!(f.task()["task"]["state"], "completed");
        for file in f.current.job.files() {
            assert_eq!(
                std::fs::read(f.current.archive.join(file.file.path.as_str())).unwrap(),
                file.bytes
            );
        }
        assert!(f.current.job.prepare_engine().is_err());
    }
    #[test]
    fn cancellation_before_external_has_no_cloud_or_child() {
        let mut f = fixture();
        f.cancel();
        assert!(f.step().unwrap());
        assert!(f.io.log.is_empty());
        assert_eq!(f.task()["task"]["state"], "cancelled");
    }
    #[test]
    fn local_resource_failure_is_failed_without_remote_side_effect() {
        let mut f = fixture();
        f.io.fail_probe = true;
        assert!(!f.step().unwrap());
        assert!(f.step().unwrap());
        assert_eq!(f.io.log, ["probe"]);
        assert_eq!(f.task()["task"]["state"], "failed");
    }
    #[test]
    fn unknown_c01_retains_same_job_and_payload_even_after_cancel() {
        let mut f = fixture();
        f.io.fail_create = 1;
        f.until(NativeExecutionPhase::CreatingSession);
        assert!(f.step().is_err());
        f.cancel();
        assert_eq!(f.current.phase, NativeExecutionPhase::CreatingSession);
        assert!(
            !f.host
                .lock()
                .unwrap()
                .desktop_close_state()
                .active_tasks
                .is_empty()
        );
        f.finish();
        assert_eq!(f.io.creates.len(), 2);
        assert_eq!(f.io.creates[0], f.io.creates[1]);
        assert!(f.io.boots.is_empty());
        assert!(f.io.released);
        assert_eq!(f.task()["task"]["state"], "cancelled");
    }
    #[test]
    fn partial_spawn_failure_publishes_stop_then_clean_exit_retire_release() {
        let mut f = fixture();
        f.io.fail_spawn_at = Some(1);
        f.finish();
        assert_eq!(f.io.boots.len(), 1);
        assert_eq!(f.task()["task"]["state"], "failed");
        let stop=f.io.published.iter().position(|w|matches!(w,CloudWorkItem::PublishSnapshot{publication,..}if matches!(publication.snapshot.lifecycle.state,ExecutionSnapshotLifecycleState::Stopped))).unwrap();
        assert!(stop < f.io.published.len());
        let shutdown =
            f.io.log
                .iter()
                .position(|s| s.starts_with("shutdown:"))
                .unwrap();
        let retire = f.io.log.iter().position(|s| s == "C20").unwrap();
        let release = f.io.log.iter().position(|s| s == "C10").unwrap();
        assert!(shutdown < retire && retire < release);
        assert!(
            f.host
                .lock()
                .unwrap()
                .desktop_close_state()
                .execution_blockers
                .is_empty()
        );
    }
    #[test]
    fn live_step_publishes_latest_frame_with_complete_records_before_advancing_again() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        f.run();
        for _ in 0..4 {
            thread::sleep(Duration::from_millis(2));
            with_host(&f.host, |h| {
                h.execution_gateway_tick(f.current.gateway(), Instant::now())
            })
            .unwrap();
        }
        let originals = with_host(&f.host, |h| {
            h.execution_gateway_cloud_work(f.current.gateway())
        })
        .unwrap()
        .items;
        assert!(originals.len() > 1);
        let before = f.io.published.len();
        f.step().unwrap();
        let original_records: Vec<_> = originals.iter().flat_map(|work| match work {
            CloudWorkItem::PublishSnapshot { publication, .. } => publication.observation_records.clone(),
            _ => panic!("This fixture queues only snapshots"),
        }).collect();
        let CloudWorkItem::PublishSnapshot { publication: latest, .. } = originals.last().unwrap() else { panic!() };
        let CloudWorkItem::PublishSnapshot { publication: sent, .. } = &f.io.published[before] else { panic!() };
        assert_eq!(serde_json::to_value(&sent.snapshot).unwrap(), serde_json::to_value(&latest.snapshot).unwrap());
        assert_eq!(serde_json::to_value(&sent.observation_records).unwrap(), serde_json::to_value(&original_records).unwrap());
        assert!(with_host(&f.host, |h| {
            h.execution_gateway_cloud_work(f.current.gateway())
        })
        .unwrap()
        .items
        .is_empty());
    }
    #[test]
    fn unknown_coalesced_publication_keeps_original_payload_when_new_frames_arrive() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        f.run();
        with_host(&f.host, |h| h.execution_gateway_tick(f.current.gateway(), Instant::now())).unwrap();
        let before = f.io.published.len();
        f.io.fail_publish = 1;
        assert!(f.step().is_err());
        let sent = serde_json::to_value(&f.io.published[before]).unwrap();
        thread::sleep(Duration::from_millis(2));
        with_host(&f.host, |h| h.execution_gateway_tick(f.current.gateway(), Instant::now())).unwrap();
        f.step().unwrap();
        assert_eq!(sent, serde_json::to_value(&f.io.published[before + 1]).unwrap());
    }
    #[test]
    fn uncertain_publication_retries_exact_original_work() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::PublishingPrepared);
        f.io.fail_publish = 1;
        assert!(f.step().is_err());
        let original = serde_json::to_value(&f.io.published[0]).unwrap();
        f.until(NativeExecutionPhase::Live);
        assert_eq!(original, serde_json::to_value(&f.io.published[1]).unwrap());
    }
    #[test]
    fn foreign_slot_cannot_complete_preparation() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::ConfirmingBoots);
        f.io.slot_owner_mismatch = true;
        assert!(f.step().is_err());
        assert!(!f.current.prepared);
        assert!(f.io.retired.is_empty());
        assert!(!f.io.released);
    }
    #[test]
    fn one_live_cycle_admits_the_fixed_multi_device_frontier() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        f.run();
        let snapshot = f.current.snapshot(&f.host).unwrap();
        f.io.push_telemetry(&snapshot, 0, 1);
        f.io.push_telemetry(&snapshot, 1, 1);
        f.step().unwrap();
        assert_eq!(f.current.ingress.after, 2);
        assert!(f.current.ingress.frontier.is_none());
        assert!(f.current.ingress.rows.is_empty());
        let admitted = f.current.snapshot(&f.host).unwrap();
        assert!(admitted.devices.iter().all(|device| device.latest_observation.is_some()));
    }
    #[test]
    fn prepared_raw_telemetry_advances_cloud_cursor_without_host_freshness() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        let s = f.current.snapshot(&f.host).unwrap();
        f.io.push_telemetry(&s, 0, 100);
        f.current
            .receive_one(&f.host, &mut f.io, Instant::now())
            .unwrap();
        assert_eq!(f.current.ingress.after, 1);
        let next = f.current.snapshot(&f.host).unwrap();
        assert!(next.devices[0].latest_observation.is_none());
        assert_eq!(
            s.execution_revision.value(),
            next.execution_revision.value()
        );
    }
    #[test]
    fn completed_task_cancel_does_not_stop_prepared_or_running_session() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        let response = request(
            &mut f.host.lock().unwrap(),
            &f.ui,
            Some(&f.project),
            "late-task-cancel",
            "cancel_task",
            json!({"task_id":f.current.job.lease().task_id(),"action":"cancel"}),
        );
        assert_eq!(response["outcome"]["status"], "ok");
        f.step().unwrap();
        assert_eq!(f.current.phase, NativeExecutionPhase::Live);
        assert!(!f.current.job.is_cancelled());
        f.run();
        f.step().unwrap();
        assert_eq!(f.current.phase, NativeExecutionPhase::Live);
        assert!(f.current.snapshot(&f.host).unwrap().coverage.clock_started);
    }
    #[test]
    fn larger_telemetry_sequence_does_not_suppress_lower_production_receipts() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        f.run();
        let s = f.current.snapshot(&f.host).unwrap();
        f.io.push_telemetry(&s, 0, 100);
        f.io.push_telemetry(&s, 1, 100);
        let mut command = None;
        for _ in 0..60 {
            f.step().unwrap();
            command = f.io.published.iter().find_map(|w| {
                if let CloudWorkItem::PublishCommand { command, .. } = w {
                    Some(command.clone())
                } else {
                    None
                }
            });
            if command.is_some() {
                break;
            }
        }
        let command = command.expect("actual Engine command after two Host probes");
        f.io.push_receipt(
            &command,
            2,
            DeviceReceiptKind::Completed {
                device_reported_duration_ms: Some(10000u64.try_into().unwrap()),
            },
        );
        for _ in 0..30 {
            f.step().unwrap();
            if f.current.ingress.after >= 3 {
                break;
            }
        }
        assert_eq!(f.current.ingress.after, 3);
        let s = f.current.snapshot(&f.host).unwrap();
        assert!(
            s.receipt_admissions
                .iter()
                .any(|r| r.receipt_id == id("production-2")
                    && matches!(r.decision, ReceiptAdmissionDecision::PendingDependency))
        );
        f.io.push_receipt(&command, 1, DeviceReceiptKind::Started {});
        for _ in 0..30 {
            f.step().unwrap();
            if f.current.ingress.after >= 4 {
                break;
            }
        }
        assert_eq!(f.current.ingress.after, 4);
        let s = f.current.snapshot(&f.host).unwrap();
        assert!(
            s.receipt_admissions
                .iter()
                .any(|r| r.receipt_id == id("production-1")
                    && matches!(r.decision, ReceiptAdmissionDecision::Applied))
        );
    }
    #[test]
    fn unexpected_exit_keeps_original_children_and_does_not_restart_or_invent_receipt() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        f.io.unexpected_exit = true;
        assert!(f.step().is_err());
        assert_eq!(f.io.boots.len(), 2);
        assert!(!f.io.released);
        assert!(
            f.current
                .snapshot(&f.host)
                .unwrap()
                .receipt_admissions
                .is_empty()
        );
        assert_eq!(f.current.phase, NativeExecutionPhase::Live);
    }
    #[test]
    fn actual_embedded_python_import_probe_uses_real_native_device_adapter() {
        let runtime = NativeDeviceRuntime::from_native_paths(
            &root().join("resources"),
            &std::env::temp_dir().join(unique_id("sf-actor-real-probe")),
            device_process::NativeCloudOrigin::from_native_configuration(
                "https://example.invalid".into(),
            )
            .unwrap(),
        )
        .unwrap();
        let p = runtime.verify_imports().unwrap();
        assert_eq!(p.python_version, "3.14.9");
        assert!(
            p.isolated && p.no_site && p.thread_ran && p.sqlite_roundtrip && p.monotonic_advanced
        );
    }
    #[test]
    fn terminal_normal_stop_and_unknown_retire_release_keep_exact_cleanup_identity() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        f.run();
        f.stop();
        f.io.fail_retire = 1;
        f.io.fail_release = 1;
        let mut errors = 0;
        let mut done = false;
        for _ in 0..100 {
            match f.step() {
                Ok(true) => {
                    done = true;
                    break;
                }
                Ok(false) => {}
                Err(_) => {
                    errors += 1;
                    assert!(
                        !f.host
                            .lock()
                            .unwrap()
                            .desktop_close_state()
                            .execution_blockers
                            .is_empty()
                    );
                }
            }
        }
        assert!(done);
        assert_eq!(errors, 2);
        assert_eq!(f.io.retire_payloads[0], f.io.retire_payloads[1]);
        assert_eq!(f.io.release_payloads[0], f.io.release_payloads[1]);
        assert_eq!(f.io.retired.len(), 2);
        assert_eq!(f.task()["task"]["state"], "completed");
        assert!(
            f.host
                .lock()
                .unwrap()
                .desktop_close_state()
                .execution_blockers
                .is_empty()
        );
    }
    #[test]
    fn native_public_queue_owner_exits_when_uninstalled_sender_is_dropped() {
        let ui = id("queue-owner-ui");
        let host = Arc::new(Mutex::new(ProjectHost::new(id("queue-owner-host"), ui)));
        let devices = Arc::new(
            NativeDeviceRuntime::from_native_paths(
                &root().join("resources"),
                &std::env::temp_dir().join(unique_id("sf-actor-queue")),
                device_process::NativeCloudOrigin::from_native_configuration(
                    "https://example.invalid".into(),
                )
                .unwrap(),
            )
            .unwrap(),
        );
        let cloud = Arc::new(
            ExecutionCloudConnection::https(
                id("queue-connection"),
                "queue-project".to_owned().try_into().unwrap(),
                "queue-cloud-project".to_owned().try_into().unwrap(),
                "https://example.invalid".into(),
                "native-test-secret-never-log".into(),
            )
            .unwrap(),
        );
        let bindings = [
            (
                id("A"),
                NativeExecutionDevice {
                    profile: DeviceProfile::ProcessorA,
                    token: DeviceToken::from_native_secret("A-secret".into()).unwrap(),
                },
            ),
            (
                id("B"),
                NativeExecutionDevice {
                    profile: DeviceProfile::ProcessorB,
                    token: DeviceToken::from_native_secret("B-secret".into()).unwrap(),
                },
            ),
        ]
        .into_iter()
        .collect();
        let weak_host = Arc::downgrade(&host);
        let (runtime, actor) = start_execution_runtime(
            host.clone(),
            NativeExecutionConfig {
                cloud,
                devices,
                connection_ref: id("queue-connection"),
                bindings,
                archive_root: std::env::temp_dir().join(unique_id("actor-archive")),
            },
        )
        .unwrap();
        assert!(!actor.status().has_obligations);
        drop(host);
        assert!(
            weak_host.upgrade().is_none(),
            "Idle worker must not keep a Host/sender reference cycle"
        );
        drop(runtime);
        actor.worker.join().unwrap();
    }
    #[test]
    fn unadmitted_production_receipt_keeps_original_ingress_position() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::Live);
        let (device, boot) = f.current.boots.iter().next().unwrap();
        let binding = f
            .current
            .cloud_input
            .candidate_bindings
            .iter()
            .find(|b| b.device_id == *device)
            .unwrap();
        f.io.events.push(CloudEventRow::Receipt {
            receipt_id: id("unadmitted-production"),
            ingress_id: 1u64.try_into().unwrap(),
            cloud_received_at: "2026-10-27T00:00:00Z".to_owned().try_into().unwrap(),
            observation: DeviceReceipt {
                session_id: f.current.identity.session_id.clone(),
                execution_epoch_id: f.current.identity.execution_epoch_id.clone(),
                command_id: "not-an-issued-command".to_owned().try_into().unwrap(),
                device_id: device.clone(),
                phase_id: binding.phase_id.clone(),
                phase_token: id("not-an-issued-token"),
                device_boot_id: boot.clone(),
                device_sequence: 1u64.try_into().unwrap(),
                device_timestamp: "component-negative".to_owned().try_into().unwrap(),
                receipt: DeviceReceiptKind::Accepted {},
            },
        });
        assert!(
            f.current
                .receive_one(&f.host, &mut f.io, Instant::now())
                .is_err()
        );
        assert_eq!(f.current.ingress.after, 0);
        assert_eq!(f.current.ingress.rows.len(), 1);
        f.run();
        assert!(
            f.current
                .receive_one(&f.host, &mut f.io, Instant::now())
                .is_err()
        );
        assert_eq!(f.current.ingress.after, 0);
        assert!(
            f.current
                .snapshot(&f.host)
                .unwrap()
                .receipt_admissions
                .iter()
                .any(|a| a.receipt_id == id("unadmitted-production")
                    && matches!(a.decision, ReceiptAdmissionDecision::Rejected))
        );
    }
    #[test]
    fn child_exit_during_final_c13_cannot_be_committed_as_prepared() {
        let mut f = fixture();
        f.until(NativeExecutionPhase::PublishingPrepared);
        f.io.exit_during_active = true;
        for _ in 0..20 {
            f.step().unwrap();
            if f.current.phase == NativeExecutionPhase::StoppingPreparation {
                break;
            }
        }
        assert!(!f.current.prepared);
        assert_eq!(f.current.phase, NativeExecutionPhase::StoppingPreparation);
        assert_ne!(f.task()["task"]["state"], "completed");
        assert_eq!(f.io.boots.len(), 2);
        assert!(!f.io.released);
    }
}
