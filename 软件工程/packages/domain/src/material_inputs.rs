//! Read-only checks of declared material inputs, not a runtime stock or reservation ledger.
use std::collections::{BTreeMap, BTreeSet};

use crate::*;

// A borrowed field location, never serialized or retained outside this check.
struct ItemAt<'a> {
    item: &'a InitialItem,
    pointer: String,
    group: usize,
}

fn issue(
    issues: &mut Vec<Issue>,
    scenario: &ScenarioInput,
    pointer: impl Into<String>,
    code: &str,
    message: impl Into<String>,
) {
    issues.push(Issue {
        code: format!("business_inputs.material_{code}"),
        object_ref: Some(ObjectRef {
            kind: ObjectKind::Scenario,
            id: ObjectId::try_from(scenario.id.as_str().to_owned())
                .expect("typed ScenarioId is a valid ObjectId"),
        }),
        field_path: Some(pointer.into()),
        message: message.into(),
        blocks: vec![
            CapabilityPurpose::Execution,
            CapabilityPurpose::CompleteConclusion,
        ],
    });
}

fn escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}
fn same_type(a: &SubjectRevisionRef, b: &SubjectRevisionRef) -> bool {
    a.id == b.id && a.revision == b.revision
}
fn amount(q: &Quantity) -> u128 {
    u128::from(q.value.coefficient.value())
        * 10_u128.pow(9 - u32::from(q.value.decimal_places.value()))
}

fn quantity(
    issues: &mut Vec<Issue>,
    scenario: &ScenarioInput,
    pointer: &str,
    q: &Quantity,
    expected: Option<&UnitId>,
) -> bool {
    let mut valid = true;
    if expected.is_some_and(|unit| *unit != q.unit) {
        issue(
            issues,
            scenario,
            format!("{pointer}/unit"),
            "unit_mismatch",
            "物料数量单位与所引用的明确单位不一致，未提供单位换算。",
        );
        valid = false;
    }
    if q.unit.as_str() == "piece" && q.value.decimal_places.value() != 0 {
        issue(
            issues,
            scenario,
            format!("{pointer}/value/decimal_places"),
            "fractional_piece",
            "离散piece须为decimal_places=0的整数声明。",
        );
        valid = false;
    }
    valid
}

fn unique(index: &BTreeMap<&ObjectId, Vec<usize>>, id: &ObjectId) -> Option<usize> {
    index
        .get(id)
        .filter(|matches| matches.len() == 1)
        .map(|matches| matches[0])
}

/// Diagnose one Scenario's immutable declarations. No stock, split, claim or Run is created.
pub fn check_material_inputs(project: &ProjectState, scenario: &ScenarioInput) -> Vec<Issue> {
    let mut issues = vec![];
    let mut items: Vec<ItemAt<'_>> = scenario
        .initial_conditions
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| ItemAt {
            item,
            pointer: format!("/initial_conditions/items/{i}"),
            group: 0,
        })
        .collect();
    let mut arrival_ids = BTreeSet::new();
    for (i, arrival) in scenario.arrivals.iter().enumerate() {
        if !arrival_ids.insert(&arrival.id) {
            issue(
                &mut issues,
                scenario,
                format!("/arrivals/{i}/id"),
                "duplicate_arrival",
                "未来到料计划ID重复，不能静默合并。",
            );
        }
        if !project
            .locations
            .iter()
            .any(|location| location.id == arrival.destination)
        {
            issue(
                &mut issues,
                scenario,
                format!("/arrivals/{i}/destination"),
                "missing_arrival_destination",
                "未来到料的实际接收位置尚未声明。",
            );
        }
        for (j, item) in arrival.items.iter().enumerate() {
            items.push(ItemAt {
                item,
                pointer: format!("/arrivals/{i}/items/{j}"),
                group: i + 1,
            });
        }
    }
    let mut index: BTreeMap<&ObjectId, Vec<usize>> = BTreeMap::new();
    for (i, view) in items.iter().enumerate() {
        let matches = index.entry(&view.item.id).or_default();
        if !matches.is_empty() {
            issue(
                &mut issues,
                scenario,
                format!("{}/id", view.pointer),
                "duplicate_item",
                "起点或未来到料中重复声明同一物品ID，不能当成两件实物。",
            );
        }
        matches.push(i);
    }
    let mut source_valid = vec![true; items.len()];
    let mut parents = vec![None; items.len()];
    let mut content_parents: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut physical_container_fields: BTreeMap<usize, String> = BTreeMap::new();
    let mut component_parents: BTreeMap<usize, BTreeMap<usize, String>> = BTreeMap::new();
    for (i, view) in items.iter().enumerate() {
        let material = project.item_types.iter().find(|m| {
            same_type(
                &SubjectRevisionRef {
                    id: m.id.clone(),
                    revision: m.revision.clone(),
                },
                &view.item.item_type_ref,
            )
        });
        if material.is_none() {
            issue(
                &mut issues,
                scenario,
                format!("{}/item_type_ref", view.pointer),
                "missing_item_version",
                "物料引用的精确物品类型版本不存在，不能选择最新版本替代。",
            );
            source_valid[i] = false;
        }
        source_valid[i] &= quantity(
            &mut issues,
            scenario,
            &format!("{}/quantity", view.pointer),
            &view.item.quantity,
            material.map(|m| &m.unit),
        );
        match &view.item.holder {
            HolderRef::Location(id) => {
                if !project.locations.iter().any(|location| location.id == *id) {
                    issue(
                        &mut issues,
                        scenario,
                        format!("{}/holder/id", view.pointer),
                        "missing_holder_location",
                        "物品持有位置尚未声明。",
                    );
                }
            }
            HolderRef::Container(id) => match unique(&index, id) {
                None => issue(
                    &mut issues,
                    scenario,
                    format!("{}/holder/id", view.pointer),
                    "missing_container",
                    "父容器不存在或身份重复，不能任选一个。",
                ),
                Some(parent) => {
                    physical_container_fields
                        .entry(i)
                        .or_insert_with(|| format!("{}/holder", view.pointer));
                    physical_container_fields
                        .entry(parent)
                        .or_insert_with(|| format!("{}/holder/id", view.pointer));
                    if view.group != items[parent].group {
                        issue(
                            &mut issues,
                            scenario,
                            format!("{}/holder/id", view.pointer),
                            "cross_source_container",
                            "容器关系跨越起点或不同到料声明，不能借未来物品提前形成当前持有关系。",
                        );
                    } else {
                        parents[i] = Some(parent);
                    }
                    if items[parent]
                        .item
                        .contents
                        .iter()
                        .filter(|child| **child == view.item.id)
                        .count()
                        != 1
                    {
                        issue(
                            &mut issues,
                            scenario,
                            format!("{}/holder/id", view.pointer),
                            "container_reverse_mismatch",
                            "子物品的holder.container与父容器contents不一致。",
                        );
                    }
                }
            },
            HolderRef::Job(_) | HolderRef::Transport(_) => issue(
                &mut issues,
                scenario,
                format!("{}/holder", view.pointer),
                "unsupported_runtime_holder",
                "本版起点未声明可恢复的Job/Transport运行快照，不能用生产任务或供货计划冒充运行持有者。",
            ),
        }
        let mut seen_contents = BTreeSet::new();
        if !view.item.contents.is_empty() {
            physical_container_fields
                .entry(i)
                .or_insert_with(|| format!("{}/contents", view.pointer));
        }
        for (j, child_id) in view.item.contents.iter().enumerate() {
            let pointer = format!("{}/contents/{j}", view.pointer);
            if !seen_contents.insert(child_id) {
                issue(
                    &mut issues,
                    scenario,
                    &pointer,
                    "duplicate_content",
                    "同一容器contents重复引用一件物品。",
                );
            }
            if *child_id == view.item.id {
                issue(
                    &mut issues,
                    scenario,
                    &pointer,
                    "container_self_content",
                    "容器不能包含自身。",
                );
            }
            match unique(&index, child_id) {
                None => issue(
                    &mut issues,
                    scenario,
                    &pointer,
                    "missing_content",
                    "容器内容物不存在或身份不明确。",
                ),
                Some(child) => {
                    physical_container_fields
                        .entry(child)
                        .or_insert_with(|| pointer.clone());
                    if view.group != items[child].group {
                        issue(
                            &mut issues,
                            scenario,
                            &pointer,
                            "cross_source_container",
                            "contents不能把未来或另一到料声明中的物品提前当作本容器当前内容。",
                        );
                    } else {
                        content_parents.entry(child).or_default().insert(i);
                    }
                    if !matches!(&items[child].item.holder, HolderRef::Container(parent) if *parent == view.item.id)
                    {
                        issue(
                            &mut issues,
                            scenario,
                            &pointer,
                            "content_holder_mismatch",
                            "容器contents与子物品唯一holder不一致。",
                        );
                    }
                }
            }
        }
        let mut seen_components = BTreeSet::new();
        for (j, component) in view.item.components.iter().enumerate() {
            let pointer = format!("{}/components/{j}", view.pointer);
            if !seen_components.insert(component) {
                issue(
                    &mut issues,
                    scenario,
                    &pointer,
                    "duplicate_component",
                    "同一装配体重复列出同一组成来源，不能作为两份来源使用。",
                );
            }
            if *component == view.item.id {
                issue(
                    &mut issues,
                    scenario,
                    &pointer,
                    "component_self_reference",
                    "组成来源不能引用自身；components不作为第二holder。",
                );
                continue;
            }
            let Some(source) = unique(&index, component) else {
                issue(
                    &mut issues,
                    scenario,
                    &pointer,
                    "unresolved_component_source",
                    "组成来源未在当前来源组中唯一声明，保留缺证ID，不重建现货。",
                );
                continue;
            };
            if view.group != items[source].group {
                issue(
                    &mut issues,
                    scenario,
                    &pointer,
                    "cross_source_component",
                    "组成来源须属于同一起点或同一Arrival声明，不能跨时间来源引用。",
                );
                continue;
            }
            component_parents
                .entry(source)
                .or_default()
                .entry(i)
                .or_insert(pointer);
        }
    }
    for (child, holders) in content_parents {
        if holders.len() > 1 {
            issue(
                &mut issues,
                scenario,
                format!("{}/holder", items[child].pointer),
                "multiple_containers",
                "同一内容物被多个容器声明包含，唯一持有关系不成立。",
            );
        }
    }
    let mut colors = vec![0_u8; items.len()];
    for start in 0..items.len() {
        if colors[start] != 0 {
            continue;
        }
        let mut path = vec![];
        let mut next = Some(start);
        while let Some(i) = next {
            if colors[i] != 0 {
                if colors[i] == 1 {
                    issue(
                        &mut issues,
                        scenario,
                        format!("{}/holder", items[i].pointer),
                        "container_cycle",
                        "容器持有关系存在环，无法追到一个实际位置。",
                    );
                }
                break;
            }
            colors[i] = 1;
            path.push(i);
            next = parents[i];
        }
        for i in path {
            colors[i] = 2;
        }
    }
    let mut composition_parent = vec![None; items.len()];
    for (source, owners) in &component_parents {
        if owners.len() == 1 {
            composition_parent[*source] = owners.keys().next().copied();
        } else {
            for pointer in owners.values() {
                issue(
                    &mut issues,
                    scenario,
                    pointer,
                    "multiple_component_parents",
                    "同一组成来源被多个装配体声明使用，不能重复形成产品来源。",
                );
            }
        }
        if let Some(pointer) = physical_container_fields.get(source) {
            issue(
                &mut issues,
                scenario,
                pointer,
                "component_container_conflict",
                "已转化的组成来源不能同时作为容器或容器内容；普通Location仅保留来源位置。",
            );
        }
    }
    colors.fill(0);
    for start in 0..items.len() {
        if colors[start] != 0 {
            continue;
        }
        let mut path = vec![];
        let mut next = Some(start);
        while let Some(i) = next {
            if colors[i] != 0 {
                if colors[i] == 1 {
                    issue(
                        &mut issues,
                        scenario,
                        format!("{}/components", items[i].pointer),
                        "component_cycle",
                        "组成来源关系存在环，不能形成真实来源链。",
                    );
                }
                break;
            }
            colors[i] = 1;
            path.push(i);
            next = composition_parent[i];
        }
        for i in path {
            colors[i] = 2;
        }
    }
    let mut supply_ids = BTreeSet::new();
    for (i, supply) in scenario.supplies.iter().enumerate() {
        let pointer = format!("/supplies/{i}");
        if !supply_ids.insert(&supply.id) {
            issue(
                &mut issues,
                scenario,
                format!("{pointer}/id"),
                "duplicate_supply",
                "供货关系ID重复。",
            );
        }
        for (field, id) in [
            ("source", &supply.source),
            ("destination", &supply.destination),
        ] {
            if !project.locations.iter().any(|location| location.id == *id) {
                issue(
                    &mut issues,
                    scenario,
                    format!("{pointer}/{field}"),
                    "missing_supply_location",
                    "供货关系的实际源或接收位置尚未声明。",
                );
            }
        }
        if let Some(path) = &supply.presentation_path {
            let endpoints = [project.locations.iter().find(|l| l.id == supply.source), project.locations.iter().find(|l| l.id == supply.destination)];
            let valid_hall = project.halls.iter().any(|h| h.id == path.hall_id)
                && endpoints.iter().all(|l| l.is_some_and(|l| l.hall_id.as_ref() == Some(&path.hall_id)));
            let ends_match = path.waypoints_m.len() >= 2 && endpoints.iter().zip([path.waypoints_m.first(), path.waypoints_m.last()]).all(|(location, point)| {
                let (Some(location), Some(point)) = (location, point) else { return false };
                let p = &location.position;
                let scale = if matches!(p.unit, LengthUnit::Millimeter) { 0.001 } else { 1.0 };
                let v = &p.translation;
                let xyz = if matches!(p.coordinate_frame, CoordinateFrame::RightHandedZUp) { [v.x.value(),v.y.value(),v.z.value()] } else { [v.x.value(),-v.z.value(),v.y.value()] };
                xyz.into_iter().zip([point.x.value(),point.y.value(),point.z.value()]).all(|(a,b)| (a*scale-b).abs() < 0.001)
            });
            if !valid_hall || !ends_match {
                issue(&mut issues, scenario, format!("{pointer}/presentation_path"), "transport_path", "运输显示路径须位于源、目标共同的已声明厂房，至少含两个点，首尾须连接实际库位。路径以厂房局部米制Z向上坐标声明。");
            }
        }
        let material = project.item_types.iter().find(|m| {
            m.id == supply.item_type_ref.id && m.revision == supply.item_type_ref.revision
        });
        if material.is_none() {
            issue(
                &mut issues,
                scenario,
                format!("{pointer}/item_type_ref"),
                "missing_supply_item_version",
                "供货目标引用的精确物品版本不存在。",
            );
        }
        quantity(
            &mut issues,
            scenario,
            &format!("{pointer}/target_quantity"),
            &supply.target_quantity,
            material.map(|m| &m.unit),
        );
        if !project
            .module_refs
            .iter()
            .any(|m| m.id == supply.transport_ref.id && m.version == supply.transport_ref.version)
        {
            issue(
                &mut issues,
                scenario,
                format!("{pointer}/transport_ref"),
                "missing_transport_module",
                "供货运输模块的精确版本尚未在工程中声明；这不是required文件缺失豁免。",
            );
        }
        for (role, id) in &supply.resources {
            if !project.resources.iter().any(|resource| resource.id == *id) {
                issue(
                    &mut issues,
                    scenario,
                    format!("{pointer}/resources/{}", escape(role.as_str())),
                    "missing_supply_resource",
                    "供货关系绑定的实际资源尚未声明。",
                );
            }
        }
    }
    let mut allocation_ids = BTreeSet::new();
    // Temporary arithmetic bounds for input validation only; no reservation is installed.
    let mut remaining: BTreeMap<usize, u128> = BTreeMap::new();
    for (i, allocation) in scenario
        .initial_conditions
        .material_allocations
        .iter()
        .enumerate()
    {
        let pointer = format!("/initial_conditions/material_allocations/{i}");
        let unique_id = allocation_ids.insert(&allocation.id);
        if !unique_id {
            issue(
                &mut issues,
                scenario,
                format!("{pointer}/id"),
                "duplicate_allocation",
                "物料分配ID重复，不能形成独立份额。",
            );
        }
        let mut valid = quantity(
            &mut issues,
            scenario,
            &format!("{pointer}/quantity"),
            &allocation.quantity,
            None,
        );
        valid &= unique_id;
        match allocation.mode {
            AllocationMode::SoftPlan => {
                let count = scenario
                    .orders
                    .iter()
                    .filter(|order| order.id == allocation.demand_ref)
                    .count()
                    + scenario
                        .production_tasks
                        .iter()
                        .filter(|task| task.id == allocation.demand_ref)
                        .count();
                if count != 1 {
                    issue(
                        &mut issues,
                        scenario,
                        format!("{pointer}/demand_ref"),
                        "missing_soft_demand",
                        "软计划须引用本方案唯一现存Order或ProductionTask。",
                    );
                    valid = false;
                }
            }
            AllocationMode::HardReserved => {
                if scenario
                    .production_tasks
                    .iter()
                    .filter(|task| task.id == allocation.demand_ref)
                    .count()
                    != 1
                {
                    issue(
                        &mut issues,
                        scenario,
                        format!("{pointer}/demand_ref"),
                        "missing_hard_task",
                        "硬预留须明确引用本方案唯一现存ProductionTask，不把订单或运行Job当生产任务。",
                    );
                    valid = false;
                }
                if allocation.item_refs.is_empty() {
                    issue(
                        &mut issues,
                        scenario,
                        format!("{pointer}/item_refs"),
                        "missing_hard_source",
                        "硬预留必须声明起点实物源，不能从BOM或目标补造物品。",
                    );
                    valid = false;
                }
            }
            AllocationMode::Committed => {
                issue(
                    &mut issues,
                    scenario,
                    format!("{pointer}/mode"),
                    "unsupported_committed",
                    "同起点仅支持soft_plan/hard_reserved；committed与WIP恢复保留具名不支持，不改写模式。",
                );
                valid = false;
            }
        }
        let mut sources = vec![];
        let mut seen = BTreeSet::new();
        for (j, id) in allocation.item_refs.iter().enumerate() {
            let field = format!("{pointer}/item_refs/{j}");
            if !seen.insert(id) {
                issue(
                    &mut issues,
                    scenario,
                    &field,
                    "duplicate_allocation_source",
                    "同条分配重复引用同一源，不能把它算两件实物。",
                );
                valid = false;
                continue;
            }
            let Some(source) = unique(&index, id) else {
                issue(
                    &mut issues,
                    scenario,
                    &field,
                    "missing_allocation_source",
                    "分配源在本方案声明中不存在或身份不明确。",
                );
                valid = false;
                continue;
            };
            valid &= quantity(
                &mut issues,
                scenario,
                &format!("{pointer}/quantity"),
                &allocation.quantity,
                Some(&items[source].item.quantity.unit),
            );
            if matches!(allocation.mode, AllocationMode::HardReserved) && items[source].group != 0 {
                issue(
                    &mut issues,
                    scenario,
                    &field,
                    "future_hard_source",
                    "未来到料只可作为软计划引用，不能成为起点硬预留现货。",
                );
                valid = false;
            }
            if matches!(allocation.mode, AllocationMode::HardReserved)
                && component_parents.contains_key(&source)
            {
                issue(
                    &mut issues,
                    scenario,
                    &field,
                    "composed_hard_source",
                    "已转化的组成来源不能再次作为起点硬预留可用源；不因保留来源Location而恢复现货。",
                );
                valid = false;
            }
            if !source_valid[source] {
                valid = false;
            }
            sources.push(source);
        }
        if !matches!(allocation.mode, AllocationMode::HardReserved) {
            continue;
        }
        if let Some(first) = sources.first() {
            for source in &sources[1..] {
                if !same_type(
                    &items[*first].item.item_type_ref,
                    &items[*source].item.item_type_ref,
                ) || items[*first].item.quantity.unit != items[*source].item.quantity.unit
                {
                    issue(
                        &mut issues,
                        scenario,
                        format!("{pointer}/item_refs"),
                        "mixed_hard_sources",
                        "多源硬预留必须是同物品类型、精确版本和单位。",
                    );
                    valid = false;
                }
            }
        }
        if sources.len() > 1 && valid {
            let total = sources.iter().try_fold(0_u128, |sum, source| {
                sum.checked_add(amount(&items[*source].item.quantity))
            });
            if total != Some(amount(&allocation.quantity)) {
                issue(
                    &mut issues,
                    scenario,
                    format!("{pointer}/quantity"),
                    "multi_source_partial",
                    "多源硬预留仅支持各源全量之和，不能猜测每源部分取量或舍入溢出的总量。",
                );
                valid = false;
            }
        }
        if !valid {
            continue;
        }
        for source in &sources {
            let requested = if sources.len() == 1 {
                amount(&allocation.quantity)
            } else {
                amount(&items[*source].item.quantity)
            };
            let available = remaining
                .entry(*source)
                .or_insert_with(|| amount(&items[*source].item.quantity));
            if requested > *available {
                issue(
                    &mut issues,
                    scenario,
                    format!("{pointer}/quantity"),
                    "hard_overallocated",
                    format!(
                        "源 {} 的累计硬预留份额超过明确实量；不按成品订单数量封顶材料量。",
                        items[*source].item.id.as_str()
                    ),
                );
            }
            *available = available.saturating_sub(requested);
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(value: &str) -> ObjectId {
        ObjectId::try_from(value.to_owned()).unwrap()
    }
    fn q(value: u64, unit: &str) -> Quantity {
        Quantity {
            value: DecimalAmount {
                coefficient: SafeUint::try_from(value).unwrap(),
                decimal_places: DecimalPlaces::try_from(0).unwrap(),
            },
            unit: UnitId::try_from(unit.to_owned()).unwrap(),
        }
    }
    fn stock(name: &str, value: u64) -> InitialItem {
        InitialItem {
            id: id(name),
            item_type_ref: SubjectRevisionRef {
                id: id("item"),
                revision: SubjectRevisionId::try_from("r1".to_owned()).unwrap(),
            },
            quantity: q(value, "piece"),
            holder: HolderRef::Location(id("source")),
            quality: QualityState::Pending,
            components: vec![],
            contents: vec![],
            source: SourceBasis::UserProvided,
        }
    }
    fn allocation(
        name: &str,
        demand: &str,
        sources: &[&str],
        value: u64,
        mode: AllocationMode,
    ) -> MaterialAllocation {
        MaterialAllocation {
            id: id(name),
            demand_ref: id(demand),
            item_refs: sources.iter().map(|s| id(s)).collect(),
            quantity: q(value, "piece"),
            mode,
        }
    }
    fn fixture() -> (ProjectState, ScenarioInput) {
        let (mut project, scenarios) = crate::business_inputs::tests::fixture();
        project.item_types[0].tracking = TrackingKind::Batch;
        let mut location = project.locations[0].clone();
        location.id = id("source");
        location.purpose = LocationPurpose::Input;
        location.ordinary_issue_enabled = true;
        project.locations.push(location);
        let transport: ModuleRef =
            serde_json::from_value(json!({"id":"transport","version":"1.0.0"})).unwrap();
        project.module_refs.push(transport.clone());
        let mut scenario = scenarios.into_values().next().unwrap();
        scenario.orders[0].quantity = Some(q(8, "piece"));
        scenario.initial_conditions.items = vec![stock("batch10", 10)];
        scenario.arrivals = vec![ArrivalPlan {
            id: id("arrival"),
            at_ms: SimTimeMs(SafeUint::try_from(100_u64).unwrap()),
            destination: id("customer"),
            items: vec![stock("future5", 5)],
            release_policy: ReleasePolicy::InspectionRequired,
        }];
        scenario.supplies = vec![SupplyRelation {
            id: id("supply"),
            source: id("source"),
            destination: id("customer"),
            item_type_ref: scenario.initial_conditions.items[0].item_type_ref.clone(),
            target_quantity: q(20, "piece"),
            window: scenario.window.clone(),
            transport_ref: transport,
            resources: BTreeMap::new(),
            departure_windows: vec![],
            allow_partial_load: true,
            presentation_path: None,
        }];
        (project, scenario)
    }
    fn has<'a>(issues: &'a [Issue], code: &str) -> &'a Issue {
        issues
            .iter()
            .find(|issue| issue.code == format!("business_inputs.material_{code}"))
            .unwrap_or_else(|| panic!("missing {code}: {issues:?}"))
    }

    #[test]
    fn transport_display_path_connects_declared_same_hall_stocks_and_old_inputs_remain_optional() {
        let (mut project, mut scenario) = fixture();
        let old = serde_json::to_value(&scenario.supplies[0]).unwrap();
        assert!(old.get("presentation_path").is_none());
        let pose = serde_json::to_value(&project.locations[0].position).unwrap();
        project.halls.push(serde_json::from_value(json!({"id":"hall","title":"hall","origin":pose,"width_m":20,"depth_m":20,"height_m":5,"input_locations":[],"output_locations":[]})).unwrap());
        for location in &mut project.locations { location.hall_id = Some(id("hall")); }
        let end = project.locations.iter_mut().find(|l|l.id.as_str()=="customer").unwrap();
        end.position.translation.x = FiniteNumber::try_from(10.0).unwrap();
        scenario.supplies[0].presentation_path = Some(serde_json::from_value(json!({"hall_id":"hall","waypoints_m":[{"x":0,"y":0,"z":0},{"x":5,"y":0,"z":0},{"x":10,"y":0,"z":0}],"cargo_layout":{"assignment":"item_id_order","slots":[]}})).unwrap());
        assert!(check_material_inputs(&project,&scenario).is_empty());
        scenario.supplies[0].presentation_path.as_mut().unwrap().waypoints_m[2].x=FiniteNumber::try_from(9.0).unwrap();
        assert_eq!(has(&check_material_inputs(&project,&scenario),"transport_path").field_path.as_deref(),Some("/supplies/0/presentation_path"));
        scenario.supplies[0].presentation_path.as_mut().unwrap().waypoints_m[2].x=FiniteNumber::try_from(10.0).unwrap();
        project.locations[0].hall_id=None;
        has(&check_material_inputs(&project,&scenario),"transport_path");
    }

    #[test]
    fn six_input_kinds_stay_separate_and_reference_gaps_are_located() {
        let (mut project, mut scenario) = fixture();
        let before = serde_json::to_value((&project, &scenario)).unwrap();
        assert!(check_material_inputs(&project, &scenario).is_empty());
        assert_eq!(serde_json::to_value((&project, &scenario)).unwrap(), before);
        assert_eq!(scenario.initial_conditions.items.len(), 1);
        assert_eq!(
            scenario.initial_conditions.items[0]
                .quantity
                .value
                .coefficient
                .value(),
            10
        );
        assert_eq!(
            scenario.arrivals[0].items[0]
                .quantity
                .value
                .coefficient
                .value(),
            5
        );
        assert_eq!(
            scenario.supplies[0]
                .target_quantity
                .value
                .coefficient
                .value(),
            20
        );
        assert_eq!(
            scenario.orders[0]
                .quantity
                .as_ref()
                .unwrap()
                .value
                .coefficient
                .value(),
            8
        );
        project.module_refs.retain(|m| m.id.as_str() != "transport");
        scenario.supplies[0].resources.insert(
            RoleId::try_from("/~worker".to_owned()).unwrap(),
            id("missing-worker"),
        );
        scenario.arrivals[0].destination = id("missing-receiver");
        scenario.initial_conditions.items[0].holder = HolderRef::Location(id("missing-source"));
        scenario.arrivals[0].items[0].item_type_ref.revision =
            SubjectRevisionId::try_from("r2".to_owned()).unwrap();
        let issues = check_material_inputs(&project, &scenario);
        let transport = has(&issues, "missing_transport_module");
        assert_eq!(
            transport.object_ref.as_ref().unwrap().id.as_str(),
            scenario.id.as_str()
        );
        assert_eq!(
            transport.field_path.as_deref(),
            Some("/supplies/0/transport_ref")
        );
        assert_eq!(
            has(&issues, "missing_supply_resource")
                .field_path
                .as_deref(),
            Some("/supplies/0/resources/~1~0worker")
        );
        has(&issues, "missing_arrival_destination");
        has(&issues, "missing_holder_location");
        has(&issues, "missing_item_version");
        for issue in &issues {
            assert!(
                issue
                    .blocks
                    .iter()
                    .any(|b| matches!(b, CapabilityPurpose::Execution))
            );
            assert!(
                issue
                    .blocks
                    .iter()
                    .any(|b| matches!(b, CapabilityPurpose::CompleteConclusion))
            );
        }
    }

    #[test]
    fn soft_future_plans_are_nonexclusive_and_hard_sources_are_initial_only() {
        let (project, mut scenario) = fixture();
        scenario.initial_conditions.material_allocations = vec![
            allocation("soft-a", "order", &["batch10"], 7, AllocationMode::SoftPlan),
            allocation(
                "soft-b",
                "production",
                &["batch10"],
                7,
                AllocationMode::SoftPlan,
            ),
            allocation(
                "soft-future",
                "order",
                &["future5"],
                9,
                AllocationMode::SoftPlan,
            ),
            allocation("soft-unbound", "order", &[], 8, AllocationMode::SoftPlan),
            allocation(
                "hard-pending",
                "production",
                &["batch10"],
                4,
                AllocationMode::HardReserved,
            ),
        ];
        let before = serde_json::to_value(&scenario).unwrap();
        assert!(check_material_inputs(&project, &scenario).is_empty());
        assert_eq!(serde_json::to_value(&scenario).unwrap(), before);
        assert!(matches!(
            scenario.initial_conditions.items[0].quality,
            QualityState::Pending
        ));
        scenario.initial_conditions.material_allocations = vec![
            allocation(
                "hard-order",
                "order",
                &["batch10"],
                1,
                AllocationMode::HardReserved,
            ),
            allocation(
                "hard-future",
                "production",
                &["future5"],
                1,
                AllocationMode::HardReserved,
            ),
            allocation(
                "committed",
                "production",
                &["batch10"],
                1,
                AllocationMode::Committed,
            ),
            allocation(
                "soft-missing",
                "order",
                &["unknown"],
                1,
                AllocationMode::SoftPlan,
            ),
        ];
        let issues = check_material_inputs(&project, &scenario);
        has(&issues, "missing_hard_task");
        has(&issues, "future_hard_source");
        has(&issues, "unsupported_committed");
        has(&issues, "missing_allocation_source");
    }

    #[test]
    fn partial_hard_claims_use_source_budget_without_splitting_input() {
        let (project, mut scenario) = fixture();
        scenario.initial_conditions.material_allocations = vec![
            allocation(
                "a",
                "production",
                &["batch10"],
                4,
                AllocationMode::HardReserved,
            ),
            allocation(
                "b",
                "production",
                &["batch10"],
                6,
                AllocationMode::HardReserved,
            ),
        ];
        let before = serde_json::to_value(&scenario).unwrap();
        // Material 10 is not capped by this task's finished-product plan of 2.
        assert!(check_material_inputs(&project, &scenario).is_empty());
        assert_eq!(serde_json::to_value(&scenario).unwrap(), before);
        scenario.initial_conditions.material_allocations[1].quantity = q(7, "piece");
        assert_eq!(
            has(
                &check_material_inputs(&project, &scenario),
                "hard_overallocated"
            )
            .field_path
            .as_deref(),
            Some("/initial_conditions/material_allocations/1/quantity")
        );
        scenario.initial_conditions.material_allocations[1].id = id("a");
        has(
            &check_material_inputs(&project, &scenario),
            "duplicate_allocation",
        );
        scenario.initial_conditions.material_allocations = vec![allocation(
            "repeat",
            "production",
            &["batch10", "batch10"],
            10,
            AllocationMode::HardReserved,
        )];
        has(
            &check_material_inputs(&project, &scenario),
            "duplicate_allocation_source",
        );
        assert_eq!(scenario.initial_conditions.items.len(), 1);
        assert_eq!(
            scenario.initial_conditions.items[0]
                .quantity
                .value
                .coefficient
                .value(),
            10
        );
    }

    #[test]
    fn multiple_hard_sources_require_exact_full_same_type_version_and_unit() {
        let (project, mut scenario) = fixture();
        scenario.initial_conditions.items.push(stock("batch2", 2));
        scenario.initial_conditions.material_allocations = vec![allocation(
            "full",
            "production",
            &["batch10", "batch2"],
            12,
            AllocationMode::HardReserved,
        )];
        assert!(check_material_inputs(&project, &scenario).is_empty());
        scenario.initial_conditions.material_allocations[0].quantity = q(4, "piece");
        has(
            &check_material_inputs(&project, &scenario),
            "multi_source_partial",
        );
        scenario.initial_conditions.material_allocations[0].quantity = q(12, "piece");
        scenario.initial_conditions.items[1].item_type_ref.revision =
            SubjectRevisionId::try_from("r2".to_owned()).unwrap();
        has(
            &check_material_inputs(&project, &scenario),
            "mixed_hard_sources",
        );
        scenario.initial_conditions.items[1].item_type_ref.revision =
            SubjectRevisionId::try_from("r1".to_owned()).unwrap();
        scenario.initial_conditions.items[1].quantity.unit =
            UnitId::try_from("kg".to_owned()).unwrap();
        let issues = check_material_inputs(&project, &scenario);
        has(&issues, "unit_mismatch");
        has(&issues, "mixed_hard_sources");
        scenario.initial_conditions.material_allocations[0]
            .item_refs
            .clear();
        has(
            &check_material_inputs(&project, &scenario),
            "missing_hard_source",
        );
    }

    #[test]
    fn container_closure_and_component_origins_are_separate() {
        let (mut project, mut scenario) = fixture();
        let mut box_type = project.item_types[0].clone();
        box_type.id = id("box");
        box_type.unit = UnitId::try_from("box".to_owned()).unwrap();
        project.item_types.push(box_type);
        let mut outer = stock("C1", 1);
        outer.item_type_ref.id = id("box");
        outer.quantity = q(1, "box");
        outer.contents = vec![id("C2")];
        outer.components = vec![id("source-part")];
        let mut inner = outer.clone();
        inner.id = id("C2");
        inner.holder = HolderRef::Container(id("C1"));
        inner.contents = vec![id("P")];
        inner.components.clear();
        let mut part = stock("P", 2);
        part.holder = HolderRef::Container(id("C2"));
        scenario.initial_conditions.items = vec![outer, inner, part, stock("source-part", 1)];
        let valid = scenario.clone();
        let checked = |scenario: &ScenarioInput| {
            let before = serde_json::to_value((&project, scenario)).unwrap();
            let issues = check_material_inputs(&project, scenario);
            assert_eq!(serde_json::to_value((&project, scenario)).unwrap(), before);
            issues
        };
        assert!(checked(&scenario).is_empty());
        assert!(matches!(
            &scenario.initial_conditions.items[3].holder,
            HolderRef::Location(location) if *location == id("source")
        ));

        let mut future_assembly = stock("future-assembly", 1);
        future_assembly.components = vec![id("future5")];
        scenario.arrivals[0].items.push(future_assembly);
        assert!(checked(&scenario).is_empty());

        scenario = valid.clone();
        scenario.initial_conditions.items[0].components = vec![id("future5")];
        assert_eq!(
            has(&checked(&scenario), "cross_source_component")
                .field_path
                .as_deref(),
            Some("/initial_conditions/items/0/components/0")
        );

        scenario = valid.clone();
        let mut later_arrival = scenario.arrivals[0].clone();
        later_arrival.id = id("later-arrival");
        later_arrival.items = vec![stock("later-source", 1)];
        scenario.arrivals.push(later_arrival);
        scenario.arrivals[0].items[0].components = vec![id("later-source")];
        has(&checked(&scenario), "cross_source_component");

        scenario = valid.clone();
        scenario.initial_conditions.items[0]
            .components
            .push(id("source-part"));
        has(&checked(&scenario), "duplicate_component");

        scenario = valid.clone();
        let mut other_assembly = stock("other-assembly", 1);
        other_assembly.components = vec![id("source-part")];
        scenario.initial_conditions.items.push(other_assembly);
        has(&checked(&scenario), "multiple_component_parents");

        scenario = valid.clone();
        let mut first = stock("composition-a", 1);
        let mut second = stock("composition-b", 1);
        first.components = vec![second.id.clone()];
        second.components = vec![first.id.clone()];
        scenario.initial_conditions.items.extend([first, second]);
        has(&checked(&scenario), "component_cycle");

        scenario = valid.clone();
        scenario.initial_conditions.items[3].components = vec![id("source-part")];
        has(&checked(&scenario), "component_self_reference");

        scenario = valid.clone();
        scenario.initial_conditions.items[1]
            .contents
            .push(id("source-part"));
        scenario.initial_conditions.items[3].holder = HolderRef::Container(id("C2"));
        has(&checked(&scenario), "component_container_conflict");

        scenario = valid.clone();
        scenario.initial_conditions.items[1].contents.clear();
        scenario.initial_conditions.items[2].holder = HolderRef::Container(id("source-part"));
        scenario.initial_conditions.items[3].contents = vec![id("P")];
        has(&checked(&scenario), "component_container_conflict");

        scenario = valid.clone();
        scenario.initial_conditions.material_allocations = vec![allocation(
            "source-reserved",
            "production",
            &["source-part"],
            1,
            AllocationMode::HardReserved,
        )];
        has(&checked(&scenario), "composed_hard_source");

        scenario = valid.clone();
        scenario.initial_conditions.material_allocations = vec![
            allocation(
                "historical-soft",
                "production",
                &["source-part"],
                7,
                AllocationMode::SoftPlan,
            ),
            allocation(
                "future-soft",
                "production",
                &["future5"],
                7,
                AllocationMode::SoftPlan,
            ),
        ];
        assert!(checked(&scenario).is_empty());

        scenario = valid.clone();
        scenario.initial_conditions.items[0]
            .components
            .push(id("unknown-origin"));
        has(
            &check_material_inputs(&project, &scenario),
            "unresolved_component_source",
        );
        assert_eq!(scenario.initial_conditions.items.len(), 4);
        scenario = valid.clone();
        let mut another = scenario.initial_conditions.items[0].clone();
        another.id = id("C3");
        another.contents = vec![id("P")];
        scenario.initial_conditions.items.push(another);
        has(
            &check_material_inputs(&project, &scenario),
            "multiple_containers",
        );
        scenario = valid.clone();
        scenario.initial_conditions.items[0].holder = HolderRef::Container(id("C2"));
        scenario.initial_conditions.items[1].contents.push(id("C1"));
        has(
            &check_material_inputs(&project, &scenario),
            "container_cycle",
        );
        scenario = valid.clone();
        scenario.initial_conditions.items[2].holder = HolderRef::Container(id("missing-parent"));
        has(
            &check_material_inputs(&project, &scenario),
            "missing_container",
        );
        scenario = valid.clone();
        scenario.initial_conditions.items[2].holder = HolderRef::Container(id("future5"));
        has(
            &check_material_inputs(&project, &scenario),
            "cross_source_container",
        );
        scenario = valid.clone();
        scenario.initial_conditions.items[0].contents.push(id("C2"));
        has(
            &check_material_inputs(&project, &scenario),
            "duplicate_content",
        );
        scenario = valid.clone();
        scenario.initial_conditions.items[0].holder = HolderRef::Job(id("production"));
        has(
            &check_material_inputs(&project, &scenario),
            "unsupported_runtime_holder",
        );
        scenario = valid;
        scenario.initial_conditions.items[0].holder = HolderRef::Transport(id("supply"));
        has(
            &check_material_inputs(&project, &scenario),
            "unsupported_runtime_holder",
        );
    }

    #[test]
    fn combined_business_diagnostics_preserve_drafts_and_scenario_scope() {
        let (project, mut scenario) = fixture();
        scenario.initial_conditions.material_allocations = vec![allocation(
            "future-hard",
            "production",
            &["future5"],
            1,
            AllocationMode::HardReserved,
        )];
        scenario.arrivals[0].items.push(stock("batch10", 1));
        let before = serde_json::to_value((&project, &scenario)).unwrap();
        let scenarios = BTreeMap::from([(scenario.id.clone(), scenario.clone())]);
        let issues = check_business_inputs(&project, &scenarios);
        has(&issues, "future_hard_source");
        has(&issues, "duplicate_item");
        assert_eq!(serde_json::to_value((&project, &scenario)).unwrap(), before);
        for diagnostic in issues
            .iter()
            .filter(|i| i.code.starts_with("business_inputs.material_"))
        {
            assert_eq!(
                diagnostic.object_ref.as_ref().unwrap().id.as_str(),
                scenario.id.as_str()
            );
            assert!(diagnostic.field_path.as_ref().unwrap().starts_with('/'));
        }
        let mut old = scenario.clone();
        old.initial_conditions.items.clear();
        old.initial_conditions.material_allocations.clear();
        old.arrivals.clear();
        old.supplies.clear();
        old.orders.clear();
        old.production_tasks.clear();
        assert!(check_material_inputs(&project, &old).is_empty());
    }
}
