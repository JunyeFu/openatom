//! Independent original-template contracts; Host and Engine consumers live in commands tests.
use serde_json::{Value, json};
use sf_domain::*;
use sf_module_kit::*;
use std::collections::{BTreeMap, BTreeSet};
fn id(s: &str) -> ObjectId {
    ObjectId::try_from(s.to_owned()).unwrap()
}
fn baseline() -> ProjectState {
    let v: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap();
    serde_json::from_value(v["project"].clone()).unwrap()
}
fn choices(c: &TemplateCandidate, namespace: &str, product: bool) -> TemplateChoices {
    TemplateChoices {
        namespace: namespace.into(),
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
        include_example_product: product,
    }
}
fn prepare(c: &TemplateCandidate, namespace: &str, product: bool) -> PreparedTemplate {
    prepare_template(c, &baseline(), &choices(c, namespace, product)).unwrap()
}
fn seeds(
    c: &TemplateCandidate,
    p: &PreparedTemplate,
    supply: bool,
    demand: bool,
) -> Result<ScenarioInput, Vec<Issue>> {
    prepare_template_seed(
        c,
        p,
        &TemplateSeedChoices {
            scenario_id: ScenarioId::try_from(format!(
                "{}-explicit-seed",
                p.id_map.values().next().unwrap().as_str()
            ))
            .unwrap(),
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
                .map(|stock| (stock.location_id.clone(), QualityState::Released))
                .collect(),
            adopt_declared_inspection_results: supply,
        },
    )
}
fn all() -> [BuiltinTemplate; 4] {
    [
        BuiltinTemplate::MachiningDelivery,
        BuiltinTemplate::PurchasedAssembly,
        BuiltinTemplate::ParallelElectromechanical,
        BuiltinTemplate::CartonFactory,
    ]
}
#[test]
fn all_three_templates_and_carton_configuration_are_typed_editable_candidates_without_implicit_stock_orders_or_authority()
 {
    let reference = baseline().time_reference;
    let empty = empty_template_project(
        ProjectId::try_from("W5-EMPTY".to_owned()).unwrap(),
        "explicit empty entry".into(),
        reference.clone(),
    );
    let empty = serde_json::to_value(empty).unwrap();
    assert_eq!(
        empty["time_reference"],
        serde_json::to_value(reference).unwrap()
    );
    for field in [
        "item_types",
        "locations",
        "halls",
        "lines",
        "devices",
        "resources",
        "calendars",
        "products",
        "boms",
        "routes",
        "connections",
        "scenario_refs",
        "artifact_refs",
        "acknowledgments",
        "bindings",
    ] {
        assert!(
            empty[field].as_array().unwrap().is_empty(),
            "empty entry must not invent {field}"
        );
    }
    assert!(empty["generation_context"].is_null());
    for (i, which) in all().into_iter().enumerate() {
        let c = builtin_template(which).unwrap();
        assert!(matches!(
            c.package.definition.payload,
            ModulePayload::Templates(_)
        ));
        assert!(c.subset.seed.is_none());
        let before = serde_json::to_value(&c.example_project).unwrap();
        let p = prepare(&c, &format!("w5-{i}"), false);
        assert_eq!(serde_json::to_value(&c.example_project).unwrap(), before);
        let command = p.command_data(None);
        assert!(command["explicit_new_scenario_seed"].is_null());
        assert_eq!(command["subset"], serde_json::to_value(&c.subset).unwrap());
        assert!(
            command["proposed_objects"]
                .as_array()
                .unwrap()
                .iter()
                .all(|o| o["kind"] != "scenario")
        );
        assert!(p.proposed_project.artifact_refs.is_empty());
        assert!(p.proposed_project.acknowledgments.is_empty());
        assert!(p.proposed_project.bindings.is_empty());
        assert!(p.proposed_project.generation_context.is_none());
        assert!(!p.proposed_project.devices.is_empty());
    }
}
#[test]
fn copying_twice_creates_disjoint_ids_and_rewrites_internal_hall_line_location_port_and_connection_references()
 {
    let c = builtin_template(BuiltinTemplate::ParallelElectromechanical).unwrap();
    let target = baseline();
    let before = serde_json::to_value(&target).unwrap();
    let a = prepare_template(&c, &target, &choices(&c, "w5-copy-a", true)).unwrap();
    let b = prepare_template(&c, &target, &choices(&c, "w5-copy-b", true)).unwrap();
    assert_eq!(serde_json::to_value(&target).unwrap(), before);
    let ai: BTreeSet<_> = a.id_map.values().collect();
    let bi: BTreeSet<_> = b.id_map.values().collect();
    assert!(ai.is_disjoint(&bi));
    for p in [&a, &b] {
        for (old, new) in &p.id_map {
            assert_ne!(old, new);
        }
        let halls: BTreeSet<_> = p.proposed_project.halls.iter().map(|h| &h.id).collect();
        let devices: BTreeSet<_> = p.proposed_project.devices.iter().map(|d| &d.id).collect();
        for l in &p.proposed_project.lines {
            assert!(halls.contains(&l.hall_id));
            for member in &l.members {
                assert!(devices.contains(member));
            }
        }
        for l in &p.proposed_project.locations {
            assert!(halls.contains(l.hall_id.as_ref().unwrap()));
        }
        for d in &p.proposed_project.devices {
            assert!(halls.contains(&d.hall_id));
        }
        let v = serde_json::to_value(&p.proposed_project).unwrap();
        for connection in v["connections"].as_array().unwrap() {
            let data = &connection["data"];
            for endpoint in ["source", "destination"] {
                if data[endpoint]["kind"] == "location" {
                    assert!(
                        p.proposed_project
                            .locations
                            .iter()
                            .any(|l| json!(l.id) == data[endpoint]["data"])
                    );
                }
                if data[endpoint]["kind"] == "port" {
                    assert!(
                        p.id_map
                            .values()
                            .any(|i| json!(i) == data[endpoint]["data"]["instance_id"])
                    );
                }
            }
        }
        assert!(
            p.proposed_project
                .halls
                .iter()
                .all(|h| h.height_m.is_none())
        );
    }
}
#[test]
fn inserting_without_explicit_product_and_seed_never_copies_orders_inventory_runs_or_human_approvals()
 {
    let c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let p = prepare(&c, "w5-no-seed", false);
    assert!(p.proposed_project.products.is_empty());
    assert!(p.proposed_project.boms.is_empty());
    assert!(p.proposed_project.routes.is_empty());
    let command = p.command_data(None);
    assert!(command["explicit_new_scenario_seed"].is_null());
    let text = command.to_string();
    for forbidden in [
        "resource_claims",
        "final_items",
        "acceptance_events",
        "run_id",
        "record_artifact_acknowledgment",
    ] {
        assert!(!text.contains(forbidden));
    }
}
#[test]
fn supply_and_demand_are_separate_explicit_seed_choices_and_full_seed_preserves_original_quantities()
 {
    for (which, counts) in [
        (
            BuiltinTemplate::MachiningDelivery,
            vec![("blank", 2), ("carton", 2)],
        ),
        (
            BuiltinTemplate::PurchasedAssembly,
            vec![("shell", 2), ("pcb", 2), ("fastener", 8), ("carton", 2)],
        ),
        (
            BuiltinTemplate::ParallelElectromechanical,
            vec![
                ("blank", 2),
                ("bare_pcb", 2),
                ("component", 16),
                ("fastener", 8),
                ("carton", 2),
            ],
        ),
    ] {
        let c = builtin_template(which).unwrap();
        let p = prepare(&c, "w5-explicit", true);
        let supply = seeds(&c, &p, true, false).unwrap();
        assert!(supply.orders.is_empty());
        assert!(!supply.initial_conditions.items.is_empty());
        let demand = seeds(&c, &p, false, true).unwrap();
        assert!(demand.initial_conditions.items.is_empty());
        assert!(!demand.orders.is_empty());
        let full = seeds(&c, &p, true, true).unwrap();
        let fullv = serde_json::to_value(&full).unwrap();
        for (item, quantity) in counts {
            let mapped = p.id_map.get(&id(item)).unwrap();
            let total: u64 = fullv["initial_conditions"]["items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["item_type_ref"]["id"] == json!(mapped))
                .map(|i| i["quantity"]["value"]["coefficient"].as_u64().unwrap())
                .sum();
            assert_eq!(total, quantity);
        }
        assert_eq!(fullv["orders"][0]["quantity"]["value"]["coefficient"], 2);
        assert!(
            full.initial_conditions
                .declared_quality_decisions
                .is_empty()
        );
    }
}
#[test]
fn explicitly_reused_actual_worker_is_not_duplicated_and_undersized_or_wrong_unit_resource_is_not_silently_accepted()
 {
    let c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let source = c
        .example_project
        .resources
        .iter()
        .find(|r| matches!(r.kind, ResourceKind::Worker))
        .unwrap();
    let mut target = baseline();
    let mut actual = source.clone();
    actual.id = id("w5-existing-worker");
    actual.home_location = None;
    actual.calendar_id = target.calendars[0].id.clone();
    target.resources.push(actual.clone());
    let mut choice = choices(&c, "w5-reuse", true);
    choice.resources.insert(
        source.id.clone(),
        TemplateResourceSelection::Reuse(actual.id.clone()),
    );
    let p = prepare_template(&c, &target, &choice).unwrap();
    assert!(
        p.proposed_project
            .resources
            .iter()
            .all(|r| r.id != actual.id)
    );
    assert!(
        p.proposed_project.devices.iter().any(|d| d
            .role_bindings
            .resources
            .values()
            .any(|r| r == &actual.id))
    );
    for bad_unit in [false, true] {
        let mut bad = target.clone();
        let r = bad
            .resources
            .iter_mut()
            .find(|r| r.id == actual.id)
            .unwrap();
        if bad_unit {
            r.capacity.unit = UnitId::try_from("slot".to_owned()).unwrap();
        } else {
            r.capacity.value.coefficient = SafeUint::try_from(0u64).unwrap();
        }
        assert!(prepare_template(&c, &bad, &choice).is_err());
    }
}
#[test]
fn missing_explicit_resource_choice_and_colliding_new_ids_are_located_instead_of_creating_implicit_people()
 {
    let c = builtin_template(BuiltinTemplate::MachiningDelivery).unwrap();
    let mut choice = choices(&c, "w5-missing", true);
    choice.resources.clear();
    let result = prepare_template(&c, &baseline(), &choice);
    assert!(
        result.is_err()
            || result
                .as_ref()
                .unwrap()
                .issues
                .iter()
                .any(|i| !i.blocks.is_empty())
    );
    let mut target = baseline();
    let p = prepare(&c, "w5-collision", true);
    target.devices.extend(p.proposed_project.devices);
    assert!(prepare_template(&c, &target, &choices(&c, "w5-collision", true)).is_err());
}
#[test]
fn all_nine_categories_have_editable_adoptable_samples_without_inventing_a_twelve_object_threshold_or_provider_success()
 {
    let mut categories = BTreeSet::new();
    for p in builtin_template_support().unwrap() {
        categories.insert(
            serde_json::to_value(&p.definition).unwrap()["payload"]["category"]
                .as_str()
                .unwrap()
                .to_owned(),
        );
    }
    for which in all() {
        let c = builtin_template(which).unwrap();
        categories.insert("templates".into());
        for d in c.definitions {
            categories.insert(
                serde_json::to_value(d).unwrap()["payload"]["category"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
            );
        }
        assert!(c.example_project.artifact_refs.is_empty());
    }
    assert_eq!(
        categories,
        BTreeSet::from_iter(
            [
                "items",
                "devices",
                "resources",
                "tasks",
                "operations",
                "transport",
                "templates",
                "professional",
                "media"
            ]
            .into_iter()
            .map(str::to_owned)
        )
    );
}
#[test]
fn carton_profile_keeps_carton_manufacturing_and_actual_carton_acceptance_distinct_from_packing_finished_products()
 {
    let profile = serde_json::to_value(carton_generation_profile().unwrap()).unwrap();
    assert!(profile.to_string().contains("carton"));
    let c = builtin_template(BuiltinTemplate::CartonFactory).unwrap();
    let mut p = prepare(&c, "w5-carton", true);
    let products = serde_json::to_value(&p.proposed_project.products).unwrap();
    assert!(products.as_array().unwrap().iter().any(|pr| {
        pr["item_type_ref"]["id"]
            .as_str()
            .is_some_and(|id| id.ends_with(":carton"))
    }));
    let site:SiteEnvelope=serde_json::from_value(json!({"regions":[],"length_m":31,"width_m":13,"height_m":5,"usable_polygon_m":[],"clearance_m":0})).unwrap();
    fill_template_site(&mut p, &site).unwrap();
    for h in &p.proposed_project.halls {
        assert_eq!(h.width_m.value(), 31.);
        assert_eq!(h.depth_m.value(), 13.);
        assert_eq!(h.height_m.as_ref().unwrap().value(), 5.);
    }
    assert!(p.proposed_project.generation_context.is_none());
    assert!(p.command_data(None)["explicit_new_scenario_seed"].is_null());
}
