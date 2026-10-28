use crate::EngineInput;
use crate::compile::*;
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub(crate) struct Pick {
    pub item: ObjectId,
    pub amount: u64,
    pub material: usize,
}

pub(crate) type CandidateKey = (ObjectId, ObjectId, SubjectRevisionId, UnitId, u8);
pub(crate) struct Candidate {
    id: ObjectId,
    landed: u64,
    allocation_indices: Vec<usize>,
}
pub(crate) type PlanCandidates = BTreeMap<CandidateKey, Vec<Candidate>>;

pub(crate) struct Stock {
    pub items: BTreeMap<ObjectId, RuntimeItem>,
    pub allocations: Vec<MaterialAllocation>,
    landed: BTreeMap<ObjectId, u64>,
    history: Vec<(u64, RuntimeItem)>,
    declared_ids: BTreeSet<ObjectId>,
    attribute_defaults: BTreeMap<ObjectId, BTreeMap<String, TypedValue>>,
    /// External dispatch reserves availability without changing physical items.
    pending: BTreeMap<DispatchCommandId, Vec<Pick>>,
}
pub(crate) struct InitialPlan {
    declared_ids: BTreeSet<ObjectId>,
    splits: BTreeMap<ObjectId, Vec<(ObjectId, u64)>>,
}
/// Pure initialization plan shared by enqueue admission and the actual Worker.
/// No RuntimeItem, history, holder or quantity is changed by this check.
pub(crate) fn plan_initial(input: &EngineInput) -> Result<InitialPlan, Vec<Issue>> {
    let declared_ids: BTreeSet<_> = input
        .scenario
        .initial_conditions
        .items
        .iter()
        .chain(input.scenario.arrivals.iter().flat_map(|a| a.items.iter()))
        .map(|i| i.id.clone())
        .collect();
    let amounts: BTreeMap<_, _> = input
        .scenario
        .initial_conditions
        .items
        .iter()
        .map(|i| (i.id.clone(), amount(&i.quantity)))
        .collect();
    let mut splits: BTreeMap<ObjectId, Vec<(ObjectId, u64)>> = BTreeMap::new();
    let mut allocations: Vec<_> = input
        .scenario
        .initial_conditions
        .material_allocations
        .iter()
        .filter(|a| matches!(a.mode, AllocationMode::HardReserved))
        .collect();
    allocations.sort_by(|a, b| a.id.cmp(&b.id));
    for allocation in allocations {
        for source in &allocation.item_refs {
            let n = if allocation.item_refs.len() == 1 {
                amount(&allocation.quantity)
            } else {
                amounts[source]
            };
            splits
                .entry(source.clone())
                .or_default()
                .push((allocation.id.clone(), n));
        }
    }
    let mut generated = BTreeSet::new();
    for (source, parts) in &splits {
        let total: u64 = parts.iter().map(|(_, n)| *n).sum();
        if parts.len() == 1 && total == amounts[source] {
            continue;
        }
        let mut keys: Vec<_> = parts
            .iter()
            .map(|(allocation, _)| {
                stable(
                    "initial-reservation",
                    &[source.as_str(), allocation.as_str()],
                )
            })
            .collect();
        if total < amounts[source] {
            keys.push(stable("initial-free", &[source.as_str()]));
        }
        for key in keys {
            if declared_ids.contains(&key) || !generated.insert(key.clone()) {
                return Err(vec![issue(
                    object(ObjectKind::Item, source),
                    "/id",
                    "generated_item_identity_collision",
                    format!(
                        "Generated initial identity {} conflicts with a declared initial/future or generated identity; initialization was not committed.",
                        key.as_str()
                    ),
                )]);
            }
        }
    }
    Ok(InitialPlan {
        declared_ids,
        splits,
    })
}
impl Stock {
    pub fn can_create(&self, keys: &[ObjectId], owner: &ObjectId) -> Result<(), Issue> {
        let mut proposed = BTreeSet::new();
        for key in keys {
            if self.declared_ids.contains(key)
                || self.items.contains_key(key)
                || !proposed.insert(key.clone())
            {
                return Err(issue(
                    object(ObjectKind::Item, owner),
                    "/id",
                    "generated_item_identity_collision",
                    format!(
                        "Generated item identity {} conflicts with a declared or existing physical identity; no material transition was committed.",
                        key.as_str()
                    ),
                ));
            }
        }
        Ok(())
    }
    pub fn new(input: &EngineInput, plan: InitialPlan) -> Self {
        let mut stock = Self {
            items: BTreeMap::new(),
            allocations: input
                .scenario
                .initial_conditions
                .material_allocations
                .clone(),
            landed: BTreeMap::new(),
            history: vec![],
            pending: BTreeMap::new(),
            declared_ids: plan.declared_ids,
            attribute_defaults: input.project.item_types.iter()
                .map(|t| (t.id.clone(), t.attributes.clone())).collect(),
        };
        stock.insert_group(&input.scenario.initial_conditions.items, 0);
        let by_source = plan.splits;
        for (source, parts) in by_source {
            let original = stock.items[&source].clone();
            let total: u64 = parts.iter().map(|(_, n)| *n).sum();
            if parts.len() == 1 && total == amount(&original.quantity) {
                stock
                    .items
                    .get_mut(&source)
                    .unwrap()
                    .allocation_refs
                    .push(parts[0].0.clone());
                continue;
            }
            stock.items.get_mut(&source).unwrap().lifecycle = Lifecycle::Transformed;
            for (allocation, n) in parts {
                let child_id = stable(
                    "initial-reservation",
                    &[source.as_str(), allocation.as_str()],
                );
                let mut child = original.clone();
                child.id = child_id.clone();
                child.quantity = quantity(n, &original.quantity.unit);
                child.split_from_ref = Some(source.clone());
                child.allocation_refs = vec![allocation.clone()];
                stock.items.insert(child_id.clone(), child);
                stock.landed.insert(child_id.clone(), 0);
                let a = stock
                    .allocations
                    .iter_mut()
                    .find(|a| a.id == allocation)
                    .unwrap();
                for reference in &mut a.item_refs {
                    if *reference == source {
                        *reference = child_id.clone();
                    }
                }
            }
            if total < amount(&original.quantity) {
                let child_id = stable("initial-free", &[source.as_str()]);
                let mut child = original.clone();
                child.id = child_id.clone();
                child.quantity =
                    quantity(amount(&original.quantity) - total, &original.quantity.unit);
                child.split_from_ref = Some(source);
                child.allocation_refs.clear();
                stock.items.insert(child_id.clone(), child);
                stock.landed.insert(child_id, 0);
            }
        }
        stock.history.clear();
        stock.record_all(0);
        stock
    }
    pub fn insert_group(&mut self, group: &[InitialItem], at: u64) {
        let composed: BTreeSet<_> = group
            .iter()
            .flat_map(|i| i.components.iter().cloned())
            .collect();
        for i in group {
            let runtime = RuntimeItem {
                id: i.id.clone(),
                item_type_ref: i.item_type_ref.clone(),
                quantity: i.quantity.clone(),
                attributes: self.attributes_for(&i.item_type_ref.id),
                holder: i.holder.clone(),
                lifecycle: if composed.contains(&i.id) {
                    Lifecycle::Transformed
                } else {
                    Lifecycle::Available
                },
                quality: i.quality.clone(),
                allocation_refs: vec![],
                composition_refs: i.components.clone(),
                contents: i.contents.clone(),
                completed_reworks: uint(0),
                split_from_ref: None,
            };
            self.landed.insert(i.id.clone(), at);
            self.history.push((at, runtime.clone()));
            self.items.insert(i.id.clone(), runtime);
        }
    }
    pub fn record(&mut self, key: &ObjectId, at: u64) {
        self.history.push((at, self.items[key].clone()));
    }
    pub fn attributes_for(&self, item_type: &ObjectId) -> BTreeMap<String, TypedValue> {
        self.attribute_defaults
            .get(item_type)
            .cloned()
            .unwrap_or_default()
    }
    fn record_all(&mut self, at: u64) {
        self.history
            .extend(self.items.values().cloned().map(|i| (at, i)));
    }
    pub fn objects_at(&self, at: u64) -> Vec<RuntimeItem> {
        let mut items = BTreeMap::new();
        for (when, item) in &self.history {
            if *when <= at {
                items.insert(item.id.clone(), item.clone());
            }
        }
        items.into_values().collect()
    }
    pub fn item_history(&self, until: u64) -> Vec<(SimTimeMs, RuntimeItem)> {
        self.history
            .iter()
            .filter(|(at, _)| *at <= until)
            .map(|(at, item)| (time(*at), item.clone()))
            .collect()
    }
    fn reserved_for(&self, item: &Candidate, task: &ObjectId) -> (bool, bool) {
        let mut own = false;
        let mut other = false;
        for &index in &item.allocation_indices {
            let a = &self.allocations[index];
            if matches!(a.mode, AllocationMode::Committed) {
                other = true;
            }
            if matches!(a.mode, AllocationMode::HardReserved) {
                if a.demand_ref == *task {
                    own = true;
                } else {
                    other = true;
                }
            }
        }
        (own, other)
    }
    pub fn plan_candidates(&self) -> PlanCandidates {
        let mut allocations = BTreeMap::<_, Vec<usize>>::new();
        for (index, allocation) in self.allocations.iter().enumerate() {
            for id in &allocation.item_refs {
                allocations.entry(id).or_default().push(index);
            }
        }
        let mut grouped = BTreeMap::<_, Vec<Candidate>>::new();
        for item in self.items.values() {
            let HolderRef::Location(location) = &item.holder else { continue };
            if !matches!(item.lifecycle, Lifecycle::Available) { continue; }
            grouped.entry((location, &item.item_type_ref.id, &item.item_type_ref.revision,
                &item.quantity.unit, item.quantity.value.decimal_places.value()))
                .or_default().push(Candidate {
                    id: item.id.clone(),
                    landed: self.landed[&item.id],
                    allocation_indices: allocations.remove(&item.id).unwrap_or_default(),
                });
        }
        grouped.into_iter().map(|((location, item_type, revision, unit, decimal), ids)| {
            ((location.clone(), item_type.clone(), revision.clone(), unit.clone(), decimal), ids)
        }).collect()
    }
    pub fn plan(
        &self,
        job: &Job,
        project: &ProjectState,
        exact_item: Option<&ObjectId>,
        available: &PlanCandidates,
    ) -> Result<Vec<Pick>, Vec<WaitReason>> {
        let mut plan = vec![];
        let mut used = BTreeMap::<ObjectId, u64>::new();
        let mut reasons = vec![];
        for (ordinal, material) in job.inputs.iter().enumerate() {
            let store = project
                .locations
                .iter()
                .find(|l| l.id == material.location)
                .unwrap();
            let key = (material.location.clone(), material.item_type.id.clone(),
                material.item_type.revision.clone(), material.unit.clone(), material.decimal_places);
            let mut candidates: Vec<_> = available.get(&key).into_iter().flatten()
                .filter(|i| exact_item.is_none_or(|required| i.id == *required))
                .collect();
            candidates.sort_by_cached_key(|&i| {
                (
                    !self.reserved_for(i, &job.task).0,
                    i.landed,
                    &i.id,
                )
            });
            let mut needed = material.amount;
            let mut quality_blocked = false;
            for candidate in candidates {
                if self.reserved_for(candidate, &job.task).1 {
                    continue;
                }
                let item = &self.items[&candidate.id];
                if job.op().material_flow.as_ref().is_some_and(|flow|material.role==flow.primary_role
                    && flow.required_primary_attributes.iter().any(|(key,required)| {
                        !matches!((item.attributes.get(key),required),(Some(TypedValue::Enum(actual)),TypedValue::Enum(wanted)) if actual==wanted)
                    })) {continue;}
                let quality_operation = matches!(
                    job.op().builtin,
                    RuleKind::InspectAndRoute | RuleKind::Rework
                );
                if quality_operation
                    && (amount(&item.quantity) != material.amount || needed != material.amount)
                {
                    continue;
                }
                let allowed_quality = (quality_operation
                    || matches!(
                        item.quality,
                        QualityState::Released | QualityState::Unchecked
                    ))
                    && ((quality_operation && material.qualities.is_empty())
                        || material.qualities.iter().any(|q| {
                            std::mem::discriminant(q) == std::mem::discriminant(&item.quality)
                        }));
                if !allowed_quality
                    || (!quality_operation
                        && (!store.ordinary_issue_enabled
                            || matches!(
                                store.purpose,
                                LocationPurpose::Pending | LocationPurpose::Isolation
                            )))
                {
                    quality_blocked = true;
                    continue;
                }
                let reserved: u64 = self.pending.values().flatten()
                    .filter(|pick| pick.item == item.id).map(|pick| pick.amount).sum();
                let remaining = amount(&item.quantity) - reserved
                    - used.get(&item.id).copied().unwrap_or(0);
                let take = needed.min(remaining);
                if take == 0 {
                    continue;
                }
                plan.push(Pick {
                    item: item.id.clone(),
                    amount: take,
                    material: ordinal,
                });
                *used.entry(item.id.clone()).or_default() += take;
                needed -= take;
                if needed == 0 {
                    break;
                }
            }
            if needed > 0 {
                reasons.push(WaitReason {
                    kind: if quality_blocked {
                        WaitReasonKind::Quality
                    } else {
                        WaitReasonKind::MissingMaterial
                    },
                    blocking_object: Some(object(ObjectKind::Location, &material.location)),
                    required: format!(
                        "{} {} of {}@{} for port {}",
                        needed,
                        material.unit.as_str(),
                        material.item_type.id.as_str(),
                        material.item_type.revision.as_str(),
                        material.port.as_str()
                    ),
                    next_known_boundary_ms: None,
                });
            }
        }
        if reasons.is_empty() {
            Ok(plan)
        } else {
            Err(reasons)
        }
    }
    pub(crate) fn reserve_external(
        &mut self, command: DispatchCommandId, job: &Job, project: &ProjectState, exact_item: Option<&ObjectId>,
        handoff: Option<Result<Vec<Pick>, Vec<WaitReason>>>,
    ) -> Result<Vec<Pick>, Vec<WaitReason>> {
        let picks = handoff.unwrap_or_else(|| self.plan(job, project, exact_item, &self.plan_candidates()))?;
        if picks.iter().any(|pick| self.pending.values().flatten().any(|other| other.item == pick.item))
            && exact_item.is_some() {
            return Err(vec![WaitReason { kind: WaitReasonKind::Quality,
                blocking_object: exact_item.map(|id| object(ObjectKind::Item, id)),
                required: "Exact quality item is already reserved by another command".into(), next_known_boundary_ms: None }]);
        }
        self.pending.insert(command, picks.clone());
        Ok(picks)
    }

    pub(crate) fn release_external(&mut self, command: &DispatchCommandId) {
        self.pending.remove(command);
    }
    pub(crate) fn external_available(&self, item: &ObjectId, except: Option<&DispatchCommandId>) -> u64 {
        amount(&self.items[item].quantity).saturating_sub(self.pending.iter()
            .filter(|(command, _)| Some(*command) != except).flat_map(|(_, picks)| picks)
            .filter(|pick| &pick.item == item).map(|pick| pick.amount).sum())
    }
    pub(crate) fn external_picks(&self, command: &DispatchCommandId) -> &[Pick] {
        self.pending.get(command).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The original atomic take still owns splitting, allocation IDs and history.
    pub(crate) fn take_external(
        &mut self, command: &DispatchCommandId, job: &Job, project: &ProjectState, at: u64,
        handoff: Option<Result<Vec<Pick>, Vec<WaitReason>>>,
    ) -> Result<Vec<(ObjectId, usize)>, Issue> {
        let picks = self.pending.remove(command).expect("admitted external reservation");
        let ids: BTreeSet<_> = picks.iter().map(|p| p.item.clone()).collect();
        let mut candidates = self.plan_candidates();
        for values in candidates.values_mut() { values.retain(|c| ids.contains(&c.id)); }
        let valid = handoff.unwrap_or_else(|| self.plan(job, project, None, &candidates)).is_ok_and(|planned| {
            planned.len() == picks.len() && planned.iter().zip(&picks).all(|(a,b)|
                a.item == b.item && a.amount == b.amount && a.material == b.material)
        });
        if !valid {
            self.pending.insert(command.clone(), picks);
            return Err(issue(object(ObjectKind::Job, &job.id), "/external/reservation",
                "external_reservation_changed", "Reserved physical material no longer meets its frozen input contract; reservation retained for reconciliation."));
        }
        let taken = match self.take(job, &picks, at) {
            Ok(taken) => taken,
            Err(error) => { self.pending.insert(command.clone(), picks); return Err(error); }
        };
        // Taking a partial batch creates the original `remaining` child. Other
        // prepared commands follow that free child, not its transformed parent.
        for source in ids {
            if matches!(self.items[&source].lifecycle, Lifecycle::Transformed) {
                let remaining = stable("remaining", &[source.as_str(), job.id.as_str()]);
                for pick in self.pending.values_mut().flatten().filter(|p| p.item == source) {
                    assert!(self.items.contains_key(&remaining));
                    pick.item = remaining.clone();
                }
            }
        }
        Ok(taken)
    }

    /// Called only after the whole material/resource/calendar plan succeeds.
    pub fn take(
        &mut self,
        job: &Job,
        picks: &[Pick],
        at: u64,
    ) -> Result<Vec<(ObjectId, usize)>, Issue> {
        let moving = job.transport.is_some();
        let mut groups = BTreeMap::<ObjectId, Vec<&Pick>>::new();
        for p in picks {
            groups.entry(p.item.clone()).or_default().push(p);
        }
        let mut generated = vec![];
        let mut generated_allocations = BTreeSet::new();
        for (source, parts) in &groups {
            let original = &self.items[source];
            let total: u64 = parts.iter().map(|p| p.amount).sum();
            let split = total < amount(&original.quantity) || parts.len() > 1;
            if split && total < amount(&original.quantity) {
                generated.push(stable("remaining", &[source.as_str(), job.id.as_str()]));
            }
            for (ordinal, _) in parts.iter().enumerate() {
                let target = if split {
                    let child = stable(
                        "taken",
                        &[source.as_str(), job.id.as_str(), &ordinal.to_string()],
                    );
                    generated.push(child.clone());
                    child
                } else {
                    source.clone()
                };
                let allocation = stable("commit", &[job.id.as_str(), target.as_str()]);
                if self.allocations.iter().any(|a| a.id == allocation)
                    || !generated_allocations.insert(allocation)
                {
                    return Err(issue(
                        object(ObjectKind::Job, &job.id),
                        "/allocations",
                        "generated_allocation_identity_collision",
                        "Generated committed allocation identity conflicts before any material acquisition.",
                    ));
                }
            }
        }
        self.can_create(&generated, &job.id)?;
        let mut taken = vec![];
        for (source, parts) in groups {
            let original = self.items[&source].clone();
            let total: u64 = parts.iter().map(|p| p.amount).sum();
            let split = total < amount(&original.quantity) || parts.len() > 1;
            let mut remainder = None;
            if split {
                self.items.get_mut(&source).unwrap().lifecycle = Lifecycle::Transformed;
                self.items.get_mut(&source).unwrap().allocation_refs.clear();
                self.record(&source, at);
                if total < amount(&original.quantity) {
                    let child_id = stable("remaining", &[source.as_str(), job.id.as_str()]);
                    let mut child = original.clone();
                    child.id = child_id.clone();
                    child.quantity = crate::phase_material::fixed_quantity(
                        amount(&original.quantity) - total,
                        &original.quantity.unit,
                        original.quantity.value.decimal_places.value(),
                    );
                    child.split_from_ref = Some(source.clone());
                    self.items.insert(child_id.clone(), child);
                    self.landed.insert(child_id.clone(), self.landed[&source]);
                    self.record(&child_id, at);
                    remainder = Some(child_id);
                }
            }
            for (n, pick) in parts.into_iter().enumerate() {
                let target = if split {
                    stable("taken", &[source.as_str(), job.id.as_str(), &n.to_string()])
                } else {
                    source.clone()
                };
                let mut item = original.clone();
                item.id = target.clone();
                item.quantity = crate::phase_material::fixed_quantity(
                    pick.amount,
                    &original.quantity.unit,
                    original.quantity.value.decimal_places.value(),
                );
                if !moving {
                    if !job
                        .op()
                        .material_flow
                        .as_ref()
                        .is_some_and(|flow| flow.acquire_after_phase.is_some())
                    {
                        item.holder = HolderRef::Job(job.id.clone());
                    }
                    item.lifecycle = Lifecycle::InProcess;
                }
                if split {
                    item.split_from_ref = Some(source.clone());
                }
                let committed = stable("commit", &[job.id.as_str(), target.as_str()]);
                if !moving {
                    item.allocation_refs.clear();
                }
                item.allocation_refs.push(committed.clone());
                self.allocations.push(MaterialAllocation {
                    id: committed,
                    demand_ref: job.task.clone(),
                    item_refs: vec![target.clone()],
                    quantity: item.quantity.clone(),
                    mode: AllocationMode::Committed,
                });
                self.items.insert(target.clone(), item);
                self.landed.insert(
                    target.clone(),
                    if moving { self.landed[&source] } else { at },
                );
                self.record(&target, at);
                taken.push((target, pick.material));
            }
            for allocation in &mut self.allocations {
                if moving
                    && split
                    && !matches!(allocation.mode, AllocationMode::Committed)
                    && allocation.item_refs.contains(&source)
                {
                    allocation.item_refs.retain(|r| r != &source);
                    allocation.item_refs.extend(
                        taken
                            .iter()
                            .filter(|(id, _)| {
                                self.items[id].split_from_ref.as_ref() == Some(&source)
                            })
                            .map(|(id, _)| id.clone()),
                    );
                    if let Some(child) = &remainder {
                        allocation.item_refs.push(child.clone());
                    }
                }
                if !moving
                    && matches!(allocation.mode, AllocationMode::HardReserved)
                    && allocation.demand_ref == job.task
                    && allocation.item_refs.contains(&source)
                {
                    allocation.quantity = quantity(
                        amount(&allocation.quantity) - total,
                        &allocation.quantity.unit,
                    );
                    allocation.item_refs.retain(|r| *r != source);
                    if let Some(child) = &remainder {
                        allocation.item_refs.push(child.clone());
                    }
                }
            }
            self.allocations.retain(|a| {
                !matches!(a.mode, AllocationMode::HardReserved) || amount(&a.quantity) > 0
            });
        }
        Ok(taken)
    }
    pub fn landed_at(&self, item: &ObjectId) -> u64 {
        self.landed[item]
    }
    pub fn location_occupied(&self, location: &ObjectId, unit: &UnitId) -> u128 {
        self.items
            .values()
            .filter(|i| {
                !matches!(i.lifecycle, Lifecycle::Transformed)
                    && matches!(&i.holder,HolderRef::Location(l) if l==location)
                    && i.quantity.unit == *unit
            })
            .map(|i| amount(&i.quantity) as u128)
            .sum()
    }
    pub fn move_item(&mut self, key: &ObjectId, holder: HolderRef, at: u64) -> TransferEvent {
        let item = self.items.get_mut(key).unwrap();
        let previous = item.holder.clone();
        item.holder = holder.clone();
        if matches!(holder, HolderRef::Location(_)) {
            self.landed.insert(key.clone(), at);
        }
        self.record(key, at);
        TransferEvent {
            item_ref: key.clone(),
            from_holder: previous,
            to_holder: holder,
        }
    }
    pub fn add_output(&mut self, item: RuntimeItem, at: u64) {
        let key = item.id.clone();
        self.items.insert(key.clone(), item);
        self.landed.insert(key.clone(), at);
        self.record(&key, at);
    }
    pub fn end_allocation(&mut self, item: &ObjectId) {
        self.allocations.retain(|a| {
            !matches!(a.mode, AllocationMode::Committed) || !a.item_refs.contains(item)
        });
        self.items
            .get_mut(item)
            .unwrap()
            .allocation_refs
            .retain(|id| self.allocations.iter().any(|a| a.id == *id));
    }
}

#[cfg(test)]
mod candidate_tests {
    use super::*;
    use serde_json::json;

    fn flags(stock: &Stock, candidates: &PlanCandidates, item: &str, task: &str) -> (bool, bool) {
        let candidate = candidates.values().flatten()
            .find(|candidate| candidate.id.as_str() == item).unwrap();
        stock.reserved_for(candidate, &ObjectId::try_from(task.to_owned()).unwrap())
    }

    #[test]
    fn member_metadata_keeps_reservation_modes_and_rebuilds_after_actual_release() {
        let value: serde_json::Value = serde_json::from_str(include_str!(
            "../../commands/tests/fixtures/production/local-assembly-pack.json"
        )).unwrap();
        let scenario: ScenarioInput = serde_json::from_value(value["scenario"].clone()).unwrap();
        let input = EngineInput {
            run_id: RunId::try_from("candidate-flags".to_owned()).unwrap(),
            input_ref: serde_json::from_value(value["input_ref"].clone()).unwrap(),
            project: serde_json::from_value(value["project"].clone()).unwrap(),
            scope: scenario.scope.clone(), scenario,
            definitions: serde_json::from_value(value["definitions"].clone()).unwrap(),
        };
        let mut stock = Stock::new(&input, plan_initial(&input).unwrap());
        stock.allocations = serde_json::from_value(json!([
            {"id":"HA","demand_ref":"TASK-A","item_refs":["shell-001"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"},
            {"id":"HB","demand_ref":"TASK-B","item_refs":["shell-002"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"hard_reserved"},
            {"id":"CA","demand_ref":"TASK-A","item_refs":["pcb-001"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"committed"},
            {"id":"SB","demand_ref":"TASK-B","item_refs":["carton-001"],"quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},"mode":"soft_plan"}
        ])).unwrap();
        for allocation in &stock.allocations {
            for id in &allocation.item_refs {
                stock.items.get_mut(id).unwrap().allocation_refs.push(allocation.id.clone());
            }
        }
        {
            let candidates = stock.plan_candidates();
            assert_eq!(flags(&stock, &candidates, "shell-001", "TASK-A"), (true, false));
            assert_eq!(flags(&stock, &candidates, "shell-001", "TASK-B"), (false, true));
            assert_eq!(flags(&stock, &candidates, "shell-002", "TASK-B"), (true, false));
            assert_eq!(flags(&stock, &candidates, "shell-002", "TASK-A"), (false, true));
            // Committed material is unavailable even to its own demand.
            assert_eq!(flags(&stock, &candidates, "pcb-001", "TASK-A"), (false, true));
            assert_eq!(flags(&stock, &candidates, "pcb-001", "TASK-B"), (false, true));
            assert_eq!(flags(&stock, &candidates, "carton-001", "TASK-A"), (false, false));
            assert_eq!(flags(&stock, &candidates, "carton-002", "TASK-B"), (false, false));
        }
        // The actual release compacts allocations: the next dispatch must
        // rebuild indices rather than retain a previous call's metadata.
        stock.end_allocation(&ObjectId::try_from("pcb-001".to_owned()).unwrap());
        let candidates = stock.plan_candidates();
        assert_eq!(flags(&stock, &candidates, "pcb-001", "TASK-A"), (false, false));
        assert_eq!(flags(&stock, &candidates, "shell-002", "TASK-B"), (true, false));
        assert_eq!(flags(&stock, &candidates, "carton-001", "TASK-B"), (false, false));
        let free = candidates.values().flatten().find(|c| c.id.as_str() == "carton-002").unwrap();
        assert!(free.allocation_indices.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn inline_operations_initialize_actual_item_attributes_with_and_without_phase_flow() {
        // Isolate stock initialization; operation admission/execution is covered by Host QA.
        let mut value: Value = serde_json::from_str(include_str!("../../commands/tests/fixtures/production/local-assembly-pack.json")).unwrap();
        let mut operation = value["definitions"][0]["payload"]["data"].clone();
        operation["material_flow"] = json!({"primary_role":"shell","auxiliary_roles":[],"acquire_after_phase":null,"consume_after_phase":null,"reserve_before_phase":null,"deliver_after_phase":null,"required_primary_attributes":{},"attributes_after_consume":{}});
        value["definitions"][4]["payload"]["data"]["behavior"] = json!({"selected":"inline","inline":operation,"operation_ref":null});
        value["definitions"].as_array_mut().unwrap().retain(|d| d["payload"]["category"] != "operations");
        value["project"]["item_types"][0]["attributes"] = json!({"presentation_state":{"kind":"enum","value":"raw"}});
        let scenario: ScenarioInput = serde_json::from_value(value["scenario"].clone()).unwrap();
        let input = EngineInput {run_id:RunId::try_from("inline-stock-test".to_owned()).unwrap(),
            input_ref:serde_json::from_value(json!({"project_id":"P1","project_revision":1,"scenario_id":scenario.id,"scenario_revision":1,"frozen_project_file":{"path":"test/project.json","kind":"project"},"frozen_scenario_file":{"path":"test/scenario.json","kind":"scenario"}})).unwrap(),
            project:serde_json::from_value(value["project"].clone()).unwrap(),definitions:serde_json::from_value(value["definitions"].clone()).unwrap(),scope:scenario.scope.clone(),scenario};
        for with_flow in [true, false] {
        let mut input = input.clone();
        if !with_flow {
            for definition in &mut input.definitions {
                if let ModulePayload::Devices(device) = &mut definition.payload {
                    if let Some(operation) = &mut device.behavior.inline {
                        operation.material_flow = None;
                    }
                }
            }
        }
        let stock = Stock::new(&input, plan_initial(&input).unwrap());
        let shell = ObjectId::try_from("shell-001".to_owned()).unwrap();
        assert!(matches!(stock.items[&shell].attributes.get("presentation_state"), Some(TypedValue::Enum(v)) if v=="raw"));
        assert_eq!(stock.attributes_for(&stock.items[&shell].item_type_ref.id).len(), 1);
        }
    }
}
