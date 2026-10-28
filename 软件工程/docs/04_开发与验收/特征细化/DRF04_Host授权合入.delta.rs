//! H04-01 reference-only delta for review window 1. NOT canonical API or Host code.
//! Insert private records beside HostCallContext in packages/commands/api.design.rs.
//! Use the existing crate::contract::* imports. No Serde/Schema/TS/wire derives.

#[derive(Debug, Clone)]
pub enum HostEditBranch {
    CreateProject, PutObject, RemoveObject, SetProjectSettings,
    CreateDefinitionDraft, PutDefinition, SetDeclaredParameter,
    AdoptModule, InstantiateTemplate, PutPackagingJob,
    RecordArtifactAcknowledgment, AdoptImportedDraft,
    CommitDefinitionVersion, Undo, Redo,
}

#[derive(Debug, Clone)]
pub struct HostFieldGrant {
    // RFC6901 leaf pointer relative to the canonical target, not a script/glob/root.
    pub pointer: String,
    // For arrays, pin the ID of the containing declared element. Host resolves the
    // current numeric index; reordering must not transfer authority to a new ID.
    pub element_id: Option<ObjectId>,
}

#[derive(Debug, Clone)]
pub struct HostObjectEditGrant {
    pub target: ObjectRef,
    pub fields: Vec<HostFieldGrant>,
    pub can_remove: bool,
}

#[derive(Debug, Clone)]
pub struct HostCreateAllowance {
    pub kind: ObjectKind,
    // Cumulative distinct new IDs across this grant, not a fresh allowance/batch.
    pub max_new_objects: SafeUint,
    pub fields: Vec<HostFieldGrant>,
}

#[derive(Debug, Clone)]
pub struct HostEditGrant {
    pub id: ObjectId,
    pub scope: TaskScope,
    pub branches: Vec<HostEditBranch>,
    pub objects: Vec<HostObjectEditGrant>,
    pub creates: Vec<HostCreateAllowance>,
    // Exact selected module/template versions for all adoption/creation branches.
    // Host resolves their use; a caller's origin/source tag is never authority.
    pub source_versions: Vec<ModuleRef>,
    pub reusable_resources: Vec<ObjectId>,
    // Host-owned committed creation ledger. Never serialized into project/MCP.
    // Deletes/Undo do not refund the budget; authorized Redo restores the same ID.
    pub created_objects: Vec<ObjectRef>,
}

// Add exactly this member to existing HostCallContext (not to Request/ApiScope):
// pub edit_grants: Vec<HostEditGrant>,
// Resolve current authority by (real session_id, grant.id) under the same short
// serialized Commands transaction. Never commit against a caller's old clone.
// TaskScope Workspace must match this real session. EditScope compares stable
// project/draft identity; Host updates the draft revision after a permitted commit.
// Existing allowed_operations/selected_file_tokens/origin checks stay in force.

// W00 normalization/atomic authorization algorithm and all 15 branch mappings:
// DRF04_Host授权合入.json. The candidate effects are Host-private, never parameters.
// Normalized candidate + current grant ledger are checked/committed together;
// PermissionDenied commits neither objects/revision/history nor creation budget.
