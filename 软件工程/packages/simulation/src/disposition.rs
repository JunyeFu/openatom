//! Explicit scheduled quality decisions and later physical handoffs. Completed
//! inspections stay completed; a pending-store release is its own item effect.
use super::*;

pub(super) struct Transfer {
    origin: usize,
    output: Material,
    waiting_since: Option<u64>,
    ready_at: u64,
}
impl Engine {
    fn unapplied_quality(&mut self, decision: &QualityDecision, message: &str) {
        self.limitations.push(issue(
            object(ObjectKind::Item, &decision.item_ref),
            "/declared_quality_decisions",
            "quality_decision_unapplied",
            message,
        ));
    }
    pub(super) fn apply_declared_quality(&mut self, decision: QualityDecision) {
        if !self.declared_source_matches(&decision) {
            self.unapplied_quality(
                &decision,
                "Declared source kind, identity and actual item/inspection scope do not match.",
            );
            return;
        }
        let Some(current) = self.stock.items.get(&decision.item_ref) else {
            self.unapplied_quality(&decision,"No actual physical item exists at the decision time; future identities are not cached for release.");
            return;
        };
        let current_holder = current.holder.clone();
        let origin = self
            .jobs
            .iter()
            .enumerate()
            .filter(|(_, j)| {
                j.spec.quality.is_some()
                    && j.produced
                    && j.state.input_refs.contains(&decision.item_ref)
                    && same_module(&j.spec.operation_ref, &decision.inspection_ref)
                    && match &current_holder {
                        HolderRef::Job(held) => *held == j.spec.id,
                        _ => true,
                    }
            })
            .max_by_key(|(i, j)| (j.output_at.unwrap_or(0), *i))
            .map(|(i, _)| i);
        let Some(index) = origin else {
            self.unapplied_quality(&decision,"No actual completed inspection matches this item and exact inspection module at this instant; the decision is not cached for future work.");
            return;
        };
        let item = &self.stock.items[&decision.item_ref];
        if !matches!(item.lifecycle, Lifecycle::Available)
            || !matches!(item.quality, QualityState::Pending | QualityState::Rejected)
        {
            self.unapplied_quality(&decision,"The exact item is not currently available in a pending/rejected state; active work and completed release cannot be overridden.");
            return;
        }
        if let (Some(required), DecisionSource::ScheduledPreset(actual)) = (
            &self.jobs[index]
                .spec
                .quality
                .as_ref()
                .unwrap()
                .rule
                .result_ref,
            &decision.decision_source,
        ) {
            if required != actual {
                self.unapplied_quality(&decision,"The scheduled preset is outside the actual inspection rule's declared result source.");
                return;
            }
        }
        match decision.disposition {
            QualityDisposition::Reinspect=>self.enqueue_reinspection(index,decision),
            QualityDisposition::Scrap=>{
                let item=decision.item_ref.clone();
                self.cancel_quality_transfer(&item);
                self.stock.items.get_mut(&item).unwrap().lifecycle=Lifecycle::Scrapped;
                self.stock.record(&item,self.now);self.pending_inspections.remove(&item);
                self.cancel_unstarted_rechecks(&item);
                // No implicit physical evacuation: holder, composition and
                // occupied slot/claim remain until an actual movement rule.
                self.jobs[index].outputs.retain(|(id,_)|id!=&item);
                self.emit(object(ObjectKind::Item,&item),SimulationEventPayload::QualityChanged(decision));
            },
            QualityDisposition::Release|QualityDisposition::Rework|QualityDisposition::Isolate=>{
                self.cancel_unstarted_rechecks(&decision.item_ref);
                self.route_quality(index,decision);
            },
            _=>self.unapplied_quality(&decision,"This disposition does not perform a supported release, rework, isolation or explicit scrap."),
        }
    }
    fn cancel_unstarted_rechecks(&mut self, item: &ObjectId) {
        let mut cancelled = vec![];
        for (index, job) in self.jobs.iter_mut().enumerate() {
            if job.exact_item.as_ref() == Some(item)
                && job.spec.quality.is_some()
                && job.state.started_at_ms.is_none()
            {
                job.state.state = JobStatus::Cancelled;
                cancelled.push(index);
            }
        }
        for index in cancelled {
            self.close_wait(index, WaitEndCause::ConditionChanged);
        }
    }
    pub(super) fn enqueue_reinspection(&mut self, index: usize, decision: QualityDecision) {
        let item = decision.item_ref.clone();
        let holder = self.stock.items[&item].holder.clone();
        let held = matches!(&holder,HolderRef::Job(id) if id==&self.jobs[index].spec.id);
        if !held && !matches!(holder, HolderRef::Location(_)) {
            self.unapplied_quality(&decision,"Reinspection requires the actual held inspection item or a real finite source location; containers are not teleported.");
            return;
        }
        self.cancel_quality_transfer(&item);
        self.cancel_unstarted_rechecks(&item);
        if held {
            self.jobs[index].outputs.clear();
        }
        let mut spec = self.jobs[index].spec.clone();
        spec.id = stable("reinspection", &[decision.id.as_str(), item.as_str()]);
        spec.release = self.now;
        spec.predecessors = if held {
            vec![]
        } else {
            vec![self.jobs[index].spec.id.clone()]
        };
        spec.inputs[0].qualities = vec![QualityState::Pending, QualityState::Rejected];
        if let HolderRef::Location(location) = holder {
            spec.inputs[0].location = location;
        }
        let mut next = RunningJob::new(spec);
        next.exact_item = Some(item.clone());
        next.inspection_origin = Some(self.jobs[index].inspection_origin.unwrap_or(index));
        next.quality_handoff_from = held.then_some(index);
        self.jobs.push(next);
        self.emit(
            object(ObjectKind::Item, &item),
            SimulationEventPayload::QualityChanged(decision),
        );
    }
    pub(super) fn recheck_material(
        &self,
        index: usize,
    ) -> Option<Result<Vec<crate::stock::Pick>, Vec<WaitReason>>> {
        let origin = self.jobs[index].quality_handoff_from?;
        let id = self.jobs[index].exact_item.as_ref().unwrap();
        let item = &self.stock.items[id];
        if matches!(&item.holder,HolderRef::Job(job) if job==&self.jobs[origin].spec.id)
            && matches!(item.lifecycle, Lifecycle::Available)
            && matches!(item.quality, QualityState::Pending | QualityState::Rejected)
        {
            Some(Ok(vec![crate::stock::Pick {
                item: id.clone(),
                amount: amount(&item.quantity),
                material: 0,
            }]))
        } else {
            Some(Err(vec![WaitReason {
                kind: WaitReasonKind::Quality,
                blocking_object: Some(object(ObjectKind::Item, id)),
                required:
                    "The same still-pending/rejected physical item held by its completed check"
                        .into(),
                next_known_boundary_ms: None,
            }]))
        }
    }
    pub(super) fn finish_recheck_handoff(&mut self, index: usize, old_allocations: &[ObjectId]) {
        let Some(origin) = self.jobs[index].quality_handoff_from else {
            return;
        };
        self.stock.allocations.retain(|a| {
            !matches!(a.mode, AllocationMode::Committed) || !old_allocations.contains(&a.id)
        });
        self.jobs[origin].state.pending_outputs.clear();
        self.jobs[origin].outputs.clear();
        self.finish(origin);
    }
    fn cancel_quality_transfer(&mut self, item: &ObjectId) {
        if let Some(transfer) = self.quality_transfers.remove(item) {
            if transfer.waiting_since.is_some_and(|start| start < self.now) {
                let wait =
                    self.quality_wait(item, &transfer, self.now, WaitEndCause::ConditionChanged);
                self.waits.push(wait.clone());
                self.emit(
                    object(ObjectKind::Item, item),
                    SimulationEventPayload::WaitChanged(wait),
                );
            }
        }
    }
    pub(super) fn queue_quality_transfer(&mut self, index: usize, item: ObjectId, ordinal: usize) {
        let output = self.jobs[index].spec.outputs[ordinal].clone();
        if let Some(prior) = self.quality_transfers.get(&item) {
            if prior.output.location == output.location
                && prior.output.capacity == output.capacity
                && prior.output.unit == output.unit
                && same_type(&prior.output.item_type, &output.item_type)
            {
                return;
            }
            if prior.waiting_since.is_some_and(|start| start < self.now) {
                let wait =
                    self.quality_wait(&item, prior, self.now, WaitEndCause::ConditionChanged);
                self.waits.push(wait.clone());
                self.emit(
                    object(ObjectKind::Item, &item),
                    SimulationEventPayload::WaitChanged(wait),
                );
            }
        }
        self.quality_transfers.insert(
            item,
            Transfer {
                origin: index,
                output,
                waiting_since: None,
                ready_at: self.now,
            },
        );
    }
    fn quality_wait(
        &self,
        item: &ObjectId,
        transfer: &Transfer,
        end: u64,
        cause: WaitEndCause,
    ) -> WaitInterval {
        WaitInterval {
            object_ref: object(ObjectKind::Item, item),
            // A postcheck item handoff does not occupy the old inspector.
            execution_ref: None,
            phase_ref: None,
            interval: IntervalMs {
                start_ms: time(transfer.waiting_since.unwrap()),
                end_ms: time(end),
            },
            reasons: vec![WaitReason {
                kind: WaitReasonKind::OutputCapacity,
                blocking_object: Some(object(ObjectKind::Location, &transfer.output.location)),
                required: format!(
                    "{} {} quality-disposition receiving capacity",
                    amount(&self.stock.items[item].quantity),
                    transfer.output.unit.as_str()
                ),
                next_known_boundary_ms: None,
            }],
            end_cause: cause,
        }
    }
    pub(super) fn quality_transfer_waits(&self, cause: WaitEndCause) -> Vec<WaitInterval> {
        self.quality_transfers
            .iter()
            .filter(|(_, t)| t.waiting_since.is_some_and(|start| start < self.now))
            .map(|(item, t)| self.quality_wait(item, t, self.now, cause.clone()))
            .collect()
    }
    pub(super) fn queued_quality_transfers(
        &self,
    ) -> Vec<(u64, ObjectId, PortId, ObjectId, usize, Option<usize>)> {
        self.quality_transfers
            .iter()
            .map(|(item, t)| {
                (
                    t.ready_at,
                    self.jobs[t.origin].spec.id.clone(),
                    t.output.port.clone(),
                    item.clone(),
                    t.origin,
                    None,
                )
            })
            .collect()
    }
    pub(super) fn transfer_quality_item(&mut self, item: &ObjectId) -> bool {
        let transfer = &self.quality_transfers[item];
        let material = &transfer.output;
        let same_holder = matches!(&self.stock.items[item].holder,HolderRef::Location(l) if l==&material.location);
        let store = self
            .input
            .project
            .locations
            .iter()
            .find(|l| l.id == material.location)
            .unwrap();
        let capacity = amount(store.capacity.as_ref().unwrap())
            .min(material.capacity.unwrap_or(MAX_SAFE_UINT)) as u128;
        if !same_holder
            && self
                .stock
                .location_occupied(&material.location, &material.unit)
                + self.flow_reserved(&material.location, &material.unit, None)
                + amount(&self.stock.items[item].quantity) as u128
                > capacity
        {
            self.quality_transfers
                .get_mut(item)
                .unwrap()
                .waiting_since
                .get_or_insert(self.now);
            return false;
        }
        let transfer = self.quality_transfers.remove(item).unwrap();
        if transfer.waiting_since.is_some_and(|start| start < self.now) {
            let wait = self.quality_wait(item, &transfer, self.now, WaitEndCause::Resolved);
            self.waits.push(wait.clone());
            self.emit(
                object(ObjectKind::Item, item),
                SimulationEventPayload::WaitChanged(wait),
            );
        }
        if !same_holder {
            let event = self.stock.move_item(
                item,
                HolderRef::Location(transfer.output.location),
                self.now,
            );
            self.stock.end_allocation(item);
            self.stock.record(item, self.now);
            self.emit(
                object(ObjectKind::Item, item),
                SimulationEventPayload::ItemTransferred(event),
            );
        }
        true
    }
}
