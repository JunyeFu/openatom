use crate::EngineInput;
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn id(text: impl Into<String>) -> ObjectId {
    ObjectId::try_from(text.into()).expect("nonempty generated identity")
}
pub(crate) fn stable(tag: &str, parts: &[&str]) -> ObjectId {
    id(format!(
        "{tag}:{}",
        parts
            .iter()
            .map(|p| format!("{}:{p}", p.len()))
            .collect::<String>()
    ))
}
pub(crate) fn uint(n: u64) -> SafeUint {
    SafeUint::try_from(n).expect("admitted safe integer")
}
pub(crate) fn time(n: u64) -> SimTimeMs {
    SimTimeMs(uint(n))
}
pub(crate) fn duration(n: u64) -> DurationMs {
    DurationMs(uint(n))
}
pub(crate) fn amount(q: &Quantity) -> u64 {
    q.value.coefficient.value()
}
pub(crate) fn quantity(n: u64, unit: &UnitId) -> Quantity {
    Quantity {
        value: DecimalAmount {
            coefficient: uint(n),
            decimal_places: DecimalPlaces::try_from(0).unwrap(),
        },
        unit: unit.clone(),
    }
}
pub(crate) fn same_module(a: &ModuleRef, b: &ModuleRef) -> bool {
    a.id == b.id && a.version == b.version
}
pub(crate) fn same_type(a: &SubjectRevisionRef, b: &SubjectRevisionRef) -> bool {
    a.id == b.id && a.revision == b.revision
}
pub(crate) fn issue(
    object: ObjectRef,
    path: impl Into<String>,
    code: &str,
    message: impl Into<String>,
) -> Issue {
    Issue {
        code: format!("simulation.{code}"),
        object_ref: Some(object),
        field_path: Some(path.into()),
        message: message.into(),
        blocks: vec![
            CapabilityPurpose::Execution,
            CapabilityPurpose::CompleteConclusion,
        ],
    }
}
pub(crate) fn object(kind: ObjectKind, value: &ObjectId) -> ObjectRef {
    ObjectRef {
        kind,
        id: value.clone(),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Material {
    pub item_type: SubjectRevisionRef,
    pub amount: u64,
    pub decimal_places: u8,
    pub unit: UnitId,
    pub port: PortId,
    pub role: RoleId,
    pub location: ObjectId,
    pub qualities: Vec<QualityState>,
    pub output_quality: Option<QualityState>,
    pub individual: bool,
    pub capacity: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Phase {
    pub id: PhaseId,
    pub work: u64,
    pub resources: BTreeMap<ObjectId, u64>,
    pub pause_resume: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Preparation {
    pub target: ObjectId,
    pub marker: String,
    pub prerequisites: Vec<(ObjectId, String)>,
}
#[derive(Debug, Clone)]
pub(crate) struct Job {
    pub id: ObjectId,
    pub task: ObjectId,
    pub operation_ref: ModuleRef,
    pub execution: JobExecutionRef,
    pub operation: Option<OperationDefinition>,
    pub family: Option<crate::engine::setup::Policy>,
    pub setup: Option<crate::engine::setup::Change>,
    pub transport: Option<crate::engine::transport::Plan>,
    pub inputs: Vec<Material>,
    pub outputs: Vec<Material>,
    pub job_resources: BTreeMap<ObjectId, u64>,
    pub output_holds: BTreeMap<ObjectId, u64>,
    pub phases: Vec<Phase>,
    pub predecessors: Vec<ObjectId>,
    pub priority: i32,
    pub due: Option<u64>,
    pub release: u64,
    pub preparation: Option<Preparation>,
    pub cycle: u64,
    pub quality: Option<crate::quality::Policy>,
}

impl Job {
    pub(crate) fn op(&self) -> &OperationDefinition {
        self.operation
            .as_ref()
            .expect("production-only operation access")
    }
}

pub(crate) fn discrete(
    q: &Quantity,
    owner: &ObjectRef,
    path: &str,
    issues: &mut Vec<Issue>,
) -> bool {
    if q.value.decimal_places.value() != 0 || amount(q) == 0 {
        issues.push(issue(
            owner.clone(),
            path,
            "discrete_quantity",
            "Discrete production requires an explicit positive integer quantity and unit.",
        ));
        false
    } else {
        true
    }
}
pub(crate) fn module<'a>(
    input: &'a EngineInput,
    reference: &ModuleRef,
) -> Option<&'a ModuleDefinition> {
    let mut found = input
        .definitions
        .iter()
        .filter(|d| d.id == reference.id && d.version == reference.version);
    let first = found.next()?;
    if found.next().is_some() {
        None
    } else {
        Some(first)
    }
}
pub(crate) fn resource_requirements(
    input: &EngineInput,
    device: &DeviceInstance,
    step: &RouteStep,
    requirements: &[ResourceRequirement],
    owner: &ObjectRef,
    issues: &mut Vec<Issue>,
) -> BTreeMap<ObjectId, u64> {
    let mut result = BTreeMap::new();
    for requirement in requirements {
        let path = format!("/resource_bindings/{}", requirement.role.as_str());
        let Some(resource_id) = step
            .resource_bindings
            .get(&requirement.role)
            .or_else(|| device.role_bindings.resources.get(&requirement.role))
        else {
            issues.push(issue(
                owner.clone(),
                path,
                "resource_binding",
                "Required resource role has no explicit binding.",
            ));
            continue;
        };
        let Some(resource) = input
            .project
            .resources
            .iter()
            .find(|r| r.id == *resource_id)
        else {
            issues.push(issue(
                owner.clone(),
                path,
                "resource_missing",
                "Bound resource does not exist.",
            ));
            continue;
        };
        if matches!(resource.kind, ResourceKind::ServiceDeclaration) {
            issues.push(issue(owner.clone(),path,"service_execution_deferred","A service declaration may be saved, but its execution solver is deferred; it cannot stand in for an implemented worker, fixture or carrier resource."));
            continue;
        }
        if !discrete(&requirement.quantity, owner, &path, issues) {
            continue;
        }
        if requirement.quantity.unit != resource.capacity.unit
            || requirement
                .qualifications
                .iter()
                .any(|q| !resource.qualifications.contains(q))
            || amount(&requirement.quantity) > amount(&resource.capacity)
        {
            issues.push(issue(
                owner.clone(),
                path,
                "resource_qualification",
                "Resource unit, qualification or declared capacity does not meet the requirement.",
            ));
            continue;
        }
        result
            .entry(resource_id.clone())
            .and_modify(|n: &mut u64| *n = (*n).max(amount(&requirement.quantity)))
            .or_insert(amount(&requirement.quantity));
    }
    result
}
fn operation_allowed(input: &EngineInput, device: &DeviceInstance, reference: &ModuleRef) -> bool {
    if device
        .route_operation_refs
        .iter()
        .any(|m| same_module(m, reference))
    {
        return true;
    }
    if let Some(primary) = &device.operation_binding {
        return same_module(primary, reference);
    }
    module(input,&device.module_ref).is_some_and(|d|matches!(&d.payload,ModulePayload::Devices(data) if matches!(data.behavior.selected,Some(BehaviorSourceKind::OperationRef)) && data.behavior.operation_ref.as_ref().is_some_and(|m|same_module(m,reference))))
}

fn endpoint_matches(endpoint: &MaterialEndpoint, device: &DeviceInstance, port: &PortId) -> bool {
    matches!(endpoint,MaterialEndpoint::Port(p) if p.instance_id==device.id && same_module(&p.module_ref,&device.module_ref) && p.port_id==*port)
}
fn material(
    input: &EngineInput,
    device: &DeviceInstance,
    step: &RouteStep,
    row: &MaterialRoleQuantity,
    output: bool,
    owner: &ObjectRef,
    issues: &mut Vec<Issue>,
) -> Option<Material> {
    let path = format!(
        "/{}/{}/{}",
        if output { "outputs" } else { "inputs" },
        row.role.as_str(),
        row.port.as_str()
    );
    let fractional_auxiliary = !output && input.definitions.iter().any(|definition| {
        same_module(&step.operation_ref, &ModuleRef { id: definition.id.clone(), version: definition.version.clone() })
        && matches!(&definition.payload, ModulePayload::Operations(op) if op.material_flow.as_ref().is_some_and(|flow| flow.auxiliary_roles.contains(&row.role)))
    });
    if !fractional_auxiliary && !discrete(&row.quantity, owner, &path, issues) {
        return None;
    }
    let type_id = step
        .item_bindings
        .get(&row.role)
        .or_else(|| device.role_bindings.item_types.get(&row.role));
    let Some(item_type) =
        type_id.and_then(|id| input.project.item_types.iter().find(|t| t.id == *id))
    else {
        issues.push(issue(
            owner.clone(),
            path,
            "material_binding",
            "Required material role has no exact declared item type.",
        ));
        return None;
    };
    if item_type.unit != row.quantity.unit {
        issues.push(issue(
            owner.clone(),
            path,
            "material_unit",
            "Operation quantity and bound item type use different units.",
        ));
        return None;
    }
    let Some(definition) = module(input, &device.module_ref) else {
        issues.push(issue(
            owner.clone(),
            path,
            "device_definition",
            "Exact device definition is absent or ambiguous.",
        ));
        return None;
    };
    let ModulePayload::Devices(d) = &definition.payload else {
        return None;
    };
    if !d.ports.iter().any(|p|p.id==row.port && matches!((&p.direction,output),(Direction::Input,false)|(Direction::Output,true)|(Direction::Bidirectional,_)) && matches!(&p.payload,PortPayload::Material(m) if m.item_role==row.role && m.unit==row.quantity.unit)) {
        issues.push(issue(owner.clone(),path,"material_port","Operation port, direction, role and unit must match the exact device definition.")); return None;
    }
    let connections: Vec<_> = input
        .project
        .connections
        .iter()
        .filter_map(|c| match c {
            Connection::Material(c)
                if c.item_type == item_type.id
                    && c.unit == item_type.unit
                    && endpoint_matches(
                        if output { &c.source } else { &c.destination },
                        device,
                        &row.port,
                    ) =>
            {
                Some(c)
            }
            _ => None,
        })
        .collect();
    if connections.len() != 1 {
        issues.push(issue(owner.clone(),path,"material_location","Exactly one explicit material connection and receiving store are required; ports are not hidden stock."));
        return None;
    }
    let connection = connections[0];
    for (endpoint, source) in [(&connection.source, true), (&connection.destination, false)] {
        if let MaterialEndpoint::Port(port) = endpoint {
            let bound =
                input.project.devices.iter().find(|d| {
                    d.id == port.instance_id && same_module(&d.module_ref, &port.module_ref)
                });
            let valid=bound.and_then(|bound|module(input,&bound.module_ref)).is_some_and(|definition|match &definition.payload {ModulePayload::Devices(d)=>d.ports.iter().any(|p|p.id==port.port_id&&matches!((&p.direction,source),(Direction::Output,true)|(Direction::Input,false)|(Direction::Bidirectional,_))&&matches!(&p.payload,PortPayload::Material(m)if m.unit==connection.unit)),_=>false});
            if !valid {
                issues.push(issue(object(ObjectKind::Connection,&connection.id),if source{"/source"}else{"/destination"},"connection_port","Both material endpoints must resolve to exact existing device modules, material ports, direction and units."));
            }
        }
    }
    let location = if output {
        connection.destination_store.clone()
    } else {
        match &connection.source {
            MaterialEndpoint::Location(id) => id.clone(),
            MaterialEndpoint::Port(_) => connection.destination_store.clone(),
        }
    };
    let Some(store) = input.project.locations.iter().find(|l| l.id == location) else {
        issues.push(issue(
            owner.clone(),
            path,
            "store_missing",
            "Explicit material location is absent.",
        ));
        return None;
    };
    if !store.accepted_item_types.contains(&item_type.id)
        || store.capacity.as_ref().is_none_or(|q| {
            q.unit != item_type.unit
                || (!fractional_auxiliary && q.value.decimal_places.value() != 0)
        })
    {
        issues.push(issue(
            owner.clone(),
            path,
            "store_capacity",
            "Material requires a matching accepted type and explicit capacity in its own unit.",
        ));
        return None;
    }
    if let MaterialEndpoint::Location(id) = &connection.destination {
        if *id != connection.destination_store {
            issues.push(issue(
                owner.clone(),
                path.clone(),
                "store_binding",
                "Destination location and destination_store disagree.",
            ));
        }
    }
    if !output
        && matches!(&connection.source, MaterialEndpoint::Location(_))
        && connection.destination_store != location
    {
        issues.push(issue(owner.clone(),path.clone(),"input_teleport","Location to input port must use that same real location; no implicit transfer is performed."));
    }
    if connection
        .capacity_rule
        .as_ref()
        .is_some_and(|q| q.unit != item_type.unit || q.value.decimal_places.value() != 0)
    {
        issues.push(issue(
            owner.clone(),
            path.clone(),
            "connection_capacity_unit",
            "Connection capacity has incompatible units.",
        ));
    }
    Some(Material {
        item_type: SubjectRevisionRef {
            id: item_type.id.clone(),
            revision: item_type.revision.clone(),
        },
        amount: amount(&row.quantity),
        decimal_places: row.quantity.value.decimal_places.value(),
        unit: item_type.unit.clone(),
        port: row.port.clone(),
        role: row.role.clone(),
        location,
        qualities: row.required_quality.clone(),
        output_quality: row.declared_output_quality.clone(),
        individual: matches!(item_type.tracking, TrackingKind::Individual),
        capacity: connection.capacity_rule.as_ref().map(amount),
    })
}

pub(crate) fn compile(input: &EngineInput) -> Result<(Vec<Job>, Vec<Issue>), Vec<Issue>> {
    // This checker invocation is the selected document projection. The Host owns
    // other Scenario documents; it must validate their closure before freezing.
    let mut project = input.project.clone();
    project.scenario_refs = vec![input.scenario.id.clone()];
    let mut issues = check_business_inputs_with_definitions(
        &project,
        &BTreeMap::from([(input.scenario.id.clone(), input.scenario.clone())]),
        &input.definitions,
    );
    issues.extend(
        input
            .project
            .issues
            .iter()
            .chain(&input.scenario.issues)
            .cloned(),
    );
    let scenario_owner = ObjectRef {
        kind: ObjectKind::Scenario,
        id: id(input.scenario.id.as_str()),
    };
    if input.input_ref.project_id != input.project.id
        || input.input_ref.project_revision.0 != input.project.revision.0
        || input.input_ref.scenario_id != input.scenario.id
        || input.input_ref.scenario_revision.0 != input.scenario.revision.0
        || input.scenario.project_revision.0 != input.project.revision.0
    {
        issues.push(issue(
            scenario_owner.clone(),
            "/input_ref",
            "frozen_identity",
            "Run input references must match the exact frozen Project and Scenario revisions.",
        ));
    }
    if input.scenario.window.start_ms.0.value() != 0
        || input.scenario.window.end_ms.0.value() < input.scenario.window.start_ms.0.value()
    {
        issues.push(issue(scenario_owner.clone(),"/window","initial_origin","This initial-state Run starts at zero; a nonzero start needs an explicit runtime recovery snapshot."));
    }
    if !input.scenario.parameter_overrides.is_empty() {
        issues.push(issue(scenario_owner.clone(),"/parameter_overrides","unlowered_override","Parameter overrides must be explicitly lowered into the frozen execution definition before simulation."));
    }
    for (i, resource) in input.project.resources.iter().enumerate() {
        if resource.capacity.value.decimal_places.value() != 0
            || !input
                .project
                .calendars
                .iter()
                .any(|c| c.id == resource.calendar_id)
        {
            issues.push(issue(
                object(ObjectKind::Resource, &resource.id),
                format!("/resources/{i}/calendar_id"),
                "resource_calendar",
                "Every resource requires an explicit calendar and integer capacity.",
            ));
        }
    }
    let mut jobs = Vec::new();
    let mut tasks: Vec<_> = input.scenario.production_tasks.iter().collect();
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    for task in tasks {
        let owner = object(ObjectKind::Task, &task.id);
        if !discrete(
            &task.planned_quantity,
            &owner,
            "/planned_quantity",
            &mut issues,
        ) {
            continue;
        }
        let Some(route) = input
            .project
            .routes
            .iter()
            .find(|r| r.id == task.routing_ref.id && r.revision == task.routing_ref.revision)
        else {
            continue;
        };
        let due = task
            .order_allocations
            .iter()
            .filter_map(|a| {
                input
                    .scenario
                    .orders
                    .iter()
                    .find(|o| o.id == a.order_id)
                    .and_then(|o| o.due_at_ms.as_ref().map(|t| t.0.value()))
            })
            .min();
        let release = task
            .order_allocations
            .iter()
            .filter_map(|a| {
                input
                    .scenario
                    .orders
                    .iter()
                    .find(|o| o.id == a.order_id)
                    .map(|o| o.release_at_ms.0.value())
            })
            .max()
            .unwrap_or(task.release_at_ms.0.value())
            .max(task.release_at_ms.0.value());
        // Compile one exact route first. An allocated task plans finished
        // product units; a split operation can produce several per cycle.
        // Unallocated normal-use tasks retain their explicit cycle quantity.
        let first_job = jobs.len();
        for cycle in 0..1 {
            let cycle_key = cycle.to_string();
            let mut steps: Vec<_> = route.steps.iter().collect();
            steps.sort_by(|a, b| a.id.cmp(&b.id));
            for step in steps {
                if !input.scope.is_empty()
                    && !input
                        .scope
                        .iter()
                        .any(|s| matches!(s.kind, ObjectKind::Device) && s.id == step.device_id)
                {
                    continue;
                }
                let Some(device) = input
                    .project
                    .devices
                    .iter()
                    .find(|d| d.id == step.device_id)
                else {
                    continue;
                };
                let execution_capacity = device
                    .role_bindings
                    .resources
                    .get(&RoleId::try_from("execution_capacity".to_owned()).unwrap());
                let Some(capacity) = execution_capacity
                    .and_then(|id| input.project.resources.iter().find(|r| r.id == *id))
                else {
                    issues.push(issue(object(ObjectKind::Device,&device.id),"/role_bindings/resources/execution_capacity","execution_capacity","Execution requires an explicitly bound station resource and its own real calendar."));
                    continue;
                };
                if capacity.capacity.value.decimal_places.value() != 0
                    || amount(&capacity.capacity) == 0
                    || !matches!(capacity.kind, ResourceKind::Fixture)
                {
                    issues.push(issue(owner.clone(),"/execution_capacity","execution_capacity_unit","Execution capacity must be an explicitly bound positive integer fixture pool in its declared unit."));
                    continue;
                }
                let Some(definition) = module(input, &step.operation_ref) else {
                    issues.push(issue(
                        owner.clone(),
                        "/operation_ref",
                        "operation_missing",
                        "Exact operation definition is absent or ambiguous.",
                    ));
                    continue;
                };
                let mut preparation = None;
                let mut exclusive = true;
                let mut operation = match &definition.payload {
                    ModulePayload::Operations(op)
                        if operation_allowed(input, device, &step.operation_ref) =>
                    {
                        op.clone()
                    }
                    ModulePayload::Devices(d)
                        if same_module(&device.module_ref, &step.operation_ref)
                            && matches!(d.behavior.selected, Some(BehaviorSourceKind::Inline)) =>
                    {
                        match &d.behavior.inline {
                            Some(op) => op.clone(),
                            None => {
                                issues.push(issue(
                                    owner.clone(),
                                    "/operation_ref",
                                    "inline_missing",
                                    "Selected inline behavior is missing.",
                                ));
                                continue;
                            }
                        }
                    }
                    ModulePayload::Tasks(t)
                        if matches!(
                            t.kind,
                            TaskKind::ManualPreparation | TaskKind::SupportedAutomatic
                        ) =>
                    {
                        exclusive = matches!(t.equipment_access, EquipmentAccess::Exclusive);
                        if t.completion_effect.produces_material {
                            issues.push(issue(
                                owner.clone(),
                                "/completion_effect",
                                "preparation_material",
                                "Preparation readiness cannot manufacture material.",
                            ));
                            continue;
                        }
                        let Some(target) = device
                            .role_bindings
                            .targets
                            .get(&t.completion_effect.target_role)
                        else {
                            issues.push(issue(
                                owner.clone(),
                                "/target_role",
                                "preparation_target",
                                "Preparation target must be explicitly bound.",
                            ));
                            continue;
                        };
                        let mut prerequisites = Vec::new();
                        for required in &t.prerequisites {
                            if let Some(target) =
                                device.role_bindings.targets.get(&required.target_role)
                            {
                                prerequisites.push((target.id.clone(), required.marker.clone()));
                            } else {
                                issues.push(issue(
                                    owner.clone(),
                                    "/prerequisites",
                                    "preparation_target",
                                    "Readiness prerequisite has no explicit target.",
                                ));
                            }
                        }
                        preparation = Some(Preparation {
                            target: target.id.clone(),
                            marker: t.completion_effect.marker.clone(),
                            prerequisites,
                        });
                        t.preparation_operation()
                    }
                    _ => {
                        issues.push(issue(owner.clone(),"/operation_ref","operation_binding","Route operation must match the explicitly selected inline, Operations or supported Tasks source."));
                        continue;
                    }
                };
                if preparation.is_none()
                    && current_consumer_bindings(&input.project)
                        .iter()
                        .any(|binding| {
                            matches!(binding.consumer_ref.kind, ObjectKind::Device)
                                && binding.consumer_ref.id == device.id
                                && matches!(binding.mode, ProfessionalMode::CalculatedNominal)
                                && !same_module(&binding.operation_ref, &step.operation_ref)
                        })
                {
                    issues.push(issue(object(ObjectKind::Device,&device.id),"/route_operation_refs","nominal_operation_scope","A Nominal binding authorizes only its exact consumer operation; the route allowlist cannot extend it."));
                    continue;
                }
                if preparation.is_none() {
                    crate::parameters::apply(
                        input,
                        device,
                        &step.operation_ref,
                        &mut operation,
                        &mut issues,
                    );
                }
                let family = crate::engine::setup::compile(
                    input,
                    device,
                    step,
                    &operation,
                    &task.routing_ref,
                    &mut issues,
                );
                let inputs: Vec<_> = operation
                    .inputs
                    .iter()
                    .filter_map(|r| material(input, device, step, r, false, &owner, &mut issues))
                    .collect();
                let outputs: Vec<_> = operation
                    .outputs
                    .iter()
                    .filter_map(|r| material(input, device, step, r, true, &owner, &mut issues))
                    .collect();
                if preparation.is_none() {
                    if let Some(product) = task
                        .order_allocations
                        .first()
                        .and_then(|a| input.scenario.orders.iter().find(|o| o.id == a.order_id))
                        .and_then(|o| {
                            input.project.products.iter().find(|p| {
                                p.id == o.product_ref.id && p.revision == o.product_ref.revision
                            })
                        })
                    {
                        if let Some(bom) = input.project.boms.iter().find(|b| {
                            b.id == product.bom_ref.id && b.revision == product.bom_ref.revision
                        }) {
                            for (i, row) in bom
                                .rows
                                .iter()
                                .enumerate()
                                .filter(|(_, r)| r.consume_at_operation == step.id)
                            {
                                let matching: Vec<_> = inputs
                                    .iter()
                                    .filter(|m| {
                                        same_type(&m.item_type, &row.item_type_ref)
                                            && m.unit == row.quantity.unit
                                    })
                                    .collect();
                                let precision=matching.iter().map(|m|m.decimal_places).chain(std::iter::once(row.quantity.value.decimal_places.value())).max().unwrap();
                                let actual:u128=matching.iter().map(|m|m.amount as u128*10_u128.pow((precision-m.decimal_places) as u32)).sum();
                                if actual != amount(&row.quantity) as u128*10_u128.pow((precision-row.quantity.value.decimal_places.value()) as u32)
                                {
                                    issues.push(issue(object(ObjectKind::Bom,&bom.id),format!("/rows/{i}/quantity"),"bom_operation_quantity","The selected operation's explicit material ratio must equal this product BOM row at its consuming operation."));
                                }
                            }
                        }
                    }
                }
                let mut job_resources = resource_requirements(
                    input,
                    device,
                    step,
                    &operation.resource_roles,
                    &owner,
                    &mut issues,
                );
                if exclusive {
                    job_resources
                        .entry(capacity.id.clone())
                        .and_modify(|n| *n = (*n).max(1))
                        .or_insert(1);
                }
                let mut phases = Vec::new();
                if operation.phases.is_empty() {
                    if let Some(work) = &operation.duration {
                        phases.push(Phase {
                            id: PhaseId::try_from("cycle".to_owned()).unwrap(),
                            work: work.value.0.value(),
                            resources: BTreeMap::new(),
                            pause_resume: false,
                        });
                    } else {
                        issues.push(issue(
                            owner.clone(),
                            "/duration",
                            "duration_missing",
                            "A fixed duration or complete explicit phases is required.",
                        ));
                    }
                } else {
                    if operation.duration.is_some() {
                        issues.push(issue(
                            owner.clone(),
                            "/duration",
                            "duplicate_duration",
                            "Explicit phases must not be added to another cycle duration.",
                        ));
                    }
                    let mut ids = BTreeSet::new();
                    for (phase_index, p) in operation.phases.iter().enumerate() {
                        if !ids.insert(p.id.clone()) {
                            issues.push(issue(
                                owner.clone(),
                                "/phases",
                                "duplicate_phase",
                                "Phase identities must be unique within an operation.",
                            ));
                        }
                        if let Some(work) = &p.duration {
                            phases.push(Phase {
                                id: p.id.clone(),
                                work: work.value.0.value(),
                                resources: resource_requirements(
                                    input,
                                    device,
                                    step,
                                    &p.phase_resources,
                                    &owner,
                                    &mut issues,
                                ),
                                pause_resume: matches!(
                                    p.interruption_policy,
                                    InterruptionPolicy::PauseResume
                                ),
                            });
                        } else {
                            issues.push(issue(
                                owner.clone(),
                                format!("/phases/{phase_index}/duration"),
                                "duration_missing",
                                "Each phase requires a declared fixed duration.",
                            ));
                        }
                    }
                }
                let mut output_holds = BTreeMap::new();
                for role in &operation.hold_on_output_wait {
                    let path = format!("/hold_on_output_wait/{}", role.as_str());
                    let last = operation
                        .phases
                        .last()
                        .and_then(|p| p.phase_resources.iter().find(|r| r.role == *role));
                    let resource = step
                        .resource_bindings
                        .get(role)
                        .or_else(|| device.role_bindings.resources.get(role));
                    if last.is_none() || resource.is_none() {
                        issues.push(issue(owner.clone(),path,"output_hold_source","An extra output hold must explicitly name a resource role required in the final declared phase; no new auxiliary resource is guessed at completion."));
                        continue;
                    }
                    let resource = resource.unwrap();
                    if job_resources.contains_key(resource) {
                        issues.push(issue(owner.clone(),path,"output_hold_conflict","The extra output-hold resource is already held for the whole Job; conflicting ownership declarations must be resolved explicitly."));
                        continue;
                    }
                    if let Some(units) = phases.last().and_then(|p| p.resources.get(resource)) {
                        output_holds.insert(resource.clone(), *units);
                    }
                }
                if matches!(
                    operation.identity_policy,
                    IdentityPolicy::AssemblyWithComposition | IdentityPolicy::ContainerWithContents
                ) && (outputs.len() != 1 || outputs[0].amount != 1)
                {
                    issues.push(issue(owner.clone(),"/outputs","identity_output","Assembly/container identity requires one explicit output per cycle; per-output composition cannot be guessed."));
                }
                if preparation.is_none()
                    && !matches!(operation.builtin, RuleKind::InspectAndRoute)
                    && matches!(operation.identity_policy, IdentityPolicy::Preserve)
                    && operation.material_flow.is_none()
                    && (inputs.len() != 1
                        || outputs.len() != 1
                        || inputs[0].amount != outputs[0].amount
                        || !same_type(&inputs[0].item_type, &outputs[0].item_type))
                {
                    issues.push(issue(
                        owner.clone(),
                        "/identity_policy",
                        "preserve_identity",
                        "Preserve requires one matching input/output quantity and exact type.",
                    ));
                }
                crate::phase_material::check_flow(
                    input,
                    &operation,
                    &inputs,
                    &outputs,
                    &phases,
                    &owner,
                    &mut issues,
                );
                if matches!(operation.builtin, RuleKind::Pack)
                    && matches!(
                        operation.identity_policy,
                        IdentityPolicy::ContainerWithContents
                    )
                {
                    let valid = operation.pack_identity.as_ref().is_some_and(|mapping| {
                        let materials: BTreeSet<_> =
                            mapping.container_material_roles.iter().cloned().collect();
                        let contents: BTreeSet<_> = mapping.content_roles.iter().cloned().collect();
                        let roles: BTreeSet<_> =
                            operation.inputs.iter().map(|i| i.role.clone()).collect();
                        outputs.len() == 1
                            && outputs[0].role == mapping.output_role
                            && !materials.is_empty()
                            && !contents.is_empty()
                            && materials.len() == mapping.container_material_roles.len()
                            && contents.len() == mapping.content_roles.len()
                            && materials.is_disjoint(&contents)
                            && materials.union(&contents).cloned().collect::<BTreeSet<_>>() == roles
                            && roles.len() == operation.inputs.len()
                    });
                    if !valid {
                        issues.push(issue(owner.clone(),"/pack_identity","pack_identity_mapping","Pack requires one output role and nonempty, disjoint, unique material/content roles exactly covering the declared inputs."));
                    }
                } else if operation.pack_identity.is_some()
                    || matches!(
                        operation.identity_policy,
                        IdentityPolicy::ContainerWithContents
                    )
                {
                    issues.push(issue(owner.clone(),"/pack_identity","pack_identity_rule","Explicit packaging identity mapping is supported only by Pack with ContainerWithContents."));
                }
                let job_id = stable("job", &[task.id.as_str(), &cycle_key, step.id.as_str()]);
                jobs.push(Job {
                    id: job_id,
                    task: task.id.clone(),
                    operation_ref: step.operation_ref.clone(),
                    execution: JobExecutionRef::ProjectDevice {
                        device_id: device.id.clone(),
                        route_step_id: Some(step.id.clone()),
                        execution_capacity_ref: object(ObjectKind::Resource, &capacity.id),
                    },
                    operation: Some(operation),
                    family,
                    setup: None,
                    transport: None,
                    inputs,
                    outputs,
                    job_resources,
                    output_holds,
                    phases,
                    predecessors: step
                        .predecessors
                        .iter()
                        .map(|p| stable("job", &[task.id.as_str(), &cycle_key, p.as_str()]))
                        .collect(),
                    priority: task.priority,
                    due,
                    release,
                    preparation,
                    cycle,
                    quality: None,
                });
            }
        }
        let mut cycles = amount(&task.planned_quantity);
        if let Some(product) = task.order_allocations.first()
            .and_then(|a| input.scenario.orders.iter().find(|o| o.id == a.order_id))
            .and_then(|o| input.project.products.iter().find(|p|
                p.id == o.product_ref.id && p.revision == o.product_ref.revision))
        {
            let per_cycle: u128 = jobs[first_job..].iter().filter(|job| {
                let JobExecutionRef::ProjectDevice { route_step_id: Some(step), .. } = &job.execution else { return false };
                !route.steps.iter().any(|next| next.predecessors.contains(step))
            }).flat_map(|job| job.outputs.iter().filter(|material| {
                same_type(&material.item_type, &product.item_type_ref)
                    && material.unit == task.planned_quantity.unit
                    && job.operation.as_ref().and_then(|op| op.quality_and_routes.as_ref())
                        .is_none_or(|rule| material.port == rule.pass_port)
                    // Some established routes express the material sequence
                    // through stock/supply bindings rather than predecessors.
                    // A package later sealed is still the same one product.
                    && !jobs[first_job..].iter().any(|next| next.id != job.id && next.inputs.iter().any(|input_material|
                        same_type(&input_material.item_type, &material.item_type)
                            && (input_material.location == material.location || input.scenario.supplies.iter().any(|s|
                                s.source == material.location && s.destination == input_material.location
                                    && same_type(&s.item_type_ref, &material.item_type)))))
            })).map(|material| u128::from(material.amount) * 10_u128.pow(9 - u32::from(material.decimal_places))).sum();
            // A scoped observation may exclude the terminal product step.
            if per_cycle != 0 {
                let planned = u128::from(cycles) * 1_000_000_000;
                if planned % per_cycle != 0 {
                    issues.push(issue(owner.clone(), "/planned_quantity", "product_cycle_quantity",
                        "Planned finished quantity must be a whole number of the route's declared terminal product yield; explicitly replan a partial cycle."));
                    continue;
                }
                let Ok(repetitions) = u64::try_from(planned / per_cycle) else {
                    issues.push(issue(owner.clone(), "/planned_quantity", "product_cycle_quantity",
                        "The declared output ratio exceeds the supported integral cycle count."));
                    continue;
                };
                cycles = repetitions;
            }
        }
        let first_cycle = jobs.split_off(first_job);
        for cycle in 0..cycles {
            let ids: BTreeMap<_, _> = route.steps.iter()
                .flat_map(|step| std::iter::once(&step.id).chain(&step.predecessors)).map(|step| {
                (stable("job", &[task.id.as_str(), "0", step.as_str()]),
                 stable("job", &[task.id.as_str(), &cycle.to_string(), step.as_str()]))
            }).collect();
            for original in &first_cycle {
                let mut job = original.clone();
                job.id = ids[&original.id].clone();
                job.predecessors = original.predecessors.iter().map(|p| ids[p].clone()).collect();
                job.cycle = cycle;
                jobs.push(job);
            }
        }
    }
    // A source preparation for the same frozen target/configuration is one real
    // Job. Other cycles depend on it; no synthetic completed Jobs are invented.
    let mut aliases = BTreeMap::new();
    let mut canonical: Vec<Job> = vec![];
    jobs.sort_by(|a, b| {
        a.release
            .cmp(&b.release)
            .then(b.priority.cmp(&a.priority))
            .then(
                a.due
                    .unwrap_or(MAX_SAFE_UINT)
                    .cmp(&b.due.unwrap_or(MAX_SAFE_UINT)),
            )
            .then(a.id.cmp(&b.id))
    });
    for job in jobs {
        let compatible = if job.preparation.is_some() && job.predecessors.is_empty() {
            canonical.iter().find(|prior|prior.preparation==job.preparation&&prior.predecessors.is_empty()&&same_module(&prior.operation_ref,&job.operation_ref)&&prior.job_resources==job.job_resources&&prior.phases==job.phases&&matches!((&prior.execution,&job.execution),(JobExecutionRef::ProjectDevice{device_id:a,..},JobExecutionRef::ProjectDevice{device_id:b,..}) if a==b))
        } else {
            None
        };
        if let Some(prior) = compatible {
            aliases.insert(job.id.clone(), prior.id.clone());
        } else {
            canonical.push(job);
        }
    }
    let mut jobs = canonical;
    for job in &mut jobs {
        for predecessor in &mut job.predecessors {
            if let Some(actual) = aliases.get(predecessor) {
                *predecessor = actual.clone();
            }
        }
        job.predecessors.sort();
        job.predecessors.dedup();
    }
    crate::quality::compile(input, &mut jobs, &mut issues);
    jobs.extend(crate::engine::transport::compile(input, &mut issues));
    for selected in &input.scope {
        let declared_device = input
            .project
            .devices
            .iter()
            .find(|device| device.id == selected.id);
        let known_device = declared_device
            .and_then(|device| module(input, &device.module_ref))
            .is_some_and(|definition| matches!(definition.payload, ModulePayload::Devices(_)));
        if !matches!(selected.kind, ObjectKind::Device) || !known_device {
            issues.push(issue(
                selected.clone(),
                "/scope",
                "scope_uncompiled",
                "Selected execution scope must name an existing device with its exact loaded device definition; an empty demand plan does not invent or remove scope objects.",
            ));
        }
    }
    // Select whole declared tasks whose route devices are in the requested scope;
    // dependencies cannot silently disappear when a partial scope is requested.
    if !input.scope.is_empty() {
        let wanted: BTreeSet<_> = input.scope.iter().map(|s| s.id.clone()).collect();
        if input.scenario.production_tasks.iter().any(|task| {
            input
                .project
                .routes
                .iter()
                .find(|r| r.id == task.routing_ref.id && r.revision == task.routing_ref.revision)
                .is_some_and(|r| r.steps.iter().any(|s| !wanted.contains(&s.device_id)))
        }) {
            let mut gap = issue(
                scenario_owner.clone(),
                "/scope",
                "partial_scope",
                "This Run executes only the selected supported production devices; excluded operations cannot support a complete factory/order conclusion.",
            );
            gap.blocks = vec![CapabilityPurpose::CompleteConclusion];
            issues.push(gap);
        }
        jobs.retain(|j|matches!(&j.execution,JobExecutionRef::ProjectDevice{device_id,..} if wanted.contains(device_id)));
        let present: BTreeSet<_> = jobs.iter().map(|j| j.id.clone()).collect();
        for job in &jobs {
            if job.predecessors.iter().any(|p| !present.contains(p)) {
                issues.push(issue(object(ObjectKind::Job,&job.id),"/predecessors","scope_dependency","Selected scope excludes a declared predecessor; choose a complete supported route scope."));
            }
        }
    }
    for (i, item) in input.scenario.initial_conditions.items.iter().enumerate() {
        if item.quantity.value.decimal_places.value() != 0
            && !jobs.iter().any(|job| {
                job.inputs.iter().any(|m| {
                    same_type(&m.item_type, &item.item_type_ref)
                        && job
                            .operation
                            .as_ref()
                            .and_then(|op| op.material_flow.as_ref())
                            .is_some_and(|flow| flow.auxiliary_roles.contains(&m.role))
                        && m.decimal_places == item.quantity.value.decimal_places.value()
                })
            })
        {
            issues.push(issue(scenario_owner.clone(),format!("/initial_conditions/items/{i}/quantity"),"fractional_stock_scope","This discrete kernel requires integer stock quantities; fractional material is not rescaled or split as integer stock."));
        }
    }
    for (i, arrival) in input.scenario.arrivals.iter().enumerate() {
        for (j, item) in arrival.items.iter().enumerate() {
            if item.quantity.value.decimal_places.value() != 0 {
                issues.push(issue(scenario_owner.clone(),format!("/arrivals/{i}/items/{j}/quantity"),"fractional_stock_scope","Fractional arrival quantities require a fixed-scale material consumer before execution."));
            }
        }
    }
    for (i, allocation) in input
        .scenario
        .initial_conditions
        .material_allocations
        .iter()
        .enumerate()
    {
        if allocation.quantity.value.decimal_places.value() != 0 {
            issues.push(issue(
                scenario_owner.clone(),
                format!("/initial_conditions/material_allocations/{i}/quantity"),
                "fractional_stock_scope",
                "Fractional reservations are not converted to integer quantities.",
            ));
        }
    }
    for (i, order) in input.scenario.orders.iter().enumerate() {
        if !matches!(order.acceptance_stage, AcceptanceStage::FactoryReleased)
            && !crate::engine::transport::supports_acceptance(
                input,
                &order.product_ref,
                &order.destination,
            )
        {
            issues.push(issue(scenario_owner.clone(),format!("/orders/{i}/acceptance_stage"),"acceptance_stage_scope","This production kernel proves factory_released only. Received and unloaded_and_released require actual transport/receipt/unloading effects; the requested stage is not lowered."));
        }
        if order
            .quantity
            .as_ref()
            .is_some_and(|q| q.value.decimal_places.value() != 0)
        {
            issues.push(issue(
                scenario_owner.clone(),
                format!("/orders/{i}/quantity"),
                "fractional_stock_scope",
                "This discrete production Run requires integer order quantities.",
            ));
        }
    }
    let (blocking, limitations): (Vec<_>, Vec<_>) = issues.into_iter().partition(|i| {
        i.blocks
            .iter()
            .any(|b| matches!(b, CapabilityPurpose::Execution))
    });
    if blocking.is_empty() {
        Ok((jobs, limitations))
    } else {
        Err(blocking)
    }
}
