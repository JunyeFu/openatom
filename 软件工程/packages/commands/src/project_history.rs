//! Pure session draft history. Host owns authorization, transaction/revision
//! assignment, creation ledgers, tasks, I/O, Runs and nonreversible effects.
//! Project and all current Scenario documents are restored together. Scenario
//! revision high-water marks and derived issues remain Host-owned.
use crate::{ApiError, ErrorCode, HistoryAvailability, ProjectDraft};
use sf_domain::{ProjectId, Revision};

#[derive(Debug)]
struct HistoryEntry {
    before: ProjectDraft,
    after: ProjectDraft,
}

#[derive(Debug)]
pub struct ProjectHistory {
    head: Option<(ProjectId, Revision)>,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    professional_boundary_reason: Option<String>,
}

impl ProjectHistory {
    pub fn new() -> Self {
        Self {
            head: None,
            undo: Vec::new(),
            redo: Vec::new(),
            professional_boundary_reason: None,
        }
    }

    /// Start a new session history for a newly created/opened project.
    /// Saving the current project does not call reset.
    pub fn reset(&mut self, draft: &ProjectDraft) {
        self.head = Some((draft.project.id.clone(), draft.project.revision.clone()));
        self.undo.clear();
        self.redo.clear();
        self.professional_boundary_reason = None;
    }

    /// Called only after Host has committed an ordinary reversible edit.
    pub fn record(&mut self, before: &ProjectDraft, after: &ProjectDraft) {
        self.assert_commit(before, after);
        if crate::project_professional::professional_changed(before, after) {
            self.professional_boundary(after);
            return;
        }
        self.undo.push(HistoryEntry {
            before: before.clone(),
            after: after.clone(),
        });
        self.redo.clear();
        self.head = Some((after.project.id.clone(), after.project.revision.clone()));
        self.professional_boundary_reason = None;
    }

    pub fn undo_candidate(&self, current: &ProjectDraft) -> Result<ProjectDraft, ApiError> {
        self.check_current(current)?;
        let entry = self
            .undo
            .last()
            .ok_or_else(|| unavailable("No undo candidate in this session"))?;
        let mut candidate = entry.before.clone();
        retain_professional(current, &mut candidate);
        candidate.project.revision = current.project.revision.clone();
        Ok(candidate)
    }

    pub fn redo_candidate(&self, current: &ProjectDraft) -> Result<ProjectDraft, ApiError> {
        self.check_current(current)?;
        let entry = self
            .redo
            .last()
            .ok_or_else(|| unavailable("No redo candidate in this session"))?;
        let mut candidate = entry.after.clone();
        retain_professional(current, &mut candidate);
        candidate.project.revision = current.project.revision.clone();
        Ok(candidate)
    }

    /// Candidate validation/authorization may fail without calling this method.
    /// Only a successful Host transaction moves the history entry.
    pub fn commit_undo(&mut self, before: &ProjectDraft, after: &ProjectDraft) {
        self.assert_commit(before, after);
        let entry = self
            .undo
            .pop()
            .expect("Host must obtain an undo candidate before committing");
        self.redo.push(entry);
        self.head = Some((after.project.id.clone(), after.project.revision.clone()));
    }

    pub fn commit_redo(&mut self, before: &ProjectDraft, after: &ProjectDraft) {
        self.assert_commit(before, after);
        let entry = self
            .redo
            .pop()
            .expect("Host must obtain a redo candidate before committing");
        self.undo.push(entry);
        self.head = Some((after.project.id.clone(), after.project.revision.clone()));
    }

    pub fn availability(&self) -> HistoryAvailability {
        HistoryAvailability {
            can_undo: !self.undo.is_empty(),
            can_redo: !self.redo.is_empty(),
            boundary_reason: if self.head.is_none() {
                Some("No active project history".into())
            } else if self.undo.is_empty()
                && self.redo.is_empty()
                && self.professional_boundary_reason.is_some()
            {
                self.professional_boundary_reason.clone()
            } else if self.undo.is_empty() && self.redo.is_empty() {
                Some("No reversible draft edits in this session".into())
            } else {
                None
            },
        }
    }

    /// Actual human recognition/explicit binding is not ordinary undo. Host
    /// also calls this for a real recognition event with unchanged wire fields.
    pub fn professional_boundary(&mut self, draft: &ProjectDraft) {
        self.reset(draft);
        self.professional_boundary_reason = Some("Professional facts, human acknowledgment and explicit binding are outside ordinary undo".into());
    }

    pub fn module_import_boundary(&mut self, draft: &ProjectDraft) {
        self.reset(draft);
        self.professional_boundary_reason = Some("导入的不可变模块版本不属于工程草稿撤销范围。".into());
    }

    fn check_current(&self, current: &ProjectDraft) -> Result<(), ApiError> {
        let Some((project, revision)) = &self.head else {
            return Err(unavailable(
                "Project history has not been reset for this session",
            ));
        };
        if project != &current.project.id {
            // Do not expose the old project's identity or history revision.
            return Err(unavailable(
                "Current project does not own this session history",
            ));
        }
        if revision.0.value() != current.project.revision.0.value() {
            return Err(ApiError {
                code: ErrorCode::RevisionConflict,
                message: "Current project revision does not match the history head".into(),
                object_refs: Vec::new(),
                field_errors: Vec::new(),
                expected_revision: Some(current.project.revision.clone()),
                actual_revision: Some(revision.clone()),
            });
        }
        Ok(())
    }

    // These callbacks have the fixed infallible private API. Their inputs are
    // trusted, already committed Host transitions, not unvalidated requests.
    fn assert_commit(&self, before: &ProjectDraft, after: &ProjectDraft) {
        self.check_current(before)
            .expect("Host history commit must use the current project/head");
        assert_eq!(
            before.project.id, after.project.id,
            "History cannot commit across projects"
        );
        assert!(
            after.project.revision.0.value() > before.project.revision.0.value(),
            "Host must assign a fresh revision before history commit"
        );
    }
}

fn retain_professional(current: &ProjectDraft, candidate: &mut ProjectDraft) {
    candidate.professional = current.professional.clone();
    candidate.project.artifact_refs = current.project.artifact_refs.clone();
    candidate.project.acknowledgments = current.project.acknowledgments.clone();
    candidate.project.bindings = current.project.bindings.clone();
}

fn unavailable(message: &str) -> ApiError {
    ApiError {
        code: ErrorCode::InvalidInput,
        message: message.into(),
        object_refs: Vec::new(),
        field_errors: Vec::new(),
        expected_revision: None,
        actual_revision: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_domain::{ProjectState, SafeUint, ScenarioId, ScenarioInput, Versioned};
    use std::collections::BTreeMap;

    fn revision(value: u64) -> Revision {
        Revision(SafeUint::try_from(value).unwrap())
    }

    fn project(id: &str, title: &str, value: u64) -> ProjectDraft {
        // Canonical complete ProjectState; this fixture is read-only. Tests do
        // not need the concurrently implemented Host/store or a fake domain DTO.
        let mut project: ProjectState =
            serde_json::from_str::<Versioned<ProjectState>>(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tools/type-contract-proof/fixtures/project.json"
            )))
            .unwrap()
            .data;
        project.id = id.to_owned().try_into().unwrap();
        project.title = title.into();
        project.revision = revision(value);
        project.scenario_refs.clear();
        ProjectDraft {
            project,
            scenarios: BTreeMap::new(),
            professional: Default::default(),
        }
    }

    fn next(current: &ProjectDraft, title: &str, value: u64) -> ProjectDraft {
        let mut next = current.clone();
        next.project.title = title.into();
        next.project.revision = revision(value);
        next
    }

    fn add_scenario(draft: &mut ProjectDraft, id: &str, title: &str, priority: i32) {
        let mut scenario = serde_json::from_str::<Versioned<ScenarioInput>>(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../tools/type-contract-proof/fixtures/scenario.json"
        )))
        .unwrap()
        .data;
        let id: ScenarioId = id.to_owned().try_into().unwrap();
        scenario.id = id.clone();
        scenario.title = title.into();
        scenario.revision = revision(1);
        scenario.project_revision = draft.project.revision.clone();
        scenario.orders[0].priority = priority;
        draft.project.scenario_refs.push(id.clone());
        draft.scenarios.insert(id, scenario);
    }

    fn same(left: &ProjectDraft, right: &ProjectDraft) {
        assert_eq!(
            serde_json::to_value(&left.project).unwrap(),
            serde_json::to_value(&right.project).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&left.scenarios).unwrap(),
            serde_json::to_value(&right.scenarios).unwrap()
        );
    }

    fn conflict(error: ApiError, expected: u64, actual: u64) {
        assert!(matches!(error.code, ErrorCode::RevisionConflict));
        assert_eq!(error.expected_revision.unwrap().0.value(), expected);
        assert_eq!(error.actual_revision.unwrap().0.value(), actual);
    }

    #[test]
    fn empty_history_and_reset_never_invent_an_undo() {
        let mut history = ProjectHistory::new();
        let current = project("P-HISTORY", "initial", 1);
        assert!(!history.availability().can_undo);
        assert!(!history.availability().can_redo);
        assert!(matches!(
            history.undo_candidate(&current).unwrap_err().code,
            ErrorCode::InvalidInput
        ));
        history.reset(&current);
        assert!(matches!(
            history.redo_candidate(&current).unwrap_err().code,
            ErrorCode::InvalidInput
        ));
        assert!(!history.availability().can_undo && !history.availability().can_redo);
    }

    #[test]
    fn candidates_and_revision_failures_leave_history_unconsumed() {
        let mut history = ProjectHistory::new();
        let a = project("P-HISTORY", "A", 1);
        let b = next(&a, "B", 2);
        history.reset(&a);
        history.record(&a, &b);
        let candidate = history.undo_candidate(&b).unwrap();
        same(&candidate, &next(&a, "A", 2));
        // Host could reject authorization or the candidate after this point.
        same(&history.undo_candidate(&b).unwrap(), &candidate);
        let stale = next(&b, "B", 1);
        conflict(history.undo_candidate(&stale).unwrap_err(), 1, 2);
        assert!(history.availability().can_undo && !history.availability().can_redo);
        same(&history.undo_candidate(&b).unwrap(), &candidate);
        let committed = next(&candidate, "A", 3);
        history.commit_undo(&b, &committed);
        assert!(!history.availability().can_undo && history.availability().can_redo);
        // Check the head before empty-stack handling, even after a valid undo.
        conflict(history.undo_candidate(&b).unwrap_err(), 2, 3);
        conflict(history.redo_candidate(&b).unwrap_err(), 2, 3);
        let redo = history.redo_candidate(&committed).unwrap();
        same(&redo, &next(&b, "B", 3));
        same(&history.redo_candidate(&committed).unwrap(), &redo);
    }

    #[test]
    fn host_supplies_monotonic_revisions_across_multiple_undo_and_redo() {
        let mut history = ProjectHistory::new();
        let a = project("P-HISTORY", "A", 10);
        let b = next(&a, "B", 11);
        let c = next(&a, "C", 12);
        history.reset(&a);
        history.record(&a, &b);
        history.record(&b, &c);
        let candidate = history.undo_candidate(&c).unwrap();
        assert_eq!(candidate.project.revision.0.value(), 12);
        let undo_b = next(&candidate, "B", 100);
        history.commit_undo(&c, &undo_b);
        let candidate = history.undo_candidate(&undo_b).unwrap();
        same(&candidate, &next(&a, "A", 100));
        let undo_a = next(&candidate, "A", 101);
        history.commit_undo(&undo_b, &undo_a);
        let candidate = history.redo_candidate(&undo_a).unwrap();
        same(&candidate, &next(&b, "B", 101));
        let redo_b = next(&candidate, "B", 102);
        history.commit_redo(&undo_a, &redo_b);
        let candidate = history.redo_candidate(&redo_b).unwrap();
        same(&candidate, &next(&c, "C", 102));
        let redo_c = next(&candidate, "C", 103);
        history.commit_redo(&redo_b, &redo_c);
        assert!(history.availability().can_undo && !history.availability().can_redo);
    }

    #[test]
    fn an_ordinary_committed_branch_discards_redo_only_after_record() {
        let mut history = ProjectHistory::new();
        let a = project("P-HISTORY", "A", 1);
        let b = next(&a, "B", 2);
        history.reset(&a);
        history.record(&a, &b);
        let undone = next(&history.undo_candidate(&b).unwrap(), "A", 3);
        history.commit_undo(&b, &undone);
        assert!(history.availability().can_redo);
        let branch = next(&undone, "new branch", 4);
        history.record(&undone, &branch);
        assert!(!history.availability().can_redo);
        assert!(matches!(
            history.redo_candidate(&branch).unwrap_err().code,
            ErrorCode::InvalidInput
        ));
        same(&history.undo_candidate(&branch).unwrap(), &next(&a, "A", 4));
    }

    #[test]
    fn another_project_cannot_reuse_matching_revision_history() {
        let mut history = ProjectHistory::new();
        let a = project("P-OLD", "A", 1);
        let b = next(&a, "B", 2);
        history.reset(&a);
        history.record(&a, &b);
        let other = project("P-OTHER", "other", 2);
        let rejected = history.undo_candidate(&other).unwrap_err();
        assert!(matches!(rejected.code, ErrorCode::InvalidInput));
        assert!(rejected.expected_revision.is_none() && rejected.actual_revision.is_none());
        assert!(rejected.object_refs.is_empty());
        assert!(history.availability().can_undo && !history.availability().can_redo);
        same(&history.undo_candidate(&b).unwrap(), &next(&a, "A", 2));
    }

    #[test]
    fn reset_clears_old_project_history_and_establishes_the_new_head() {
        let mut history = ProjectHistory::new();
        let a = project("P-OLD", "old A", 1);
        let b = next(&a, "old B", 2);
        history.reset(&a);
        history.record(&a, &b);
        let other = project("P-NEW", "new A", 20);
        history.reset(&other);
        assert!(!history.availability().can_undo && !history.availability().can_redo);
        assert!(history.undo_candidate(&b).is_err());
        let edited = next(&other, "new B", 21);
        history.record(&other, &edited);
        same(
            &history.undo_candidate(&edited).unwrap(),
            &next(&other, "new A", 21),
        );
    }

    #[test]
    fn scenario_creation_and_removal_restore_documents_and_project_refs_together() {
        let mut history = ProjectHistory::new();
        let a = project("P-MULTI", "no scenarios", 1);
        let mut b = next(&a, "two scenarios", 2);
        add_scenario(&mut b, "SC-A", "scenario A", 10);
        add_scenario(&mut b, "SC-B", "scenario B", 20);
        history.reset(&a);
        history.record(&a, &b);

        let undo = history.undo_candidate(&b).unwrap();
        same(&undo, &next(&a, "no scenarios", 2));
        assert!(undo.project.scenario_refs.is_empty() && undo.scenarios.is_empty());
        let committed_empty = next(&undo, "no scenarios", 3);
        history.commit_undo(&b, &committed_empty);
        let redo = history.redo_candidate(&committed_empty).unwrap();
        same(&redo, &next(&b, "two scenarios", 3));
        assert_eq!(redo.project.scenario_refs.len(), 2);
        assert_eq!(redo.scenarios.len(), 2);
        let restored = next(&redo, "two scenarios", 4);
        history.commit_redo(&committed_empty, &restored);

        let removed_id = ScenarioId::try_from("SC-A".to_owned()).unwrap();
        let mut removed = next(&restored, "one scenario", 5);
        removed.scenarios.remove(&removed_id);
        removed.project.scenario_refs.retain(|id| id != &removed_id);
        history.record(&restored, &removed);
        let candidate = history.undo_candidate(&removed).unwrap();
        same(&candidate, &next(&restored, "two scenarios", 5));
        assert!(candidate.project.scenario_refs.contains(&removed_id));
        assert!(candidate.scenarios.contains_key(&removed_id));
        assert_eq!(candidate.scenarios.len(), 2);
    }

    #[test]
    fn scenario_content_and_failed_candidates_stay_atomic_without_assigning_revisions() {
        let mut history = ProjectHistory::new();
        let mut a = project("P-MULTI", "before", 1);
        add_scenario(&mut a, "SC-A", "before A", 10);
        add_scenario(&mut a, "SC-B", "before B", 20);
        let mut b = next(&a, "after", 2);
        let id_a = ScenarioId::try_from("SC-A".to_owned()).unwrap();
        let id_b = ScenarioId::try_from("SC-B".to_owned()).unwrap();
        b.scenarios.get_mut(&id_a).unwrap().orders[0].priority = 99;
        b.scenarios.get_mut(&id_b).unwrap().title = "after B".into();
        for scenario in b.scenarios.values_mut() {
            scenario.revision = revision(2);
            scenario.project_revision = revision(2);
        }
        history.reset(&a);
        history.record(&a, &b);
        let frozen_current = b.clone();
        let candidate = history.undo_candidate(&b).unwrap();
        same(&candidate, &next(&a, "before", 2));
        // No Host commit after a denied/invalid candidate: all documents and
        // the same next candidate remain available, not just scenario_refs.
        same(&b, &frozen_current);
        same(&history.undo_candidate(&b).unwrap(), &candidate);
        conflict(history.undo_candidate(&a).unwrap_err(), 1, 2);
        assert!(history.availability().can_undo && !history.availability().can_redo);
        same(&history.undo_candidate(&b).unwrap(), &candidate);

        // Host supplies both the Project revision and independent Scenario
        // high-water revisions. History must not manufacture or rewind them.
        let mut committed = next(&candidate, "before", 10);
        for (index, scenario) in committed.scenarios.values_mut().enumerate() {
            scenario.revision = revision(100 + index as u64);
            scenario.project_revision = revision(10);
        }
        history.commit_undo(&b, &committed);
        let redo = history.redo_candidate(&committed).unwrap();
        same(&redo, &next(&b, "after", 10));
        assert_eq!(redo.scenarios[&id_a].revision.0.value(), 2);
        assert_eq!(committed.scenarios[&id_a].revision.0.value(), 100);
        same(&history.redo_candidate(&committed).unwrap(), &redo);
    }

    #[test]
    fn professional_acknowledgment_commit_creates_a_nonreversible_history_boundary() {
        let mut history = ProjectHistory::new();
        let a = project("P-PROFESSIONAL", "before", 1);
        let b = next(&a, "ordinary edit", 2);
        history.reset(&a);
        history.record(&a, &b);
        let mut c = next(&b, "after explicit recognition", 3);
        let artifact =
            serde_json::from_str::<Versioned<sf_domain::ArtifactRecord>>(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tools/type-contract-proof/fixtures/artifact.json"
            )))
            .unwrap()
            .data;
        c.project
            .acknowledgments
            .push(sf_domain::ArtifactAcknowledgment {
                id: sf_domain::ObjectId::try_from("recognition-boundary-input".to_owned()).unwrap(),
                artifact_ref: artifact.id,
                accepted_subject: artifact.subject_ref,
                accepted_scope: vec!["design_reference".into()],
                decision_source: sf_domain::DecisionSource::HumanStatement(
                    "typed committed recognition".into(),
                ),
                conditions: vec![],
                recorded_at_iso8601: None,
            });
        history.record(&b, &c);
        assert!(!history.availability().can_undo && !history.availability().can_redo);
        assert!(
            history
                .availability()
                .boundary_reason
                .unwrap()
                .contains("outside ordinary undo")
        );
        assert!(history.undo_candidate(&c).is_err());
        let ordinary = next(&c, "later display change", 4);
        history.record(&c, &ordinary);
        let undo = history.undo_candidate(&ordinary).unwrap();
        assert_eq!(undo.project.title, c.project.title);
        assert_eq!(
            serde_json::to_value(&undo.project.acknowledgments).unwrap(),
            serde_json::to_value(&c.project.acknowledgments).unwrap()
        );
    }

    #[test]
    fn ordinary_draft_undo_keeps_immutable_professional_results_received_after_the_snapshot() {
        let mut history = ProjectHistory::new();
        let a = project("P-PROFESSIONAL", "before", 1);
        let mut b = next(&a, "ordinary edit", 2);
        history.reset(&a);
        history.record(&a, &b);
        let artifact =
            serde_json::from_str::<Versioned<sf_domain::ArtifactRecord>>(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../tools/type-contract-proof/fixtures/artifact.json"
            )))
            .unwrap()
            .data;
        let id = artifact.id.clone();
        b.professional.artifacts.insert(id.clone(), artifact);
        b.project.artifact_refs.push(id.clone());
        let candidate = history.undo_candidate(&b).unwrap();
        assert_eq!(candidate.project.title, "before");
        assert!(candidate.professional.artifacts.contains_key(&id));
        assert!(candidate.project.artifact_refs.contains(&id));
        assert_eq!(candidate.project.revision.0.value(), 2);
        assert!(matches!(
            candidate.professional.artifacts[&id].actual_outcome,
            sf_domain::CheckOutcome::Undetermined
        ));
    }
}
