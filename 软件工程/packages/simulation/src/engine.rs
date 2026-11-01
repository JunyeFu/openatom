use crate::{EngineInput, compile::*, stock::Stock};
use sf_domain::*;
#[path = "disposition.rs"]
mod disposition;
#[path = "phase_flow.rs"]
mod phase_flow;
#[path = "external.rs"]
pub(crate) mod external;
#[path = "setup.rs"]
pub(crate) mod setup;
#[path = "transport.rs"]
pub(crate) mod transport;
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap, HashSet},
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Boundary {
    Work(usize, usize, u64),
    Arrival(usize),
    Quality(usize),
    Calendar,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Event {
    at: u64,
    phase: u8,
    subject: String,
    sequence: u64,
    kind: Boundary,
}
#[derive(Clone)]
struct Active {
    start: u64,
    progress: u64,
    stop: u64,
    complete: bool,
    token: u64,
}
struct RunningJob {
    spec: Job,
    state: JobState,
    phase: usize,
    progress: u64,
    active: Option<Active>,
    inputs: Vec<(ObjectId, usize)>,
    produced: bool,
    outputs: Vec<(ObjectId, usize)>,
    ready: Option<u64>,
    output_at: Option<u64>,
    dormant: bool,
    exact_item: Option<ObjectId>,
    inspection_origin: Option<usize>,
    inspection: Option<InspectionAttempt>,
    article_key: Option<ObjectId>,
    quality_handoff_from: Option<usize>,
}
#[derive(Clone)]
struct InspectionAttempt {
    preset: Option<ObjectId>,
    observed: Option<TypedValue>,
}
impl RunningJob {
    fn new(spec: Job) -> Self {
        Self {
            state: JobState {
                id: spec.id.clone(),
                task_ref: spec.task.clone(),
                operation_ref: spec.operation_ref.clone(),
                execution_ref: spec.execution.clone(),
                input_refs: vec![],
                job_resources: vec![],
                phase_ref: None,
                phase_resources: vec![],
                effective_progress_ms: duration(0),
                started_at_ms: None,
                finished_at_ms: None,
                state: JobStatus::Ready,
                pending_outputs: vec![],
            },
            dormant: spec
                .operation
                .as_ref()
                .is_some_and(|op| matches!(op.builtin, RuleKind::Rework)),
            spec,
            phase: 0,
            progress: 0,
            active: None,
            inputs: vec![],
            produced: false,
            outputs: vec![],
            ready: None,
            output_at: None,
            exact_item: None,
            inspection_origin: None,
            inspection: None,
            article_key: None,
            quality_handoff_from: None,
        }
    }
}
#[derive(Clone)]
struct OpenClaim {
    index: usize,
    start: u64,
}
#[derive(Clone)]
struct OpenWait {
    start: u64,
    reasons: Vec<WaitReason>,
    phase: Option<PhaseId>,
}

/// One mutable business state and one virtual clock. Callers may only obtain
/// immutable DTO snapshots; presentation and Host do not own parallel stock.
pub struct Engine {
    external: Option<external::ExternalExecution>,
    input: EngineInput,
    goal: Option<FactoryGenerationGoal>,
    stock: Stock,
    jobs: Vec<RunningJob>,
    completed_job_ids: HashSet<String>,
    queue: BinaryHeap<Reverse<Event>>,
    next_sequence: u64,
    now: u64,
    computed: Option<u64>,
    status: ComputationStatus,
    events: Vec<SimulationEvent>,
    claims: Vec<ResourceClaim>,
    open_claims: BTreeMap<(usize, ObjectId, String), OpenClaim>,
    work: Vec<PhaseWorkInterval>,
    waits: Vec<WaitInterval>,
    open_waits: BTreeMap<usize, OpenWait>,
    acceptance: Vec<AcceptanceEvent>,
    accepted: BTreeMap<ObjectId, u64>,
    fulfilled: BTreeMap<ObjectId, u64>,
    output_task: BTreeMap<ObjectId, ObjectId>,
    task_accepted: BTreeMap<(ObjectId, ObjectId), u64>,
    pending_arrivals: BTreeMap<usize, u64>,
    markers: BTreeSet<(ObjectId, String)>,
    limitations: Vec<Issue>,
    token: u64,
    preset_cursors: BTreeMap<ObjectId, usize>,
    pending_inspections: BTreeMap<ObjectId, usize>,
    families: BTreeMap<ObjectId, setup::Family>,
    article_gates: BTreeMap<ObjectId, setup::ArticleGate>,
    article_items: BTreeMap<ObjectId, ObjectId>,
    trips: BTreeMap<usize, TransportState>,
    shipped: BTreeMap<ObjectId, u64>,
    receipts: BTreeMap<ObjectId, ObjectId>,
    quality_transfers: BTreeMap<ObjectId, disposition::Transfer>,
    job_history: Vec<(SimTimeMs, JobState)>,
    job_history_latest: BTreeMap<ObjectId, usize>,
    port_output_history: Vec<crate::PortOutputRecord>,
    flow_destinations: BTreeSet<usize>,
}
struct CheckedPlan {
    jobs: Vec<Job>,
    limitations: Vec<Issue>,
    initial: crate::stock::InitialPlan,
}
fn checked_plan(input: &EngineInput) -> Result<CheckedPlan, Vec<Issue>> {
    let (specs, limitations) = crate::compile::compile(input)?;
    let declared: BTreeSet<_> = input
        .scenario
        .initial_conditions
        .items
        .iter()
        .chain(input.scenario.arrivals.iter().flat_map(|a| a.items.iter()))
        .map(|i| i.id.clone())
        .collect();
    let mut generated = BTreeSet::new();
    for job in &specs {
        if job
            .operation
            .as_ref()
            .is_some_and(|op| !matches!(op.identity_policy, IdentityPolicy::Preserve))
        {
            for output in &job.outputs {
                let count = if output.individual { output.amount } else { 1 };
                for n in 0..count {
                    let key = stable(
                        "output",
                        &[job.id.as_str(), output.port.as_str(), &n.to_string()],
                    );
                    if declared.contains(&key) || !generated.insert(key) {
                        return Err(vec![issue(
                            object(ObjectKind::Job, &job.id),
                            "/outputs",
                            "generated_item_identity_collision",
                            "Generated output identity conflicts with a declared initial/future identity or another output; initialization has not been committed.",
                        )]);
                    }
                }
            }
        }
    }
    let initial = crate::stock::plan_initial(input)?;
    Ok(CheckedPlan {
        jobs: specs,
        limitations,
        initial,
    })
}
/// Pure shared admission before enqueue. It compiles the same frozen rules and
/// checks planned identities as Engine::new, without creating runtime inventory.
pub fn check_inputs(input: &EngineInput) -> Result<(), Vec<Issue>> {
    crate::goals::select(input, None)?;
    checked_plan(input).map(|_| ())
}
/// Additive internal Rust handoff of an existing, explicitly frozen goal. It
/// does not write GenerationContext or add an API operation/wire field.
pub fn check_inputs_with_goal(
    input: &EngineInput,
    goal: &FactoryGenerationGoal,
) -> Result<(), Vec<Issue>> {
    crate::goals::select(input, Some(goal))?;
    checked_plan(input).map(|_| ())
}
impl Engine {
    pub fn new(input: EngineInput) -> Result<Self, Vec<Issue>> {
        Self::build(input, None)
    }
    pub fn new_with_goal(
        input: EngineInput,
        goal: FactoryGenerationGoal,
    ) -> Result<Self, Vec<Issue>> {
        Self::build(input, Some(goal))
    }
    fn build(
        input: EngineInput,
        explicit: Option<FactoryGenerationGoal>,
    ) -> Result<Self, Vec<Issue>> {
        let goal = crate::goals::select(&input, explicit.as_ref())?;
        let CheckedPlan {
            jobs: specs,
            mut limitations,
            initial,
        } = checked_plan(&input)?;
        if let Some(goal) = &goal {
            limitations.extend(
                goal.constraints
                    .iter()
                    .filter(|i| {
                        !i.blocks
                            .iter()
                            .any(|p| matches!(p, CapabilityPurpose::Execution))
                    })
                    .cloned(),
            );
        } else if input.scenario.orders.is_empty() && !input.scenario.production_tasks.is_empty() {
            let mut gap = issue(
                object(ObjectKind::Scenario, &id(input.scenario.id.as_str())),
                "/acceptance_goal",
                "goal_acceptance_missing",
                "No OrderLine, frozen Scenario acceptance_goal or other explicit product/unit/destination/stage goal was supplied. Work may run, but a finished output is not an inferred acceptance or target. Declare the real Scenario goal rather than manufacturing GenerationContext or OrderLines.",
            );
            gap.blocks = vec![CapabilityPurpose::CompleteConclusion];
            limitations.push(gap);
        }
        let stock = Stock::new(&input, initial);
        let jobs = specs.into_iter().map(RunningJob::new).collect();
        let mut engine = Self {
            external: None,
            now: input.scenario.window.start_ms.0.value(),
            input,
            goal,
            stock,
            jobs,
            completed_job_ids: HashSet::new(),
            queue: BinaryHeap::new(),
            next_sequence: 0,
            computed: None,
            status: ComputationStatus::Queued,
            events: vec![],
            claims: vec![],
            open_claims: BTreeMap::new(),
            work: vec![],
            waits: vec![],
            open_waits: BTreeMap::new(),
            acceptance: vec![],
            accepted: BTreeMap::new(),
            fulfilled: BTreeMap::new(),
            output_task: BTreeMap::new(),
            task_accepted: BTreeMap::new(),
            pending_arrivals: BTreeMap::new(),
            markers: BTreeSet::new(),
            limitations,
            token: 0,
            preset_cursors: BTreeMap::new(),
            pending_inspections: BTreeMap::new(),
            families: BTreeMap::new(),
            article_gates: BTreeMap::new(),
            article_items: BTreeMap::new(),
            trips: BTreeMap::new(),
            flow_destinations: BTreeSet::new(),
            shipped: BTreeMap::new(),
            receipts: BTreeMap::new(),
            quality_transfers: BTreeMap::new(),
            job_history: vec![],
            job_history_latest: BTreeMap::new(),
            port_output_history: vec![],
        };
        let mut boundaries = vec![];
        for (i, a) in engine.input.scenario.arrivals.iter().enumerate() {
            boundaries.push((
                a.at_ms.0.value(),
                1,
                a.id.as_str().to_owned(),
                Boundary::Arrival(i),
            ));
        }
        for (i, d) in engine
            .input
            .scenario
            .initial_conditions
            .declared_quality_decisions
            .iter()
            .enumerate()
        {
            boundaries.push((
                d.effective_at_ms.0.value(),
                1,
                d.id.as_str().to_owned(),
                Boundary::Quality(i),
            ));
        }
        for calendar in &engine.input.project.calendars {
            for interval in &calendar.intervals {
                boundaries.push((
                    interval.start_ms.0.value(),
                    1,
                    calendar.id.as_str().to_owned(),
                    Boundary::Calendar,
                ));
                boundaries.push((
                    interval.end_ms.0.value(),
                    1,
                    calendar.id.as_str().to_owned(),
                    Boundary::Calendar,
                ));
            }
        }
        for job in &engine.jobs {
            boundaries.push((
                job.spec.release,
                1,
                job.spec.id.as_str().to_owned(),
                Boundary::Calendar,
            ));
        }
        boundaries.sort();
        for supply in &engine.input.scenario.supplies {
            for window in &supply.departure_windows {
                boundaries.push((
                    window.start_ms.0.value(),
                    1,
                    supply.id.as_str().to_owned(),
                    Boundary::Calendar,
                ));
                boundaries.push((
                    window.end_ms.0.value(),
                    1,
                    supply.id.as_str().to_owned(),
                    Boundary::Calendar,
                ));
            }
            boundaries.push((
                supply.window.start_ms.0.value(),
                1,
                supply.id.as_str().to_owned(),
                Boundary::Calendar,
            ));
            boundaries.push((
                supply.window.end_ms.0.value(),
                1,
                supply.id.as_str().to_owned(),
                Boundary::Calendar,
            ));
        }
        boundaries.sort();
        for (at, phase, subject, kind) in boundaries {
            if at >= engine.now && at <= engine.end() {
                engine.push(at, phase, subject, kind);
            }
        }
        Ok(engine)
    }
    fn end(&self) -> u64 {
        self.input.scenario.window.end_ms.0.value()
    }
    fn push(&mut self, at: u64, phase: u8, subject: String, kind: Boundary) {
        let sequence = self.next_sequence;
        self.next_sequence += 1;
        self.queue.push(Reverse(Event {
            at,
            phase,
            subject,
            sequence,
            kind,
        }));
    }
    fn emit(&mut self, subject: ObjectRef, payload: SimulationEventPayload) {
        self.events.push(SimulationEvent {
            sequence: uint(self.events.len() as u64),
            at_ms: time(self.now),
            subject,
            payload,
        });
    }
    pub fn run_to_end(&mut self) {
        if self.external.is_some() { return; }
        while matches!(
            self.status,
            ComputationStatus::Queued | ComputationStatus::Running
        ) {
            self.step();
        }
    }
    pub fn is_finished(&self) -> bool {
        matches!(
            self.status,
            ComputationStatus::Completed | ComputationStatus::Failed | ComputationStatus::Cancelled
        )
    }
    pub fn pause(&mut self) {
        if matches!(
            self.status,
            ComputationStatus::Queued | ComputationStatus::Running
        ) {
            self.status = ComputationStatus::Paused;
        }
    }
    pub fn resume(&mut self) {
        if matches!(self.status, ComputationStatus::Paused) {
            self.status = ComputationStatus::Running;
        }
    }
    pub fn cancel(&mut self) {
        if self.external.is_some() {
            self.external_stop_dispatch();
            return;
        }
        if !matches!(
            self.status,
            ComputationStatus::Completed | ComputationStatus::Failed | ComputationStatus::Cancelled
        ) {
            self.status = ComputationStatus::Cancelled;
        }
    }
    /// Advance one complete virtual instant, including causally generated zero
    /// duration completions and the resulting transfers and dispatches.
    pub fn step(&mut self) -> usize {
        if self.external.is_some() { return 0; }
        if !matches!(
            self.status,
            ComputationStatus::Queued | ComputationStatus::Running
        ) {
            return 0;
        }
        self.status = ComputationStatus::Running;
        if self.computed.is_some() {
            self.now = self
                .queue
                .peek()
                .map(|e| e.0.at)
                .unwrap_or(self.end())
                .min(self.end());
        }
        let mut processed = 0;
        loop {
            while self.queue.peek().is_some_and(|e| e.0.at == self.now) {
                let event = self.queue.pop().unwrap().0;
                self.boundary(event.kind);
                processed += 1;
            }
            if matches!(self.status, ComputationStatus::Failed) {
                break;
            }
            if self.try_arrival_one() {
                processed += 1;
                continue;
            }
            if self.transfer_one() {
                processed += 1;
                continue;
            }
            if self.receive_transport_one() || self.commit_setup_one() {
                processed += 1;
                continue;
            }
            self.accept_orders();
            if self.dispatch_one() {
                processed += 1;
                continue;
            }
            break;
        }
        self.computed = Some(self.now);
        self.record_job_history();
        if self.now == self.end() && !matches!(self.status, ComputationStatus::Failed) {
            self.status = ComputationStatus::Completed;
        }
        processed
    }
    fn boundary(&mut self, boundary: Boundary) {
        match boundary {
            Boundary::Calendar => {}
            Boundary::Work(index, phase, token) => {
                let Some(active) = self.jobs[index].active.clone() else {
                    return;
                };
                if active.token != token || self.jobs[index].phase != phase {
                    return;
                }
                let delta = self.now - active.start;
                if delta > 0 {
                    self.work.push(PhaseWorkInterval {
                        job_id: self.jobs[index].spec.id.clone(),
                        phase_id: self.jobs[index].spec.phases[phase].id.clone(),
                        interval: IntervalMs {
                            start_ms: time(active.start),
                            end_ms: time(self.now),
                        },
                        effective_progress_start_ms: duration(active.progress),
                        effective_progress_end_ms: duration(active.progress + delta),
                        end_cause: if active.complete {
                            PhaseWorkEndCause::PhaseCompleted
                        } else {
                            PhaseWorkEndCause::Interrupted
                        },
                    });
                }
                self.jobs[index].progress = active.progress + delta;
                self.jobs[index].active = None;
                self.release_claims(index, false);
                if active.complete {
                    if phase + 1 == self.jobs[index].spec.phases.len() {
                        let holds = self.jobs[index].spec.output_holds.clone();
                        self.acquire(index, holds.clone(), false);
                        for resource in holds.keys() {
                            if let Some(open) =
                                self.open_claims
                                    .get(&(index, resource.clone(), String::new()))
                            {
                                self.claims[open.index].kind = ResourceClaimKind::OutputHold;
                            }
                        }
                    }
                    let phase_id = self.jobs[index].spec.phases[phase].id.clone();
                    self.emit(
                        object(ObjectKind::Job, &self.jobs[index].spec.id),
                        SimulationEventPayload::PhaseCompleted(phase_id.clone()),
                    );
                    self.jobs[index].phase += 1;
                    self.jobs[index].progress = 0;
                    if self.jobs[index].spec.transport.is_some() {
                        self.transport_phase_completed(index, phase);
                        return;
                    }
                    self.apply_flow_boundary(index, &phase_id);
                    if self.jobs[index].phase == self.jobs[index].spec.phases.len() {
                        if self.jobs[index].produced {
                            if self.jobs[index].state.pending_outputs.is_empty() {
                                self.finish(index);
                            }
                        } else {
                            self.produce(index);
                        }
                    } else if !self.jobs[index].produced {
                        self.jobs[index].state.state = JobStatus::WaitingResource;
                        self.jobs[index].state.phase_ref = Some(
                            self.jobs[index].spec.phases[self.jobs[index].phase]
                                .id
                                .clone(),
                        );
                    }
                } else {
                    self.jobs[index].state.state = JobStatus::Paused;
                }
            }
            Boundary::Arrival(index) => {
                self.pending_arrivals.insert(index, self.now);
            }
            Boundary::Quality(index) => {
                let decision = self
                    .input
                    .scenario
                    .initial_conditions
                    .declared_quality_decisions[index]
                    .clone();
                self.apply_declared_quality(decision);
            }
        }
    }
    fn arrival_wait(&self, index: usize, start: u64, cause: WaitEndCause) -> WaitInterval {
        let arrival = &self.input.scenario.arrivals[index];
        WaitInterval {
            object_ref: object(ObjectKind::Task, &arrival.id),
            execution_ref: None,
            phase_ref: None,
            interval: IntervalMs {
                start_ms: time(start),
                end_ms: time(self.now),
            },
            reasons: vec![WaitReason {
                kind: WaitReasonKind::ReceiverCapacity,
                blocking_object: Some(object(ObjectKind::Location, &arrival.destination)),
                required: "Declared arrival must physically fit before it becomes local stock"
                    .into(),
                next_known_boundary_ms: None,
            }],
            end_cause: cause,
        }
    }
    fn try_arrival_one(&mut self) -> bool {
        let mut pending: Vec<_> = self
            .pending_arrivals
            .keys()
            .map(|i| {
                (
                    self.input.scenario.arrivals[*i].at_ms.0.value(),
                    self.input.scenario.arrivals[*i].id.clone(),
                    *i,
                )
            })
            .collect();
        pending.sort();
        for (_, _, index) in pending {
            let arrival = self.input.scenario.arrivals[index].clone();
            let composed: BTreeSet<_> = arrival
                .items
                .iter()
                .flat_map(|i| i.components.iter().cloned())
                .collect();
            let mut totals = BTreeMap::<(ObjectId, UnitId), u128>::new();
            for item in &arrival.items {
                if !composed.contains(&item.id) {
                    if let HolderRef::Location(location) = &item.holder {
                        *totals
                            .entry((location.clone(), item.quantity.unit.clone()))
                            .or_default() += amount(&item.quantity) as u128;
                    }
                }
            }
            let fits = totals.iter().all(|((location, unit), needed)| {
                self.input
                    .project
                    .locations
                    .iter()
                    .find(|l| l.id == *location)
                    .and_then(|l| l.capacity.as_ref())
                    .is_some_and(|q| {
                        q.unit == *unit
                            && (self.stock.location_occupied(location, unit)
                                + self.flow_reserved(location, unit, None))
                            .checked_add(*needed)
                            .is_some_and(|n| n <= amount(q) as u128)
                    })
            });
            if !fits {
                continue;
            }
            let start = self.pending_arrivals.remove(&index).unwrap();
            if start < self.now {
                let wait = self.arrival_wait(index, start, WaitEndCause::Resolved);
                self.waits.push(wait.clone());
                self.emit(
                    object(ObjectKind::Task, &arrival.id),
                    SimulationEventPayload::WaitChanged(wait),
                );
            }
            let mut receipt = arrival.items.clone();
            if matches!(arrival.release_policy, ReleasePolicy::InspectionRequired) {
                for item in &mut receipt {
                    if matches!(item.quality, QualityState::Unchecked) {
                        item.quality = QualityState::Pending;
                    }
                }
            }
            self.stock.insert_group(&receipt, self.now);
            self.emit(
                object(ObjectKind::Task, &arrival.id),
                SimulationEventPayload::ArrivalAccepted(arrival.id),
            );
            return true;
        }
        false
    }
    fn calendar_end(&self, resource: &ObjectId) -> Option<u64> {
        let resource = self
            .input
            .project
            .resources
            .iter()
            .find(|r| r.id == *resource)
            .unwrap();
        let calendar = self
            .input
            .project
            .calendars
            .iter()
            .find(|c| c.id == resource.calendar_id)
            .unwrap();
        let mut intervals: Vec<_> = calendar
            .intervals
            .iter()
            .map(|i| (i.start_ms.0.value(), i.end_ms.0.value()))
            .collect();
        intervals.sort();
        let mut merged: Vec<(u64, u64)> = vec![];
        for (a, b) in intervals {
            if let Some(last) = merged.last_mut() {
                if a <= last.1 {
                    last.1 = last.1.max(b);
                    continue;
                }
            }
            merged.push((a, b));
        }
        merged
            .into_iter()
            .find(|(a, b)| *a <= self.now && self.now < *b)
            .map(|(_, b)| b)
    }
    fn held(&self, index: usize, resource: &ObjectId) -> u64 {
        self.open_claims
            .iter()
            .filter(|((j, r, _), _)| *j == index && r == resource)
            .map(|(_, c)| self.claims[c.index].units.value())
            .sum()
    }
    fn occupied(&self, resource: &ObjectId) -> u64 {
        self.open_claims
            .iter()
            .filter(|((_, r, _), _)| r == resource)
            .map(|(_, c)| self.claims[c.index].units.value())
            .sum::<u64>() + self.external.as_ref().map_or(0, |e| e.reserved_units(resource))
    }
    fn requirements(&self, index: usize) -> BTreeMap<ObjectId, u64> {
        let job = &self.jobs[index];
        let mut result = job.spec.job_resources.clone();
        for (r, n) in &job.spec.phases[job.phase].resources {
            result
                .entry(r.clone())
                .and_modify(|m| *m = (*m).max(*n))
                .or_insert(*n);
        }
        result
    }
    fn resource_plan(&self, index: usize) -> Result<u64, Vec<WaitReason>> {
        let job = &self.jobs[index];
        let phase = &job.spec.phases[job.phase];
        let mut reasons = vec![];
        let mut until = MAX_SAFE_UINT;
        for (resource, needed) in self.requirements(index) {
            let r = self
                .input
                .project
                .resources
                .iter()
                .find(|r| r.id == resource)
                .unwrap();
            let inherited = self.jobs[index]
                .quality_handoff_from
                .map(|old| self.held(old, &resource))
                .unwrap_or(0);
            if self.occupied(&resource) - self.held(index, &resource) - inherited + needed
                > amount(&r.capacity)
            {
                reasons.push(WaitReason {
                    kind: WaitReasonKind::ResourceBusy,
                    blocking_object: Some(object(ObjectKind::Resource, &resource)),
                    required: format!("{needed} {}", r.capacity.unit.as_str()),
                    next_known_boundary_ms: None,
                });
            }
            match self.calendar_end(&resource) {
                Some(end) => until = until.min(end),
                None => reasons.push(WaitReason {
                    kind: WaitReasonKind::Calendar,
                    blocking_object: Some(object(ObjectKind::Resource, &resource)),
                    required: "A declared open calendar interval for this phase".into(),
                    next_known_boundary_ms: None,
                }),
            }
        }
        if !phase.pause_resume && until.saturating_sub(self.now) < phase.work - job.progress {
            reasons.push(WaitReason {
                kind: WaitReasonKind::Calendar,
                blocking_object: None,
                required: format!(
                    "{} continuous ms for phase {}",
                    phase.work - job.progress,
                    phase.id.as_str()
                ),
                next_known_boundary_ms: None,
            });
        }
        if reasons.is_empty() {
            Ok(until)
        } else {
            Err(reasons)
        }
    }
    fn acquire(&mut self, index: usize, resources: BTreeMap<ObjectId, u64>, phase: bool) {
        for (resource, needed) in resources {
            let additional = needed.saturating_sub(self.held(index, &resource));
            if additional == 0 {
                continue;
            }
            let phase_key = if phase {
                self.jobs[index].spec.phases[self.jobs[index].phase]
                    .id
                    .as_str()
                    .to_owned()
            } else {
                String::new()
            };
            let claim_id = stable(
                "claim",
                &[
                    self.jobs[index].spec.id.as_str(),
                    resource.as_str(),
                    &phase_key,
                ],
            );
            let claim = self
                .claims
                .iter()
                .position(|c| c.id == claim_id)
                .unwrap_or_else(|| {
                    self.claims.push(ResourceClaim {
                        id: claim_id,
                        resource_id: resource.clone(),
                        job_id: self.jobs[index].spec.id.clone(),
                        units: uint(additional),
                        intervals: vec![],
                        kind: if phase {
                            ResourceClaimKind::Phase
                        } else {
                            ResourceClaimKind::Job
                        },
                        phase_ref: if phase {
                            Some(
                                self.jobs[index].spec.phases[self.jobs[index].phase]
                                    .id
                                    .clone(),
                            )
                        } else {
                            None
                        },
                        interval_end_causes: vec![],
                    });
                    self.claims.len() - 1
                });
            self.open_claims.insert(
                (index, resource, phase_key),
                OpenClaim {
                    index: claim,
                    start: self.now,
                },
            );
        }
    }
    fn release_claims(&mut self, index: usize, all: bool) {
        let keys: Vec<_> = self
            .open_claims
            .keys()
            .filter(|(j, _, phase)| *j == index && (all || !phase.is_empty()))
            .cloned()
            .collect();
        for key in keys {
            let open = self.open_claims.remove(&key).unwrap();
            if open.start < self.now {
                self.claims[open.index].intervals.push(IntervalMs {
                    start_ms: time(open.start),
                    end_ms: time(self.now),
                });
                self.claims[open.index]
                    .interval_end_causes
                    .push(ClaimIntervalEndCause::Released);
            }
            self.emit(
                object(ObjectKind::Resource, &key.1),
                SimulationEventPayload::ResourceReleased(key.1),
            );
        }
    }
    fn wait_signature(reasons: &[WaitReason]) -> Vec<String> {
        reasons
            .iter()
            .map(|r| format!("{:?}:{:?}:{}", r.kind, r.blocking_object, r.required))
            .collect()
    }
    fn close_wait(&mut self, index: usize, cause: WaitEndCause) {
        if let Some(open) = self.open_waits.remove(&index) {
            if open.start < self.now {
                let wait = WaitInterval {
                    object_ref: object(ObjectKind::Job, &self.jobs[index].spec.id),
                    execution_ref: Some(self.jobs[index].spec.execution.clone()),
                    phase_ref: open.phase,
                    interval: IntervalMs {
                        start_ms: time(open.start),
                        end_ms: time(self.now),
                    },
                    reasons: open.reasons,
                    end_cause: cause,
                };
                self.waits.push(wait.clone());
                self.emit(
                    object(ObjectKind::Job, &self.jobs[index].spec.id),
                    SimulationEventPayload::WaitChanged(wait),
                );
            }
        }
    }
    fn set_wait(&mut self, index: usize, reasons: Vec<WaitReason>, output: bool) {
        let changed = self
            .open_waits
            .get(&index)
            .is_some_and(|w| Self::wait_signature(&w.reasons) != Self::wait_signature(&reasons));
        if changed {
            self.close_wait(index, WaitEndCause::ConditionChanged);
        }
        let phase = if output {
            None
        } else {
            Some(
                self.jobs[index].spec.phases[self.jobs[index].phase]
                    .id
                    .clone(),
            )
        };
        self.open_waits.entry(index).or_insert(OpenWait {
            start: self.now,
            reasons,
            phase,
        });
    }
    fn dispatch_one(&mut self) -> bool {
        let mut candidates = vec![];
        for index in 0..self.jobs.len() {
            let job = &self.jobs[index];
            if job.dormant
                || job.active.is_some()
                || (job.produced
                    && (!self.has_flow_tail(index) || !job.state.pending_outputs.is_empty()))
                || matches!(job.state.state, JobStatus::Completed | JobStatus::Cancelled)
                || job.spec.release > self.now
            {
                continue;
            }
            if !job.spec.predecessors.iter().all(|id| self.completed_job_ids.contains(id.as_str())) {
                continue;
            }
            if self.jobs[index].ready.is_none() {
                self.jobs[index].ready = Some(self.now);
            }
            let job = &self.jobs[index];
            candidates.push((
                Reverse(job.spec.priority),
                job.spec.due.unwrap_or(MAX_SAFE_UINT),
                job.ready.unwrap(),
                job.spec.id.clone(),
                index,
            ));
        }
        candidates.sort();
        // Failed candidates leave Stock unchanged; any acquisition ends this call.
        let mut material_candidates = None;
        for (_, _, _, _, index) in candidates {
            if self.jobs[index].spec.transport.is_some() {
                if self.dispatch_transport(index) {
                    return true;
                }
                continue;
            }
            if let Some(created) = self.prepare_family(index) {
                if created {
                    return true;
                }
                continue;
            }
            let first = self.jobs[index].state.started_at_ms.is_none();
            let material = if first {
                self.recheck_material(index).unwrap_or_else(|| {
                    let available = material_candidates
                        .get_or_insert_with(|| self.stock.plan_candidates());
                    self.stock.plan(
                        &self.jobs[index].spec,
                        &self.input.project,
                        self.jobs[index].exact_item.as_ref(),
                        available,
                    )
                })
            } else {
                Ok(vec![])
            };
            let resources = self.resource_plan(index);
            let mut reasons = vec![];
            if let Err(r) = &material {
                reasons.extend(r.clone());
            }
            if let Err(r) = &resources {
                reasons.extend(r.clone());
            }
            if let Some(preparation) = &self.jobs[index].spec.preparation {
                for marker in &preparation.prerequisites {
                    if !self.markers.contains(marker) {
                        reasons.push(WaitReason {
                            kind: WaitReasonKind::Preparation,
                            blocking_object: Some(object(ObjectKind::Device, &marker.0)),
                            required: marker.1.clone(),
                            next_known_boundary_ms: None,
                        });
                    }
                }
            }
            if !reasons.is_empty() {
                self.jobs[index].state.state = JobStatus::WaitingResource;
                self.set_wait(index, reasons, false);
                continue;
            }
            if !self.reserve_flow_destination(index) {
                continue;
            }
            let inspection = if first && self.jobs[index].spec.quality.is_some() {
                match self.inspection_attempt(index, &material.as_ref().unwrap()[0].item) {
                    Ok(attempt) => Some(attempt),
                    Err(issue) => {
                        self.limitations.push(issue);
                        self.status = ComputationStatus::Failed;
                        return false;
                    }
                }
            } else {
                None
            };
            self.close_wait(index, WaitEndCause::Resolved);
            if first {
                let picks = material.unwrap();
                let old_allocations = self.jobs[index]
                    .quality_handoff_from
                    .map(|_| self.stock.items[&picks[0].item].allocation_refs.clone())
                    .unwrap_or_default();
                let sources = picks.iter().map(|p| p.item.clone()).collect();
                let inputs = match self.stock.take(&self.jobs[index].spec, &picks, self.now) {
                    Ok(inputs) => inputs,
                    Err(issue) => {
                        self.limitations.push(issue);
                        self.status = ComputationStatus::Failed;
                        return false;
                    }
                };
                self.inherit_splits(&sources);
                self.finish_recheck_handoff(index, &old_allocations);
                for (item, material) in &inputs {
                    if self.jobs[index]
                        .spec
                        .op()
                        .material_flow
                        .as_ref()
                        .is_some_and(|flow| flow.acquire_after_phase.is_some())
                    {
                        continue;
                    }
                    let transfer = TransferEvent {
                        item_ref: item.clone(),
                        from_holder: self.jobs[index]
                            .quality_handoff_from
                            .map(|old| HolderRef::Job(self.jobs[old].spec.id.clone()))
                            .unwrap_or_else(|| {
                                HolderRef::Location(
                                    self.jobs[index].spec.inputs[*material].location.clone(),
                                )
                            }),
                        to_holder: HolderRef::Job(self.jobs[index].spec.id.clone()),
                    };
                    self.emit(
                        object(ObjectKind::Item, item),
                        SimulationEventPayload::ItemTransferred(transfer),
                    );
                }
                self.jobs[index].state.input_refs =
                    inputs.iter().map(|(id, _)| id.clone()).collect();
                self.jobs[index].inputs = inputs;
                if let Some(attempt) = inspection {
                    if let Some(preset) = &attempt.preset {
                        *self.preset_cursors.entry(preset.clone()).or_default() += 1;
                    }
                    let item = self.jobs[index].inputs[0].0.clone();
                    self.stock.items.get_mut(&item).unwrap().quality = QualityState::Pending;
                    self.stock.record(&item, self.now);
                    self.jobs[index].inspection = Some(attempt);
                }
                self.start_article(index);
                self.acquire(index, self.jobs[index].spec.job_resources.clone(), false);
                self.jobs[index].state.started_at_ms = Some(time(self.now));
                self.emit(
                    object(ObjectKind::Job, &self.jobs[index].spec.id),
                    SimulationEventPayload::JobStarted(self.jobs[index].spec.id.clone()),
                );
            }
            self.start_phase(index, resources.unwrap());
            return true;
        }
        false
    }
    fn start_phase(&mut self, index: usize, until: u64) {
        self.acquire(index, self.requirements(index), true);
        if self.external.is_some() {
            self.token += 1;
            self.jobs[index].active = Some(Active { start: self.now, progress: self.jobs[index].progress,
                stop: MAX_SAFE_UINT, complete: false, token: self.token });
            self.jobs[index].state.phase_ref = Some(self.jobs[index].spec.phases[self.jobs[index].phase].id.clone());
            self.jobs[index].state.state = JobStatus::Working;
            return; // no nominal Boundary::Work is scheduled for an external phase
        }
        let phase = &self.jobs[index].spec.phases[self.jobs[index].phase];
        let remaining = phase.work - self.jobs[index].progress;
        let delta = remaining.min(until - self.now);
        self.token += 1;
        let active = Active {
            start: self.now,
            progress: self.jobs[index].progress,
            stop: self.now + delta,
            complete: remaining == delta,
            token: self.token,
        };
        self.jobs[index].state.phase_ref = Some(phase.id.clone());
        self.jobs[index].state.state = JobStatus::Working;
        self.jobs[index].active = Some(active.clone());
        self.push(
            active.stop,
            0,
            self.jobs[index].spec.id.as_str().to_owned(),
            Boundary::Work(index, self.jobs[index].phase, active.token),
        );
    }
    // A material split preserves production/acceptance ownership on both the
    // taken and remaining children. The transformed parent is historical.
    fn inherit_splits(&mut self, sources: &BTreeSet<ObjectId>) {
        for source in sources {
            if !matches!(self.stock.items[source].lifecycle, Lifecycle::Transformed) {
                continue;
            }
            let mut children: Vec<_> = self
                .stock
                .items
                .values()
                .filter(|i| i.split_from_ref.as_ref() == Some(source))
                .map(|i| i.id.clone())
                .collect();
            children.sort();
            let mut counted = self.fulfilled.get(source).copied().unwrap_or(0);
            for child in children {
                if let Some(task) = self.output_task.get(source).cloned() {
                    self.output_task.insert(child.clone(), task);
                }
                let n = counted.min(amount(&self.stock.items[&child].quantity));
                if n > 0 {
                    self.fulfilled.insert(child, n);
                    counted -= n;
                }
            }
        }
    }
    fn lineage(&self, item: &ObjectId) -> BTreeSet<ObjectId> {
        let mut ancestors = BTreeSet::new();
        let mut pending = vec![item.clone()];
        while let Some(id) = pending.pop() {
            if !ancestors.insert(id.clone()) {
                continue;
            }
            if let Some(parent) = self.stock.items.get(&id) {
                pending.extend(parent.composition_refs.iter().cloned());
                pending.extend(parent.split_from_ref.iter().cloned());
            }
        }
        ancestors
    }
    fn declared_source_matches(&self, decision: &QualityDecision) -> bool {
        match (&decision.source_kind, &decision.decision_source) {
            (QualitySourceKind::Manual, DecisionSource::HumanStatement(text)) => {
                !text.trim().is_empty()
            }
            (QualitySourceKind::Preset, DecisionSource::ScheduledPreset(id)) => {
                let lineage = self.lineage(&decision.item_ref);
                self.input
                    .scenario
                    .quality_presets
                    .iter()
                    .filter(|p| {
                        p.id == *id
                            && same_module(&p.inspection_module, &decision.inspection_ref)
                            && match p.match_kind {
                                QualityMatchKind::ExactItem => {
                                    p.item_or_ancestor == decision.item_ref
                                }
                                QualityMatchKind::Ancestor | QualityMatchKind::Batch => {
                                    lineage.contains(&p.item_or_ancestor)
                                }
                            }
                    })
                    .count()
                    == 1
            }
            (QualitySourceKind::ToolResult, DecisionSource::ProviderResult(id)) => {
                self.input.project.artifact_refs.contains(id)
            }
            _ => false,
        }
    }
    fn inspection_attempt(
        &self,
        index: usize,
        item: &ObjectId,
    ) -> Result<InspectionAttempt, Issue> {
        let job = &self.jobs[index];
        let rule = &job.spec.quality.as_ref().unwrap().rule;
        if !matches!(rule.source_kind, QualitySourceKind::Preset) {
            return Ok(InspectionAttempt {
                preset: None,
                observed: None,
            });
        }
        let ancestors = self.lineage(item);
        let sources: Vec<_> = self
            .input
            .scenario
            .quality_presets
            .iter()
            .filter(|p| {
                same_module(&p.inspection_module, &job.spec.operation_ref)
                    && rule
                        .result_ref
                        .as_ref()
                        .is_none_or(|reference| p.id == *reference)
                    && match p.match_kind {
                        QualityMatchKind::ExactItem => p.item_or_ancestor == *item,
                        QualityMatchKind::Ancestor | QualityMatchKind::Batch => {
                            ancestors.contains(&p.item_or_ancestor)
                        }
                    }
            })
            .collect();
        if sources.len() > 1 {
            return Err(issue(
                object(ObjectKind::Item, item),
                "/quality_presets",
                "quality_preset_ambiguous",
                "More than one declared preset matches the actual physical identity/ancestry and exact inspection module; no material or resource has been taken.",
            ));
        }
        Ok(match sources.first() {
            Some(p) => InspectionAttempt {
                preset: Some(p.id.clone()),
                observed: p
                    .results
                    .get(self.preset_cursors.get(&p.id).copied().unwrap_or(0))
                    .cloned()
                    .flatten(),
            },
            None => InspectionAttempt {
                preset: None,
                observed: None,
            },
        })
    }
    fn quality_output(&mut self, index: usize, item: ObjectId, ordinal: usize) {
        self.record_port_output(index, &item, ordinal, false);
        self.jobs[index].state.pending_outputs = vec![item.clone()];
        self.jobs[index].outputs = vec![(item.clone(), ordinal)];
        self.jobs[index].produced = true;
        self.jobs[index].output_at = Some(self.now);
        self.jobs[index].state.phase_ref = None;
        self.jobs[index].state.state = JobStatus::WaitingOutput;
        self.output_task
            .insert(item, self.jobs[index].spec.task.clone());
    }
    fn complete_inspection(&mut self, index: usize) {
        let attempt = self.jobs[index].inspection.clone().unwrap();
        let item = self.jobs[index].inputs[0].0.clone();
        let outcome = crate::quality::outcome(attempt.observed.as_ref());
        if outcome == crate::quality::Outcome::Unknown {
            self.stock.items.get_mut(&item).unwrap().lifecycle = Lifecycle::Available;
            self.stock.record(&item, self.now);
            self.pending_inspections.insert(item.clone(), index);
            let pending = self.jobs[index]
                .spec
                .quality
                .as_ref()
                .unwrap()
                .pending
                .clone();
            if let Some(location) = pending {
                let mut material = self.jobs[index].spec.inputs[0].clone();
                material.location = location;
                material.capacity = None;
                let ordinal = self.jobs[index].spec.outputs.len();
                self.jobs[index].spec.outputs.push(material);
                self.quality_output(index, item, ordinal);
            } else {
                self.jobs[index].produced = true;
                self.jobs[index].output_at = Some(self.now);
                self.jobs[index].state.phase_ref = None;
                self.jobs[index].state.state = JobStatus::WaitingOutput;
                self.jobs[index].state.pending_outputs = vec![item.clone()];
                self.set_wait(index,vec![WaitReason{kind:WaitReasonKind::Quality,blocking_object:Some(object(ObjectKind::Item,&item)),required:"A declared result for this exact item and inspection module; UNKNOWN holds the item and Job resources".into(),next_known_boundary_ms:None}],true);
            }
            return;
        }
        let decision = QualityDecision {
            id: stable(
                "quality-decision",
                &[self.jobs[index].spec.id.as_str(), item.as_str()],
            ),
            item_ref: item,
            inspection_ref: self.jobs[index].spec.operation_ref.clone(),
            source_kind: QualitySourceKind::Preset,
            observed_result: attempt.observed,
            disposition: if outcome == crate::quality::Outcome::Pass {
                QualityDisposition::Release
            } else {
                QualityDisposition::Rework
            },
            effective_at_ms: time(self.now),
            decision_source: DecisionSource::ScheduledPreset(attempt.preset.unwrap()),
        };
        self.route_quality(index, decision);
    }
    fn route_quality(&mut self, index: usize, mut decision: QualityDecision) {
        let policy = self.jobs[index].spec.quality.clone().unwrap();
        let item = decision.item_ref.clone();
        let rework = matches!(decision.disposition, QualityDisposition::Rework)
            && self.stock.items[&item].completed_reworks.value()
                < policy.rule.max_completed_reworks.value()
            && policy.rework.is_some();
        let ordinal = if matches!(decision.disposition, QualityDisposition::Release) {
            policy.pass
        } else if rework {
            policy.fail
        } else {
            decision.disposition = QualityDisposition::Isolate;
            policy.isolation
        };
        let state = self.stock.items.get_mut(&item).unwrap();
        state.lifecycle = Lifecycle::Available;
        state.quality = if matches!(decision.disposition, QualityDisposition::Release) {
            QualityState::Released
        } else {
            QualityState::Rejected
        };
        self.stock.record(&item, self.now);
        self.close_wait(index, WaitEndCause::Resolved);
        self.release_article(&decision);
        self.emit(
            object(ObjectKind::Item, &item),
            SimulationEventPayload::QualityChanged(decision),
        );
        if matches!(self.jobs[index].state.state, JobStatus::Completed) {
            self.queue_quality_transfer(index, item.clone(), ordinal);
        } else {
            self.quality_output(index, item.clone(), ordinal);
        }
        self.pending_inspections.remove(&item);
        if rework {
            let template = self
                .jobs
                .iter()
                .find(|j| j.spec.id == *policy.rework.as_ref().unwrap())
                .unwrap();
            let mut spec = template.spec.clone();
            spec.id = stable(
                "rework",
                &[
                    spec.id.as_str(),
                    item.as_str(),
                    &(self.stock.items[&item].completed_reworks.value() + 1).to_string(),
                ],
            );
            // A later explicit instruction can refer to an already queued
            // automatic attempt; it cannot create a second Job with that ID.
            if self.jobs.iter().any(|job| job.spec.id == spec.id) {
                return;
            }
            spec.predecessors.push(self.jobs[index].spec.id.clone());
            spec.release = self.now;
            let mut next = RunningJob::new(spec);
            next.dormant = false;
            next.exact_item = Some(item);
            next.inspection_origin = Some(self.jobs[index].inspection_origin.unwrap_or(index));
            self.jobs.push(next);
        }
    }
    fn produce(&mut self, index: usize) {
        if self.jobs[index].spec.setup.is_some() {
            self.jobs[index].produced = true;
            self.jobs[index].state.phase_ref = None;
            self.release_claims(index, true);
            self.jobs[index].state.state = JobStatus::WaitingResource;
            return;
        }
        if self.jobs[index].spec.quality.is_some() {
            self.complete_inspection(index);
            return;
        }
        let spec = self.jobs[index].spec.clone();
        let inputs = self.jobs[index].inputs.clone();
        let mut generated = vec![];
        if !matches!(spec.op().identity_policy, IdentityPolicy::Preserve) {
            for output in &spec.outputs {
                let count = if output.individual { output.amount } else { 1 };
                for n in 0..count {
                    generated.push(stable(
                        "output",
                        &[spec.id.as_str(), output.port.as_str(), &n.to_string()],
                    ));
                }
            }
        }
        if let Err(issue) = self.stock.can_create(&generated, &spec.id) {
            self.limitations.push(issue);
            self.status = ComputationStatus::Failed;
            return;
        }
        let mut outputs = vec![];
        if let Some(preparation) = &spec.preparation {
            self.markers
                .insert((preparation.target.clone(), preparation.marker.clone()));
        }
        if matches!(spec.op().identity_policy, IdentityPolicy::Preserve) {
            for (item, ordinal) in &inputs {
                if spec
                    .op()
                    .material_flow
                    .as_ref()
                    .is_some_and(|flow| spec.inputs[*ordinal].role != flow.primary_role)
                {
                    continue;
                }
                self.stock.items.get_mut(item).unwrap().lifecycle = Lifecycle::Available;
                if matches!(spec.op().builtin, RuleKind::Rework) {
                    let item = self.stock.items.get_mut(item).unwrap();
                    item.completed_reworks = uint(item.completed_reworks.value() + 1);
                    item.quality = QualityState::Unchecked;
                }
                if let Some(quality) = spec.outputs[0].output_quality.clone() {
                    self.stock.items.get_mut(item).unwrap().quality = quality;
                }
                self.stock.record(item, self.now);
                outputs.push((item.clone(), 0));
            }
        } else if matches!(
            spec.op().identity_policy,
            IdentityPolicy::ContainerWithContents
        ) {
            let mapping = spec.op().pack_identity.as_ref().unwrap();
            let carrier = stable(
                "output",
                &[spec.id.as_str(), spec.outputs[0].port.as_str(), "0"],
            );
            let contents: Vec<_> = inputs
                .iter()
                .filter(|(_, row)| mapping.content_roles.contains(&spec.inputs[*row].role))
                .map(|(item, _)| item.clone())
                .collect();
            let materials: Vec<_> = inputs
                .iter()
                .filter(|(_, row)| {
                    mapping
                        .container_material_roles
                        .contains(&spec.inputs[*row].role)
                })
                .map(|(item, _)| item.clone())
                .collect();
            for item in &materials {
                self.stock.items.get_mut(item).unwrap().lifecycle = Lifecycle::Transformed;
                self.stock.end_allocation(item);
                self.stock.record(item, self.now);
            }
            for item in &contents {
                self.stock.items.get_mut(item).unwrap().lifecycle = Lifecycle::Available;
                self.stock.end_allocation(item);
                let transfer =
                    self.stock
                        .move_item(item, HolderRef::Container(carrier.clone()), self.now);
                self.emit(
                    object(ObjectKind::Item, item),
                    SimulationEventPayload::ItemTransferred(transfer),
                );
            }
            let material = &spec.outputs[0];
            self.stock.add_output(
                RuntimeItem {
                    id: carrier.clone(),
                    item_type_ref: material.item_type.clone(),
                    quantity: quantity(1, &material.unit),
                    attributes: self.stock.attributes_for(&material.item_type.id),
                    holder: HolderRef::Job(spec.id.clone()),
                    lifecycle: Lifecycle::Available,
                    quality: material
                        .output_quality
                        .clone()
                        .unwrap_or(QualityState::Unchecked),
                    allocation_refs: vec![],
                    composition_refs: materials,
                    contents,
                    completed_reworks: uint(0),
                    split_from_ref: None,
                },
                self.now,
            );
            outputs.push((carrier, 0));
        } else {
            for (item, _) in &inputs {
                self.stock.items.get_mut(item).unwrap().lifecycle = Lifecycle::Transformed;
                self.stock.end_allocation(item);
                self.stock.record(item, self.now);
            }
            for (ordinal, material) in spec.outputs.iter().enumerate() {
                let count = if material.individual {
                    material.amount
                } else {
                    1
                };
                for n in 0..count {
                    let output = stable(
                        "output",
                        &[spec.id.as_str(), material.port.as_str(), &n.to_string()],
                    );
                    self.stock.add_output(
                        RuntimeItem {
                            id: output.clone(),
                            item_type_ref: material.item_type.clone(),
                            quantity: quantity(
                                if material.individual {
                                    1
                                } else {
                                    material.amount
                                },
                                &material.unit,
                            ),
                            attributes: self.stock.attributes_for(&material.item_type.id),
                            holder: HolderRef::Job(spec.id.clone()),
                            lifecycle: Lifecycle::Available,
                            quality: material
                                .output_quality
                                .clone()
                                .unwrap_or(QualityState::Unchecked),
                            allocation_refs: vec![],
                            composition_refs: inputs.iter().map(|(id, _)| id.clone()).collect(),
                            contents: vec![],
                            completed_reworks: uint(0),
                            split_from_ref: None,
                        },
                        self.now,
                    );
                    outputs.push((output, ordinal));
                }
            }
        }
        for (output, ordinal) in &outputs {
            self.record_port_output(index, output, *ordinal, false);
            self.output_task.insert(output.clone(), spec.task.clone());
            if let Some(key) = &self.jobs[index].article_key {
                self.article_items.insert(output.clone(), key.clone());
            }
        }
        self.jobs[index].state.pending_outputs = outputs.iter().map(|(id, _)| id.clone()).collect();
        self.jobs[index].outputs = outputs;
        self.jobs[index].produced = true;
        self.jobs[index].output_at = Some(self.now);
        self.jobs[index].state.phase_ref = None;
        self.jobs[index].state.state = JobStatus::WaitingOutput;
        if self.jobs[index].outputs.is_empty() {
            self.finish(index);
        }
    }
    fn finish(&mut self, index: usize) {
        self.flow_destinations.remove(&index);
        self.close_wait(index, WaitEndCause::Resolved);
        self.release_claims(index, true);
        self.jobs[index].state.state = JobStatus::Completed;
        self.completed_job_ids.insert(self.jobs[index].spec.id.as_str().to_owned());
        self.jobs[index].state.finished_at_ms = Some(time(self.now));
        self.jobs[index].state.phase_ref = None;
        self.jobs[index].state.pending_outputs.clear();
        if self.jobs[index]
            .spec
            .operation
            .as_ref()
            .is_some_and(|op| matches!(op.builtin, RuleKind::Rework))
            && self.jobs[index].spec.setup.is_none()
        {
            let origin = self.jobs[index].inspection_origin.unwrap();
            let item = self.jobs[index].inputs[0].0.clone();
            let mut spec = self.jobs[origin].spec.clone();
            spec.id = stable(
                "inspection",
                &[
                    spec.id.as_str(),
                    item.as_str(),
                    &self.stock.items[&item]
                        .completed_reworks
                        .value()
                        .to_string(),
                ],
            );
            spec.predecessors.push(self.jobs[index].spec.id.clone());
            spec.release = self.now;
            let mut next = RunningJob::new(spec);
            next.exact_item = Some(item);
            next.inspection_origin = Some(origin);
            self.jobs.push(next);
        }
    }
    fn transfer_one(&mut self) -> bool {
        let mut pending = vec![];
        for (index, job) in self.jobs.iter().enumerate() {
            for (item, ordinal) in &job.outputs {
                if job.state.pending_outputs.contains(item) {
                    pending.push((
                        job.output_at.unwrap(),
                        job.spec.id.clone(),
                        job.spec.outputs[*ordinal].port.clone(),
                        item.clone(),
                        index,
                        Some(*ordinal),
                    ));
                }
            }
        }
        pending.extend(self.queued_quality_transfers());
        pending.sort();
        for (_, _, _, item, index, ordinal) in pending {
            let Some(ordinal) = ordinal else {
                if self.transfer_quality_item(&item) {
                    return true;
                }
                continue;
            };
            let material = self.jobs[index].spec.outputs[ordinal].clone();
            let store = self
                .input
                .project
                .locations
                .iter()
                .find(|l| l.id == material.location)
                .unwrap();
            let capacity = amount(store.capacity.as_ref().unwrap())
                .min(material.capacity.unwrap_or(MAX_SAFE_UINT));
            let occupied = self
                .stock
                .location_occupied(&material.location, &material.unit)
                + self.flow_reserved(&material.location, &material.unit, Some(index));
            let needed = amount(&self.stock.items[&item].quantity);
            if occupied
                .checked_add(needed as u128)
                .is_none_or(|n| n > capacity as u128)
            {
                self.set_wait(
                    index,
                    vec![WaitReason {
                        kind: WaitReasonKind::OutputCapacity,
                        blocking_object: Some(object(ObjectKind::Location, &material.location)),
                        required: format!("{needed} {} receiving capacity", material.unit.as_str()),
                        next_known_boundary_ms: None,
                    }],
                    true,
                );
                continue;
            }
            let transfer =
                self.stock
                    .move_item(&item, HolderRef::Location(material.location), self.now);
            self.stock.end_allocation(&item);
            self.stock.record(&item, self.now);
            self.record_port_output(index, &item, ordinal, true);
            self.emit(
                object(ObjectKind::Item, &item),
                SimulationEventPayload::ItemTransferred(transfer),
            );
            self.jobs[index]
                .state
                .pending_outputs
                .retain(|id| *id != item);
            self.flow_destinations.remove(&index);
            if self.jobs[index].state.pending_outputs.is_empty() {
                if self.has_flow_tail(index) {
                    self.close_wait(index, WaitEndCause::Resolved);
                    self.jobs[index].state.state = JobStatus::WaitingResource;
                    self.jobs[index].state.phase_ref = Some(
                        self.jobs[index].spec.phases[self.jobs[index].phase]
                            .id
                            .clone(),
                    );
                } else {
                    self.finish(index);
                }
            }
            return true;
        }
        false
    }
    fn location_of(&self, item: &RuntimeItem) -> Option<ObjectId> {
        let mut holder = &item.holder;
        let mut visited = BTreeSet::new();
        loop {
            match holder {
                HolderRef::Location(id) => return Some(id.clone()),
                HolderRef::Container(id) => {
                    if !visited.insert(id.clone()) {
                        return None;
                    }
                    holder = &self.stock.items.get(id)?.holder;
                }
                _ => return None,
            }
        }
    }
    fn accept_orders(&mut self) {
        if self.input.scenario.orders.is_empty() {
            self.accept_goal();
            return;
        }
        let mut orders: Vec<_> = self.input.scenario.orders.iter().cloned().collect();
        orders.sort_by(|a, b| b.priority.cmp(&a.priority).then(a.id.cmp(&b.id)));
        for order in orders {
            let Some(demand) = &order.quantity else {
                continue;
            };
            if order.release_at_ms.0.value() > self.now {
                continue;
            }
            let accepted = self.accepted.get(&order.id).copied().unwrap_or(0);
            let remaining = amount(demand) - accepted;
            if remaining == 0 {
                continue;
            }
            let product = self
                .input
                .project
                .products
                .iter()
                .find(|p| p.id == order.product_ref.id && p.revision == order.product_ref.revision)
                .unwrap();
            let mut candidates = vec![];
            let mut planned = BTreeMap::<ObjectId, u64>::new();
            for item in self.stock.items.values() {
                if !same_type(&item.item_type_ref, &product.item_type_ref)
                    || item.quantity.unit != demand.unit
                    || !matches!(item.lifecycle, Lifecycle::Available)
                    || !matches!(item.quality, QualityState::Released)
                    || self.location_of(item) != Some(order.destination.clone())
                    || !self.stage_reached(item, &order.destination, &order.acceptance_stage)
                {
                    continue;
                }
                let Some(task_id) = self.output_task.get(&item.id) else {
                    continue;
                };
                let task = self
                    .input
                    .scenario
                    .production_tasks
                    .iter()
                    .find(|t| t.id == *task_id)
                    .unwrap();
                if !task
                    .order_allocations
                    .iter()
                    .any(|a| a.order_id == order.id)
                {
                    continue;
                }
                let quota: u64 = task
                    .order_allocations
                    .iter()
                    .filter(|a| a.order_id == order.id)
                    .map(|a| amount(&a.quantity))
                    .sum();
                let quota = quota
                    - self
                        .task_accepted
                        .get(&(task_id.clone(), order.id.clone()))
                        .copied()
                        .unwrap_or(0)
                    - planned.get(task_id).copied().unwrap_or(0);
                let available = (amount(&item.quantity)
                    - self.fulfilled.get(&item.id).copied().unwrap_or(0))
                .min(quota);
                if available > 0 {
                    *planned.entry(task_id.clone()).or_default() += available;
                    candidates.push((item.id.clone(), available, task_id.clone()));
                }
            }
            let available: u64 = candidates.iter().map(|(_, n, _)| *n).sum();
            if available < remaining && !order.allow_partial {
                continue;
            }
            let mut need = remaining.min(available);
            if need == 0 {
                continue;
            }
            let counted = need;
            let mut refs = vec![];
            for (item, n, task) in candidates {
                let n = n.min(need);
                *self
                    .task_accepted
                    .entry((task, order.id.clone()))
                    .or_default() += n;
                *self.fulfilled.entry(item.clone()).or_default() += n;
                refs.push(item);
                need -= n;
                if need == 0 {
                    break;
                }
            }
            let event = AcceptanceEvent {
                order_id: Some(order.id.clone()),
                item_refs: refs,
                quantity: quantity(counted, &demand.unit),
                destination: order.destination,
                acceptance_stage: order.acceptance_stage.clone(),
                at_ms: time(self.now),
            };
            *self.accepted.entry(order.id.clone()).or_default() += counted;
            self.acceptance.push(event.clone());
            self.emit(
                object(ObjectKind::Order, &order.id),
                SimulationEventPayload::OrderAccepted(event),
            );
        }
    }
    fn accept_goal(&mut self) {
        let Some(goal) = &self.goal else {
            return;
        };
        let scopes: Vec<_> = std::iter::once(goal.acceptance_scope.clone())
            .chain(goal.additional_acceptance_scopes.iter().cloned()).collect();
        for scope in scopes {
            let product = self
                .input
                .project
                .products
                .iter()
                .find(|p| p.id == scope.product_ref.id && p.revision == scope.product_ref.revision)
                .unwrap();
            let candidates: Vec<_> = self
                .stock
                .items
                .values()
                .filter(|item| {
                    same_type(&item.item_type_ref, &product.item_type_ref)
                        && item.quantity.unit == scope.unit
                        && matches!(item.lifecycle, Lifecycle::Available)
                        && matches!(item.quality, QualityState::Released)
                        && self.output_task.contains_key(&item.id)
                        && self.location_of(item) == Some(scope.destination.clone())
                        && self.stage_reached(item, &scope.destination, &scope.stage)
                })
                .filter_map(|item| {
                    let remaining =
                        amount(&item.quantity) - self.fulfilled.get(&item.id).copied().unwrap_or(0);
                    (remaining > 0).then_some((item.id.clone(), remaining))
                })
                .collect();
            for (item, n) in candidates {
                *self.fulfilled.entry(item.clone()).or_default() += n;
                let event = AcceptanceEvent {
                    order_id: None,
                    item_refs: vec![item],
                    quantity: quantity(n, &scope.unit),
                    destination: scope.destination.clone(),
                    acceptance_stage: scope.stage.clone(),
                    at_ms: time(self.now),
                };
                self.acceptance.push(event.clone());
                self.emit(
                    object(ObjectKind::Project, &id(self.input.project.id.as_str())),
                    SimulationEventPayload::OrderAccepted(event),
                );
            }
        }
    }
    pub fn objects_at(&self, at: SimTimeMs) -> Option<Vec<RuntimeItem>> {
        let at = at.0.value();
        self.computed
            .filter(|until| at <= *until && at >= self.input.scenario.window.start_ms.0.value())
            .map(|_| self.stock.objects_at(at))
    }
    /// Immutable material changes in actual commit order. Host persistence may
    /// retain this Run-owned evidence for history queries after reopening.
    pub fn item_history(&self) -> Vec<(SimTimeMs, RuntimeItem)> {
        self.computed
            .map(|until| self.stock.item_history(until))
            .unwrap_or_default()
    }
    /// Actual Job changes after each complete same-instant closure. The caller
    /// can persist these existing DTOs alongside this Run's work intervals.
    pub fn job_history(&self) -> Vec<(SimTimeMs, JobState)> {
        self.job_history.clone()
    }
    pub fn port_output_history(&self) -> Vec<crate::PortOutputRecord> {
        self.port_output_history.clone()
    }
    fn record_port_output(
        &mut self,
        index: usize,
        item: &ObjectId,
        ordinal: usize,
        transferred: bool,
    ) {
        let job = &self.jobs[index];
        let material = &job.spec.outputs[ordinal];
        // A pending-store UNKNOWN reuses the input material as a private
        // disposition path. It is not a declared device output port. Later
        // location-to-location dispositions are not another Job completion.
        if !job.spec.operation.as_ref().is_some_and(|op| {
            ordinal < op.outputs.len() && op.outputs[ordinal].port == material.port
        }) {
            return;
        }
        let actual = &self.stock.items[item];
        if self.port_output_history.iter().any(|row| {
            row.1 == job.spec.id
                && row.3 == material.port
                && row.6 == transferred
                && row.7 == actual.id
        }) {
            return;
        }
        self.port_output_history.push((
            time(self.now),
            job.spec.id.clone(),
            job.spec.execution.clone(),
            material.port.clone(),
            actual.item_type_ref.clone(),
            actual.quantity.clone(),
            transferred,
            actual.id.clone(),
        ));
    }
    pub fn jobs_at(&self, at: SimTimeMs) -> Option<Vec<JobState>> {
        crate::jobs_at_from_history(
            &self.job_history,
            &self.work_intervals(),
            self.input.scenario.window.start_ms.clone(),
            self.computed.map(time),
            at,
        )
    }
    fn record_job_history(&mut self) {
        for state in self.job_states() {
            if self.job_history_latest.get(&state.id).is_some_and(|index| {
                crate::job_history::same_observable_state(&self.job_history[*index].1, &state)
            }) {
                continue;
            }
            self.job_history_latest
                .insert(state.id.clone(), self.job_history.len());
            self.job_history.push((time(self.now), state));
        }
    }
    fn job_states(&self) -> Vec<JobState> {
        self.jobs
            .iter()
            .enumerate()
            .filter(|(_, job)| !job.dormant && (self.external.is_none() || job.state.started_at_ms.is_some()))
            .map(|(index, job)| {
                let mut state = job.state.clone();
                state.job_resources = job
                    .spec
                    .job_resources
                    .keys()
                    .filter(|r| self.held(index, r) > 0)
                    .cloned()
                    .collect();
                state.phase_resources = if job.phase < job.spec.phases.len() {
                    job.spec.phases[job.phase]
                        .resources
                        .keys()
                        .filter(|r| self.held(index, r) > 0)
                        .cloned()
                        .collect()
                } else {
                    vec![]
                };
                state.effective_progress_ms = duration(job.progress);
                if let Some(active) = &job.active {
                    state.effective_progress_ms =
                        duration(active.progress + self.now.min(active.stop) - active.start);
                }
                state
            })
            .collect()
    }
    fn work_intervals(&self) -> Vec<PhaseWorkInterval> {
        let mut work = self.work.clone();
        for job in &self.jobs {
            if job.dormant {
                continue;
            }
            if let Some(active) = &job.active {
                let until = self.now.min(active.stop);
                if active.start < until {
                    work.push(PhaseWorkInterval {
                        job_id: job.spec.id.clone(),
                        phase_id: job.spec.phases[job.phase].id.clone(),
                        interval: IntervalMs {
                            start_ms: time(active.start),
                            end_ms: time(until),
                        },
                        effective_progress_start_ms: duration(active.progress),
                        effective_progress_end_ms: duration(active.progress + until - active.start),
                        end_cause: PhaseWorkEndCause::ComputedBoundary,
                    });
                }
            }
        }
        work
    }
    pub fn snapshot(&self) -> RunResult {
        let full = matches!(self.status, ComputationStatus::Completed)
            && self.computed == Some(self.end());
        let mut claims = self.claims.clone();
        for open in self.open_claims.values() {
            if open.start < self.now {
                claims[open.index].intervals.push(IntervalMs {
                    start_ms: time(open.start),
                    end_ms: time(self.now),
                });
                claims[open.index].interval_end_causes.push(if full {
                    ClaimIntervalEndCause::ObservationEnd
                } else {
                    ClaimIntervalEndCause::ComputedBoundary
                });
            }
        }
        claims.retain(|c| !c.intervals.is_empty());
        let mut waits = self.waits.clone();
        for (index, start) in &self.pending_arrivals {
            if *start < self.now {
                waits.push(self.arrival_wait(
                    *index,
                    *start,
                    if full {
                        WaitEndCause::ObservationEnd
                    } else {
                        WaitEndCause::ComputedBoundary
                    },
                ));
            }
        }
        for (index, open) in &self.open_waits {
            if open.start < self.now {
                waits.push(WaitInterval {
                    object_ref: object(ObjectKind::Job, &self.jobs[*index].spec.id),
                    execution_ref: Some(self.jobs[*index].spec.execution.clone()),
                    phase_ref: open.phase.clone(),
                    interval: IntervalMs {
                        start_ms: time(open.start),
                        end_ms: time(self.now),
                    },
                    reasons: open.reasons.clone(),
                    end_cause: if full {
                        WaitEndCause::ObservationEnd
                    } else {
                        WaitEndCause::ComputedBoundary
                    },
                });
            }
        }
        waits.extend(self.quality_transfer_waits(if full {
            WaitEndCause::ObservationEnd
        } else {
            WaitEndCause::ComputedBoundary
        }));
        let work = self.work_intervals();
        let jobs = self.job_states();
        let mut orders: Vec<_> = self
            .input
            .scenario
            .orders
            .iter()
            .filter(|_| self.computed.is_some())
            .filter_map(|order| {
                let demand = order.quantity.as_ref()?;
                let accepted = self.accepted.get(&order.id).copied().unwrap_or(0);
                let complete = accepted == amount(demand);
                let completed = if complete {
                    self.acceptance
                        .iter()
                        .rev()
                        .find(|e| e.order_id.as_ref() == Some(&order.id))
                        .map(|e| e.at_ms.0.value())
                } else {
                    None
                };
                Some(OrderOutcome {
                    order_id: order.id.clone(),
                    demand: demand.clone(),
                    accepted: quantity(accepted, &demand.unit),
                    outstanding: quantity(amount(demand) - accepted, &demand.unit),
                    completion_ms: completed.map(time),
                    due_at_ms: order.due_at_ms.clone(),
                    tardiness_ms: completed
                        .zip(order.due_at_ms.as_ref().map(|t| t.0.value()))
                        .map(|(c, d)| duration(c.saturating_sub(d))),
                })
            })
            .collect();
        orders.sort_by(|a, b| a.order_id.cmp(&b.order_id));
        let goal = if full
            && !orders.is_empty()
            && !self.limitations.iter().any(|i| {
                i.code == "simulation.partial_scope" || i.code == "simulation.transport_scope"
            }) {
            Some(orders.iter().all(|o| amount(&o.outstanding) == 0))
        } else if full
            && self.input.scenario.orders.is_empty()
            && !self.limitations.iter().any(|i| {
                i.blocks
                    .iter()
                    .any(|p| matches!(p, CapabilityPurpose::CompleteConclusion))
            })
        {
            self.goal
                .as_ref()
                .and_then(|goal| goal.accepted_quantity_per_day.as_ref())
                .map(|target| {
                    let accepted: u128 = self
                        .acceptance
                        .iter()
                        .filter(|e| e.order_id.is_none())
                        .map(|e| amount(&e.quantity) as u128)
                        .sum();
                    accepted * 10u128.pow(target.value.decimal_places.value() as u32)
                        >= amount(target) as u128
                })
        } else {
            None
        };
        RunResult {
            evidence_origin: EvidenceOrigin::ActualSoftware,
            id: self.input.run_id.clone(),
            input_ref: self.input.input_ref.clone(),
            scope: self.input.scope.clone(),
            window: self.input.scenario.window.clone(),
            computed_until_ms: self.computed.map(time),
            computation_status: self.status.clone(),
            whole_window_computed: full,
            goal_met: goal,
            final_items: self.stock.items.values().cloned().collect(),
            jobs,
            phase_work_intervals: work,
            transport: self.trips.values().cloned().collect(),
            resource_claims: claims,
            waits,
            allocations: self.stock.allocations.clone(),
            events: self.events.clone(),
            acceptance_events: self.acceptance.clone(),
            orders,
            limitations: self.limitations.clone(),
        }
    }
}

/// Background Worker entry: cancellation is observed between complete atomic
/// instants, and the returned partial Run is the same Engine's actual prefix.
pub fn execute_engine(input: EngineInput, cancelled: &AtomicBool) -> Result<Engine, Vec<Issue>> {
    run_worker(Engine::new(input)?, cancelled)
}
pub fn execute_engine_with_goal(
    input: EngineInput,
    goal: FactoryGenerationGoal,
    cancelled: &AtomicBool,
) -> Result<Engine, Vec<Issue>> {
    run_worker(Engine::new_with_goal(input, goal)?, cancelled)
}
fn run_worker(mut engine: Engine, cancelled: &AtomicBool) -> Result<Engine, Vec<Issue>> {
    loop {
        if cancelled.load(Ordering::Acquire) {
            engine.cancel();
            break;
        }
        engine.step();
        if matches!(
            engine.status,
            ComputationStatus::Completed | ComputationStatus::Failed | ComputationStatus::Cancelled
        ) {
            break;
        }
    }
    Ok(engine)
}
pub fn execute(input: EngineInput, cancelled: &AtomicBool) -> Result<RunResult, Vec<Issue>> {
    execute_engine(input, cancelled).map(|engine| engine.snapshot())
}
