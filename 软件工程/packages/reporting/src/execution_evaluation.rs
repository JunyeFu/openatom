//! Pure execution report producer. All arguments are Host-loaded frozen facts;
//! no Engine, filesystem, caller authority or precomputed report is accepted.
use crate::comparison::{issue, same, value};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sf_domain::{cloud_execution::*, *};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportInput {
    pub report_id: ObjectId,
    pub project_id: ProjectId,
    pub export_project_revision: SafeUint,
    pub generated_at_utc: CloudTimestamp,
    pub sessions: Vec<ExecutionReportInputSession>,
    pub prediction: Option<PredictionInput>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReportInputSession {
    pub source: ExecutionReportSourceStamp,
    pub inputs: ExecutionFrozenInputs,
    pub snapshot: ExecutionSnapshot,
    pub history: ExecutionObservationHistoryDocument,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictionInput {
    pub reference: PredictionRunReference,
    pub project: ProjectState,
    pub scenario: ScenarioInput,
    pub definitions: Vec<ModuleDefinition>,
    pub run: RunResult,
    pub execution: ExecutionReportSource,
}
fn missing(code: &str, text: impl Into<String>) -> Issue {
    issue(code, text, None)
}
fn reject(text: impl Into<String>) -> Vec<Issue> {
    vec![missing("execution_report_source", text)]
}
fn decode<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, Vec<Issue>> {
    serde_json::from_value(v).map_err(|e| reject(format!("Canonical typed report invariant: {e}")))
}
fn number(n: u128) -> Option<SafeUint> {
    u64::try_from(n).ok()?.try_into().ok()
}
fn scaled(q: &Quantity) -> u128 {
    q.value.coefficient.value() as u128 * 10u128.pow(9 - q.value.decimal_places.value() as u32)
}
fn exact(mut n: u128, unit: &UnitId) -> Option<Quantity> {
    let mut places = 9u8;
    while places > 0 && n % 10 == 0 {
        n /= 10;
        places -= 1;
    }
    Some(Quantity {
        value: DecimalAmount {
            coefficient: number(n)?,
            decimal_places: places.try_into().ok()?,
        },
        unit: unit.clone(),
    })
}
fn quantity_equal(a: &Quantity, b: &Quantity) -> bool {
    a.unit == b.unit && scaled(a) == scaled(b)
}
fn sorted<T: Serialize>(xs: &[T]) -> Vec<String> {
    let mut v: Vec<_> = xs.iter().map(|x| value(x).to_string()).collect();
    v.sort();
    v
}
fn module_versions(defs: &[ModuleDefinition]) -> Vec<ModuleRef> {
    let mut d: Vec<_> = defs
        .iter()
        .map(|d| ModuleRef {
            id: d.id.clone(),
            version: d.version.clone(),
        })
        .collect();
    d.sort_by_key(|d| (d.id.as_str().to_owned(), value(&d.version).to_string()));
    d
}
fn check_definitions(p: &ProjectState, defs: &[ModuleDefinition]) -> Result<(), Vec<Issue>> {
    let mut seen = BTreeSet::new();
    if defs
        .iter()
        .any(|d| !seen.insert((d.id.as_str().to_owned(), value(&d.version).to_string())))
        || p.module_refs
            .iter()
            .any(|m| !defs.iter().any(|d| d.id == m.id && d.version == m.version))
    {
        return Err(reject(
            "Missing or duplicate exact frozen execution module definition",
        ));
    }
    Ok(())
}
fn object(o: &ExecutionObservedObject) -> (&'static str, String, Value) {
    match o {
        ExecutionObservedObject::Transport { data } => ("transport", data.id.as_str().into(), value(data)),
        ExecutionObservedObject::Item { data } => ("item", data.id.as_str().into(), value(data)),
        ExecutionObservedObject::Job { data } => ("job", data.job_id.as_str().into(), value(data)),
        ExecutionObservedObject::Device { data } => (
            "device",
            data.binding.device_id.as_str().into(),
            value(data),
        ),
        ExecutionObservedObject::LogicalWork { data } => (
            "logical_work",
            data.logical_job_id.as_str().into(),
            value(data),
        ),
        ExecutionObservedObject::Attempt { data } => (
            "attempt",
            data.attempt.command_id.as_str().into(),
            value(data),
        ),
    }
}
fn validate_session(
    s: &ExecutionReportInputSession,
    project: &ProjectId,
) -> Result<(), Vec<Issue>> {
    let (f, x, h, src) = (&s.inputs, &s.snapshot, &s.history, &s.source);
    if &src.identity.project_id != project
        || !same(&src.identity, &f.identity)
        || !same(&src.identity, &x.identity)
        || !same(&src.identity, &h.identity)
        || src.execution_revision != x.execution_revision
        || src.observation_frontier != x.observation_frontier
        || h.observation_frontier != x.observation_frontier
        || !same(&f.project_input_ref, &x.project_input_ref)
        || f.project.id != *project
        || f.project_input_ref.project_id != *project
        || !same(&f.project.revision, &f.project_input_ref.project_revision)
        || f.scenario.id != f.project_input_ref.scenario_id
        || !same(&f.scenario.revision, &f.project_input_ref.scenario_revision)
        || !same(&f.scenario.project_revision, &f.project.revision)
    {
        return Err(reject(
            "Session/epoch/project/R/F or original frozen input pins disagree",
        ));
    }
    match src.read_origin {
        ExecutionReportSourceStampReadOrigin::Host
            if src.delivery.local_execution_revision != Some(src.execution_revision.clone()) =>
        {
            return Err(reject("Host source local revision is not selected R"));
        }
        ExecutionReportSourceStampReadOrigin::Cloud
            if src.delivery.cloud_published_revision != Some(src.execution_revision.clone()) =>
        {
            return Err(reject("Cloud published revision is not selected R"));
        }
        _ => {}
    }
    if value(&src.read_origin) != value(&src.delivery.read_origin) {
        return Err(reject("Read origin and delivery origin disagree"));
    }
    check_definitions(&f.project, &f.execution_definitions)?;
    if f.project.module_refs.iter().any(|r| {
        !f.modules
            .iter()
            .any(|m| m.definition.id == r.id && m.definition.version == r.version)
    }) {
        return Err(reject("Frozen Project module package closure is missing"));
    }
    for m in &f.modules {
        if !f
            .execution_definitions
            .iter()
            .any(|d| d.id == m.definition.id && d.version == m.definition.version)
            || m.dependencies.iter().any(|r| {
                !f.modules
                    .iter()
                    .any(|d| d.definition.id == r.id && d.definition.version == r.version)
            })
        {
            return Err(reject(
                "Frozen module package/dependency closure is incomplete",
            ));
        }
    }
    if value(&x.policy.source_prediction_window.start_ms) != json!(0)
        || x.policy.source_prediction_window.end_ms.value() != f.scenario.window.end_ms.0.value()
        || f.scenario.window.start_ms.0.value() != 0
        || (!x.coverage.clock_started && x.coverage.observed_until_host_ms.value() != 0)
    {
        return Err(reject(
            "Frozen prediction window or external clock coverage disagrees",
        ));
    }
    if h.records.len() as u64 != h.observation_frontier.value() {
        return Err(reject(
            "Full observation prefix through F is required; no missing pages/resimulation",
        ));
    }
    let mut latest = BTreeMap::new();
    let mut old_time = 0;
    let mut old_revision = 0;
    for (n, r) in h.records.iter().enumerate() {
        if r.record_sequence.value() != n as u64 + 1
            || r.execution_revision.value() > src.execution_revision.value()
            || r.execution_revision.value() < old_revision
            || r.at_host_ms.value() < old_time
            || r.at_host_ms.value() > x.coverage.observed_until_host_ms.value()
        {
            return Err(reject(
                "Observation sequence/revision/Host time is missing, out of order or beyond selected R/F/T",
            ));
        }
        old_time = r.at_host_ms.value();
        old_revision = r.execution_revision.value();
        let (k, id, v) = object(&r.object);
        latest.insert((k, id), v);
    }
    let mut final_states = BTreeMap::new();
    for trip in &x.business_facts.transport {
        final_states.insert(("transport", trip.id.as_str().to_owned()), value(trip));
    }
    for i in &x.business_facts.items {
        final_states.insert(("item", i.id.as_str().to_owned()), value(i));
    }
    for j in &x.bound_jobs {
        final_states.insert(("job", j.job_id.as_str().to_owned()), value(j));
    }
    for d in &x.devices {
        final_states.insert(
            ("device", d.binding.device_id.as_str().to_owned()),
            value(d),
        );
    }
    for w in &x.logical_work {
        final_states.insert(
            ("logical_work", w.logical_job_id.as_str().to_owned()),
            value(w),
        );
    }
    for a in &x.attempts {
        final_states.insert(
            ("attempt", a.attempt.command_id.as_str().to_owned()),
            value(a),
        );
    }
    if latest != final_states {
        return Err(reject(
            "Full history last-state projection differs from selected snapshot",
        ));
    }
    let mut ids = BTreeSet::new();
    if x.attempts
        .iter()
        .any(|a| !ids.insert(a.attempt.command_id.clone()))
    {
        return Err(reject("Duplicate Command identity in snapshot"));
    }
    for w in &x.logical_work {
        let created = h.records.iter().find(|r| matches!(&r.object, ExecutionObservedObject::LogicalWork { data } if data.logical_job_id == w.logical_job_id));
        let first_attempt = x.attempts.iter().filter(|a| a.attempt.logical_job_id == w.logical_job_id)
            .map(|a| a.dispatch_decision.decision_host_ms.value()).min();
        if created.is_none_or(|r| first_attempt.is_some_and(|at| r.at_host_ms.value() > at)) {
            return Err(reject("Logical work history must begin before its first actual attempt, including dynamically created setup, rework and supply trips"));
        }
    }
    for a in &x.business_facts.acceptance_events {
        if a.at_ms.0.value() > x.coverage.observed_until_host_ms.value() {
            return Err(reject("Acceptance exceeds observed Host prefix"));
        }
    }
    Ok(())
}
fn scopes(p: &ProjectState, s: &ScenarioInput) -> Vec<AcceptanceScope> {
    let mut out: Vec<AcceptanceScope> = if s.orders.is_empty() {
        s.acceptance_goal
            .as_ref()
            .or_else(|| p.generation_context.as_ref().map(|g| &g.goal))
            .map(|g| {
                std::iter::once(g.acceptance_scope.clone())
                    .chain(g.additional_acceptance_scopes.clone())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        s.orders
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
    };
    out.sort_by_key(|s| value(s).to_string());
    out.dedup_by(|a, b| same(a, b));
    out
}
fn matching_order(o: &OrderLine, s: &AcceptanceScope) -> bool {
    same(&o.product_ref, &s.product_ref)
        && o.destination == s.destination
        && same(&o.acceptance_stage, &s.stage)
        && o.quantity.as_ref().is_some_and(|q| q.unit == s.unit)
}
fn accepted(
    s: &ExecutionReportInputSession,
) -> (Vec<ExecutionAcceptedMetric>, Option<SafeUint>, Vec<Issue>) {
    let scope = scopes(&s.inputs.project, &s.inputs.scenario);
    let x = &s.snapshot;
    let mut all_missing = vec![];
    let mut selected_events: Vec<&AcceptanceEvent> = vec![];
    let metrics=scope.iter().map(|sc| {
        let mut reasons=vec![];
        if !x.coverage.clock_started {reasons.push(missing("execution_clock_unstarted","Host execution clock has not started; acceptance coverage is unobserved"));}
        let ambiguous=s.inputs.scenario.orders.is_empty() && scope.iter().any(|a|a.unit==sc.unit&&a.destination==sc.destination&&same(&a.stage,&sc.stage)&&!same(&a.product_ref,&sc.product_ref));
        if ambiguous {reasons.push(missing("execution_acceptance_ambiguous","Overlapping products share destination/stage/unit; raw acceptance events cannot distinguish product"));}
        let orders:Vec<_>=s.inputs.scenario.orders.iter().filter(|o|matching_order(o,sc)).collect();
        let events:Vec<_>=x.business_facts.acceptance_events.iter().filter(|a| a.destination==sc.destination && same(&a.acceptance_stage,&sc.stage)
            && if s.inputs.scenario.orders.is_empty(){a.order_id.is_none()} else {orders.iter().any(|o|a.order_id.as_ref()==Some(&o.id))}).collect();
        if events.iter().any(|a|a.quantity.unit!=sc.unit) {reasons.push(missing("execution_acceptance_unit","Acceptance quantity unit differs from frozen scope"));}
        for o in &orders {
            if let Some(outcome)=x.business_facts.order_outcomes.iter().find(|a|a.order_id==o.id) {
                let sum:u128=events.iter().filter(|a|a.order_id.as_ref()==Some(&o.id)).map(|a|scaled(&a.quantity)).sum();
                if outcome.accepted.unit!=sc.unit||scaled(&outcome.accepted)!=sum {reasons.push(missing("execution_order_evidence","Order outcome and acceptance event prefix disagree"));}
            } else {reasons.push(missing("execution_order_evidence","Matching frozen order lacks its actual outcome"));}
        }
        let total=if reasons.is_empty(){exact(events.iter().map(|a|scaled(&a.quantity)).sum(),&sc.unit)}else{None};
        if total.is_none()&&reasons.is_empty(){reasons.push(missing("execution_decimal_overflow","Exact accepted sum exceeds shared quantity representation"));}
        if scope.len()==1&&total.is_some(){selected_events=events;}
        all_missing.extend(reasons.clone());
        ExecutionAcceptedMetric{scope:sc.clone(),accepted_quantity:total,missing_conditions:reasons}
    }).collect::<Vec<_>>();
    let target = &x.policy.target_quantity;
    let completion = if scope.len() == 1
        && scope[0].unit == target.unit
        && scaled(target) > 0
        && metrics[0].accepted_quantity.is_some()
    {
        selected_events.sort_by_key(|a| a.at_ms.0.value());
        let mut n = 0u128;
        selected_events.iter().find_map(|a| {
            n += scaled(&a.quantity);
            (n >= scaled(target)).then(|| a.at_ms.0.clone())
        })
    } else {
        None
    };
    if completion.is_none() {
        all_missing.push(missing("execution_target_time_unknown","Unique scope, positive fixed target, started Host clock and actual target-reaching prefix are required"));
    }
    (metrics, completion, all_missing)
}
fn union(mut v: Vec<(u64, u64)>) -> u128 {
    v.sort_unstable();
    let mut current = None;
    let mut n = 0;
    for (a, b) in v {
        match current {
            Some((x, y)) if a <= y => current = Some((x, y.max(b))),
            Some((x, y)) => {
                n += (y - x) as u128;
                current = Some((a, b));
            }
            None => current = Some((a, b)),
        }
    }
    if let Some((a, b)) = current {
        n += (b - a) as u128;
    }
    n
}
fn waiting(s: &ExecutionReportInputSession) -> Vec<ExecutionWaitingMetric> {
    s.snapshot
        .logical_work
        .iter()
        .map(|w| {
            let records: Vec<_> = s
                .history
                .records
                .iter()
                .filter_map(|r| match &r.object {
                    ExecutionObservedObject::LogicalWork { data }
                        if data.logical_job_id == w.logical_job_id =>
                    {
                        Some((r.at_host_ms.value(), &data.state))
                    }
                    _ => None,
                })
                .collect();
            let mut sums = [0u128; 4];
            for (n, (at, state)) in records.iter().enumerate() {
                let end = records
                    .get(n + 1)
                    .map_or(s.snapshot.coverage.observed_until_host_ms.value(), |r| r.0);
                let slot = match state {
                    LogicalWorkViewState::Queued => Some(0),
                    LogicalWorkViewState::Reserved
                    | LogicalWorkViewState::Published
                    | LogicalWorkViewState::DeviceAccepted => Some(1),
                    LogicalWorkViewState::OutputWait => Some(2),
                    LogicalWorkViewState::ReconciliationRequired => Some(3),
                    _ => None,
                };
                if let Some(k) = slot {
                    sums[k] += (end - at) as u128;
                }
            }
            let known = s.snapshot.coverage.clock_started;
            ExecutionWaitingMetric {
                logical_job_id: w.logical_job_id.clone(),
                queued_wait_ms: known.then(|| number(sums[0])).flatten(),
                pre_start_wait_ms: known.then(|| number(sums[1])).flatten(),
                output_wait_ms: known.then(|| number(sums[2])).flatten(),
                reconciliation_wait_ms: known.then(|| number(sums[3])).flatten(),
                missing_conditions: if known {
                    vec![]
                } else {
                    vec![missing(
                        "execution_wait_unobserved",
                        "Host clock has not started; waiting is not observed zero",
                    )]
                },
            }
        })
        .collect()
}
fn resources(s: &ExecutionReportInputSession) -> Result<Vec<ExecutionResourceMetric>, Vec<Issue>> {
    let mut claims = BTreeMap::new();
    let until = s.snapshot.coverage.observed_until_host_ms.value();
    for p in &s.snapshot.business_facts.resource_claims {
        let c = &p.claim;
        if let Some(old) = claims.insert(c.id.clone(), p) {
            if !same(old, p) {
                return Err(reject("Conflicting projected claim identity"));
            }
        }
        if p.open_interval_index
            .as_ref()
            .is_some_and(|n| n.value() as usize >= c.intervals.len())
            || c.intervals
                .iter()
                .any(|i| i.start_ms.0.value() > i.end_ms.0.value())
        {
            return Err(reject(
                "Invalid original resource claim interval/open index",
            ));
        }
    }
    Ok(s.inputs
        .project
        .resources
        .iter()
        .map(|r| {
            let mut intervals = vec![];
            let mut unit_ms = 0;
            for p in claims.values().filter(|p| p.claim.resource_id == r.id) {
                let clipped: Vec<_> = p
                    .claim
                    .intervals
                    .iter()
                    .filter_map(|i| {
                        let a = i.start_ms.0.value();
                        let b = i.end_ms.0.value().min(until);
                        (a < b).then_some((a, b))
                    })
                    .collect();
                unit_ms += union(clipped.clone()) * p.claim.units.value() as u128;
                intervals.extend(clipped);
            }
            let occupation = number(union(intervals));
            let units = number(unit_ms);
            let mut missing_conditions = vec![missing(
                "physical_work_unmeasured",
                "Host-confirmed resource occupation is not measured physical effective work",
            )];
            if occupation.is_none() || units.is_none() {
                missing_conditions.push(missing(
                    "execution_resource_overflow",
                    "Exact resource duration or unit-ms exceeds safe representation",
                ));
            }
            if !s.snapshot.coverage.clock_started {
                missing_conditions.push(missing(
                    "execution_resource_unobserved",
                    "Host clock has not started",
                ));
            }
            ExecutionResourceMetric {
                resource_id: r.id.clone(),
                confirmed_occupation_ms: if s.snapshot.coverage.clock_started {
                    occupation
                } else {
                    None
                },
                claimed_unit_ms: if s.snapshot.coverage.clock_started {
                    units
                } else {
                    None
                },
                physical_effective_work_ms: None,
                missing_conditions,
            }
        })
        .collect())
}
fn source(s: &ExecutionReportSourceStamp) -> ExecutionReportSource {
    serde_json::from_value(json!({"read_origin":s.read_origin,"identity":s.identity,"execution_revision":s.execution_revision})).expect("same source enum alternatives")
}
fn terminal(x: &ExecutionSnapshot) -> bool {
    matches!(
        x.lifecycle.state,
        ExecutionSnapshotLifecycleState::Completed | ExecutionSnapshotLifecycleState::Stopped
    ) && x.lifecycle.unresolved_command_ids.is_empty()
        && x.pending_reservations.is_empty()
        && x.business_facts.pending_output_refs.is_empty()
        && !x.attempts.iter().any(|a| {
            matches!(
                a.state,
                AttemptViewState::Reserved
                    | AttemptViewState::Published
                    | AttemptViewState::DeviceAccepted
                    | AttemptViewState::DeviceStarted
                    | AttemptViewState::ReconciliationRequired
                    | AttemptViewState::DeviceFailed
            )
        })
}
fn normalized<T: Serialize>(x: &T, ignore: &[&str]) -> Value {
    let mut v = value(x);
    for k in ignore {
        v.as_object_mut().unwrap().remove(*k);
    }
    v
}
fn equivalent(a: &ExecutionFrozenInputs, b: &ExecutionFrozenInputs) -> bool {
    normalized(&a.project, &["revision", "title", "views"])
        == normalized(&b.project, &["revision", "title", "views"])
        && normalized(
            &a.scenario,
            &["revision", "project_revision", "title", "window"],
        ) == normalized(
            &b.scenario,
            &["revision", "project_revision", "title", "window"],
        )
        && sorted(&a.execution_definitions) == sorted(&b.execution_definitions)
        && sorted(&a.modules) == sorted(&b.modules)
        && sorted(&a.scope) == sorted(&b.scope)
        && sorted(&a.candidate_bindings) == sorted(&b.candidate_bindings)
}
fn binding_history_covered(s: &ExecutionReportSession, f: &ExecutionFrozenInputs) -> bool {
    !f.candidate_bindings.is_empty() && f.candidate_bindings.iter().all(|binding|
        s.snapshot.devices.iter().any(|d|same(&d.binding,binding))
        && s.history.records.iter().any(|r|r.at_host_ms.value()==0&&matches!(&r.object,ExecutionObservedObject::Device{data} if same(&data.binding,binding))))
        && s.snapshot.attempts.iter().all(|a|f.candidate_bindings.iter().any(|b|b.binding_id==a.attempt.selected_binding_id&&b.device_id==a.device_id&&b.phase_id==a.attempt.phase_id))
}
fn delta(a: &Quantity, b: &Quantity) -> Option<SignedQuantityChange> {
    if a.unit != b.unit {
        return None;
    }
    let (a0, b0) = (scaled(a), scaled(b));
    Some(SignedQuantityChange {
        direction: if b0 > a0 {
            SignedQuantityChangeDirection::Increase
        } else if b0 < a0 {
            SignedQuantityChangeDirection::Decrease
        } else {
            SignedQuantityChangeDirection::Equal
        },
        magnitude: exact(a0.abs_diff(b0), &a.unit)?,
    })
}
fn time_delta(a: Option<&SafeUint>, b: Option<&SafeUint>) -> Option<SignedSafeInt> {
    let n = b?.value() as i128 - a?.value() as i128;
    i64::try_from(n).ok()?.try_into().ok()
}
fn condition(key: &str, matches: bool, detail: &str) -> Value {
    json!({"key":key,"matches":matches,"detail":detail})
}
fn comparison(
    a: &ExecutionReportSession,
    b: &ExecutionReportSession,
    ia: &ExecutionFrozenInputs,
    ib: &ExecutionFrozenInputs,
) -> Result<ExecutionReportComparison, Vec<Issue>> {
    let modes = same(&a.snapshot.provenance.mode, &b.snapshot.provenance.mode);
    let clock = a.snapshot.coverage.clock_started
        && b.snapshot.coverage.clock_started
        && same(&a.snapshot.policy.clock_mode, &b.snapshot.policy.clock_mode);
    let target = scaled(&a.metrics.target_quantity) > 0
        && quantity_equal(&a.metrics.target_quantity, &b.metrics.target_quantity);
    let input = equivalent(ia, ib);
    let scope = a.metrics.accepted.len() == 1
        && b.metrics.accepted.len() == 1
        && same(&a.metrics.accepted[0].scope, &b.metrics.accepted[0].scope);
    let availability = a.snapshot.coverage.data_quality_issues.is_empty()
        && b.snapshot.coverage.data_quality_issues.is_empty()
        && binding_history_covered(a, ia)
        && binding_history_covered(b, ib);
    let common = modes && clock && target && input && scope && availability;
    let settlement = terminal(&a.snapshot)
        && terminal(&b.snapshot)
        && matches!(
            a.snapshot.goal_at_snapshot,
            ExecutionSnapshotGoalAtSnapshot::Satisfied
        )
        && matches!(
            b.snapshot.goal_at_snapshot,
            ExecutionSnapshotGoalAtSnapshot::Satisfied
        );
    let coverage = a.snapshot.coverage.observed_until_host_ms
        == b.snapshot.coverage.observed_until_host_ms
        && a.snapshot.policy.report_cutoff_ms == b.snapshot.policy.report_cutoff_ms
        && a.snapshot.coverage.requested_cutoff_reached
            == b.snapshot.coverage.requested_cutoff_reached;
    let completion = if common && settlement {
        time_delta(
            a.metrics.target_reached_at_host_ms.as_ref(),
            b.metrics.target_reached_at_host_ms.as_ref(),
        )
    } else {
        None
    };
    let quantity = if common && coverage {
        a.metrics.accepted[0]
            .accepted_quantity
            .as_ref()
            .zip(b.metrics.accepted[0].accepted_quantity.as_ref())
            .and_then(|(a, b)| delta(a, b))
    } else {
        None
    };
    decode(
        json!({"baseline":source(&a.source),"candidate":source(&b.source),"status":if completion.is_some()||quantity.is_some(){"observational"}else{"side_by_side_only"},
        "conditions":[condition("source_mode",modes,"Actual execution source modes must match"),condition("clock_policy",clock,"Host monotonic epochs, started observations"),condition("fixed_target",target,"Same exact positive fixed quantity and unit"),condition("frozen_inputs",input,"Actual frozen project/scenario/definitions/scope/candidates, not revision equality"),condition("observation_window",coverage,"Same actual observed endpoint and cutoff coverage"),condition("goal_and_settlement",settlement,"Both goals proved and all obligations settled"),condition("availability_changes",availability,"Actual device history retained; missing coverage denies affected deltas"),condition("quantity_coverage",coverage&&scope,"Exact common scope and actual observed endpoint")],
        "completion_delta_ms":completion,"accepted_delta":quantity,"limitations":[missing("observational_not_causal","Candidate minus baseline is observational; availability/control/network differences do not establish causal improvement"),missing("comparison_unknown_conditions","A null delta means its stated common/settlement/coverage conditions, exact source quantities or safe exact arithmetic were not all proved; it is not zero")]}),
    )
}
fn predicted_target(
    p: &ProjectState,
    s: &ScenarioInput,
    sc: &[AcceptanceScope],
) -> Option<Quantity> {
    if sc.len() != 1 {
        return None;
    }
    if s.orders.is_empty() {
        s.acceptance_goal
            .as_ref()
            .or_else(|| p.generation_context.as_ref().map(|c| &c.goal))?
            .accepted_quantity_per_day
            .clone()
    } else {
        let orders: Vec<_> = s
            .orders
            .iter()
            .filter(|o| matching_order(o, &sc[0]))
            .collect();
        exact(
            orders
                .iter()
                .filter_map(|o| o.quantity.as_ref())
                .map(scaled)
                .sum(),
            &sc[0].unit,
        )
    }
}
fn prediction(
    p: PredictionInput,
    sessions: &[ExecutionReportSession],
    inputs: &[ExecutionReportInputSession],
    project: &ProjectId,
) -> Result<PredictionExecutionComparison, Vec<Issue>> {
    let index = sessions
        .iter()
        .position(|s| same(&source(&s.source), &p.execution))
        .ok_or_else(|| {
            reject("Selected prediction execution link does not equal a selected report source")
        })?;
    let e = &sessions[index];
    let f = &inputs[index].inputs;
    let r = &p.run;
    let reference = &p.reference;
    if p.project.id != *project
        || r.input_ref.project_id != *project
        || reference.run_id != r.id
        || reference.input_ref.project_id != *project
        || !same(&reference.input_ref.project_revision, &p.project.revision)
        || reference.input_ref.scenario_id != p.scenario.id
        || !same(&reference.input_ref.scenario_revision, &p.scenario.revision)
        || !same(&r.input_ref.project_revision, &p.project.revision)
        || r.input_ref.scenario_id != p.scenario.id
        || !same(&r.input_ref.scenario_revision, &p.scenario.revision)
        || !same(&p.scenario.project_revision, &p.project.revision)
        || !same(&r.window, &p.scenario.window)
        || !matches!(r.evidence_origin, EvidenceOrigin::ActualSoftware)
        || !matches!(
            r.computation_status,
            ComputationStatus::Completed | ComputationStatus::Failed | ComputationStatus::Cancelled
        )
        || r.computed_until_ms.as_ref().is_some_and(|t| {
            t.0.value() < r.window.start_ms.0.value() || t.0.value() > r.window.end_ms.0.value()
        })
    {
        return Err(reject(
            "Prediction must be the exact selected actual terminal Project Run and its own frozen inputs/window/scope",
        ));
    }
    check_definitions(&p.project, &p.definitions)?;
    let frozen = (
        p.project.clone(),
        p.scenario.clone(),
        p.definitions.clone(),
        r.clone(),
    );
    let ps = scopes(&p.project, &p.scenario);
    let target = predicted_target(&p.project, &p.scenario, &ps);
    let mut objects: BTreeMap<String, ObjectRef> = r
        .waits
        .iter()
        .map(|w| (value(&w.object_ref).to_string(), w.object_ref.clone()))
        .collect();
    for j in &r.jobs {
        let o = ObjectRef {
            kind: ObjectKind::Job,
            id: j.id.clone(),
        };
        objects.insert(value(&o).to_string(), o);
    }
    let objects: Vec<_> = objects.into_values().collect();
    let calculated = if r.computed_until_ms.is_some() {
        crate::metrics::build(
            std::slice::from_ref(&frozen),
            &[
                MetricKey::AcceptedQuantity,
                MetricKey::CompletionTime,
                MetricKey::WaitingTime,
            ],
            &objects,
            &ps,
            &[],
        )
    } else {
        vec![]
    };
    let mut limits = vec![];
    let accepted = ps
        .iter()
        .map(|sc| {
            let v = calculated
                .iter()
                .find(|m| {
                    matches!(m.metric, MetricKey::AcceptedQuantity)
                        && m.acceptance_scope.as_ref().is_some_and(|s| same(sc, s))
                })
                .map(|m| &m.values[0].value);
            match v {
                Some(MetricValue::Quantity(q)) => ExecutionAcceptedMetric {
                    scope: sc.clone(),
                    accepted_quantity: Some(q.clone()),
                    missing_conditions: vec![],
                },
                other => {
                    let reason = match other {
                        Some(MetricValue::Unknown(i)) => i.clone(),
                        _ => missing(
                            "prediction_frontier_unknown",
                            "Actual terminal prediction has no proved computed prefix",
                        ),
                    };
                    ExecutionAcceptedMetric {
                        scope: sc.clone(),
                        accepted_quantity: None,
                        missing_conditions: vec![reason],
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    let reached = if ps.len() == 1 {
        calculated
            .iter()
            .find(|m| matches!(m.metric, MetricKey::CompletionTime))
            .and_then(|m| match &m.values[0].value {
                MetricValue::Duration(t) => Some(t.0.clone()),
                _ => None,
            })
    } else {
        None
    };
    let waiting=objects.iter().map(|o|{
        let v=calculated.iter().find(|m|matches!(m.metric,MetricKey::WaitingTime)&&m.object_ref.as_ref().is_some_and(|x|same(x,o))).map(|m|&m.values[0].value);
        let(known,missing_conditions)=match v{Some(MetricValue::Duration(t))=>(Some(t.0.clone()),vec![]),Some(MetricValue::Unknown(i))=>(None,vec![i.clone()]),_=>(None,vec![missing("prediction_wait_unobserved","No actual computed prefix/object coverage; absence is not zero waiting")])};
        let source_intervals=r.waits.iter().filter(|w|same(&w.object_ref,o)||(matches!(o.kind,ObjectKind::Transport)&&matches!(w.object_ref.kind,ObjectKind::Job)&&w.object_ref.id==o.id)||matches!(&w.execution_ref,Some(JobExecutionRef::ProjectDevice{device_id,..}) if matches!(o.kind,ObjectKind::Device)&&*device_id==o.id)).cloned().collect();
        PredictionWaitingMetric{object_ref:o.clone(),covered_from_sim_ms:r.window.start_ms.0.clone(),covered_until_sim_ms:r.computed_until_ms.as_ref().map(|t|t.0.clone()),known_sim_ms:known,basis:PredictionWaitingMetricBasis::ModelObjectIntervalUnion,source_intervals,missing_conditions}
    }).collect::<Vec<_>>();
    if r.computed_until_ms.is_none() {
        limits.push(missing(
            "prediction_frontier_unknown",
            "Terminal source retained; all computed-prefix-dependent metrics remain unknown",
        ));
    }
    let summary = decode::<PredictionRunSummary>(
        json!({"source":reference,"provenance":"model_prediction","evidence_origin":"actual_software","clock_basis":"model_virtual_ms","time_reference":p.scenario.time_reference,"window":r.window,"computed_until_sim_ms":r.computed_until_ms.as_ref().map(|t|t.0.clone()),"computation_status":r.computation_status,"whole_window_computed":r.whole_window_computed,"goal_met":r.goal_met,"scope":r.scope,"module_versions":module_versions(&p.definitions),"target_quantity":target,"accepted":accepted,"target_reached_at_sim_ms":reached,"missing_conditions":limits,"waiting":waiting}),
    )?;
    let input = normalized(&p.project, &["revision", "title", "views"])
        == normalized(&f.project, &["revision", "title", "views"])
        && normalized(
            &p.scenario,
            &["revision", "project_revision", "title", "window"],
        ) == normalized(
            &f.scenario,
            &["revision", "project_revision", "title", "window"],
        )
        && sorted(&p.definitions) == sorted(&f.execution_definitions)
        && sorted(&r.scope) == sorted(&f.scope);
    let models = binding_history_covered(e, f)
        && f.candidate_bindings.iter().all(|b| {
            f.project
                .devices
                .iter()
                .any(|d| d.id == b.device_id && same(&d.module_ref, &b.device_module_ref))
                && p.definitions.iter().any(|d| {
                    d.id == b.device_module_ref.id && d.version == b.device_module_ref.version
                })
                && p.definitions
                    .iter()
                    .any(|d| d.id == b.operation_ref.id && d.version == b.operation_ref.version)
                && matches!(b.execution_capacity_ref.kind, ObjectKind::Resource)
                && p.project
                    .resources
                    .iter()
                    .any(|r| r.id == b.execution_capacity_ref.id)
                && b.resources.iter().all(|binding| {
                    p.project
                        .resources
                        .iter()
                        .any(|r| r.id == binding.resource_id)
                })
                && b.input_ports.iter().chain(&b.output_ports).all(|port| {
                    p.project.locations.iter().any(|l| l.id == port.location_id)
                        && p.project
                            .item_types
                            .iter()
                            .any(|i| i.id == port.item_type_id && i.unit == port.unit_id)
                })
        });
    let fixed = summary
        .target_quantity
        .as_ref()
        .is_some_and(|q| scaled(q) > 0 && quantity_equal(q, &e.metrics.target_quantity));
    let sc = if ps.len() == 1
        && e.metrics.accepted.len() == 1
        && same(&ps[0], &e.metrics.accepted[0].scope)
    {
        Some(ps[0].clone())
    } else {
        None
    };
    let clock = r.window.start_ms.0.value() == 0
        && e.snapshot.coverage.clock_started
        && same(&p.scenario.time_reference, &f.scenario.time_reference);
    let availability =
        e.snapshot.coverage.data_quality_issues.is_empty() && binding_history_covered(e, f);
    let common = input && models && fixed && sc.is_some() && clock && availability;
    let settled = terminal(&e.snapshot)
        && matches!(
            e.snapshot.goal_at_snapshot,
            ExecutionSnapshotGoalAtSnapshot::Satisfied
        );
    let window = r.whole_window_computed
        && r.computed_until_ms
            .as_ref()
            .is_some_and(|t| same(t, &r.window.end_ms))
        && e.snapshot.coverage.observed_until_host_ms.value() == r.window.end_ms.0.value();
    let completion = if common && settled {
        time_delta(
            summary.target_reached_at_sim_ms.as_ref(),
            e.metrics.target_reached_at_host_ms.as_ref(),
        )
    } else {
        None
    };
    let quantity = if common && window {
        summary.accepted[0]
            .accepted_quantity
            .as_ref()
            .zip(e.metrics.accepted[0].accepted_quantity.as_ref())
            .and_then(|(a, b)| delta(a, b))
    } else {
        None
    };
    decode(
        json!({"prediction":summary,"execution":p.execution,"comparison_scope":sc,"status":if completion.is_some()||quantity.is_some(){"observational"}else{"side_by_side_only"},
        "conditions":[condition("project_scope",true,"Exact original Project Run/source identities admitted"),condition("frozen_input_content",input,"Actual typed inputs compared; only revision/title/views metadata ignored"),condition("model_bindings",models,"Frozen model devices and exact definitions preserved"),condition("fixed_target",fixed,"Same positive exact fixed target and unit"),condition("acceptance_scope",sc.is_some(),"Unique matching product revision/unit/destination/stage"),condition("clock_basis",clock,"Model origin zero and started Host epoch with same time reference"),condition("observation_window",window,"Quantity needs fully computed prediction and exact same observed endpoint"),condition("prediction_goal",summary.target_reached_at_sim_ms.is_some(),"Original actual acceptance prefix proves target time"),condition("execution_goal_and_settlement",settled,"Execution target and obligations settled"),condition("availability_record",availability,"Actual fixed-F device history and data-quality conditions retained"),condition("waiting_basis",false,"Model object interval union and Host logical wait states differ; shown separately without unsupported waiting improvement")],
        "completion_delta_ms":completion,"accepted_delta":quantity,"limitations":[missing("prediction_observational_deviation","Execution Host time minus model virtual time is prediction deviation, not AI speedup, cost improvement or physical effective work"),missing("waiting_not_equivalent","Actual model wait intervals and Host logical waiting are source-qualified, not directly subtracted"),missing("prediction_unknown_conditions","A null deviation retains missing common/goal/coverage conditions, unknown original metrics or unrepresentable exact arithmetic; no zero/float substitute is used")]}),
    )
}
/// Refuse machine-local/private context rather than copying it into every format.
/// phase_token is public execution identity, not an access credential.
pub(crate) fn check_export_content(v: &Value) -> Result<(), Vec<Issue>> {
    fn walk(v: &Value, key: &str) -> bool {
        match v {
            Value::Object(m) => m.iter().any(|(k, v)| {
                matches!(
                    k.as_str(),
                    "access_token"
                        | "refresh_token"
                        | "authorization"
                        | "grants"
                        | "selected_file"
                        | "selected_file_ref"
                        | "device_journal"
                        | "ipc_handle"
                ) || walk(v, k)
            }),
            Value::Array(a) => a.iter().any(|v| walk(v, key)),
            Value::String(s) => {
                let b = s.as_bytes();
                b.windows(3).enumerate().any(|(n, w)| {
                    w[0].is_ascii_alphabetic()
                        && w[1] == b':'
                        && matches!(w[2], b'/' | b'\\')
                        && (n == 0
                            || b[n - 1].is_ascii_whitespace()
                            || matches!(b[n - 1], b'\'' | b'"' | b'(' | b'=' | b'`')
                            || !b[n - 1].is_ascii())
                }) || s.starts_with("\\\\")
                    || ["/Users/", "/home/", "/tmp/", "/var/", "/etc/"]
                        .iter()
                        .any(|p| {
                            s.match_indices(p).any(|(n, _)| {
                                n == 0
                                    || b[n - 1].is_ascii_whitespace()
                                    || matches!(b[n - 1], b'\'' | b'"' | b'(' | b'=' | b'`')
                                    || !b[n - 1].is_ascii()
                            })
                        })
            }
            _ => false,
        }
    }
    if walk(v, "") {
        Err(reject(
            "Report contains machine-local/private source content; Host must provide export-safe facts",
        ))
    } else {
        Ok(())
    }
}
pub fn build_execution_report(input: ExecutionReportInput) -> Result<ExecutionReport, Vec<Issue>> {
    if !(1..=2).contains(&input.sessions.len()) {
        return Err(reject(
            "Exactly one or two distinct execution sources required",
        ));
    }
    let mut identities = BTreeSet::new();
    let mut reports = vec![];
    for s in &input.sessions {
        if !identities.insert((
            s.source.identity.session_id.clone(),
            s.source.identity.execution_epoch_id.clone(),
        )) {
            return Err(reject(
                "Duplicate session/epoch is not an independent comparison source",
            ));
        }
        validate_session(s, &input.project_id)?;
        let (accepted, reached, mut missing_conditions) = accepted(s);
        missing_conditions.extend(s.snapshot.coverage.data_quality_issues.clone());
        let unresolved: BTreeSet<_> = s
            .snapshot
            .lifecycle
            .unresolved_command_ids
            .iter()
            .map(|x| x.as_str())
            .chain(
                s.snapshot
                    .attempts
                    .iter()
                    .filter(|a| {
                        matches!(
                            a.state,
                            AttemptViewState::Reserved
                                | AttemptViewState::Published
                                | AttemptViewState::DeviceAccepted
                                | AttemptViewState::DeviceStarted
                                | AttemptViewState::ReconciliationRequired
                                | AttemptViewState::DeviceFailed
                        )
                    })
                    .map(|a| a.attempt.command_id.as_str()),
            )
            .collect();
        let commands = ExecutionCommandMetrics {
            attempt_count: number(s.snapshot.attempts.len() as u128).unwrap(),
            started_attempt_count: number(
                s.snapshot
                    .attempts
                    .iter()
                    .filter(|a| a.started_at_host_ms.is_some())
                    .count() as u128,
            )
            .unwrap(),
            device_completed_attempt_count: number(
                s.snapshot
                    .attempts
                    .iter()
                    .filter(|a| a.completed_at_host_ms.is_some())
                    .count() as u128,
            )
            .unwrap(),
            accepted_logical_work_count: number(
                s.snapshot
                    .logical_work
                    .iter()
                    .filter(|w| matches!(w.state, LogicalWorkViewState::Accepted))
                    .count() as u128,
            )
            .unwrap(),
            unresolved_command_count: number(unresolved.len() as u128).unwrap(),
        };
        let asset_rights = s
            .inputs
            .modules
            .iter()
            .flat_map(|m| {
                m.rights.iter().map(|r| ExecutionReportAssetRights {
                    module_ref: ModuleRef {
                        id: m.definition.id.clone(),
                        version: m.definition.version.clone(),
                    },
                    rights: r.clone(),
                })
            })
            .collect();
        reports.push(ExecutionReportSession {
            source: s.source.clone(),
            project_title: s.inputs.project.title.clone(),
            scenario_title: s.inputs.scenario.title.clone(),
            module_versions: module_versions(&s.inputs.execution_definitions),
            asset_rights,
            snapshot: s.snapshot.clone(),
            history: s.history.clone(),
            metrics: ExecutionReportMetrics {
                accepted,
                target_quantity: s.snapshot.policy.target_quantity.clone(),
                target_reached_at_host_ms: reached,
                commands,
                waiting: waiting(s),
                resources: resources(s)?,
                missing_conditions,
            },
        });
    }
    let compare = if reports.len() == 2 {
        Some(comparison(
            &reports[0],
            &reports[1],
            &input.sessions[0].inputs,
            &input.sessions[1].inputs,
        )?)
    } else {
        None
    };
    let predicted = input
        .prediction
        .map(|p| prediction(p, &reports, &input.sessions, &input.project_id))
        .transpose()?;
    let report = ExecutionReport {
        document_kind: ExecutionReportDocumentKind::ExecutionReport,
        contract_version: ExecutionReportContractVersion::SfExecution20Draft11,
        report_id: input.report_id,
        project_id: input.project_id,
        export_project_revision: input.export_project_revision,
        generated_at_utc: input.generated_at_utc,
        sessions: reports,
        comparison: compare,
        conclusions: vec![
            "接受量来自冻结范围内的实际物料验收；设备完成、逻辑工作及接受量分别报告。".into(),
            "Host等待、资源占用与预测模型等待分别标源；任何差值均为观察比较，不表示因果改善。"
                .into(),
        ],
        limitations: vec![missing(
            "execution_evidence_scope",
            "软件在环或物理设备来源以每个快照provenance为准；不将Host确认占用当物理有效工时",
        )],
        prediction_comparison: predicted,
    };
    check_export_content(&value(&report))?;
    Ok(report)
}
