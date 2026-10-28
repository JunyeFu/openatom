//! A read-only projection of the same actual, frozen Engine inputs used by comparison.
use crate::comparison::{FrozenInput, declared_scopes, issue, same, value};
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn problem(code: &str, text: impl Into<String>) -> Issue {
    issue(&format!("evaluation.{code}"), text, None)
}
pub(crate) fn number(n: f64) -> Option<FiniteNumber> {
    FiniteNumber::try_from(n).ok()
}
pub(crate) fn amount(q: &Quantity) -> f64 {
    q.value.coefficient.value() as f64 / 10f64.powi(q.value.decimal_places.value() as i32)
}
fn scaled(q: &Quantity) -> u128 {
    q.value.coefficient.value() as u128 * 10u128.pow(9 - q.value.decimal_places.value() as u32)
}
fn exact_quantity(mut n: u128, unit: &UnitId) -> Option<Quantity> {
    let mut p = 9u8;
    while p > 0 && n % 10 == 0 {
        n /= 10;
        p -= 1;
    }
    Some(Quantity {
        value: DecimalAmount {
            coefficient: SafeUint::try_from(u64::try_from(n).ok()?).ok()?,
            decimal_places: DecimalPlaces::try_from(p).ok()?,
        },
        unit: unit.clone(),
    })
}
fn safe(n: u128) -> Option<SafeUint> {
    SafeUint::try_from(u64::try_from(n).ok()?).ok()
}
fn dur(n: u128) -> DurationMs {
    DurationMs(safe(n).expect("bounded Run duration"))
}
pub(crate) fn range(i: &FrozenInput) -> (u64, u64) {
    (
        i.3.window.start_ms.0.value(),
        i.3.computed_until_ms
            .as_ref()
            .map_or(i.3.window.start_ms.0.value(), |v| {
                v.0.value().min(i.3.window.end_ms.0.value())
            }),
    )
}
fn clip(t: &IntervalMs, a: u64, b: u64) -> Option<(u64, u64)> {
    let x = t.start_ms.0.value().max(a);
    let y = t.end_ms.0.value().min(b);
    (x < y).then_some((x, y))
}
pub(crate) fn union(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.sort_unstable();
    let mut out: Vec<(u64, u64)> = vec![];
    for (a, b) in ranges {
        if a >= b {
            continue;
        }
        if let Some(last) = out.last_mut() {
            if a <= last.1 {
                last.1 = last.1.max(b);
                continue;
            }
        }
        out.push((a, b));
    }
    out
}
fn duration(v: &[(u64, u64)]) -> u128 {
    v.iter().map(|(a, b)| (b - a) as u128).sum()
}
fn intersections(a: &[(u64, u64)], b: &[(u64, u64)]) -> Vec<(u64, u64)> {
    union(
        a.iter()
            .flat_map(|&(x, y)| {
                b.iter().filter_map(move |&(z, w)| {
                    let s = x.max(z);
                    let e = y.min(w);
                    (s < e).then_some((s, e))
                })
            })
            .collect(),
    )
}
fn resources(i: &FrozenInput) -> Result<Vec<ResourceMetric>, Vec<Issue>> {
    let (start, end) = range(i);
    let mut unique = BTreeMap::new();
    for claim in &i.3.resource_claims {
        if claim.intervals.len() != claim.interval_end_causes.len() {
            return Err(vec![problem(
                "invalid_context",
                "Claim interval/end-cause arrays differ",
            )]);
        }
        if let Some(old) = unique.insert(claim.id.as_str(), claim) {
            if !same(old, claim) {
                return Err(vec![problem(
                    "invalid_context",
                    "Conflicting repeated Claim identity",
                )]);
            }
        }
    }
    let mut rows = vec![];
    for resource in &i.0.resources {
        let claims: Vec<_> = unique
            .values()
            .filter(|c| c.resource_id == resource.id)
            .copied()
            .collect();
        let open: Option<Vec<_>> =
            i.0.calendars
                .iter()
                .find(|c| c.id == resource.calendar_id)
                .map(|c| {
                    union(
                        c.intervals
                            .iter()
                            .filter_map(|t| clip(t, start, end))
                            .collect(),
                    )
                });
        let mut held = vec![];
        let mut integral = 0u128;
        let mut open_integral = 0u128;
        let mut work = vec![];
        let mut waits = vec![];
        let mut work_known = true;
        for c in &claims {
            let ranges = union(
                c.intervals
                    .iter()
                    .filter_map(|t| clip(t, start, end))
                    .collect(),
            );
            integral += duration(&ranges) * c.units.value() as u128;
            held.extend(ranges.clone());
            if let Some(open) = &open {
                open_integral += duration(&intersections(&ranges, open)) * c.units.value() as u128;
            }
            if matches!(c.kind, ResourceClaimKind::Transport) {
                work_known = false;
            }
            if i.3
                .jobs
                .iter()
                .any(|j| j.id == c.job_id && j.effective_progress_ms.0.value() > 0)
                && !i
                    .3
                    .phase_work_intervals
                    .iter()
                    .any(|w| w.job_id == c.job_id)
            {
                work_known = false;
            }
            for w in i.3.phase_work_intervals.iter().filter(|w| {
                w.job_id == c.job_id && c.phase_ref.as_ref().is_none_or(|p| p == &w.phase_id)
            }) {
                if let Some(t) = clip(&w.interval, start, end) {
                    work.extend(intersections(&ranges, &[t]));
                }
            }
            for w in &i.3.waits {
                // Two concurrent Jobs may share the same device/step. Its
                // execution reference alone does not attribute another Job's wait.
                let matches_job =
                    matches!(w.object_ref.kind, ObjectKind::Job) && w.object_ref.id == c.job_id;
                if matches_job
                    && c.phase_ref
                        .as_ref()
                        .is_none_or(|p| w.phase_ref.as_ref().is_none_or(|wp| p == wp))
                {
                    if let Some(t) = clip(&w.interval, start, end) {
                        waits.extend(intersections(&ranges, &[t]));
                    }
                }
            }
        }
        let occupied = safe(integral).ok_or_else(|| {
            vec![problem(
                "invalid_context",
                "Capacity-time exceeds shared exact representation",
            )]
        })?;
        let available = open.as_ref().and_then(|o| {
            let n = duration(o) * scaled(&resource.capacity);
            (n % 1_000_000_000 == 0)
                .then(|| safe(n / 1_000_000_000))
                .flatten()
        });
        let mut missing = vec![];
        if available.is_none() {
            missing.push(problem(
                "resource_calendar",
                "Resource calendar/capacity integral is unavailable or not exactly representable",
            ));
        }
        if available.as_ref().is_some_and(|v| v.value() == 0) {
            missing.push(problem(
                "resource_denominator",
                "日历可用容量积分为0；利用率未知，不填0",
            ));
        }
        if !work_known {
            missing.push(problem(
                "resource_work",
                "缺少可归属的有效阶段工作历史；运输持有等不能冒充加工时间",
            ));
        }
        let open_ms = open.as_ref().and_then(|_| safe(open_integral));
        let closed_ms = open.as_ref().and_then(|_| safe(integral - open_integral));
        rows.push(ResourceMetric {
            resource_id: resource.id.clone(),
            any_busy_ms: dur(duration(&union(held))),
            occupied_capacity_ms: occupied,
            available_capacity_ms: available.clone(),
            utilization: available
                .as_ref()
                .filter(|v| v.value() > 0)
                .and_then(|v| number(open_integral as f64 / v.value() as f64)),
            effective_work_ms: work_known.then(|| dur(duration(&union(work)))),
            waiting_ms: Some(dur(duration(&union(waits)))),
            capacity_unit: resource.capacity.unit.clone(),
            open_calendar_occupied_capacity_ms: open_ms,
            closed_calendar_held_capacity_ms: closed_ms,
            missing_conditions: missing,
        });
    }
    Ok(rows)
}
fn object_item(_i: &FrozenInput, o: &ObjectRef, item: &RuntimeItem) -> bool {
    match o.kind {
        ObjectKind::Item => o.id == item.id,
        ObjectKind::ItemType => o.id.as_str() == item.item_type_ref.id.as_str(),
        _ => false,
    }
}
fn measured(i: &FrozenInput, c: &CostInput, rs: &[ResourceMetric]) -> Option<Quantity> {
    let (start, end) = range(i);
    let exists = match c.object_ref.kind {
        ObjectKind::Item => i.3.final_items.iter().any(|t| t.id == c.object_ref.id),
        ObjectKind::ItemType => {
            i.0.item_types
                .iter()
                .any(|t| t.id.as_str() == c.object_ref.id.as_str())
        }
        ObjectKind::Resource => i.0.resources.iter().any(|r| r.id == c.object_ref.id),
        ObjectKind::Transport => i.3.transport.iter().any(|t| t.id == c.object_ref.id),
        ObjectKind::Supply => i.1.supplies.iter().any(|s| s.id == c.object_ref.id),
        _ => false,
    };
    if !exists {
        return None;
    }
    let ms = |n: SafeUint, capacity_unit: &UnitId| {
        Some(Quantity {
            value: DecimalAmount {
                coefficient: n,
                decimal_places: DecimalPlaces::try_from(0u8).unwrap(),
            },
            unit: UnitId::try_from(format!("{}_ms", capacity_unit.as_str())).ok()?,
        })
    };
    match c.basis {
        CostBasis::OccupiedTime | CostBasis::PaidCalendar => {
            if !matches!(c.object_ref.kind, ObjectKind::Resource) {
                return None;
            }
            let r = rs.iter().find(|r| r.resource_id == c.object_ref.id)?;
            if matches!(c.basis, CostBasis::PaidCalendar) {
                // Paid-calendar pricing is the declared paid window, independent
                // of actual occupancy and the amount of Engine work completed.
                let resource = i.0.resources.iter().find(|s| s.id == c.object_ref.id)?;
                let calendar =
                    i.0.calendars
                        .iter()
                        .find(|s| s.id == resource.calendar_id)?;
                let open = union(
                    calendar
                        .intervals
                        .iter()
                        .filter_map(|t| {
                            clip(
                                t,
                                i.3.window.start_ms.0.value(),
                                i.3.window.end_ms.0.value(),
                            )
                        })
                        .collect(),
                );
                let n = duration(&open) * scaled(&resource.capacity);
                if n % 1_000_000_000 != 0 {
                    return None;
                }
                ms(safe(n / 1_000_000_000)?, &r.capacity_unit)
            } else {
                ms(r.occupied_capacity_ms.clone(), &r.capacity_unit)
            }
        }
        CostBasis::ConsumedMaterial => {
            let ids: BTreeSet<_> =
                i.3.jobs
                    .iter()
                    // Phase consumption can precede delivery/return or remain
                    // held at the output. The frozen transformed item, rather
                    // than whole-Job completion, establishes material consumed.
                    .flat_map(|j| j.input_refs.iter())
                    .collect();
            let items: Vec<_> =
                i.3.final_items
                    .iter()
                    .filter(|t| {
                        ids.contains(&t.id)
                            && object_item(i, &c.object_ref, t)
                            && matches!(t.lifecycle, Lifecycle::Transformed)
                    })
                    .collect();
            if !matches!(c.object_ref.kind, ObjectKind::Item | ObjectKind::ItemType) {
                return None;
            }
            if items
                .iter()
                .any(|t| t.quantity.unit != c.rate_quantity.unit)
            {
                return None;
            }
            exact_quantity(
                items.iter().map(|t| scaled(&t.quantity)).sum(),
                &c.rate_quantity.unit,
            )
        }
        CostBasis::ExplicitDisposal => {
            let ids: BTreeSet<_> =
                i.3.events
                    .iter()
                    .filter(|e| e.at_ms.0.value() >= start && e.at_ms.0.value() <= end)
                    .filter_map(|e| match &e.payload {
                        SimulationEventPayload::QualityChanged(d)
                            if matches!(d.disposition, QualityDisposition::Scrap) =>
                        {
                            Some(&d.item_ref)
                        }
                        _ => None,
                    })
                    .collect();
            if !matches!(c.object_ref.kind, ObjectKind::Item | ObjectKind::ItemType) {
                return None;
            }
            let items: Vec<_> =
                i.3.final_items
                    .iter()
                    .filter(|t| ids.contains(&t.id) && object_item(i, &c.object_ref, t))
                    .collect();
            if items
                .iter()
                .any(|t| t.quantity.unit != c.rate_quantity.unit)
            {
                return None;
            }
            exact_quantity(
                items.iter().map(|t| scaled(&t.quantity)).sum(),
                &c.rate_quantity.unit,
            )
        }
        CostBasis::BatchTransport => {
            let mut seen = BTreeSet::new();
            let n =
                i.3.transport
                    .iter()
                    .filter(|t| {
                        t.departed_ms
                            .as_ref()
                            .is_some_and(|at| at.0.value() >= start && at.0.value() <= end)
                            && seen.insert(&t.id)
                            && match c.object_ref.kind {
                                ObjectKind::Transport => t.id == c.object_ref.id,
                                ObjectKind::Supply => t.supply_ref == c.object_ref.id,
                                ObjectKind::Resource => t.carrier_id == c.object_ref.id,
                                _ => false,
                            }
                    })
                    .count();
            if !matches!(
                c.object_ref.kind,
                ObjectKind::Transport | ObjectKind::Supply | ObjectKind::Resource
            ) {
                return None;
            }
            exact_quantity(n as u128 * 1_000_000_000, &c.rate_quantity.unit)
        }
    }
}
fn costs(
    i: &FrozenInput,
    rs: &[ResourceMetric],
    accepted: &[(AcceptanceScope, Quantity)],
) -> CostEvaluation {
    let mut lines = vec![];
    let mut unassigned = vec![];
    let mut missing = vec![];
    let mut unique = BTreeMap::new();
    let mut conflicting = BTreeSet::new();
    for c in &i.1.cost_inputs {
        if let Some(previous) = unique.insert(c.id.as_str(), c) {
            if !same(previous, c) {
                conflicting.insert(c.id.as_str());
            }
        }
    }
    let mut seen = BTreeSet::new();
    for c in
        i.1.cost_inputs
            .iter()
            .filter(|c| seen.insert(c.id.as_str()))
    {
        let mut gaps = vec![];
        if conflicting.contains(c.id.as_str()) {
            gaps.push(problem(
                "cost_identity",
                "相同成本输入ID存在冲突值；保持原输入记录，金额未知",
            ));
        }
        let measurement = measured(i, c, rs);
        if measurement.is_none() {
            gaps.push(problem(
                "cost_measurement",
                "No actual measured basis for this cost object/unit",
            ));
        }
        let conflict = i.1.cost_inputs.iter().any(|other| {
            other.id != c.id
                && same(&other.object_ref, &c.object_ref)
                && (c.component == other.component
                    || c.includes.contains(&other.component)
                    || other.includes.contains(&c.component))
        });
        if conflict {
            gaps.push(problem(
                "cost_overlap",
                "同一成本对象的班次/占用方式或已含组成项重叠，必须明确选择，未相加",
            ));
        }
        let rate = c.rate.as_ref();
        if rate.is_none() {
            gaps.push(problem("cost_rate", "费率未填写；未知不补零"));
        }
        if c.currency.as_ref().is_none_or(|v| v.is_empty()) {
            gaps.push(problem("cost_currency", "币种未填写"));
            unassigned.push(c.id.clone());
        }
        let units_match = measurement.as_ref().is_some_and(|q| {
            q.unit == c.rate_quantity.unit && c.rate_quantity.value.coefficient.value() > 0
        });
        if !units_match {
            gaps.push(problem(
                "cost_unit",
                "计量与费率分母单位不匹配，不能猜测换算",
            ));
        }
        let charge = if gaps.is_empty() {
            rate.zip(measurement.as_ref())
                .and_then(|(rate, q)| crate::exact_cost::charge(rate, q, &c.rate_quantity))
        } else {
            None
        };
        if charge.is_none() && gaps.is_empty() {
            gaps.push(problem("cost_overflow", "金额不可表示"));
        }
        missing.extend(gaps.clone());
        lines.push(ConditionalCost {
            cost_input_id: c.id.clone(),
            object_ref: c.object_ref.clone(),
            basis: c.basis.clone(),
            measured_quantity: measurement,
            amount: charge,
            currency: c.currency.clone(),
            missing_conditions: gaps,
        });
    }
    let currencies: BTreeSet<_> = lines
        .iter()
        .filter_map(|l| l.currency.clone())
        .filter(|s| !s.is_empty())
        .collect();
    let currencies = currencies
        .into_iter()
        .map(|currency| {
            let group: Vec<_> = lines
                .iter()
                .filter(|l| l.currency.as_ref() == Some(&currency))
                .collect();
            let denominator = (accepted.len() == 1).then(|| accepted[0].clone());
            let (sum, exact_unit_cost) = crate::exact_cost::sum_charges(
                group.iter().filter(|l| l.amount.is_some()).map(|l| {
                    let input = unique[l.cost_input_id.as_str()];
                    (
                        input.rate.as_ref().unwrap(),
                        l.measured_quantity.as_ref().unwrap(),
                        &input.rate_quantity,
                    )
                }),
                denominator.as_ref().map(|v| &v.1),
            );
            let mut gaps: Vec<_> = group
                .iter()
                .flat_map(|l| l.missing_conditions.clone())
                .collect();
            if !unassigned.is_empty() {
                gaps.push(problem(
                    "cost_unassigned_currency",
                    "存在未分配币种成本项，完整总额未知",
                ));
            }
            if !i.3.whole_window_computed {
                gaps.push(problem("cost_prefix", "仅已计算前缀；完整窗口总成本未知"));
            }
            let total = gaps.is_empty().then(|| sum.clone());
            let unit_cost = if total.is_some() {
                exact_unit_cost
            } else {
                None
            };
            CurrencyCostSummary {
                currency,
                cost_input_ids: group.iter().map(|l| l.cost_input_id.clone()).collect(),
                known_subtotal: sum,
                complete_total: total,
                accepted_scope: denominator.as_ref().map(|v| v.0.clone()),
                accepted_quantity: denominator.map(|v| v.1),
                unit_cost,
                missing_conditions: gaps,
            }
        })
        .collect();
    if i.1.cost_inputs.is_empty() {
        missing.push(problem(
            "cost_inputs",
            "未提供价格与成本范围；没有完整总成本",
        ));
    }
    CostEvaluation {
        lines,
        currencies,
        unassigned_cost_input_ids: unassigned,
        inputs: i.1.cost_inputs.clone(),
        missing_conditions: missing,
    }
}

/// Context is selected and frozen by Host. This producer neither reads disk nor changes Engine state.
pub fn build_evaluation(
    mut context: EvaluationReport,
    inputs: &[(
        ProjectState,
        ScenarioInput,
        Vec<ModuleDefinition>,
        RunResult,
    )],
    keys: &[MetricKey],
    objects: &[ObjectRef],
    acceptance: &[AcceptanceScope],
) -> Result<Versioned<EvaluationReport>, Vec<Issue>> {
    if matches!(context.mode, ReportMode::DesignDescription) {
        if !inputs.is_empty() || !context.run_refs.is_empty() {
            return Err(vec![problem("invalid_context", "设计说明不能携带运行结果")]);
        }
        context.run_evaluations.clear();
        context.scenario_comparison = None;
        context
            .conclusions
            .push("本报告为设计说明，尚无实际运行；不包含实际产量、完工时间或改善量。".into());
    } else {
        if inputs.is_empty() {
            return Err(vec![problem("invalid_context", "模拟评估需要实际软件Run")]);
        }
        let mut seen = BTreeSet::new();
        let mut results = vec![];
        let mut modules = BTreeMap::new();
        let selected: BTreeSet<_> = context.run_refs.iter().map(|r| r.as_str()).collect();
        if !selected.is_empty()
            && (selected.len() != inputs.len()
                || inputs.iter().any(|i| !selected.contains(i.3.id.as_str())))
        {
            return Err(vec![problem("invalid_context", "所选Run与冻结输入不一致")]);
        }
        for i in inputs {
            let f = &i.3.input_ref;
            if !seen.insert(i.3.id.as_str())
                || i.0.id != context.project_id
                || f.project_id != i.0.id
                || !same(&f.project_revision, &i.0.revision)
                || f.scenario_id != i.1.id
                || !same(&f.scenario_revision, &i.1.revision)
                || !same(&i.1.project_revision, &f.project_revision)
                || !matches!(i.3.evidence_origin, EvidenceOrigin::ActualSoftware)
                || i.3.computed_until_ms.as_ref().is_none_or(|v| {
                    v.0.value() < i.3.window.start_ms.0.value()
                        || v.0.value() > i.3.window.end_ms.0.value()
                })
            {
                return Err(vec![problem(
                    "invalid_context",
                    "实际Run身份、冻结输入或计算边界不一致",
                )]);
            }
            let rs = resources(i)?;
            let actual_scopes = declared_scopes(i);
            // Reuse the exact existing metric producer, including saved manual goals.
            let metrics = crate::metrics::build(
                std::slice::from_ref(i),
                &[MetricKey::AcceptedQuantity, MetricKey::CompletionTime],
                &[],
                &actual_scopes,
                &[],
            );
            let accepted: Vec<_> = metrics
                .iter()
                .filter(|m| matches!(m.metric, MetricKey::AcceptedQuantity))
                .filter_map(|m| match (&m.acceptance_scope, &m.values[0].value) {
                    (Some(s), MetricValue::Quantity(q)) => Some((s.clone(), q.clone())),
                    _ => None,
                })
                .collect();
            for metric in &metrics {
                let text = match (&metric.metric, &metric.values[0].value) {
                    (MetricKey::AcceptedQuantity, MetricValue::Quantity(q)) => format!(
                        "实际接受量 A={} {}，精确基础值{}",
                        amount(q),
                        q.unit.as_str(),
                        serde_json::to_string(q).unwrap()
                    ),
                    (MetricKey::CompletionTime, MetricValue::Duration(t)) => format!(
                        "完整接受时刻 C={}ms（{}秒）",
                        t.0.value(),
                        t.0.value() as f64 / 1000.
                    ),
                    (MetricKey::CompletionTime, MetricValue::Unknown(e)) => {
                        format!("C=null（未知）：{}", e.message)
                    }
                    (_, MetricValue::Unknown(e)) => format!("接受量未知：{}", e.message),
                    _ => serde_json::to_string(&metric.values[0].value).unwrap(),
                };
                context.conclusions.push(format!(
                    "Run {} / 接受口径 {}：{}。",
                    i.3.id.as_str(),
                    serde_json::to_string(&metric.acceptance_scope).unwrap(),
                    text
                ));
            }
            let mut limitations: Vec<_> =
                i.0.issues
                    .iter()
                    .chain(&i.1.issues)
                    .chain(&i.3.limitations)
                    .cloned()
                    .collect();
            if !i.3.whole_window_computed {
                limitations.push(problem(
                    "partial_result",
                    "仅已计算前缀；未知尾项不补零，不形成整窗产能/达标结论",
                ));
            }
            for r in &i.0.module_refs {
                if !i
                    .2
                    .iter()
                    .any(|d| d.id == r.id && same(&d.version, &r.version))
                {
                    limitations.push(problem(
                        "missing_module",
                        format!("缺少精确定义 {} {}", r.id.as_str(), value(&r.version)),
                    ));
                }
                modules.insert(value(r).to_string(), r.clone());
            }
            // Costs are not implicitly divided across multiple product scopes.
            // One explicit selected scope may choose its actual acceptance;
            // otherwise retain the original all-declared-scope ambiguity.
            let cost_accepted = if acceptance.len() == 1 {
                accepted
                    .iter()
                    .filter(|(s, _)| same(s, &acceptance[0]))
                    .cloned()
                    .collect()
            } else if acceptance.is_empty() {
                accepted.clone()
            } else {
                vec![]
            };
            let cost = costs(i, &rs, &cost_accepted);
            let mut assumptions = i.0.parameter_sources.clone();
            if let Some(goal) = crate::comparison::saved_goal(i) {
                assumptions.push(ParameterSource {
                    target: ObjectRef {
                        kind: if i.1.acceptance_goal.is_some() {
                            ObjectKind::Scenario
                        } else {
                            ObjectKind::Project
                        },
                        id: if i.1.acceptance_goal.is_some() {
                            ObjectId::try_from(i.1.id.as_str().to_owned()).unwrap()
                        } else {
                            ObjectId::try_from(i.0.id.as_str().to_owned()).unwrap()
                        },
                    },
                    field_path: if i.1.acceptance_goal.is_some() {
                        "/acceptance_goal".into()
                    } else {
                        "/generation_context/goal".into()
                    },
                    value: Some(TypedValue::String(serde_json::to_string(goal).unwrap())),
                    basis: SourceBasis::Unknown,
                    source_ref: None,
                });
            }
            results.push(RunEvaluation {
                run_id: i.3.id.clone(),
                input_ref: ReportRunInputRef {
                    project_id: f.project_id.clone(),
                    project_revision: f.project_revision.clone(),
                    scenario_id: f.scenario_id.clone(),
                    scenario_revision: f.scenario_revision.clone(),
                },
                evidence_origin: i.3.evidence_origin.clone(),
                scope: i.3.scope.clone(),
                time_reference: i.1.time_reference.clone(),
                requested_window: i.3.window.clone(),
                computed_until_ms: i.3.computed_until_ms.clone(),
                computation_status: i.3.computation_status.clone(),
                whole_window_computed: i.3.whole_window_computed,
                goal_met: if i.3.whole_window_computed {
                    i.3.goal_met
                } else {
                    None
                },
                acceptance_scope: actual_scopes,
                order_metrics: i.3.orders.clone(),
                resource_metrics: rs,
                costs: cost,
                events: i.3.events.clone(),
                acceptance_events: i.3.acceptance_events.clone(),
                resource_claims: i.3.resource_claims.clone(),
                waits: i.3.waits.clone(),
                phase_work_intervals: i.3.phase_work_intervals.clone(),
                limitations,
                module_versions: i.0.module_refs.clone(),
                input_assumptions: assumptions,
                jobs: i.3.jobs.clone(),
                transport: i.3.transport.clone(),
            });
        }
        context.run_refs = inputs.iter().map(|i| i.3.id.clone()).collect();
        context.scenario_refs = inputs
            .iter()
            .map(|i| ScenarioRevisionRef {
                id: i.1.id.clone(),
                revision: i.1.revision.clone(),
            })
            .collect();
        context.module_versions = modules.into_values().collect();
        context.acceptance_scope = inputs.iter().flat_map(declared_scopes).collect();
        context.time_reference = inputs
            .iter()
            .all(|i| same(&i.1.time_reference, &inputs[0].1.time_reference))
            .then(|| inputs[0].1.time_reference.clone());
        context.observation_window = inputs
            .iter()
            .all(|i| same(&i.3.window, &inputs[0].3.window))
            .then(|| inputs[0].3.window.clone());
        context.scenario_comparison = if inputs.len() > 1 {
            Some(crate::compare_runs(inputs, keys, objects, acceptance)?)
        } else {
            None
        };
        context.missing_conditions.extend(
            results
                .iter()
                .flat_map(|r| r.costs.missing_conditions.iter().chain(&r.limitations))
                .cloned(),
        );
        context.run_evaluations = results;
        if keys.is_empty() || keys.iter().any(|k| matches!(k, MetricKey::UnitCost)) {
            if let Some(comparison) = &mut context.scenario_comparison {
                cost_comparison(comparison, &context.run_evaluations);
            }
        }
    }
    Ok(Versioned {
        schema_version: serde_json::from_value(serde_json::json!("1.0"))
            .expect("frozen schema version"),
        data: context,
    })
}

// Cost pair projections use this same frozen report's measurements, never new prices.
fn cost_comparison(comparison: &mut RunComparison, runs: &[RunEvaluation]) {
    let currencies: BTreeSet<_> = runs
        .iter()
        .flat_map(|r| r.costs.currencies.iter().map(|c| c.currency.clone()))
        .collect();
    if currencies.is_empty() {
        return;
    }
    let mut cost_metrics = vec![];
    for currency in currencies {
        let scopes: BTreeMap<_, _> = runs
            .iter()
            .flat_map(|r| r.costs.currencies.iter())
            .filter(|c| c.currency == currency)
            .filter_map(|c| c.accepted_scope.as_ref())
            .map(|s| (value(s).to_string(), s.clone()))
            .collect();
        for scope in scopes.values() {
            let values: Vec<RunMetric> = runs
                .iter()
                .map(|r| {
                    let charge = r.costs.currencies.iter().find(|c| {
                        c.currency == currency
                            && c.accepted_scope.as_ref().is_some_and(|s| same(s, scope))
                    });
                    let value = charge.and_then(|c| c.unit_cost.as_ref()).map_or_else(
                        || {
                            MetricValue::Unknown(problem(
                                "unit_cost",
                                "所选成本范围不完整、币种/接受口径不匹配或接受量为0",
                            ))
                        },
                        |amount| MetricValue::MoneyPerUnit {
                            amount: amount.clone(),
                            currency: currency.clone(),
                            unit: scope.unit.clone(),
                        },
                    );
                    RunMetric {
                        run_id: r.run_id.clone(),
                        metric: MetricKey::UnitCost,
                        value,
                    }
                })
                .collect();
            let pairs = comparison
                .pairs
                .iter()
                .map(|p| {
                    let a = values
                        .iter()
                        .find(|v| v.run_id == p.baseline_run_id)
                        .unwrap();
                    let b = values
                        .iter()
                        .find(|v| v.run_id == p.candidate_run_id)
                        .unwrap();
                    let calculated = if !matches!(p.status, ComparisonStatus::SideBySideOnly) {
                        match (&a.value, &b.value) {
                            (
                                MetricValue::MoneyPerUnit { amount: a, .. },
                                MetricValue::MoneyPerUnit { amount: b, .. },
                            ) => Some((
                                NumericDifference::MoneyPerUnit {
                                    amount: number(b.value() - a.value()).unwrap(),
                                    currency: currency.clone(),
                                    unit: scope.unit.clone(),
                                },
                                if a.value() != 0. {
                                    number((b.value() - a.value()) / a.value() * 100.)
                                } else {
                                    None
                                },
                            )),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    MetricPairDifference {
                        baseline_run_id: p.baseline_run_id.clone(),
                        candidate_run_id: p.candidate_run_id.clone(),
                        comparable: calculated.is_some(),
                        difference: calculated.as_ref().map(|c| c.0.clone()),
                        change_percent: calculated.and_then(|c| c.1),
                        conditions: if matches!(p.status, ComparisonStatus::SideBySideOnly) {
                            p.conditions.iter().flat_map(|c| c.issues.clone()).collect()
                        } else if matches!(a.value, MetricValue::Unknown(_))
                            || matches!(b.value, MetricValue::Unknown(_))
                        {
                            vec![problem("unit_cost", "单位成本缺少完整同口径数值")]
                        } else {
                            vec![]
                        },
                    }
                })
                .collect();
            cost_metrics.push(MetricComparison {
                metric: MetricKey::UnitCost,
                object_ref: None,
                acceptance_scope: Some(scope.clone()),
                currency: Some(currency.clone()),
                values,
                pair_differences: pairs,
            });
        }
    }
    if !cost_metrics.is_empty() {
        comparison
            .metrics
            .retain(|m| !matches!(m.metric, MetricKey::UnitCost));
        comparison.metrics.extend(cost_metrics);
    }
}
