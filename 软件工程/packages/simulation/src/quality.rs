use crate::{EngineInput, compile::*};
use sf_domain::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(crate) struct Policy {
    pub rule: QualityRule,
    pub pass: usize,
    pub fail: usize,
    pub isolation: usize,
    pub rework: Option<ObjectId>,
    pub pending: Option<ObjectId>,
}
fn location_target(input: &EngineInput, job: &Job, role: &RoleId) -> Option<ObjectId> {
    let JobExecutionRef::ProjectDevice { device_id, .. } = &job.execution else {
        return None;
    };
    input
        .project
        .devices
        .iter()
        .find(|d| d.id == *device_id)?
        .role_bindings
        .targets
        .get(role)
        .filter(|r| matches!(r.kind, ObjectKind::Location))
        .map(|r| r.id.clone())
}
pub(crate) fn compile(input: &EngineInput, jobs: &mut [Job], issues: &mut Vec<Issue>) {
    let mut policies = vec![];
    for (index, job) in jobs.iter().enumerate() {
        if matches!(job.op().builtin, RuleKind::Rework) {
            if !matches!(job.op().identity_policy, IdentityPolicy::Preserve)
                || job.inputs.len() != 1
                || job.outputs.len() != 1
                || job.outputs.iter().any(|o| {
                    o.output_quality
                        .as_ref()
                        .is_some_and(|q| !matches!(q, QualityState::Unchecked))
                })
            {
                issues.push(issue(object(ObjectKind::Job,&job.id),"/identity_policy","rework_identity","Rework preserves one complete physical identity; its completion cannot release quality or manufacture another item."));
            }
            continue;
        }
        if !matches!(job.op().builtin, RuleKind::InspectAndRoute) {
            continue;
        }
        let owner = object(ObjectKind::Job, &job.id);
        let Some(rule) = &job.op().quality_and_routes else {
            issues.push(issue(owner,"/quality_and_routes","quality_rule_missing","Inspection requires an explicit result source, branch ports and finite completed-rework limit."));
            continue;
        };
        if !matches!(job.op().identity_policy, IdentityPolicy::Preserve)
            || job.inputs.len() != 1
            || job.outputs.is_empty()
            || job.outputs.iter().any(|o| {
                o.amount != job.inputs[0].amount
                    || o.unit != job.inputs[0].unit
                    || !same_type(&o.item_type, &job.inputs[0].item_type)
                    || o.output_quality.is_some()
            })
        {
            issues.push(issue(owner,"/outputs","inspection_identity","Inspection outputs are alternatives for the same complete item, exact type and quantity. Output declarations must not override the actual quality decision."));
            continue;
        }
        let pass: Vec<_> = job
            .outputs
            .iter()
            .enumerate()
            .filter(|(_, o)| o.port == rule.pass_port)
            .map(|(i, _)| i)
            .collect();
        let fail: Vec<_> = job
            .outputs
            .iter()
            .enumerate()
            .filter(|(_, o)| o.port == rule.fail_port)
            .map(|(i, _)| i)
            .collect();
        if pass.len() != 1 || fail.len() != 1 {
            issues.push(issue(
                owner,
                "/quality_and_routes",
                "quality_branch_missing",
                "Pass/fail must each resolve to one declared physical output port.",
            ));
            continue;
        }
        let isolation_location = rule
            .isolation_location_role
            .as_ref()
            .and_then(|r| location_target(input, job, r))
            .or_else(|| {
                input
                    .project
                    .locations
                    .iter()
                    .find(|l| {
                        l.id == job.outputs[fail[0]].location
                            && matches!(l.purpose, LocationPurpose::Isolation)
                    })
                    .map(|l| l.id.clone())
            });
        let isolation: Vec<_> = job
            .outputs
            .iter()
            .enumerate()
            .filter(|(_, o)| Some(&o.location) == isolation_location.as_ref())
            .map(|(i, _)| i)
            .collect();
        if isolation.len() != 1
            || isolation_location.as_ref().is_none_or(|id| {
                !input.project.locations.iter().any(|l| {
                    l.id == *id
                        && matches!(l.purpose, LocationPurpose::Isolation)
                        && !l.ordinary_issue_enabled
                })
            })
        {
            issues.push(issue(owner,"/quality_and_routes/isolation_location_role","quality_isolation_missing","Exhausted or unavailable rework requires one explicitly bound non-issuable isolation destination and output."));
            continue;
        }
        let pending = if matches!(rule.pending_policy, PendingPolicy::PendingStore) {
            let target = rule
                .pending_location_role
                .as_ref()
                .and_then(|r| location_target(input, job, r));
            if target.as_ref().is_none_or(|id| {
                !input.project.locations.iter().any(|l| {
                    l.id == *id
                        && matches!(l.purpose, LocationPurpose::Pending)
                        && l.accepted_item_types.contains(&job.inputs[0].item_type.id)
                        && l.capacity.as_ref().is_some_and(|q| {
                            q.unit == job.inputs[0].unit && q.value.decimal_places.value() == 0
                        })
                })
            }) {
                issues.push(issue(owner,"/quality_and_routes/pending_location_role","quality_pending_store_missing","Pending-store policy needs an explicit real pending location, accepted type, matching unit and finite capacity."));
                continue;
            }
            target
        } else {
            None
        };
        let rework: Vec<_> = jobs
            .iter()
            .filter(|r| {
                matches!(r.op().builtin, RuleKind::Rework)
                    && r.task == job.task
                    && r.cycle == job.cycle
                    && r.inputs.len() == 1
                    && r.outputs.len() == 1
                    && r.inputs[0].location == job.outputs[fail[0]].location
                    && r.outputs[0].location == job.inputs[0].location
                    && same_type(&r.inputs[0].item_type, &job.inputs[0].item_type)
                    && r.inputs[0].amount == job.inputs[0].amount
                    && r.outputs[0].amount == job.inputs[0].amount
            })
            .map(|r| r.id.clone())
            .collect();
        if rework.len() > 1 {
            issues.push(issue(owner,"/quality_and_routes/fail_port","quality_rework_ambiguous","Multiple rework routes claim the same failed item; no route is chosen by module name or vector order."));
            continue;
        }
        if let Some(reference) = &rule.result_ref {
            let sources: Vec<_> = input
                .scenario
                .quality_presets
                .iter()
                .filter(|p| p.id == *reference)
                .collect();
            if matches!(rule.source_kind, QualitySourceKind::Preset)
                && (sources.len() != 1
                    || !same_module(&sources[0].inspection_module, &job.operation_ref))
            {
                issues.push(issue(owner,"/quality_and_routes/result_ref","quality_source_reference","Inspection preset reference must name exactly one source for this exact inspection module."));
                continue;
            }
        }
        policies.push((
            index,
            Policy {
                rule: rule.clone(),
                pass: pass[0],
                fail: fail[0],
                isolation: isolation[0],
                rework: rework.first().cloned(),
                pending,
            },
        ));
    }
    for (index, policy) in policies {
        jobs[index].quality = Some(policy);
    }
    let mut preset_ids = BTreeSet::new();
    let mut matches = BTreeSet::new();
    for (i, preset) in input.scenario.quality_presets.iter().enumerate() {
        let owner = ObjectRef {
            kind: ObjectKind::Scenario,
            id: id(input.scenario.id.as_str()),
        };
        let selected = input.definitions.iter().any(|d| {
            d.id == preset.inspection_module.id
                && d.version == preset.inspection_module.version
                && match &d.payload {
                    ModulePayload::Operations(op) => {
                        matches!(op.builtin, RuleKind::InspectAndRoute)
                    }
                    ModulePayload::Devices(device) => device
                        .behavior
                        .inline
                        .as_ref()
                        .is_some_and(|op| matches!(op.builtin, RuleKind::InspectAndRoute)),
                    _ => false,
                }
        });
        if !selected {
            issues.push(issue(owner.clone(),format!("/quality_presets/{i}/inspection_module"),"quality_preset_module","A declared preset must refer to an available exact inspection definition; wrong versions or categories are not silently used."));
        }
        let key = (
            preset.inspection_module.id.clone(),
            String::from(preset.inspection_module.version.clone()),
            format!("{:?}", preset.match_kind),
            preset.item_or_ancestor.clone(),
        );
        if !preset_ids.insert(preset.id.clone()) || !matches.insert(key) {
            issues.push(issue(
                owner,
                format!("/quality_presets/{i}"),
                "quality_preset_ambiguous",
                "Inspection preset identity or matching scope is duplicated.",
            ));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Pass,
    Fail,
    Unknown,
}
pub(crate) fn outcome(value: Option<&TypedValue>) -> Outcome {
    match value {
        Some(TypedValue::Enum(v) | TypedValue::String(v)) if v.eq_ignore_ascii_case("pass") => {
            Outcome::Pass
        }
        Some(TypedValue::Enum(v) | TypedValue::String(v)) if v.eq_ignore_ascii_case("fail") => {
            Outcome::Fail
        }
        Some(TypedValue::Boolean(true)) => Outcome::Pass,
        Some(TypedValue::Boolean(false)) => Outcome::Fail,
        _ => Outcome::Unknown,
    }
}
