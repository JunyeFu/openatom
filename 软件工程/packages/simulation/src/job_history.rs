//! Read actual, closed-instant Job states without replaying production.
use sf_domain::*;
use std::collections::BTreeMap;

/// Sparse Job DTO changes at completed virtual instants, with effective work
/// supplied by the same Run. None is outside its actual computed frontier.
/// An absent legacy history must be handled as unknown by the caller, not
/// replaced by final Job states.
pub fn jobs_at_from_history(
    records: &[(SimTimeMs, JobState)],
    work: &[PhaseWorkInterval],
    window_start: SimTimeMs,
    computed_until: Option<SimTimeMs>,
    at: SimTimeMs,
) -> Option<Vec<JobState>> {
    let at = at.0.value();
    if at < window_start.0.value() || at > computed_until?.0.value() {
        return None;
    }
    let mut states = BTreeMap::new();
    for (when, state) in records {
        if when.0.value() <= at {
            states.insert(state.id.clone(), state.clone());
        }
    }
    for state in states.values_mut() {
        if !matches!(state.state, JobStatus::Working) {
            continue;
        }
        if let Some(interval) = work.iter().find(|row| {
            row.job_id == state.id
                && state.phase_ref.as_ref() == Some(&row.phase_id)
                && row.interval.start_ms.0.value() <= at
                && at <= row.interval.end_ms.0.value()
        }) {
            // At a computed boundary an active Job still has the actual
            // end-point progress; completed/interrupted states come from the
            // closed-instant record above and are never advanced here.
            state.effective_progress_ms = DurationMs(
                (interval.effective_progress_start_ms.0.value() + at
                    - interval.interval.start_ms.0.value())
                .try_into()
                .unwrap(),
            );
        }
    }
    Some(states.into_values().collect())
}

/// Immutable execution identity is fixed for each RunningJob. Running progress
/// is read from actual work intervals rather than duplicating a state record at
/// every unrelated event. Non-running progress (e.g. an interruption) is kept.
pub(crate) fn same_observable_state(a: &JobState, b: &JobState) -> bool {
    a.input_refs == b.input_refs
        && a.job_resources == b.job_resources
        && a.phase_ref == b.phase_ref
        && a.phase_resources == b.phase_resources
        && (matches!(a.state, JobStatus::Working)
            || a.effective_progress_ms.0.value() == b.effective_progress_ms.0.value())
        && a.started_at_ms.as_ref().map(|t| t.0.value())
            == b.started_at_ms.as_ref().map(|t| t.0.value())
        && a.finished_at_ms.as_ref().map(|t| t.0.value())
            == b.finished_at_ms.as_ref().map(|t| t.0.value())
        && std::mem::discriminant(&a.state) == std::mem::discriminant(&b.state)
        && a.pending_outputs == b.pending_outputs
}
