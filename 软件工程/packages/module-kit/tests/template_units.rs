use serde_json::json;
use sf_domain::*;
use sf_module_kit::*;
use std::collections::BTreeMap;
#[test]
fn generated_device_asset_rights_match_both_package_and_definition_aggregate() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, which) in [
        ("machining-delivery", BuiltinTemplate::MachiningDelivery),
        ("purchased-assembly", BuiltinTemplate::PurchasedAssembly),
        (
            "parallel-electromechanical",
            BuiltinTemplate::ParallelElectromechanical,
        ),
        ("carton-factory", BuiltinTemplate::CartonFactory),
    ] {
        let candidate = builtin_template(which).unwrap();
        let base = root.join("modules/templates").join(name).join("candidate");
        let mut asset_sources = 0;
        for file in &candidate.package.files {
            if !file.path.as_str().ends_with("/module.json") {
                continue;
            }
            let data: Versioned<ModulePackage> =
                serde_json::from_slice(&std::fs::read(base.join(file.path.as_str())).unwrap())
                    .unwrap();
            let package = data.data;
            assert_eq!(
                serde_json::to_value(&package.definition.contribution.assets).unwrap(),
                serde_json::to_value(&package.rights).unwrap(),
                "{name}: {}",
                file.path.as_str()
            );
            if let Some(definition) = candidate
                .definitions
                .iter()
                .find(|d| d.id == package.definition.id && d.version == package.definition.version)
            {
                assert_eq!(
                    serde_json::to_value(definition).unwrap(),
                    serde_json::to_value(&package.definition).unwrap()
                );
            }
            for asset in &package.assets {
                assert!(package.rights.iter().any(|r| r.asset_ref == asset.id));
                asset_sources += 1;
            }
        }
        assert!(
            asset_sources > 0,
            "must inspect actual packaged source assets for {name}"
        );
    }
}
#[test]
fn explicit_preparation_encodes_new_reused_and_unresolved_resources_without_forged_metadata() {
    let c = builtin_template(BuiltinTemplate::ParallelElectromechanical).unwrap();
    let mut target = target();
    let mut existing = c.example_project.resources[0].clone();
    let source = existing.id.clone();
    existing.id = "actual-worker".to_owned().try_into().unwrap();
    target.calendars.push(
        c.example_project
            .calendars
            .iter()
            .find(|x| x.id == existing.calendar_id)
            .unwrap()
            .clone(),
    );
    target.resources.push(existing.clone());
    let mut choices = options(&c);
    choices.namespace = "explicit:namespace".into();
    choices.resources.insert(
        source.clone(),
        TemplateResourceSelection::Reuse(existing.id.clone()),
    );
    let unresolved = c.example_project.resources[1].id.clone();
    choices.resources.remove(&unresolved);
    let created = c.example_project.resources[2].id.clone();
    let p = prepare_template(&c, &target, &choices).unwrap();
    let path: RelativeFilePath = "example-project.json".to_owned().try_into().unwrap();
    let value = p.command_data_with_preparation(&path, &choices, None, None);
    assert_eq!(
        value["preparation"]["source_project"],
        "example-project.json"
    );
    assert_eq!(value["preparation"]["namespace"], "explicit:namespace");
    assert_eq!(
        value["preparation"]["resources"][source.as_str()],
        existing.id.as_str()
    );
    assert!(value["preparation"]["resources"][created.as_str()].is_null());
    assert!(
        !value["preparation"]["resources"]
            .as_object()
            .unwrap()
            .contains_key(unresolved.as_str())
    );
    assert!(value["preparation"]["site_envelope"].is_null());
    assert!(
        p.command_data(None).get("preparation").is_none(),
        "legacy API stays usable"
    );
    for key in [
        "adoption",
        "template_adoptions",
        "sources",
        "parameter_sources",
        "acknowledgments",
    ] {
        assert!(value.get(key).is_none());
    }
    assert_eq!(value["subset"], serde_json::to_value(&c.subset).unwrap());
}
#[test]
fn explicit_site_preparation_records_the_same_supplied_dimensions_and_unknown_geometry() {
    let c = builtin_template(BuiltinTemplate::CartonFactory).unwrap();
    let choices = options(&c);
    let mut p = prepare_template(&c, &target(), &choices).unwrap();
    let mut site = carton_generation_profile().unwrap().site_applicability;
    site.length_m = Some(32.0.try_into().unwrap());
    site.width_m = Some(20.0.try_into().unwrap());
    fill_template_site(&mut p, &site).unwrap();
    let path = "example-project.json".to_owned().try_into().unwrap();
    let data = p.command_data_with_preparation(&path, &choices, Some(&site), None);
    assert_eq!(
        data["preparation"]["site_envelope"],
        serde_json::to_value(&site).unwrap()
    );
    assert!(p.proposed_project.halls[0].height_m.is_none());
    assert!(
        p.proposed_project
            .devices
            .iter()
            .all(|x| x.footprint_m.is_empty() && x.height_m.is_none())
    );
    assert!(
        p.proposed_project
            .parameter_sources
            .iter()
            .any(|s| s.field_path == "/width_m" && matches!(s.basis, SourceBasis::UserProvided))
    );
}
#[test]
fn typed_item_references_are_remapped_but_identical_display_strings_are_not() {
    let mut c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let old = c.example_project.item_types[0].id.clone();
    c.example_project.item_types[0]
        .attributes
        .insert("linked_type".into(), TypedValue::ItemType(old.clone()));
    c.example_project.item_types[0]
        .attributes
        .insert("display".into(), TypedValue::String(old.as_str().into()));
    let p = prepare_template(&c, &target(), &options(&c)).unwrap();
    let v = serde_json::to_value(&p.proposed_project.item_types[0].attributes).unwrap();
    assert_eq!(v["linked_type"]["value"], format!("NEW:{}", old.as_str()));
    assert_eq!(v["display"]["value"], old.as_str());
}
#[test]
fn source_packages_list_existing_files_with_unique_relative_paths() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    for (name, which) in [
        ("machining-delivery", BuiltinTemplate::MachiningDelivery),
        ("purchased-assembly", BuiltinTemplate::PurchasedAssembly),
        (
            "parallel-electromechanical",
            BuiltinTemplate::ParallelElectromechanical,
        ),
        ("carton-factory", BuiltinTemplate::CartonFactory),
    ] {
        let candidate = builtin_template(which).unwrap();
        let mut seen = std::collections::BTreeSet::new();
        for f in candidate.package.files {
            assert!(
                seen.insert(f.path.as_str().to_owned()),
                "duplicate {}",
                f.path.as_str()
            );
            assert!(
                root.join("modules/templates")
                    .join(name)
                    .join("candidate")
                    .join(f.path.as_str())
                    .is_file(),
                "missing {}",
                f.path.as_str()
            );
        }
    }
}
fn target() -> ProjectState {
    empty_template_project(
        "target".to_owned().try_into().unwrap(),
        "target".into(),
        TimeReference {
            label: "scenario_start".into(),
            origin_iso8601: None,
            timezone: None,
        },
    )
}
fn options(c: &TemplateCandidate) -> TemplateChoices {
    TemplateChoices {
        namespace: "NEW".into(),
        resources: c
            .example_project
            .resources
            .iter()
            .map(|r| {
                (
                    r.id.clone(),
                    TemplateResourceSelection::CreateDeclaredExample,
                )
            })
            .collect(),
        reuse_items: BTreeMap::new(),
        include_example_product: true,
    }
}
fn seed_options(c: &TemplateCandidate, supply: bool, demand: bool) -> TemplateSeedChoices {
    TemplateSeedChoices {
        scenario_id: "new-seed".to_owned().try_into().unwrap(),
        supply,
        demand,
        initial_quality: c
            .subset
            .adoption
            .as_ref()
            .unwrap()
            .deferred_seed
            .as_ref()
            .unwrap()
            .initial_stock
            .iter()
            .map(|r| (r.location_id.clone(), QualityState::Released))
            .collect(),
        adopt_declared_inspection_results: supply && demand,
    }
}
#[test]
fn aggregate_reuse_map_keeps_every_execution_pool_instead_of_overwriting_the_shared_role() {
    let c = builtin_template(BuiltinTemplate::ParallelElectromechanical).unwrap();
    let p = prepare_template(&c, &target(), &options(&c)).unwrap();
    let unique: std::collections::BTreeSet<_> = p.mapped_bindings.resources.values().collect();
    assert_eq!(unique.len(), c.example_project.resources.len());
}
#[test]
fn nine_category_examples_are_current_typed_assets_not_solver_availability() {
    let packages = builtin_template_support().unwrap();
    let categories: std::collections::BTreeSet<_> = packages
        .iter()
        .map(|p| {
            serde_json::to_value(&p.definition.payload).unwrap()["category"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(categories.len(), 9);
    assert!(
        packages
            .iter()
            .any(|p| matches!(&p.definition.payload, ModulePayload::Professional(_)))
    );
    assert!(packages.iter().any(|p|matches!(&p.definition.payload,ModulePayload::Media(m) if matches!(m.production_effect,ProductionEffect::None))));
}
#[test]
fn carton_profile_and_site_fill_preserve_unknowns_and_never_assign_output_from_goal() {
    let profile = carton_generation_profile().unwrap();
    assert_eq!(profile.industry, "paper_carton_manufacturing");
    assert!(profile.site_applicability.height_m.is_none());
    let c = builtin_template(BuiltinTemplate::CartonFactory).unwrap();
    let mut plan = prepare_template(&c, &target(), &options(&c)).unwrap();
    let mut site = profile.site_applicability;
    site.length_m = Some(30.0.try_into().unwrap());
    site.width_m = Some(20.0.try_into().unwrap());
    fill_template_site(&mut plan, &site).unwrap();
    assert_eq!(plan.proposed_project.halls[0].width_m.value(), 30.0);
    assert_eq!(plan.proposed_project.halls[0].depth_m.value(), 20.0);
    assert!(plan.proposed_project.halls[0].height_m.is_none());
    assert!(plan.proposed_project.generation_context.is_none());
    assert!(
        plan.proposed_project
            .devices
            .iter()
            .all(|d| d.footprint_m.is_empty())
    );
    let before = serde_json::to_value(&plan.proposed_project).unwrap();
    site.width_m = None;
    assert!(fill_template_site(&mut plan, &site).is_err());
    assert_eq!(
        serde_json::to_value(&plan.proposed_project).unwrap(),
        before
    );
}
#[test]
fn four_current_candidates_decode_and_empty_entry_has_no_hidden_seed() {
    for which in [
        BuiltinTemplate::MachiningDelivery,
        BuiltinTemplate::PurchasedAssembly,
        BuiltinTemplate::ParallelElectromechanical,
        BuiltinTemplate::CartonFactory,
    ] {
        let c = builtin_template(which).unwrap();
        assert!(c.subset.seed.is_none());
        assert!(c.subset.halls[0].height_m.is_none());
        assert!(
            c.subset
                .devices
                .iter()
                .all(|d| d.footprint_m.is_empty() && d.height_m.is_none())
        );
        assert!(c.example_project.acknowledgments.is_empty());
    }
    let p = target();
    assert!(p.item_types.is_empty() && p.scenario_refs.is_empty() && p.devices.is_empty());
}
#[test]
fn fresh_ids_rewrite_all_internal_roles_connections_routes_and_claim_consumers() {
    let c = builtin_template(BuiltinTemplate::ParallelElectromechanical).unwrap();
    let p = prepare_template(&c, &target(), &options(&c)).unwrap();
    for d in &p.proposed_project.devices {
        assert!(d.id.as_str().starts_with("NEW:"));
        assert!(d.hall_id.as_str().starts_with("NEW:"));
        for x in d.role_bindings.resources.values() {
            assert!(x.as_str().starts_with("NEW:"));
        }
        for x in d.role_bindings.targets.values() {
            assert!(x.id.as_str().starts_with("NEW:"));
        }
    }
    for r in &p.proposed_project.routes {
        for s in &r.steps {
            assert!(s.id.as_str().starts_with("NEW:"));
            assert!(s.device_id.as_str().starts_with("NEW:"));
            assert!(
                s.predecessors
                    .iter()
                    .all(|x| x.as_str().starts_with("NEW:"))
            );
        }
    }
    let a = &p.proposed_project.template_adoptions[0];
    assert!(
        a.preparation_bindings[0]
            .worker_roles
            .values()
            .all(|x| x.as_str() == "W-E")
    );
    assert!(
        a.preparation_bindings[0]
            .target_device_id
            .as_str()
            .starts_with("NEW:")
    );
    assert!(
        a.imported_claims
            .iter()
            .flat_map(|x| &x.consumer_device_ids)
            .all(|x| x.as_str().starts_with("NEW:"))
    );
    assert_eq!(a.source_archive.path.as_str(), "source-project.design.json");
    assert_eq!(
        serde_json::to_value(&c.subset.module_refs).unwrap(),
        serde_json::to_value(&p.proposed_project.module_refs).unwrap()
    );
}
#[test]
fn command_preserves_exact_source_subset_and_defaults_to_no_scenario() {
    let c = builtin_template(BuiltinTemplate::MachiningDelivery).unwrap();
    let p = prepare_template(&c, &target(), &options(&c)).unwrap();
    let value = p.command_data(None);
    assert_eq!(value["subset"], serde_json::to_value(&c.subset).unwrap());
    assert!(value["explicit_new_scenario_seed"].is_null());
    assert!(!value.to_string().contains("runtime_items"));
    assert!(p.proposed_project.scenario_refs.is_empty());
}
#[test]
fn namespace_collision_rejects_without_touching_target() {
    let c = builtin_template(BuiltinTemplate::MachiningDelivery).unwrap();
    let a = prepare_template(&c, &target(), &options(&c))
        .unwrap()
        .proposed_project;
    let before = serde_json::to_value(&a).unwrap();
    assert!(
        prepare_template(&c, &a, &options(&c))
            .unwrap_err()
            .iter()
            .any(|x| x.code == "template.id_collision")
    );
    assert_eq!(serde_json::to_value(&a).unwrap(), before);
}
#[test]
fn supply_and_demand_are_independent_and_seed_requires_explicit_quality() {
    let c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let p = prepare_template(&c, &target(), &options(&c)).unwrap();
    for (s, d) in [(false, false), (true, false), (false, true), (true, true)] {
        let sc = prepare_template_seed(&c, &p, &seed_options(&c, s, d)).unwrap();
        assert_eq!(!sc.initial_conditions.items.is_empty(), s);
        assert_eq!(!sc.orders.is_empty(), d);
        assert_eq!(!sc.production_tasks.is_empty(), d);
        assert!(sc.arrivals.is_empty() && sc.initial_conditions.material_allocations.is_empty());
    }
    let mut ch = seed_options(&c, true, false);
    ch.initial_quality.clear();
    assert_eq!(
        prepare_template_seed(&c, &p, &ch).unwrap_err()[0].code,
        "template.initial_quality"
    );
}
#[test]
fn product_example_is_a_separate_decision_and_professional_claims_stay_claims() {
    let c = builtin_template(BuiltinTemplate::ParallelElectromechanical).unwrap();
    let mut o = options(&c);
    o.include_example_product = false;
    let p = prepare_template(&c, &target(), &o).unwrap();
    assert!(p.proposed_project.products.is_empty() && p.proposed_project.routes.is_empty());
    assert!(prepare_template_seed(&c, &p, &seed_options(&c, false, true)).is_err());
    let a = &p.proposed_project.template_adoptions[0];
    assert!(a.imported_claims.iter().all(
        |x| matches!(x.mode, ProfessionalMode::DeclaredProcess) && !x.missing_fields.is_empty()
    ));
    assert!(a.preparation_bindings[0].route_refs.is_empty());
    assert!(
        p.proposed_project.bindings.is_empty() && p.proposed_project.acknowledgments.is_empty()
    );
}
#[test]
fn missing_staffing_does_not_invent_people_or_leave_fake_binding() {
    let c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let mut o = options(&c);
    o.resources.clear();
    let p = prepare_template(&c, &target(), &o).unwrap();
    assert!(p.proposed_project.resources.is_empty());
    assert!(p.proposed_project.calendars.is_empty());
    assert!(
        p.proposed_project
            .devices
            .iter()
            .all(|d| d.role_bindings.resources.is_empty())
    );
    assert!(
        p.proposed_project.template_adoptions[0]
            .resource_choices
            .iter()
            .all(|r| r.resource_id.is_none())
    );
    assert!(!p.issues.is_empty());
}
#[test]
fn explicit_resource_reuse_preserves_actual_capacity_calendar_home_and_identity() {
    let c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let mut t = target();
    let r = c.example_project.resources[0].clone();
    let cal = c
        .example_project
        .calendars
        .iter()
        .find(|x| x.id == r.calendar_id)
        .unwrap()
        .clone();
    t.resources.push(r.clone());
    t.calendars.push(cal);
    t.resources[0].capacity.value.coefficient = 10u64.try_into().unwrap();
    let before = serde_json::to_value(&t).unwrap();
    let mut o = options(&c);
    o.resources
        .insert(r.id.clone(), TemplateResourceSelection::Reuse(r.id.clone()));
    let p = prepare_template(&c, &t, &o).unwrap();
    assert!(!p.proposed_project.resources.iter().any(|x| x.id == r.id));
    assert!(
        p.proposed_project.devices.iter().any(|d| d
            .role_bindings
            .resources
            .values()
            .any(|x| x == &r.id))
    );
    assert_eq!(serde_json::to_value(&t).unwrap(), before);
    t.resources[0].capacity.unit = "slot".to_owned().try_into().unwrap();
    assert!(prepare_template(&c, &t, &o).is_err());
}
#[test]
fn unknown_item_specification_cannot_be_merged_by_display_name() {
    let c = builtin_template(BuiltinTemplate::MachiningDelivery).unwrap();
    let mut t = target();
    t.item_types.push(c.example_project.item_types[0].clone());
    let mut o = options(&c);
    o.reuse_items
        .insert(t.item_types[0].id.clone(), t.item_types[0].id.clone());
    assert_eq!(
        prepare_template(&c, &t, &o).unwrap_err()[0].code,
        "template.item_mismatch"
    );
}
#[test]
fn seed_reopen_and_edit_is_domain_roundtrip_not_shared_runtime() {
    let c = builtin_template(BuiltinTemplate::ParallelElectromechanical).unwrap();
    let p = prepare_template(&c, &target(), &options(&c)).unwrap();
    let s = prepare_template_seed(&c, &p, &seed_options(&c, true, true)).unwrap();
    let bytes = serde_json::to_vec(&Versioned {
        schema_version: SchemaVersion::V1,
        data: s,
    })
    .unwrap();
    let mut reopened: Versioned<ScenarioInput> = serde_json::from_slice(&bytes).unwrap();
    reopened.data.orders[0].due_at_ms = Some(SimTimeMs(260000u64.try_into().unwrap()));
    assert_eq!(
        reopened.data.orders[0]
            .due_at_ms
            .as_ref()
            .unwrap()
            .0
            .value(),
        260000
    );
    assert_eq!(
        c.subset
            .adoption
            .as_ref()
            .unwrap()
            .deferred_seed
            .as_ref()
            .unwrap()
            .window
            .end_ms
            .0
            .value(),
        300000
    );
}
#[test]
fn source_versions_and_bad_requirement_indices_are_rejected_at_decode() {
    let c = builtin_template(BuiltinTemplate::MachiningDelivery).unwrap();
    let m = serde_json::to_vec(&Versioned {
        schema_version: SchemaVersion::V1,
        data: c.package.clone(),
    })
    .unwrap();
    let p = serde_json::to_vec(&Versioned {
        schema_version: SchemaVersion::V1,
        data: c.example_project.clone(),
    })
    .unwrap();
    let d = serde_json::to_vec(&c.definitions).unwrap();
    let mut t = serde_json::to_value(Versioned {
        schema_version: SchemaVersion::V1,
        data: c.subset,
    })
    .unwrap();
    t["data"]["adoption"]["resource_choices"][0]["requirement_index"] = json!(999);
    assert_eq!(
        decode_template_candidate(&m, &serde_json::to_vec(&t).unwrap(), &p, &d).unwrap_err()[0]
            .code,
        "template.resource_source"
    );
    t["data"]["adoption"]["resource_choices"][0]["requirement_index"] = json!(0);
    t["data"]["adoption"]["template_ref"]["version"] = json!("99.0.0");
    assert_eq!(
        decode_template_candidate(&m, &serde_json::to_vec(&t).unwrap(), &p, &d).unwrap_err()[0]
            .code,
        "template.source_version"
    );
}
#[test]
fn carton_route_manufactures_cartons_instead_of_putting_products_into_purchased_cartons() {
    let c = builtin_template(BuiltinTemplate::CartonFactory).unwrap();
    let x = serde_json::to_value(&c.example_project).unwrap();
    assert_eq!(x["products"][0]["item_type_ref"]["id"], "carton");
    assert_eq!(x["item_types"].as_array().unwrap().len(), 3);
    assert_eq!(
        c.subset
            .adoption
            .as_ref()
            .unwrap()
            .deferred_seed
            .as_ref()
            .unwrap()
            .initial_stock[0]
            .item_type_id
            .as_str(),
        "board"
    );
    assert!(!c.definitions.iter().any(|d|matches!(&d.payload,ModulePayload::Devices(v) if matches!(v.behavior.inline.as_ref().unwrap().builtin,RuleKind::Pack))));
}
