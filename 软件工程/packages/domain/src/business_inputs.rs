//! Pure diagnostics for current business input drafts; never an inventory or executor.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::*;

fn owner(kind: ObjectKind, id: &str) -> ObjectRef {
    ObjectRef {
        kind,
        id: ObjectId::try_from(id.to_owned()).expect("existing typed ID is a valid ObjectId"),
    }
}

fn problem(
    issues: &mut Vec<Issue>,
    object: &ObjectRef,
    path: impl Into<String>,
    code: &str,
    message: impl Into<String>,
) {
    issues.push(Issue {
        code: format!("business_inputs.{code}"),
        object_ref: Some(object.clone()),
        field_path: Some(path.into()),
        message: message.into(),
        blocks: vec![
            CapabilityPurpose::Execution,
            CapabilityPurpose::CompleteConclusion,
        ],
    });
}

fn same_subject(a: &SubjectRevisionRef, b: &SubjectRevisionRef) -> bool {
    a.id == b.id && a.revision == b.revision
}

fn same_module(a: &ModuleRef, b: &ModuleRef) -> bool {
    a.id == b.id && a.version == b.version
}

fn item<'a>(project: &'a ProjectState, reference: &SubjectRevisionRef) -> Option<&'a ItemType> {
    project
        .item_types
        .iter()
        .find(|x| x.id == reference.id && x.revision == reference.revision)
}
fn product<'a>(
    project: &'a ProjectState,
    reference: &SubjectRevisionRef,
) -> Option<&'a ProductRevision> {
    project
        .products
        .iter()
        .find(|x| x.id == reference.id && x.revision == reference.revision)
}
fn route<'a>(
    project: &'a ProjectState,
    reference: &SubjectRevisionRef,
) -> Option<&'a RouteDefinition> {
    project
        .routes
        .iter()
        .find(|x| x.id == reference.id && x.revision == reference.revision)
}
fn bom<'a>(project: &'a ProjectState, reference: &SubjectRevisionRef) -> Option<&'a BomDefinition> {
    project
        .boms
        .iter()
        .find(|x| x.id == reference.id && x.revision == reference.revision)
}

fn pointer_part(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn quantity(
    issues: &mut Vec<Issue>,
    object: &ObjectRef,
    path: &str,
    value: &Quantity,
    unit: Option<&UnitId>,
) {
    if unit.is_some_and(|unit| *unit != value.unit) {
        problem(
            issues,
            object,
            format!("{path}/unit"),
            "unit_mismatch",
            "数量单位与精确引用的物品或需求单位不一致；未声明单位换算。",
        );
    }
    if value.unit.as_str() == "piece" && value.value.decimal_places.value() != 0 {
        problem(
            issues,
            object,
            format!("{path}/value/decimal_places"),
            "fractional_piece",
            "离散piece数量必须使用decimal_places=0的整数表示。",
        );
    }
}

// SafeUint <= 2^53-1 and DecimalPlaces <= 9: each scaled value fits in u128.
fn scaled(value: &Quantity) -> u128 {
    u128::from(value.value.coefficient.value())
        * 10_u128.pow(9 - u32::from(value.value.decimal_places.value()))
}

fn exact_definition<'a>(
    definitions: &'a [ModuleDefinition],
    reference: &ModuleRef,
) -> Option<&'a ModuleDefinition> {
    let mut found = definitions
        .iter()
        .filter(|d| d.id == reference.id && d.version == reference.version);
    let first = found.next()?;
    if found.next().is_some() {
        None
    } else {
        Some(first)
    }
}

fn selected_operation(
    device: &DeviceInstance,
    definitions: &[ModuleDefinition],
) -> Option<ModuleRef> {
    if let Some(reference) = &device.operation_binding {
        return exact_definition(definitions, reference)
            .filter(|d| matches!(d.payload, ModulePayload::Operations(_)))
            .map(|_| reference.clone());
    }
    let definition = exact_definition(definitions, &device.module_ref)?;
    let ModulePayload::Devices(data) = &definition.payload else {
        return None;
    };
    match data.behavior.selected {
        Some(BehaviorSourceKind::Inline) if data.behavior.inline.is_some() => {
            Some(device.module_ref.clone())
        }
        Some(BehaviorSourceKind::OperationRef) => {
            let reference = data.behavior.operation_ref.as_ref()?;
            exact_definition(definitions, reference)
                .filter(|d| matches!(d.payload, ModulePayload::Operations(_)))
                .map(|_| reference.clone())
        }
        _ => None,
    }
}

fn check_step_binding(
    value: &RouteDefinition,
    index: usize,
    step: &RouteStep,
    device: &DeviceInstance,
    definitions: Option<&[ModuleDefinition]>,
    issues: &mut Vec<Issue>,
) {
    let object = owner(ObjectKind::Route, value.id.as_str());
    let path = format!("/steps/{index}/operation_ref");
    let Some(definitions) = definitions else {
        if device
            .operation_binding
            .as_ref()
            .is_some_and(|b| !same_module(b, &step.operation_ref))
        {
            problem(
                issues,
                &object,
                path,
                "operation_definition_required",
                "路线引用不同于设备所选工序；缺精确模块定义，不能判断这是准备任务还是误绑定。",
            );
        }
        return;
    };
    let Some(definition) = exact_definition(definitions, &step.operation_ref) else {
        problem(
            issues,
            &object,
            path,
            "operation_definition_required",
            "路线步骤的精确模块定义缺失或身份不唯一；不按名称、近似版本或其他分支猜测。",
        );
        return;
    };
    match &definition.payload {
        ModulePayload::Tasks(task) => {
            if !matches!(
                task.kind,
                TaskKind::ManualPreparation | TaskKind::SupportedAutomatic
            ) || task.completion_effect.produces_material
            {
                problem(
                    issues,
                    &object,
                    path,
                    "unsupported_preparation_task",
                    "路线准备步骤须为已支持的准备任务，完成效果不得凭空制造物品。",
                );
                return;
            }
            let targets_device = |role: &RoleId| {
                device
                    .role_bindings
                    .targets
                    .get(role)
                    .is_some_and(|target| {
                        matches!(target.kind, ObjectKind::Device) && target.id == device.id
                    })
            };
            if task
                .target_role
                .as_ref()
                .is_none_or(|role| !targets_device(role))
                || !targets_device(&task.completion_effect.target_role)
                || task.steps.iter().any(|s| !targets_device(&s.target_role))
            {
                problem(
                    issues,
                    &object,
                    path.clone(),
                    "preparation_target_mismatch",
                    "准备任务及完成标记须显式指向本步骤的真实设备；不创建假设备或猜目标角色。",
                );
            }
            if selected_operation(device, definitions).is_none() {
                problem(
                    issues,
                    &object,
                    path,
                    "preparation_operation_required",
                    "准备任务不替代设备的正常工序；所选Operations或inline行为须有精确定义。",
                );
            }
            // EquipmentAccess stays explicit in TaskDefinition. The unique Engine
            // consumes Exclusive/Shared and its real capacity/resource claims.
        }
        ModulePayload::Operations(_) | ModulePayload::Devices(_) => {
            let selected = selected_operation(device, definitions)
                .as_ref()
                .is_some_and(|r| same_module(r, &step.operation_ref));
            let explicit_alternative = matches!(definition.payload, ModulePayload::Operations(_))
                && device
                    .route_operation_refs
                    .iter()
                    .filter(|reference| same_module(reference, &step.operation_ref))
                    .count()
                    == 1;
            if !selected && !explicit_alternative {
                problem(
                    issues,
                    &object,
                    path,
                    "operation_binding_mismatch",
                    "路线步骤须匹配设备默认/inline分支或显式列出的一项精确Operations版本；不因另有定义就切换行为。",
                );
            }
        }
        _ => problem(
            issues,
            &object,
            path,
            "route_module_kind",
            "路线只能消费明确的Operations、所选inline或支持的Tasks准备定义。",
        ),
    }
}

fn check_route(
    project: &ProjectState,
    value: &RouteDefinition,
    definitions: Option<&[ModuleDefinition]>,
    issues: &mut Vec<Issue>,
) {
    let object = owner(ObjectKind::Route, value.id.as_str());
    let mut indices = BTreeMap::new();
    for (i, step) in value.steps.iter().enumerate() {
        if indices.insert(&step.id, i).is_some() {
            problem(
                issues,
                &object,
                format!("/steps/{i}/id"),
                "duplicate_route_step",
                "同一路线内工序ID重复，不能猜测前置引用的对象。",
            );
        }
    }
    if value.steps.is_empty() {
        problem(
            issues,
            &object,
            "/steps",
            "route_without_steps",
            "路线尚无工序，不能作为已经实现的生产路线执行。",
        );
    }
    let mut degree: BTreeMap<&ObjectId, usize> = indices.keys().map(|id| (*id, 0)).collect();
    let mut successors: BTreeMap<&ObjectId, Vec<&ObjectId>> = BTreeMap::new();
    for (i, step) in value.steps.iter().enumerate() {
        let path = format!("/steps/{i}");
        if !project
            .module_refs
            .iter()
            .any(|m| same_module(m, &step.operation_ref))
        {
            problem(
                issues,
                &object,
                format!("{path}/operation_ref"),
                "missing_module",
                format!(
                    "缺少精确工序模块 {}@{} 的工程声明。",
                    step.operation_ref.id.as_str(),
                    String::from(step.operation_ref.version.clone())
                ),
            );
        }
        match project.devices.iter().find(|d| d.id == step.device_id) {
            None => problem(
                issues,
                &object,
                format!("{path}/device_id"),
                "missing_device",
                format!("工序设备 {} 尚未声明。", step.device_id.as_str()),
            ),
            Some(device) => {
                if !project
                    .module_refs
                    .iter()
                    .any(|m| same_module(m, &device.module_ref))
                {
                    problem(
                        issues,
                        &owner(ObjectKind::Device, device.id.as_str()),
                        "/module_ref",
                        "missing_device_module",
                        "所选设备的精确模块版本尚未在工程中声明。",
                    );
                }
                check_step_binding(value, i, step, device, definitions, issues);
            }
        }
        for (role, id) in &step.item_bindings {
            if !project.item_types.iter().any(|item| item.id == *id) {
                problem(
                    issues,
                    &object,
                    format!("{path}/item_bindings/{}", pointer_part(role.as_str())),
                    "missing_bound_item",
                    format!("物品角色引用 {} 尚未声明。", id.as_str()),
                );
            }
        }
        for (role, id) in &step.resource_bindings {
            if !project.resources.iter().any(|r| r.id == *id)
                && !project.devices.iter().any(|d| d.id == *id)
            {
                problem(
                    issues,
                    &object,
                    format!("{path}/resource_bindings/{}", pointer_part(role.as_str())),
                    "missing_bound_resource",
                    format!("资源角色引用 {} 尚未声明。", id.as_str()),
                );
            }
        }
        let mut seen = BTreeSet::new();
        for (j, predecessor) in step.predecessors.iter().enumerate() {
            if !indices.contains_key(predecessor) {
                problem(
                    issues,
                    &object,
                    format!("{path}/predecessors/{j}"),
                    "missing_predecessor",
                    format!("前置 {} 不属于当前路线。", predecessor.as_str()),
                );
            } else if seen.insert(predecessor) {
                *degree.get_mut(&step.id).expect("indexed step") += 1;
                successors.entry(predecessor).or_default().push(&step.id);
            }
        }
    }
    let mut ready: VecDeque<_> = degree
        .iter()
        .filter_map(|(id, n)| (*n == 0).then_some(*id))
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        if let Some(next) = successors.get(id) {
            for id in next {
                let remaining = degree.get_mut(id).expect("indexed successor");
                *remaining -= 1;
                if *remaining == 0 {
                    ready.push_back(id);
                }
            }
        }
    }
    if visited != degree.len() {
        problem(
            issues,
            &object,
            "/steps",
            "route_dependency_cycle",
            "路线工序前置存在环，无法建立生产先后关系。",
        );
    }
}

/// Return located, saveable-draft Issues. No input, quantity or runtime state is changed.
pub fn check_business_inputs(
    project: &ProjectState,
    scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
) -> Vec<Issue> {
    check_business_inputs_inner(project, scenarios, None)
}

/// Resolve preparation versus selected production behavior from exact loaded definitions.
pub fn check_business_inputs_with_definitions(
    project: &ProjectState,
    scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
    definitions: &[ModuleDefinition],
) -> Vec<Issue> {
    check_business_inputs_inner(project, scenarios, Some(definitions))
}

fn check_business_inputs_inner(
    project: &ProjectState,
    scenarios: &BTreeMap<ScenarioId, ScenarioInput>,
    definitions: Option<&[ModuleDefinition]>,
) -> Vec<Issue> {
    let mut issues = vec![];
    let project_ref = owner(ObjectKind::Project, project.id.as_str());
    for (i, id) in project.scenario_refs.iter().enumerate() {
        if !scenarios.contains_key(id) {
            problem(
                &mut issues,
                &project_ref,
                format!("/scenario_refs/{i}"),
                "missing_scenario",
                format!("缺少当前方案文档 {}。", id.as_str()),
            );
        }
    }
    for material in &project.item_types {
        let object = owner(ObjectKind::ItemType, material.id.as_str());
        if material
            .spec_ref
            .as_ref()
            .is_some_and(|reference| product(project, reference).is_none())
        {
            problem(
                &mut issues,
                &object,
                "/spec_ref",
                "missing_item_spec_product",
                "物品规格引用的精确产品版本不存在；保留该引用草稿，不选择最新版本。",
            );
        }
        for (key, value) in &material.attributes {
            if let TypedValue::ItemType(id) = value {
                if !project
                    .item_types
                    .iter()
                    .any(|candidate| candidate.id == *id)
                {
                    problem(
                        &mut issues,
                        &object,
                        format!("/attributes/{}/value", pointer_part(key)),
                        "missing_attribute_item",
                        format!("自定义属性引用的物品类型 {} 尚未声明。", id.as_str()),
                    );
                }
            }
        }
    }
    for p in &project.products {
        let object = owner(ObjectKind::Product, p.id.as_str());
        if item(project, &p.item_type_ref).is_none() {
            problem(
                &mut issues,
                &object,
                "/item_type_ref",
                "missing_product_item",
                "缺少产品引用的精确物品版本。",
            );
        }
        let b = bom(project, &p.bom_ref);
        if b.is_none() {
            problem(
                &mut issues,
                &object,
                "/bom_ref",
                "missing_product_bom",
                "缺少产品引用的精确BOM版本。",
            );
        }
        let r = route(project, &p.routing_ref);
        if r.is_none() {
            problem(
                &mut issues,
                &object,
                "/routing_ref",
                "missing_product_route",
                "缺少产品引用的精确路线版本。",
            );
        }
        if let (Some(b), Some(r)) = (b, r) {
            for (i, row) in b.rows.iter().enumerate() {
                if !r
                    .steps
                    .iter()
                    .any(|step| step.id == row.consume_at_operation)
                {
                    problem(
                        &mut issues,
                        &owner(ObjectKind::Bom, b.id.as_str()),
                        format!("/rows/{i}/consume_at_operation"),
                        "bom_operation_outside_product_route",
                        format!(
                            "消耗工序 {} 不属于产品 {} 采用的精确路线 {}@{}。",
                            row.consume_at_operation.as_str(),
                            p.id.as_str(),
                            r.id.as_str(),
                            r.revision.as_str()
                        ),
                    );
                }
            }
        }
        for (i, id) in p.artifact_refs.iter().enumerate() {
            if !project.artifact_refs.contains(id) {
                problem(
                    &mut issues,
                    &object,
                    format!("/artifact_refs/{i}"),
                    "missing_product_artifact",
                    "产品所需成果尚未在工程中声明；不将声明视为成果认可。",
                );
            }
        }
    }
    for b in &project.boms {
        let object = owner(ObjectKind::Bom, b.id.as_str());
        for (i, row) in b.rows.iter().enumerate() {
            let path = format!("/rows/{i}");
            let material = item(project, &row.item_type_ref);
            if material.is_none() {
                problem(
                    &mut issues,
                    &object,
                    format!("{path}/item_type_ref"),
                    "missing_bom_item",
                    "BOM项引用的精确物品版本不存在。",
                );
            }
            quantity(
                &mut issues,
                &object,
                &format!("{path}/quantity"),
                &row.quantity,
                material.map(|x| &x.unit),
            );
            if !project
                .routes
                .iter()
                .any(|r| r.steps.iter().any(|s| s.id == row.consume_at_operation))
            {
                problem(
                    &mut issues,
                    &object,
                    format!("{path}/consume_at_operation"),
                    "missing_consumption_operation",
                    "BOM消耗工序尚未在工程路线中声明。",
                );
            }
        }
    }
    for r in &project.routes {
        check_route(project, r, definitions, &mut issues);
    }
    for (scenario_id, scenario) in scenarios {
        issues.extend(crate::check_material_inputs(project, scenario));
        let object = owner(ObjectKind::Scenario, scenario_id.as_str());
        if scenario_id != &scenario.id {
            problem(
                &mut issues,
                &object,
                "/id",
                "scenario_identity_mismatch",
                "当前方案索引与方案文档身份不一致。",
            );
        }
        if !project.scenario_refs.contains(scenario_id) {
            problem(
                &mut issues,
                &object,
                "/id",
                "scenario_not_indexed",
                "当前方案文档未被工程scenario_refs引用。",
            );
        }
        let mut orders: BTreeMap<&ObjectId, Vec<&OrderLine>> = BTreeMap::new();
        for (i, order) in scenario.orders.iter().enumerate() {
            let path = format!("/orders/{i}");
            let entries = orders.entry(&order.id).or_default();
            if !entries.is_empty() {
                problem(
                    &mut issues,
                    &object,
                    format!("{path}/id"),
                    "duplicate_order",
                    "同一方案内订单ID重复，引用不明确。",
                );
            }
            entries.push(order);
            let p = product(project, &order.product_ref);
            if p.is_none() {
                problem(
                    &mut issues,
                    &object,
                    format!("{path}/product_ref"),
                    "missing_order_product",
                    "订单所引用的精确产品版本不存在；不自动选择最新版本。",
                );
            }
            match &order.quantity {
                None => problem(
                    &mut issues,
                    &object,
                    format!("{path}/quantity"),
                    "missing_order_quantity",
                    "订单未给数量，保留null草稿，不补成0。",
                ),
                Some(q) => quantity(
                    &mut issues,
                    &object,
                    &format!("{path}/quantity"),
                    q,
                    p.and_then(|p| item(project, &p.item_type_ref))
                        .map(|x| &x.unit),
                ),
            }
            if order.due_at_ms.is_none() {
                issues.push(Issue {
                    code: "business_inputs.order_due_unknown".to_owned(),
                    object_ref: Some(object.clone()),
                    field_path: Some(format!("{path}/due_at_ms")),
                    message: "订单未给交期，可彩排但不能给出该订单准时率结论。".to_owned(),
                    blocks: vec![CapabilityPurpose::CompleteConclusion],
                });
            }
            if !project
                .locations
                .iter()
                .any(|location| location.id == order.destination)
            {
                problem(
                    &mut issues,
                    &object,
                    format!("{path}/destination"),
                    "missing_order_destination",
                    "订单接受库位或外部接收节点尚未在locations中声明。",
                );
            }
        }
        let mut task_ids = BTreeSet::new();
        let mut order_remaining: BTreeMap<&ObjectId, u128> = BTreeMap::new();
        for (i, task) in scenario.production_tasks.iter().enumerate() {
            let path = format!("/production_tasks/{i}");
            if !task_ids.insert(&task.id) {
                problem(
                    &mut issues,
                    &object,
                    format!("{path}/id"),
                    "duplicate_production_task",
                    "同一方案内生产任务ID重复。",
                );
            }
            if route(project, &task.routing_ref).is_none() {
                problem(
                    &mut issues,
                    &object,
                    format!("{path}/routing_ref"),
                    "missing_task_route",
                    "生产任务的精确路线版本不存在。",
                );
            }
            quantity(
                &mut issues,
                &object,
                &format!("{path}/planned_quantity"),
                &task.planned_quantity,
                None,
            );
            let mut planned_remaining = scaled(&task.planned_quantity);
            let mut task_product: Option<&SubjectRevisionRef> = None;
            for (j, allocation) in task.order_allocations.iter().enumerate() {
                let allocation_path = format!("{path}/order_allocations/{j}");
                quantity(
                    &mut issues,
                    &object,
                    &format!("{allocation_path}/quantity"),
                    &allocation.quantity,
                    Some(&task.planned_quantity.unit),
                );
                if allocation.quantity.unit == task.planned_quantity.unit {
                    let amount = scaled(&allocation.quantity);
                    if amount > planned_remaining {
                        problem(
                            &mut issues,
                            &object,
                            format!("{allocation_path}/quantity"),
                            "allocation_exceeds_task",
                            "订单分配合计超过生产任务明确给出的计划量。",
                        );
                    }
                    planned_remaining = planned_remaining.saturating_sub(amount);
                }
                let Some(matches) = orders.get(&allocation.order_id) else {
                    problem(
                        &mut issues,
                        &object,
                        format!("{allocation_path}/order_id"),
                        "missing_allocated_order",
                        "生产任务引用的订单不在当前方案中。",
                    );
                    continue;
                };
                if matches.len() != 1 {
                    problem(
                        &mut issues,
                        &object,
                        format!("{allocation_path}/order_id"),
                        "ambiguous_allocated_order",
                        "生产任务的订单引用对应多个同ID订单，不能任选一个。",
                    );
                    continue;
                }
                let order = matches[0];
                if let Some(expected) = task_product {
                    if !same_subject(expected, &order.product_ref) {
                        problem(
                            &mut issues,
                            &object,
                            format!("{allocation_path}/order_id"),
                            "task_product_mismatch",
                            "同一生产任务只能合并同一精确产品ID及版本的订单；共用路线不能代替产品身份，其他产品须独立任务。",
                        );
                    }
                } else {
                    task_product = Some(&order.product_ref);
                }
                if product(project, &order.product_ref)
                    .is_some_and(|p| !same_subject(&p.routing_ref, &task.routing_ref))
                {
                    problem(
                        &mut issues,
                        &object,
                        format!("{allocation_path}/order_id"),
                        "task_order_route_mismatch",
                        "生产任务所用路线版本与分配订单产品的认可路线不同。",
                    );
                }
                if let Some(ordered) = &order.quantity {
                    quantity(
                        &mut issues,
                        &object,
                        &format!("{allocation_path}/quantity"),
                        &allocation.quantity,
                        Some(&ordered.unit),
                    );
                    if allocation.quantity.unit == ordered.unit {
                        let remaining = order_remaining
                            .entry(&order.id)
                            .or_insert_with(|| scaled(ordered));
                        let amount = scaled(&allocation.quantity);
                        if amount > *remaining {
                            problem(
                                &mut issues,
                                &object,
                                format!("{allocation_path}/quantity"),
                                "allocation_exceeds_order",
                                "同一方案的生产任务分配合计超过该订单明确数量；不复制需求。",
                            );
                        }
                        *remaining = remaining.saturating_sub(amount);
                    }
                }
            }
        }
    }
    issues
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    pub(crate) fn fixture() -> (ProjectState, BTreeMap<ScenarioId, ScenarioInput>) {
        let pose = json!({ "translation": {"x":0,"y":0,"z":0}, "rotation":{"x":0,"y":0,"z":0,"w":1}, "scale":{"x":1,"y":1,"z":1}, "unit":"meter", "coordinate_frame":"right_handed_z_up" });
        let camera = json!({"coordinate_frame":"right_handed_z_up","unit":"meter","eye":{"x":10,"y":-10,"z":10},"target":{"x":0,"y":0,"z":0},"zoom":1});
        let time = json!({"label":"business fixture","origin_iso8601":null,"timezone":null});
        let q = json!({"value":{"coefficient":2,"decimal_places":0},"unit":"piece"});
        let project: ProjectState = serde_json::from_value(json!({
            "id":"project","revision":1,"title":"Pure business inputs","time_reference":time,
            "setup_policies":[],"item_types":[{"id":"item","revision":"r1","title":"item","unit":"piece","tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]}],
            "locations":[{"id":"customer","title":"customer","hall_id":null,"position":pose,"capacity":null,"accepted_item_types":["item"],"purpose":"external_destination","ordinary_issue_enabled":false}],
            "halls":[],"lines":[],"devices":[{"id":"device","title":"device","hall_id":"hall","line_id":null,"module_ref":{"id":"device-module","version":"1.0.0"},"pose":pose,"footprint_m":[],"height_m":null,"configuration":{},"role_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"presentation_override":null,"operation_binding":{"id":"operation","version":"1.0.0"}}],
            "resources":[],"calendars":[],
            "products":[{"id":"product","revision":"r1","item_type_ref":{"id":"item","revision":"r1"},"bom_ref":{"id":"bom","revision":"r1"},"routing_ref":{"id":"route","revision":"r1"},"artifact_refs":[]}],
            "boms":[{"id":"bom","revision":"r1","rows":[{"item_type_ref":{"id":"item","revision":"r1"},"quantity":q,"consume_at_operation":"step"}]}],
            "routes":[{"id":"route","revision":"r1","steps":[{"id":"step","operation_ref":{"id":"operation","version":"1.0.0"},"device_id":"device","predecessors":[],"item_bindings":{},"resource_bindings":{}}]}],
            "connections":[],"module_refs":[{"id":"device-module","version":"1.0.0"},{"id":"operation","version":"1.0.0"}],"scenario_refs":["scenario"],"artifact_refs":[],"acknowledgments":[],"bindings":[],
            "views":{"normal_canvas":"line_canvas","lod":"detailed","selected_hall":null,"selected_objects":[],"line_camera":camera,"multi_hall_camera":camera},"generation_context":null,"sources":[],"issues":[],"template_adoptions":[]
        })).expect("formal ProjectState business fixture");
        let scenario: ScenarioInput = serde_json::from_value(json!({
            "id":"scenario","revision":1,"project_revision":1,"title":"orders","time_reference":time,"window":{"start_ms":0,"end_ms":1000},
            "initial_conditions":{"items":[],"initial_setup_families":{},"declared_quality_decisions":[],"material_allocations":[]},
            "orders":[{"id":"order","product_ref":{"id":"product","revision":"r1"},"quantity":q,"release_at_ms":0,"due_at_ms":1000,"destination":"customer","acceptance_stage":"received","allow_partial":true,"priority":1}],
            "production_tasks":[{"id":"production","routing_ref":{"id":"route","revision":"r1"},"order_allocations":[{"order_id":"order","quantity":q}],"planned_quantity":q,"release_at_ms":0,"priority":1}],
            "quality_presets":[],"arrivals":[],"supplies":[],"parameter_overrides":[],"scope":[],"issues":[],"cost_inputs":[]
        })).expect("formal ScenarioInput business fixture");
        (project, BTreeMap::from([(scenario.id.clone(), scenario)]))
    }

    fn has<'a>(issues: &'a [Issue], code: &str) -> &'a Issue {
        issues
            .iter()
            .find(|issue| issue.code == format!("business_inputs.{code}"))
            .unwrap_or_else(|| panic!("missing {code}: {issues:?}"))
    }

    #[test]
    fn complete_exact_business_references_have_no_input_issues() {
        let (project, scenarios) = fixture();
        assert!(check_business_inputs(&project, &scenarios).is_empty());
    }

    #[test]
    fn old_subject_revision_is_not_silently_upgraded() {
        let (mut project, scenarios) = fixture();
        project.item_types[0].spec_ref = Some(SubjectRevisionRef {
            id: project.products[0].id.clone(),
            revision: project.products[0].revision.clone(),
        });
        let material_id = project.item_types[0].id.clone();
        project.item_types[0]
            .attributes
            .insert("type/~".to_owned(), TypedValue::ItemType(material_id));
        assert!(check_business_inputs(&project, &scenarios).is_empty());
        project.products[0].revision = SubjectRevisionId::try_from("r2".to_owned()).unwrap();
        project.item_types[0].attributes.insert(
            "type/~".to_owned(),
            TypedValue::ItemType(ObjectId::try_from("missing-attribute-item".to_owned()).unwrap()),
        );
        let before = serde_json::to_value((&project, &scenarios)).unwrap();
        let issues = check_business_inputs(&project, &scenarios);
        let spec = has(&issues, "missing_item_spec_product");
        assert_eq!(spec.object_ref.as_ref().unwrap().id.as_str(), "item");
        assert_eq!(spec.field_path.as_deref(), Some("/spec_ref"));
        let attribute = has(&issues, "missing_attribute_item");
        assert_eq!(attribute.object_ref.as_ref().unwrap().id.as_str(), "item");
        assert_eq!(
            attribute.field_path.as_deref(),
            Some("/attributes/type~1~0/value")
        );
        assert_eq!(
            serde_json::to_value((&project, &scenarios)).unwrap(),
            before
        );
        let issue = has(&issues, "missing_order_product");
        assert_eq!(issue.object_ref.as_ref().unwrap().id.as_str(), "scenario");
        assert_eq!(issue.field_path.as_deref(), Some("/orders/0/product_ref"));
        assert_eq!(
            scenarios.values().next().unwrap().orders[0]
                .product_ref
                .revision
                .as_str(),
            "r1"
        );
    }

    #[test]
    fn missing_quantity_due_and_legacy_orders_preserve_input() {
        let (project, mut scenarios) = fixture();
        let scenario = scenarios.values_mut().next().unwrap();
        scenario.orders[0].quantity = None;
        scenario.orders[0].due_at_ms = None;
        let before = serde_json::to_value((&project, &scenarios)).unwrap();
        let issues = check_business_inputs(&project, &scenarios);
        has(&issues, "missing_order_quantity");
        let due = has(&issues, "order_due_unknown");
        assert_eq!(due.blocks.len(), 1);
        assert!(matches!(
            due.blocks[0],
            CapabilityPurpose::CompleteConclusion
        ));
        assert_eq!(
            serde_json::to_value((&project, &scenarios)).unwrap(),
            before
        );
        let scenario = scenarios.values_mut().next().unwrap();
        scenario.orders.clear();
        scenario.production_tasks.clear();
        assert!(check_business_inputs(&project, &scenarios).is_empty());
    }

    #[test]
    fn units_and_piece_integrality_are_explicit() {
        let (mut project, mut scenarios) = fixture();
        project.boms[0].rows[0].quantity.unit = UnitId::try_from("kg".to_owned()).unwrap();
        scenarios.values_mut().next().unwrap().orders[0]
            .quantity
            .as_mut()
            .unwrap()
            .value
            .decimal_places = DecimalPlaces::try_from(1).unwrap();
        let issues = check_business_inputs(&project, &scenarios);
        assert_eq!(
            has(&issues, "unit_mismatch").field_path.as_deref(),
            Some("/rows/0/quantity/unit")
        );
        assert_eq!(
            has(&issues, "fractional_piece").field_path.as_deref(),
            Some("/orders/0/quantity/value/decimal_places")
        );
    }

    #[test]
    fn bom_consumption_must_belong_to_the_selected_product_route() {
        let (mut project, scenarios) = fixture();
        let mut other = project.routes[0].clone();
        other.id = ObjectId::try_from("other-route".to_owned()).unwrap();
        other.steps[0].id = ObjectId::try_from("other-step".to_owned()).unwrap();
        project.boms[0].rows[0].consume_at_operation = other.steps[0].id.clone();
        project.routes.push(other);
        let issues = check_business_inputs(&project, &scenarios);
        let issue = has(&issues, "bom_operation_outside_product_route");
        assert_eq!(issue.object_ref.as_ref().unwrap().id.as_str(), "bom");
        assert_eq!(
            issue.field_path.as_deref(),
            Some("/rows/0/consume_at_operation")
        );
    }

    #[test]
    fn route_gaps_cycles_and_escaped_binding_pointers_are_located() {
        let (mut project, scenarios) = fixture();
        project.module_refs.clear();
        let step = &mut project.routes[0].steps[0];
        step.predecessors = vec![
            step.id.clone(),
            ObjectId::try_from("outside".to_owned()).unwrap(),
        ];
        step.device_id = ObjectId::try_from("missing-device".to_owned()).unwrap();
        step.item_bindings.insert(
            RoleId::try_from("/~role".to_owned()).unwrap(),
            ObjectId::try_from("missing-item".to_owned()).unwrap(),
        );
        step.resource_bindings.insert(
            RoleId::try_from("person".to_owned()).unwrap(),
            ObjectId::try_from("missing-person".to_owned()).unwrap(),
        );
        let issues = check_business_inputs(&project, &scenarios);
        assert_eq!(
            has(&issues, "missing_module").field_path.as_deref(),
            Some("/steps/0/operation_ref")
        );
        has(&issues, "missing_device");
        has(&issues, "missing_predecessor");
        has(&issues, "route_dependency_cycle");
        has(&issues, "missing_bound_resource");
        assert_eq!(
            has(&issues, "missing_bound_item").field_path.as_deref(),
            Some("/steps/0/item_bindings/~1~0role")
        );

        let (mut project, scenarios) = fixture();
        let make_definition = |name: &str, payload| ModuleDefinition {
            id: ModuleId::try_from(name.to_owned()).unwrap(),
            version: VersionId::try_from("1.0.0".to_owned()).unwrap(),
            title: name.to_owned(),
            definition_state: DefinitionState::RuntimeCandidate,
            contribution: Contribution {
                author: None,
                license: None,
                assets: vec![],
            },
            parameters: vec![],
            attribute_schema: vec![],
            payload,
        };
        let operation: OperationDefinition = serde_json::from_value(json!({
            "builtin":"transform","item_roles":[],"inputs":[],"outputs":[],"resource_roles":[],
            "duration":{"value":1,"unit":"millisecond","basis":"user_provided","source_ref":null},
            "phases":[],"hold_on_output_wait":[],"quality_and_routes":null,"identity_policy":"preserve","pack_identity":null,"family":null
        })).unwrap();
        let preparation: TaskDefinition = serde_json::from_value(json!({
            "kind":"manual_preparation","target_role":"target","prerequisites":[],"steps":[],"resource_roles":[],
            "duration":{"value":1,"unit":"millisecond","basis":"user_provided","source_ref":null},
            "equipment_access":"exclusive","completion_effect":{"target_role":"target","marker":"ready","produces_material":false}
        })).unwrap();
        let definitions = vec![
            make_definition("operation", ModulePayload::Operations(operation.clone())),
            make_definition("preparation", ModulePayload::Tasks(preparation)),
        ];
        let preparation_ref = ModuleRef {
            id: definitions[1].id.clone(),
            version: definitions[1].version.clone(),
        };
        project.module_refs.push(preparation_ref.clone());
        let mut step = project.routes[0].steps[0].clone();
        step.id = ObjectId::try_from("prepare".to_owned()).unwrap();
        step.operation_ref = preparation_ref;
        project.routes[0].steps[0]
            .predecessors
            .push(step.id.clone());
        project.routes[0].steps.push(step);
        project.devices[0].role_bindings.targets.insert(
            RoleId::try_from("target".to_owned()).unwrap(),
            owner(ObjectKind::Device, "device"),
        );
        has(
            &check_business_inputs(&project, &scenarios),
            "operation_definition_required",
        );
        let before = serde_json::to_value((&project, &scenarios, &definitions)).unwrap();
        assert!(
            check_business_inputs_with_definitions(&project, &scenarios, &definitions).is_empty()
        );
        assert_eq!(
            serde_json::to_value((&project, &scenarios, &definitions)).unwrap(),
            before
        );

        let mut invalid = project.clone();
        invalid.devices[0].role_bindings.targets.clear();
        has(
            &check_business_inputs_with_definitions(&invalid, &scenarios, &definitions),
            "preparation_target_mismatch",
        );
        let mut invalid_definitions = definitions.clone();
        if let ModulePayload::Tasks(task) = &mut invalid_definitions[1].payload {
            task.completion_effect.produces_material = true;
        }
        has(
            &check_business_inputs_with_definitions(&project, &scenarios, &invalid_definitions),
            "unsupported_preparation_task",
        );
        let mut invalid = project.clone();
        invalid.routes[0].steps[1].operation_ref.version =
            VersionId::try_from("2.0.0".to_owned()).unwrap();
        has(
            &check_business_inputs_with_definitions(&invalid, &scenarios, &definitions),
            "operation_definition_required",
        );
        let mut other = make_definition(
            "other-operation",
            ModulePayload::Operations(operation.clone()),
        );
        let mut invalid = project.clone();
        invalid.routes[0].steps[1].operation_ref.id = other.id.clone();
        let mut invalid_definitions = definitions.clone();
        invalid_definitions.push(other.clone());
        has(
            &check_business_inputs_with_definitions(&invalid, &scenarios, &invalid_definitions),
            "operation_binding_mismatch",
        );
        let alternate = invalid.routes[0].steps[1].operation_ref.clone();
        invalid.module_refs.push(alternate.clone());
        invalid.devices[0].route_operation_refs = vec![alternate];
        let alternate_issues =
            check_business_inputs_with_definitions(&invalid, &scenarios, &invalid_definitions);
        assert!(alternate_issues.is_empty(), "{alternate_issues:?}");
        invalid.devices[0].route_operation_refs[0].version =
            VersionId::try_from("2.0.0".to_owned()).unwrap();
        has(
            &check_business_inputs_with_definitions(&invalid, &scenarios, &invalid_definitions),
            "operation_binding_mismatch",
        );
        let mut duplicate = definitions.clone();
        duplicate.push(definitions[1].clone());
        has(
            &check_business_inputs_with_definitions(&project, &scenarios, &duplicate),
            "operation_definition_required",
        );

        let (mut inline_project, inline_scenarios) = fixture();
        inline_project.devices[0].operation_binding = None;
        inline_project.routes[0].steps[0].operation_ref =
            inline_project.devices[0].module_ref.clone();
        let inline: DeviceDefinition = serde_json::from_value(json!({
            "placement":"fixed_during_simulation","ports":[],"behavior":{"inline":operation,"operation_ref":null,"selected":"inline"},
            "configuration":{},"presentation":null,"operations":[]
        })).unwrap();
        other = make_definition("device-module", ModulePayload::Devices(inline));
        assert!(
            check_business_inputs_with_definitions(
                &inline_project,
                &inline_scenarios,
                &[other.clone()]
            )
            .is_empty()
        );
        if let ModulePayload::Devices(device) = &mut other.payload {
            device.behavior.selected = None;
        }
        has(
            &check_business_inputs_with_definitions(&inline_project, &inline_scenarios, &[other]),
            "operation_binding_mismatch",
        );
    }

    #[test]
    fn task_allocation_references_and_demand_limits_are_checked() {
        let (project, mut scenarios) = fixture();
        let scenario = scenarios.values_mut().next().unwrap();
        let mut extra = scenario.production_tasks[0].clone();
        extra.id = ObjectId::try_from("extra-task".to_owned()).unwrap();
        extra.routing_ref.revision = SubjectRevisionId::try_from("r2".to_owned()).unwrap();
        scenario.production_tasks.push(extra);
        let issues = check_business_inputs(&project, &scenarios);
        has(&issues, "missing_task_route");
        has(&issues, "task_order_route_mismatch");
        has(&issues, "allocation_exceeds_order");
        scenarios.values_mut().next().unwrap().production_tasks[0].order_allocations[0].order_id =
            ObjectId::try_from("not-this-scenario".to_owned()).unwrap();
        has(
            &check_business_inputs(&project, &scenarios),
            "missing_allocated_order",
        );

        let (base_project, mut shared) = fixture();
        let scenario = shared.values_mut().next().unwrap();
        let mut second = scenario.orders[0].clone();
        second.id = ObjectId::try_from("second-order".to_owned()).unwrap();
        let one_quantity = scenario.production_tasks[0].planned_quantity.clone();
        scenario.production_tasks[0]
            .order_allocations
            .push(OrderAllocation {
                order_id: second.id.clone(),
                quantity: one_quantity.clone(),
            });
        scenario.production_tasks[0]
            .planned_quantity
            .value
            .coefficient = SafeUint::try_from(one_quantity.value.coefficient.value() * 2).unwrap();
        scenario.orders.push(second);
        assert!(check_business_inputs(&base_project, &shared).is_empty());

        let mut other_product = base_project.products[0].clone();
        other_product.id = ObjectId::try_from("other-product".to_owned()).unwrap();
        let mut other_version = base_project.products[0].clone();
        other_version.revision = SubjectRevisionId::try_from("r2".to_owned()).unwrap();
        for product in [other_product, other_version] {
            let mut project = base_project.clone();
            let mut scenarios = shared.clone();
            scenarios.values_mut().next().unwrap().orders[1].product_ref = SubjectRevisionRef {
                id: product.id.clone(),
                revision: product.revision.clone(),
            };
            project.products.push(product);
            let before = serde_json::to_value((&project, &scenarios)).unwrap();
            let issues = check_business_inputs(&project, &scenarios);
            assert_eq!(
                has(&issues, "task_product_mismatch").field_path.as_deref(),
                Some("/production_tasks/0/order_allocations/1/order_id")
            );
            assert_eq!(
                serde_json::to_value((&project, &scenarios)).unwrap(),
                before
            );

            let scenario = scenarios.values_mut().next().unwrap();
            let mut separate = scenario.production_tasks[0].clone();
            separate.id = ObjectId::try_from("separate-task".to_owned()).unwrap();
            separate.order_allocations = vec![separate.order_allocations[1].clone()];
            separate.planned_quantity = one_quantity.clone();
            scenario.production_tasks[0].order_allocations.truncate(1);
            scenario.production_tasks[0].planned_quantity = one_quantity.clone();
            scenario.production_tasks.push(separate);
            assert!(check_business_inputs(&project, &scenarios).is_empty());
        }
    }

    #[test]
    fn decimal_allocation_overrun_is_exact_below_float_resolution() {
        let (mut project, mut scenarios) = fixture();
        let unit = UnitId::try_from("kg".to_owned()).unwrap();
        project.item_types[0].unit = unit.clone();
        project.boms[0].rows[0].quantity.unit = unit.clone();
        let q = |coefficient| Quantity {
            value: DecimalAmount {
                coefficient: SafeUint::try_from(coefficient).unwrap(),
                decimal_places: DecimalPlaces::try_from(9).unwrap(),
            },
            unit: unit.clone(),
        };
        let scenario = scenarios.values_mut().next().unwrap();
        scenario.orders[0].quantity = Some(q(MAX_SAFE_UINT));
        let task = &mut scenario.production_tasks[0];
        task.planned_quantity = q(MAX_SAFE_UINT);
        task.order_allocations[0].quantity = q(MAX_SAFE_UINT - 1);
        task.order_allocations.push(OrderAllocation {
            order_id: ObjectId::try_from("order".to_owned()).unwrap(),
            quantity: q(2),
        });
        let issues = check_business_inputs(&project, &scenarios);
        has(&issues, "allocation_exceeds_task");
        has(&issues, "allocation_exceeds_order");
    }

    #[test]
    fn scenario_index_and_document_identity_do_not_silently_repair() {
        let (mut project, mut scenarios) = fixture();
        scenarios.values_mut().next().unwrap().id =
            ScenarioId::try_from("different".to_owned()).unwrap();
        project
            .scenario_refs
            .push(ScenarioId::try_from("missing".to_owned()).unwrap());
        let issues = check_business_inputs(&project, &scenarios);
        has(&issues, "scenario_identity_mismatch");
        has(&issues, "missing_scenario");
        assert_eq!(scenarios.values().next().unwrap().id.as_str(), "different");
    }
}
