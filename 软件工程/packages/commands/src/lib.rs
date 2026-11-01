//! Shared typed Commands boundary and the minimal single-project Host.
mod api;
pub mod execution_api;
pub use execution_api::*;
mod execution_session;
pub mod execution_projection;
pub mod execution_cloud;
mod project_draft;
mod project_history;
mod project_host;
mod project_professional;
mod project_store;
mod execution_archive;
pub use execution_archive::{StoredExecutionArchive, StoredExecutionFile};
pub use api::*;
pub use project_draft::ProjectDraft;
pub use project_history::ProjectHistory;
pub use project_host::{
    CloudReadConnection,
    DesktopCloseState, ExecutionCloseBlocker, ExecutionControlGrant, ExecutionGatewayLease,
    ExecutionPreparationJob, ExecutionPreparationLease, ExecutionPreparationRuntime, FrozenExecutionFile,
    FrozenRenderAsset, FrozenRenderFile, FrozenRenderInput, ManufacturingFileDeclaration, ProjectHost,
    ProjectIoCompletion, ProjectIoWork, RenderBackend, RenderComparisonInput, RenderFrame, RenderInputSource, RenderJob,
    RenderPackagingInput, RenderedCapture, RenderedImage, RunPortOutput,
};
pub use project_professional::{
    ProfessionalDocuments, ProfessionalTrust, freeze_packaging_execution_input,
    ingest_professional_result, prepare_acknowledgment, prepare_artifact_binding,
    professional_issues, put_packaging_job,
};
pub use project_store::{ProjectArchive, read_project_archive, write_project_archive};
pub use sf_reporting::EvaluationResources;
