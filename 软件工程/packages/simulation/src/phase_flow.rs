//! Physical phase effects on the existing Engine's jobs and inventory.
use super::*;

impl Engine {
    fn material_flow(&self, index: usize) -> Option<PhaseMaterialFlow> {
        self.jobs[index]
            .spec
            .operation
            .as_ref()?
            .material_flow
            .clone()
    }
    pub(super) fn has_flow_tail(&self, index: usize) -> bool {
        self.material_flow(index).is_some_and(|flow| {
            flow.deliver_after_phase.is_some()
                && self.jobs[index].phase < self.jobs[index].spec.phases.len()
        })
    }
    pub(super) fn flow_reserved(
        &self,
        location: &ObjectId,
        unit: &UnitId,
        except: Option<usize>,
    ) -> u128 {
        self.flow_destinations
            .iter()
            .filter(|i| Some(**i) != except)
            .map(|i| &self.jobs[*i].spec.outputs[0])
            .filter(|m| m.location == *location && m.unit == *unit)
            .map(|m| m.amount as u128)
            .sum()
    }
    pub(super) fn reserve_flow_destination(&mut self, index: usize) -> bool {
        let Some(flow) = self.material_flow(index) else {
            return true;
        };
        let phase = &self.jobs[index].spec.phases[self.jobs[index].phase].id;
        if flow.reserve_before_phase.as_ref() != Some(phase)
            || self.flow_destinations.contains(&index)
        {
            return true;
        }
        let output = &self.jobs[index].spec.outputs[0];
        let store = self
            .input
            .project
            .locations
            .iter()
            .find(|l| l.id == output.location)
            .unwrap();
        let capacity = amount(store.capacity.as_ref().unwrap())
            .min(output.capacity.unwrap_or(MAX_SAFE_UINT)) as u128;
        if self.stock.location_occupied(&output.location, &output.unit)
            + self.flow_reserved(&output.location, &output.unit, None)
            + output.amount as u128
            > capacity
        {
            let reason = WaitReason {
                kind: WaitReasonKind::OutputCapacity,
                blocking_object: Some(object(ObjectKind::Location, &output.location)),
                required: format!(
                    "{} {} reserved before physical release",
                    output.amount,
                    output.unit.as_str()
                ),
                next_known_boundary_ms: None,
            };
            self.jobs[index].state.state = JobStatus::WaitingOutput;
            self.set_wait(index, vec![reason], false);
            return false;
        }
        self.flow_destinations.insert(index);
        true
    }
    pub(super) fn apply_flow_boundary(&mut self, index: usize, phase: &PhaseId) {
        let Some(flow) = self.material_flow(index) else {
            return;
        };
        if flow.acquire_after_phase.as_ref() == Some(phase) {
            for (item, _) in self.jobs[index].inputs.clone() {
                let transfer = self.stock.move_item(
                    &item,
                    HolderRef::Job(self.jobs[index].spec.id.clone()),
                    self.now,
                );
                self.emit(
                    object(ObjectKind::Item, &item),
                    SimulationEventPayload::ItemTransferred(transfer),
                );
            }
        }
        let last = &self.jobs[index].spec.phases.last().unwrap().id;
        if flow.consume_after_phase.as_ref().unwrap_or(last) == phase {
            for (item, ordinal) in self.jobs[index].inputs.clone() {
                let role = &self.jobs[index].spec.inputs[ordinal].role;
                if flow.auxiliary_roles.contains(role) {
                    self.stock.items.get_mut(&item).unwrap().lifecycle = Lifecycle::Transformed;
                    self.stock.end_allocation(&item);
                    self.stock.record(&item, self.now);
                } else if role == &flow.primary_role && !flow.attributes_after_consume.is_empty() {
                    self.stock
                        .items
                        .get_mut(&item)
                        .unwrap()
                        .attributes
                        .extend(flow.attributes_after_consume.clone());
                    self.stock.record(&item, self.now);
                }
            }
        }
        if flow.deliver_after_phase.as_ref() == Some(phase) {
            self.produce(index);
        }
    }
}
