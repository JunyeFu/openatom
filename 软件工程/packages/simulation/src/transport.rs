//! Supply trips are Jobs on the production Engine's queue. Cargo is reserved
//! in Stock, moved to a real Transport holder, and received before return.
use super::*;
use crate::stock::Pick;

#[derive(Debug, Clone)]
pub(crate) struct Plan {
    pub(super) supply: SupplyRelation,
    pub(super) definition: TransportDefinition,
    pub(super) carrier: ObjectId,
    pub(super) batch: u64,
}

pub(crate) fn supports_acceptance(
    input: &EngineInput,
    product: &SubjectRevisionRef,
    destination: &ObjectId,
) -> bool {
    input
        .project
        .products
        .iter()
        .find(|p| {
            same_type(
                &SubjectRevisionRef {
                    id: p.id.clone(),
                    revision: p.revision.clone(),
                },
                product,
            )
        })
        .is_some_and(|p| {
            input.scenario.supplies.iter().any(|s| {
                s.destination == *destination
                    && same_type(&s.item_type_ref, &p.item_type_ref)
                    && module(input, &s.transport_ref)
                        .is_some_and(|d| matches!(d.payload, ModulePayload::Transport(_)))
            })
        })
}
fn requirements(
    input: &EngineInput,
    supply: &SupplyRelation,
    rows: &[ResourceRequirement],
    owner: &ObjectRef,
    issues: &mut Vec<Issue>,
) -> BTreeMap<ObjectId, u64> {
    let mut out = BTreeMap::new();
    for row in rows {
        let resource = supply
            .resources
            .get(&row.role)
            .and_then(|key| input.project.resources.iter().find(|r| r.id == *key));
        let Some(r) = resource else {
            issues.push(issue(
                owner.clone(),
                format!("/resources/{}", row.role.as_str()),
                "transport_resource_binding",
                "Transport resource role requires its actual instance.",
            ));
            continue;
        };
        if !discrete(&row.quantity, owner, "/resources/quantity", issues) {
            continue;
        }
        if r.capacity.unit != row.quantity.unit
            || amount(&r.capacity) < amount(&row.quantity)
            || row
                .qualifications
                .iter()
                .any(|q| !r.qualifications.contains(q))
            || matches!(r.kind, ResourceKind::ServiceDeclaration)
        {
            issues.push(issue(owner.clone(),"/resources","transport_resource_qualification","Actual transport resource needs declared unit, qualification, capacity and implemented resource kind."));
            continue;
        }
        out.entry(r.id.clone())
            .and_modify(|n: &mut u64| *n = (*n).max(amount(&row.quantity)))
            .or_insert(amount(&row.quantity));
    }
    out
}
pub(crate) fn compile(input: &EngineInput, issues: &mut Vec<Issue>) -> Vec<Job> {
    let mut jobs = vec![];
    if !input.scope.is_empty() && !input.scenario.supplies.is_empty() {
        let mut gap = issue(
            object(ObjectKind::Scenario, &id(input.scenario.id.as_str())),
            "/supplies",
            "transport_scope",
            "Selected device-only scope excludes supply trips and cannot establish transport acceptance.",
        );
        gap.blocks = vec![CapabilityPurpose::CompleteConclusion];
        issues.push(gap);
        return jobs;
    }
    for supply in &input.scenario.supplies {
        let owner = object(ObjectKind::Supply, &supply.id);
        let before = issues.len();
        let Some(ModuleDefinition {
            payload: ModulePayload::Transport(definition),
            ..
        }) = module(input, &supply.transport_ref)
        else {
            issues.push(issue(
                owner,
                "/transport_ref",
                "transport_definition",
                "Supply requires exactly its declared Transport definition and version.",
            ));
            continue;
        };
        let Some(batch) = &definition.batch_capacity else {
            issues.push(issue(
                owner,
                "/batch_capacity",
                "transport_batch",
                "Batch capacity is required; target is not inventory or carrier capacity.",
            ));
            continue;
        };
        discrete(batch, &owner, "/batch_capacity", issues);
        discrete(&supply.target_quantity, &owner, "/target_quantity", issues);
        let fractional_source = input
            .scenario
            .initial_conditions
            .items
            .iter()
            .chain(input.scenario.arrivals.iter().flat_map(|arrival| &arrival.items))
            .any(|item| {
                same_type(&item.item_type_ref, &supply.item_type_ref)
                    && item.quantity.unit == supply.target_quantity.unit
                    && item.quantity.value.decimal_places.value() != 0
                    && matches!(&item.holder, HolderRef::Location(location) if location == &supply.source)
            });
        if fractional_source {
            issues.push(issue(owner.clone(), "/item_type_ref", "transport_fractional_stock_scope",
                "Discrete transport cannot count fixed-scale source coefficients as whole units; fractional auxiliary consumption does not provide transport scale conversion."));
        }
        let item = input.project.item_types.iter().find(|t| {
            same_type(
                &SubjectRevisionRef {
                    id: t.id.clone(),
                    revision: t.revision.clone(),
                },
                &supply.item_type_ref,
            )
        });
        if item.is_none_or(|i| i.unit != batch.unit || i.unit != supply.target_quantity.unit) {
            issues.push(issue(
                owner.clone(),
                "/item_type_ref",
                "transport_type_unit",
                "Supply type/revision, target and batch must share the actual declared item unit.",
            ));
        }
        for endpoint in [&supply.source, &supply.destination] {
            let location = input.project.locations.iter().find(|l| l.id == *endpoint);
            if location.is_none_or(|l| {
                l.capacity
                    .as_ref()
                    .is_none_or(|c| c.unit != batch.unit || c.value.decimal_places.value() != 0)
                    || (!l.accepted_item_types.is_empty()
                        && !l.accepted_item_types.contains(&supply.item_type_ref.id))
            }) {
                issues.push(issue(owner.clone(),"/source_destination","transport_endpoint","Supply endpoints require actual locations, compatible exact item types and finite capacity in the same unit."));
            }
        }
        if supply.source == supply.destination
            || supply.window.start_ms.0.value() >= supply.window.end_ms.0.value()
            || supply
                .departure_windows
                .iter()
                .any(|w| w.start_ms.0.value() >= w.end_ms.0.value())
        {
            issues.push(issue(
                owner.clone(),
                "/departure_windows",
                "transport_window",
                "Distinct endpoints and explicit nonempty half-open windows are required.",
            ));
        }
        let full = requirements(input, supply, &definition.resources, &owner, issues);
        let loading = requirements(input, supply, &definition.loading_resources, &owner, issues);
        let unloading = requirements(
            input,
            supply,
            &definition.unloading_resources,
            &owner,
            issues,
        );
        let carriers: Vec<_> = full
            .keys()
            .filter(|key| {
                input.project.resources.iter().any(|r| {
                    r.id == **key
                        && matches!(
                            r.kind,
                            ResourceKind::Vehicle | ResourceKind::ReusableCarrier
                        )
                })
            })
            .cloned()
            .collect();
        let carrier = if carriers.len() == 1 {
            Some(carriers[0].clone())
        } else if full.len() == 1 {
            full.keys().next().cloned()
        } else {
            None
        };
        let Some(carrier) = carrier else {
            issues.push(issue(owner,"/resources","transport_carrier_ambiguous","Declare one identifiable full-trip carrier; role spelling cannot identify a carrier among ambiguous resources."));
            continue;
        };
        let mut phases = vec![];
        for (name, work, resources) in [
            ("loading", &definition.times.loading, loading),
            ("outbound", &definition.times.outbound, BTreeMap::new()),
            ("unloading", &definition.times.unloading, unloading),
            ("returning", &definition.times.returning, BTreeMap::new()),
        ] {
            if let Some(work) = work.as_ref() {
                phases.push(Phase {
                    id: PhaseId::try_from(name.to_owned()).unwrap(),
                    work: work.value.0.value(),
                    resources,
                    pause_resume: false,
                });
            } else {
                issues.push(issue(owner.clone(),format!("/times/{name}"),"transport_duration","Every transport phase requires explicit canonical milliseconds, including explicit zero."));
            }
        }
        for condition in &definition.acceptance_conditions {
            if matches!(condition.kind, DestinationConditionKind::DeclaredReadiness)
                && !matches!((&condition.target_role,&condition.required_value),(Some(role),Some(TypedValue::String(marker))) if supply.resources.contains_key(role) && !marker.is_empty())
            {
                issues.push(issue(owner.clone(),"/acceptance_conditions","transport_readiness","Declared readiness requires an explicitly bound target role and exact string marker."));
            }
        }
        if issues.len() != before {
            continue;
        }
        jobs.push(Job {
            id: stable("trip", &[supply.id.as_str(), "0"]),
            task: supply.id.clone(),
            operation_ref: supply.transport_ref.clone(),
            execution: JobExecutionRef::DefinitionCycle {
                module_ref: supply.transport_ref.clone(),
            },
            operation: None,
            family: None,
            setup: None,
            transport: Some(Plan {
                supply: supply.clone(),
                definition: definition.clone(),
                carrier,
                batch: amount(batch),
            }),
            inputs: vec![],
            outputs: vec![],
            job_resources: full,
            output_holds: BTreeMap::new(),
            phases,
            predecessors: vec![],
            priority: 0,
            due: None,
            release: supply.window.start_ms.0.value(),
            preparation: None,
            cycle: 0,
            quality: None,
        });
    }
    jobs
}

impl Engine {
    fn cargo_plan(&self, index: usize, except: Option<&DispatchCommandId>) -> Result<Vec<Pick>, Vec<WaitReason>> {
        let plan = self.jobs[index].spec.transport.as_ref().unwrap();
        let supply = &plan.supply;
        let remaining =
            amount(&supply.target_quantity) - self.shipped.get(&supply.id).copied().unwrap_or(0);
        let desired = plan.batch.min(remaining);
        let mut candidates: Vec<_> = self
            .stock
            .items
            .values()
            .filter(|i| {
                same_type(&i.item_type_ref, &supply.item_type_ref)
                    && i.quantity.unit == supply.target_quantity.unit
                    && matches!(i.lifecycle, Lifecycle::Available)
                    && matches!(&i.holder,HolderRef::Location(l) if l==&supply.source)
                    && matches!(i.quality, QualityState::Unchecked | QualityState::Released)
                    && !self.stock.allocations.iter().any(|a| {
                        matches!(a.mode, AllocationMode::Committed) && a.item_refs.contains(&i.id)
                    })
            })
            .collect();
        candidates.sort_by_key(|i| (self.stock.landed_at(&i.id), i.id.clone()));
        let mut picks = vec![];
        let mut needed = desired;
        for item in candidates {
            let n = needed.min(self.stock.external_available(&item.id, except));
            if n > 0 {
                picks.push(Pick {
                    item: item.id.clone(),
                    amount: n,
                    material: 0,
                });
                needed -= n;
            }
            if needed == 0 {
                break;
            }
        }
        if desired == 0
            || (!supply.allow_partial_load && (desired < plan.batch || needed > 0))
            || needed == desired
        {
            Err(vec![WaitReason {
                kind: WaitReasonKind::MissingMaterial,
                blocking_object: Some(object(ObjectKind::Location, &supply.source)),
                required: format!(
                    "Actual {} {} batch within remaining supply target",
                    plan.batch,
                    supply.target_quantity.unit.as_str()
                ),
                next_known_boundary_ms: None,
            }])
        } else {
            Ok(picks)
        }
    }
    pub(super) fn receive_reasons(&self, index: usize) -> Vec<WaitReason> {
        let plan = self.jobs[index].spec.transport.as_ref().unwrap();
        let supply = &plan.supply;
        let mut reasons = vec![];
        let store = self
            .input
            .project
            .locations
            .iter()
            .find(|l| l.id == supply.destination)
            .unwrap();
        let quantity: u128 = self.trips[&index]
            .cargo
            .iter()
            .map(|id| amount(&self.stock.items[id].quantity) as u128)
            .sum();
        if self
            .stock
            .location_occupied(&supply.destination, &supply.target_quantity.unit)
            + self.flow_reserved(&supply.destination, &supply.target_quantity.unit, None)
            + quantity
            > amount(store.capacity.as_ref().unwrap()) as u128
        {
            reasons.push(WaitReason {
                kind: WaitReasonKind::OutputCapacity,
                blocking_object: Some(object(ObjectKind::Location, &supply.destination)),
                required: format!(
                    "{quantity} {} receiving capacity",
                    supply.target_quantity.unit.as_str()
                ),
                next_known_boundary_ms: None,
            });
        }
        for condition in &plan.definition.acceptance_conditions {
            let required = match condition.kind {
                DestinationConditionKind::QualityReleased
                    if self.trips[&index].cargo.iter().any(|id| {
                        !matches!(self.stock.items[id].quality, QualityState::Released)
                    }) =>
                {
                    Some((
                        WaitReasonKind::Quality,
                        "Cargo requires actual released quality".to_owned(),
                    ))
                }
                DestinationConditionKind::DeclaredReadiness => {
                    let target = &supply.resources[condition.target_role.as_ref().unwrap()];
                    let TypedValue::String(marker) = condition.required_value.as_ref().unwrap()
                    else {
                        unreachable!()
                    };
                    (!self.markers.contains(&(target.clone(), marker.clone())))
                        .then_some((WaitReasonKind::Preparation, marker.clone()))
                }
                _ => None,
            };
            if let Some((kind, required)) = required {
                reasons.push(WaitReason {
                    kind,
                    blocking_object: Some(object(ObjectKind::Location, &supply.destination)),
                    required,
                    next_known_boundary_ms: None,
                });
            }
        }
        reasons
    }
    // Return true only for a real same-instant transition, never for a wait.
    pub(super) fn dispatch_transport(&mut self, index: usize) -> bool {
        let first = self.jobs[index].state.started_at_ms.is_none();
        let plan = self.jobs[index].spec.transport.as_ref().unwrap().clone();
        let mut reasons = vec![];
        if self.jobs[index].phase == 2 {
            let receive = self.receive_reasons(index);
            if !receive.is_empty() {
                self.jobs[index].state.state = JobStatus::WaitingResource;
                self.set_wait(index, receive, false);
                return false;
            }
        }
        if first
            && !(self.now >= plan.supply.window.start_ms.0.value()
                && self.now < plan.supply.window.end_ms.0.value()
                && plan
                    .supply
                    .departure_windows
                    .iter()
                    .any(|w| w.start_ms.0.value() <= self.now && self.now < w.end_ms.0.value()))
        {
            reasons.push(WaitReason {
                kind: WaitReasonKind::Calendar,
                blocking_object: Some(object(ObjectKind::Supply, &plan.supply.id)),
                required: "Actual loading start inside an explicit departure opportunity".into(),
                next_known_boundary_ms: None,
            });
        }
        let picks = if first {
            self.cargo_plan(index, None)
        } else {
            Ok(vec![])
        };
        if let Err(r) = &picks {
            reasons.extend(r.clone());
        }
        if self.jobs[index].phase == 2 {
            reasons.extend(self.receive_reasons(index));
        }
        if self.jobs[index].phase == 3 && self.trips[&index].received_ms.is_none() {
            return false;
        }
        let resources = self.resource_plan(index);
        if let Err(r) = &resources {
            reasons.extend(r.clone());
        }
        if !reasons.is_empty() {
            self.jobs[index].state.state = JobStatus::WaitingResource;
            self.set_wait(index, reasons, false);
            return false;
        }
        self.close_wait(index, WaitEndCause::Resolved);
        if first {
            let picks = picks.unwrap();
            let cargo = match self.stock.take(&self.jobs[index].spec, &picks, self.now) {
                Ok(ids) => ids,
                Err(issue) => {
                    self.limitations.push(issue);
                    self.status = ComputationStatus::Failed;
                    return false;
                }
            };
            self.start_transport_cargo(index, cargo, &picks);
        } else if self.jobs[index].phase == 2 {
            self.trips.get_mut(&index).unwrap().phase = TransportPhase::Unloading;
            self.emit_trip(index);
        }
        self.start_phase(index, resources.unwrap());
        true
    }
    pub(super) fn start_transport_cargo(&mut self, index: usize, cargo: Vec<(ObjectId, usize)>, picks: &[Pick]) {
        let plan = self.jobs[index].spec.transport.as_ref().unwrap().clone();
        let n: u64 = picks.iter().map(|p| p.amount).sum();
        self.inherit_splits(&picks.iter().map(|p| p.item.clone()).collect());
        self.jobs[index].inputs = cargo;
        self.jobs[index].state.input_refs = self.jobs[index]
            .inputs
            .iter()
            .map(|(id, _)| id.clone())
            .collect();
        self.acquire(index, self.jobs[index].spec.job_resources.clone(), false);
        for ((i, _, key), open) in &self.open_claims {
            if *i == index && key.is_empty() {
                self.claims[open.index].kind = ResourceClaimKind::Transport;
            }
        }
        self.jobs[index].state.started_at_ms = Some(time(self.now));
        self.emit(
            object(ObjectKind::Job, &self.jobs[index].spec.id),
            SimulationEventPayload::JobStarted(self.jobs[index].spec.id.clone()),
        );
        self.trips.insert(
            index,
            TransportState {
                id: self.jobs[index].spec.id.clone(),
                supply_ref: plan.supply.id.clone(),
                cargo: self.jobs[index].state.input_refs.clone(),
                carrier_id: plan.carrier,
                phase: TransportPhase::Reserved,
                departed_ms: None,
                arrived_ms: None,
                received_ms: None,
                returned_ms: None,
            },
        );
        self.emit_trip(index);
        self.trips.get_mut(&index).unwrap().phase = TransportPhase::Loading;
        self.emit_trip(index);
        *self.shipped.entry(plan.supply.id.clone()).or_default() += n;
        if self.shipped[&plan.supply.id] < amount(&plan.supply.target_quantity) {
            let mut next = self.jobs[index].spec.clone();
            next.cycle += 1;
            next.id = stable("trip", &[plan.supply.id.as_str(), &next.cycle.to_string()]);
            self.jobs.push(RunningJob::new(next));
        }
    }
    pub(super) fn external_transport_plan(&self, index: usize, except: Option<&DispatchCommandId>) -> Result<Vec<Pick>, Vec<WaitReason>> {
        let job = &self.jobs[index];
        let plan = job.spec.transport.as_ref().unwrap();
        if job.state.started_at_ms.is_none() {
            if self.now < plan.supply.window.start_ms.0.value() || self.now >= plan.supply.window.end_ms.0.value()
                || !plan.supply.departure_windows.iter().any(|w| w.start_ms.0.value() <= self.now && self.now < w.end_ms.0.value()) {
                return Err(vec![WaitReason { kind: WaitReasonKind::Calendar,
                    blocking_object: Some(object(ObjectKind::Supply, &plan.supply.id)),
                    required: "Actual loading requires an open departure opportunity".into(), next_known_boundary_ms: None }]);
            }
            self.cargo_plan(index, except)
        } else {
            let reasons = if job.phase == 2 || job.phase == 3 && self.trips[&index].received_ms.is_none() {
                self.receive_reasons(index)
            } else { vec![] };
            if reasons.is_empty() { Ok(vec![]) } else { Err(reasons) }
        }
    }
    pub(super) fn emit_trip(&mut self, index: usize) {
        let trip = self.trips[&index].clone();
        self.emit(
            object(ObjectKind::Transport, &trip.id),
            SimulationEventPayload::TransportChanged(trip),
        );
    }
    pub(super) fn transport_phase_completed(&mut self, index: usize, phase: usize) {
        match phase {
            0 => {
                let trip = self.trips.get_mut(&index).unwrap();
                trip.departed_ms = Some(time(self.now));
                trip.phase = TransportPhase::Outbound;
                let holder = HolderRef::Transport(trip.id.clone());
                for id in trip.cargo.clone() {
                    let event = self.stock.move_item(&id, holder.clone(), self.now);
                    self.emit(
                        object(ObjectKind::Item, &id),
                        SimulationEventPayload::ItemTransferred(event),
                    );
                }
            }
            1 => {
                let trip = self.trips.get_mut(&index).unwrap();
                trip.arrived_ms = Some(time(self.now));
                trip.phase = TransportPhase::WaitingReceive;
            }
            2 => {
                self.trips.get_mut(&index).unwrap().phase = TransportPhase::WaitingReceive;
            }
            3 => {
                let trip = self.trips.get_mut(&index).unwrap();
                trip.returned_ms = Some(time(self.now));
                trip.phase = TransportPhase::Returned;
                self.finish(index);
            }
            _ => unreachable!(),
        }
        self.emit_trip(index);
        if phase < 3 {
            self.jobs[index].state.state = JobStatus::WaitingResource;
            self.jobs[index].state.phase_ref = Some(
                self.jobs[index].spec.phases[self.jobs[index].phase]
                    .id
                    .clone(),
            );
        }
    }
    pub(super) fn receive_transport_one(&mut self) -> bool {
        for index in self.trips.keys().copied().collect::<Vec<_>>() {
            if self.jobs[index].phase != 3 || self.trips[&index].received_ms.is_some() {
                continue;
            }
            let reasons = self.receive_reasons(index);
            if !reasons.is_empty() {
                self.jobs[index].state.state = JobStatus::WaitingOutput;
                self.jobs[index].state.phase_ref = None;
                self.set_wait(index, reasons, true);
                continue;
            }
            self.close_wait(index, WaitEndCause::Resolved);
            let destination = self.jobs[index]
                .spec
                .transport
                .as_ref()
                .unwrap()
                .supply
                .destination
                .clone();
            for id in self.trips[&index].cargo.clone() {
                let event =
                    self.stock
                        .move_item(&id, HolderRef::Location(destination.clone()), self.now);
                self.stock.end_allocation(&id);
                self.stock.record(&id, self.now);
                self.receipts.insert(id.clone(), destination.clone());
                self.emit(
                    object(ObjectKind::Item, &id),
                    SimulationEventPayload::ItemTransferred(event),
                );
            }
            let trip = self.trips.get_mut(&index).unwrap();
            trip.received_ms = Some(time(self.now));
            trip.phase = TransportPhase::Returning;
            self.emit_trip(index);
            return true;
        }
        false
    }
    pub(super) fn stage_reached(
        &self,
        item: &RuntimeItem,
        destination: &ObjectId,
        stage: &AcceptanceStage,
    ) -> bool {
        if matches!(stage, AcceptanceStage::FactoryReleased) {
            return true;
        }
        let mut current = item;
        let mut seen = BTreeSet::new();
        loop {
            if self.receipts.get(&current.id) == Some(destination) {
                return true;
            }
            if !seen.insert(current.id.clone()) {
                return false;
            }
            match &current.holder {
                HolderRef::Container(id) => {
                    let Some(parent) = self.stock.items.get(id) else {
                        return false;
                    };
                    current = parent;
                }
                _ => return false,
            }
        }
    }
}
