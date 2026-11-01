//! Admission for declared effects on a preserved physical item. Runtime effects
//! are executed by the existing Engine queue, claims and Stock histories.
use crate::{EngineInput, compile::*};
use sf_domain::*;
use std::collections::BTreeSet;

pub(crate) fn check_flow(
    input: &EngineInput,
    operation: &OperationDefinition,
    inputs: &[Material],
    outputs: &[Material],
    phases: &[Phase],
    owner: &ObjectRef,
    issues: &mut Vec<Issue>,
) {
    let Some(flow) = &operation.material_flow else {
        return;
    };
    let primary: Vec<_> = inputs
        .iter()
        .filter(|m| m.role == flow.primary_role)
        .collect();
    let auxiliary: BTreeSet<_> = flow.auxiliary_roles.iter().collect();
    if !matches!(operation.identity_policy, IdentityPolicy::Preserve)
        || !matches!(operation.builtin, RuleKind::Transform)
        || primary.len() != 1
        || outputs.len() != 1
        || auxiliary.len() != flow.auxiliary_roles.len()
        || auxiliary.contains(&flow.primary_role)
        || inputs
            .iter()
            .any(|m| m.role != flow.primary_role && !auxiliary.contains(&m.role))
        || flow
            .auxiliary_roles
            .iter()
            .any(|r| inputs.iter().filter(|m| &m.role == r).count() != 1)
        || (primary.len() == 1
            && outputs.len() == 1
            && (!same_type(&primary[0].item_type, &outputs[0].item_type)
                || primary[0].amount != outputs[0].amount
                || primary[0].unit != outputs[0].unit
                || primary[0].decimal_places != 0))
    {
        issues.push(issue(owner.clone(),"/material_flow","material_flow_identity",
            "Phase material flow requires one matching preserved main item/output and explicitly enumerated distinct auxiliary inputs."));
        return;
    }
    for m in inputs.iter().filter(|m| auxiliary.contains(&m.role)) {
        if m.individual || m.amount == 0 {
            issues.push(issue(
                owner.clone(),
                "/material_flow/auxiliary_roles",
                "material_flow_auxiliary",
                "Consumed auxiliary material must be a positive declared batch quantity.",
            ));
        }
        for item in input
            .scenario
            .initial_conditions
            .items
            .iter()
            .filter(|i| same_type(&i.item_type_ref, &m.item_type))
        {
            if item.quantity.value.decimal_places.value() != m.decimal_places {
                issues.push(issue(owner.clone(),"/initial_conditions/items/quantity","material_flow_precision","Initial auxiliary stock and operation quantity require the same explicit fixed precision; no unit or implicit scale conversion."));
            }
        }
    }
    let index = |id: &Option<PhaseId>, default: usize| -> Option<usize> {
        match id {
            None => Some(default),
            Some(id) => phases.iter().position(|p| &p.id == id).map(|i| i + 1),
        }
    };
    let end = phases.len();
    let acquire = index(&flow.acquire_after_phase, 0);
    let consume = if flow.auxiliary_roles.is_empty() && flow.attributes_after_consume.is_empty() {
        acquire
    } else {
        index(&flow.consume_after_phase, end)
    };
    let deliver = index(&flow.deliver_after_phase, end);
    let reserve = index(&flow.reserve_before_phase, 0);
    if acquire.is_none()
        || consume.is_none()
        || deliver.is_none()
        || reserve.is_none()
        || acquire > consume
        || acquire > deliver
        || consume > deliver
        || flow.reserve_before_phase.is_some() && reserve > deliver
    {
        issues.push(issue(owner.clone(),"/material_flow","material_flow_phase_order","Physical phase effects must reference existing ordered acquisition, consumption, reservation and delivery boundaries."));
    }
    let item_type = input
        .project
        .item_types
        .iter()
        .find(|t| t.id == primary[0].item_type.id)
        .unwrap();
    for (name, value) in flow
        .required_primary_attributes
        .iter()
        .chain(&flow.attributes_after_consume)
    {
        let declaration = item_type.attribute_schema.iter().find(|a| a.name == *name);
        let valid=declaration.is_some_and(|a| {
            matches!((&a.value_type,value),(ValueKind::Enum,TypedValue::Enum(v)) if a.options.contains(v))
        });
        if !valid {
            issues.push(issue(owner.clone(),format!("/material_flow/attributes/{name}"),"material_flow_attribute","Phase material flow supports declared finite enum instance states; values must match the main-item schema options."));
        }
    }
}

pub(crate) fn fixed_quantity(n: u64, unit: &UnitId, places: u8) -> Quantity {
    Quantity {
        value: DecimalAmount {
            coefficient: uint(n),
            decimal_places: DecimalPlaces::try_from(places).unwrap(),
        },
        unit: unit.clone(),
    }
}
