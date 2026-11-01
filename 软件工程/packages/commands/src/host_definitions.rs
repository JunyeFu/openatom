use super::*;

#[derive(Clone)]
pub(super) struct DefinitionDraftState {
    pub definition: ModuleDefinition,
    pub revision: Revision,
    pub professional: ProfessionalDocuments,
    pub dirty: bool,
    pub composition: Option<super::host_template_composition::CompositionState>,
    undo: Vec<DefinitionSnapshot>,
    redo: Vec<DefinitionSnapshot>,
    history_boundary_reason: Option<String>,
}

#[derive(Clone)]
struct DefinitionSnapshot {
    definition: ModuleDefinition,
    composition: Option<super::host_template_composition::CompositionState>,
}

impl DefinitionDraftState {
    pub(super) fn history(&self) -> HistoryAvailability {
        definition_history(
            !self.undo.is_empty(),
            !self.redo.is_empty(),
            self.history_boundary_reason.clone(),
        )
    }

    fn snapshot(&self) -> DefinitionSnapshot {
        DefinitionSnapshot {
            definition: self.definition.clone(),
            composition: self.composition.clone(),
        }
    }
}

// A profile declares concrete site conditions, not an inferred minimum-size or
// geometric-equivalence rule. Layout/production still use their actual checks.
fn module_application_conditions(
    module: &ModuleDefinition,
    profile: Option<&TemplateGenerationProfile>,
    params: &QueryModuleLibraryParams,
) -> (Vec<Issue>, bool) {
    if params.industry.is_none() && params.product_family.is_none() && params.site_envelope.is_none() {
        return (vec![], false); // Unfiltered browsing makes no site-fit claim.
    }
    let mut issues = vec![];
    let mut mismatch = false;
    let issue = |code: &str, path: &str, message: &str| Issue {
        code: format!("module_library.{code}"),
        object_ref: Some(ObjectRef { kind: ObjectKind::Module, id: ObjectId::try_from(module.id.as_str().to_owned()).unwrap() }),
        field_path: Some(path.into()), message: message.into(), blocks: vec![CapabilityPurpose::Adoption],
    };
    let Some(profile) = profile else {
        return (vec![issue("unresolved_application_conditions", "/template_profile", "No actual typed generation profile is available for these requested conditions")], false);
    };
    for (path, requested, declared) in [
        ("/params/industry", params.industry.as_deref(), profile.industry.as_str()),
        ("/params/product_family", params.product_family.as_deref(), profile.product_family.as_str()),
    ] {
        match requested.filter(|value| !value.trim().is_empty()) {
            Some(value) if !declared.trim().is_empty() => {
                if value != declared {
                    mismatch = true;
                    issues.push(issue("application_mismatch", path, "Requested industry/product does not match this exact profile declaration"));
                }
            }
            _ => issues.push(issue("unresolved_application_conditions", path, "Requested or declared industry/product is missing")),
        }
    }
    let Some(site) = params.site_envelope.as_ref() else {
        issues.push(issue("unresolved_application_conditions", "/params/site_envelope", "Explicit site conditions are required for a site applicability conclusion"));
        return (issues, mismatch);
    };
    let declared = &profile.site_applicability;
    for (field, requested, expected) in [
        ("length_m", site.length_m.as_ref(), declared.length_m.as_ref()),
        ("width_m", site.width_m.as_ref(), declared.width_m.as_ref()),
        ("height_m", site.height_m.as_ref(), declared.height_m.as_ref()),
    ] {
        let path = format!("/params/site_envelope/{field}");
        match (requested, expected) {
            (Some(value), Some(expected)) if value.value() > 0.0 && expected.value() > 0.0 => {
                if value.value() != expected.value() {
                    mismatch = true;
                    issues.push(issue("site_condition_mismatch", &path, "Site dimension differs from the profile's concrete supported condition; no size range is inferred"));
                }
                let binding = format!("/site_envelope/{field}");
                for parameter in profile.required_parameters.iter().filter(|p| p.binding == binding) {
                    if parameter.minimum.as_ref().is_some_and(|n| value.value() < n.value())
                        || parameter.maximum.as_ref().is_some_and(|n| value.value() > n.value())
                        || parameter.integer && value.value().fract() != 0.0
                    {
                        mismatch = true;
                        issues.push(issue("parameter_mismatch", &path, "Site value violates the profile's declared parameter bounds"));
                    }
                }
            }
            _ => issues.push(issue("unresolved_application_conditions", &path, "Requested or profile site dimension is missing/nonpositive; Unknown is not a fit")),
        }
    }
    let regions = |site: &SiteEnvelope| site.regions.iter().map(|r| serde_json::json!({"kind":r.kind,"polygon_m":r.polygon_m,"clearance_m":r.clearance_m})).collect::<Vec<_>>();
    if !exact_data(&regions(site), &regions(declared))
        || !exact_data(&site.usable_polygon_m, &declared.usable_polygon_m)
        || !exact_data(&site.clearance_m, &declared.clearance_m)
    {
        issues.push(issue("unresolved_application_conditions", "/params/site_envelope", "Region/polygon/clearance conditions differ from the declared site; no unexecuted geometry adaptation is claimed"));
    }
    if profile.layout_modes.is_empty() {
        issues.push(issue("unresolved_application_conditions", "/template_profile/layout_modes", "No supported layout mode is declared"));
    }
    (issues, mismatch)
}

impl ProjectHost {
    pub(super) fn publish_definition_professional(
        &mut self,
        id: &ModuleId,
        documents: ProfessionalDocuments,
    ) -> Result<(), ApiError> {
        let draft = self
            .definition_drafts
            .get_mut(id)
            .expect("admitted definition completion");
        let revision = next_revision(&draft.revision)?;
        draft.professional = documents;
        draft.revision = revision;
        draft.undo.clear();
        draft.redo.clear();
        draft.history_boundary_reason = Some("Professional input is a history boundary".into());
        draft.dirty = true;
        Ok(())
    }
    pub(super) fn available_definition_commands(&self, caller: &HostCallContext) -> Vec<String> {
        let mut commands = vec![];
        if caller.edit_grants.iter().any(|grant| {
            self.current_grant(grant)
                && grant
                    .branches
                    .iter()
                    .any(|branch| matches!(branch, HostEditBranch::CreateDefinitionDraft))
                && grant.creates.iter().any(|allowance| {
                    matches!(allowance.kind, ObjectKind::DefinitionDraft)
                        && grant
                            .created_objects
                            .iter()
                            .filter(|object| matches!(object.kind, ObjectKind::DefinitionDraft))
                            .map(|object| &object.id)
                            .collect::<BTreeSet<_>>()
                            .len()
                            < allowance.max_new_objects.value() as usize
                        && definition_creation_available(allowance)
                })
        }) {
            commands.push("create_definition_draft".into());
        }
        for (id, draft) in &self.definition_drafts {
            if !caller.allowed_definition_drafts.contains(id) {
                continue;
            }
            let value = serde_json::to_value(&draft.definition).unwrap();
            for (name, branch) in [
                ("put_definition", HostEditBranch::PutDefinition),
                (
                    "commit_definition_version",
                    HostEditBranch::CommitDefinitionVersion,
                ),
                (
                    "set_declared_parameter",
                    HostEditBranch::SetDeclaredParameter,
                ),
            ] {
                let available = caller.edit_grants.iter().any(|grant| matches!(&grant.scope, TaskScope::Edit(EditScope::DefinitionDraft(identity)) if &identity.id == id && identity.revision.0.value() == draft.revision.0.value()) && grant.branches.iter().any(|actual| same(actual, &branch)) && grant.objects.iter().any(|object| matches!(object.target.kind, ObjectKind::DefinitionDraft) && object.target.id.as_str() == id.as_str() && object.fields.iter().any(|field| match branch {
                    HostEditBranch::CommitDefinitionVersion => leaf_granted(field, "/version"),
                    HostEditBranch::SetDeclaredParameter => draft.definition.parameters.iter().any(|parameter| field.pointer == parameter.binding && editable_existing_leaf(field, &value, false)),
                    _ => !matches!(field.pointer.as_str(), "/id" | "/version") && editable_existing_leaf(field, &value, false),
                })));
                if available && !commands.iter().any(|command| command == name) {
                    commands.push(name.into());
                }
            }
        }
        if self.composition_command_available(caller) {
            commands.push("put_template_composition".into());
        }
        commands
    }

    fn bundled_version_conflicts(&self, reference: &ModuleRef) -> bool {
        let key = module_key(reference);
        let (Some(bundle), Some(current)) = (self.bundled_modules.get(&key), self.modules.get(&key)) else {
            return false;
        };
        if !exact_data(current, &bundle.package) {
            return true;
        }
        let files = self.module_source_files(reference);
        // Contribution archives carry the declared source files. The library's
        // original-module.json and generated metadata are not additional source
        // requirements; the typed immutable metadata was compared above.
        bundle.additions.iter().filter(|addition| bundle.package.files.iter().any(|file|
            file.path == addition.entry.file_ref.path)).any(|addition|
            files.get(addition.entry.file_ref.path.as_str()) != Some(&addition.bytes))
    }

    pub(super) fn query_modules(
        &self,
        caller: &HostCallContext,
        _: &ApiScope,
        params: QueryModuleLibraryParams,
    ) -> Result<ModuleTemplateCandidates, ApiError> {
        if matches!(caller.origin, CallerOrigin::ExternalAgent)
            && self
                .project
                .as_ref()
                .is_some_and(|project| caller.granted_project.as_ref() != Some(&project.id))
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Module directory belongs to a different authorized project",
                None,
            ));
        }
        let mut selected = BTreeMap::new();
        if params
            .source_scope
            .iter()
            .any(|scope| matches!(scope, LibrarySourceScope::Bundled))
        {
            selected.extend(
                self.bundled_modules
                    .iter()
                    .map(|(key, module)| (key.clone(), (&module.package, true))),
            );
        }
        if params
            .source_scope
            .iter()
            .any(|scope| matches!(scope, LibrarySourceScope::ProjectPackages))
        {
            selected.extend(
                self.modules
                    .iter()
                    .map(|(key, package)| (key.clone(), (package, false))),
            );
        }
        let mut candidates = vec![];
        let mut query_missing = vec![];
        for (module, bundled) in selected.values() {
            let category = match &module.definition.payload {
                ModulePayload::Items(_) => ApiCategory::Items,
                ModulePayload::Devices(_) => ApiCategory::Devices,
                ModulePayload::Resources(_) => ApiCategory::Resources,
                ModulePayload::Tasks(_) => ApiCategory::Tasks,
                ModulePayload::Operations(_) => ApiCategory::Operations,
                ModulePayload::Transport(_) => ApiCategory::Transport,
                ModulePayload::Templates(_) => ApiCategory::Templates,
                ModulePayload::Professional(_) => ApiCategory::Professional,
                ModulePayload::Media(_) => ApiCategory::Media,
            };
            if !params.category_filters.is_empty()
                && !params
                    .category_filters
                    .iter()
                    .any(|filter| same(filter, &category))
            {
                continue;
            }
            let mut missing = module.issues.clone();
            let reference = ModuleRef { id: module.definition.id.clone(), version: module.definition.version.clone() };
            if self.bundled_version_conflicts(&reference)
            {
                missing.push(Issue {code:"module_library.exact_version_conflict".into(),object_ref:Some(ObjectRef {kind:ObjectKind::Module,id:ObjectId::try_from(module.definition.id.as_str().to_owned()).unwrap()}),field_path:Some("/module_ref".into()),message:"Current Project stores different immutable metadata or declared source bytes for this exact bundled ID/version".into(),blocks:vec![CapabilityPurpose::Adoption]});
            }
            let reference = ModuleRef {
                id: module.definition.id.clone(),
                version: module.definition.version.clone(),
            };
            let files = if *bundled {
                self.bundled_modules[&module_key(&reference)]
                    .additions
                    .iter()
                    .map(|addition| {
                        (
                            addition.entry.file_ref.path.as_str().to_owned(),
                            addition.bytes.clone(),
                        )
                    })
                    .collect()
            } else {
                self.module_source_files(&reference)
            };
            let template_profile = if let ModulePayload::Templates(template) =
                &module.definition.payload
            {
                template
                    .generation_profile
                    .as_ref()
                    .and_then(|path| files.get(path.as_str()))
                    .and_then(|bytes| {
                        serde_json::from_slice::<Versioned<TemplateGenerationProfile>>(bytes).ok()
                    })
                    .map(|versioned| versioned.data)
            } else {
                None
            };
            let (conditions, mismatched) = module_application_conditions(&module.definition, template_profile.as_ref(), &params);
            missing.extend(conditions);
            if mismatched {
                query_missing.extend(missing);
                continue;
            }
            candidates.push(ModuleCandidate {
                module_ref: ModuleRef {
                    id: module.definition.id.clone(),
                    version: module.definition.version.clone(),
                },
                category,
                title: module.definition.title.clone(),
                applicable: !missing.iter().any(|issue| {
                    issue
                        .blocks
                        .iter()
                        .any(|purpose| matches!(purpose, CapabilityPurpose::Adoption))
                }),
                missing,
                capabilities: vec![],
                template_profile,
                sources: vec![],
            });
        }
        if params
                .source_scope
                .iter()
                .any(|scope| matches!(scope, LibrarySourceScope::UserGrantedLibraries))
            {
                query_missing.push(Issue {
                    code: "module_library.user_directory_not_registered".into(),
                    object_ref: None,
                    field_path: Some("/source_scope".into()),
                    message: "No user-granted library has been registered in this Host".into(),
                    blocks: vec![],
                });
            }
        Ok(ModuleTemplateCandidates { candidates, missing: query_missing })
    }

    pub(super) fn template_preview(
        &self,
        scope: &ApiScope,
        target: &ObjectRef,
        reference: &ModuleRef,
        base_revision: &Revision,
        choices: Option<&TemplatePreparationChoices>,
        seed: Option<&TemplateSeedSelection>,
    ) -> Result<TemplatePreview, ApiError> {
        if !matches!(scope, ApiScope::Project)
            || !matches!(target.kind, ObjectKind::Module)
            || target.id.as_str() != reference.id.as_str()
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Template previews require Project scope and matching exact Module identity",
                Some("/params/source"),
            ));
        }
        let project = self.project.as_ref().expect("admitted current Project");
        check_revision(
            base_revision,
            &project.revision,
            "/params/source/data/base_revision",
        )?;
        self.readable_project_module(reference)?;
        if choices.is_none() && seed.is_some() {
            return Err(error(
                ErrorCode::InvalidInput,
                "An explicit seed requires explicit template preparation choices",
                Some("/params/source/data/seed"),
            ));
        }
        let mut sources = self.template_source_candidates(
            reference,
            choices.map(|choices| &choices.source_project),
        )?;
        if sources.len() != 1 {
            return Err(error(
                ErrorCode::InvalidInput,
                "Template preparation must select one actual listed source Project",
                Some("/params/source/data/choices"),
            ));
        }
        let (source_project, source) = sources.pop().unwrap();
        self.preview_template_candidate(
            project,
            reference,
            source_project,
            source,
            &self.module_source_files(reference),
            choices,
            seed,
        )
    }

    pub(super) fn preview_template_candidate(
        &self,
        project: &ProjectState,
        reference: &ModuleRef,
        source_project: RelativeFilePath,
        source: sf_module_kit::TemplateCandidate,
        source_files: &BTreeMap<String, Vec<u8>>,
        choices: Option<&TemplatePreparationChoices>,
        seed: Option<&TemplateSeedSelection>,
    ) -> Result<TemplatePreview, ApiError> {
        let mut issues = source.package.issues.clone();
        let source_scenario = template_seed_source(&source, source_files)?;
        let command = if let Some(choices) = choices {
            let blocked: Vec<_> = source
                .package
                .issues
                .iter()
                .filter(|issue| {
                    issue
                        .blocks
                        .iter()
                        .any(|purpose| matches!(purpose, CapabilityPurpose::Adoption))
                })
                .cloned()
                .collect();
            if !blocked.is_empty() {
                return Err(template_preparation_failure(blocked));
            }
            let kit_choices = sf_module_kit::TemplateChoices {
                namespace: choices.namespace.clone(),
                resources: choices
                    .resources
                    .iter()
                    .map(|(source, target)| {
                        (
                            source.clone(),
                            match target {
                                Some(target) => {
                                    sf_module_kit::TemplateResourceSelection::Reuse(target.clone())
                                }
                                None => {
                                    sf_module_kit::TemplateResourceSelection::CreateDeclaredExample
                                }
                            },
                        )
                    })
                    .collect(),
                reuse_items: choices.reuse_items.clone(),
                include_example_product: choices.include_example_product,
            };
            let mut prepared = sf_module_kit::prepare_template(&source, project, &kit_choices)
                .map_err(template_preparation_failure)?;
            if let Some(site) = &choices.site_envelope {
                sf_module_kit::fill_template_site(&mut prepared, site)
                    .map_err(template_preparation_failure)?;
            }
            let explicit_seed = seed
                .map(|seed| {
                    let context = source_scenario
                        .as_ref()
                        .map(|scenario| {
                            sf_module_kit::template_seed_context_from_scenario(&source, scenario)
                        })
                        .transpose()?;
                    sf_module_kit::prepare_template_seed_with_context(
                        &source,
                        &prepared,
                        &sf_module_kit::TemplateSeedChoices {
                            scenario_id: seed.scenario_id.clone(),
                            supply: seed.supply,
                            demand: seed.demand,
                            initial_quality: seed.initial_quality.clone(),
                            adopt_declared_inspection_results: seed
                                .adopt_declared_inspection_results,
                        },
                        context.as_ref(),
                    )
                })
                .transpose()
                .map_err(template_preparation_failure)?;
            for issue in &prepared.issues {
                if !issues.iter().any(|existing| exact_data(existing, issue)) {
                    issues.push(issue.clone());
                }
            }
            Some(
                serde_json::from_value(prepared.command_data_with_preparation(
                    &source_project,
                    &kit_choices,
                    choices.site_envelope.as_ref(),
                    explicit_seed,
                ))
                .expect("same Commands template shape"),
            )
        } else {
            None
        };
        Ok(TemplatePreview {
            module_ref: reference.clone(),
            project_id: project.id.clone(),
            base_revision: project.revision.clone(),
            subset: source.subset,
            source_project,
            resources: source.example_project.resources,
            command,
            issues,
            source_scenario: source_scenario.as_ref().map(|scenario| ScenarioReference {
                scenario_id: scenario.id.clone(),
                revision: scenario.revision.clone(),
            }),
        })
    }

    pub(super) fn template_source_candidates(
        &self,
        reference: &ModuleRef,
        source_project: Option<&RelativeFilePath>,
    ) -> Result<Vec<(RelativeFilePath, sf_module_kit::TemplateCandidate)>, ApiError> {
        let invalid = |message: &str| error(ErrorCode::InvalidInput, message, Some("/preparation"));
        let source = self.adoption_source(reference).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Exact template source is not loaded",
                Some("/template_ref"),
            )
        })?;
        let ModulePayload::Templates(template) = &source.definition.payload else {
            return Err(invalid("Selected source is not a Templates definition"));
        };
        let files = self.module_source_files(reference);
        let subset_bytes = files.get(template.project.as_str()).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Template subset source file is absent",
                Some("/subset"),
            )
        })?;
        if self.bundled_version_conflicts(reference) {
            return Err(error(ErrorCode::InvalidInput, "Bundled exact version conflicts with the immutable Project package or declared source bytes", Some("/params/source/data/module_ref")));
        }
        let subset: Versioned<TemplateSubset> = serde_json::from_slice(subset_bytes)
            .map_err(|_| invalid("Actual template subset is not a supported typed document"))?;
        let definitions: Vec<_> = subset
            .data
            .module_refs
            .iter()
            .map(|dependency| {
                (if self.modules.contains_key(&module_key(reference)) {
                    self.modules.get(&module_key(dependency))
                } else {
                    self.adoption_source(dependency)
                })
                .map(|package| package.definition.clone())
                .ok_or_else(|| {
                    error(
                        ErrorCode::MissingReference,
                        "Template requires an actual exact loaded definition",
                        Some("/subset/module_refs"),
                    )
                })
            })
            .collect::<Result<_, _>>()?;
        let paths: Vec<_> = if let Some(path) = source_project {
            let path = self.bundled_source_path(reference, path);
            if !source.files.iter().any(|file| file.path == path) {
                return Err(invalid(
                    "Source Project must be declared in the actual selected ModulePackage files",
                ));
            }
            vec![path]
        } else {
            source
                .files
                .iter()
                .filter(|file| {
                    files.get(file.path.as_str()).is_some_and(|bytes| {
                        serde_json::from_slice::<Versioned<ProjectState>>(bytes).is_ok()
                    })
                })
                .map(|file| file.path.clone())
                .collect()
        };
        let mut candidates = vec![];
        for path in paths {
            let bytes = files.get(path.as_str()).ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Selected template source Project file is absent",
                    Some("/preparation/source_project"),
                )
            })?;
            match sf_module_kit::decode_template_candidate(
                &encode_document(source),
                subset_bytes,
                bytes,
                &serde_json::to_vec(&definitions).unwrap(),
            ) {
                Ok(candidate) => candidates.push((path, candidate)),
                Err(issues) if source_project.is_some() => {
                    return Err(invalid(
                        &issues
                            .iter()
                            .map(|issue| issue.message.as_str())
                            .collect::<Vec<_>>()
                            .join("; "),
                    ));
                }
                Err(_) => {}
            }
        }
        Ok(candidates)
    }

    pub(super) fn prepare_template_instantiation(
        &self,
        target: &ProjectState,
        request: &TemplateInstantiation,
    ) -> Result<sf_module_kit::PreparedTemplate, ApiError> {
        let invalid = |message: &str| error(ErrorCode::InvalidInput, message, Some("/preparation"));
        let sources = self.template_source_candidates(
            &request.template_ref,
            request
                .preparation
                .as_ref()
                .map(|choices| &choices.source_project),
        )?;
        let source = self
            .adoption_source(&request.template_ref)
            .expect("resolved template source");
        let requested_subset =
            super::host_library::relocate_template_subset(source, &request.subset);
        if sources
            .iter()
            .any(|(_, candidate)| !exact_data(&candidate.subset, &requested_subset))
        {
            return Err(error(
                ErrorCode::MissingReference,
                "Template subset must match actual selected source archive bytes",
                Some("/subset"),
            ));
        }
        let mut accepted = vec![];
        let files = self.module_source_files(&request.template_ref);
        for (_, candidate) in sources {
            let blocked: Vec<_> = candidate
                .package
                .issues
                .iter()
                .filter(|issue| {
                    issue
                        .blocks
                        .iter()
                        .any(|purpose| matches!(purpose, CapabilityPurpose::Adoption))
                })
                .cloned()
                .collect();
            if !blocked.is_empty() {
                return Err(template_preparation_failure(blocked));
            }
            let choices = if let Some(choices) = &request.preparation {
                vec![sf_module_kit::TemplateChoices {
                    namespace: choices.namespace.clone(),
                    resources: choices.resources.iter().map(|(source, target)| (source.clone(), match target {
                        Some(target) => sf_module_kit::TemplateResourceSelection::Reuse(target.clone()),
                        None => sf_module_kit::TemplateResourceSelection::CreateDeclaredExample,
                    })).collect(),
                    reuse_items: choices.reuse_items.clone(),
                    include_example_product: choices.include_example_product,
                }]
            } else {
                inferred_template_choices(&candidate, target, request)
            };
            for choices in choices {
                let Ok(mut prepared) =
                    sf_module_kit::prepare_template(&candidate, target, &choices)
                else {
                    continue;
                };
                if let Some(site) = request
                    .preparation
                    .as_ref()
                    .and_then(|choices| choices.site_envelope.as_ref())
                {
                    sf_module_kit::fill_template_site(&mut prepared, site).map_err(|issues| {
                        invalid(
                            &issues
                                .iter()
                                .map(|issue| issue.message.as_str())
                                .collect::<Vec<_>>()
                                .join("; "),
                        )
                    })?;
                }
                let replay = prepared.command_data(None);
                let mut actual_objects: Vec<_> = request
                    .proposed_objects
                    .iter()
                    .map(|object| {
                        serde_json::to_string(&serde_json::to_value(object).unwrap()).unwrap()
                    })
                    .collect();
                let mut expected_objects: Vec<_> = replay["proposed_objects"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|object| serde_json::to_string(object).unwrap())
                    .collect();
                actual_objects.sort();
                expected_objects.sort();
                let requested_policies = request
                    .project_setup_policies
                    .as_ref()
                    .unwrap_or(&target.setup_policies);
                let seed_matches = if let Some(scenario) = &request.explicit_new_scenario_seed {
                    let source_scenario = template_seed_source(&candidate, &files)?;
                    let context = source_scenario
                        .as_ref()
                        .map(|source| {
                            sf_module_kit::template_seed_context_from_scenario(&candidate, source)
                        })
                        .transpose()
                        .map_err(template_preparation_failure)?;
                    template_seed_replays(&candidate, &prepared, scenario, context.as_ref())
                } else {
                    true
                };
                if actual_objects == expected_objects
                    && exact_data(&prepared.mapped_bindings, &request.mapped_bindings)
                    && exact_data(
                        requested_policies,
                        prepared
                            .project_setup_policies
                            .as_ref()
                            .unwrap_or(&target.setup_policies),
                    )
                    && seed_matches
                {
                    accepted.push(prepared);
                }
            }
        }
        if accepted.len() != 1 {
            return Err(invalid(
                "Template objects and bindings must match one complete replay of actual source plus explicit new-ID/resource choices",
            ));
        }
        Ok(accepted.pop().unwrap())
    }

    pub(super) fn apply_adoption_candidate(
        &self,
        ledger: &mut HostCallContext,
        candidate: &mut ProjectDraft,
        branch: &HostEditBranch,
        adoption: ModuleAdoption,
    ) -> Result<Vec<ObjectRef>, ApiError> {
        self.authorize_branch(ledger, branch)?;
        if matches!(ledger.origin, CallerOrigin::ExternalAgent)
            && !ledger.edit_grants.iter().any(|grant| {
                self.current_grant(grant)
                    && grant.branches.iter().any(|actual| same(actual, branch))
                    && grant
                        .source_versions
                        .iter()
                        .any(|reference| module_key(reference) == module_key(&adoption.module_ref))
            })
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Exact adoption source version was not granted",
                None,
            ));
        }
        // Source/reference errors cannot conceal a later unauthorized target in
        // this same adoption command. Creation uses only the disposable ledger.
        let mut deferred = None;
        for target in &adoption.targets {
            effect_admitted(
                self.authorize_update(ledger, branch, target, &Value::Null, &Value::Null),
                &mut deferred,
            )?;
        }
        for object in &adoption.new_instances {
            let target = editable_target(object).unwrap();
            effect_admitted(
                self.authorize_create_as(
                    ledger,
                    &target,
                    &authorization_value(&target, editable_object_value(object)),
                    branch,
                ),
                &mut deferred,
            )?;
        }
        if let Some(scenario) = &adoption.explicit_seed {
            let target = ObjectRef {
                kind: ObjectKind::Scenario,
                id: ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap(),
            };
            effect_admitted(
                self.authorize_create_as(
                    ledger,
                    &target,
                    &authorization_value(&target, serde_json::to_value(scenario).unwrap()),
                    branch,
                ),
                &mut deferred,
            )?;
        }
        if let Some(failure) = deferred {
            return Err(failure);
        }
        let source = self.adoption_source(&adoption.module_ref).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Exact adoption module is not loaded",
                Some("/module_ref"),
            )
        })?;
        if matches!(ledger.origin, CallerOrigin::ExternalAgent) {
            for resource in adoption.bindings.resources.values().filter(|id| {
                candidate
                    .project
                    .resources
                    .iter()
                    .any(|resource| &resource.id == *id)
            }) {
                if !ledger.edit_grants.iter().any(|grant| {
                    self.current_grant(grant)
                        && grant.branches.iter().any(|actual| same(actual, branch))
                        && grant.reusable_resources.contains(resource)
                }) {
                    return Err(error(
                        ErrorCode::PermissionDenied,
                        "Existing resource reuse was not granted",
                        None,
                    ));
                }
            }
        }
        let mut changed = vec![];
        for target in adoption.targets {
            let old = draft_value(candidate, &target)
                .ok_or_else(|| missing(&target, "Adoption target does not exist"))?;
            let mut updated = old.clone();
            match (&source.definition.payload, &target.kind) {
                (ModulePayload::Devices(_), ObjectKind::Device) => {
                    updated["module_ref"] = serde_json::to_value(&adoption.module_ref).unwrap();
                    updated["role_bindings"] = serde_json::to_value(&adoption.bindings).unwrap();
                }
                (ModulePayload::Operations(_), ObjectKind::Device) => {
                    updated["operation_binding"] =
                        serde_json::to_value(&adoption.module_ref).unwrap();
                    // Operations adds its reviewed roles to this device; it does
                    // not replace capacity, port or other device-level bindings.
                    let mappings = serde_json::to_value(&adoption.bindings).unwrap();
                    for section in ["item_types", "resources", "targets", "ports"] {
                        updated["role_bindings"][section]
                            .as_object_mut()
                            .unwrap()
                            .extend(mappings[section].as_object().unwrap().clone());
                    }
                }
                (ModulePayload::Resources(_), ObjectKind::Resource) => {
                    updated["definition_ref"] = serde_json::to_value(&adoption.module_ref).unwrap();
                }
                (ModulePayload::Items(definition), ObjectKind::ItemType) => {
                    updated["module_ref"] = serde_json::to_value(&adoption.module_ref).unwrap();
                    if let Some(unit) = &definition.unit {
                        updated["unit"] = serde_json::to_value(unit).unwrap();
                    }
                    if let Some(tracking) = &definition.tracking {
                        updated["tracking"] = serde_json::to_value(tracking).unwrap();
                    }
                    updated["spec_ref"] = serde_json::to_value(&definition.spec_ref).unwrap();
                    updated["attributes"] = serde_json::to_value(&definition.attributes).unwrap();
                    updated["attribute_schema"] =
                        serde_json::to_value(&definition.attribute_schema).unwrap();
                }
                (ModulePayload::Media(presentation), ObjectKind::Device) => {
                    // Resolve the whole exact Media package before selecting its
                    // LOD/assets. The Device package does not own those bytes.
                    let reference = MediaLocator::MediaRef(adoption.module_ref.clone());
                    let mut linked = presentation.clone();
                    linked.source = reference.clone();
                    linked.lod.detailed = Some(reference.clone());
                    linked.lod.simplified = Some(reference);
                    linked.nodes.clear();
                    linked.poses.clear();
                    linked.actions.clear();
                    linked.timeline.clear();
                    linked.tags.clear();
                    linked.anchor_roles.clear();
                    linked.port_anchors.clear();
                    updated["presentation_override"] = serde_json::to_value(linked).unwrap();
                }
                _ => {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Selected module category cannot bind this explicit target kind",
                        Some("/targets"),
                    ));
                }
            }
            self.authorize_update(ledger, branch, &target, &old, &updated)?;
            let kind = serde_json::to_value(&target.kind).unwrap();
            let object = serde_json::from_value(serde_json::json!({"kind":kind,"data":updated}))
                .map_err(|failure| {
                    error(
                        ErrorCode::InvalidInput,
                        &format!("Adoption instance violates its typed object: {failure}"),
                        Some("/targets"),
                    )
                })?;
            put_draft_object(candidate, object);
            push_ref(&mut changed, target);
        }
        for mut object in adoption.new_instances {
            let target = editable_target(&object).unwrap();
            if draft_value(candidate, &target).is_some() {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "New adopted instances must have distinct new stable IDs",
                    Some("/new_instances"),
                ));
            }
            match (&source.definition.payload, &mut object) {
                (ModulePayload::Devices(_), EditableProjectObject::Device(device))
                    if module_key(&device.module_ref) != module_key(&adoption.module_ref) =>
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Adopted Device must reference the selected exact definition",
                        Some("/new_instances"),
                    ));
                }
                (ModulePayload::Resources(_), EditableProjectObject::Resource(resource))
                    if module_key(&resource.definition_ref) != module_key(&adoption.module_ref) =>
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Adopted Resource must reference the selected exact definition",
                        Some("/new_instances"),
                    ));
                }
                (ModulePayload::Items(definition), EditableProjectObject::ItemType(item)) => {
                    item.module_ref = Some(adoption.module_ref.clone());
                    if let Some(unit) = &definition.unit {
                        item.unit = unit.clone();
                    }
                    if let Some(tracking) = &definition.tracking {
                        item.tracking = tracking.clone();
                    }
                    item.spec_ref = definition.spec_ref.clone();
                    item.attributes = definition.attributes.clone();
                    item.attribute_schema = definition.attribute_schema.clone();
                }
                _ => {}
            }
            self.authorize_create_as(
                ledger,
                &target,
                &authorization_value(&target, editable_object_value(&object)),
                branch,
            )?;
            put_draft_object(candidate, object);
            push_ref(&mut changed, target);
        }
        if let Some(scenario) = adoption.explicit_seed {
            let target = ObjectRef {
                kind: ObjectKind::Scenario,
                id: ObjectId::try_from(scenario.id.as_str().to_owned()).unwrap(),
            };
            if candidate.scenarios.contains_key(&scenario.id)
                || scenario.revision.0.value() != 1
                || scenario.project_revision.0.value() != candidate.project.revision.0.value()
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "Explicit adoption seed must name a new Scenario ID at revision1 and current Project base",
                    Some("/explicit_seed"),
                ));
            }
            self.authorize_create_as(
                ledger,
                &target,
                &authorization_value(&target, serde_json::to_value(&scenario).unwrap()),
                branch,
            )?;
            put_draft_object(
                candidate,
                EditableProjectObject::Scenario(Box::new(scenario)),
            );
            push_ref(&mut changed, target);
        }
        if !candidate
            .project
            .module_refs
            .iter()
            .any(|reference| module_key(reference) == module_key(&adoption.module_ref))
        {
            candidate.project.module_refs.push(adoption.module_ref);
        }
        Ok(changed)
    }

    pub(super) fn create_definition(
        &mut self,
        caller: &HostCallContext,
        definition: ModuleDefinition,
    ) -> Result<EditCommit, ApiError> {
        validate_definition(&definition)?;
        let id = ModuleId::try_from(format!(
            "{}:draft:{}",
            self.host_instance_id.as_str(),
            self.next_definition_draft
        ))
        .unwrap();
        let target = ObjectRef {
            kind: ObjectKind::DefinitionDraft,
            id: ObjectId::try_from(id.as_str().to_owned()).unwrap(),
        };
        let mut ledger = caller.clone();
        self.authorize_create_as(
            &mut ledger,
            &target,
            &serde_json::to_value(&definition).unwrap(),
            &HostEditBranch::CreateDefinitionDraft,
        )?;
        let revision = Revision(SafeUint::try_from(1).unwrap());
        self.definition_drafts.insert(
            id.clone(),
            DefinitionDraftState {
                definition,
                revision: revision.clone(),
                professional: ProfessionalDocuments::default(),
                dirty: true,
                composition: None,
                undo: vec![],
                redo: vec![],
                history_boundary_reason: None,
            },
        );
        if matches!(ledger.origin, CallerOrigin::ExternalAgent)
            && !ledger.allowed_definition_drafts.contains(&id)
        {
            ledger.allowed_definition_drafts.push(id.clone());
        }
        self.sessions.insert(ledger.session_id.clone(), ledger);
        self.next_definition_draft += 1;
        Ok(EditCommit {
            project_id: self.project.as_ref().unwrap().id.clone(),
            scope: EditScope::DefinitionDraft(DefinitionScopeIdentity {
                id,
                revision: revision.clone(),
            }),
            previous_revision: None,
            new_revision: revision,
            changed_objects: vec![target],
            issues: vec![],
            history: definition_history(false, false, None),
        })
    }

    pub(super) fn definition_fields_authorized(
        &self,
        caller: &HostCallContext,
        id: &ModuleId,
        branch: &HostEditBranch,
        target: &ObjectRef,
        old: &Value,
        new: &Value,
    ) -> Result<(), ApiError> {
        if matches!(caller.origin, CallerOrigin::InteractiveUi) {
            return Ok(());
        }
        let grants: Vec<_> = caller.edit_grants.iter().filter(|grant| matches!(&grant.scope, TaskScope::Edit(EditScope::DefinitionDraft(actual)) if &actual.id == id && self.definition_drafts.get(id).is_some_and(|draft| draft.revision.0.value() == actual.revision.0.value())) && grant.branches.iter().any(|actual| same(actual, branch))).collect();
        if !grants.iter().any(|grant| {
            grant
                .objects
                .iter()
                .any(|object| same_ref(&object.target, target))
        }) {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Definition branch/target is outside the current draft authority",
                None,
            ));
        }
        for (pointer, element) in changed_fields(old, new)? {
            if !grants.iter().any(|grant| {
                grant.objects.iter().any(|object| {
                    same_ref(&object.target, target)
                        && object
                            .fields
                            .iter()
                            .any(|field| field_granted(field, &pointer, element.as_deref(), old))
                })
            }) {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Definition field is not authorized",
                    None,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn apply_definition(
        &mut self,
        caller: &HostCallContext,
        id: &ModuleId,
        params: ApplyCommandsParams,
    ) -> Result<EditCommit, ApiError> {
        let before = self
            .definition_drafts
            .get(id)
            .expect("admitted draft")
            .clone();
        let base = params.base_revision.as_ref().ok_or_else(|| {
            error(
                ErrorCode::InvalidInput,
                "Definition edits require their draft base_revision",
                Some("/params/base_revision"),
            )
        })?;
        check_revision(base, &before.revision, "/params/base_revision")?;
        if params.commands.is_empty() {
            return Err(error(
                ErrorCode::InvalidInput,
                "Definition command batch is empty",
                Some("/params/commands"),
            ));
        }
        let target = ObjectRef {
            kind: ObjectKind::DefinitionDraft,
            id: ObjectId::try_from(id.as_str().to_owned()).unwrap(),
        };
        let mut candidate = before.clone();
        let mut freeze = false;
        let mut history_action = None;
        let mut deferred = None;
        for command in &params.commands {
            match command {
                EditCommand::PutTemplateComposition(composition) => {
                    match self.prepare_composition(caller, id, &candidate, composition) {
                        Ok((definition, state)) => {
                            candidate.definition = definition;
                            candidate.composition = Some(state);
                        }
                        Err(failure) => {
                            effect_admitted(Err(failure), &mut deferred)?;
                        }
                    }
                }
                EditCommand::PutDefinition(definition) => {
                    effect_admitted(
                        self.definition_fields_authorized(
                            caller,
                            id,
                            &HostEditBranch::PutDefinition,
                            &target,
                            &serde_json::to_value(&candidate.definition).unwrap(),
                            &serde_json::to_value(definition).unwrap(),
                        ),
                        &mut deferred,
                    )?;
                    if definition.id != before.definition.id
                        || definition.version != before.definition.version
                    {
                        deferred.get_or_insert_with(|| error(ErrorCode::InvalidInput, "PutDefinition preserves module identity/version; CommitDefinitionVersion chooses a new frozen version", Some("/params/commands")));
                    }
                    candidate.definition = definition.clone();
                }
                EditCommand::SetDeclaredParameter {
                    target: selected,
                    parameter_id,
                    value,
                    basis,
                } => {
                    if !same_ref(selected, &target) {
                        return Err(error(
                            ErrorCode::PermissionDenied,
                            "Parameter target is outside this draft",
                            None,
                        ));
                    }
                    match definition_parameter(&candidate.definition, parameter_id, value, basis) {
                        Ok(definition) => {
                            let mut authorized_effect = definition.clone();
                            for parameter in &mut authorized_effect.parameters {
                                if let Some(old) = candidate
                                    .definition
                                    .parameters
                                    .iter()
                                    .find(|old| old.id == parameter.id)
                                {
                                    parameter.basis = old.basis.clone();
                                }
                            }
                            effect_admitted(
                                self.definition_fields_authorized(
                                    caller,
                                    id,
                                    &HostEditBranch::SetDeclaredParameter,
                                    &target,
                                    &serde_json::to_value(&candidate.definition).unwrap(),
                                    &serde_json::to_value(&authorized_effect).unwrap(),
                                ),
                                &mut deferred,
                            )?;
                            candidate.definition = definition;
                        }
                        Err(failure) => {
                            self.definition_fields_authorized(
                                caller,
                                id,
                                &HostEditBranch::SetDeclaredParameter,
                                &target,
                                &Value::Null,
                                &Value::Null,
                            )?;
                            effect_admitted(Err(failure), &mut deferred)?;
                        }
                    }
                }
                EditCommand::CommitDefinitionVersion { version } => {
                    if params.commands.len() != 1 {
                        deferred.get_or_insert_with(|| {
                            error(
                                ErrorCode::InvalidInput,
                                "Definition version freeze is a separate transaction",
                                Some("/params/commands"),
                            )
                        });
                    }
                    let mut definition = candidate.definition.clone();
                    definition.version = version.clone();
                    effect_admitted(
                        self.definition_fields_authorized(
                            caller,
                            id,
                            &HostEditBranch::CommitDefinitionVersion,
                            &target,
                            &serde_json::to_value(&candidate.definition).unwrap(),
                            &serde_json::to_value(&definition).unwrap(),
                        ),
                        &mut deferred,
                    )?;
                    candidate.definition = definition;
                    freeze = true;
                }
                EditCommand::Undo | EditCommand::Redo => {
                    if params.commands.len() != 1 {
                        deferred.get_or_insert_with(|| {
                            unsupported("Definition Undo/Redo must be a single command")
                        });
                        continue;
                    }
                    let undo = matches!(command, EditCommand::Undo);
                    let value = if undo {
                        candidate.undo.last()
                    } else {
                        candidate.redo.last()
                    }
                    .ok_or_else(|| {
                        error(
                            ErrorCode::InvalidInput,
                            "No definition history candidate",
                            None,
                        )
                    })?
                    .clone();
                    self.definition_fields_authorized(
                        caller,
                        id,
                        if undo {
                            &HostEditBranch::Undo
                        } else {
                            &HostEditBranch::Redo
                        },
                        &target,
                        &serde_json::to_value(&candidate.definition).unwrap(),
                        &serde_json::to_value(&value.definition).unwrap(),
                    )?;
                    self.authorize_composition_history(
                        caller,
                        id,
                        &target,
                        &candidate,
                        &value.composition,
                        undo,
                    )?;
                    candidate.definition = value.definition;
                    candidate.composition = value.composition;
                    history_action = Some(undo);
                }
                EditCommand::PutPackagingJob(job) => {
                    if !matches!(caller.origin, CallerOrigin::InteractiveUi) {
                        return Err(error(
                            ErrorCode::PermissionDenied,
                            "Definition professional inputs require explicit draft job authority",
                            None,
                        ));
                    }
                    let mut draft = self.current_draft();
                    draft.professional = candidate.professional.clone();
                    self.check_packaging_job_reservation(
                        &ApiScope::DefinitionDraft(id.clone()),
                        job,
                    )?;
                    put_packaging_job(&mut draft, *job.clone())?;
                    candidate.professional = draft.professional;
                    freeze = true;
                }
                other => {
                    self.definition_fields_authorized(
                        caller,
                        id,
                        &branch(other),
                        &target,
                        &Value::Null,
                        &Value::Null,
                    )?;
                    deferred.get_or_insert_with(|| {
                        unsupported("Command has no definition-draft effect")
                    });
                }
            }
        }
        if let Some(failure) = deferred {
            return Err(failure);
        }
        if let Some(state) = &candidate.composition {
            if !matches!(&candidate.definition.payload,ModulePayload::Templates(template) if state.files.iter().any(|file|matches!(file.entry.file_ref.kind,DocumentKind::Template) && file.entry.file_ref.path==template.project))
            {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "PutDefinition cannot detach an authored composition from its actual sidecar; use PutTemplateComposition or a new definition draft",
                    Some("/payload"),
                ));
            }
        }
        validate_definition(&candidate.definition)?;
        let mut module = None;
        let mut composition_files = vec![];
        if freeze
            && matches!(
                &params.commands[0],
                EditCommand::CommitDefinitionVersion { .. }
            )
        {
            if candidate.composition.is_some() {
                self.composition_project_read(caller)?;
            }
            let reference = ModuleRef {
                id: candidate.definition.id.clone(),
                version: candidate.definition.version.clone(),
            };
            let unchanged_composition = self
                .modules
                .get(&module_key(&reference))
                .filter(|existing| {
                    exact_data(&existing.definition, &candidate.definition)
                        && candidate.composition.as_ref().is_some_and(|state| {
                            state.files.iter().all(|file| {
                                self.additions
                                    .iter()
                                    .find(|actual| {
                                        actual.entry.file_ref.path == file.entry.file_ref.path
                                    })
                                    .map(|actual| {
                                        actual.bytes == file.bytes
                                            && exact_data(&actual.entry, &file.entry)
                                    })
                                    .or_else(|| {
                                        self.backing.as_ref().map(|archive| {
                                            archive.files.get(file.entry.file_ref.path.as_str())
                                                == Some(&file.bytes)
                                                && archive
                                                    .manifest
                                                    .documents
                                                    .iter()
                                                    .any(|entry| exact_data(entry, &file.entry))
                                        })
                                    })
                                    .unwrap_or(false)
                            })
                        })
                })
                .cloned();
            let package = if let Some(package) = unchanged_composition {
                package
            } else if let Some(state) = &mut candidate.composition {
                let (package, files) =
                    self.composition_package(&mut candidate.definition, state)?;
                composition_files = files;
                package
            } else {
                definition_package(&candidate.definition)?
            };
            if let Some(existing) = self.modules.get(&module_key(&reference)) {
                if serde_json::to_value(existing).unwrap()
                    != serde_json::to_value(&package).unwrap()
                {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "An existing exact module version is immutable",
                        Some("/params/commands/0/data/version"),
                    ));
                }
            } else {
                module = Some((reference, package));
            }
        }
        candidate.revision = next_revision(&before.revision)?;
        candidate.dirty = !freeze
            || !matches!(
                &params.commands[0],
                EditCommand::CommitDefinitionVersion { .. }
            );
        if freeze {
            candidate.undo.clear();
            candidate.redo.clear();
        } else if let Some(undo) = history_action {
            if undo {
                candidate.undo.pop();
                candidate.redo.push(before.snapshot());
            } else {
                candidate.redo.pop();
                candidate.undo.push(before.snapshot());
            }
        } else {
            candidate.undo.push(before.snapshot());
            candidate.redo.clear();
        }
        candidate.history_boundary_reason = freeze.then(|| {
            "A committed immutable version/professional input is a history boundary".into()
        });
        let history = candidate.history();
        let result = EditCommit {
            project_id: self.project.as_ref().unwrap().id.clone(),
            scope: EditScope::DefinitionDraft(DefinitionScopeIdentity {
                id: id.clone(),
                revision: candidate.revision.clone(),
            }),
            previous_revision: Some(before.revision),
            new_revision: candidate.revision.clone(),
            changed_objects: vec![target],
            issues: candidate
                .composition
                .as_ref()
                .map(|state| state.issues.clone())
                .unwrap_or_default(),
            history,
        };
        if let Some((reference, package)) = module {
            let fresh: Vec<_> = composition_files
                .into_iter()
                .filter(|addition| {
                    !self.additions.iter().any(|existing| {
                        existing.entry.file_ref.path == addition.entry.file_ref.path
                    }) && !self.backing.as_ref().is_some_and(|archive| {
                        archive
                            .files
                            .contains_key(addition.entry.file_ref.path.as_str())
                    })
                })
                .collect();
            self.additions.extend(fresh);
            let path = format!("{}/module.json", self.next_document_directory("module"));
            self.additions.push(ArchiveAddition {
                entry: DocumentEntry {
                    role: DocumentRole::ModuleDefinition,
                    module_ref: Some(reference.clone()),
                    object_ref: ObjectRef {
                        kind: ObjectKind::Module,
                        id: ObjectId::try_from(reference.id.as_str().to_owned()).unwrap(),
                    },
                    revision: None,
                    file_ref: file_ref(&path, DocumentKind::Module),
                    required: true,
                },
                bytes: encode_document(&package),
            });
            self.modules.insert(module_key(&reference), package);
            self.content_generation += 1;
            self.dirty = true;
        }
        self.accept_composition_ordinal(candidate.composition.as_ref());
        self.definition_drafts.insert(id.clone(), candidate);
        Ok(result)
    }
}

fn template_preparation_failure(issues: Vec<Issue>) -> ApiError {
    let mut failure = engine_failure(issues);
    failure.message = "Template choices could not be prepared".into();
    failure
}

fn template_seed_source(
    source: &sf_module_kit::TemplateCandidate,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<Option<ScenarioInput>, ApiError> {
    let Some(seed) = &source.subset.seed else {
        return Ok(None);
    };
    let mut matches = vec![];
    for file in source
        .package
        .files
        .iter()
        .filter(|file| matches!(file.kind, DocumentKind::Scenario))
    {
        let bytes = files.get(file.path.as_str()).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Declared template source Scenario file is absent",
                Some("/source_scenario"),
            )
        })?;
        let scenario: Versioned<ScenarioInput> = serde_json::from_slice(bytes).map_err(|_| {
            error(
                ErrorCode::InvalidInput,
                "Template source Scenario is not a canonical typed document",
                Some("/source_scenario"),
            )
        })?;
        if exact_data(
            &scenario.data.project_revision,
            &source.example_project.revision,
        ) && exact_data(&scenario.data.initial_conditions, &seed.initial_conditions)
            && exact_data(&scenario.data.orders, &seed.orders)
            && exact_data(&scenario.data.arrivals, &seed.arrivals)
        {
            matches.push(scenario.data);
        }
    }
    if matches.len() != 1 {
        return Err(error(
            ErrorCode::MissingReference,
            "template.seed_context_ambiguous: Select exactly one actual matching Scenario context listed by this exact template version",
            Some("/source_scenario"),
        ));
    }
    Ok(matches.pop())
}

fn template_seed_replays(
    candidate: &sf_module_kit::TemplateCandidate,
    prepared: &sf_module_kit::PreparedTemplate,
    requested: &ScenarioInput,
    context: Option<&sf_module_kit::TemplateSeedContext>,
) -> bool {
    let actual_items: Vec<_> = requested
        .initial_conditions
        .items
        .iter()
        .chain(requested.arrivals.iter().flat_map(|arrival| &arrival.items))
        .collect();
    let qualities: BTreeMap<ObjectId, QualityState> = if let Some(seed) = &candidate.subset.seed {
        seed.initial_conditions
            .items
            .iter()
            .chain(seed.arrivals.iter().flat_map(|arrival| &arrival.items))
            .zip(&actual_items)
            .map(|(source, actual)| (source.id.clone(), actual.quality.clone()))
            .collect()
    } else {
        candidate
            .subset
            .adoption
            .as_ref()
            .and_then(|adoption| adoption.deferred_seed.as_ref())
            .map(|seed| {
                seed.initial_stock
                    .iter()
                    .zip(&actual_items)
                    .map(|(source, actual)| (source.location_id.clone(), actual.quality.clone()))
                    .collect()
            })
            .unwrap_or_default()
    };
    // The existing command carries the full proposed Scenario. Replay the finite
    // explicit switches and its item qualities with the same SDK producer.
    for supply in [false, true] {
        for demand in [false, true] {
            for inspection in [false, true] {
                let choices = sf_module_kit::TemplateSeedChoices {
                    scenario_id: requested.id.clone(),
                    supply,
                    demand,
                    initial_quality: qualities.clone(),
                    adopt_declared_inspection_results: inspection,
                };
                if sf_module_kit::prepare_template_seed_with_context(
                    candidate, prepared, &choices, context,
                )
                .is_ok_and(|expected| exact_data(&expected, requested))
                {
                    return true;
                }
            }
        }
    }
    false
}

fn inferred_template_choices(
    source: &sf_module_kit::TemplateCandidate,
    target: &ProjectState,
    request: &TemplateInstantiation,
) -> Vec<sf_module_kit::TemplateChoices> {
    let Some(first) = request.proposed_objects.first().and_then(editable_target) else {
        return vec![];
    };
    let source_draft = ProjectDraft {
        project: source.example_project.clone(),
        scenarios: BTreeMap::new(),
        professional: ProfessionalDocuments::default(),
    };
    let namespaces: BTreeSet<_> = draft_objects(&source_draft)
        .into_iter()
        .filter(|(object, _)| same(&object.kind, &first.kind))
        .filter_map(|(object, _)| {
            first
                .id
                .as_str()
                .strip_suffix(&format!(":{}", object.id.as_str()))
                .map(str::to_owned)
        })
        .filter(|namespace| !namespace.is_empty())
        .collect();
    namespaces.into_iter().map(|namespace| {
        let resources = source.example_project.resources.iter().filter_map(|resource| {
            let role = RoleId::try_from(format!("template.resource.{}", resource.id.as_str())).ok()?;
            let id = request.mapped_bindings.resources.get(&role)?;
            if request.proposed_objects.iter().any(|object| matches!(object, EditableProjectObject::Resource(proposed) if &proposed.id == id)) {
                Some((resource.id.clone(), sf_module_kit::TemplateResourceSelection::CreateDeclaredExample))
            } else if target.resources.iter().any(|resource| &resource.id == id) {
                Some((resource.id.clone(), sf_module_kit::TemplateResourceSelection::Reuse(id.clone())))
            } else { None }
        }).collect();
        let reuse_items = source.subset.suggested_role_bindings.item_types.iter().filter_map(|(role, old)| {
            let id = request.mapped_bindings.item_types.get(role)?;
            target.item_types.iter().any(|item| &item.id == id).then(|| (old.clone(), id.clone()))
        }).collect();
        sf_module_kit::TemplateChoices {
            namespace,
            resources,
            reuse_items,
            include_example_product: request.proposed_objects.iter().any(|object| matches!(object, EditableProjectObject::Product(_))),
        }
    }).collect()
}

fn definition_history(
    can_undo: bool,
    can_redo: bool,
    boundary_reason: Option<String>,
) -> HistoryAvailability {
    HistoryAvailability {
        can_undo,
        can_redo,
        boundary_reason,
    }
}

fn definition_creation_available(allowance: &HostCreateAllowance) -> bool {
    let has = |pointer: &str| {
        allowance
            .fields
            .iter()
            .any(|field| leaf_granted(field, pointer))
    };
    if ![
        "/version",
        "/title",
        "/definition_state",
        "/contribution/author",
        "/contribution/license",
        "/payload/category",
    ]
    .into_iter()
    .all(has)
    {
        return false;
    }
    let options: &[&[&str]] = &[
        &["presentation_or_media_ref", "unit", "tracking", "spec_ref"],
        &[
            "placement",
            "behavior/inline",
            "behavior/operation_ref",
            "behavior/selected",
            "presentation",
        ],
        &["kind", "role", "capacity", "presentation"],
        &[
            "kind",
            "target_role",
            "duration",
            "equipment_access",
            "completion_effect/target_role",
            "completion_effect/marker",
            "completion_effect/produces_material",
        ],
        &[
            "builtin",
            "duration",
            "quality_and_routes",
            "identity_policy",
            "family",
        ],
        &[
            "builtin",
            "batch_capacity",
            "times/loading",
            "times/outbound",
            "times/unloading",
            "times/returning",
            "identity_policy",
        ],
        &["project", "generation_profile"],
        &["output_artifact_type", "unit", "coordinate_frame", "scope"],
        &[
            "kind",
            "source/kind",
            "source/data",
            "unit",
            "coordinate_frame",
            "scale/x",
            "scale/y",
            "scale/z",
            "lod/detailed",
            "lod/simplified",
            "lod/region",
            "production_effect",
        ],
    ];
    options.iter().any(|fields| {
        fields
            .iter()
            .all(|suffix| has(&format!("/payload/data/{suffix}")))
    })
}

pub(super) fn validate_definition(definition: &ModuleDefinition) -> Result<(), ApiError> {
    let mut ids = BTreeSet::new();
    for parameter in &definition.parameters {
        if !ids.insert(&parameter.id) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Duplicate declared parameter ID",
                Some("/parameters"),
            ));
        }
        if !parameter.binding.starts_with("/payload/data/")
            || parameter.binding.contains('*')
            || parameter.binding.split('/').any(|part| {
                matches!(
                    part,
                    "id" | "version"
                        | "definition_state"
                        | "capabilities"
                        | "acknowledgments"
                        | "stock"
                        | "clock"
                )
            })
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Declared parameter must name a payload leaf without identity/capability/runtime authority",
                Some("/parameters"),
            ));
        }
        let json = serde_json::to_value(definition).unwrap();
        if json.pointer(&parameter.binding).is_none_or(|value| {
            value.is_array()
                || (value.is_object()
                    && !matches!(
                        parameter.value_type,
                        ValueKind::Vector3 | ValueKind::Duration | ValueKind::Quantity
                    ))
        }) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Declared parameter binding must resolve to an existing scalar leaf",
                Some("/parameters"),
            ));
        }
    }
    Ok(())
}

pub(super) fn definition_parameter(
    definition: &ModuleDefinition,
    id: &ObjectId,
    value: &TypedValue,
    basis: &SourceBasis,
) -> Result<ModuleDefinition, ApiError> {
    let parameter = definition
        .parameters
        .iter()
        .find(|parameter| &parameter.id == id)
        .ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Parameter was not declared by the selected exact module",
                Some("/parameter_id"),
            )
        })?;
    let raw = parameter_value(parameter, value)?;
    let mut json = serde_json::to_value(definition).unwrap();
    *json.pointer_mut(&parameter.binding).ok_or_else(|| {
        error(
            ErrorCode::MissingReference,
            "Parameter binding no longer exists",
            Some("/parameter_id"),
        )
    })? = raw;
    let mut output: ModuleDefinition = serde_json::from_value(json).map_err(|failure| {
        error(
            ErrorCode::InvalidInput,
            &format!("Parameter binding violates its typed definition: {failure}"),
            Some("/value"),
        )
    })?;
    output
        .parameters
        .iter_mut()
        .find(|parameter| &parameter.id == id)
        .unwrap()
        .basis = basis.clone();
    validate_definition(&output)?;
    Ok(output)
}

pub(super) fn instance_parameter_value(
    definition: &ModuleDefinition,
    id: &ObjectId,
    value: &TypedValue,
) -> Result<(), ApiError> {
    let parameter = definition
        .parameters
        .iter()
        .find(|parameter| &parameter.id == id)
        .ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Parameter was not declared by the selected exact module",
                Some("/parameter_id"),
            )
        })?;
    if !parameter.binding.starts_with("/payload/data/")
        || parameter.binding.contains('*')
        || parameter.binding.split('/').any(|part| {
            matches!(
                part,
                "id" | "version"
                    | "definition_state"
                    | "capabilities"
                    | "acknowledgments"
                    | "stock"
                    | "clock"
            )
        })
    {
        return Err(error(
            ErrorCode::InvalidInput,
            "Declared instance binding is not an allowed payload parameter",
            Some("/parameter_id"),
        ));
    }
    parameter_value(parameter, value).map(|_| ())
}

fn parameter_value(
    parameter: &ParameterDeclaration,
    value: &TypedValue,
) -> Result<Value, ApiError> {
    let valid = matches!(
        (&parameter.value_type, value),
        (ValueKind::Number, TypedValue::Number(_))
            | (ValueKind::Integer, TypedValue::Integer(_))
            | (ValueKind::Boolean, TypedValue::Boolean(_))
            | (ValueKind::String, TypedValue::String(_))
            | (ValueKind::Enum, TypedValue::Enum(_))
            | (ValueKind::Vector3, TypedValue::Vector3(_))
            | (ValueKind::Duration, TypedValue::Duration(_))
            | (ValueKind::Quantity, TypedValue::Quantity(_))
            | (ValueKind::ItemType, TypedValue::ItemType(_))
    );
    if !valid {
        return Err(error(
            ErrorCode::InvalidInput,
            "Parameter value does not match its declared type",
            Some("/value"),
        ));
    }
    let encoded = serde_json::to_value(value).unwrap();
    let raw = encoded.get("value").cloned().unwrap();
    if let Some(number) = raw.as_f64() {
        if (parameter.integer && number.fract() != 0.0)
            || parameter
                .minimum
                .as_ref()
                .is_some_and(|minimum| number < minimum.value())
            || parameter
                .maximum
                .as_ref()
                .is_some_and(|maximum| number > maximum.value())
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Parameter violates its declared integer/range bounds",
                Some("/value"),
            ));
        }
    }
    if let TypedValue::Enum(choice) = value {
        if !parameter.options.contains(choice) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Parameter enum choice is not declared",
                Some("/value"),
            ));
        }
    }
    if let TypedValue::Quantity(quantity) = value {
        if parameter
            .unit
            .as_ref()
            .is_some_and(|unit| unit != &quantity.unit)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Parameter quantity unit differs from its declaration",
                Some("/value"),
            ));
        }
    }
    Ok(raw)
}

pub(super) fn definition_dependencies(definition: &ModuleDefinition) -> Vec<ModuleRef> {
    fn visit(value: &Value, refs: &mut Vec<ModuleRef>) {
        if let Ok(reference) = serde_json::from_value::<ModuleRef>(value.clone()) {
            if !refs
                .iter()
                .any(|old| module_key(old) == module_key(&reference))
            {
                refs.push(reference);
            }
            return;
        }
        match value {
            Value::Object(fields) => {
                for child in fields.values() {
                    visit(child, refs);
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(child, refs);
                }
            }
            _ => {}
        }
    }
    let mut refs = vec![];
    visit(
        &serde_json::to_value(&definition.payload).unwrap(),
        &mut refs,
    );
    refs
}

pub(super) fn definition_package(definition: &ModuleDefinition) -> Result<ModulePackage, ApiError> {
    fn visit(value: &Value, assets: &mut Vec<AssetRef>, files: &mut Vec<FileRef>) {
        if let Ok(asset) = serde_json::from_value::<AssetRef>(value.clone()) {
            if !assets.iter().any(|old| old.id == asset.id) {
                assets.push(asset);
            }
            return;
        }
        if let Ok(file) = serde_json::from_value::<FileRef>(value.clone()) {
            if !files.iter().any(|old| old.path == file.path) {
                files.push(file);
            }
            return;
        }
        match value {
            Value::Object(fields) => {
                for child in fields.values() {
                    visit(child, assets, files);
                }
            }
            Value::Array(values) => {
                for child in values {
                    visit(child, assets, files);
                }
            }
            _ => {}
        }
    }
    let mut assets = vec![];
    let mut files = vec![];
    visit(
        &serde_json::to_value(&definition.payload).unwrap(),
        &mut assets,
        &mut files,
    );
    if let ModulePayload::Templates(template) = &definition.payload {
        files.push(FileRef {
            path: template.project.clone(),
            kind: DocumentKind::Template,
        });
        if let Some(path) = &template.generation_profile {
            files.push(FileRef {
                path: path.clone(),
                kind: DocumentKind::GenerationProfile,
            });
        }
    }
    let mut rights = vec![];
    for asset in &assets {
        rights.push(definition.contribution.assets.iter().find(|right| right.asset_ref == asset.id).cloned().ok_or_else(|| error(ErrorCode::MissingReference, "An actual included media asset needs its explicit canonical rights declaration", Some("/contribution/assets")))?);
    }
    Ok(ModulePackage {
        assets,
        definition: definition.clone(),
        files,
        dependencies: definition_dependencies(definition),
        rights,
        issues: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn id(value: &str) -> ObjectId {
        ObjectId::try_from(value.to_owned()).unwrap()
    }
    fn call(host: &mut ProjectHost, scope: Value, operation: &str, params: Value) -> Value {
        let project_id = if scope["kind"] == "workspace" {
            Value::Null
        } else {
            serde_json::to_value(&host.project.as_ref().unwrap().id).unwrap()
        };
        host.dispatch(&id("definition-ui"), serde_json::from_value(json!({"api_version":"0.1","request_id":"definition-test","project_id":project_id,"scope":scope,"operation":operation,"params":params})).unwrap())
    }
    fn host() -> ProjectHost {
        let mut host = ProjectHost::new(id("definition-host"), id("definition-ui"));
        let response = call(
            &mut host,
            json!({"kind":"workspace"}),
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"definition test","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}),
        );
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        host
    }
    fn definition() -> Value {
        json!({"id":"local.item","version":"1.0.0","title":"Item source","definition_state":"design","contribution":{"author":null,"license":null,"assets":[]},"parameters":[],"attribute_schema":[],"payload":{"category":"items","data":{"presentation_or_media_ref":null,"unit":"piece","tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]}}})
    }

    #[test]
    fn actual_packaging_task_stays_in_its_definition_namespace() {
        let mut host = host();
        let created = call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":definition()}]}),
        );
        let draft = ok(&created)["scope"]["data"]["id"].clone();
        let scope = json!({"kind":"definition_draft","data":draft});
        let values: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/packaging-acceptance-cases.json"
        ))
        .unwrap();
        let job = values["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == "HOLD01")
            .unwrap()["job"]["data"]
            .clone();
        let response = call(
            &mut host,
            scope.clone(),
            "compute_packaging",
            json!({"packaging_job":job,"input_revision":1,"computation_budget":job["budget"]}),
        );
        let task = ok(&response)["id"].clone();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let state = call(
            &mut host,
            scope.clone(),
            "get_task",
            json!({"task_id":task}),
        );
        assert_eq!(ok(&state)["task"]["state"], "completed", "{state}");
        let artifact = ok(&state)["result"]["data"]["artifact_id"].clone();
        let read = call(
            &mut host,
            scope.clone(),
            "get_object",
            json!({"object_id":{"kind":"artifact","id":artifact},"source":{"kind":"draft"},"node_index":null}),
        );
        assert_eq!(ok(&read)["data"]["evidence_origin"], "actual_software");
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 1);
        assert!(host.professional.jobs.is_empty());
        assert!(host.professional.artifacts.is_empty());
        let draft_id: ModuleId = serde_json::from_value(draft).unwrap();
        assert_eq!(host.definition_drafts[&draft_id].revision.0.value(), 2);
        assert!(host.definition_drafts[&draft_id].dirty);
        assert!(host.definition_drafts[&draft_id].undo.is_empty());
        let project_read = call(
            &mut host,
            json!({"kind":"project"}),
            "get_object",
            json!({"object_id":{"kind":"artifact","id":artifact},"source":{"kind":"draft"},"node_index":null}),
        );
        assert_eq!(project_read["outcome"]["data"]["code"], "MissingReference");
    }
    fn ok(value: &Value) -> &Value {
        assert_eq!(value["outcome"]["status"], "ok", "{value}");
        &value["outcome"]["data"]
    }

    #[test]
    fn definition_export_requires_commit_and_writes_the_selected_exact_module() {
        let mut host = host();
        let created = call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":definition()}]}),
        );
        let scope = json!({"kind":"definition_draft","data":ok(&created)["scope"]["data"]["id"]});
        let root = std::env::temp_dir().join(format!(
            "sf-definition-export-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let destination = root.join("explicit-module.sfproj");
        let token = host
            .select_file(
                &id("definition-ui"),
                FilePurpose::ExportPackage,
                destination.clone(),
            )
            .unwrap();
        let selection = json!({"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[{"id":"local.item","version":"1.0.0"}],"include_redistributable_assets":false});
        let dirty = call(
            &mut host,
            scope.clone(),
            "export_package",
            json!({"revision":1,"package_scope":selection,"selected_destination_ref":token}),
        );
        assert_eq!(dirty["outcome"]["data"]["code"], "InvalidInput", "{dirty}");
        assert!(host.drain_io_work().is_empty());
        ok(&call(
            &mut host,
            scope.clone(),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"commit_definition_version","data":{"version":"1.0.0"}}]}),
        ));
        let response = call(
            &mut host,
            scope.clone(),
            "export_package",
            json!({"revision":2,"package_scope":selection,"selected_destination_ref":token}),
        );
        let task: TaskHandle = serde_json::from_value(ok(&response).clone()).unwrap();
        assert!(matches!(
            task.scope,
            TaskScope::Edit(EditScope::DefinitionDraft(_))
        ));
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let state = call(
            &mut host,
            scope.clone(),
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(ok(&state)["task"]["state"], "completed", "{state}");
        let archive =
            crate::project_store::read_project_zip(&destination, &AtomicBool::new(false)).unwrap();
        assert!(archive.scenarios.is_empty());
        assert!(archive.project.devices.is_empty());
        assert!(archive.project.bindings.is_empty());
        let modules = archive.modules().unwrap();
        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].definition.id.as_str(), "local.item");
        assert_eq!(
            serde_json::to_value(&modules[0].definition.version).unwrap(),
            json!("1.0.0")
        );
        assert_eq!(modules[0].definition.title, "Item source");
        let wrong_scope = call(
            &mut host,
            json!({"kind":"project"}),
            "get_task",
            json!({"task_id":task.id}),
        );
        assert_eq!(
            wrong_scope["outcome"]["data"]["code"], "PermissionDenied",
            "{wrong_scope}"
        );
        let cases: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/packaging-acceptance-cases.json"
        ))
        .unwrap();
        let job = cases["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == "HOLD01")
            .unwrap()["job"]["data"]
            .clone();
        let produced = call(
            &mut host,
            json!({"kind":"project"}),
            "compute_packaging",
            json!({"packaging_job":job,"input_revision":1,"computation_budget":job["budget"]}),
        );
        let production = ok(&produced)["id"].clone();
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
        let actual = call(
            &mut host,
            json!({"kind":"project"}),
            "get_task",
            json!({"task_id":production}),
        );
        assert_eq!(ok(&actual)["task"]["state"], "completed", "{actual}");
        let mut forbidden = selection;
        forbidden["artifact_ids"] = json!([ok(&actual)["result"]["data"]["artifact_id"]]);
        let denied = call(
            &mut host,
            scope,
            "export_package",
            json!({"revision":2,"package_scope":forbidden,"selected_destination_ref":token}),
        );
        assert_eq!(
            denied["outcome"]["data"]["code"], "PermissionDenied",
            "{denied}"
        );
        assert!(denied["context"].is_null());
        assert!(host.drain_io_work().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn actual_definition_scope_has_its_own_revision_and_freezes_immutable_exact_versions() {
        let mut host = host();
        let created = call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":definition()}]}),
        );
        let draft = ok(&created)["scope"]["data"]["id"].clone();
        assert_ne!(draft, "local.item");
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 1);
        let scope = json!({"kind":"definition_draft","data":draft});
        let mut changed = definition();
        changed["title"] = json!("Editable default");
        let edit = call(
            &mut host,
            scope.clone(),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_definition","data":changed}]}),
        );
        assert_eq!(ok(&edit)["new_revision"], 2);
        assert_eq!(edit["context"]["scope_revision"], 2);
        assert!(edit["context"]["project_revision"].is_null());
        let commit = call(
            &mut host,
            scope.clone(),
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"commit_definition_version","data":{"version":"1.0.0"}}]}),
        );
        assert_eq!(ok(&commit)["new_revision"], 3);
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 1);
        let frozen = host.modules.values().next().unwrap().definition.clone();
        let mut changed = serde_json::to_value(&frozen).unwrap();
        changed["title"] = json!("Next revision");
        ok(&call(
            &mut host,
            scope.clone(),
            "apply_commands",
            json!({"base_revision":3,"commands":[{"kind":"put_definition","data":changed}]}),
        ));
        let rejected = call(
            &mut host,
            scope.clone(),
            "apply_commands",
            json!({"base_revision":4,"commands":[{"kind":"commit_definition_version","data":{"version":"1.0.0"}}]}),
        );
        assert_eq!(rejected["outcome"]["data"]["code"], "InvalidInput");
        assert_eq!(
            serde_json::to_value(&host.modules.values().next().unwrap().definition).unwrap(),
            serde_json::to_value(frozen).unwrap()
        );
        assert_eq!(
            host.definition_drafts
                .values()
                .next()
                .unwrap()
                .revision
                .0
                .value(),
            4
        );
        ok(&call(
            &mut host,
            scope,
            "apply_commands",
            json!({"base_revision":4,"commands":[{"kind":"commit_definition_version","data":{"version":"1.0.1"}}]}),
        ));
        assert_eq!(host.modules.len(), 2);
        let queried = call(
            &mut host,
            json!({"kind":"project"}),
            "query_module_library",
            json!({"category_filters":["items"],"industry":null,"product_family":null,"site_envelope":null,"source_scope":["project_packages"]}),
        );
        assert_eq!(ok(&queried)["candidates"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn stable_array_leaf_grant_follows_declared_identity_after_a_ui_reorder() {
        let old = json!({"orders":[{"id":"B","quantity":2},{"id":"A","quantity":1}]});
        let new = json!({"orders":[{"id":"B","quantity":2},{"id":"A","quantity":3}]});
        let fields = changed_fields(&old, &new).unwrap();
        assert_eq!(
            fields,
            vec![("/orders/1/quantity".into(), Some("A".into()))]
        );
        let grant = HostFieldGrant {
            pointer: "/orders/0/quantity".into(),
            element_id: Some(id("A")),
        };
        assert!(field_granted(
            &grant,
            &fields[0].0,
            fields[0].1.as_deref(),
            &old
        ));
        assert!(!field_granted(
            &grant,
            "/orders/0/quantity",
            Some("B"),
            &old
        ));
        let numeric = HostFieldGrant {
            pointer: "/orders/1/quantity".into(),
            element_id: None,
        };
        assert!(!field_granted(&numeric, &fields[0].0, Some("A"), &old));
        assert!(
            changed_fields(
                &old,
                &json!({"orders":[{"id":"A","quantity":1},{"id":"B","quantity":2}]})
            )
            .is_err()
        );
    }

    #[test]
    fn instance_parameter_and_its_source_commit_together_without_editing_the_exact_library_version()
    {
        let mut host = host();
        let mut source = definition();
        source["id"] = json!("local.adjustable-device");
        source["payload"] = json!({"category":"devices","data":{"placement":"fixed_during_simulation","ports":[],"behavior":{"inline":null,"operation_ref":null,"selected":null},"configuration":{"speed":{"kind":"number","value":1}},"presentation":null,"operations":[]}});
        source["parameters"] = json!([{"id":"speed","value_type":"number","binding":"/payload/data/configuration/speed/value","label":null,"unit":null,"minimum":0,"maximum":10,"integer":false,"options":[],"basis":"module_declared"}]);
        let created = call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":source}]}),
        );
        let scope = json!({"kind":"definition_draft","data":ok(&created)["scope"]["data"]["id"]});
        ok(&call(
            &mut host,
            scope,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"commit_definition_version","data":{"version":"1.0.0"}}]}),
        ));
        let fixture: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/local-assembly-pack.json"
        ))
        .unwrap();
        let mut device = fixture["project"]["devices"][0].clone();
        device["id"] = json!("adjustable");
        device["module_ref"] = json!({"id":"local.adjustable-device","version":"1.0.0"});
        device["operation_binding"] = Value::Null;
        device["configuration"] = json!({"speed":{"kind":"number","value":1}});
        ok(&call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"adopt_module","data":{"module_ref":{"id":"local.adjustable-device","version":"1.0.0"},"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[{"kind":"device","data":device}],"explicit_seed":null}}]}),
        ));
        ok(&call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"adjustable"},"parameter_id":"speed","value":{"kind":"number","value":7},"basis":"user_provided"}}]}),
        ));
        let current = host.current_draft();
        assert_eq!(current.project.parameter_sources.len(), 1);
        assert_eq!(
            current.project.parameter_sources[0].field_path,
            "/configuration/speed/value"
        );
        assert_eq!(
            serde_json::to_value(&current.project.parameter_sources[0].value).unwrap(),
            json!({"kind":"number","value":7.0})
        );
        assert_eq!(
            serde_json::to_value(&host.modules.values().next().unwrap().definition.payload)
                .unwrap()["data"]["configuration"]["speed"]["value"]
                .as_f64(),
            Some(1.0)
        );
        let mut invalid = serde_json::to_value(&current.project.devices[0]).unwrap();
        invalid["configuration"]["speed"]["value"] = json!(8);
        let rejected = call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":3,"commands":[{"kind":"put_object","data":{"kind":"device","data":invalid}}]}),
        );
        assert_eq!(rejected["outcome"]["data"]["code"], "InvalidInput");
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), 3);
        ok(&call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":3,"commands":[{"kind":"undo"}]}),
        ));
        assert!(host.project.as_ref().unwrap().parameter_sources.is_empty());
        assert_eq!(
            serde_json::to_value(&host.project.as_ref().unwrap().devices[0].configuration["speed"])
                .unwrap()["value"]
                .as_f64(),
            Some(1.0)
        );
        ok(&call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":4,"commands":[{"kind":"redo"}]}),
        ));
        assert_eq!(host.project.as_ref().unwrap().parameter_sources.len(), 1);
        assert_eq!(
            serde_json::to_value(&host.project.as_ref().unwrap().devices[0].configuration["speed"])
                .unwrap()["value"]
                .as_f64(),
            Some(7.0)
        );
    }

    #[test]
    fn actual_ui_reacknowledging_equal_imported_metadata_still_establishes_a_history_boundary() {
        let mut host = host();
        let values: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/production/professional-documents.json"
        ))
        .unwrap();
        let artifact: ArtifactRecord =
            serde_json::from_value(values["artifacts"][0].clone()).unwrap();
        let ack: ArtifactAcknowledgment = serde_json::from_value(json!({"id":"REACK","artifact_ref":artifact.id,"accepted_subject":artifact.subject_ref,"accepted_scope":[artifact.supported_scope[0]],"decision_source":{"kind":"human_statement","data":"current explicit UI confirmation"},"conditions":[],"recorded_at_iso8601":null})).unwrap();
        host.professional
            .artifacts
            .insert(artifact.id.clone(), artifact.clone());
        host.project
            .as_mut()
            .unwrap()
            .acknowledgments
            .push(ack.clone());
        host.professional_trust.clear();
        host.history.reset(&host.current_draft());
        let settings = settings_value(host.project.as_ref().unwrap());
        let mut settings = settings;
        settings["title"] = json!("ordinary reversible edit");
        ok(&call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"set_project_settings","data":settings}]}),
        ));
        assert!(host.history.availability().can_undo);
        let before = serde_json::to_value(&artifact).unwrap();
        let acknowledged = call(
            &mut host,
            json!({"kind":"project"}),
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"record_artifact_acknowledgment","data":ack}]}),
        );
        assert_eq!(ok(&acknowledged)["new_revision"], 3);
        assert!(!host.history.availability().can_undo);
        assert!(host.history.availability().boundary_reason.is_some());
        assert_eq!(host.project.as_ref().unwrap().acknowledgments.len(), 1);
        assert_eq!(
            serde_json::to_value(host.professional.artifacts.values().next().unwrap()).unwrap(),
            before
        );
    }
}
