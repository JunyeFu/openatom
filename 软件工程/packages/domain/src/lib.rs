//! Shared value boundaries from the frozen first-release contract.
//! Project, module and scenario models share the frozen wire format.
//! Persistence and Engine behavior are implemented by their owning packages.

mod business_inputs;
mod business_plans;
pub mod cloud_execution;
mod ids;
mod material_inputs;
mod models;
mod professional_inputs;
mod value_types;

pub use business_inputs::{check_business_inputs, check_business_inputs_with_definitions};
pub use business_plans::fork_without_unstarted_demands;
pub use ids::{
    ArtifactId, AssetId, CloudDatasetId, CloudProjectId, DeviceBootId, DispatchCommandId,
    ExecutionEpochId, ExecutionSessionId, ModuleId, ObjectId, ObservationProbeId, PhaseId, PortId,
    ProjectId, RoleId, RunId, ScenarioId, SubjectRevisionId, TaskId, UnitId, VersionId,
};
pub use material_inputs::check_material_inputs;
pub use models::*;
pub use professional_inputs::{
    binding_input_issues, check_professional_inputs, current_consumer_bindings,
};
pub use value_types::{
    DurationMs, FiniteNumber, MAX_SAFE_UINT, RelativeFilePath, Revision, SafeUint, SimTimeMs,
};
