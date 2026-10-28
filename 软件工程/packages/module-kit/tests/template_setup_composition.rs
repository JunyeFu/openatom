use serde_json::{Value, json};
use sf_domain::*;
use sf_module_kit::*;
use std::collections::BTreeMap;

fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/p104-B-first.json"
    ))
    .unwrap()
}
fn candidate(article: bool) -> TemplateCandidate {
    let input = fixture();
    let mut c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    c.example_project = serde_json::from_value(input["project"].clone()).unwrap();
    c.definitions = serde_json::from_value(input["definitions"].clone()).unwrap();
    c.example_project.sources.push(SourceRecord {
        id: id("setup-declaration"),
        title: "Finite source setup declaration: p104-B-first".into(),
        locator: Some(
            "packages/commands/tests/fixtures/production/p104-B-first.json#/project/setup_policies"
                .into(),
        ),
        read_scope: Some("Declared 15000ms/25000ms rules; not industrial measurement".into()),
        read_date: None,
        basis: SourceBasis::UserProvided,
    });
    c.example_project.setup_policies[0].changeovers[0]
        .duration
        .source_ref = Some(id("setup-declaration"));
    if article {
        c.example_project.setup_policies[0].first_article=Some(serde_json::from_value(json!({"route_ref":{"id":"ROUTE-B","revision":"r1"},"family":"B","inspection_module":{"id":"w5.op.inspect","version":"0.1.0"},"gate_id":"B-FIRST-ARTICLE"})).unwrap());
    }
    let p = &c.example_project;
    let adoption = TemplateAdoption {
        id: id("finite-setup-adoption"),
        template_ref: ModuleRef {
            id: c.package.definition.id.clone(),
            version: c.package.definition.version.clone(),
        },
        source_archive: FileRef {
            path: "source-project.json".to_owned().try_into().unwrap(),
            kind: DocumentKind::Project,
        },
        resource_choices: vec![],
        preparation_bindings: vec![],
        imported_claims: vec![],
        inspection_presets: vec![],
        deferred_seed: None,
        missing_inputs: vec![],
    };
    c.subset = TemplateSubset {
        layout_scope: c.subset.layout_scope.clone(),
        halls: p.halls.clone(),
        lines: p.lines.clone(),
        devices: p.devices.clone(),
        connections: p.connections.clone(),
        module_refs: p.module_refs.clone(),
        resource_requirements: vec![],
        item_types: p.item_types.clone(),
        role_requirements: vec![],
        suggested_role_bindings: RoleBindings {
            item_types: BTreeMap::new(),
            resources: BTreeMap::new(),
            targets: BTreeMap::new(),
            ports: BTreeMap::new(),
        },
        seed: None,
        locations: p.locations.clone(),
        adoption: Some(adoption.clone()),
    };
    c.example_project.template_adoptions = vec![adoption];
    c
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

#[test]
fn exact_source_policy_maps_device_gate_route_and_source_without_changing_text_times_or_versions() {
    let c = candidate(true);
    let before = serde_json::to_value(&c.example_project).unwrap();
    let p = prepare_template(&c, &target(), &options(&c)).unwrap();
    let policy = &p.proposed_project.setup_policies[0];
    assert_eq!(policy.device_id.as_str(), "NEW:Assembly");
    let article = policy.first_article.as_ref().unwrap();
    assert_eq!(article.route_ref.id.as_str(), "NEW:ROUTE-B");
    assert_eq!(article.route_ref.revision.as_str(), "r1");
    assert_eq!(article.gate_id.as_str(), "NEW:B-FIRST-ARTICLE");
    assert_eq!(
        serde_json::to_value(&article.inspection_module).unwrap(),
        json!({"id":"w5.op.inspect","version":"0.1.0"})
    );
    assert_eq!(article.family, "B");
    assert_eq!(policy.initial_family.as_deref(), Some("A"));
    assert_eq!(policy.changeovers[0].duration.value.0.value(), 15000);
    assert_eq!(policy.changeovers[1].duration.value.0.value(), 25000);
    assert_eq!(
        policy.changeovers[0]
            .duration
            .source_ref
            .as_ref()
            .unwrap()
            .as_str(),
        "NEW:setup-declaration"
    );
    let mut expected = serde_json::to_value(&c.example_project.setup_policies[0]).unwrap();
    expected["device_id"] = json!("NEW:Assembly");
    expected["first_article"]["route_ref"]["id"] = json!("NEW:ROUTE-B");
    expected["first_article"]["gate_id"] = json!("NEW:B-FIRST-ARTICLE");
    expected["changeovers"][0]["duration"]["source_ref"] = json!("NEW:setup-declaration");
    assert_eq!(serde_json::to_value(policy).unwrap(), expected);
    assert_eq!(serde_json::to_value(&c.example_project).unwrap(), before);
    assert_eq!(
        p.command_data(None)["project_setup_policies"],
        json!([expected])
    );
}

#[test]
fn existing_target_policies_remain_byte_equivalent_and_source_rules_append_once() {
    let c = candidate(true);
    let mut t = c.example_project.clone();
    t.id = "target".to_owned().try_into().unwrap();
    let original = serde_json::to_value(&t.setup_policies).unwrap();
    let p = prepare_template(&c, &t, &options(&c)).unwrap();
    let merged = p.project_setup_policies.as_ref().unwrap();
    assert_eq!(merged.len(), 2);
    assert_eq!(serde_json::to_value(&merged[..1]).unwrap(), original);
    assert_eq!(merged[1].device_id.as_str(), "NEW:Assembly");
    assert_eq!(p.proposed_project.setup_policies.len(), 1);
    assert_eq!(serde_json::to_value(&t.setup_policies).unwrap(), original);
}

#[test]
fn no_source_policy_keeps_legacy_json_shape_and_preserves_target_policy_by_none() {
    let mut c = candidate(false);
    let mut t = target();
    t.setup_policies = c.example_project.setup_policies.clone();
    c.example_project.setup_policies.clear();
    let p = prepare_template(&c, &t, &options(&c)).unwrap();
    assert!(p.project_setup_policies.is_none());
    assert!(p.command_data(None).get("project_setup_policies").is_none());
}

#[test]
fn explicit_worker_reuse_keeps_target_calendar_capacity_and_policy_resource_resolution() {
    let c = candidate(false);
    let mut t = target();
    let mut worker = c
        .example_project
        .resources
        .iter()
        .find(|r| r.id.as_str() == "W-A")
        .unwrap()
        .clone();
    worker.id = id("actual-worker");
    worker.qualifications.push("setup-certified".into());
    t.resources.push(worker.clone());
    t.calendars = c.example_project.calendars.clone();
    let mut choices = options(&c);
    choices.resources.insert(
        id("W-A"),
        TemplateResourceSelection::Reuse(worker.id.clone()),
    );
    let p = prepare_template(&c, &t, &choices).unwrap();
    let step = &p.proposed_project.routes[0].steps[0];
    assert_eq!(
        step.resource_bindings[&"worker".to_owned().try_into().unwrap()],
        worker.id
    );
    assert!(
        !p.proposed_project
            .resources
            .iter()
            .any(|r| r.id.as_str() == "NEW:W-A")
    );
    assert_eq!(
        serde_json::to_value(&t.resources[0]).unwrap(),
        serde_json::to_value(&worker).unwrap()
    );
}

#[test]
fn missing_mapping_qualification_calendar_or_exact_first_article_source_is_explicit() {
    let c = candidate(true);
    let mut choices = options(&c);
    choices.resources.remove(&id("W-A"));
    assert_eq!(
        prepare_template(&c, &target(), &choices).unwrap_err()[0].code,
        "template.setup_policy_resource"
    );
    let mut no_calendar = c.clone();
    no_calendar
        .example_project
        .resources
        .iter_mut()
        .find(|r| r.id.as_str() == "W-A")
        .unwrap()
        .calendar_id = id("missing-calendar");
    assert_eq!(
        prepare_template(&no_calendar, &target(), &options(&no_calendar)).unwrap_err()[0].code,
        "template.setup_policy_resource"
    );
    let mut c = c;
    c.example_project.setup_policies[0].changeovers[0].resources[0]
        .qualifications
        .push("setup-certified".into());
    assert_eq!(
        prepare_template(&c, &target(), &options(&c)).unwrap_err()[0].code,
        "template.setup_policy_resource"
    );
    c.example_project
        .resources
        .iter_mut()
        .find(|r| r.id.as_str() == "W-A")
        .unwrap()
        .qualifications
        .push("setup-certified".into());
    c.example_project.setup_policies[0]
        .first_article
        .as_mut()
        .unwrap()
        .inspection_module
        .version = "9.0.0".to_owned().try_into().unwrap();
    assert_eq!(
        prepare_template(&c, &target(), &options(&c)).unwrap_err()[0].code,
        "template.setup_policy_first_article_inspection"
    );
    c.example_project.setup_policies[0]
        .first_article
        .as_mut()
        .unwrap()
        .inspection_module
        .version = "0.1.0".to_owned().try_into().unwrap();
    let mut choices = options(&c);
    choices.include_example_product = false;
    assert_eq!(
        prepare_template(&c, &target(), &choices).unwrap_err()[0].code,
        "template.setup_policy_first_article_route"
    );
}

#[test]
fn policy_collisions_and_missing_duration_source_are_not_overwritten_or_cleared() {
    let mut c = candidate(false);
    let mut t = target();
    let mut conflicting = c.example_project.setup_policies[0].clone();
    conflicting.device_id = id("NEW:Assembly");
    t.setup_policies.push(conflicting);
    assert_eq!(
        prepare_template(&c, &t, &options(&c)).unwrap_err()[0].code,
        "template.setup_policy_conflict"
    );
    c.example_project.sources.clear();
    assert_eq!(
        prepare_template(&c, &target(), &options(&c)).unwrap_err()[0].code,
        "template.setup_policy_source"
    );
}

#[test]
fn route_override_precedes_device_binding_and_must_meet_policy_qualification() {
    let mut c = candidate(false);
    c.example_project
        .resources
        .iter_mut()
        .find(|r| r.id.as_str() == "W-B")
        .unwrap()
        .qualifications
        .push("setup-certified".into());
    c.example_project.setup_policies[0].changeovers[0].resources[0]
        .qualifications
        .push("setup-certified".into());
    for route in &mut c.example_project.routes {
        for step in &mut route.steps {
            if step.device_id.as_str() == "Assembly" {
                step.resource_bindings
                    .insert("worker".to_owned().try_into().unwrap(), id("W-B"));
            }
        }
    }
    assert!(prepare_template(&c, &target(), &options(&c)).is_ok());
    c.example_project.routes[0].steps[0]
        .resource_bindings
        .clear();
    assert_eq!(
        prepare_template(&c, &target(), &options(&c)).unwrap_err()[0].code,
        "template.setup_policy_resource"
    );
}

#[test]
fn represented_source_inspection_requires_exact_metadata_and_explicit_adoption() {
    let mut c = candidate(true);
    let scene: ScenarioInput = serde_json::from_value(fixture()["scenario"].clone()).unwrap();
    c.subset.seed = Some(TemplateSeed {
        initial_conditions: scene.initial_conditions.clone(),
        orders: scene.orders.clone(),
        arrivals: scene.arrivals.clone(),
    });
    c.subset.adoption.as_mut().unwrap().inspection_presets = scene
        .quality_presets
        .iter()
        .map(|p| TemplateInspectionPreset {
            id: p.id.clone(),
            consumer_device_id: id("Inspection"),
            inspection_module: p.inspection_module.clone(),
            item_or_ancestor: Some(p.item_or_ancestor.clone()),
            match_kind: Some(p.match_kind.clone()),
            results: p.results.clone(),
        })
        .collect();
    c.example_project.template_adoptions = vec![c.subset.adoption.as_ref().unwrap().clone()];
    let source_before = serde_json::to_value(&scene).unwrap();
    let context = template_seed_context_from_scenario(&c, &scene).unwrap();
    let plan = prepare_template(&c, &target(), &options(&c)).unwrap();
    let mut choices = TemplateSeedChoices {
        scenario_id: "checked-quality-seed".to_owned().try_into().unwrap(),
        supply: true,
        demand: true,
        initial_quality: scene
            .initial_conditions
            .items
            .iter()
            .chain(scene.arrivals.iter().flat_map(|a| &a.items))
            .map(|i| (i.id.clone(), QualityState::Unchecked))
            .collect(),
        adopt_declared_inspection_results: false,
    };
    let without = prepare_template_seed_with_context(&c, &plan, &choices, Some(&context)).unwrap();
    assert!(without.quality_presets.is_empty());
    assert!(
        without
            .initial_conditions
            .declared_quality_decisions
            .is_empty()
    );
    assert!(
        without
            .initial_conditions
            .items
            .iter()
            .chain(without.arrivals.iter().flat_map(|a| &a.items))
            .all(|i| matches!(i.quality, QualityState::Unchecked))
    );
    choices.adopt_declared_inspection_results = true;
    let with = prepare_template_seed_with_context(&c, &plan, &choices, Some(&context)).unwrap();
    assert_eq!(with.quality_presets.len(), scene.quality_presets.len());
    for (mapped, original) in with.quality_presets.iter().zip(&scene.quality_presets) {
        assert_eq!(mapped.id, plan.id_map[&original.id]);
        assert_eq!(
            serde_json::to_value(&mapped.inspection_module).unwrap(),
            serde_json::to_value(&original.inspection_module).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&mapped.results).unwrap(),
            serde_json::to_value(&original.results).unwrap()
        );
        assert_eq!(
            mapped.item_or_ancestor.as_str(),
            format!(
                "checked-quality-seed:seed:{}",
                original.item_or_ancestor.as_str()
            )
        );
    }
    for field in ["id", "item", "match", "module", "results"] {
        let mut changed = scene.clone();
        let preset = &mut changed.quality_presets[0];
        match field {
            "id" => preset.id = id("another-preset"),
            "item" => preset.item_or_ancestor = id("another-item"),
            "match" => preset.match_kind = QualityMatchKind::ExactItem,
            "module" => preset.inspection_module.version = "9.0.0".to_owned().try_into().unwrap(),
            "results" => preset.results = vec![None],
            _ => unreachable!(),
        }
        assert_eq!(
            template_seed_context_from_scenario(&c, &changed).unwrap_err()[0].code,
            "template.seed_context_unrepresented",
            "mismatched {field} cannot be discarded or synthesized"
        );
    }
    assert_eq!(serde_json::to_value(&scene).unwrap(), source_before);
}

#[test]
fn mapped_policy_inputs_are_saved_for_the_single_engine_consumer_without_making_a_host() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/模板与交接/runtime/setup-composition");
    std::fs::create_dir_all(&root).unwrap();
    for (name, pending, external) in [
        ("internal-pass", false, false),
        ("external-pass", false, true),
        ("internal-pending", true, false),
    ] {
        let mut c = candidate(true);
        let mut scene: ScenarioInput =
            serde_json::from_value(fixture()["scenario"].clone()).unwrap();
        scene.orders.retain(|o| o.id.as_str() == "B");
        scene.production_tasks.retain(|t| t.id.as_str() == "TASK-B");
        if external {
            c.example_project.setup_policies[0].changeovers[0].preparation_mode =
                PreparationMode::External;
        }
        if pending {
            scene.quality_presets[0].results = vec![None];
        }
        c.subset.seed = Some(TemplateSeed {
            initial_conditions: scene.initial_conditions.clone(),
            orders: scene.orders.clone(),
            arrivals: scene.arrivals.clone(),
        });
        c.subset.adoption.as_mut().unwrap().inspection_presets = scene
            .quality_presets
            .iter()
            .map(|p| TemplateInspectionPreset {
                id: p.id.clone(),
                consumer_device_id: id("Inspection"),
                inspection_module: p.inspection_module.clone(),
                item_or_ancestor: Some(p.item_or_ancestor.clone()),
                match_kind: Some(p.match_kind.clone()),
                results: p.results.clone(),
            })
            .collect();
        c.example_project.template_adoptions = vec![c.subset.adoption.as_ref().unwrap().clone()];
        // The target is an existing, idle typed plant. Its policies/resources stay
        // present and one worker is explicitly reused by the new composition.
        let t = c.example_project.clone();
        let mut choices = options(&c);
        choices
            .resources
            .insert(id("W-A"), TemplateResourceSelection::Reuse(id("W-A")));
        let p = prepare_template(&c, &t, &choices).unwrap();
        let ctx = template_seed_context_from_scenario(&c, &scene).unwrap();
        let seed = prepare_template_seed_with_context(
            &c,
            &p,
            &TemplateSeedChoices {
                scenario_id: format!("{name}-seed").try_into().unwrap(),
                supply: true,
                demand: true,
                initial_quality: scene
                    .initial_conditions
                    .items
                    .iter()
                    .chain(scene.arrivals.iter().flat_map(|a| &a.items))
                    .map(|i| (i.id.clone(), i.quality.clone()))
                    .collect(),
                adopt_declared_inspection_results: true,
            },
            Some(&ctx),
        )
        .unwrap();
        assert_ne!(
            seed.orders[0].id, seed.orders[0].product_ref.id,
            "source Order B and Product B need distinct maps"
        );
        assert!(
            !seed.supplies.is_empty() && seed.scope.is_empty(),
            "explicit supply must execute under the established whole-Project scope, not be silently excluded"
        );
        let mut merged = t.clone();
        macro_rules! append{($($field:ident),*)=>{$(merged.$field.extend(p.proposed_project.$field.clone());)*};}
        append!(
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
            connections,
            sources,
            parameter_sources,
            template_adoptions
        );
        merged.setup_policies = p.project_setup_policies.clone().unwrap();
        for reference in &p.proposed_project.module_refs {
            if !merged
                .module_refs
                .iter()
                .any(|r| r.id == reference.id && r.version == reference.version)
            {
                merged.module_refs.push(reference.clone());
            }
        }
        merged.scenario_refs = vec![seed.id.clone()];
        std::fs::write(root.join(format!("{name}.input.json")),serde_json::to_vec_pretty(&json!({"project":merged,"scenario":seed,"definitions":c.definitions,"command":p.command_data(None)})).unwrap()).unwrap();
        assert_eq!(p.project_setup_policies.as_ref().unwrap().len(), 2);
    }
}
