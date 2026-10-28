use serde_json::{Value, json};
use sf_domain::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy)]
pub enum BuiltinTemplate {
    MachiningDelivery,
    PurchasedAssembly,
    ParallelElectromechanical,
    CartonFactory,
}
#[derive(Debug, Clone)]
pub struct TemplateCandidate {
    pub package: ModulePackage,
    pub subset: TemplateSubset,
    pub example_project: ProjectState,
    pub definitions: Vec<ModuleDefinition>,
}
#[derive(Debug, Clone)]
pub enum TemplateResourceSelection {
    CreateDeclaredExample,
    Reuse(ObjectId),
}
#[derive(Debug, Clone)]
pub struct TemplateChoices {
    pub namespace: String,
    pub resources: BTreeMap<ObjectId, TemplateResourceSelection>,
    pub reuse_items: BTreeMap<ObjectId, ObjectId>,
    pub include_example_product: bool,
}
/// Proposed common objects only; no authority, commit, inventory or clock.
#[derive(Debug, Clone)]
pub struct PreparedTemplate {
    pub source_ref: ModuleRef,
    pub source_subset: TemplateSubset,
    pub proposed_project: ProjectState,
    pub id_map: BTreeMap<ObjectId, ObjectId>,
    pub mapped_bindings: RoleBindings,
    pub issues: Vec<Issue>,
    /// None preserves the target list. Some is the complete target + mapped-source
    /// proposal for the one canonical Project setup-policy list.
    pub project_setup_policies: Option<Vec<DeviceSetupPolicy>>,
}
#[derive(Debug, Clone)]
pub struct TemplateSeedChoices {
    pub scenario_id: ScenarioId,
    pub supply: bool,
    pub demand: bool,
    pub initial_quality: BTreeMap<ObjectId, QualityState>,
    pub adopt_declared_inspection_results: bool,
}
/// Actual declared Scenario context accompanying TemplateSubset.seed. A source
/// Project or the one-product deferred summary cannot stand in for these fields.
#[derive(Debug, Clone)]
pub struct TemplateSeedContext {
    pub time_reference: TimeReference,
    pub window: WindowMs,
    pub production_tasks: Vec<ProductionTask>,
    pub supplies: Vec<SupplyRelation>,
}
/// Read actual Scenario context without silently dropping execution conditions
/// that TemplateSeed does not represent. File selection/authority remains Host-owned.
pub fn template_seed_context_from_scenario(
    candidate: &TemplateCandidate,
    source: &ScenarioInput,
) -> Result<TemplateSeedContext, Vec<Issue>> {
    let seed = candidate.subset.seed.as_ref().ok_or_else(|| {
        vec![issue(
            "template.seed_source_missing",
            "/seed",
            "No full source seed",
        )]
    })?;
    if !same(&seed.initial_conditions, &source.initial_conditions)
        || !same(&seed.orders, &source.orders)
        || !same(&seed.arrivals, &source.arrivals)
        || !same(
            &source.project_revision,
            &candidate.example_project.revision,
        )
        // Labels describe the two documents; only exact optional clock-origin
        // and timezone declarations determine source-clock compatibility.
        || source.time_reference.origin_iso8601
            != candidate.example_project.time_reference.origin_iso8601
        || source.time_reference.timezone
            != candidate.example_project.time_reference.timezone
    {
        return Err(vec![issue(
            "template.seed_context_mismatch",
            "/seed_context",
            "Source Scenario seed bodies, Project revision, clock origin and timezone must match the selected typed source; descriptive time labels are preserved independently",
        )]);
    }
    let represented_quality = source.quality_presets.iter().all(|source_preset| {
        candidate.subset.adoption.as_ref().is_some_and(|adoption| {
            adoption.inspection_presets.iter().any(|preset| {
                preset.id == source_preset.id
                    && preset.item_or_ancestor.as_ref() == Some(&source_preset.item_or_ancestor)
                    && preset
                        .match_kind
                        .as_ref()
                        .is_some_and(|kind| same(kind, &source_preset.match_kind))
                    && same(&preset.inspection_module, &source_preset.inspection_module)
                    && same(&preset.results, &source_preset.results)
            })
        })
    });
    if !source.parameter_overrides.is_empty()
        || !source.cost_inputs.is_empty()
        || !represented_quality
        || source.acceptance_goal.is_some()
    {
        return Err(vec![issue(
            "template.seed_context_unrepresented",
            "/seed_context",
            "This Scenario has parameter overrides, costs, an acceptance goal or quality presets not exactly represented in the template adoption. Preserve/configure these explicitly; do not drop them when extracting a seed.",
        )]);
    }
    let source_scope: BTreeSet<_> = source.scope.iter().map(|o| o.id.clone()).collect();
    let project_scope: BTreeSet<_> = candidate
        .example_project
        .devices
        .iter()
        .map(|d| d.id.clone())
        .collect();
    if !source.scope.is_empty()
        && (source
            .scope
            .iter()
            .any(|o| !matches!(o.kind, ObjectKind::Device))
            || source_scope != project_scope
            || !source.supplies.is_empty())
    {
        return Err(vec![issue(
            "template.seed_context_scope",
            "/seed_context/scope",
            "A partial Scenario scope cannot be widened. Supply execution requires the actual source's empty whole-Project scope; Supply is not an executable scope object",
        )]);
    }
    let blocking: Vec<_> = source
        .issues
        .iter()
        .filter(|i| {
            i.blocks
                .iter()
                .any(|p| matches!(p, CapabilityPurpose::Execution))
        })
        .cloned()
        .collect();
    if !blocking.is_empty() {
        return Err(blocking);
    }
    Ok(TemplateSeedContext {
        time_reference: source.time_reference.clone(),
        window: source.window.clone(),
        production_tasks: source.production_tasks.clone(),
        supplies: source.supplies.clone(),
    })
}

pub fn builtin_template_support() -> Result<Vec<ModulePackage>, Vec<Issue>> {
    serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/catalog.json"
    ))
    .map_err(|e| vec![issue("template.support_document", "/", e.to_string())])
}
pub fn carton_generation_profile() -> Result<TemplateGenerationProfile, Vec<Issue>> {
    let p: Versioned<TemplateGenerationProfile> = serde_json::from_str(include_str!(
        "../../../modules/templates/carton-factory/candidate/generation-profile.json"
    ))
    .map_err(|e| vec![issue("template.profile_document", "/", e.to_string())])?;
    Ok(p.data)
}
/// Fill only the supplied site dimensions; no target-to-throughput assignment,
/// equipment sizing, automatic hiring, or geometry Pass is manufactured here.
pub fn fill_template_site(
    plan: &mut PreparedTemplate,
    site: &SiteEnvelope,
) -> Result<(), Vec<Issue>> {
    let positive = |x: &Option<FiniteNumber>| x.as_ref().filter(|v| v.value() > 0.0).cloned();
    let length = positive(&site.length_m).ok_or_else(|| {
        vec![issue(
            "template.site_length",
            "/site_envelope/length_m",
            "Positive explicit site length required",
        )]
    })?;
    let width = positive(&site.width_m).ok_or_else(|| {
        vec![issue(
            "template.site_width",
            "/site_envelope/width_m",
            "Positive explicit site width required",
        )]
    })?;
    if site.height_m.as_ref().is_some_and(|h| h.value() <= 0.0) {
        return Err(vec![issue(
            "template.site_height",
            "/site_envelope/height_m",
            "Supplied height must be positive",
        )]);
    }
    if plan.proposed_project.halls.len() != 1 {
        return Err(vec![issue(
            "template.site_scope",
            "/halls",
            "Choose one source hall explicitly before filling this profile",
        )]);
    }
    let h = &mut plan.proposed_project.halls[0];
    h.width_m = length;
    h.depth_m = width;
    h.height_m = site.height_m.clone();
    let hid = h.id.clone();
    let fields = [
        ("/width_m", site.length_m.as_ref()),
        ("/depth_m", site.width_m.as_ref()),
        ("/height_m", site.height_m.as_ref()),
    ];
    for (path, value) in fields {
        plan.proposed_project
            .parameter_sources
            .retain(|p| p.target.id != hid || p.field_path != path);
        plan.proposed_project
            .parameter_sources
            .push(ParameterSource {
                target: ObjectRef {
                    kind: ObjectKind::Hall,
                    id: hid.clone(),
                },
                field_path: path.into(),
                value: value.map(|v| TypedValue::Number(v.clone())),
                basis: if value.is_some() {
                    SourceBasis::UserProvided
                } else {
                    SourceBasis::Unknown
                },
                source_ref: None,
            });
    }
    Ok(())
}

fn issue(code: &str, path: &str, message: impl Into<String>) -> Issue {
    Issue {
        code: code.into(),
        field_path: Some(path.into()),
        object_ref: None,
        message: message.into(),
        blocks: vec![CapabilityPurpose::Execution],
    }
}
fn id(s: impl Into<String>) -> ObjectId {
    ObjectId::try_from(s.into()).expect("validated object ID")
}
fn same(a: &impl std::fmt::Debug, b: &impl std::fmt::Debug) -> bool {
    format!("{a:?}") == format!("{b:?}")
}
fn mapped(x: &ObjectId, ids: &BTreeMap<ObjectId, ObjectId>) -> ObjectId {
    ids.get(x).cloned().unwrap_or_else(|| x.clone())
}
fn remap(x: &mut ObjectId, ids: &BTreeMap<ObjectId, ObjectId>) {
    *x = mapped(x, ids);
}
fn object(x: &mut ObjectRef, ids: &BTreeMap<ObjectId, ObjectId>) {
    remap(&mut x.id, ids);
}
fn subject(x: &mut SubjectRevisionRef, ids: &BTreeMap<ObjectId, ObjectId>) {
    remap(&mut x.id, ids);
}
fn item_layout(layout: &mut Option<ItemSlotLayout>, ids: &BTreeMap<ObjectId, ObjectId>) {
    if let Some(layout) = layout {
        for slot in &mut layout.slots {
            subject(&mut slot.item_type_ref, ids);
        }
    }
}
fn typed_reference(value: &mut TypedValue, ids: &BTreeMap<ObjectId, ObjectId>) {
    if let TypedValue::ItemType(id) = value {
        remap(id, ids);
    }
}
fn port(x: &mut PortRef, ids: &BTreeMap<ObjectId, ObjectId>) {
    remap(&mut x.instance_id, ids);
}
fn bindings(
    b: &mut RoleBindings,
    ids: &BTreeMap<ObjectId, ObjectId>,
    unresolved: &BTreeSet<ObjectId>,
) {
    for x in b.item_types.values_mut() {
        remap(x, ids);
    }
    b.resources.retain(|_, x| !unresolved.contains(x));
    for x in b.resources.values_mut() {
        remap(x, ids);
    }
    for x in b.targets.values_mut() {
        object(x, ids);
    }
    for x in b.ports.values_mut() {
        port(x, ids);
    }
}
pub fn builtin_template(which: BuiltinTemplate) -> Result<TemplateCandidate, Vec<Issue>> {
    macro_rules! docs {
        ($n:literal) => {
            (
                include_str!(concat!(
                    "../../../modules/templates/",
                    $n,
                    "/candidate/module.json"
                )),
                include_str!(concat!(
                    "../../../modules/templates/",
                    $n,
                    "/candidate/template.json"
                )),
                include_str!(concat!(
                    "../../../modules/templates/",
                    $n,
                    "/candidate/example-project.json"
                )),
                include_str!(concat!(
                    "../../../modules/templates/",
                    $n,
                    "/candidate/definitions.json"
                )),
            )
        };
    }
    let (m, t, p, d) = match which {
        BuiltinTemplate::MachiningDelivery => docs!("machining-delivery"),
        BuiltinTemplate::PurchasedAssembly => docs!("purchased-assembly"),
        BuiltinTemplate::ParallelElectromechanical => docs!("parallel-electromechanical"),
        BuiltinTemplate::CartonFactory => docs!("carton-factory"),
    };
    decode_template_candidate(m.as_bytes(), t.as_bytes(), p.as_bytes(), d.as_bytes())
}
/// Bytes must come from the actual selected archive. Selection grants and version
/// registration remain Host responsibilities, not caller-body assertions here.
pub fn decode_template_candidate(
    module: &[u8],
    subset: &[u8],
    project: &[u8],
    definitions: &[u8],
) -> Result<TemplateCandidate, Vec<Issue>> {
    let err = |e: serde_json::Error| vec![issue("template.document", "/", e.to_string())];
    let m: Versioned<ModulePackage> = serde_json::from_slice(module).map_err(err)?;
    let t: Versioned<TemplateSubset> = serde_json::from_slice(subset).map_err(err)?;
    let p: Versioned<ProjectState> = serde_json::from_slice(project).map_err(err)?;
    let defs: Vec<ModuleDefinition> = serde_json::from_slice(definitions).map_err(err)?;
    if !matches!(m.data.definition.payload, ModulePayload::Templates(_)) {
        return Err(vec![issue(
            "template.category",
            "/definition/payload",
            "Selected source is not Templates",
        )]);
    }
    let a = t.data.adoption.as_ref().ok_or_else(|| {
        vec![issue(
            "template.adoption",
            "/adoption",
            "Typed source/roles/deferred conditions required",
        )]
    })?;
    for choice in &a.resource_choices {
        if choice.requirement_index.value() as usize >= t.data.resource_requirements.len()
            || !p.data.resources.iter().any(|r| r.id == choice.source_id)
        {
            return Err(vec![issue(
                "template.resource_source",
                "/adoption/resource_choices",
                "Resource source/index must resolve in the selected documents",
            )]);
        }
    }
    if a.template_ref.id != m.data.definition.id
        || !same(&a.template_ref.version, &m.data.definition.version)
    {
        return Err(vec![issue(
            "template.source_version",
            "/adoption/template_ref",
            "Exact source ID/version differs from selected package",
        )]);
    }
    let tv = serde_json::to_value(&t.data).unwrap();
    let pv = serde_json::to_value(&p.data).unwrap();
    for key in [
        "devices",
        "connections",
        "halls",
        "locations",
        "lines",
        "item_types",
    ] {
        if tv[key] != pv[key] {
            return Err(vec![issue(
                "template.blueprint_mismatch",
                "/project",
                format!("Source subset and example {key} differ"),
            )]);
        }
    }
    let mut keys = BTreeSet::new();
    for d in &defs {
        if !keys.insert((d.id.clone(), String::from(d.version.clone()))) {
            return Err(vec![issue(
                "template.module_conflict",
                "/definitions",
                "Duplicate exact module identity",
            )]);
        }
    }
    for r in &t.data.module_refs {
        if !keys.contains(&(r.id.clone(), String::from(r.version.clone()))) {
            return Err(vec![issue(
                "template.module_missing",
                "/module_refs",
                format!("Missing exact definition {}", r.id.as_str()),
            )]);
        }
    }
    Ok(TemplateCandidate {
        package: m.data,
        subset: t.data,
        example_project: p.data,
        definitions: defs,
    })
}
pub fn empty_template_project(
    project_id: ProjectId,
    title: String,
    time_reference: TimeReference,
) -> ProjectState {
    let camera = CameraState {
        coordinate_frame: CoordinateFrame::RightHandedZUp,
        unit: LengthUnit::Meter,
        eye: Vec3 {
            x: 10.0.try_into().unwrap(),
            y: (-10.0).try_into().unwrap(),
            z: 10.0.try_into().unwrap(),
        },
        target: Vec3 {
            x: 0.0.try_into().unwrap(),
            y: 0.0.try_into().unwrap(),
            z: 0.0.try_into().unwrap(),
        },
        zoom: 1.0.try_into().unwrap(),
    };
    ProjectState {
        id: project_id,
        revision: Revision(SafeUint::try_from(1u64).unwrap()),
        title,
        time_reference,
        item_types: vec![],
        locations: vec![],
        halls: vec![],
        lines: vec![],
        devices: vec![],
        resources: vec![],
        calendars: vec![],
        products: vec![],
        boms: vec![],
        routes: vec![],
        connections: vec![],
        module_refs: vec![],
        scenario_refs: vec![],
        artifact_refs: vec![],
        acknowledgments: vec![],
        bindings: vec![],
        views: ViewState {
            normal_canvas: CanvasKind::LineCanvas,
            selected_hall: None,
            selected_objects: vec![],
            lod: LodLevel::Detailed,
            line_camera: camera.clone(),
            multi_hall_camera: camera,
        },
        generation_context: None,
        parameter_sources: vec![],
        sources: vec![],
        issues: vec![],
        template_adoptions: vec![],
        setup_policies: vec![],
    }
}
fn project_ids(p: &ProjectState) -> BTreeSet<ObjectId> {
    let mut out = BTreeSet::new();
    macro_rules! add{($($f:ident),*)=>{$(out.extend(p.$f.iter().map(|x|x.id.clone()));)*};}
    add!(
        item_types,
        locations,
        halls,
        lines,
        devices,
        resources,
        calendars,
        products,
        boms,
        routes,
        sources,
        template_adoptions
    );
    for r in &p.routes {
        out.extend(r.steps.iter().map(|s| s.id.clone()));
    }
    for c in &p.connections {
        out.insert(match c {
            Connection::Material(c) => c.id.clone(),
            Connection::Signal(c) => c.id.clone(),
            Connection::Artifact(c) => c.id.clone(),
        });
    }
    for a in &p.template_adoptions {
        out.extend(a.imported_claims.iter().map(|x| x.id.clone()));
        out.extend(a.inspection_presets.iter().map(|x| x.id.clone()));
    }
    for policy in &p.setup_policies {
        if let Some(article) = &policy.first_article {
            out.insert(article.gate_id.clone());
        }
    }
    out
}
fn enough(a: &Quantity, r: &Quantity) -> bool {
    a.unit == r.unit
        && u128::from(a.value.coefficient.value())
            * 10u128.pow(u32::from(r.value.decimal_places.value()))
            >= u128::from(r.value.coefficient.value())
                * 10u128.pow(u32::from(a.value.decimal_places.value()))
}

pub fn prepare_template(
    candidate: &TemplateCandidate,
    target: &ProjectState,
    choices: &TemplateChoices,
) -> Result<PreparedTemplate, Vec<Issue>> {
    ObjectId::try_from(choices.namespace.clone())
        .map_err(|m| vec![issue("template.namespace", "/namespace", m)])?;
    let source = &candidate.example_project;
    for layout in source
        .devices
        .iter()
        .filter_map(|v| v.item_layout.as_ref())
        .chain(
            source
                .locations
                .iter()
                .filter_map(|v| v.item_layout.as_ref()),
        )
        .chain(
            source
                .item_types
                .iter()
                .filter_map(|v| v.contents_layout.as_ref()),
        )
    {
        for slot in &layout.slots {
            if !source
                .item_types
                .iter()
                .any(|v| v.id == slot.item_type_ref.id && v.revision == slot.item_type_ref.revision)
            {
                return Err(vec![issue(
                    "template.item_layout_reference",
                    "/item_layout/slots/item_type_ref",
                    "Display slots require an exact selected source ItemType revision",
                )]);
            }
        }
    }
    let mut ids = BTreeMap::new();
    let existing = project_ids(target);
    for old in project_ids(source) {
        let fresh = ObjectId::try_from(format!("{}:{}", choices.namespace, old.as_str()))
            .map_err(|m| vec![issue("template.id", "/namespace", m)])?;
        if existing.contains(&fresh) {
            return Err(vec![issue(
                "template.id_collision",
                "/namespace",
                format!("Already exists: {}", fresh.as_str()),
            )]);
        }
        ids.insert(old, fresh);
    }
    for (old, new) in &choices.reuse_items {
        let Some(a) = source.item_types.iter().find(|i| i.id == *old) else {
            return Err(vec![issue(
                "template.item_source",
                "/reuse_items",
                "Unknown source item",
            )]);
        };
        let Some(b) = target.item_types.iter().find(|i| i.id == *new) else {
            return Err(vec![issue(
                "template.item_target",
                "/reuse_items",
                "Selected actual item absent",
            )]);
        };
        if a.unit != b.unit
            || !same(&a.tracking, &b.tracking)
            || !same(&a.revision, &b.revision)
            || a.spec_ref.is_none()
            || !same(&a.spec_ref, &b.spec_ref)
        {
            return Err(vec![issue(
                "template.item_mismatch",
                "/reuse_items",
                "Reuse needs matching unit/tracking/revision and known exact specification",
            )]);
        }
        ids.insert(old.clone(), new.clone());
    }
    let mut unresolved = BTreeSet::new();
    let mut issues = vec![];
    for r in &source.resources {
        match choices.resources.get(&r.id) {
            Some(TemplateResourceSelection::CreateDeclaredExample) => {}
            Some(TemplateResourceSelection::Reuse(target_id)) => {
                let Some(a) = target.resources.iter().find(|x| x.id == *target_id) else {
                    return Err(vec![issue(
                        "template.resource_missing",
                        "/resources",
                        "Actual selected resource absent",
                    )]);
                };
                if !same(&a.kind, &r.kind)
                    || a.role != r.role
                    || !enough(&a.capacity, &r.capacity)
                    || !r
                        .qualifications
                        .iter()
                        .all(|q| a.qualifications.contains(q))
                    || !target.calendars.iter().any(|c| c.id == a.calendar_id)
                {
                    return Err(vec![issue(
                        "template.resource_mismatch",
                        "/resources",
                        "Actual kind/role/unit/capacity/qualifications/calendar mismatch",
                    )]);
                }
                ids.insert(r.id.clone(), a.id.clone());
            }
            None => {
                unresolved.insert(r.id.clone());
                issues.push(issue(
                    "template.resource_choice",
                    "/resources",
                    format!("Choose/create {} explicitly", r.id.as_str()),
                ));
            }
        }
    }
    if choices
        .resources
        .keys()
        .any(|id| !source.resources.iter().any(|r| r.id == *id))
    {
        return Err(vec![issue(
            "template.unknown_resource",
            "/resources",
            "Unknown source role",
        )]);
    }
    let mut p = source.clone();
    p.id = target.id.clone();
    p.revision = target.revision.clone();
    p.scenario_refs.clear();
    p.artifact_refs.clear();
    p.acknowledgments.clear();
    p.bindings.clear();
    p.generation_context = None;
    p.item_types
        .retain(|x| !choices.reuse_items.contains_key(&x.id));
    for x in &mut p.item_types {
        item_layout(&mut x.contents_layout, &ids);
        remap(&mut x.id, &ids);
        for value in x.attributes.values_mut() {
            typed_reference(value, &ids);
        }
        if let Some(s) = &mut x.spec_ref {
            subject(s, &ids);
        }
    }
    for x in &mut p.locations {
        item_layout(&mut x.item_layout, &ids);
        remap(&mut x.id, &ids);
        if let Some(h) = &mut x.hall_id {
            remap(h, &ids);
        }
        for i in &mut x.accepted_item_types {
            remap(i, &ids);
        }
    }
    for x in &mut p.halls {
        remap(&mut x.id, &ids);
        for i in x.input_locations.iter_mut().chain(&mut x.output_locations) {
            remap(i, &ids);
        }
    }
    for x in &mut p.lines {
        remap(&mut x.id, &ids);
        remap(&mut x.hall_id, &ids);
        for i in &mut x.members {
            remap(i, &ids);
        }
        for i in &mut x.boundary_ports {
            port(i, &ids);
        }
    }
    for x in &mut p.devices {
        item_layout(&mut x.item_layout, &ids);
        remap(&mut x.id, &ids);
        remap(&mut x.hall_id, &ids);
        if let Some(l) = &mut x.line_id {
            remap(l, &ids);
        }
        bindings(&mut x.role_bindings, &ids, &unresolved);
        for value in x.configuration.values_mut() {
            typed_reference(value, &ids);
        }
    }
    let created: BTreeSet<_> = source
        .resources
        .iter()
        .filter(|r| {
            matches!(
                choices.resources.get(&r.id),
                Some(TemplateResourceSelection::CreateDeclaredExample)
            )
        })
        .map(|r| r.id.clone())
        .collect();
    p.resources.retain(|r| created.contains(&r.id));
    let calendars: BTreeSet<_> = p.resources.iter().map(|r| r.calendar_id.clone()).collect();
    p.calendars.retain(|c| calendars.contains(&c.id));
    for x in &mut p.resources {
        remap(&mut x.id, &ids);
        remap(&mut x.calendar_id, &ids);
        if let Some(h) = &mut x.home_location {
            remap(h, &ids);
        }
    }
    for x in &mut p.calendars {
        remap(&mut x.id, &ids);
    }
    if !choices.include_example_product {
        p.products.clear();
        p.boms.clear();
        p.routes.clear();
        p.parameter_sources.retain(|s| {
            !matches!(
                s.target.kind,
                ObjectKind::Product | ObjectKind::Bom | ObjectKind::Route
            )
        });
        issues.push(issue(
            "template.product_choice",
            "/include_example_product",
            "Choose the labeled example Product/BOM/Route or author real input",
        ));
    }
    for x in &mut p.products {
        remap(&mut x.id, &ids);
        subject(&mut x.item_type_ref, &ids);
        subject(&mut x.bom_ref, &ids);
        subject(&mut x.routing_ref, &ids);
        x.artifact_refs.clear();
    }
    for x in &mut p.boms {
        remap(&mut x.id, &ids);
        for row in &mut x.rows {
            subject(&mut row.item_type_ref, &ids);
            remap(&mut row.consume_at_operation, &ids);
        }
    }
    for x in &mut p.routes {
        remap(&mut x.id, &ids);
        for row in &mut x.steps {
            remap(&mut row.id, &ids);
            remap(&mut row.device_id, &ids);
            for v in &mut row.predecessors {
                remap(v, &ids);
            }
            for v in row.item_bindings.values_mut() {
                remap(v, &ids);
            }
            row.resource_bindings.retain(|_, v| !unresolved.contains(v));
            for v in row.resource_bindings.values_mut() {
                remap(v, &ids);
            }
        }
    }
    for c in &mut p.connections {
        match c {
            Connection::Material(c) => {
                remap(&mut c.id, &ids);
                remap(&mut c.item_type, &ids);
                remap(&mut c.destination_store, &ids);
                for e in [&mut c.source, &mut c.destination] {
                    match e {
                        MaterialEndpoint::Location(x) => remap(x, &ids),
                        MaterialEndpoint::Port(x) => port(x, &ids),
                    }
                }
            }
            Connection::Signal(c) => {
                remap(&mut c.id, &ids);
                port(&mut c.source, &ids);
                port(&mut c.destination, &ids);
                if let Some(x) = &mut c.effect_target {
                    object(x, &ids);
                }
            }
            Connection::Artifact(c) => {
                remap(&mut c.id, &ids);
                port(&mut c.source, &ids);
                port(&mut c.destination, &ids);
                remap(&mut c.binding_ref, &ids);
            }
        }
    }
    for x in &mut p.sources {
        remap(&mut x.id, &ids);
    }
    for x in &mut p.parameter_sources {
        object(&mut x.target, &ids);
        if let Some(value) = &mut x.value {
            typed_reference(value, &ids);
        }
        if let Some(s) = &mut x.source_ref {
            remap(s, &ids);
        }
    }
    let mut mapped_bindings = candidate.subset.suggested_role_bindings.clone();
    bindings(&mut mapped_bindings, &ids, &unresolved);
    for a in &mut p.template_adoptions {
        remap(&mut a.id, &ids);
        for x in &mut a.resource_choices {
            x.resource_id = if unresolved.contains(&x.source_id) {
                None
            } else {
                Some(mapped(&x.source_id, &ids))
            };
            let req = &candidate.subset.resource_requirements[x.requirement_index.value() as usize];
            if let (Some(role), Some(r)) = (&req.role, &x.resource_id) {
                mapped_bindings
                    .resources
                    .entry(role.clone())
                    .or_insert_with(|| r.clone());
                mapped_bindings.resources.insert(
                    RoleId::try_from(format!("template.resource.{}", x.source_id.as_str()))
                        .unwrap(),
                    r.clone(),
                );
            }
        }
        for x in &mut a.preparation_bindings {
            remap(&mut x.target_device_id, &ids);
            for i in &mut x.before_device_ids {
                remap(i, &ids);
            }
            for i in &mut x.route_refs {
                subject(i, &ids);
            }
            if !choices.include_example_product {
                x.route_refs.clear();
            }
        }
        for x in &mut a.imported_claims {
            remap(&mut x.id, &ids);
            for i in &mut x.consumer_device_ids {
                remap(i, &ids);
            }
        }
        for x in &mut a.inspection_presets {
            remap(&mut x.id, &ids);
            remap(&mut x.consumer_device_id, &ids);
            if let Some(i) = &mut x.item_or_ancestor {
                remap(i, &ids);
            }
        }
        if let Some(seed) = &mut a.deferred_seed {
            for row in &mut seed.initial_stock {
                remap(&mut row.location_id, &ids);
                remap(&mut row.item_type_id, &ids);
            }
            remap(&mut seed.demand.item_type_id, &ids);
            remap(&mut seed.demand.destination_location_id, &ids);
            for r in [
                &mut seed.product_ref,
                &mut seed.bom_ref,
                &mut seed.routing_ref,
            ]
            .into_iter()
            .flatten()
            {
                subject(r, &ids);
            }
        }
        for x in &mut a.missing_inputs {
            if let Some(o) = &mut x.object_ref {
                object(o, &ids);
            }
        }
    }
    if let Some(h) = &mut p.views.selected_hall {
        remap(h, &ids);
    }
    p.views.selected_objects.clear();
    let project_setup_policies = prepare_setup_policies(candidate, target, &mut p, &ids)?;
    Ok(PreparedTemplate {
        source_ref: ModuleRef {
            id: candidate.package.definition.id.clone(),
            version: candidate.package.definition.version.clone(),
        },
        source_subset: candidate.subset.clone(),
        proposed_project: p,
        id_map: ids,
        mapped_bindings,
        issues,
        project_setup_policies,
    })
}
impl PreparedTemplate {
    /// Explicit current Commands preparation shape. Host replays these choices
    /// against its selected archive; no caller-owned metadata/approval is sent.
    /// `source_project` must name the actual listed typed example Project.
    pub fn command_data_with_preparation(
        &self,
        source_project: &RelativeFilePath,
        choices: &TemplateChoices,
        site_envelope: Option<&SiteEnvelope>,
        explicit_seed: Option<ScenarioInput>,
    ) -> Value {
        let resources: BTreeMap<_, Option<ObjectId>> = choices
            .resources
            .iter()
            .map(|(source, choice)| {
                (
                    source.clone(),
                    match choice {
                        TemplateResourceSelection::CreateDeclaredExample => None,
                        TemplateResourceSelection::Reuse(actual) => Some(actual.clone()),
                    },
                )
            })
            .collect();
        let mut data = self.command_data(explicit_seed);
        data["preparation"] = json!({
            "source_project":source_project,
            "namespace":choices.namespace,
            "resources":resources,
            "reuse_items":choices.reuse_items,
            "include_example_product":choices.include_example_product,
            "site_envelope":site_envelope,
        });
        data
    }
    /// Existing Commands::TemplateInstantiation data shape, decoded by its owner.
    /// The original subset stays byte-equivalent to selected source, not remapped.
    pub fn command_data(&self, explicit_seed: Option<ScenarioInput>) -> Value {
        let mut objects = vec![];
        macro_rules! add{($f:ident,$k:literal)=>{objects.extend(self.proposed_project.$f.iter().map(|v|json!({"kind":$k,"data":v})));};}
        add!(item_types, "item_type");
        add!(locations, "location");
        add!(halls, "hall");
        add!(lines, "line");
        add!(devices, "device");
        add!(resources, "resource");
        add!(calendars, "calendar");
        add!(products, "product");
        add!(boms, "bom");
        add!(routes, "route");
        add!(connections, "connection");
        let mut data = json!({"template_ref":self.source_ref,"subset":self.source_subset,"mapped_bindings":self.mapped_bindings,"proposed_objects":objects,"explicit_new_scenario_seed":explicit_seed});
        if let Some(policies) = &self.project_setup_policies {
            data["project_setup_policies"] = json!(policies);
        }
        data
    }
}

fn prepare_setup_policies(
    candidate: &TemplateCandidate,
    target: &ProjectState,
    proposed: &mut ProjectState,
    ids: &BTreeMap<ObjectId, ObjectId>,
) -> Result<Option<Vec<DeviceSetupPolicy>>, Vec<Issue>> {
    if candidate.example_project.setup_policies.is_empty() {
        return Ok(None);
    }
    let mut mapped_policies = vec![];
    let mut devices = BTreeSet::new();
    let map = |old: &ObjectId, path: &str| {
        ids.get(old).cloned().ok_or_else(|| {
            vec![issue(
                "template.setup_policy_adoption",
                path,
                format!(
                    "Policy reference {} has no explicit selected mapping",
                    old.as_str()
                ),
            )]
        })
    };
    for policy in &candidate.example_project.setup_policies {
        if !devices.insert(policy.device_id.clone())
            || !candidate
                .example_project
                .devices
                .iter()
                .any(|d| d.id == policy.device_id)
        {
            return Err(vec![issue(
                "template.setup_policy_adoption",
                "/project/setup_policies/device_id",
                "Each source policy must name one unique selected source device",
            )]);
        }
        let mut value = policy.clone();
        value.device_id = map(&policy.device_id, "/project/setup_policies/device_id")?;
        let device = proposed
            .devices
            .iter()
            .find(|d| d.id == value.device_id)
            .ok_or_else(|| {
                vec![issue(
                    "template.setup_policy_adoption",
                    "/project/setup_policies/device_id",
                    "Policy device was not adopted",
                )]
            })?;
        if target
            .setup_policies
            .iter()
            .any(|p| p.device_id == value.device_id)
        {
            return Err(vec![issue(
                "template.setup_policy_conflict",
                "/project/setup_policies/device_id",
                "A source policy cannot replace an existing target policy",
            )]);
        }
        let route_steps: Vec<_> = proposed
            .routes
            .iter()
            .flat_map(|r| &r.steps)
            .filter(|s| s.device_id == value.device_id)
            .collect();
        for rule in &mut value.changeovers {
            if let Some(source) = &mut rule.duration.source_ref {
                if !candidate
                    .example_project
                    .sources
                    .iter()
                    .any(|s| s.id == *source)
                {
                    return Err(vec![issue(
                        "template.setup_policy_source",
                        "/project/setup_policies/changeovers/duration/source_ref",
                        "Duration source must be a declared source record in this source Project",
                    )]);
                }
                *source = map(
                    source,
                    "/project/setup_policies/changeovers/duration/source_ref",
                )?;
            }
            for requirement in &rule.resources {
                // Mirror the established route override -> device binding resolution;
                // no caller-supplied role or extra worker is manufactured here.
                let bindings: Vec<_> = if route_steps.is_empty() {
                    vec![device.role_bindings.resources.get(&requirement.role)]
                } else {
                    route_steps
                        .iter()
                        .map(|step| {
                            step.resource_bindings
                                .get(&requirement.role)
                                .or_else(|| device.role_bindings.resources.get(&requirement.role))
                        })
                        .collect()
                };
                for selected in bindings {
                    let selected = selected.ok_or_else(|| {
                        vec![issue(
                            "template.setup_policy_resource",
                            "/project/setup_policies/changeovers/resources",
                            format!(
                                "Role {} has no explicitly selected binding",
                                requirement.role.as_str()
                            ),
                        )]
                    })?;
                    let resource=proposed.resources.iter().chain(&target.resources).find(|r|r.id==*selected).ok_or_else(||vec![issue("template.setup_policy_resource","/project/setup_policies/changeovers/resources","Selected resource is absent; omitted resources are not implicitly created")])?;
                    if matches!(resource.kind, ResourceKind::ServiceDeclaration)
                        || !enough(&resource.capacity, &requirement.quantity)
                        || requirement
                            .qualifications
                            .iter()
                            .any(|q| !resource.qualifications.contains(q))
                        || !proposed
                            .calendars
                            .iter()
                            .chain(&target.calendars)
                            .any(|c| c.id == resource.calendar_id)
                    {
                        return Err(vec![issue(
                            "template.setup_policy_resource",
                            "/project/setup_policies/changeovers/resources",
                            "Selected actual resource must meet unit/capacity/qualification/calendar requirements; deferred services cannot execute setup",
                        )]);
                    }
                }
            }
        }
        if let Some(article) = &mut value.first_article {
            let source_route = candidate
                .example_project
                .routes
                .iter()
                .find(|r| {
                    r.id == article.route_ref.id && same(&r.revision, &article.route_ref.revision)
                })
                .ok_or_else(|| {
                    vec![issue(
                        "template.setup_policy_first_article_route",
                        "/project/setup_policies/first_article/route_ref",
                        "First article requires its exact declared source route",
                    )]
                })?;
            if !source_route
                .steps
                .iter()
                .any(|s| s.device_id == policy.device_id)
            {
                return Err(vec![issue(
                    "template.setup_policy_first_article_route",
                    "/project/setup_policies/first_article/route_ref",
                    "First-article route does not use the policy device",
                )]);
            }
            article.route_ref.id = map(
                &article.route_ref.id,
                "/project/setup_policies/first_article/route_ref",
            )?;
            if !proposed.routes.iter().any(|r| {
                r.id == article.route_ref.id && same(&r.revision, &article.route_ref.revision)
            }) {
                return Err(vec![issue(
                    "template.setup_policy_first_article_route",
                    "/project/setup_policies/first_article/route_ref",
                    "Select the route/product explicitly before adopting its first-article requirement",
                )]);
            }
            if !proposed.module_refs.iter().any(|r|same(r,&article.inspection_module)) || !candidate.definitions.iter().any(|definition|definition.id==article.inspection_module.id && same(&definition.version,&article.inspection_module.version) && matches!(&definition.payload,ModulePayload::Operations(operation) if matches!(operation.builtin,RuleKind::InspectAndRoute))) {
                return Err(vec![issue("template.setup_policy_first_article_inspection","/project/setup_policies/first_article/inspection_module","First article requires its exact declared InspectAndRoute operation, not a different version or a device illustration")]);
            }
            article.gate_id = map(
                &article.gate_id,
                "/project/setup_policies/first_article/gate_id",
            )?;
        }
        mapped_policies.push(value);
    }
    proposed.setup_policies = mapped_policies.clone();
    let mut merged = target.setup_policies.clone();
    merged.extend(mapped_policies);
    Ok(Some(merged))
}
/// Explicit supply and demand enter a NEW scenario only. No source RuntimeItem,
/// Claim, Task, frozen Run, acknowledgment or current inventory is an input.
pub fn prepare_template_seed(
    candidate: &TemplateCandidate,
    plan: &PreparedTemplate,
    choices: &TemplateSeedChoices,
) -> Result<ScenarioInput, Vec<Issue>> {
    prepare_template_seed_with_context(candidate, plan, choices, None)
}

/// Legacy sources retain their established behavior. Generic compositions require
/// an actual declared time window and tasks; no first-product fallback is used.
pub fn prepare_template_seed_with_context(
    candidate: &TemplateCandidate,
    plan: &PreparedTemplate,
    choices: &TemplateSeedChoices,
    context: Option<&TemplateSeedContext>,
) -> Result<ScenarioInput, Vec<Issue>> {
    if !same(
        &plan.source_ref,
        &ModuleRef {
            id: candidate.package.definition.id.clone(),
            version: candidate.package.definition.version.clone(),
        },
    ) {
        return Err(vec![issue(
            "template.seed_plan_source",
            "/template_ref",
            "Prepared mapping and seed must come from the same exact source version",
        )]);
    }
    if candidate.subset.seed.is_some() {
        return prepare_composition_seed(candidate, plan, choices, context);
    }
    if context.is_some() {
        return Err(vec![issue(
            "template.seed_source_missing",
            "/seed",
            "Scenario context was supplied but this source has no TemplateSubset.seed",
        )]);
    }
    prepare_legacy_template_seed(candidate, plan, choices)
}

fn prepare_legacy_template_seed(
    candidate: &TemplateCandidate,
    plan: &PreparedTemplate,
    choices: &TemplateSeedChoices,
) -> Result<ScenarioInput, Vec<Issue>> {
    let a = candidate.subset.adoption.as_ref().unwrap();
    let seed = a
        .deferred_seed
        .as_ref()
        .ok_or_else(|| vec![issue("template.seed_missing", "/seed", "No declared seed")])?;
    let sid = choices.scenario_id.clone();
    let mut scenario = ScenarioInput {
        acceptance_goal: None,
        quality_presets: vec![],
        id: sid.clone(),
        revision: Revision(SafeUint::try_from(1u64).unwrap()),
        project_revision: plan.proposed_project.revision.clone(),
        title: "Explicitly selected template example conditions".into(),
        time_reference: seed.time_reference.clone(),
        window: seed.window.clone(),
        initial_conditions: InitialConditions {
            items: vec![],
            initial_setup_families: BTreeMap::new(),
            declared_quality_decisions: vec![],
            material_allocations: vec![],
        },
        orders: vec![],
        production_tasks: vec![],
        arrivals: vec![],
        supplies: vec![],
        parameter_overrides: vec![],
        scope: plan
            .proposed_project
            .devices
            .iter()
            .map(|d| ObjectRef {
                kind: ObjectKind::Device,
                id: d.id.clone(),
            })
            .collect(),
        issues: vec![],
        cost_inputs: vec![],
    };
    if choices.supply {
        for (i, row) in seed.initial_stock.iter().enumerate() {
            let quality = choices
                .initial_quality
                .get(&row.location_id)
                .cloned()
                .ok_or_else(|| {
                    vec![issue(
                        "template.initial_quality",
                        "/initial_quality",
                        format!("Explicit quality required at {}", row.location_id.as_str()),
                    )]
                })?;
            let ty = candidate
                .example_project
                .item_types
                .iter()
                .find(|x| x.id == row.item_type_id)
                .unwrap();
            scenario.initial_conditions.items.push(InitialItem {
                id: id(format!("{}:stock:{i}", sid.as_str())),
                item_type_ref: SubjectRevisionRef {
                    id: mapped(&row.item_type_id, &plan.id_map),
                    revision: ty.revision.clone(),
                },
                quantity: row.quantity.clone(),
                holder: HolderRef::Location(mapped(&row.location_id, &plan.id_map)),
                quality,
                components: vec![],
                contents: vec![],
                source: SourceBasis::TemplateDeclaredExample,
            });
        }
    }
    if choices.demand {
        let product = match &seed.product_ref {
            Some(reference) => plan.proposed_project.products.iter().find(|product| {
                plan.id_map.get(&reference.id) == Some(&product.id)
                    && product.revision == reference.revision
            }),
            // Existing single-product examples omit product_ref. Multiple products
            // require an exact selection rather than choosing by vector order.
            None if plan.proposed_project.products.len() == 1 => {
                plan.proposed_project.products.first()
            }
            None => None,
        }
        .ok_or_else(|| {
            vec![issue(
                "template.product_missing",
                "/demand",
                "Select an exact example Product revision, or keep one unambiguous legacy Product/BOM/Route",
            )]
        })?;
        let oid = id(format!("{}:order", sid.as_str()));
        let quantity = seed.demand.quantity.clone();
        scenario.orders.push(OrderLine {
            id: oid.clone(),
            product_ref: SubjectRevisionRef {
                id: product.id.clone(),
                revision: product.revision.clone(),
            },
            quantity: Some(quantity.clone()),
            release_at_ms: seed.window.start_ms.clone(),
            due_at_ms: Some(seed.window.end_ms.clone()),
            destination: mapped(&seed.demand.destination_location_id, &plan.id_map),
            acceptance_stage: seed.demand.acceptance_stage.clone(),
            allow_partial: true,
            priority: 0,
        });
        scenario.production_tasks.push(ProductionTask {
            id: id(format!("{}:task", sid.as_str())),
            routing_ref: product.routing_ref.clone(),
            order_allocations: vec![OrderAllocation {
                order_id: oid,
                quantity: quantity.clone(),
            }],
            planned_quantity: quantity,
            release_at_ms: seed.window.start_ms.clone(),
            priority: 0,
        });
    }
    if choices.adopt_declared_inspection_results {
        let root = scenario.initial_conditions.items.first().ok_or_else(|| {
            vec![issue(
                "template.inspection_target",
                "/quality_presets",
                "Select actual supply before adopting an ancestor-specific preset",
            )]
        })?;
        for (i, preset) in a.inspection_presets.iter().enumerate() {
            scenario.quality_presets.push(InspectionPreset {
                id: id(format!("{}:inspection:{i}", sid.as_str())),
                item_or_ancestor: root.id.clone(),
                match_kind: QualityMatchKind::Ancestor,
                inspection_module: preset.inspection_module.clone(),
                results: preset.results.clone(),
            });
        }
    }
    Ok(scenario)
}

fn seed_reference(
    ids: &BTreeMap<ObjectId, ObjectId>,
    old: &ObjectId,
    path: &str,
) -> Result<ObjectId, Vec<Issue>> {
    ids.get(old).cloned().ok_or_else(|| {
        vec![issue(
            "template.seed_reference",
            path,
            format!("No selected source/mapping for {}", old.as_str()),
        )]
    })
}

fn prepare_composition_seed(
    candidate: &TemplateCandidate,
    plan: &PreparedTemplate,
    choices: &TemplateSeedChoices,
    context: Option<&TemplateSeedContext>,
) -> Result<ScenarioInput, Vec<Issue>> {
    let seed = candidate.subset.seed.as_ref().expect("generic seed branch");
    if candidate
        .subset
        .adoption
        .as_ref()
        .is_some_and(|a| a.deferred_seed.is_some())
    {
        return Err(vec![issue(
            "template.seed_competing_sources",
            "/seed",
            "TemplateSubset.seed and the legacy deferred summary coexist. Resolve the two source declarations explicitly; neither is silently flattened or preferred.",
        )]);
    }
    let context = context.ok_or_else(|| vec![issue("template.seed_context_required", "/seed_context", "TemplateSeed has no time reference, window or production-task declarations. Select an actual source Scenario context before adopting this seed.")])?;
    if context.window.end_ms.0.value() <= context.window.start_ms.0.value() {
        return Err(vec![issue(
            "template.seed_window",
            "/seed_context/window",
            "An explicit positive observation window is required",
        )]);
    }
    // Project identities and seed business identities are different namespaces:
    // canonical data may name both a Product and an Order "B".
    let mut ids = BTreeMap::new();
    let mut local = BTreeSet::new();
    let mut add = |old: &ObjectId, path: &str| -> Result<(), Vec<Issue>> {
        if ids.contains_key(old) || !local.insert(old.clone()) {
            return Err(vec![issue(
                "template.seed_duplicate_id",
                path,
                format!("Duplicate source identity {}", old.as_str()),
            )]);
        }
        let fresh = ObjectId::try_from(format!(
            "{}:seed:{}",
            choices.scenario_id.as_str(),
            old.as_str()
        ))
        .map_err(|e| vec![issue("template.seed_id", path, e)])?;
        if ids.values().any(|id| *id == fresh) {
            return Err(vec![issue(
                "template.seed_duplicate_id",
                path,
                "New seed identity collides with a prepared object",
            )]);
        }
        ids.insert(old.clone(), fresh);
        Ok(())
    };
    let source_items: Vec<_> = seed
        .initial_conditions
        .items
        .iter()
        .chain(seed.arrivals.iter().flat_map(|a| &a.items))
        .collect();
    if choices.supply {
        for item in &source_items {
            add(&item.id, "/seed/initial_conditions/items")?;
        }
        for arrival in &seed.arrivals {
            add(&arrival.id, "/seed/arrivals")?;
        }
        for allocation in &seed.initial_conditions.material_allocations {
            add(
                &allocation.id,
                "/seed/initial_conditions/material_allocations",
            )?;
        }
        for supply in &context.supplies {
            add(&supply.id, "/seed_context/supplies")?;
        }
        if choices.adopt_declared_inspection_results {
            for decision in &seed.initial_conditions.declared_quality_decisions {
                add(
                    &decision.id,
                    "/seed/initial_conditions/declared_quality_decisions",
                )?;
            }
            if let Some(adoption) = &candidate.subset.adoption {
                for preset in &adoption.inspection_presets {
                    add(&preset.id, "/adoption/inspection_presets")?;
                }
            }
        }
    }
    if choices.demand {
        for order in &seed.orders {
            add(&order.id, "/seed/orders")?;
        }
        for task in &context.production_tasks {
            add(&task.id, "/seed_context/production_tasks")?;
        }
    }
    if choices.supply && choices.adopt_declared_inspection_results {
        if let Some(adoption) = &candidate.subset.adoption {
            for preset in &adoption.inspection_presets {
                if candidate
                    .example_project
                    .template_adoptions
                    .iter()
                    .any(|a| a.inspection_presets.iter().any(|p| p.id == preset.id))
                {
                    if let Some(mapped) = plan.id_map.get(&preset.id) {
                        ids.insert(preset.id.clone(), mapped.clone());
                    }
                }
            }
        }
    }
    let mut scenario = ScenarioInput {
        acceptance_goal: None,
        quality_presets: vec![],
        id: choices.scenario_id.clone(),
        revision: Revision(SafeUint::try_from(1u64).unwrap()),
        project_revision: plan.proposed_project.revision.clone(),
        title: format!("{} · 模板示例", choices.scenario_id.as_str()),
        time_reference: context.time_reference.clone(),
        window: context.window.clone(),
        initial_conditions: InitialConditions {
            items: vec![],
            initial_setup_families: BTreeMap::new(),
            declared_quality_decisions: vec![],
            material_allocations: vec![],
        },
        orders: vec![],
        production_tasks: vec![],
        arrivals: vec![],
        supplies: vec![],
        parameter_overrides: vec![],
        // The one Engine admits only Device scope refs. Explicit cross-hall
        // supply needs the checked source's whole-Project scope, not fake Supply refs.
        scope: if choices.supply && !context.supplies.is_empty() {
            vec![]
        } else {
            plan.proposed_project
                .devices
                .iter()
                .map(|d| ObjectRef {
                    kind: ObjectKind::Device,
                    id: d.id.clone(),
                })
                .collect()
        },
        issues: vec![],
        cost_inputs: vec![],
    };
    let map = |old: &ObjectId, path: &str| seed_reference(&ids, old, path);
    let object_map = |old: &ObjectId, path: &str| seed_reference(&plan.id_map, old, path);
    if choices.supply {
        let map_item = |item: &InitialItem| -> Result<InitialItem, Vec<Issue>> {
            let mut value = item.clone();
            let ty = candidate
                .example_project
                .item_types
                .iter()
                .find(|ty| {
                    ty.id == item.item_type_ref.id
                        && same(&ty.revision, &item.item_type_ref.revision)
                })
                .ok_or_else(|| {
                    vec![issue(
                        "template.seed_item_type",
                        "/seed/items/item_type_ref",
                        "Seed item requires its exact declared source type",
                    )]
                })?;
            if ty.unit != item.quantity.unit {
                return Err(vec![issue(
                    "template.seed_item_unit",
                    "/seed/items/quantity",
                    "Seed quantity unit differs from its declared type",
                )]);
            }
            value.id = map(&item.id, "/seed/items/id")?;
            value.item_type_ref.id =
                object_map(&item.item_type_ref.id, "/seed/items/item_type_ref")?;
            value.holder = match &item.holder {
                HolderRef::Location(id)
                    if candidate
                        .example_project
                        .locations
                        .iter()
                        .any(|l| l.id == *id) =>
                {
                    HolderRef::Location(object_map(id, "/seed/items/holder")?)
                }
                HolderRef::Container(id) if source_items.iter().any(|item| item.id == *id) => {
                    HolderRef::Container(map(id, "/seed/items/holder")?)
                }
                _ => {
                    return Err(vec![issue(
                        "template.seed_runtime_holder",
                        "/seed/items/holder",
                        "Job/transport or missing-container holders require runtime state; template adoption does not copy work in progress",
                    )]);
                }
            };
            // Item keys distinguish two declared items at one location. The old
            // location-key choice is accepted only when that location is unique.
            let location_quality = if let HolderRef::Location(location) = &item.holder {
                (source_items
                    .iter()
                    .filter(
                        |other| matches!(&other.holder, HolderRef::Location(id) if id == location),
                    )
                    .count()
                    == 1)
                    .then(|| choices.initial_quality.get(location))
                    .flatten()
            } else {
                None
            };
            value.quality = choices
                .initial_quality
                .get(&item.id)
                .or(location_quality)
                .cloned()
                .ok_or_else(|| {
                    vec![issue(
                        "template.initial_quality",
                        "/initial_quality",
                        format!(
                            "Explicit item-specific quality required for {}",
                            item.id.as_str()
                        ),
                    )]
                })?;
            for reference in value.components.iter_mut().chain(&mut value.contents) {
                if !source_items.iter().any(|item| item.id == *reference) {
                    return Err(vec![issue(
                        "template.seed_item_link",
                        "/seed/items/components",
                        "Composition/content reference is not a selected declared seed item",
                    )]);
                }
                *reference = map(reference, "/seed/items/components")?;
            }
            Ok(value)
        };
        scenario.initial_conditions.items = seed
            .initial_conditions
            .items
            .iter()
            .map(map_item)
            .collect::<Result<_, _>>()?;
        for arrival in &seed.arrivals {
            let mut value = arrival.clone();
            value.id = map(&arrival.id, "/seed/arrivals/id")?;
            value.destination = object_map(&arrival.destination, "/seed/arrivals/destination")?;
            value.items = arrival
                .items
                .iter()
                .map(map_item)
                .collect::<Result<_, _>>()?;
            scenario.arrivals.push(value);
        }
        for (device, family) in &seed.initial_conditions.initial_setup_families {
            let mapped = object_map(device, "/seed/initial_conditions/initial_setup_families")?;
            if !plan.proposed_project.devices.iter().any(|d| d.id == mapped) {
                return Err(vec![issue(
                    "template.seed_setup_device",
                    "/seed/initial_conditions/initial_setup_families",
                    "Setup family must name a selected actual device",
                )]);
            }
            scenario
                .initial_conditions
                .initial_setup_families
                .insert(mapped, family.clone());
        }
        for allocation in &seed.initial_conditions.material_allocations {
            if !choices.demand
                || !context
                    .production_tasks
                    .iter()
                    .any(|t| t.id == allocation.demand_ref)
            {
                return Err(vec![issue(
                    "template.seed_allocation_task",
                    "/seed/initial_conditions/material_allocations",
                    "Reservation requires its explicitly declared and selected production task",
                )]);
            }
            if matches!(allocation.mode, AllocationMode::Committed) {
                return Err(vec![issue(
                    "template.seed_committed_allocation",
                    "/seed/initial_conditions/material_allocations",
                    "Committed work-in-progress allocation is not a template initial reservation",
                )]);
            }
            let mut value = allocation.clone();
            value.id = map(&allocation.id, "/seed/allocations/id")?;
            value.demand_ref = map(&allocation.demand_ref, "/seed/allocations/demand_ref")?;
            for item in &mut value.item_refs {
                if !source_items.iter().any(|i| i.id == *item) {
                    return Err(vec![issue(
                        "template.seed_allocation_item",
                        "/seed/allocations/item_refs",
                        "Reservation must reference a selected declared seed item",
                    )]);
                }
                *item = map(item, "/seed/allocations/item_refs")?;
            }
            scenario.initial_conditions.material_allocations.push(value);
        }
        for supply in &context.supplies {
            let mut value = supply.clone();
            value.id = map(&supply.id, "/seed_context/supplies/id")?;
            value.source = object_map(&supply.source, "/seed_context/supplies/source")?;
            value.destination =
                object_map(&supply.destination, "/seed_context/supplies/destination")?;
            value.item_type_ref.id = object_map(
                &supply.item_type_ref.id,
                "/seed_context/supplies/item_type_ref",
            )?;
            if let Some(path) = &mut value.presentation_path {
                path.hall_id = object_map(&path.hall_id, "/seed_context/supplies/presentation_path/hall_id")?;
                for slot in &mut path.cargo_layout.slots {
                    slot.item_type_ref.id = object_map(&slot.item_type_ref.id, "/seed_context/supplies/presentation_path/cargo_layout/slots/item_type_ref")?;
                }
            }
            for resource in value.resources.values_mut() {
                *resource = object_map(resource, "/seed_context/supplies/resources")?;
                if !plan
                    .proposed_project
                    .resources
                    .iter()
                    .any(|r| r.id == *resource)
                    && !plan
                        .mapped_bindings
                        .resources
                        .values()
                        .any(|r| r == resource)
                {
                    return Err(vec![issue(
                        "template.seed_supply_resource",
                        "/seed_context/supplies/resources",
                        "Supply requires an explicitly created or reused resource",
                    )]);
                }
            }
            if !candidate.definitions.iter().any(|d| {
                d.id == value.transport_ref.id
                    && same(&d.version, &value.transport_ref.version)
                    && matches!(d.payload, ModulePayload::Transport(_))
            }) {
                return Err(vec![issue(
                    "template.seed_transport",
                    "/seed_context/supplies/transport_ref",
                    "Exact declared transport definition is missing",
                )]);
            }
            scenario.supplies.push(value);
        }
    }
    if choices.demand {
        for order in &seed.orders {
            let mut value = order.clone();
            value.id = map(&order.id, "/seed/orders/id")?;
            value.product_ref.id = object_map(&order.product_ref.id, "/seed/orders/product_ref")?;
            value.destination = object_map(&order.destination, "/seed/orders/destination")?;
            if !plan.proposed_project.products.iter().any(|p| {
                p.id == value.product_ref.id && same(&p.revision, &value.product_ref.revision)
            }) {
                return Err(vec![issue(
                    "template.seed_product",
                    "/seed/orders/product_ref",
                    "Select the exact declared Product/BOM/Route; no first-product substitution is permitted",
                )]);
            }
            if !context.production_tasks.iter().any(|task| {
                task.order_allocations
                    .iter()
                    .any(|a| a.order_id == order.id)
            }) {
                return Err(vec![issue(
                    "template.seed_production_task",
                    "/seed_context/production_tasks",
                    format!(
                        "Order {} has no declared production task; a task/route will not be invented",
                        order.id.as_str()
                    ),
                )]);
            }
            scenario.orders.push(value);
        }
        for task in &context.production_tasks {
            let mut value = task.clone();
            value.id = map(&task.id, "/seed_context/production_tasks/id")?;
            value.routing_ref.id = object_map(
                &task.routing_ref.id,
                "/seed_context/production_tasks/routing_ref",
            )?;
            if !plan.proposed_project.routes.iter().any(|r| {
                r.id == value.routing_ref.id && same(&r.revision, &value.routing_ref.revision)
            }) {
                return Err(vec![issue(
                    "template.seed_route",
                    "/seed_context/production_tasks/routing_ref",
                    "Exact declared production route is absent",
                )]);
            }
            for allocation in &mut value.order_allocations {
                if !seed.orders.iter().any(|o| o.id == allocation.order_id) {
                    return Err(vec![issue(
                        "template.seed_task_order",
                        "/seed_context/production_tasks/order_allocations",
                        "Task allocation must name a selected declared order, not another mapped object",
                    )]);
                }
                allocation.order_id = map(
                    &allocation.order_id,
                    "/seed_context/production_tasks/order_allocations",
                )?;
            }
            scenario.production_tasks.push(value);
        }
    }
    if choices.adopt_declared_inspection_results {
        if !choices.supply {
            return Err(vec![issue(
                "template.inspection_target",
                "/quality_presets",
                "Declared quality examples require selected seed items",
            )]);
        }
        if let Some(adoption) = &candidate.subset.adoption {
            for preset in &adoption.inspection_presets {
                let item = preset.item_or_ancestor.as_ref().ok_or_else(|| vec![issue("template.seed_inspection_target", "/adoption/inspection_presets", "Generic compositions require an explicit item/ancestor; the first seed item is not a substitute")])?;
                if !source_items.iter().any(|i| i.id == *item) {
                    return Err(vec![issue(
                        "template.seed_inspection_target",
                        "/adoption/inspection_presets/item_or_ancestor",
                        "Preset must name a selected declared seed item/ancestor",
                    )]);
                }
                let match_kind = preset.match_kind.clone().ok_or_else(|| {
                    vec![issue(
                        "template.seed_inspection_match",
                        "/adoption/inspection_presets",
                        "Declare item/ancestor match semantics explicitly",
                    )]
                })?;
                scenario.quality_presets.push(InspectionPreset {
                    id: map(&preset.id, "/adoption/inspection_presets/id")?,
                    item_or_ancestor: map(item, "/adoption/inspection_presets/item_or_ancestor")?,
                    match_kind,
                    inspection_module: preset.inspection_module.clone(),
                    results: preset.results.clone(),
                });
            }
        }
        for decision in &seed.initial_conditions.declared_quality_decisions {
            if !source_items.iter().any(|i| i.id == decision.item_ref) {
                return Err(vec![issue(
                    "template.seed_inspection_target",
                    "/seed/quality/item_ref",
                    "Quality decision must name a selected declared seed item",
                )]);
            }
            if matches!(decision.source_kind, QualitySourceKind::ToolResult)
                || matches!(decision.decision_source, DecisionSource::ProviderResult(_))
            {
                return Err(vec![issue(
                    "template.seed_quality_evidence",
                    "/seed/initial_conditions/declared_quality_decisions",
                    "A template cannot manufacture or carry provider-result authority as a declared preset",
                )]);
            }
            let mut value = decision.clone();
            value.id = map(&decision.id, "/seed/quality/id")?;
            value.item_ref = map(&decision.item_ref, "/seed/quality/item_ref")?;
            if let DecisionSource::ScheduledPreset(preset) = &mut value.decision_source {
                *preset = map(preset, "/seed/quality/decision_source")?;
            }
            scenario
                .initial_conditions
                .declared_quality_decisions
                .push(value);
        }
    }
    Ok(scenario)
}
