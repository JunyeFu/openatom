// DRF05-r3 DESIGN FIELD FRAGMENT, not product implementation.
// Merge owner: review window1. Use canonical domain imports/derives.
// Replace named declarations; never append duplicate declarations.
// WaitEndCause / ClaimIntervalEndCause already merged: keep existing definitions.
// MetricKey/MetricValue/RunMetric move from API to domain only once; API imports them.
// See packages/reporting/public-delta.design.json for units, nullability and admission rules.

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum InputChangeKind {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonStatus {
    Strict,
    ControlledChanges,
    SideBySideOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonConditionKey {
    TimeReference,
    ObservationWindow,
    AcceptanceScope,
    Demand,
    SelectedScope,
    InitialConditions,
    SupportedCapabilities,
    ExplainedChanges,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputChangeValue {
    Null,
    Scalar {
        value: TypedValue,
        #[ts(optional = nullable)]
        unit: Option<UnitId>,
    },
    InitialItem(InitialItem),
    Arrival(ArrivalPlan),
    Order(OrderLine),
    Task(ProductionTask),
    Supply(SupplyRelation),
    ModuleRef(ModuleRef),
    ItemType(ItemType),
    Location(StockLocation),
    Hall(HallDesign),
    Line(LineGroup),
    Device(DeviceInstance),
    Resource(ResourceInstance),
    Calendar(Calendar),
    Product(ProductRevision),
    Bom(BomDefinition),
    Route(RouteDefinition),
    Connection(Connection),
    Binding(ConsumerBinding),
    Presentation(Presentation),
    QualityDecision(QualityDecision),
    CostInput(CostInput),
    Window(WindowMs),
    SetupPolicy(DeviceSetupPolicy),
    ParameterOverride(ParameterOverride),
    SourceRecord(SourceRecord),
    Definition(Box<ModuleDefinition>),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum NumericDifference {
    Quantity {
        negative: bool,
        magnitude: Quantity,
    },
    Duration {
        negative: bool,
        magnitude: DurationMs,
    },
    Scalar {
        value: FiniteNumber,
        unit: UnitId,
    },
    MoneyPerUnit {
        amount: FiniteNumber,
        currency: String,
        unit: UnitId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum MetricKey {
    AcceptedQuantity,
    CompletionTime,
    WaitingTime,
    ResourceUtilization,
    UnitCost,
    ReworkQuantity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case", deny_unknown_fields)]
pub enum MetricValue {
    Quantity(Quantity),
    Duration(DurationMs),
    Scalar {
        value: FiniteNumber,
        unit: UnitId,
    },
    Unknown(Issue),
    MoneyPerUnit {
        amount: FiniteNumber,
        currency: String,
        unit: UnitId,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunMetric {
    pub run_id: RunId,
    pub metric: MetricKey,
    pub value: MetricValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct InputChange {
    pub target: ObjectRef,
    pub path: String,
    pub kind: InputChangeKind,
    #[ts(optional = nullable)]
    pub before: Option<InputChangeValue>,
    #[ts(optional = nullable)]
    pub after: Option<InputChangeValue>,
    #[ts(optional = nullable)]
    pub basis: Option<SourceBasis>,
    #[ts(optional = nullable)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ComparisonCondition {
    pub key: ComparisonConditionKey,
    pub matches: bool,
    pub issues: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunPairComparison {
    pub baseline_run_id: RunId,
    pub candidate_run_id: RunId,
    pub status: ComparisonStatus,
    pub conditions: Vec<ComparisonCondition>,
    pub input_changes: Vec<InputChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MetricPairDifference {
    pub baseline_run_id: RunId,
    pub candidate_run_id: RunId,
    pub comparable: bool,
    #[ts(optional = nullable)]
    pub difference: Option<NumericDifference>,
    #[ts(optional = nullable)]
    pub change_percent: Option<FiniteNumber>,
    pub conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct MetricComparison {
    pub metric: MetricKey,
    #[ts(optional = nullable)]
    pub object_ref: Option<ObjectRef>,
    #[ts(optional = nullable)]
    pub acceptance_scope: Option<AcceptanceScope>,
    #[ts(optional = nullable)]
    pub currency: Option<String>,
    pub values: Vec<RunMetric>,
    pub pair_differences: Vec<MetricPairDifference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunComparison {
    pub run_ids: Vec<RunId>,
    pub pairs: Vec<RunPairComparison>,
    pub metrics: Vec<MetricComparison>,
    pub conclusions: Vec<String>,
    pub limitations: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CostInput {
    pub id: ObjectId,
    pub object_ref: ObjectRef,
    pub component: String,
    pub basis: CostBasis,
    #[ts(optional = nullable)]
    pub rate: Option<DecimalAmount>,
    #[ts(optional = nullable)]
    pub currency: Option<String>,
    pub rate_quantity: Quantity,
    pub includes: Vec<String>,
    pub source: ParameterSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ConditionalCost {
    pub cost_input_id: ObjectId,
    pub object_ref: ObjectRef,
    pub basis: CostBasis,
    #[ts(optional = nullable)]
    pub measured_quantity: Option<Quantity>,
    #[ts(optional = nullable)]
    pub amount: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub currency: Option<String>,
    pub missing_conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CurrencyCostSummary {
    pub currency: String,
    pub cost_input_ids: Vec<ObjectId>,
    pub known_subtotal: FiniteNumber,
    #[ts(optional = nullable)]
    pub complete_total: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub accepted_scope: Option<AcceptanceScope>,
    #[ts(optional = nullable)]
    pub accepted_quantity: Option<Quantity>,
    #[ts(optional = nullable)]
    pub unit_cost: Option<FiniteNumber>,
    pub missing_conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct CostEvaluation {
    pub inputs: Vec<CostInput>,
    pub lines: Vec<ConditionalCost>,
    pub currencies: Vec<CurrencyCostSummary>,
    pub unassigned_cost_input_ids: Vec<ObjectId>,
    pub missing_conditions: Vec<Issue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct RunEvaluation {
    pub run_id: RunId,
    pub input_ref: ReportRunInputRef,
    pub evidence_origin: EvidenceOrigin,
    pub scope: Vec<ObjectRef>,
    pub time_reference: TimeReference,
    pub requested_window: WindowMs,
    #[ts(optional = nullable)]
    pub computed_until_ms: Option<SimTimeMs>,
    pub computation_status: ComputationStatus,
    pub whole_window_computed: bool,
    #[ts(optional = nullable)]
    pub goal_met: Option<bool>,
    pub acceptance_scope: Vec<AcceptanceScope>,
    pub order_metrics: Vec<OrderOutcome>,
    pub resource_metrics: Vec<ResourceMetric>,
    pub costs: CostEvaluation,
    pub events: Vec<SimulationEvent>,
    pub acceptance_events: Vec<AcceptanceEvent>,
    pub resource_claims: Vec<ResourceClaim>,
    pub waits: Vec<WaitInterval>,
    pub phase_work_intervals: Vec<PhaseWorkInterval>,
    pub limitations: Vec<Issue>,
    pub module_versions: Vec<ModuleRef>,
    pub input_assumptions: Vec<ParameterSource>,
    pub jobs: Vec<JobState>,
    pub transport: Vec<TransportState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ReportRunInputRef {
    pub project_id: ProjectId,
    pub project_revision: Revision,
    pub scenario_id: ScenarioId,
    pub scenario_revision: Revision,
}

// Replace WaitInterval; other fields retained from current canonical source.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct WaitInterval {
    pub object_ref: ObjectRef,
    #[ts(optional = nullable)]
    pub execution_ref: Option<JobExecutionRef>,
    #[ts(optional = nullable)]
    pub phase_ref: Option<PhaseId>,
    pub interval: IntervalMs,
    pub reasons: Vec<WaitReason>,
    pub end_cause: WaitEndCause,
}

// Replace ResourceClaim; other fields retained from current canonical source.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceClaim {
    pub id: ObjectId,
    pub resource_id: ObjectId,
    pub job_id: ObjectId,
    pub units: SafeUint,
    pub intervals: Vec<IntervalMs>,
    pub kind: ResourceClaimKind,
    #[ts(optional = nullable)]
    pub phase_ref: Option<PhaseId>,
    pub interval_end_causes: Vec<ClaimIntervalEndCause>,
}

// Replace ScenarioInput; other fields retained from current canonical source.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInput {
    pub quality_presets: Vec<InspectionPreset>,
    pub id: ScenarioId,
    pub revision: Revision,
    pub project_revision: Revision,
    pub title: String,
    pub time_reference: TimeReference,
    pub window: WindowMs,
    pub initial_conditions: InitialConditions,
    pub orders: Vec<OrderLine>,
    pub production_tasks: Vec<ProductionTask>,
    pub arrivals: Vec<ArrivalPlan>,
    pub supplies: Vec<SupplyRelation>,
    pub parameter_overrides: Vec<ParameterOverride>,
    pub scope: Vec<ObjectRef>,
    pub issues: Vec<Issue>,
    pub cost_inputs: Vec<CostInput>,
}

// Replace ResourceMetric; other fields retained from current canonical source.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct ResourceMetric {
    pub resource_id: ObjectId,
    pub any_busy_ms: DurationMs,
    pub occupied_capacity_ms: SafeUint,
    #[ts(optional = nullable)]
    pub available_capacity_ms: Option<SafeUint>,
    #[ts(optional = nullable)]
    pub utilization: Option<FiniteNumber>,
    #[ts(optional = nullable)]
    pub effective_work_ms: Option<DurationMs>,
    #[ts(optional = nullable)]
    pub waiting_ms: Option<DurationMs>,
    pub capacity_unit: UnitId,
    #[ts(optional = nullable)]
    pub open_calendar_occupied_capacity_ms: Option<SafeUint>,
    #[ts(optional = nullable)]
    pub closed_calendar_held_capacity_ms: Option<SafeUint>,
    pub missing_conditions: Vec<Issue>,
}

// Replace EvaluationReport; other fields retained from current canonical source.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(deny_unknown_fields)]
pub struct EvaluationReport {
    pub id: ObjectId,
    pub mode: ReportMode,
    pub project_id: ProjectId,
    pub project_revision: Revision,
    pub scenario_refs: Vec<ScenarioRevisionRef>,
    pub run_refs: Vec<RunId>,
    #[ts(optional = nullable)]
    pub time_reference: Option<TimeReference>,
    #[ts(optional = nullable)]
    pub observation_window: Option<WindowMs>,
    pub acceptance_scope: Vec<AcceptanceScope>,
    pub question_and_scope: String,
    pub input_assumptions: Vec<ParameterSource>,
    pub module_versions: Vec<ModuleRef>,
    pub layout_and_schedule: Vec<ReportImage>,
    #[ts(optional = nullable)]
    pub scenario_comparison: Option<RunComparison>,
    pub conclusions: Vec<String>,
    pub missing_conditions: Vec<Issue>,
    pub supported_scope: Vec<String>,
    pub limitations: Vec<Issue>,
    #[ts(optional = nullable)]
    pub editable_project_ref: Option<FileRef>,
    pub asset_rights: Vec<AssetRights>,
    #[ts(optional = nullable)]
    pub generation_context: Option<GenerationContext>,
    pub run_evaluations: Vec<RunEvaluation>,
}
