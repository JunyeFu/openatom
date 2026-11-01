//! Actual selected-project template composition, kept in the definition session
//! until an immutable version is explicitly committed.
use super::host_definitions::DefinitionDraftState;
use super::*;

#[derive(Clone)]
pub(super) struct CompositionState {
    pub subset: TemplateSubset,
    pub source: ProjectState,
    pub source_scenario: Option<ScenarioInput>,
    pub files: Vec<ArchiveAddition>,
    pub dependencies: Vec<ModuleRef>,
    pub issues: Vec<Issue>,
    pub assets: Vec<AssetRef>,
    pub rights: Vec<AssetRights>,
    next_document: u64,
}

fn invalid_composition(message: &str, path: &str) -> ApiError {
    error(ErrorCode::InvalidInput, message, Some(path))
}
fn composition_issue(message: impl Into<String>, path: &str) -> Issue {
    Issue {
        code: "template_composition.missing_input".into(),
        message: message.into(),
        field_path: Some(path.into()),
        object_ref: None,
        blocks: vec![CapabilityPurpose::CompleteConclusion],
    }
}
fn composition_value(value: Option<&CompositionState>) -> Value {
    serde_json::json!({"template_composition":value.map(|state| &state.subset),"source_scenario":value.and_then(|state| state.source_scenario.as_ref()).map(|scenario|ScenarioReference{scenario_id:scenario.id.clone(),revision:scenario.revision.clone()})})
}

fn composition_media_refs(presentation: &Presentation) -> impl Iterator<Item = &ModuleRef> {
    std::iter::once(&presentation.source)
        .chain(presentation.lod.detailed.iter())
        .chain(presentation.lod.simplified.iter())
        .filter_map(|locator| match locator {
            MediaLocator::MediaRef(reference) => Some(reference),
            _ => None,
        })
}

fn composition_assets(
    value: &Value,
    out: &mut BTreeMap<AssetId, AssetRef>,
) -> Result<(), ApiError> {
    if let Ok(asset) = serde_json::from_value::<AssetRef>(value.clone()) {
        if out
            .get(&asset.id)
            .is_some_and(|actual| !exact_data(actual, &asset))
        {
            return Err(invalid_composition(
                "One selected asset ID must retain its exact actual metadata",
                "/subset",
            ));
        }
        out.insert(asset.id.clone(), asset);
        return Ok(());
    }
    match value {
        Value::Array(values) => {
            for value in values {
                composition_assets(value, out)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                composition_assets(value, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl ProjectHost {
    pub(super) fn composition_command_available(&self, caller: &HostCallContext) -> bool {
        self.composition_project_read(caller).is_ok() && self.definition_drafts.iter().any(|(id,draft)|matches!(draft.definition.payload,ModulePayload::Templates(_)) && caller.allowed_definition_drafts.contains(id) && caller.edit_grants.iter().any(|grant|matches!(&grant.scope,TaskScope::Edit(EditScope::DefinitionDraft(identity)) if &identity.id==id && exact_data(&identity.revision,&draft.revision)) && grant.branches.iter().any(|branch|matches!(branch,HostEditBranch::PutDefinition)) && grant.objects.iter().any(|object|matches!(object.target.kind,ObjectKind::DefinitionDraft) && object.target.id.as_str()==id.as_str() && object.fields.iter().any(|field|editable_existing_leaf(field,&composition_value(draft.composition.as_ref()),false)))))
    }
    pub(super) fn composition_preview(
        &self,
        caller: &HostCallContext,
        id: &ModuleId,
        target: &ObjectRef,
        reference: &ModuleRef,
        base: &Revision,
        choices: Option<&TemplatePreparationChoices>,
        seed: Option<&TemplateSeedSelection>,
    ) -> Result<TemplatePreview, ApiError> {
        self.composition_project_read(caller)?;
        let draft = &self.definition_drafts[id];
        check_revision(base, &draft.revision, "/params/source/data/base_revision")?;
        if !matches!(target.kind, ObjectKind::Module)
            || target.id.as_str() != reference.id.as_str()
            || reference.id != draft.definition.id
            || reference.version != draft.definition.version
        {
            return Err(invalid_composition(
                "Draft template preview requires its actual exact definition identity",
                "/params/source",
            ));
        }
        let state = draft.composition.as_ref().ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "This draft has no captured actual template composition",
                Some("/params/source"),
            )
        })?;
        let ModulePayload::Templates(template) = &draft.definition.payload else {
            return Err(invalid_composition(
                "Draft is not Templates",
                "/params/source",
            ));
        };
        let subset = state
            .files
            .iter()
            .find(|file| file.entry.file_ref.path == template.project)
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Actual draft template sidecar is absent",
                    Some("/params/source"),
                )
            })?;
        let source_project = &state
            .subset
            .adoption
            .as_ref()
            .expect("captured canonical source")
            .source_archive
            .path;
        if choices.is_some_and(|choices| &choices.source_project != source_project)
            || (choices.is_none() && seed.is_some())
        {
            return Err(invalid_composition(
                "Select this actual captured source Project and explicit preparation choices",
                "/params/source",
            ));
        }
        let source = state
            .files
            .iter()
            .find(|file| &file.entry.file_ref.path == source_project)
            .ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Actual draft source Project sidecar is absent",
                    Some("/params/source"),
                )
            })?;
        let package = ModulePackage {
            definition: draft.definition.clone(),
            assets: state.assets.clone(),
            files: state
                .files
                .iter()
                .map(|file| file.entry.file_ref.clone())
                .collect(),
            dependencies: state.dependencies.clone(),
            rights: state.rights.clone(),
            issues: state.issues.clone(),
        };
        let definitions: Vec<_> = state
            .dependencies
            .iter()
            .map(|reference| {
                self.modules
                    .get(&module_key(reference))
                    .map(|module| module.definition.clone())
                    .ok_or_else(|| {
                        error(
                            ErrorCode::MissingReference,
                            "Exact captured dependency is absent",
                            Some("/params/source"),
                        )
                    })
            })
            .collect::<Result<_, _>>()?;
        let candidate = sf_module_kit::decode_template_candidate(
            &encode_document(&package),
            &subset.bytes,
            &source.bytes,
            &serde_json::to_vec(&definitions).unwrap(),
        )
        .map_err(engine_failure)?;
        self.preview_template_candidate(
            self.project.as_ref().unwrap(),
            reference,
            source_project.clone(),
            candidate,
            &state
                .files
                .iter()
                .map(|file| {
                    (
                        file.entry.file_ref.path.as_str().to_owned(),
                        file.bytes.clone(),
                    )
                })
                .collect(),
            choices,
            seed,
        )
    }
    pub(super) fn composition_project_read(
        &self,
        caller: &HostCallContext,
    ) -> Result<(), ApiError> {
        if matches!(caller.origin, CallerOrigin::InteractiveUi)
            || (self
                .project
                .as_ref()
                .is_some_and(|project| caller.granted_project.as_ref() == Some(&project.id))
                && caller
                    .allowed_operations
                    .iter()
                    .any(|op| matches!(op, OperationName::GetObject)))
        {
            Ok(())
        } else {
            Err(error(
                ErrorCode::PermissionDenied,
                "Template composition requires current authority to read the selected Project",
                None,
            ))
        }
    }

    fn composition_directory(&self) -> (String, u64) {
        let mut ordinal = self.next_document;
        loop {
            let directory = format!("documents/template-composition-{ordinal:08}");
            let prefix = format!("{directory}/");
            ordinal += 1;
            let occupied = self
                .additions
                .iter()
                .any(|file| file.entry.file_ref.path.as_str().starts_with(&prefix))
                || self.backing.as_ref().is_some_and(|archive| {
                    archive.files.keys().any(|path| path.starts_with(&prefix))
                })
                || self
                    .definition_drafts
                    .values()
                    .filter_map(|draft| draft.composition.as_ref())
                    .any(|state| {
                        state
                            .files
                            .iter()
                            .any(|file| file.entry.file_ref.path.as_str().starts_with(&prefix))
                    });
            if !occupied {
                return (directory, ordinal);
            }
        }
    }

    pub(super) fn prepare_composition(
        &self,
        caller: &HostCallContext,
        id: &ModuleId,
        draft: &DefinitionDraftState,
        input: &TemplateComposition,
    ) -> Result<(ModuleDefinition, CompositionState), ApiError> {
        self.composition_project_read(caller)?;
        let project = self.project.as_ref().expect("admitted active Project");
        check_revision(
            &input.project_revision,
            &project.revision,
            "/params/commands/data/project_revision",
        )?;
        let target = ObjectRef {
            kind: ObjectKind::DefinitionDraft,
            id: ObjectId::try_from(id.as_str().to_owned()).unwrap(),
        };
        self.definition_fields_authorized(
            caller,
            id,
            &HostEditBranch::PutDefinition,
            &target,
            &serde_json::to_value(&draft.definition).unwrap(),
            &serde_json::to_value(&input.definition).unwrap(),
        )?;
        self.definition_fields_authorized(
            caller,
            id,
            &HostEditBranch::PutDefinition,
            &target,
            &composition_value(draft.composition.as_ref()),
            &serde_json::json!({"template_composition":input.subset,"source_scenario":input.source_scenario}),
        )?;
        if input.definition.id != draft.definition.id
            || input.definition.version != draft.definition.version
        {
            return Err(invalid_composition(
                "Template composition preserves draft module identity/version",
                "/definition",
            ));
        }
        let ModulePayload::Templates(_) = &input.definition.payload else {
            return Err(invalid_composition(
                "Composition requires a Templates definition",
                "/definition/payload",
            ));
        };
        let mut subset = input.subset.clone();
        let source_scenario = match (&subset.seed, &input.source_scenario) {
            (None, None) => None,
            (Some(seed), Some(reference)) => {
                let scenario = self.scenarios.get(&reference.scenario_id).ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Selected seed Scenario is not a current actual input",
                        Some("/source_scenario"),
                    )
                })?;
                check_revision(
                    &reference.revision,
                    &scenario.revision,
                    "/source_scenario/revision",
                )?;
                if !exact_data(&scenario.project_revision, &project.revision)
                    || !exact_data(&seed.initial_conditions, &scenario.initial_conditions)
                    || !exact_data(&seed.arrivals, &scenario.arrivals)
                    || !exact_data(&seed.orders, &scenario.orders)
                {
                    return Err(invalid_composition(
                        "Seed bodies and Project revision must match the explicitly selected actual Scenario",
                        "/subset/seed",
                    ));
                }
                Some(scenario.clone())
            }
            _ => {
                return Err(invalid_composition(
                    "A full seed requires an explicit exact source_scenario; no seed captures no Scenario",
                    "/source_scenario",
                ));
            }
        };

        if let Some(adoption) = &subset.adoption {
            if !draft
                .composition
                .as_ref()
                .and_then(|state| state.subset.adoption.as_ref())
                .is_some_and(|actual| exact_data(actual, adoption))
            {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Template source/claim metadata is generated from actual selected inputs, not accepted from a caller",
                    Some("/subset/adoption"),
                ));
            }
        }
        macro_rules! selected {
            ($field:ident,$identity:expr) => {{
                let mut seen = BTreeSet::new();
                for item in &subset.$field {
                    let identity = ($identity)(item);
                    if !seen.insert(identity.clone())
                        || !project.$field.iter().any(|source| {
                            ($identity)(source) == identity && exact_data(source, item)
                        })
                    {
                        return Err(invalid_composition(
                            "Selected object must be a distinct exact current Project object",
                            concat!("/subset/", stringify!($field)),
                        ));
                    }
                }
            }};
        }
        selected!(halls, |value: &HallDesign| value.id.clone());
        selected!(lines, |value: &LineGroup| value.id.clone());
        selected!(devices, |value: &DeviceInstance| value.id.clone());
        selected!(connections, |value: &Connection| connection_id(value)
            .clone());
        selected!(item_types, |value: &ItemType| value.id.clone());
        selected!(locations, |value: &StockLocation| value.id.clone());
        if subset.halls.is_empty()
            && subset.lines.is_empty()
            && subset.devices.is_empty()
            && subset.connections.is_empty()
            && subset.item_types.is_empty()
            && subset.locations.is_empty()
        {
            return Err(invalid_composition(
                "Explicitly select at least one actual Project object",
                "/subset",
            ));
        }
        let mut dependencies = BTreeMap::new();
        let permitted: BTreeSet<_> = project
            .module_refs
            .iter()
            .chain(subset.devices.iter().flat_map(|device| {
                std::iter::once(&device.module_ref)
                    .chain(device.operation_binding.iter())
                    .chain(device.route_operation_refs.iter())
            }))
            .map(module_key)
            .collect();
        let resource_ids: BTreeSet<_> = subset
            .devices
            .iter()
            .flat_map(|device| device.role_bindings.resources.values().cloned())
            .chain(subset.suggested_role_bindings.resources.values().cloned())
            .chain(source_scenario.iter().flat_map(|scenario| {
                scenario
                    .supplies
                    .iter()
                    .flat_map(|supply| supply.resources.values().cloned())
            }))
            .collect();
        let required_routes: Vec<_> = source_scenario
            .iter()
            .flat_map(|scenario| {
                scenario
                    .production_tasks
                    .iter()
                    .map(|task| task.routing_ref.clone())
                    .chain(
                        scenario
                            .orders
                            .iter()
                            .filter_map(|order| {
                                project.products.iter().find(|product| {
                                    product.id == order.product_ref.id
                                        && product.revision == order.product_ref.revision
                                })
                            })
                            .map(|product| product.routing_ref.clone()),
                    )
            })
            .chain(
                project
                    .setup_policies
                    .iter()
                    .filter(|policy| {
                        subset
                            .devices
                            .iter()
                            .any(|device| device.id == policy.device_id)
                    })
                    .filter_map(|policy| policy.first_article.as_ref())
                    .map(|article| article.route_ref.clone()),
            )
            .collect();
        let required = subset
            .module_refs
            .iter()
            .cloned()
            .chain(subset.devices.iter().flat_map(|device| {
                std::iter::once(device.module_ref.clone())
                    .chain(device.operation_binding.clone())
                    .chain(device.route_operation_refs.clone())
            }))
            .chain(
                subset
                    .devices
                    .iter()
                    .filter_map(|device| device.presentation_override.as_ref())
                    .chain(
                        project
                            .resources
                            .iter()
                            .filter(|resource| resource_ids.contains(&resource.id))
                            .filter_map(|resource| resource.presentation.as_ref()),
                    )
                    .chain(
                        subset
                            .resource_requirements
                            .iter()
                            .filter_map(|requirement| requirement.presentation.as_ref()),
                    )
                    .flat_map(composition_media_refs)
                    .cloned(),
            )
            .chain(
                project
                    .resources
                    .iter()
                    .filter(|resource| resource_ids.contains(&resource.id))
                    .map(|resource| resource.definition_ref.clone()),
            )
            .chain(source_scenario.iter().flat_map(|scenario| {
                scenario
                    .supplies
                    .iter()
                    .map(|supply| supply.transport_ref.clone())
            }))
            .chain(
                project
                    .setup_policies
                    .iter()
                    .filter(|policy| {
                        subset
                            .devices
                            .iter()
                            .any(|device| device.id == policy.device_id)
                    })
                    .filter_map(|policy| policy.first_article.as_ref())
                    .map(|article| article.inspection_module.clone()),
            )
            .chain(
                project
                    .routes
                    .iter()
                    .filter(|route| {
                        required_routes.iter().any(|reference| {
                            reference.id == route.id && reference.revision == route.revision
                        })
                    })
                    .flat_map(|route| route.steps.iter().map(|step| step.operation_ref.clone())),
            )
            .collect::<Vec<_>>();
        let mut pending = required;
        while let Some(reference) = pending.pop() {
            let key = module_key(&reference);
            if dependencies.contains_key(&key) {
                continue;
            }
            if !permitted.contains(&key)
                && !dependencies.values().any(|package: &ModulePackage| {
                    package
                        .dependencies
                        .iter()
                        .any(|dependency| module_key(dependency) == key)
                })
            {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Composition module is not an explicitly selected current source or an exact dependency",
                    Some("/subset/module_refs"),
                ));
            }
            let package = self.modules.get(&key).ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Exact selected composition module is not loaded",
                    Some("/subset/module_refs"),
                )
            })?;
            pending.extend(package.dependencies.clone());
            dependencies.insert(key, package.clone());
        }
        subset.module_refs = dependencies
            .values()
            .map(|package| ModuleRef {
                id: package.definition.id.clone(),
                version: package.definition.version.clone(),
            })
            .collect();
        let mut source = empty_project(
            project.id.clone(),
            NewProject {
                title: input.definition.title.clone(),
                time_reference: project.time_reference.clone(),
                departure: DepartureDecision::Stay,
            },
        );
        source.revision = project.revision.clone();
        source.halls = subset.halls.clone();
        source.lines = subset.lines.clone();
        source.devices = subset.devices.clone();
        source.connections = subset.connections.clone();
        source.item_types = subset.item_types.clone();
        source.locations = subset.locations.clone();
        host_item_layout::validate_references(&source)?;
        source.module_refs = subset.module_refs.clone();
        let ids: BTreeSet<_> = source
            .devices
            .iter()
            .map(|device| device.id.clone())
            .chain(source.item_types.iter().map(|item| item.id.clone()))
            .chain(source.locations.iter().map(|location| location.id.clone()))
            .chain(source.halls.iter().map(|hall| hall.id.clone()))
            .chain(source.lines.iter().map(|line| line.id.clone()))
            .collect();
        let resources = resource_ids;
        source.resources = project
            .resources
            .iter()
            .filter(|resource| resources.contains(&resource.id))
            .cloned()
            .collect();
        let calendars: BTreeSet<_> = source
            .resources
            .iter()
            .map(|resource| resource.calendar_id.clone())
            .collect();
        source.calendars = project
            .calendars
            .iter()
            .filter(|calendar| calendars.contains(&calendar.id))
            .cloned()
            .collect();
        source.setup_policies = project
            .setup_policies
            .iter()
            .filter(|policy| ids.contains(&policy.device_id))
            .cloned()
            .collect();
        source.parameter_sources = project
            .parameter_sources
            .iter()
            .filter(|parameter| ids.contains(&parameter.target.id))
            .cloned()
            .collect();
        let source_ids: BTreeSet<_> = source
            .parameter_sources
            .iter()
            .filter_map(|parameter| parameter.source_ref.as_ref())
            .chain(source.setup_policies.iter().flat_map(|policy| {
                policy
                    .changeovers
                    .iter()
                    .filter_map(|rule| rule.duration.source_ref.as_ref())
            }))
            .collect();
        source.sources = project
            .sources
            .iter()
            .filter(|record| source_ids.contains(&record.id))
            .cloned()
            .collect();
        let mut issues = vec![];
        for resource in &resources {
            if !source.resources.iter().any(|actual| &actual.id == resource) {
                issues.push(composition_issue(
                    format!(
                        "Selected role needs missing actual resource {}",
                        resource.as_str()
                    ),
                    "/subset/suggested_role_bindings/resources",
                ));
            }
        }
        let mut resource_choices = vec![];
        let mut assigned_requirements = BTreeSet::new();
        for resource in &source.resources {
            let matches: Vec<_> = subset
                .resource_requirements
                .iter()
                .enumerate()
                .filter(|(index, requirement)| {
                    !assigned_requirements.contains(index)
                        && requirement
                            .kind
                            .as_ref()
                            .is_none_or(|kind| same(kind, &resource.kind))
                        && requirement
                            .role
                            .as_ref()
                            .is_none_or(|role| role == &resource.role)
                        && requirement
                            .capacity
                            .as_ref()
                            .is_none_or(|capacity| exact_data(capacity, &resource.capacity))
                        && requirement
                            .qualifications
                            .iter()
                            .all(|qualification| resource.qualifications.contains(qualification))
                        && (requirement.default_calendar_ms.is_empty()
                            || source.calendars.iter().any(|calendar| {
                                calendar.id == resource.calendar_id
                                    && exact_data(
                                        &calendar.intervals,
                                        &requirement.default_calendar_ms,
                                    )
                            }))
                })
                .collect();
            if !matches.is_empty()
                && matches
                    .iter()
                    .all(|(_, requirement)| exact_data(*requirement, matches[0].1))
            {
                assigned_requirements.insert(matches[0].0);
                resource_choices.push(TemplateResourceChoice {
                    source_id: resource.id.clone(),
                    requirement_index: SafeUint::try_from(matches[0].0 as u64).unwrap(),
                    resource_id: None,
                });
            } else {
                issues.push(composition_issue(
                    format!(
                        "Declare one unambiguous actual requirement for resource {}",
                        resource.id.as_str()
                    ),
                    "/subset/resource_requirements",
                ));
            }
        }
        for issue in &mut issues {
            issue.blocks.push(CapabilityPurpose::Adoption);
        }
        if let Some(seed) = &subset.seed {
            let products: BTreeSet<_> = seed
                .orders
                .iter()
                .map(|order| (&order.product_ref.id, &order.product_ref.revision))
                .collect();
            source.products = project
                .products
                .iter()
                .filter(|product| {
                    products.iter().any(|(id, revision)| {
                        *id == &product.id && exact_data(*revision, &product.revision)
                    })
                })
                .cloned()
                .collect();
            source.boms = project
                .boms
                .iter()
                .filter(|bom| {
                    source.products.iter().any(|product| {
                        product.bom_ref.id == bom.id && product.bom_ref.revision == bom.revision
                    })
                })
                .cloned()
                .collect();
            source.routes = project
                .routes
                .iter()
                .filter(|route| {
                    source.products.iter().any(|product| {
                        product.routing_ref.id == route.id
                            && product.routing_ref.revision == route.revision
                    })
                })
                .cloned()
                .collect();
        }
        for reference in &required_routes {
            if let Some(route) = project
                .routes
                .iter()
                .find(|route| route.id == reference.id && route.revision == reference.revision)
            {
                if !source
                    .routes
                    .iter()
                    .any(|existing| existing.id == route.id && existing.revision == route.revision)
                {
                    source.routes.push(route.clone());
                }
            }
        }
        for product in project.products.iter().filter(|product| {
            required_routes.iter().any(|reference| {
                product.routing_ref.id == reference.id
                    && product.routing_ref.revision == reference.revision
            })
        }) {
            if !source
                .products
                .iter()
                .any(|existing| existing.id == product.id && existing.revision == product.revision)
            {
                source.products.push(product.clone());
            }
        }
        for bom in project.boms.iter().filter(|bom| {
            source.products.iter().any(|product| {
                product.bom_ref.id == bom.id && product.bom_ref.revision == bom.revision
            })
        }) {
            if !source
                .boms
                .iter()
                .any(|existing| existing.id == bom.id && existing.revision == bom.revision)
            {
                source.boms.push(bom.clone());
            }
        }
        // No actual inventory, Task, Run, acknowledgment or binding is copied.
        source.scenario_refs.clear();
        source.artifact_refs.clear();
        source.acknowledgments.clear();
        source.bindings.clear();
        source.generation_context = None;
        let (directory, next_document) = self.composition_directory();
        let mut definition = input.definition.clone();
        let path = RelativeFilePath::try_from(format!("{directory}/project.subset.json")).unwrap();
        let source_path = file_ref(
            &format!("{directory}/source-project.json"),
            DocumentKind::Asset,
        );
        let reference = ModuleRef {
            id: definition.id.clone(),
            version: definition.version.clone(),
        };
        let mut inspection_presets = vec![];
        if let Some(scenario) = &source_scenario {
            for preset in &scenario.quality_presets {
                let consumers: BTreeSet<_> = source
                    .devices
                    .iter()
                    .filter(|device| {
                        device.operation_binding.as_ref().is_some_and(|reference| {
                            module_key(reference) == module_key(&preset.inspection_module)
                        }) || device.route_operation_refs.iter().any(|reference| {
                            module_key(reference) == module_key(&preset.inspection_module)
                        })
                    })
                    .map(|device| device.id.clone())
                    .chain(
                        source
                            .routes
                            .iter()
                            .flat_map(|route| {
                                route
                                    .steps
                                    .iter()
                                    .filter(|step| {
                                        module_key(&step.operation_ref)
                                            == module_key(&preset.inspection_module)
                                    })
                                    .map(|step| step.device_id.clone())
                            })
                            .filter(|id| source.devices.iter().any(|device| &device.id == id)),
                    )
                    .collect();
                if consumers.len() == 1 {
                    inspection_presets.push(TemplateInspectionPreset {
                        id: preset.id.clone(),
                        consumer_device_id: consumers.into_iter().next().unwrap(),
                        inspection_module: preset.inspection_module.clone(),
                        item_or_ancestor: Some(preset.item_or_ancestor.clone()),
                        match_kind: Some(preset.match_kind.clone()),
                        results: preset.results.clone(),
                    });
                } else {
                    issues.push(composition_issue(
                        format!(
                            "Select one actual inspection consumer for declared preset {}",
                            preset.id.as_str()
                        ),
                        "/source_scenario/quality_presets",
                    ));
                }
            }
        }
        subset.adoption = Some(TemplateAdoption {
            id: ObjectId::try_from(format!("{}:composition", id.as_str())).unwrap(),
            template_ref: reference,
            source_archive: source_path.clone(),
            resource_choices,
            preparation_bindings: vec![],
            imported_claims: vec![],
            inspection_presets,
            deferred_seed: None,
            missing_inputs: vec![],
        });
        source.template_adoptions = vec![subset.adoption.clone().unwrap()];
        let ModulePayload::Templates(template) = &mut definition.payload else {
            unreachable!()
        };
        let mut extra_files = vec![];
        if let Some(profile) = &template.generation_profile {
            let allowed = draft.composition.as_ref().is_some_and(|state| {
                state.files.iter().any(|file| {
                    file.entry.file_ref.path == *profile
                        && matches!(file.entry.file_ref.kind, DocumentKind::GenerationProfile)
                })
            }) || project
                .module_refs
                .iter()
                .filter_map(|reference| self.modules.get(&module_key(reference)))
                .any(|module| {
                    module.files.iter().any(|file| {
                        file.path == *profile
                            && matches!(file.kind, DocumentKind::GenerationProfile)
                    })
                });
            if !allowed {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Composition may reference only a declared current module generation profile, never an arbitrary included file",
                    Some("/definition/payload/data/generation_profile"),
                ));
            }
            let file = self
                .additions
                .iter()
                .find(|addition| {
                    &addition.entry.file_ref.path == profile
                        && matches!(
                            addition.entry.file_ref.kind,
                            DocumentKind::GenerationProfile
                        )
                })
                .cloned()
                .or_else(|| {
                    let archive = self.backing.as_ref()?;
                    let entry = archive.manifest.documents.iter().find(|entry| {
                        &entry.file_ref.path == profile
                            && matches!(entry.file_ref.kind, DocumentKind::GenerationProfile)
                    })?;
                    Some(ArchiveAddition {
                        entry: entry.clone(),
                        bytes: archive.files.get(profile.as_str())?.clone(),
                    })
                })
                .ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Composition generation profile must be an actual included source file",
                        Some("/definition/payload/data/generation_profile"),
                    )
                })?;
            extra_files.push(file);
        }
        template.project = path.clone();
        let target = ObjectRef {
            kind: ObjectKind::Module,
            id: ObjectId::try_from(definition.id.as_str().to_owned()).unwrap(),
        };
        let mut asset_refs = BTreeMap::new();
        composition_assets(&serde_json::to_value(&subset).unwrap(), &mut asset_refs)?;
        composition_assets(&serde_json::to_value(&source).unwrap(), &mut asset_refs)?;
        let mut asset_files = vec![];
        let mut rights = vec![];
        for asset in asset_refs.values() {
            let actual = self
                .modules
                .values()
                .flat_map(|package| &package.assets)
                .find(|actual| actual.id == asset.id && exact_data(*actual, asset));
            let entry = self
                .additions
                .iter()
                .find(|file| file.entry.file_ref.path == asset.path)
                .cloned()
                .or_else(|| {
                    let backing = self.backing.as_ref()?;
                    let entry = backing
                        .manifest
                        .documents
                        .iter()
                        .find(|entry| entry.file_ref.path == asset.path)?;
                    Some(ArchiveAddition {
                        entry: entry.clone(),
                        bytes: backing.files.get(asset.path.as_str())?.clone(),
                    })
                })
                .ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Selected composition asset has no actual included bytes",
                        Some("/subset"),
                    )
                })?;
            if actual.is_none()
                && !(matches!(entry.entry.object_ref.kind, ObjectKind::Asset)
                    && entry.entry.object_ref.id.as_str() == asset.id.as_str())
            {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Composition references only actual declared media assets",
                    Some("/subset"),
                ));
            }
            let right = self
                .modules
                .values()
                .flat_map(|package| &package.rights)
                .chain(
                    self.backing
                        .iter()
                        .flat_map(|archive| &archive.manifest.assets),
                )
                .find(|right| right.asset_ref == asset.id)
                .cloned()
                .ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Selected asset has no actual canonical rights record",
                        Some("/subset"),
                    )
                })?;
            if let Some(declared) = definition
                .contribution
                .assets
                .iter()
                .find(|declared| declared.asset_ref == asset.id)
            {
                if !exact_data(declared, &right) {
                    return Err(invalid_composition(
                        "Composition cannot upgrade actual source asset rights",
                        "/definition/contribution/assets",
                    ));
                }
            } else {
                definition.contribution.assets.push(right.clone());
            }
            rights.push(right);
            asset_files.push(entry);
        }
        let files = vec![
            (
                FileRef {
                    path,
                    kind: DocumentKind::Template,
                },
                encode_document(&subset),
            ),
            (source_path, encode_document(&source)),
        ];
        let mut files: Vec<_> = files
            .into_iter()
            .map(|(file_ref, bytes)| ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::SupportingFile,
                    module_ref: None,
                    object_ref: target.clone(),
                    revision: None,
                    file_ref,
                    required: true,
                },
                bytes,
            })
            .collect();
        if let Some(scenario) = &source_scenario {
            files.push(ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::SupportingFile,
                    module_ref: None,
                    object_ref: ObjectRef {
                        kind: ObjectKind::Scenario,
                        id: ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap(),
                    },
                    revision: Some(scenario.revision.clone()),
                    file_ref: file_ref(
                        &format!("{directory}/source-scenario.json"),
                        DocumentKind::Scenario,
                    ),
                    required: true,
                },
                bytes: encode_document(scenario),
            });
        }
        files.extend(extra_files);
        files.extend(asset_files);
        let state = CompositionState {
            subset,
            source,
            source_scenario,
            files,
            dependencies: dependencies
                .values()
                .map(|package| ModuleRef {
                    id: package.definition.id.clone(),
                    version: package.definition.version.clone(),
                })
                .collect(),
            issues,
            assets: asset_refs.into_values().collect(),
            rights,
            next_document,
        };
        Ok((definition, state))
    }

    pub(super) fn authorize_composition_history(
        &self,
        caller: &HostCallContext,
        id: &ModuleId,
        target: &ObjectRef,
        before: &DefinitionDraftState,
        after: &Option<CompositionState>,
        undo: bool,
    ) -> Result<(), ApiError> {
        self.definition_fields_authorized(
            caller,
            id,
            if undo {
                &HostEditBranch::Undo
            } else {
                &HostEditBranch::Redo
            },
            target,
            &composition_value(before.composition.as_ref()),
            &composition_value(after.as_ref()),
        )
    }

    pub(super) fn composition_package(
        &self,
        definition: &mut ModuleDefinition,
        state: &mut CompositionState,
    ) -> Result<(ModulePackage, Vec<ArchiveAddition>), ApiError> {
        let reference = ModuleRef {
            id: definition.id.clone(),
            version: definition.version.clone(),
        };
        let (directory, next_document) = self.composition_directory();
        let subset_file = file_ref(
            &format!("{directory}/project.subset.json"),
            DocumentKind::Template,
        );
        let source_file = file_ref(
            &format!("{directory}/source-project.json"),
            DocumentKind::Asset,
        );
        let adoption = state
            .subset
            .adoption
            .as_mut()
            .expect("authored canonical adoption");
        adoption.template_ref = reference;
        adoption.source_archive = source_file.clone();
        state.source.template_adoptions = vec![adoption.clone()];
        let ModulePayload::Templates(template) = &mut definition.payload else {
            return Err(invalid_composition(
                "A composed definition must remain Templates until its composition is explicitly replaced",
                "/payload",
            ));
        };
        template.project = subset_file.path.clone();
        let target = ObjectRef {
            kind: ObjectKind::Module,
            id: ObjectId::try_from(definition.id.as_str().to_owned()).unwrap(),
        };
        let files = vec![
            (subset_file, encode_document(&state.subset)),
            (source_file, encode_document(&state.source)),
        ];
        let profile_files: Vec<_> = state
            .files
            .iter()
            .filter(|file| matches!(file.entry.file_ref.kind, DocumentKind::GenerationProfile))
            .cloned()
            .collect();
        let asset_files: Vec<_> = state
            .files
            .iter()
            .filter(|file| {
                state
                    .assets
                    .iter()
                    .any(|asset| asset.path == file.entry.file_ref.path)
            })
            .cloned()
            .collect();
        state.files = files
            .iter()
            .map(|(file_ref, bytes)| ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::SupportingFile,
                    module_ref: None,
                    object_ref: target.clone(),
                    revision: None,
                    file_ref: file_ref.clone(),
                    required: true,
                },
                bytes: bytes.clone(),
            })
            .collect();
        state.files.extend(asset_files);
        state.files.extend(profile_files);
        if let Some(scenario) = &state.source_scenario {
            state.files.push(ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::SupportingFile,
                    module_ref: None,
                    object_ref: ObjectRef {
                        kind: ObjectKind::Scenario,
                        id: ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap(),
                    },
                    revision: Some(scenario.revision.clone()),
                    file_ref: file_ref(
                        &format!("{directory}/source-scenario.json"),
                        DocumentKind::Scenario,
                    ),
                    required: true,
                },
                bytes: encode_document(scenario),
            });
        }
        state.next_document = next_document;
        let package = ModulePackage {
            definition: definition.clone(),
            assets: state.assets.clone(),
            files: state
                .files
                .iter()
                .map(|file| file.entry.file_ref.clone())
                .collect(),
            dependencies: state.dependencies.clone(),
            rights: state.rights.clone(),
            issues: state.issues.clone(),
        };
        Ok((package, state.files.clone()))
    }

    pub(super) fn accept_composition_ordinal(&mut self, state: Option<&CompositionState>) {
        if let Some(state) = state {
            self.next_document = self.next_document.max(state.next_document);
        }
    }
}
