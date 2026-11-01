//! Paper-packaging geometry and manufacturing. This crate owns neither factory
//! inventory nor human acknowledgment/consumer binding.

mod collision;
mod exact;
mod kernel;
mod kinematics;
mod manufacturing;
mod numeric;
mod search;

pub use kernel::compute_packaging;

use sf_domain::*;

/// Host-selected asset identities; no filesystem access is performed by the kernel.
#[derive(Debug, Clone)]
pub struct PackagingArtifactContext {
    pub artifact_id: ArtifactId,
    pub input_ref: FileRef,
    pub result_ref: FileRef,
    pub manufacturing_directory: RelativeFilePath,
}

#[derive(Debug, Clone)]
pub struct PackagingProgress {
    pub phase: String,
    pub budget_used: BudgetUsage,
}

#[derive(Debug, Clone)]
pub struct PackagingProduct {
    pub selected_job: PackageStructureJob,
    pub result: PackagingResult,
    pub artifact: ArtifactRecord,
    pub manufacturing_bytes: Vec<(FileRef, Vec<u8>)>,
    pub provider: ProviderCapability,
    pub cancelled: bool,
}
