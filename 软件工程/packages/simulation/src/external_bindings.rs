//! Canonical route/device bindings for the actual frozen compiler input.
use super::*;

impl Engine {
    /// Describe required bindings before cloud registration. These IDs are local
    /// proposals; new_external still validates the registered frozen bindings.
    pub fn required_external_bindings(input: &EngineInput) -> Result<Vec<wire::CandidateBinding>, ExternalError> {
    let (compiled, _) = crate::compile::compile(input).map_err(|issues| error("external.input", format!("{issues:?}")))?;
    let mut bindings = BTreeMap::new();
    for spec in &compiled {
        if let Some(plan) = &spec.transport {
            let requirements: BTreeMap<_, _> = plan.definition.resources.iter()
                .chain(&plan.definition.loading_resources).chain(&plan.definition.unloading_resources)
                .map(|r| (r.role.clone(), amount(&r.quantity))).collect();
            for phase in &spec.phases {
                let name = format!("{}-{}", plan.supply.id.as_str(), phase.id.as_str());
                bindings.insert(name.clone(), wire::CandidateBinding {
                    binding_id: name.try_into().unwrap(), device_id: plan.carrier.clone(), device_module_ref: spec.operation_ref.clone(),
                    operation_ref: spec.operation_ref.clone(), execution_capacity_ref: object(ObjectKind::Resource, &plan.carrier),
                    input_ports: vec![], output_ports: vec![], resources: requirements.iter().map(|(role, n)| wire::ResourceBinding {
                        role_id: role.clone(), resource_id: plan.supply.resources[role].clone(), units: (*n).try_into().unwrap(),
                    }).collect(), phase_id: phase.id.clone(), nominal_work_ms: uint(phase.work),
                    setup_transition: None, transport_supply_id: Some(plan.supply.id.clone()),
                });
            }
            continue;
        }
        let JobExecutionRef::ProjectDevice { device_id, execution_capacity_ref, route_step_id, .. } = &spec.execution else {
            return Err(error("external.binding", "Compiled work lacks a route device or supply carrier"));
        };
        let device = input.project.devices.iter().find(|d| &d.id == device_id).unwrap();
        let task = input.scenario.production_tasks.iter().find(|task| task.id == spec.task).unwrap();
        let route = input.project.routes.iter().find(|route| route.id == task.routing_ref.id && route.revision == task.routing_ref.revision).unwrap();
        let step = route.steps.iter().find(|s| Some(&s.id) == route_step_id.as_ref()).unwrap();
        let ports = |materials: &[crate::compile::Material]| materials.iter().map(|m| wire::PortBinding {
            role_id: m.role.clone(), port_id: m.port.clone(), location_id: m.location.clone(),
            item_type_id: m.item_type.id.clone(), unit_id: m.unit.clone(),
        }).collect();
        let requirements: BTreeMap<_, _> = spec.op().resource_roles.iter()
            .chain(spec.op().phases.iter().flat_map(|p| &p.phase_resources))
            .map(|r| (r.role.clone(), amount(&r.quantity))).collect();
        for phase in &spec.phases {
            let name = format!("{}-{}-{}", device_id.as_str(), spec.operation_ref.id.as_str(), phase.id.as_str());
            bindings.insert(name.clone(), wire::CandidateBinding {
                binding_id: name.try_into().unwrap(), device_id: device_id.clone(),
                device_module_ref: device.module_ref.clone(), operation_ref: spec.operation_ref.clone(),
                execution_capacity_ref: execution_capacity_ref.clone(), input_ports: ports(&spec.inputs), output_ports: ports(&spec.outputs),
                resources: requirements.iter().map(|(role, n)| wire::ResourceBinding {
                    role_id: role.clone(), resource_id: step.resource_bindings.get(role).or_else(|| device.role_bindings.resources.get(role)).unwrap().clone(), units: (*n).try_into().unwrap(),
                }).collect(), phase_id: phase.id.clone(), nominal_work_ms: uint(phase.work), setup_transition: None, transport_supply_id: None,
            });
        }
        for transition in crate::engine::setup::required_transitions(&spec) {
            let spec = crate::engine::setup::external_setup(&spec, &transition).unwrap();
            let rule = input.project.setup_policies.iter().find(|p| p.device_id == *device_id).unwrap()
                .changeovers.iter().find(|r| r.from_family == transition.from_family && r.to_family == transition.to_family).unwrap();
            let name = format!("{}-{}-setup-{:?}-{}", device_id.as_str(), spec.operation_ref.id.as_str(), transition.from_family, transition.to_family);
            bindings.insert(name.clone(), wire::CandidateBinding {
                binding_id: name.try_into().unwrap(), device_id: device_id.clone(), device_module_ref: device.module_ref.clone(),
                operation_ref: spec.operation_ref.clone(), execution_capacity_ref: execution_capacity_ref.clone(),
                input_ports: vec![], output_ports: vec![],
                resources: rule.resources.iter().map(|r| wire::ResourceBinding {
                    role_id: r.role.clone(), resource_id: step.resource_bindings.get(&r.role).or_else(|| device.role_bindings.resources.get(&r.role)).unwrap().clone(), units: amount(&r.quantity).try_into().unwrap(),
                }).collect(), phase_id: spec.phases[0].id.clone(), nominal_work_ms: uint(spec.phases[0].work), setup_transition: Some(transition), transport_supply_id: None,
            });
        }
    }
    Ok(bindings.into_values().collect())
    }
}
