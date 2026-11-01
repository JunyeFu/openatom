//! The single discrete production Engine. Input documents stay immutable; all
//! material, claims and simulation time belong to one Engine per Run.
mod compile;
mod engine;
mod goals;
mod job_history;
mod output_history;
mod parameters;
mod phase_material;
mod quality;
mod stock;

pub use engine::{
    Engine, check_inputs, check_inputs_with_goal, execute, execute_engine, execute_engine_with_goal,
};
pub use job_history::jobs_at_from_history;
pub use engine::external::{ExternalAttemptState, ExternalAttemptView, ExternalError,
    ExternalCandidatePlan, ExternalLogicalWork, ExternalObservation, ExternalReceiptOutcome, ExternalReceiptStatus};
pub use output_history::PortOutputRecord;
use sf_domain::*;

/// Host-owned frozen documents and exact module definitions. This is an internal
/// Rust handoff, not a second public wire format or an editable inventory.
#[derive(Debug, Clone)]
pub struct EngineInput {
    pub run_id: RunId,
    pub input_ref: RunInputRef,
    pub project: ProjectState,
    pub scenario: ScenarioInput,
    pub definitions: Vec<ModuleDefinition>,
    pub scope: Vec<ObjectRef>,
}
