use serde::Serialize;
use serde_json::{Value, json};
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type FrozenInput = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);
pub(crate) fn value<T: Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("shared serializable type")
}
pub(crate) fn same<T: Serialize>(a: &T, b: &T) -> bool {
    value(a) == value(b)
}
pub(crate) fn issue(
    code: &str,
    message: impl Into<String>,
    object_ref: Option<ObjectRef>,
) -> Issue {
    Issue {
        code: code.into(),
        message: message.into(),
        object_ref,
        field_path: None,
        blocks: vec![CapabilityPurpose::CompleteConclusion],
    }
}
fn object(kind: ObjectKind, id: &str) -> ObjectRef {
    ObjectRef {
        kind,
        id: ObjectId::try_from(id.to_owned()).expect("existing opaque ID"),
    }
}
fn sorted<T: Serialize>(rows: &[T]) -> Vec<String> {
    let mut v: Vec<_> = rows.iter().map(|r| value(r).to_string()).collect();
    v.sort();
    v
}
/// Actual Engine order mode and saved-goal mode are distinct. A goal never
/// manufactures an OrderLine; the frozen Scenario is the manual-goal source.
pub(crate) fn saved_goal(i: &FrozenInput) -> Option<&FactoryGenerationGoal> {
    if !i.1.orders.is_empty() {
        return None;
    }
    i.1.acceptance_goal
        .as_ref()
        .or_else(|| i.0.generation_context.as_ref().map(|context| &context.goal))
}
pub(crate) fn declared_scopes(i: &FrozenInput) -> Vec<AcceptanceScope> {
    if let Some(goal) = saved_goal(i) {
        return std::iter::once(&goal.acceptance_scope)
            .chain(goal.additional_acceptance_scopes.iter())
            .cloned()
            .collect();
    }
    i.1.orders
        .iter()
        .filter_map(|o| {
            o.quantity.as_ref().map(|q| AcceptanceScope {
                product_ref: o.product_ref.clone(),
                unit: q.unit.clone(),
                destination: o.destination.clone(),
                stage: o.acceptance_stage.clone(),
            })
        })
        .collect()
}
fn acceptance_signature(i: &FrozenInput) -> Vec<String> {
    let mut v: Vec<_> = declared_scopes(i)
        .iter()
        .map(|s| value(s).to_string())
        .collect();
    v.sort();
    v.dedup();
    v
}
fn demand_signature(i: &FrozenInput) -> Value {
    if let Some(goal) = saved_goal(i) {
        json!({"mode":"saved_goal", "quantity":goal.accepted_quantity_per_day,
            "window":goal.observation_window, "calendar":goal.production_calendar_ref})
    } else {
        json!({"mode":"orders", "orders":sorted(&i.1.orders)})
    }
}

/// Host supplies each Run's own immutable Project, Scenario and exact definitions.
pub fn compare_runs(
    inputs: &[(
        ProjectState,
        ScenarioInput,
        Vec<ModuleDefinition>,
        RunResult,
    )],
    keys: &[MetricKey],
    objects: &[ObjectRef],
    acceptance: &[AcceptanceScope],
) -> Result<RunComparison, Vec<Issue>> {
    if inputs.len() < 2 {
        return Err(vec![issue(
            "comparison_inputs",
            "Select at least two actual Runs",
            None,
        )]);
    }
    let mut ids = BTreeSet::new();
    for (p, s, _, r) in inputs {
        let f = &r.input_ref;
        if !ids.insert(r.id.as_str())
            || p.id != f.project_id
            || !same(&p.revision, &f.project_revision)
            || s.id != f.scenario_id
            || !same(&s.revision, &f.scenario_revision)
            || !same(&s.project_revision, &f.project_revision)
            || !matches!(r.evidence_origin, EvidenceOrigin::ActualSoftware)
            || r.window.start_ms.0.value() >= r.window.end_ms.0.value()
        {
            return Err(vec![issue(
                "comparison_frozen_identity",
                "Unique actual Runs with matching frozen identities and positive windows are required",
                Some(object(ObjectKind::Run, r.id.as_str())),
            )]);
        }
    }
    let mut pairs = vec![];
    let mut limitations: Vec<Issue> = inputs
        .iter()
        .flat_map(|i| i.0.issues.iter().chain(&i.1.issues).chain(&i.3.limitations))
        .cloned()
        .collect();
    for b in &inputs[1..] {
        let a = &inputs[0];
        let changes = input_changes(a, b);
        let conflicting_definition = a.2.iter().any(|left| {
            b.2.iter().any(|right| {
                left.id == right.id && same(&left.version, &right.version) && !same(left, right)
            })
        });
        let complete = |i: &FrozenInput| {
            i.3.whole_window_computed
                && matches!(i.3.computation_status, ComputationStatus::Completed)
                && i.3
                    .computed_until_ms
                    .as_ref()
                    .is_some_and(|t| t.0.value() >= i.3.window.end_ms.0.value())
                && !i
                    .0
                    .issues
                    .iter()
                    .chain(&i.1.issues)
                    .chain(&i.3.limitations)
                    .any(|e| {
                        e.blocks
                            .iter()
                            .any(|b| matches!(b, CapabilityPurpose::Execution))
                    })
                && i.0.module_refs.iter().all(|r| {
                    i.2.iter()
                        .any(|d| d.id == r.id && same(&d.version, &r.version))
                })
        };
        let c = |key, matches, text: &str| ComparisonCondition {
            key,
            matches,
            issues: if matches {
                vec![]
            } else {
                vec![issue("comparison_condition", text, None)]
            },
        };
        let conditions = vec![
            c(
                ComparisonConditionKey::TimeReference,
                same(&a.1.time_reference, &b.1.time_reference),
                "Different time references",
            ),
            c(
                ComparisonConditionKey::ObservationWindow,
                same(&a.3.window, &b.3.window),
                "Different actual observation windows",
            ),
            c(
                ComparisonConditionKey::AcceptanceScope,
                acceptance_signature(a) == acceptance_signature(b),
                "Different product revision/unit/destination/acceptance stage",
            ),
            c(
                ComparisonConditionKey::Demand,
                demand_signature(a) == demand_signature(b),
                "Different order demand/release/due conditions or saved acceptance targets",
            ),
            c(
                ComparisonConditionKey::SelectedScope,
                sorted(&a.3.scope) == sorted(&b.3.scope),
                "Different selected execution scopes",
            ),
            c(
                ComparisonConditionKey::InitialConditions,
                a.0.id == b.0.id && same(&a.1.initial_conditions, &b.1.initial_conditions),
                "Initial stock/setup/quality/allocation differs; review explicit changes",
            ),
            c(
                ComparisonConditionKey::SupportedCapabilities,
                complete(a) && complete(b) && !conflicting_definition,
                "Full-window execution/capabilities incomplete or immutable same-ID/version definitions conflict; known prefixes remain readable",
            ),
            c(
                ComparisonConditionKey::ExplainedChanges,
                true,
                "Complete frozen input differences listed",
            ),
        ];
        let hard = a.0.id == b.0.id
            && conditions
                .iter()
                .filter(|c| {
                    !matches!(
                        c.key,
                        ComparisonConditionKey::InitialConditions
                            | ComparisonConditionKey::ExplainedChanges
                    )
                })
                .all(|c| c.matches);
        let status = if !hard {
            ComparisonStatus::SideBySideOnly
        } else if changes.is_empty() {
            ComparisonStatus::Strict
        } else {
            ComparisonStatus::ControlledChanges
        };
        limitations.extend(conditions.iter().flat_map(|c| c.issues.clone()));
        pairs.push(RunPairComparison {
            baseline_run_id: a.3.id.clone(),
            candidate_run_id: b.3.id.clone(),
            status,
            conditions,
            input_changes: changes,
        });
    }
    let metrics = crate::metrics::build(inputs, keys, objects, acceptance, &pairs);
    let conclusions = pairs
        .iter()
        .map(|p| match p.status {
            ComparisonStatus::Strict => format!(
                "{} / {}: same frozen business conditions; actual results can be compared",
                p.baseline_run_id.as_str(),
                p.candidate_run_id.as_str()
            ),
            ComparisonStatus::ControlledChanges => format!(
                "{} / {}: controlled comparison; review {} explicit input changes first",
                p.baseline_run_id.as_str(),
                p.candidate_run_id.as_str(),
                p.input_changes.len()
            ),
            ComparisonStatus::SideBySideOnly => format!(
                "{} / {}: side-by-side evidence only; no improvement claim",
                p.baseline_run_id.as_str(),
                p.candidate_run_id.as_str()
            ),
        })
        .collect();
    Ok(RunComparison {
        run_ids: inputs.iter().map(|i| i.3.id.clone()).collect(),
        pairs,
        metrics,
        conclusions,
        limitations,
    })
}

fn input_changes(a: &FrozenInput, b: &FrozenInput) -> Vec<InputChange> {
    let mut out = vec![];
    let pa = value(&a.0);
    let pb = value(&b.0);
    let sa = value(&a.1);
    let sb = value(&b.1);
    for (field, variant, kind) in [
        ("item_types", "item_type", ObjectKind::ItemType),
        ("locations", "location", ObjectKind::Location),
        ("halls", "hall", ObjectKind::Hall),
        ("lines", "line", ObjectKind::Line),
        ("devices", "device", ObjectKind::Device),
        ("resources", "resource", ObjectKind::Resource),
        ("calendars", "calendar", ObjectKind::Calendar),
        ("products", "product", ObjectKind::Product),
        ("boms", "bom", ObjectKind::Bom),
        ("routes", "route", ObjectKind::Route),
        ("connections", "connection", ObjectKind::Connection),
        ("bindings", "binding", ObjectKind::Artifact),
        ("setup_policies", "setup_policy", ObjectKind::Device),
        ("sources", "source_record", ObjectKind::Project),
        ("module_refs", "module_ref", ObjectKind::Module),
    ] {
        diff_rows(
            &mut out,
            &pa[field],
            &pb[field],
            &format!("project/{field}"),
            variant,
            kind,
            &object(ObjectKind::Project, b.0.id.as_str()),
        );
    }
    for (field, variant, kind) in [
        ("arrivals", "arrival", ObjectKind::Location),
        ("orders", "order", ObjectKind::Order),
        ("production_tasks", "task", ObjectKind::Task),
        ("supplies", "supply", ObjectKind::Supply),
        (
            "parameter_overrides",
            "parameter_override",
            ObjectKind::Project,
        ),
        ("cost_inputs", "cost_input", ObjectKind::Project),
    ] {
        diff_rows(
            &mut out,
            &sa[field],
            &sb[field],
            &format!("scenario/{field}"),
            variant,
            kind,
            &object(ObjectKind::Scenario, b.1.id.as_str()),
        );
    }
    diff_rows(
        &mut out,
        &sa["initial_conditions"]["items"],
        &sb["initial_conditions"]["items"],
        "scenario/initial_conditions/items",
        "initial_item",
        ObjectKind::Item,
        &object(ObjectKind::Scenario, b.1.id.as_str()),
    );
    for field in [
        "time_reference",
        "parameter_sources",
        "acknowledgments",
        "generation_context",
        "issues",
        "template_adoptions",
    ] {
        diff_scalar(
            &mut out,
            &pa[field],
            &pb[field],
            &format!("project/{field}"),
            &object(ObjectKind::Project, a.0.id.as_str()),
        );
    }
    for field in [
        "time_reference",
        "window",
        "quality_presets",
        "acceptance_goal",
        "scope",
        "issues",
    ] {
        diff_scalar(
            &mut out,
            &sa[field],
            &sb[field],
            &format!("scenario/{field}"),
            &object(ObjectKind::Scenario, a.1.id.as_str()),
        );
    }
    for field in [
        "initial_setup_families",
        "declared_quality_decisions",
        "material_allocations",
    ] {
        diff_scalar(
            &mut out,
            &sa["initial_conditions"][field],
            &sb["initial_conditions"][field],
            &format!("scenario/initial_conditions/{field}"),
            &object(ObjectKind::Scenario, a.1.id.as_str()),
        );
    }
    diff_rows(
        &mut out,
        &value(&a.2),
        &value(&b.2),
        "definitions",
        "definition",
        ObjectKind::Module,
        &object(ObjectKind::Project, b.0.id.as_str()),
    );
    out
}
fn row_key(v: &Value, n: usize) -> String {
    v.get("id")
        .or_else(|| v.get("device_id"))
        .or_else(|| v.get("data").and_then(|d| d.get("id")))
        .and_then(Value::as_str)
        .map(|id| {
            if let Some(version) = v.get("version") {
                format!("{id}@{version}")
            } else {
                id.to_owned()
            }
        })
        .unwrap_or_else(|| {
            if v.get("target").is_some() {
                json!([v["target"], v["path"]]).to_string()
            } else {
                format!("row-{n}")
            }
        })
}
fn rows(v: &Value) -> BTreeMap<String, Value> {
    v.as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(n, v)| (row_key(v, n), v.clone()))
        .collect()
}
fn diff_rows(
    out: &mut Vec<InputChange>,
    a: &Value,
    b: &Value,
    path: &str,
    variant: &str,
    kind: ObjectKind,
    owner: &ObjectRef,
) {
    let aa = rows(a);
    let bb = rows(b);
    let keys: BTreeSet<_> = aa.keys().chain(bb.keys()).cloned().collect();
    for key in keys {
        let before = aa.get(&key);
        let after = bb.get(&key);
        if before == after {
            continue;
        }
        let typed = |v: Option<&Value>| {
            v.map(|data| {
                serde_json::from_value(json!({"kind":variant,"data":data}))
                    .expect("matching shared change variant")
            })
        };
        let row = after.or(before).expect("changed row exists");
        let target = match variant {
            "arrival" | "source_record" => owner.clone(),
            "parameter_override" => {
                serde_json::from_value(row["target"].clone()).expect("typed override target")
            }
            "cost_input" => {
                serde_json::from_value(row["object_ref"].clone()).expect("typed cost target")
            }
            "binding" => {
                serde_json::from_value(row["consumer_ref"].clone()).expect("typed binding consumer")
            }
            "setup_policy" => object(
                ObjectKind::Device,
                row["device_id"].as_str().expect("typed policy device"),
            ),
            "connection" => object(
                ObjectKind::Connection,
                row["data"]["id"].as_str().expect("typed connection ID"),
            ),
            _ => object(kind.clone(), row["id"].as_str().expect("typed object ID")),
        };
        out.push(InputChange {
            target,
            path: format!("/{path}/{}", key.replace('~', "~0").replace('/', "~1")),
            kind: if before.is_none() {
                InputChangeKind::Added
            } else if after.is_none() {
                InputChangeKind::Removed
            } else {
                InputChangeKind::Changed
            },
            before: typed(before),
            after: typed(after),
            basis: Some(SourceBasis::Unknown),
            reason: Some("Exact before/after values from each Run's frozen inputs".into()),
        });
    }
}
fn diff_scalar(out: &mut Vec<InputChange>, a: &Value, b: &Value, path: &str, target: &ObjectRef) {
    if a == b {
        return;
    }
    let typed = |v: &Value| {
        Some(InputChangeValue::Scalar {
            value: TypedValue::String(v.to_string()),
            unit: None,
        })
    };
    out.push(InputChange {
        target: target.clone(),
        path: format!("/{path}"),
        kind: InputChangeKind::Changed,
        before: typed(a),
        after: typed(b),
        basis: Some(SourceBasis::Unknown),
        reason: Some("JSON text representation of complete frozen field; no value inferred".into()),
    });
}
