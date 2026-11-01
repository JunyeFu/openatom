//! One editable design snapshot. These are existing wire documents, not a second
//! runtime inventory or a new public API object.
use sf_domain::{ProjectState, ScenarioId, ScenarioInput};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ProjectDraft {
    pub project: ProjectState,
    pub scenarios: BTreeMap<ScenarioId, ScenarioInput>,
    pub professional: crate::ProfessionalDocuments,
}
