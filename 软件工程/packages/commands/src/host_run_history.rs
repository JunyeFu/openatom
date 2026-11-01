//! Read the actual Run frontier; retained Job records never come from final jobs.
use super::*;

// HostRun is fixed after insertion. Keep only its latest actual timepoint,
// so paged readers do not replay the entire history for every200 selectors.
#[derive(Default)]
pub(super) struct ActualObjectCache {
    at_ms: Option<u64>,
    items: Option<Vec<RuntimeItem>>,
    jobs: Option<Vec<JobState>>,
    job_identities: Option<BTreeMap<ObjectId, Vec<usize>>>,
    item_records: Option<RecordIndex>,
    job_records: Option<RecordIndex>,
    job_work: Option<BTreeMap<ObjectId, Vec<usize>>>,
    // The Run and its frozen inputs do not change when the viewing time changes.
    pub(super) port_sources: BTreeMap<String, super::host_port_output::PortOutputSource>,
}

// At each time choose the last original record whose time is admitted. Keeping
// prefix maxima of original offsets also preserves the old scan's tie/order rule.
struct RecordIndex(BTreeMap<ObjectId, Vec<(u64, usize)>>);

impl RecordIndex {
    fn new<T>(records: &[(SimTimeMs, T)], id: impl Fn(&T) -> &ObjectId) -> Self {
        let mut values: BTreeMap<ObjectId, Vec<(u64, usize)>> = BTreeMap::new();
        for (offset, (at, value)) in records.iter().enumerate() {
            values
                .entry(id(value).clone())
                .or_default()
                .push((at.0.value(), offset));
        }
        for rows in values.values_mut() {
            rows.sort_unstable();
            let mut last = None;
            rows.retain(|(_, offset)| {
                if last.is_some_and(|previous| *offset <= previous) {
                    return false;
                }
                last = Some(*offset);
                true
            });
        }
        Self(values)
    }

    fn at(&self, at: u64) -> impl Iterator<Item = usize> + '_ {
        self.0.values().filter_map(move |rows| {
            rows.partition_point(|(when, _)| *when <= at)
                .checked_sub(1)
                .map(|index| rows[index].1)
        })
    }
}

impl ActualObjectCache {
    pub(super) fn matches_job_identity(
        &mut self,
        jobs: &[JobState],
        id: &ObjectId,
        execution: &JobExecutionRef,
    ) -> bool {
        let index = self.job_identities.get_or_insert_with(|| {
            let mut index: BTreeMap<ObjectId, Vec<usize>> = BTreeMap::new();
            for (offset, job) in jobs.iter().enumerate() {
                index.entry(job.id.clone()).or_default().push(offset);
            }
            index
        });
        index.get(id).is_some_and(|offsets| {
            offsets
                .iter()
                .any(|offset| exact_data(&jobs[*offset].execution_ref, execution))
        })
    }
}

pub(super) fn objects_at(
    run: &HostRun,
    run_id: RunId,
    at_ms: SimTimeMs,
    objects: Vec<ObjectRef>,
    page: PageRequest,
) -> Result<RunView, ApiError> {
    let values = actual_values(run, &at_ms, &objects, false)?;
    let (objects, next_index) = page_values(values, page)?;
    Ok(RunView::Objects(RunObjectsPage {
        run_id,
        at_ms,
        objects,
        next_index,
    }))
}

fn actual_values(
    run: &HostRun,
    at_ms: &SimTimeMs,
    objects: &[ObjectRef],
    all: bool,
) -> Result<Vec<DomainObject>, ApiError> {
    if at_ms.0.value() < run.result.window.start_ms.0.value()
        || run
            .result
            .computed_until_ms
            .as_ref()
            .is_none_or(|frontier| at_ms.0.value() > frontier.0.value())
    {
        return Err(error(
            ErrorCode::InvalidInput,
            "Requested time is outside the computed Run frontier",
            Some("/params/query/data/at_ms"),
        ));
    }
    if objects
        .iter()
        .any(|object| !matches!(object.kind, ObjectKind::Item | ObjectKind::Job))
    {
        return Err(unsupported(
            "ObjectsAt supports explicit computed runtime Items and Jobs",
        ));
    }
    let mut cache = run.observation_cache.lock().unwrap();
    if cache.at_ms != Some(at_ms.0.value()) {
        cache.at_ms = Some(at_ms.0.value());
        cache.items = None;
        cache.jobs = None;
    }
    let mut values = vec![];
    // An empty selector retains the original Item-only default and page order.
    if objects.is_empty()
        || objects
            .iter()
            .any(|object| matches!(object.kind, ObjectKind::Item))
    {
        if cache.items.is_none() {
            let items = run
                .engine
                .as_ref()
                .and_then(|engine| engine.objects_at(at_ms.clone()))
                .or_else(|| {
                    run.history.as_ref().map(|history| {
                        let index = cache.item_records.get_or_insert_with(|| {
                            RecordIndex::new(&history.records, |item| &item.id)
                        });
                        index
                            .at(at_ms.0.value())
                            .map(|offset| history.records[offset].1.clone())
                            .collect()
                    })
                });
            cache.items = Some(items.ok_or_else(|| {
                error(
                    ErrorCode::InvalidInput,
                    "Requested time has no actual stored Item history",
                    Some("/params/query/data/at_ms"),
                )
            })?);
        }
        values.extend(
            cache
                .items
                .as_ref()
                .unwrap()
                .iter()
                .filter(|item| {
                    objects.is_empty()
                        || objects.iter().any(|object| {
                            matches!(object.kind, ObjectKind::Item) && object.id == item.id
                        })
                })
                .cloned()
                .map(DomainObject::Item),
        );
    }
    let selected_jobs: BTreeSet<_> = objects
        .iter()
        .filter(|object| matches!(object.kind, ObjectKind::Job))
        .map(|object| &object.id)
        .collect();
    if all || !selected_jobs.is_empty() {
        if cache.jobs.is_none() {
            cache.jobs = Some(match &run.engine {
            Some(engine) => engine.jobs_at(at_ms.clone()),
            None => {
                let records = run.history.as_ref()
                    .and_then(|history| history.job_records.as_deref())
                    .ok_or_else(|| error(
                        ErrorCode::UnsupportedCapability,
                        "This stored Run has no actual Job history; final Job states cannot substitute for unknown history",
                        Some("/params/query/data/objects"),
                    ))?;
                let offsets: Vec<_> = cache.job_records.get_or_insert_with(|| RecordIndex::new(records, |job| &job.id)).at(at_ms.0.value()).collect();
                let work = cache.job_work.get_or_insert_with(|| {
                    let mut values: BTreeMap<ObjectId, Vec<usize>> = BTreeMap::new();
                    for (offset, row) in run.result.phase_work_intervals.iter().enumerate() {
                        values.entry(row.job_id.clone()).or_default().push(offset);
                    }
                    values
                });
                Some(offsets.into_iter().map(|offset| {
                    let mut state = records[offset].1.clone();
                    if matches!(state.state, JobStatus::Working) {
                        if let Some(interval) = work.get(&state.id).into_iter().flatten()
                            .map(|offset| &run.result.phase_work_intervals[*offset])
                            .find(|row| state.phase_ref.as_ref() == Some(&row.phase_id)
                                && row.interval.start_ms.0.value() <= at_ms.0.value()
                                && at_ms.0.value() <= row.interval.end_ms.0.value()) {
                            state.effective_progress_ms = DurationMs((interval.effective_progress_start_ms.0.value()
                                + at_ms.0.value() - interval.interval.start_ms.0.value()).try_into().unwrap());
                        }
                    }
                    state
                }).collect())
            }
        }.ok_or_else(|| error(
            ErrorCode::InvalidInput,
            "Requested time is outside the computed Job frontier",
            Some("/params/query/data/at_ms"),
        ))?);
        }
        values.extend(
            cache
                .jobs
                .as_ref()
                .unwrap()
                .iter()
                .filter(|job| all || selected_jobs.contains(&job.id))
                .cloned()
                .map(DomainObject::Job),
        );
    }
    Ok(values)
}

impl ProjectHost {
    /// Private desktop composition of the same actual ItemsAt/JobsAt source.
    /// Public GetRun selectors and pagination are unchanged.
    pub fn read_run_observation_ui(
        &self,
        session_id: &ObjectId,
        project_id: &ProjectId,
        scope: &ApiScope,
        scope_revision: &Revision,
        run_id: &RunId,
        at_ms: &SimTimeMs,
    ) -> Result<RunObjectsPage, ApiError> {
        let caller = self.sessions.get(session_id).ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Complete scene observation requires the actual UI session",
                None,
            ));
        }
        self.admit(
            caller,
            &Some(project_id.clone()),
            scope,
            &OperationName::GetRun,
        )?;
        let revision = match scope {
            ApiScope::Project => &self.project.as_ref().unwrap().revision,
            ApiScope::DefinitionDraft(id) => &self.definition_drafts[id].revision,
            ApiScope::Workspace => {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Scene observation requires a Project or DefinitionDraft scope",
                    Some("/scope"),
                ));
            }
        };
        check_revision(scope_revision, revision, "/scope_revision")?;
        // Retain GetRun's exact edit-scope ownership gate before using its Run.
        self.get_run(
            caller,
            scope,
            GetRunParams {
                run_id: run_id.clone(),
                query: RunQuery::Summary,
            },
        )?;
        let objects = actual_values(&self.runs[run_id], at_ms, &[], true)?;
        Ok(RunObjectsPage {
            run_id: run_id.clone(),
            at_ms: at_ms.clone(),
            objects,
            next_index: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_index_retains_original_last_admitted_record_and_reverse_seeks() {
        let records: Vec<(SimTimeMs, ObjectId)> =
            [(4, "A"), (10, "B"), (1, "A"), (7, "A"), (4, "A")]
                .into_iter()
                .map(|(time, id)| {
                    (
                        serde_json::from_value(serde_json::json!(time)).unwrap(),
                        serde_json::from_value(serde_json::json!(id)).unwrap(),
                    )
                })
                .collect();
        let index = RecordIndex::new(&records, |id| id);
        for at in [0, 1, 4, 7, 10, 12, 4, 0] {
            let mut expected = BTreeMap::new();
            for (offset, (when, id)) in records.iter().enumerate() {
                if when.0.value() <= at {
                    expected.insert(id.clone(), offset);
                }
            }
            assert_eq!(
                index.at(at).collect::<Vec<_>>(),
                expected.into_values().collect::<Vec<_>>()
            );
        }
    }
}
