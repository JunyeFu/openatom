use crate::comparison::{declared_scopes, issue, same, saved_goal, value, FrozenInput};
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

fn unknown(message: &str, object: Option<&ObjectRef>) -> MetricValue {
    MetricValue::Unknown(issue("comparison_metric_unknown", message, object.cloned()))
}
fn scaled(q: &Quantity) -> u128 {
    q.value.coefficient.value() as u128 * 10u128.pow(9 - q.value.decimal_places.value() as u32)
}
fn quantity(mut n: u128, unit: &UnitId) -> Option<Quantity> {
    let mut places = 9u8;
    while places > 0 && n % 10 == 0 {
        n /= 10;
        places -= 1;
    }
    Some(Quantity {
        unit: unit.clone(),
        value: DecimalAmount {
            coefficient: SafeUint::try_from(u64::try_from(n).ok()?).ok()?,
            decimal_places: DecimalPlaces::try_from(places).ok()?,
        },
    })
}
fn duration(n: u128) -> Option<DurationMs> {
    Some(DurationMs(SafeUint::try_from(u64::try_from(n).ok()?).ok()?))
}
fn frontier(i: &FrozenInput) -> u64 {
    i.3.computed_until_ms
        .as_ref()
        .map_or(i.3.window.start_ms.0.value(), |t| {
            t.0.value().min(i.3.window.end_ms.0.value())
        })
}
fn clip(interval: &IntervalMs, start: u64, end: u64) -> Option<(u64, u64)> {
    let a = interval.start_ms.0.value().max(start);
    let b = interval.end_ms.0.value().min(end);
    (a < b).then_some((a, b))
}
fn union_duration(mut intervals: Vec<(u64, u64)>) -> u128 {
    intervals.sort_unstable();
    let mut total = 0u128;
    let mut current: Option<(u64, u64)> = None;
    for (a, b) in intervals {
        match current {
            Some((x, y)) if a <= y => current = Some((x, y.max(b))),
            Some((x, y)) => {
                total += (y - x) as u128;
                current = Some((a, b));
            }
            None => current = Some((a, b)),
        }
    }
    if let Some((x, y)) = current {
        total += (y - x) as u128;
    }
    total
}
fn order_matches(order: &OrderLine, scope: &AcceptanceScope) -> bool {
    same(&order.product_ref, &scope.product_ref)
        && order.destination == scope.destination
        && same(&order.acceptance_stage, &scope.stage)
        && order
            .quantity
            .as_ref()
            .is_some_and(|q| q.unit == scope.unit)
}
fn accepted(i: &FrozenInput, scope: &AcceptanceScope, completion: bool) -> MetricValue {
    if let Some(goal) = saved_goal(i) {
        let scopes = declared_scopes(i);
        if !scopes.iter().any(|s| same(s, scope)) {
            return unknown("This scope is not the Run's frozen acceptance goal", None);
        }
        if scopes.iter().any(|s| {
            s.unit == scope.unit
                && s.destination == scope.destination
                && same(&s.stage, &scope.stage)
                && !same(&s.product_ref, &scope.product_ref)
        }) {
            return unknown(
                "Acceptance events have no product identity for overlapping product scopes",
                None,
            );
        }
        if completion && scopes.iter().any(|s| !same(s, scope)) {
            return unknown(
                "The shared acceptance target has no declared per-scope allocation",
                None,
            );
        }
        let target = if completion {
            match &goal.accepted_quantity_per_day {
                Some(q) if q.unit == scope.unit && scaled(q) > 0 => Some(scaled(q)),
                _ => return unknown("No positive declared completion target", None),
            }
        } else {
            None
        };
        // In no-order mode the actual Engine emits order_id=None only after
        // checking a declared goal scope's product, unit, place and stage.
        // Count its actual acceptance events, never stock or planned output.
        let mut events: Vec<_> =
            i.3.acceptance_events
                .iter()
                .filter(|e| {
                    e.order_id.is_none()
                        && e.destination == scope.destination
                        && same(&e.acceptance_stage, &scope.stage)
                        && e.at_ms.0.value() >= i.3.window.start_ms.0.value()
                        && e.at_ms.0.value() <= frontier(i)
                })
                .collect();
        events.sort_by_key(|e| e.at_ms.0.value());
        let mut total = 0u128;
        for event in events {
            if event.quantity.unit != scope.unit {
                return unknown("Actual acceptance quantity has a different unit", None);
            }
            total += scaled(&event.quantity);
            if target.is_some_and(|t| total >= t) {
                return MetricValue::Duration(DurationMs(event.at_ms.0.clone()));
            }
        }
        return if completion {
            unknown(
                "Declared acceptance target is unfinished; completion time is unknown",
                None,
            )
        } else {
            quantity(total, &scope.unit).map_or_else(
                || unknown("Exact sum exceeds shared quantity representation", None),
                MetricValue::Quantity,
            )
        };
    }
    let orders: Vec<_> =
        i.1.orders
            .iter()
            .filter(|o| order_matches(o, scope))
            .collect();
    if orders.is_empty() {
        return unknown(
            "No matching declared demand for this acceptance scope",
            None,
        );
    }
    if completion {
        let mut last = i.3.window.start_ms.0.value();
        for order in orders {
            let Some(outcome) = i.3.orders.iter().find(|o| o.order_id == order.id) else {
                return unknown("Actual order outcome is absent", None);
            };
            if scaled(&outcome.outstanding) > 0 {
                return unknown(
                    "Demand is unfinished; a completion time cannot be inferred",
                    None,
                );
            }
            let Some(at) = outcome.completion_ms.as_ref() else {
                return unknown("Completion boundary is not known", None);
            };
            last = last.max(at.0.value());
        }
        return MetricValue::Duration(DurationMs(
            SafeUint::try_from(last).expect("bounded absolute relative time"),
        ));
    }
    let mut total = 0u128;
    for event in &i.3.acceptance_events {
        if event.at_ms.0.value() < i.3.window.start_ms.0.value()
            || event.at_ms.0.value() > frontier(i)
        {
            continue;
        }
        if event.destination != scope.destination || !same(&event.acceptance_stage, &scope.stage) {
            continue;
        }
        if event
            .order_id
            .as_ref()
            .is_some_and(|id| orders.iter().any(|o| &o.id == id))
        {
            if event.quantity.unit != scope.unit {
                return unknown("Actual acceptance quantity has a different unit", None);
            }
            total += scaled(&event.quantity);
        } else if event.order_id.is_none() {
            return unknown("Acceptance event has no order/product identity", None);
        }
    }
    quantity(total, &scope.unit).map_or_else(
        || unknown("Exact sum exceeds shared quantity representation", None),
        MetricValue::Quantity,
    )
}
fn waiting(i: &FrozenInput, object: &ObjectRef) -> MetricValue {
    let exists = match object.kind {
        ObjectKind::Device => i.0.devices.iter().any(|v| v.id == object.id),
        ObjectKind::Job => i.3.jobs.iter().any(|v| v.id == object.id),
        ObjectKind::Task => i.1.production_tasks.iter().any(|v| v.id == object.id),
        ObjectKind::Transport => i.3.transport.iter().any(|v| v.id == object.id),
        ObjectKind::Resource => i.0.resources.iter().any(|v| v.id == object.id),
        ObjectKind::Order => i.1.orders.iter().any(|v| v.id == object.id),
        _ => i.3.waits.iter().any(|v| same(&v.object_ref, object)),
    };
    if !exists {
        return unknown(
            "Waiting object is absent from this Run's frozen inputs and actual result",
            Some(object),
        );
    }
    let intervals=i.3.waits.iter().filter(|w|same(&w.object_ref,object)
        ||(matches!(object.kind,ObjectKind::Transport)
            &&matches!(w.object_ref.kind,ObjectKind::Job)&&w.object_ref.id==object.id)
        ||matches!(&w.execution_ref,Some(JobExecutionRef::ProjectDevice{device_id,..}) if matches!(object.kind,ObjectKind::Device)&&device_id==&object.id))
        .filter_map(|w|clip(&w.interval,i.3.window.start_ms.0.value(),frontier(i))).collect();
    duration(union_duration(intervals)).map_or_else(
        || {
            unknown(
                "Waiting duration exceeds shared representation",
                Some(object),
            )
        },
        MetricValue::Duration,
    )
}
fn utilization(i: &FrozenInput, object: &ObjectRef) -> MetricValue {
    if !matches!(object.kind, ObjectKind::Resource) {
        return unknown(
            "Resource utilization requires an actual Resource object",
            Some(object),
        );
    }
    let Some(resource) = i.0.resources.iter().find(|r| r.id == object.id) else {
        return unknown("Resource is absent from frozen input", Some(object));
    };
    let Some(calendar) = i.0.calendars.iter().find(|c| c.id == resource.calendar_id) else {
        return unknown("Resource availability calendar is absent", Some(object));
    };
    let open: Vec<_> = calendar
        .intervals
        .iter()
        .filter_map(|t| clip(t, i.3.window.start_ms.0.value(), frontier(i)))
        .collect();
    let available = union_duration(open.clone());
    let capacity = scaled(&resource.capacity);
    if available == 0 || capacity == 0 {
        return unknown("No positive calendar/capacity denominator", Some(object));
    }
    let mut used = 0u128;
    let mut seen = BTreeMap::new();
    for claim in
        i.3.resource_claims
            .iter()
            .filter(|c| c.resource_id == resource.id)
    {
        if let Some(previous) = seen.insert(claim.id.as_str(), value(claim)) {
            if previous != value(claim) {
                return unknown(
                    "Same Claim identity contains conflicting actual records",
                    Some(object),
                );
            }
            continue;
        }
        let mut intersections = vec![];
        for interval in &claim.intervals {
            if let Some((a, b)) = clip(interval, i.3.window.start_ms.0.value(), frontier(i)) {
                for &(c, d) in &open {
                    let start = a.max(c);
                    let end = b.min(d);
                    if start < end {
                        intersections.push((start, end));
                    }
                }
            }
        }
        used += union_duration(intersections) * claim.units.value() as u128;
    }
    let ratio = used as f64 * 1_000_000_000.0 / (available as f64 * capacity as f64);
    MetricValue::Scalar {
        value: FiniteNumber::try_from(ratio).expect("finite bounded resource ratio"),
        unit: UnitId::try_from("ratio".to_owned()).expect("unit"),
    }
}
fn metric(
    i: &FrozenInput,
    key: &MetricKey,
    object: Option<&ObjectRef>,
    acceptance: Option<&AcceptanceScope>,
) -> MetricValue {
    if i.3.computed_until_ms.is_none() {
        return unknown("No computed prefix", object);
    }
    match key {
        MetricKey::AcceptedQuantity | MetricKey::CompletionTime => acceptance.map_or_else(
            || {
                unknown(
                    "A product/unit/destination/stage acceptance scope is required",
                    object,
                )
            },
            |s| accepted(i, s, matches!(key, MetricKey::CompletionTime)),
        ),
        MetricKey::WaitingTime => object.map_or_else(
            || {
                unknown(
                    "Select an actual waiting object; waiting is not factory elapsed time",
                    None,
                )
            },
            |o| waiting(i, o),
        ),
        MetricKey::ResourceUtilization => object.map_or_else(
            || unknown("Select an actual resource", None),
            |o| utilization(i, o),
        ),
        MetricKey::ReworkQuantity => {
            let Some(o) = object else {
                return unknown(
                    "Select a tracked item; unlike item types are not aggregated",
                    None,
                );
            };
            if !matches!(o.kind, ObjectKind::Item) {
                return unknown("Rework quantity requires a tracked Item identity", Some(o));
            }
            let Some(item) = i.3.final_items.iter().find(|x| x.id == o.id) else {
                return unknown("Tracked item is absent from actual results", Some(o));
            };
            let individual = i.0.item_types.iter().any(|t| {
                t.id == item.item_type_ref.id && matches!(t.tracking, TrackingKind::Individual)
            });
            let event=i.3.events.iter().any(|e|e.at_ms.0.value()<=frontier(i)&&matches!(&e.payload,SimulationEventPayload::QualityChanged(d) if d.item_ref==item.id&&matches!(d.disposition,QualityDisposition::Rework)));
            if !individual || i.3.events.is_empty() || item.completed_reworks.value() > 0 && !event
            {
                return unknown(
                    "Actual unique rework quantity is not established by event history for an individual tracked item; attempts and batch quantity are not substituted",
                    Some(o),
                );
            }
            quantity(
                if item.completed_reworks.value() > 0 {
                    scaled(&item.quantity)
                } else {
                    0
                },
                &item.quantity.unit,
            )
            .map_or_else(
                || {
                    unknown(
                        "Exact rework quantity exceeds shared representation",
                        Some(o),
                    )
                },
                MetricValue::Quantity,
            )
        }
        MetricKey::UnitCost => unknown(
            "No frozen measured cost result is available; declared prices alone do not establish actual unit cost",
            object,
        ),
    }
}
fn difference(
    a: &MetricValue,
    b: &MetricValue,
) -> Option<(NumericDifference, Option<FiniteNumber>)> {
    let percent = |a: f64, b: f64| {
        if a == 0.0 {
            None
        } else {
            FiniteNumber::try_from((b - a) / a * 100.0).ok()
        }
    };
    match (a, b) {
        (MetricValue::Quantity(a), MetricValue::Quantity(b)) if a.unit == b.unit => {
            let x = scaled(a);
            let y = scaled(b);
            Some((
                NumericDifference::Quantity {
                    negative: y < x,
                    magnitude: quantity(x.abs_diff(y), &a.unit)?,
                },
                percent(x as f64, y as f64),
            ))
        }
        (MetricValue::Duration(a), MetricValue::Duration(b)) => {
            let x = a.0.value();
            let y = b.0.value();
            Some((
                NumericDifference::Duration {
                    negative: y < x,
                    magnitude: DurationMs(SafeUint::try_from(x.abs_diff(y)).ok()?),
                },
                percent(x as f64, y as f64),
            ))
        }
        (
            MetricValue::Scalar { value: a, unit: ua },
            MetricValue::Scalar { value: b, unit: ub },
        ) if ua == ub => Some((
            NumericDifference::Scalar {
                value: FiniteNumber::try_from(b.value() - a.value()).ok()?,
                unit: ua.clone(),
            },
            percent(a.value(), b.value()),
        )),
        _ => None,
    }
}

pub(crate) fn build(
    inputs: &[FrozenInput],
    keys: &[MetricKey],
    objects: &[ObjectRef],
    requested: &[AcceptanceScope],
    pairs: &[RunPairComparison],
) -> Vec<MetricComparison> {
    let mut scopes: BTreeMap<String, AcceptanceScope> = requested
        .iter()
        .map(|s| (value(s).to_string(), s.clone()))
        .collect();
    if requested.is_empty() {
        for i in inputs {
            for s in declared_scopes(i) {
                scopes.insert(value(&s).to_string(), s);
            }
        }
    }
    let all = [
        MetricKey::AcceptedQuantity,
        MetricKey::CompletionTime,
        MetricKey::WaitingTime,
        MetricKey::ResourceUtilization,
        MetricKey::ReworkQuantity,
        MetricKey::UnitCost,
    ];
    let keys = if keys.is_empty() { &all[..] } else { keys };
    let mut result = vec![];
    let mut emitted = BTreeSet::new();
    for key in keys {
        let contexts: Vec<(Option<ObjectRef>, Option<AcceptanceScope>)> = match key {
            MetricKey::AcceptedQuantity | MetricKey::CompletionTime => {
                if scopes.is_empty() {
                    vec![(None, None)]
                } else {
                    scopes.values().map(|s| (None, Some(s.clone()))).collect()
                }
            }
            MetricKey::WaitingTime | MetricKey::ResourceUtilization | MetricKey::ReworkQuantity => {
                let mut candidates: BTreeMap<String, ObjectRef> = BTreeMap::new();
                if !objects.is_empty() {
                    for o in objects {
                        candidates.insert(value(o).to_string(), o.clone());
                    }
                } else {
                    for i in inputs {
                        match key {
                            MetricKey::WaitingTime => {
                                for w in &i.3.waits {
                                    candidates.insert(
                                        value(&w.object_ref).to_string(),
                                        w.object_ref.clone(),
                                    );
                                }
                            }
                            MetricKey::ResourceUtilization => {
                                for r in &i.0.resources {
                                    let o = ObjectRef {
                                        kind: ObjectKind::Resource,
                                        id: r.id.clone(),
                                    };
                                    candidates.insert(value(&o).to_string(), o);
                                }
                            }
                            _ => {
                                for item in &i.3.final_items {
                                    if item.completed_reworks.value() > 0 {
                                        let o = ObjectRef {
                                            kind: ObjectKind::Item,
                                            id: item.id.clone(),
                                        };
                                        candidates.insert(value(&o).to_string(), o);
                                    }
                                }
                            }
                        }
                    }
                }
                if candidates.is_empty() {
                    vec![(None, None)]
                } else {
                    candidates.into_values().map(|o| (Some(o), None)).collect()
                }
            }
            _ => vec![(None, None)],
        };
        for (object, acceptance) in contexts {
            if !emitted.insert(format!(
                "{}|{}|{}",
                value(key),
                value(&object),
                value(&acceptance)
            )) {
                continue;
            }
            let values: Vec<_> = inputs
                .iter()
                .map(|i| RunMetric {
                    run_id: i.3.id.clone(),
                    metric: key.clone(),
                    value: metric(i, key, object.as_ref(), acceptance.as_ref()),
                })
                .collect();
            let pair_differences=pairs.iter().enumerate().map(|(n,p)|{
                let diff=difference(&values[0].value,&values[n+1].value);
                let comparable=!matches!(p.status,ComparisonStatus::SideBySideOnly)&&diff.is_some();
                MetricPairDifference{baseline_run_id:p.baseline_run_id.clone(),candidate_run_id:p.candidate_run_id.clone(),comparable,
                    difference:if comparable{diff.as_ref().map(|d|d.0.clone())}else{None},change_percent:if comparable{diff.and_then(|d|d.1)}else{None},
                    conditions:if comparable{vec![]}else{vec![issue("metric_not_comparable","Different/incomplete conditions, unknown value or incompatible unit; no numeric improvement is asserted",object.clone())]}}
            }).collect();
            result.push(MetricComparison {
                metric: key.clone(),
                object_ref: object,
                acceptance_scope: acceptance,
                currency: None,
                values,
                pair_differences,
            });
        }
    }
    result
}
