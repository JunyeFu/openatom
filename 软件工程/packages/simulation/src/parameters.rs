//! Apply declared instance values to a cloned effective rule. Library definitions
//! and frozen input documents are never edited while compiling a Run.
use crate::{EngineInput, compile::*};
use sf_domain::*;

pub(crate) fn apply(
    input: &EngineInput,
    device: &DeviceInstance,
    operation_ref: &ModuleRef,
    operation: &mut OperationDefinition,
    issues: &mut Vec<Issue>,
) {
    let Some(definition) = input
        .definitions
        .iter()
        .find(|d| d.id == device.module_ref.id && d.version == device.module_ref.version)
    else {
        return;
    };
    for (key, value) in &device.configuration {
        let owner = object(ObjectKind::Device, &device.id);
        let path = format!(
            "/configuration/{}/value",
            key.replace('~', "~0").replace('/', "~1")
        );
        let declarations: Vec<_> = definition
            .parameters
            .iter()
            .filter(|p| p.id.as_str() == key)
            .collect();
        if declarations.len() != 1 {
            issues.push(issue(owner,path,"parameter_declaration","Instance configuration requires one exact parameter declaration; an unknown key is not a duration alias."));
            continue;
        }
        let parameter = declarations[0];
        let sources: Vec<_> = input
            .project
            .parameter_sources
            .iter()
            .filter(|s| {
                matches!(s.target.kind, ObjectKind::Device)
                    && s.target.id == device.id
                    && s.field_path == path
            })
            .collect();
        if sources.len() > 1
            || sources.first().is_some_and(|s| {
                s.value
                    .as_ref()
                    .is_none_or(|recorded| format!("{recorded:?}") != format!("{value:?}"))
            })
        {
            issues.push(issue(
                owner,
                path,
                "parameter_source_mismatch",
                "Stored parameter source does not name exactly this instance value.",
            ));
            continue;
        }
        // These exact declarations select the professional Job in the Host's
        // artifact consumer. They stay in the frozen input and source records;
        // they do not specialize an operation's duration or material quantity.
        let professional_selector = match (key.as_str(), parameter.binding.as_str()) {
            ("packaging_job_id", "/payload/data/configuration/packaging_job_id/value") => matches!(
                (&parameter.value_type, value),
                (ValueKind::String, TypedValue::String(_))
            ),
            (
                "packaging_job_revision",
                "/payload/data/configuration/packaging_job_revision/value",
            ) => {
                parameter.integer
                    && matches!(
                        (&parameter.value_type, value),
                        (ValueKind::Integer, TypedValue::Integer(_))
                    )
            }
            _ => false,
        };
        if professional_selector {
            continue;
        }
        let type_matches = matches!(
            (&parameter.value_type, value),
            (ValueKind::Integer, TypedValue::Integer(_))
                | (ValueKind::Number, TypedValue::Number(_))
                | (ValueKind::Duration, TypedValue::Duration(_))
        );
        if !type_matches {
            issues.push(issue(owner,path,"parameter_type","This execution binding requires its declared integer, number or duration type; other declared bindings need their own consumer."));
            continue;
        }
        let scalar = match value {
            TypedValue::Integer(n) => n.value() as f64,
            TypedValue::Number(n) => n.value(),
            TypedValue::Duration(d) => d.value.0.value() as f64,
            _ => unreachable!(),
        };
        if scalar < 0.0
            || scalar > MAX_SAFE_UINT as f64
            || scalar.fract() != 0.0
            || parameter
                .minimum
                .as_ref()
                .is_some_and(|n| scalar < n.value())
            || parameter
                .maximum
                .as_ref()
                .is_some_and(|n| scalar > n.value())
        {
            issues.push(issue(owner,path,"parameter_range","Execution duration must satisfy the declared bounds and exact nonnegative integer millisecond range."));
            continue;
        }
        if parameter
            .unit
            .as_ref()
            .is_some_and(|u| u.as_str() != "millisecond")
        {
            issues.push(issue(owner,path,"parameter_unit","Execution duration binding requires canonical milliseconds; no undeclared seconds conversion is performed."));
            continue;
        }
        let Some(binding) = parameter
            .binding
            .strip_prefix("/payload/data/behavior/inline/")
        else {
            issues.push(issue(owner,path,"parameter_binding_scope","Unsupported execution parameter binding; use the exact declared RFC6901 binding, not its final field name."));
            continue;
        };
        if !same_module(operation_ref, &device.module_ref) {
            issues.push(issue(
                owner,
                path,
                "inactive_parameter_binding",
                "Declared inline parameter is not the selected behavior for this route operation.",
            ));
            continue;
        }
        let basis = sources
            .first()
            .map(|s| s.basis.clone())
            .unwrap_or(SourceBasis::Unknown);
        let (target, whole) = if binding == "duration" {
            (operation.duration.as_mut(), true)
        } else if binding == "duration/value" {
            (operation.duration.as_mut(), false)
        } else {
            let parts: Vec<_> = binding.split('/').collect();
            if parts.first() == Some(&"phases")
                && matches!(parts.len(), 3 | 4)
                && parts[2] == "duration"
                && (parts.len() == 3 || parts[3] == "value")
            {
                (
                    parts[1]
                        .parse::<usize>()
                        .ok()
                        .and_then(|i| operation.phases.get_mut(i))
                        .and_then(|p| p.duration.as_mut()),
                    parts.len() == 3,
                )
            } else {
                (None, false)
            }
        };
        let Some(target) = target else {
            issues.push(issue(
                owner,
                path,
                "parameter_binding_missing",
                "Declared duration/phase binding does not exist in this effective operation.",
            ));
            continue;
        };
        if whole {
            let TypedValue::Duration(value) = value else {
                issues.push(issue(
                    owner,
                    path,
                    "parameter_binding_type",
                    "Whole duration binding requires a complete typed DurationValue.",
                ));
                continue;
            };
            *target = value.clone();
            target.basis = basis;
        } else {
            if matches!(value, TypedValue::Duration(_)) {
                issues.push(issue(owner,path,"parameter_binding_type","The duration value member requires the declared numeric value; a whole duration cannot replace a scalar."));
                continue;
            }
            target.value = duration(scalar as u64);
            target.basis = basis;
            target.source_ref = sources.first().and_then(|s| s.source_ref.clone());
        }
    }
}
