// Selected minimal public delta. Canonical Rust/API files remain owned by window 1.
// Existing MaterialRoleQuantity gains (inputs must keep None):
// #[ts(optional = nullable)]
// pub declared_output_quality: Option<QualityState>,
// Existing TemplateSubset gains:
// #[ts(optional = nullable)]
// pub adoption: Option<TemplateAdoption>,
// Existing ProjectState gains:
// pub template_adoptions: Vec<TemplateAdoption>,
// No new API operation, DomainObject variant, inventory or clock.

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateAdoption {
    pub id: ObjectId,
    pub template_ref: ModuleRef,
    pub source_archive: FileRef,
    pub resource_choices: Vec<TemplateResourceChoice>,
    pub preparation_bindings: Vec<TemplatePreparationBinding>,
    pub imported_claims: Vec<ImportedProfessionalClaim>,
    pub inspection_presets: Vec<TemplateInspectionPreset>,
    #[ts(optional = nullable)]
    pub deferred_seed: Option<DeferredTemplateSeed>,
    pub missing_inputs: Vec<TemplateMissingInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateResourceChoice {
    pub source_id: ObjectId,
    pub requirement_index: SafeUint,
    #[ts(optional = nullable)]
    pub resource_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplatePreparationBinding {
    pub task_ref: ModuleRef,
    pub target_device_id: ObjectId,
    pub before_device_ids: Vec<ObjectId>,
    // Values name source_id in resource_choices, never implicit current Project resources.
    pub worker_roles: BTreeMap<RoleId, ObjectId>,
    pub route_refs: Vec<SubjectRevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ImportedProfessionalClaim {
    pub id: ObjectId,
    pub artifact_type: String,
    pub mode: ProfessionalMode,
    pub consumer_device_ids: Vec<ObjectId>,
    // RFC6901 pointer into TemplateAdoption.source_archive, UI/input preservation only.
    pub source_pointer: String,
    pub declared_scope: Vec<String>,
    pub required_fields: Vec<String>,
    pub missing_fields: Vec<String>,
    // Type is exclusively imported_claim: no actual_outcome/result/ack/accepted flags.
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateInspectionPreset {
    pub id: ObjectId,
    pub consumer_device_id: ObjectId,
    pub inspection_module: ModuleRef,
    #[ts(optional = nullable)]
    pub item_or_ancestor: Option<ObjectId>,
    #[ts(optional = nullable)]
    pub match_kind: Option<QualityMatchKind>,
    pub results: Vec<Option<TypedValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct DeferredTemplateSeed {
    pub time_reference: TimeReference,
    pub window: WindowMs,
    pub initial_stock: Vec<TemplateStockSeed>,
    pub demand: TemplateDemandSeed,
    #[ts(optional = nullable)]
    pub product_ref: Option<SubjectRevisionRef>,
    #[ts(optional = nullable)]
    pub bom_ref: Option<SubjectRevisionRef>,
    #[ts(optional = nullable)]
    pub routing_ref: Option<SubjectRevisionRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateStockSeed {
    pub location_id: ObjectId,
    pub item_type_id: ObjectId,
    pub quantity: Quantity,
    #[ts(optional = nullable)]
    pub quality: Option<QualityState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateDemandSeed {
    pub item_type_id: ObjectId,
    pub quantity: Quantity,
    pub destination_location_id: ObjectId,
    pub acceptance_stage: AcceptanceStage,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct TemplateMissingInput {
    pub kind: TemplateMissingInputKind,
    #[ts(optional = nullable)]
    pub object_ref: Option<ObjectRef>,
    pub source_pointer: String,
    pub required_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum TemplateMissingInputKind {
    SiteGeometry,
    Specification,
    ProfessionalInput,
    ProductBomRoute,
    InitialQuality,
}
