//! Additive internal Rust execution handoff. Wire ownership stays in sf-domain.
use super::*;
use sf_domain::cloud_execution as wire;

#[path = "external_bindings.rs"]
mod bindings;

#[cfg(test)]
#[path = "external_tests.rs"]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalError {
    pub code: &'static str,
    pub message: String,
}
impl std::fmt::Display for ExternalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ExternalError {}
fn error(code: &'static str, message: impl Into<String>) -> ExternalError {
    ExternalError {
        code,
        message: message.into(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalAttemptState {
    Reserved,
    Published,
    Accepted,
    Started,
    Completed,
    CancelledNotStarted,
    WithdrawnUnpublished,
    ReconciliationRequired,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalReceiptStatus {
    Accepted,
    Started,
    Completed,
    CancelledNotStarted,
    RetainedUnapplied,
    Failed,
}
#[derive(Debug, Clone)]
pub struct ExternalReceiptOutcome {
    pub status: ExternalReceiptStatus,
    pub duplicate: bool,
    pub committed_at_ms: u64,
    pub command_id: DispatchCommandId,
    pub logical_job_id: ObjectId,
    pub device_reported_duration_ms: Option<u64>,
}
#[derive(Debug, Clone)]
pub struct ExternalAttemptView {
    pub command: wire::Command,
    pub state: ExternalAttemptState,
    pub pending_items: Vec<(ObjectId, u64)>,
    pub pending_input_ordinals: Vec<SafeUint>,
    pub reserved_resource_units: Vec<wire::ResourceUnits>,
    pub unapplied_receipts: Vec<wire::DeviceReceipt>,
}
#[derive(Debug, Clone)]
pub struct ExternalLogicalWork {
    pub logical_job_id: ObjectId,
    pub production_task_id: ObjectId,
    pub route_step_id: Option<ObjectId>,
    pub supply_id: Option<ObjectId>,
    pub cycle: u64,
    pub allowed_binding_ids: Vec<ObjectId>,
}
/// Actual same-Engine prefix; not a prediction RunResult or a new wire DTO.
#[derive(Debug, Clone)]
pub struct ExternalObservation {
    pub identity: wire::ExecutionIdentity,
    pub clock_started: bool,
    pub elapsed_ms: u64,
    pub dispatch_stopped: bool,
    pub drained: bool,
    pub goal_met: bool,
    pub queued_logical_jobs: Vec<ObjectId>,
    pub logical_work: Vec<ExternalLogicalWork>,
    pub attempts: Vec<ExternalAttemptView>,
    pub items: Vec<RuntimeItem>,
    pub jobs: Vec<JobState>,
    pub events: Vec<SimulationEvent>,
    pub resource_claims: Vec<ResourceClaim>,
    pub phase_work_intervals: Vec<PhaseWorkInterval>,
    pub acceptance_events: Vec<AcceptanceEvent>,
    pub allocations: Vec<MaterialAllocation>,
    pub order_outcomes: Vec<OrderOutcome>,
    pub transport: Vec<TransportState>,
}
struct FrozenBinding {
    binding: wire::CandidateBinding,
    jobs: BTreeMap<usize, Job>,
}
struct Attempt {
    command: wire::Command,
    index: usize,
    phase_index: usize,
    spec: Job,
    resources: BTreeMap<ObjectId, u64>,
    state: ExternalAttemptState,
    started: bool,
    admitted: Vec<(wire::DeviceReceipt, ExternalReceiptOutcome)>,
    unapplied: Vec<wire::DeviceReceipt>,
}
impl Attempt {
    fn reserves(&self) -> bool {
        !self.started
            && matches!(
                self.state,
                ExternalAttemptState::Reserved
                    | ExternalAttemptState::Published
                    | ExternalAttemptState::Accepted
                    | ExternalAttemptState::ReconciliationRequired
            )
    }
}
pub(crate) struct ExternalExecution {
    identity: wire::ExecutionIdentity,
    policy: wire::ExecutionPolicy,
    candidates: BTreeMap<ObjectId, FrozenBinding>,
    attempts: BTreeMap<DispatchCommandId, Attempt>,
    live: BTreeMap<usize, DispatchCommandId>,
    ordinals: BTreeMap<usize, u64>,
    activated: bool,
    pub(crate) stop_dispatch: bool,
    cutoff: Option<Box<ExternalObservation>>,
}
impl ExternalExecution {
    pub(crate) fn reserved_units(&self, resource: &ObjectId) -> u64 {
        self.attempts
            .values()
            .filter(|a| a.reserves())
            .map(|a| a.resources.get(resource).copied().unwrap_or(0))
            .sum()
    }
}
fn same_receipt(a: &wire::DeviceReceipt, b: &wire::DeviceReceipt) -> bool {
    if a.device_sequence != b.device_sequence
        || a.device_timestamp.as_str() != b.device_timestamp.as_str()
    {
        return false;
    }
    match (&a.receipt, &b.receipt) {
        (wire::DeviceReceiptKind::Accepted {}, wire::DeviceReceiptKind::Accepted {})
        | (wire::DeviceReceiptKind::Started {}, wire::DeviceReceiptKind::Started {})
        | (wire::DeviceReceiptKind::Paused {}, wire::DeviceReceiptKind::Paused {})
        | (wire::DeviceReceiptKind::Resumed {}, wire::DeviceReceiptKind::Resumed {})
        | (
            wire::DeviceReceiptKind::CancelledNotStarted {},
            wire::DeviceReceiptKind::CancelledNotStarted {},
        )
        | (
            wire::DeviceReceiptKind::CancelRejectedStarted {},
            wire::DeviceReceiptKind::CancelRejectedStarted {},
        ) => true,
        (
            wire::DeviceReceiptKind::Completed {
                device_reported_duration_ms: a,
            },
            wire::DeviceReceiptKind::Completed {
                device_reported_duration_ms: b,
            },
        ) => a.as_ref().map(|x| x.value()) == b.as_ref().map(|x| x.value()),
        (
            wire::DeviceReceiptKind::Failed {
                code: a,
                message: x,
            },
            wire::DeviceReceiptKind::Failed {
                code: b,
                message: y,
            },
        ) => a.as_str() == b.as_str() && x.as_str() == y.as_str(),
        _ => false,
    }
}

impl Engine {
    /// Freeze route phases and their legal alternative devices. Compilation of a
    /// candidate is pure; only the original logical jobs enter this Engine.
    pub fn new_external(
        input: EngineInput,
        identity: wire::ExecutionIdentity,
        policy: wire::ExecutionPolicy,
        candidates: Vec<wire::CandidateBinding>,
    ) -> Result<Self, ExternalError> {
        if identity.project_id != input.project.id
            || input.scenario.window.start_ms.0.value() != 0
            || policy.source_prediction_window.end_ms.value()
                != input.scenario.window.end_ms.0.value()
            || policy.report_cutoff_ms.value() == 0
            || amount(&policy.target_quantity) == 0
            || policy.target_quantity.value.decimal_places.value() != 0
        {
            return Err(error(
                "external.policy",
                "Exact Project, zero-based prediction window and positive target/cutoff required",
            ));
        }
        let mut engine =
            Self::new(input).map_err(|issues| error("external.input", format!("{issues:?}")))?;
        if engine.jobs.is_empty() || candidates.is_empty() {
            return Err(error(
                "external.scope",
                "Actual logical work and frozen candidates required",
            ));
        }
        for job in &engine.jobs {
            let s = &job.spec;
            let mut unsupported = Vec::new();
            if s.setup.is_some() { unsupported.push("setup"); }
            if s.outputs.is_empty() && s.preparation.is_none() && s.transport.is_none() { unsupported.push("no material output"); }
            if s.outputs.iter().any(|output| output.decimal_places != 0) { unsupported.push("fractional outputs"); }
            if !unsupported.is_empty() {
                let device = match &s.execution {
                    JobExecutionRef::ProjectDevice { device_id, .. } => device_id.as_str(),
                    _ => "transport",
                };
                return Err(error(
                    "external.unsupported",
                    format!("External binding required for device {device}, job {}, operation {}@{}, phases [{}]: {}",
                        s.id.as_str(), s.operation_ref.id.as_str(), String::from(s.operation_ref.version.clone()),
                        s.phases.iter().map(|p| p.id.as_str()).collect::<Vec<_>>().join(", "),
                        unsupported.join(", ")),
                ));
            }
        }
        let predecessors: BTreeSet<_> = engine.jobs.iter()
            .flat_map(|job| job.spec.predecessors.iter().cloned()).collect();
        let goal = &engine.input.scenario.acceptance_goal;
        let terminal_units: u128 = engine.jobs.iter()
            .filter(|job| !job.dormant && !predecessors.contains(&job.spec.id))
            .flat_map(|job| job.spec.outputs.iter().enumerate().filter(|(ordinal, _)| job.spec.quality.as_ref().is_none_or(|policy| policy.pass == *ordinal)).map(|(_, output)| output))
            .filter(|output| output.unit == policy.target_quantity.unit
                && goal.iter().filter(|_| engine.input.scenario.orders.is_empty()).flat_map(|goal| std::iter::once(&goal.acceptance_scope).chain(&goal.additional_acceptance_scopes))
                    .filter(|scope| scope.unit == output.unit).map(|scope| &scope.product_ref)
                    .chain(engine.input.scenario.orders.iter().filter(|order| order.quantity.as_ref().is_some_and(|quantity| quantity.unit == output.unit)).map(|order| &order.product_ref))
                    .any(|reference| engine.input.project.products.iter().any(|product|
                        product.id == reference.id && product.revision == reference.revision
                            && same_type(&product.item_type_ref, &output.item_type))))
            .map(|output| u128::from(output.amount) * 10_u128.pow(9 - u32::from(output.decimal_places))).sum();
        let target = u128::from(amount(&policy.target_quantity))
            * 10_u128.pow(9 - u32::from(policy.target_quantity.value.decimal_places.value()));
        if terminal_units != target {
            return Err(error(
                "external.target",
                "Frozen terminal accepted-product yield must equal the explicit target; intermediate and other-unit outputs do not count",
            ));
        }
        let mut frozen = BTreeMap::new();
        for binding in candidates {
            if frozen.contains_key(&binding.binding_id)
                || !engine.jobs.iter().any(|job| same_module(&binding.operation_ref, &job.spec.operation_ref))
            {
                return Err(error(
                    "external.binding",
                    "Duplicate binding or operation absent from the frozen route",
                ));
            }
            if let Some(supply_id) = &binding.transport_supply_id {
                let specs: BTreeMap<_, _> = engine.jobs.iter().enumerate()
                    .filter(|(_, j)| j.spec.transport.as_ref().is_some_and(|p| &p.supply.id == supply_id))
                    .map(|(i, j)| (i, j.spec.clone())).collect();
                if specs.is_empty() { return Err(error("external.binding", "Transport supply absent from frozen input")); }
                for spec in specs.values() {
                    let plan = spec.transport.as_ref().unwrap();
                    let phase = spec.phases.iter().find(|p| p.id == binding.phase_id);
                    let requirements: BTreeMap<_, _> = plan.definition.resources.iter()
                        .chain(&plan.definition.loading_resources).chain(&plan.definition.unloading_resources)
                        .map(|r| (r.role.clone(), amount(&r.quantity))).collect();
                    if binding.setup_transition.is_some() || !binding.input_ports.is_empty() || !binding.output_ports.is_empty()
                        || binding.device_id != plan.carrier || binding.execution_capacity_ref.id != plan.carrier
                        || !matches!(binding.execution_capacity_ref.kind, ObjectKind::Resource)
                        || !same_module(&binding.device_module_ref, &spec.operation_ref)
                        || !same_module(&binding.operation_ref, &spec.operation_ref)
                        || phase.is_none_or(|p| p.work != binding.nominal_work_ms.value())
                        || requirements.len() != binding.resources.len()
                        || binding.resources.iter().any(|r| requirements.get(&r.role_id) != Some(&r.units.value())
                            || plan.supply.resources.get(&r.role_id) != Some(&r.resource_id))
                        || binding.resources.iter().map(|r| &r.role_id).collect::<BTreeSet<_>>().len() != requirements.len() {
                        return Err(error("external.binding", "Transport binding differs from its actual supply, carrier, phase or resources"));
                    }
                }
                frozen.insert(binding.binding_id.clone(), FrozenBinding { binding, jobs: specs });
                continue;
            }
            let device = engine
                .input
                .project
                .devices
                .iter()
                .find(|d| d.id == binding.device_id)
                .ok_or_else(|| error("external.binding", "Candidate device missing"))?;
            if !same_module(&device.module_ref, &binding.device_module_ref) {
                return Err(error(
                    "external.binding",
                    "Candidate device exact version differs",
                ));
            }
            let canonical = engine.jobs.iter().filter(|j| same_module(&j.spec.operation_ref, &binding.operation_ref)).all(|j| {
                let JobExecutionRef::ProjectDevice { device_id, route_step_id, .. } = &j.spec.execution else { return false; };
                if device_id != &binding.device_id { return false; }
                let task = engine.input.scenario.production_tasks.iter().find(|t| t.id == j.spec.task).unwrap();
                let route = engine.input.project.routes.iter().find(|r| r.id == task.routing_ref.id && r.revision == task.routing_ref.revision).unwrap();
                let step = route.steps.iter().find(|s| Some(&s.id) == route_step_id.as_ref()).unwrap();
                binding.resources.iter().all(|r| step.resource_bindings.get(&r.role_id)
                    .or_else(|| device.role_bindings.resources.get(&r.role_id)) == Some(&r.resource_id))
            });
            let compiled = if canonical {
                engine.jobs.iter().map(|j| j.spec.clone()).collect()
            } else {
            let mut candidate_input = engine.input.clone();
            for route in &mut candidate_input.project.routes {
                for step in &mut route.steps {
                    if same_module(&step.operation_ref, &binding.operation_ref) {
                        step.device_id = binding.device_id.clone();
                        if binding.setup_transition.is_none() { step.resource_bindings.clear(); }
                        step.resource_bindings.extend(binding.resources.iter().map(|r| (r.role_id.clone(), r.resource_id.clone())));
                    }
                }
            }
            let (compiled, _) = crate::compile::compile(&candidate_input).map_err(|issues| {
                error(
                    "external.binding",
                    format!("Candidate compilation rejected: {issues:?}"),
                )
            })?;
            compiled
            };
            if compiled.len() != engine.jobs.len() {
                return Err(error(
                    "external.binding",
                    "Candidate changed logical work count",
                ));
            }
            let mut specs = BTreeMap::new();
            for (index, base) in engine.jobs.iter().enumerate()
                .filter(|(_, base)| same_module(&base.spec.operation_ref, &binding.operation_ref)) {
                let mut spec = compiled
                    .iter()
                    .find(|j| j.id == base.spec.id)
                    .ok_or_else(|| {
                        error(
                            "external.binding",
                            "Candidate changed logical task/cycle/step identity",
                        )
                    })?
                    .clone();
                if let Some(transition) = &binding.setup_transition {
                    spec = crate::engine::setup::external_setup(&spec, transition)
                        .ok_or_else(|| error("external.binding", "Setup transition absent from the frozen device policy"))?;
                }
                let phase = spec.phases.iter().find(|phase| phase.id == binding.phase_id)
                    .ok_or_else(|| error("external.binding", "Candidate phase absent from compiled operation"))?;
                if phase.work != binding.nominal_work_ms.value()
                    || spec.transport.is_some()
                    || binding.setup_transition.is_none() && spec.predecessors != base.spec.predecessors
                {
                    return Err(error(
                        "external.binding",
                        "Candidate phase/model/nominal duration differs from compiler",
                    ));
                }
                match &spec.execution {
                    JobExecutionRef::ProjectDevice {
                        device_id,
                        execution_capacity_ref,
                        ..
                    } if *device_id == binding.device_id
                        && execution_capacity_ref.id == binding.execution_capacity_ref.id
                        && matches!(binding.execution_capacity_ref.kind, ObjectKind::Resource) => {}
                    _ => {
                        return Err(error(
                            "external.binding",
                            "Execution device/capacity mapping differs",
                        ));
                    }
                }
                for (declared, actual) in [
                    (&binding.input_ports, &spec.inputs),
                    (&binding.output_ports, &spec.outputs),
                ] {
                    let mut roles = BTreeSet::new();
                    if declared.len() != actual.len()
                        || declared.iter().any(|p| {
                            !roles.insert((p.role_id.clone(), p.port_id.clone()))
                                || !actual.iter().any(|m| {
                                    m.role == p.role_id
                                        && m.port == p.port_id
                                        && m.location == p.location_id
                                        && m.item_type.id == p.item_type_id
                                        && m.unit == p.unit_id
                                })
                        })
                    {
                        return Err(error(
                            "external.binding",
                            "Exact role/port/location/item type/unit mapping differs",
                        ));
                    }
                }
                let setup_rule = binding.setup_transition.as_ref().and_then(|transition|
                    engine.input.project.setup_policies.iter().find(|p| p.device_id == binding.device_id)
                        .and_then(|p| p.changeovers.iter().find(|r| r.from_family == transition.from_family && r.to_family == transition.to_family)));
                let requirements: BTreeMap<_, _> = setup_rule.map_or(spec.op().resource_roles.as_slice(), |r| r.resources.as_slice())
                    .iter()
                    .chain(spec.op().phases.iter().filter(|_| setup_rule.is_none()).flat_map(|p| &p.phase_resources))
                    .map(|r| (r.role.clone(), amount(&r.quantity)))
                    .collect();
                let mut roles = BTreeSet::new();
                if binding.resources.len() != requirements.len()
                    || binding.resources.iter().any(|r| {
                        !roles.insert(r.role_id.clone())
                            || requirements.get(&r.role_id) != Some(&r.units.value())
                            || spec
                                .job_resources
                                .get(&r.resource_id)
                                .copied()
                                .unwrap_or(0)
                                .max(
                                    spec.phases.iter().map(|phase| phase.resources.get(&r.resource_id).copied().unwrap_or(0)).max().unwrap_or(0),
                                )
                                < r.units.value()
                    })
                {
                    return Err(error(
                        "external.binding",
                        "Resource role/units differ from frozen compiled requirements",
                    ));
                }
                specs.insert(index, spec);
            }
            frozen.insert(
                binding.binding_id.clone(),
                FrozenBinding {
                    binding,
                    jobs: specs,
                },
            );
        }
        if (0..engine.jobs.len()).any(|index| !frozen.values().any(|candidate| candidate.binding.setup_transition.is_none() && candidate.jobs.contains_key(&index))) {
            return Err(error("external.binding", "Every original route operation requires a frozen compatible device binding"));
        }
        for candidate in frozen.values().filter(|c| c.binding.setup_transition.is_none()) {
            for (index, spec) in &candidate.jobs {
                if spec.phases.iter().any(|phase| !frozen.values().any(|other|
                    other.binding.device_id == candidate.binding.device_id
                        && other.jobs.contains_key(index) && other.binding.phase_id == phase.id
                        && other.jobs[index].job_resources == spec.job_resources
                        && other.jobs[index].phases.iter().map(|p| &p.resources).eq(spec.phases.iter().map(|p| &p.resources)))) {
                    return Err(error("external.binding", "Every selectable device requires bindings for all operation phases"));
                }
                for transition in crate::engine::setup::required_transitions(spec) {
                    if !frozen.values().any(|other| other.binding.device_id == candidate.binding.device_id
                        && other.binding.setup_transition.as_ref() == Some(&transition) && other.jobs.contains_key(index)) {
                        return Err(error("external.binding", "Every declared family transition requires its frozen setup binding"));
                    }
                }
            }
        }
        engine.external = Some(ExternalExecution {
            identity,
            policy,
            candidates: frozen,
            attempts: BTreeMap::new(),
            live: BTreeMap::new(),
            ordinals: BTreeMap::new(),
            activated: false,
            stop_dispatch: false,
            cutoff: None,
        });
        Ok(engine)
    }

    pub fn activate_external(&mut self) -> Result<(), ExternalError> {
        let e = self
            .external
            .as_mut()
            .ok_or_else(|| error("external.mode", "Not an external Engine"))?;
        if e.activated {
            return Err(error(
                "external.activation",
                "Clock epoch already activated",
            ));
        }
        e.activated = true;
        self.status = ComputationStatus::Running;
        self.external_advance_to(0);
        Ok(())
    }
    pub fn advance_external(&mut self, elapsed: SafeUint) -> Result<(), ExternalError> {
        self.external_observe_time(elapsed, false)
    }
    fn external_observe_time(
        &mut self,
        elapsed: SafeUint,
        receipt_at_cutoff: bool,
    ) -> Result<(), ExternalError> {
        self.external_check_time(elapsed.value())?;
        if matches!(self.status, ComputationStatus::Completed) {
            return Ok(());
        }
        let cutoff = self
            .external
            .as_ref()
            .unwrap()
            .policy
            .report_cutoff_ms
            .value();
        if self.external.as_ref().unwrap().cutoff.is_none() && cutoff <= elapsed.value() {
            self.external_advance_to(cutoff);
            self.external_stop_dispatch();
            if !receipt_at_cutoff || elapsed.value() > cutoff {
                self.external_freeze_cutoff()?;
            }
        }
        if !matches!(self.status, ComputationStatus::Completed) && self.now < elapsed.value() {
            self.external_advance_to(elapsed.value());
        }
        Ok(())
    }
    pub(super) fn external_stop_dispatch(&mut self) {
        self.external.as_mut().unwrap().stop_dispatch = true;
        let unpublished: Vec<_> = self
            .external
            .as_ref()
            .unwrap()
            .attempts
            .iter()
            .filter(|(_, a)| a.state == ExternalAttemptState::Reserved)
            .map(|(id, _)| id.clone())
            .collect();
        for id in unpublished {
            self.external_release_unpublished(&id);
        }
        self.external_settle();
    }
    fn external_freeze_cutoff(&mut self) -> Result<(), ExternalError> {
        let e = self.external.as_ref().unwrap();
        if e.cutoff.is_none() && self.now == e.policy.report_cutoff_ms.value() {
            let view = self.external_observation()?;
            self.external.as_mut().unwrap().cutoff = Some(Box::new(view));
        }
        Ok(())
    }
    fn external_check_time(&self, elapsed: u64) -> Result<(), ExternalError> {
        let e = self
            .external
            .as_ref()
            .ok_or_else(|| error("external.mode", "Not an external Engine"))?;
        if !e.activated {
            return Err(error(
                "external.prepared",
                "Activate the original Host monotonic epoch before dispatch or receipt admission",
            ));
        }
        if elapsed < self.now {
            return Err(error("external.clock", "Host elapsed cannot move backward"));
        }
        Ok(())
    }
    fn external_advance_to(&mut self, elapsed: u64) {
        while let Some(next) = self
            .queue
            .peek()
            .map(|e| e.0.at)
            .filter(|at| *at <= elapsed)
        {
            self.now = next;
            while self.queue.peek().is_some_and(|e| e.0.at == self.now) {
                let event = self.queue.pop().unwrap().0;
                assert!(
                    !matches!(event.kind, Boundary::Work(..)),
                    "external phase must never schedule nominal completion"
                );
                self.boundary(event.kind);
            }
            self.external_settle();
        }
        self.now = elapsed;
        self.external_settle();
    }
    fn external_settle(&mut self) {
        loop {
            if self.try_arrival_one() || self.transfer_one() || self.commit_setup_one() || self.receive_transport_one() {
                continue;
            }
            self.accept_orders();
            break;
        }
        // Rework and reinspection are created by the same quality engine. Bind
        // their new identities to the already frozen exact route operation.
        let candidates = &mut self.external.as_mut().unwrap().candidates;
        for (index, job) in self.jobs.iter().enumerate() {
            for candidate in candidates.values_mut() {
                if candidate.jobs.contains_key(&index) { continue; }
                if let Some(template) = candidate.jobs.values().find(|spec|
                    same_module(&spec.operation_ref, &job.spec.operation_ref)
                        && spec.setup.as_ref().map(|s| s.transition()) == job.spec.setup.as_ref().map(|s| s.transition())
                        && spec.task == job.spec.task && (spec.cycle == job.spec.cycle || spec.transport.is_some())
                        && (matches!((&spec.execution, &job.spec.execution),
                            (JobExecutionRef::ProjectDevice { route_step_id: a, .. },
                             JobExecutionRef::ProjectDevice { route_step_id: b, .. }) if a == b)
                            || spec.transport.as_ref().zip(job.spec.transport.as_ref()).is_some_and(|(a, b)| a.supply.id == b.supply.id))) {
                    let mut spec = template.clone();
                    spec.id = job.spec.id.clone();
                    spec.predecessors = job.spec.predecessors.clone();
                    spec.release = job.spec.release;
                    spec.cycle = job.spec.cycle;
                    candidate.jobs.insert(index, spec);
                }
            }
        }
        self.computed = Some(self.now);
        self.record_job_history();
        let e = self.external.as_ref().unwrap();
        let unresolved = e
            .attempts
            .values()
            .any(|a| a.reserves() || a.started && a.state != ExternalAttemptState::Completed);
        let output_wait = self.jobs.iter().any(|j| {
            j.state.started_at_ms.is_some() && !matches!(j.state.state, JobStatus::Completed)
        });
        if !unresolved
            && !output_wait
            && (e.stop_dispatch
                || self
                    .jobs
                    .iter()
                    .all(|j| j.dormant || matches!(j.state.state, JobStatus::Completed)))
        {
            self.status = ComputationStatus::Completed;
        }
    }

    /// Before any cloud/network publication, cross the publication barrier.
    /// A returned Command is not permission to transmit before mark_published.
    pub fn prepare_external(
        &mut self,
        attempt: wire::AttemptRef,
        boot: DeviceBootId,
        elapsed: SafeUint,
    ) -> Result<wire::Command, ExternalError> {
        self.advance_external(elapsed)?;
        let e = self.external.as_ref().unwrap();
        if !matches!(self.status, ComputationStatus::Running) {
            return Err(error("external.cutoff", "New dispatch is stopped"));
        }
        if e.attempts.contains_key(&attempt.command_id) {
            return Err(error(
                "external.command",
                "Command ID has already been used",
            ));
        }
        let index = self
            .jobs
            .iter()
            .position(|j| j.spec.id == attempt.logical_job_id)
            .ok_or_else(|| {
                error(
                    "external.logical_job",
                    "Unknown original logical task/cycle/step",
                )
            })?;
        let phase_index = self.jobs[index].phase;
        let continuing = self.jobs[index].state.started_at_ms.is_some();
        if e.stop_dispatch && !continuing {
            return Err(error("external.cutoff", "New logical work is stopped"));
        }
        if e.live.contains_key(&index) || phase_index >= self.jobs[index].spec.phases.len()
            || continuing && !(matches!(self.jobs[index].state.state, JobStatus::WaitingResource)
                || matches!(self.jobs[index].state.state, JobStatus::WaitingOutput) && self.jobs[index].state.pending_outputs.is_empty()) {
            return Err(error(
                "external.live_attempt",
                "Published, unknown, started or completed work cannot be reassigned",
            ));
        }
        if e.ordinals
            .get(&index)
            .is_some_and(|old| attempt.attempt_ordinal.value() <= *old)
        {
            return Err(error(
                "external.ordinal",
                "New attempt ordinal must increase",
            ));
        }
        let candidate = e
            .candidates
            .get(&attempt.selected_binding_id)
            .ok_or_else(|| error("external.binding", "Not a frozen legal candidate"))?;
        let spec = candidate.jobs.get(&index).cloned()
            .ok_or_else(|| error("external.binding", "Candidate belongs to a different original route operation"))?;
        let device_id = candidate.binding.device_id.clone();
        if continuing && !matches!(&self.jobs[index].spec.execution, JobExecutionRef::ProjectDevice { device_id, .. } if device_id == &candidate.binding.device_id)
            && self.jobs[index].spec.transport.as_ref().is_none_or(|p| p.carrier != candidate.binding.device_id) {
            return Err(error("external.binding", "Started work must continue on its original device"));
        }
        if attempt.phase_id != candidate.binding.phase_id
            || attempt.phase_id != spec.phases[phase_index].id || spec.release > self.now
            || spec.setup.as_ref().map(|s| s.transition()) != self.jobs[index].spec.setup.as_ref().map(|s| s.transition()) {
            return Err(error(
                "external.phase",
                "Wrong phase or logical work not yet released",
            ));
        }
        if !spec.predecessors.iter().all(|id| self.completed_job_ids.contains(id.as_str())) {
            return Err(error("external.predecessor", "Original predecessor work has not completed"));
        }
        if spec.preparation.as_ref().is_some_and(|preparation| preparation.prerequisites.iter().any(|marker| !self.markers.contains(marker))) {
            return Err(error("external.preparation", "Required preparation marker has not been committed"));
        }
        let transport_plan = if spec.transport.is_some() {
            Some(self.external_transport_plan(index, None).map_err(|reasons|
                error(if continuing { "external.output_capacity" } else { "external.material" }, format!("{reasons:?}")))?)
        } else { None };
        let original = std::mem::replace(&mut self.jobs[index].spec, spec.clone());
        let family_wait = self.prepare_family(index).is_some();
        self.jobs[index].spec = original;
        if family_wait {
            self.external_settle();
            return Err(error("external.preparation", "Actual changeover or first-article release is still required"));
        }
        let old = std::mem::replace(&mut self.jobs[index].spec, spec.clone());
        let resource_plan = self.resource_plan(index);
        let resources = self.requirements(index).into_iter()
            .map(|(resource, units)| {
                let additional = units.saturating_sub(self.held(index, &resource));
                (resource, additional)
            }).filter(|(_, units)| *units > 0).collect();
        self.jobs[index].spec = old;
        resource_plan.map_err(|reasons| error("external.resources", format!("{reasons:?}")))?;
        if !continuing {
            let handoff = transport_plan.map(Ok).or_else(|| self.recheck_material(index));
            self.stock.reserve_external(attempt.command_id.clone(), &spec, &self.input.project, self.jobs[index].exact_item.as_ref(), handoff)
                .map_err(|reasons| error("external.material", format!("{reasons:?}")))?;
            if spec.quality.is_some() {
                let item = &self.stock.external_picks(&attempt.command_id)[0].item;
                if let Err(issue) = self.inspection_attempt(index, item) {
                    self.stock.release_external(&attempt.command_id);
                    return Err(error("external.quality", issue.message));
                }
            }
        }
        let old = std::mem::replace(&mut self.jobs[index].spec, spec.clone());
        let destination_ready = spec.transport.is_some() || self.reserve_flow_destination(index);
        self.jobs[index].spec = old;
        if !destination_ready {
            self.stock.release_external(&attempt.command_id);
            return Err(error("external.output_capacity", "Physical release destination has no free capacity"));
        }
        if self.flow_destinations.contains(&index) {
            self.jobs[index].spec = spec.clone();
        }
        let command = wire::Command {
            contract_version: wire::ExecutionContractVersion::Draft11,
            identity: self.external.as_ref().unwrap().identity.clone(),
            attempt: attempt.clone(),
            device_id,
            action: if spec.transport.is_some() { wire::DeviceAction::TransportCargo }
                else if spec.preparation.is_some() || spec.setup.is_some() { wire::DeviceAction::PrepareEquipment } else { wire::DeviceAction::ProcessUnit },
            parameters: wire::CommandParameters {
                quantity: if let Some(plan) = &spec.transport {
                    let n = if continuing { self.jobs[index].inputs.iter().map(|(id, _)| amount(&self.stock.items[id].quantity)).sum() }
                        else { self.stock.external_picks(&attempt.command_id).iter().map(|p| p.amount).sum() };
                    quantity(n, &plan.supply.target_quantity.unit)
                } else if let Some(output) = spec.outputs.first() {
                    crate::phase_material::fixed_quantity(output.amount, &output.unit, output.decimal_places)
                } else { quantity(1, &"cycle".to_owned().try_into().unwrap()) },
                phase_id: attempt.phase_id.clone(),
                nominal_work_ms: uint(spec.phases[phase_index].work),
                setup_transition: spec.setup.as_ref().map(|s| s.transition()),
            },
            target_device_boot_id: boot,
        };
        let e = self.external.as_mut().unwrap();
        e.live.insert(index, attempt.command_id.clone());
        e.ordinals.insert(index, attempt.attempt_ordinal.value());
        e.attempts.insert(
            attempt.command_id.clone(),
            Attempt {
                command: command.clone(),
                index,
                phase_index,
                spec,
                resources,
                state: ExternalAttemptState::Reserved,
                started: false,
                admitted: vec![],
                unapplied: vec![],
            },
        );
        Ok(command)
    }
    pub fn mark_external_published(
        &mut self,
        command: &DispatchCommandId,
        elapsed: SafeUint,
    ) -> Result<wire::Command, ExternalError> {
        self.advance_external(elapsed)?;
        let e = self.external.as_mut().unwrap();
        let a = e
            .attempts
            .get_mut(command)
            .ok_or_else(|| error("external.command", "Unknown command"))?;
        if a.state != ExternalAttemptState::Reserved {
            return Err(error(
                "external.publication",
                "Only an unpublished reservation may cross publication barrier",
            ));
        }
        a.state = ExternalAttemptState::Published;
        Ok(a.command.clone())
    }
    pub fn withdraw_external_unpublished(
        &mut self,
        command: &DispatchCommandId,
    ) -> Result<(), ExternalError> {
        let e = self
            .external
            .as_ref()
            .ok_or_else(|| error("external.mode", "Not an external Engine"))?;
        if e.attempts
            .get(command)
            .is_none_or(|a| a.state != ExternalAttemptState::Reserved)
        {
            return Err(error(
                "external.publication",
                "Published or unknown command requires positive not-started cancellation proof",
            ));
        }
        self.external_release_unpublished(command);
        Ok(())
    }
    fn external_release_unpublished(&mut self, command: &DispatchCommandId) {
        self.stock.release_external(command);
        let e = self.external.as_mut().unwrap();
        let a = e.attempts.get_mut(command).unwrap();
        if a.spec.transport.is_none() && a.spec.op().material_flow.as_ref().and_then(|flow| flow.reserve_before_phase.as_ref()) == Some(&a.command.attempt.phase_id) {
            self.flow_destinations.remove(&a.index);
        }
        a.state = ExternalAttemptState::WithdrawnUnpublished;
        e.live.remove(&a.index);
    }

    /// The trusted Host has the cloud transaction's positive proof that this
    /// published command was never delivered. This is not a device receipt.
    pub fn confirm_external_withdrawal(
        &mut self, command: &DispatchCommandId, elapsed: SafeUint,
    ) -> Result<(), ExternalError> {
        self.advance_external(elapsed)?;
        let a = self.external.as_ref().unwrap().attempts.get(command)
            .ok_or_else(|| error("external.command", "Unknown command"))?;
        if a.state == ExternalAttemptState::CancelledNotStarted && a.admitted.is_empty() {
            return Ok(());
        }
        if a.state != ExternalAttemptState::Published || a.started || !a.unapplied.is_empty() {
            return Err(error("external.withdrawal", "Withdrawal proof conflicts with observed execution"));
        }
        self.stock.release_external(command);
        let e = self.external.as_mut().unwrap();
        let a = e.attempts.get_mut(command).unwrap();
        if a.spec.transport.is_none() && a.spec.op().material_flow.as_ref().and_then(|flow| flow.reserve_before_phase.as_ref()) == Some(&a.command.attempt.phase_id) {
            self.flow_destinations.remove(&a.index);
        }
        a.state = ExternalAttemptState::CancelledNotStarted;
        e.live.remove(&a.index);
        self.external_settle();
        Ok(())
    }

    pub fn apply_external_receipt(
        &mut self,
        receipt: wire::DeviceReceipt,
        elapsed: SafeUint,
    ) -> Result<ExternalReceiptOutcome, ExternalError> {
        self.external_check_time(elapsed.value())?;
        let e = self.external.as_ref().unwrap();
        let a = e
            .attempts
            .get(&receipt.command_id)
            .ok_or_else(|| error("external.command", "Unknown command"))?;
        if receipt.session_id != e.identity.session_id
            || receipt.execution_epoch_id != e.identity.execution_epoch_id
            || receipt.device_id != a.command.device_id
            || receipt.device_boot_id != a.command.target_device_boot_id
            || receipt.phase_id != a.command.attempt.phase_id
            || receipt.phase_token != a.command.attempt.phase_token
        {
            return Err(error(
                "external.identity",
                "Receipt session/epoch/command/device/boot/phase/token mismatch",
            ));
        }
        for (old, outcome) in &a.admitted {
            if old.device_sequence == receipt.device_sequence {
                if !same_receipt(old, &receipt) {
                    return Err(error(
                        "external.sequence",
                        "Different payload for admitted device sequence",
                    ));
                }
                let mut duplicate = outcome.clone();
                duplicate.duplicate = true;
                return Ok(duplicate);
            }
        }
        if a.unapplied.iter().any(|old| {
            old.device_sequence == receipt.device_sequence && !same_receipt(old, &receipt)
        }) {
            return Err(error(
                "external.sequence",
                "Different payload for retained device sequence",
            ));
        }
        if a.admitted
            .last()
            .is_some_and(|(old, _)| old.device_sequence.value() >= receipt.device_sequence.value())
        {
            return Err(error("external.sequence", "Old unadmitted device sequence"));
        }
        if matches!(
            a.state,
            ExternalAttemptState::Reserved
                | ExternalAttemptState::WithdrawnUnpublished
                | ExternalAttemptState::CancelledNotStarted
                | ExternalAttemptState::Completed
                | ExternalAttemptState::Failed
        ) {
            return Err(error(
                "external.state",
                "Receipt is incompatible with the frozen attempt state",
            ));
        }
        self.external_observe_time(elapsed, true)?;
        let mut a = self
            .external
            .as_mut()
            .unwrap()
            .attempts
            .remove(&receipt.command_id)
            .unwrap();
        let result = self.external_apply(&mut a, &receipt);
        self.external
            .as_mut()
            .unwrap()
            .attempts
            .insert(receipt.command_id.clone(), a);
        if result.is_ok() {
            self.external_settle();
        }
        self.external_freeze_cutoff()?;
        result
    }
    fn external_apply(
        &mut self,
        a: &mut Attempt,
        receipt: &wire::DeviceReceipt,
    ) -> Result<ExternalReceiptOutcome, ExternalError> {
        let mut outcome = ExternalReceiptOutcome {
            status: ExternalReceiptStatus::RetainedUnapplied,
            duplicate: false,
            committed_at_ms: self.now,
            command_id: receipt.command_id.clone(),
            logical_job_id: a.command.attempt.logical_job_id.clone(),
            device_reported_duration_ms: None,
        };
        match &receipt.receipt {
            wire::DeviceReceiptKind::Accepted {} if !a.started => {
                a.state = if a.unapplied.is_empty() {
                    ExternalAttemptState::Accepted
                } else {
                    ExternalAttemptState::ReconciliationRequired
                };
                outcome.status = ExternalReceiptStatus::Accepted;
            }
            wire::DeviceReceiptKind::Started {} if !a.started => {
                if a.phase_index == 0 {
                    let picks = self.stock.external_picks(&receipt.command_id);
                    let transport_picks = a.spec.transport.as_ref().map(|_| picks.to_vec());
                    let inspection = if a.spec.quality.is_some() {
                        Some(self.inspection_attempt(a.index, &picks[0].item)
                            .map_err(|issue| error("external.quality", issue.message))?)
                    } else { None };
                    let old_allocations = self.jobs[a.index].quality_handoff_from
                        .map(|_| self.stock.items[&picks[0].item].allocation_refs.clone()).unwrap_or_default();
                    let handoff = if a.spec.transport.is_some() {
                        Some(self.external_transport_plan(a.index, Some(&receipt.command_id)))
                    } else { self.recheck_material(a.index) };
                    let inputs = match self.stock.take_external(
                        &receipt.command_id,
                        &a.spec,
                        &self.input.project,
                        self.now,
                        handoff,
                    ) {
                        Ok(inputs) => inputs,
                        Err(issue) => {
                            a.state = ExternalAttemptState::ReconciliationRequired;
                            if !a.unapplied.iter().any(|old| same_receipt(old, receipt)) {
                                a.unapplied.push(receipt.clone());
                            }
                            return Err(error("external.material_changed", issue.message));
                        }
                    };
                    if let Some(picks) = transport_picks {
                        self.jobs[a.index].spec = a.spec.clone();
                        self.start_transport_cargo(a.index, inputs, &picks);
                    } else {
                    let sources = inputs
                        .iter()
                        .map(|(id, _)| {
                            self.stock.items[id]
                                .split_from_ref
                                .clone()
                                .unwrap_or_else(|| id.clone())
                        })
                        .collect();
                    self.inherit_splits(&sources);
                    self.finish_recheck_handoff(a.index, &old_allocations);
                    self.jobs[a.index].spec = a.spec.clone();
                    self.jobs[a.index].state.execution_ref = a.spec.execution.clone();
                    for (item, ordinal) in &inputs {
                        if a.spec.op().material_flow.as_ref().is_some_and(|flow| flow.acquire_after_phase.is_some()) {
                            continue;
                        }
                        self.emit(
                            object(ObjectKind::Item, item),
                            SimulationEventPayload::ItemTransferred(TransferEvent {
                                item_ref: item.clone(),
                                from_holder: self.jobs[a.index].quality_handoff_from
                                    .map(|old| HolderRef::Job(self.jobs[old].spec.id.clone()))
                                    .unwrap_or_else(|| HolderRef::Location(a.spec.inputs[*ordinal].location.clone())),
                                to_holder: HolderRef::Job(a.spec.id.clone()),
                            }),
                        );
                    }
                    self.jobs[a.index].state.input_refs =
                        inputs.iter().map(|(id, _)| id.clone()).collect();
                    self.jobs[a.index].inputs = inputs;
                    if let Some(attempt) = inspection {
                        if let Some(preset) = &attempt.preset {
                            *self.preset_cursors.entry(preset.clone()).or_default() += 1;
                        }
                        let item = self.jobs[a.index].inputs[0].0.clone();
                        self.stock.items.get_mut(&item).unwrap().quality = QualityState::Pending;
                        self.stock.record(&item, self.now);
                        self.jobs[a.index].inspection = Some(attempt);
                    }
                    self.start_article(a.index);
                    self.acquire(a.index, a.spec.job_resources.clone(), false);
                    self.jobs[a.index].state.started_at_ms = Some(time(self.now));
                    self.emit(
                        object(ObjectKind::Job, &a.spec.id),
                        SimulationEventPayload::JobStarted(a.spec.id.clone()),
                    );
                    }
                }
                if a.spec.transport.is_some() && a.phase_index == 2 {
                    self.trips.get_mut(&a.index).unwrap().phase = TransportPhase::Unloading;
                    self.emit_trip(a.index);
                }
                self.start_phase(a.index, MAX_SAFE_UINT);
                a.started = true;
                a.state = ExternalAttemptState::Started;
                outcome.status = ExternalReceiptStatus::Started;
            }
            wire::DeviceReceiptKind::Paused {} if a.started
                && a.spec.phases[a.phase_index].pause_resume
                && self.jobs[a.index].active.is_some() => {
                // A paused physical machine still owns its material and claims.
                let active = self.jobs[a.index].active.take().unwrap();
                let progress = active.progress + self.now - active.start;
                if self.now > active.start {
                    self.work.push(PhaseWorkInterval {
                        job_id: a.spec.id.clone(),
                        phase_id: a.command.attempt.phase_id.clone(),
                        interval: IntervalMs { start_ms: time(active.start), end_ms: time(self.now) },
                        effective_progress_start_ms: duration(active.progress),
                        effective_progress_end_ms: duration(progress),
                        end_cause: PhaseWorkEndCause::Interrupted,
                    });
                }
                self.jobs[a.index].progress = progress;
                self.jobs[a.index].state.state = JobStatus::Paused;
                outcome.status = ExternalReceiptStatus::Started;
            }
            wire::DeviceReceiptKind::Resumed {} if a.started
                && a.spec.phases[a.phase_index].pause_resume
                && matches!(self.jobs[a.index].state.state, JobStatus::Paused)
                && a.state != ExternalAttemptState::Failed => {
                self.start_phase(a.index, MAX_SAFE_UINT);
                outcome.status = ExternalReceiptStatus::Started;
            }
            wire::DeviceReceiptKind::Completed {
                device_reported_duration_ms,
            } if a.started => {
                let active = self.jobs[a.index]
                    .active
                    .as_mut()
                    .ok_or_else(|| error("external.phase", "No active original phase"))?;
                active.complete = true;
                active.stop = self.now;
                let token = active.token;
                self.boundary(Boundary::Work(a.index, a.phase_index, token));
                self.external.as_mut().unwrap().live.remove(&a.index);
                a.state = ExternalAttemptState::Completed;
                outcome.status = ExternalReceiptStatus::Completed;
                outcome.device_reported_duration_ms =
                    device_reported_duration_ms.as_ref().map(|n| n.value());
            }
            wire::DeviceReceiptKind::CancelledNotStarted {}
                if !a.started && a.unapplied.is_empty() =>
            {
                self.stock.release_external(&receipt.command_id);
                if a.spec.transport.is_none() && a.spec.op().material_flow.as_ref().and_then(|flow| flow.reserve_before_phase.as_ref()) == Some(&a.command.attempt.phase_id) {
                    self.flow_destinations.remove(&a.index);
                }
                a.state = ExternalAttemptState::CancelledNotStarted;
                self.external.as_mut().unwrap().live.remove(&a.index);
                outcome.status = ExternalReceiptStatus::CancelledNotStarted;
            }
            wire::DeviceReceiptKind::Completed { .. }
            | wire::DeviceReceiptKind::CancelRejectedStarted {}
                if !a.started =>
            {
                outcome.duplicate = a.unapplied.iter().any(|old| same_receipt(old, receipt));
                if !outcome.duplicate {
                    a.unapplied.push(receipt.clone());
                }
                a.state = ExternalAttemptState::ReconciliationRequired;
                return Ok(outcome); // no Stock.take, no synthetic start, no output
            }
            wire::DeviceReceiptKind::CancelRejectedStarted {} if a.started => {
                outcome.status = ExternalReceiptStatus::Started;
            }
            wire::DeviceReceiptKind::Failed { .. } => {
                if a.started {
                    if let Some(active) = self.jobs[a.index].active.take() {
                        let delta = self.now - active.start;
                        if delta > 0 {
                            self.work.push(PhaseWorkInterval {
                                job_id: a.spec.id.clone(),
                                phase_id: a.spec.phases[a.phase_index].id.clone(),
                                interval: IntervalMs {
                                    start_ms: time(active.start),
                                    end_ms: time(self.now),
                                },
                                effective_progress_start_ms: duration(active.progress),
                                effective_progress_end_ms: duration(active.progress + delta),
                                end_cause: PhaseWorkEndCause::Interrupted,
                            });
                        }
                        self.jobs[a.index].progress = active.progress + delta;
                    }
                    self.jobs[a.index].state.state = JobStatus::Paused;
                    a.state = ExternalAttemptState::Failed;
                    outcome.status = ExternalReceiptStatus::Failed;
                } else {
                    a.state = ExternalAttemptState::ReconciliationRequired;
                    a.unapplied.push(receipt.clone());
                    return Ok(outcome);
                }
            }
            _ => {
                return Err(error(
                    "external.state",
                    "Repeated/conflicting start, acceptance or cancellation cannot change original business ownership",
                ));
            }
        }
        a.unapplied
            .retain(|old| old.device_sequence != receipt.device_sequence);
        a.admitted.push((receipt.clone(), outcome.clone()));
        Ok(outcome)
    }

    /// Original compiled compatibility, shared with Host dispatch without exposing Engine ownership.
    pub fn external_candidate_bindings(&self, logical_job: &ObjectId) -> Result<Vec<ObjectId>, ExternalError> {
        let e = self.external.as_ref().ok_or_else(|| error("external.mode", "Not an external Engine"))?;
        let index = self.jobs.iter().position(|job| &job.spec.id == logical_job)
            .ok_or_else(|| error("external.logical_job", "Unknown original logical work"))?;
        Ok(e.candidates.iter().filter(|(_, binding)| binding.jobs.contains_key(&index)
            && self.jobs[index].spec.phases.get(self.jobs[index].phase)
                .is_some_and(|phase| phase.id == binding.binding.phase_id)
            && (self.jobs[index].state.started_at_ms.is_none()
                || matches!(&self.jobs[index].spec.execution, JobExecutionRef::ProjectDevice { device_id, .. } if device_id == &binding.binding.device_id)
                || self.jobs[index].spec.transport.as_ref().is_some_and(|p| p.carrier == binding.binding.device_id)))
            .map(|(id, _)| id.clone()).collect())
    }

    pub fn external_observation(&self) -> Result<ExternalObservation, ExternalError> {
        let e = self
            .external
            .as_ref()
            .ok_or_else(|| error("external.mode", "Not an external Engine"))?;
        // Existing projection reads the real same Engine/Stock; no replacement
        // Run or result is constructed to simulate an external completion.
        let actual = self.snapshot();
        let accepted: u128 = actual
            .acceptance_events
            .iter()
            .filter(|a| a.quantity.unit == e.policy.target_quantity.unit)
            .map(|a| u128::from(amount(&a.quantity)) * 10_u128.pow(9 - u32::from(a.quantity.value.decimal_places.value())))
            .sum();
        Ok(ExternalObservation {
            identity: e.identity.clone(),
            clock_started: e.activated,
            elapsed_ms: self.now,
            dispatch_stopped: e.stop_dispatch,
            drained: matches!(self.status, ComputationStatus::Completed),
            goal_met: accepted >= u128::from(amount(&e.policy.target_quantity))
                * 10_u128.pow(9 - u32::from(e.policy.target_quantity.value.decimal_places.value())),
            queued_logical_jobs: {
                // Compilation orders releases for initial preparation. After
                // device waiting, dispatch must still use priority/due before
                // release order. Engine admission retains release/predecessor
                // and material/resource checks for each selected attempt.
                let mut queued: Vec<_> = self.jobs.iter()
                    .filter(|j| !j.dormant)
                    .filter(|j| j.state.started_at_ms.is_none()
                        || j.phase < j.spec.phases.len() && (matches!(j.state.state, JobStatus::WaitingResource)
                            || matches!(j.state.state, JobStatus::WaitingOutput) && j.state.pending_outputs.is_empty())).collect();
                queued.sort_by(|a, b| b.spec.priority.cmp(&a.spec.priority)
                    .then(a.spec.due.unwrap_or(MAX_SAFE_UINT).cmp(&b.spec.due.unwrap_or(MAX_SAFE_UINT)))
                    .then(a.spec.release.cmp(&b.spec.release))
                    .then(a.spec.id.cmp(&b.spec.id)));
                queued.into_iter().map(|j| j.spec.id.clone()).collect()
            },
            logical_work: self.jobs.iter().map(|job| {
                let (step, supply) = match &job.spec.execution {
                    JobExecutionRef::ProjectDevice { route_step_id: Some(step), .. } => (Some(step.clone()), None),
                    JobExecutionRef::DefinitionCycle { .. } if job.spec.transport.is_some() => (None, Some(job.spec.transport.as_ref().unwrap().supply.id.clone())),
                    _ => return Err(error("external.logical_work", "External work requires its original route-step or supply identity")),
                };
                Ok(ExternalLogicalWork { logical_job_id: job.spec.id.clone(),
                    production_task_id: job.spec.task.clone(), route_step_id: step, supply_id: supply, cycle: job.spec.cycle,
                    allowed_binding_ids: self.external_candidate_bindings(&job.spec.id)? })
            }).collect::<Result<Vec<_>, ExternalError>>()?,
            attempts: e
                .attempts
                .values()
                .map(|a| ExternalAttemptView {
                    command: a.command.clone(),
                    state: a.state,
                    pending_items: self
                        .stock
                        .external_picks(&a.command.attempt.command_id)
                        .iter()
                        .map(|p| (p.item.clone(), p.amount))
                        .collect(),
                    pending_input_ordinals: self.stock.external_picks(&a.command.attempt.command_id)
                        .iter().map(|pick| SafeUint::try_from(pick.material as u64).unwrap()).collect(),
                    reserved_resource_units: a.resources.iter().map(|(resource_id, units)| wire::ResourceUnits {
                        resource_id: resource_id.clone(), units: wire::PositiveSafeUint::try_from(*units).unwrap(),
                    }).collect(),
                    unapplied_receipts: a.unapplied.clone(),
                })
                .collect(),
            items: actual.final_items,
            jobs: actual.jobs,
            events: actual.events,
            resource_claims: actual.resource_claims,
            phase_work_intervals: actual.phase_work_intervals,
            acceptance_events: actual.acceptance_events,
            allocations: actual.allocations,
            order_outcomes: actual.orders,
            transport: actual.transport,
        })
    }
    /// Current receiving-capacity blockers for jobs already admitted before stopping.
    pub fn external_output_capacity_waits(&self) -> impl Iterator<Item = (&ObjectId, &ObjectId)> {
        self.open_waits.iter().filter_map(|(index, wait)| {
            let job = &self.jobs[*index];
            if job.state.started_at_ms.is_none() || !matches!(job.state.state, JobStatus::WaitingOutput) {
                return None;
            }
            wait.reasons.iter().find_map(|reason| {
                let blocker = reason.blocking_object.as_ref()?;
                (matches!(reason.kind, WaitReasonKind::OutputCapacity)
                    && matches!(blocker.kind, ObjectKind::Location))
                    .then_some((&job.spec.id, &blocker.id))
            })
        })
    }

    pub fn external_cutoff_observation(&self) -> Option<&ExternalObservation> {
        self.external.as_ref()?.cutoff.as_deref()
    }
    /// Next original model event, without advancing or interpreting nominal work.
    pub fn next_external_boundary_ms(&self) -> Result<Option<SafeUint>, ExternalError> {
        if self.external.is_none() { return Err(error("external.mode", "Not an external Engine")); }
        Ok(self.queue.peek().map(|event| SafeUint::try_from(event.0.at).unwrap()))
    }
}

#[derive(Debug, Clone)]
pub struct ExternalCandidatePlan {
    pub estimated_completion_ms: Option<u64>,
    pub blocking_reasons: Vec<String>,
}

impl Engine {
    fn external_spec(&self, logical: &ObjectId, binding: &ObjectId) -> Result<(usize, &Job), ExternalError> {
        let e = self.external.as_ref().ok_or_else(|| error("external.mode", "Not an external Engine"))?;
        let index = self.jobs.iter().position(|j| &j.spec.id == logical)
            .ok_or_else(|| error("external.logical_job", "Unknown original logical work"))?;
        let spec = e.candidates.get(binding).and_then(|b| b.jobs.get(&index))
            .ok_or_else(|| error("external.binding", "Binding does not implement this original work"))?;
        Ok((index, spec))
    }
    fn external_requirements(spec: &Job, phase: usize) -> BTreeMap<ObjectId, u64> {
        let mut resources = spec.job_resources.clone();
        for (id, n) in &spec.phases[phase].resources {
            resources.entry(id.clone()).and_modify(|old| *old = (*old).max(*n)).or_insert(*n);
        }
        resources
    }
    // A forecast uses only known declared work. An overdue or unstarted external command has
    // no known release time. It remains occupied even when a forecast once predicted completion.
    fn external_claim_end(&self, index: usize) -> Option<u64> {
        let job = &self.jobs[index];
        let active = job.active.as_ref()?;
        let e = self.external.as_ref().unwrap();
        let a = e.live.get(&index).and_then(|id| e.attempts.get(id))?;
        if a.state != ExternalAttemptState::Started || a.phase_index + 1 != a.spec.phases.len() { return None; }
        active.start.checked_add(a.spec.phases[a.phase_index].work).filter(|end| *end > self.now && *end <= MAX_SAFE_UINT)
    }
    fn external_slot(&self, requirements: &BTreeMap<ObjectId, u64>, earliest: u64, work: u64, own: usize) -> Option<u64> {
        let mut boundaries = BTreeSet::from([earliest]);
        for resource in requirements.keys() {
            let r = self.input.project.resources.iter().find(|r| &r.id == resource)?;
            let c = self.input.project.calendars.iter().find(|c| c.id == r.calendar_id)?;
            for interval in &c.intervals {
                if interval.start_ms.0.value() >= earliest { boundaries.insert(interval.start_ms.0.value()); }
                if interval.end_ms.0.value() >= earliest { boundaries.insert(interval.end_ms.0.value()); }
            }
            for ((index, id, _), _) in &self.open_claims {
                if id == resource {
                    if let Some(end) = self.external_claim_end(*index).filter(|end| *end >= earliest) { boundaries.insert(end); }
                }
            }
        }
        boundaries.into_iter().find(|at| {
            let Some(end) = at.checked_add(work).filter(|end| *end <= MAX_SAFE_UINT) else { return false };
            requirements.iter().all(|(resource, needed)| {
                let r = self.input.project.resources.iter().find(|r| &r.id == resource).unwrap();
                let c = self.input.project.calendars.iter().find(|c| c.id == r.calendar_id).unwrap();
                let mut intervals: Vec<_> = c.intervals.iter().map(|i| (i.start_ms.0.value(), i.end_ms.0.value())).collect();
                intervals.sort();
                let mut open_until = *at;
                for (a,b) in intervals { if a <= open_until && b > open_until { open_until = b; } }
                if open_until == *at || end > open_until { return false; }
                let mut occupied = self.external.as_ref().unwrap().reserved_units(resource);
                for ((index, id, _), claim) in &self.open_claims {
                    if *index != own && id == resource && self.external_claim_end(*index).is_none_or(|until| until > *at) {
                        occupied += self.claims[claim.index].units.value();
                    }
                }
                occupied + needed <= amount(&r.capacity)
            })
        })
    }

    /// Forecast only: published commands and setup still need their real receipts.
    pub fn external_candidate_plan(&self, logical: &ObjectId, binding: &ObjectId) -> Result<ExternalCandidatePlan, ExternalError> {
        let (index, spec) = self.external_spec(logical, binding)?;
        let phase = self.jobs[index].phase;
        let mut plan = ExternalCandidatePlan { estimated_completion_ms: None, blocking_reasons: vec![] };
        let e = self.external.as_ref().unwrap();
        if e.live.contains_key(&index) || phase >= spec.phases.len() {
            plan.blocking_reasons.push("external.live_attempt".into()); return Ok(plan);
        }
        let mut earliest = self.now.max(spec.release);
        let family = match self.external_family_plan(spec) {
            Ok(plan) => plan, Err(reason) => { plan.blocking_reasons.push(reason); return Ok(plan); }
        };
        if let Some(preparation) = family {
            if let Some(pending) = preparation.pending {
                if !self.jobs[pending].produced {
                    let Some(end) = self.external_claim_end(pending) else {
                        plan.blocking_reasons.push("external.preparation_completion_unknown".into()); return Ok(plan);
                    };
                    earliest = earliest.max(end);
                }
            } else {
                let Some(start) = self.external_slot(&preparation.resources, earliest, preparation.work, index) else {
                    plan.blocking_reasons.push("external.preparation_resources_unknown".into()); return Ok(plan);
                };
                earliest = start.checked_add(preparation.work).filter(|v| *v <= MAX_SAFE_UINT)
                    .ok_or_else(|| error("external.estimate_overflow", "Preparation estimate exceeds safe time"))?;
            }
        }
        let requirements = Self::external_requirements(spec, phase);
        let work = spec.phases[phase].work.saturating_sub(self.jobs[index].progress);
        if let Some(start) = self.external_slot(&requirements, earliest, work, index) {
            plan.estimated_completion_ms = start.checked_add(work).filter(|v| *v <= MAX_SAFE_UINT);
        } else { plan.blocking_reasons.push("external.resource_or_calendar_completion_unknown".into()); }
        Ok(plan)
    }
}
