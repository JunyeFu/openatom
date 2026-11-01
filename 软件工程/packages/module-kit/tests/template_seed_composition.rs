use serde_json::{Value, json};
use sf_domain::*;
use sf_module_kit::*;
use std::collections::BTreeMap;

fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn options(c: &TemplateCandidate) -> TemplateChoices {
    TemplateChoices {
        namespace: "COMPOSE".into(),
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
        "composition-target".to_owned().try_into().unwrap(),
        "Composition target".into(),
        TimeReference {
            label: "scenario_start".into(),
            origin_iso8601: None,
            timezone: None,
        },
    )
}
fn source() -> (TemplateCandidate, TemplateSeedContext, TemplateSeedChoices) {
    let mut c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let deferred = c
        .subset
        .adoption
        .as_ref()
        .unwrap()
        .deferred_seed
        .as_ref()
        .unwrap()
        .clone();
    let product = c.example_project.products[0].clone();
    let items = deferred
        .initial_stock
        .iter()
        .enumerate()
        .map(|(i, row)| InitialItem {
            id: id(&format!("item-{i}")),
            item_type_ref: SubjectRevisionRef {
                id: row.item_type_id.clone(),
                revision: c
                    .example_project
                    .item_types
                    .iter()
                    .find(|t| t.id == row.item_type_id)
                    .unwrap()
                    .revision
                    .clone(),
            },
            quantity: row.quantity.clone(),
            holder: HolderRef::Location(row.location_id.clone()),
            quality: QualityState::Unchecked,
            components: vec![],
            contents: vec![],
            source: SourceBasis::TemplateDeclaredExample,
        })
        .collect::<Vec<_>>();
    let mut arrival_item = items[0].clone();
    arrival_item.id = id("arrival-shell");
    arrival_item.quantity = serde_json::from_value(
        json!({"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}),
    )
    .unwrap();
    let orders = (0..2)
        .map(|i| OrderLine {
            id: id(&format!("order-{i}")),
            product_ref: SubjectRevisionRef {
                id: product.id.clone(),
                revision: product.revision.clone(),
            },
            quantity: Some(
                serde_json::from_value(
                    json!({"value":{"coefficient":1,"decimal_places":0},"unit":"piece"}),
                )
                .unwrap(),
            ),
            release_at_ms: SimTimeMs((i * 80_000u64).try_into().unwrap()),
            due_at_ms: if i == 0 {
                Some(SimTimeMs(210_000u64.try_into().unwrap()))
            } else {
                None
            },
            destination: deferred.demand.destination_location_id.clone(),
            acceptance_stage: deferred.demand.acceptance_stage.clone(),
            allow_partial: i == 0,
            priority: i as i32,
        })
        .collect::<Vec<_>>();
    let tasks = orders
        .iter()
        .enumerate()
        .map(|(i, o)| ProductionTask {
            id: id(&format!("task-{i}")),
            routing_ref: product.routing_ref.clone(),
            order_allocations: vec![OrderAllocation {
                order_id: o.id.clone(),
                quantity: o.quantity.clone().unwrap(),
            }],
            planned_quantity: o.quantity.clone().unwrap(),
            release_at_ms: o.release_at_ms.clone(),
            priority: o.priority,
        })
        .collect();
    let mut later_item = arrival_item.clone();
    later_item.id = id("arrival-shell-later");
    let seed = TemplateSeed {
        initial_conditions: InitialConditions {
            items,
            initial_setup_families: BTreeMap::new(),
            material_allocations: vec![],
            declared_quality_decisions: vec![],
        },
        orders,
        arrivals: vec![
            ArrivalPlan {
                id: id("arrival-1"),
                at_ms: SimTimeMs(20_000u64.try_into().unwrap()),
                destination: deferred.initial_stock[0].location_id.clone(),
                items: vec![arrival_item],
                release_policy: ReleasePolicy::UncheckedAllowed,
            },
            ArrivalPlan {
                id: id("arrival-2"),
                at_ms: SimTimeMs(30_000u64.try_into().unwrap()),
                destination: deferred.initial_stock[0].location_id.clone(),
                items: vec![later_item],
                release_policy: ReleasePolicy::UncheckedAllowed,
            },
        ],
    };
    let qualities = seed
        .initial_conditions
        .items
        .iter()
        .chain(seed.arrivals.iter().flat_map(|a| &a.items))
        .map(|i| (i.id.clone(), QualityState::Released))
        .collect();
    c.subset.seed = Some(seed);
    let adoption = c.subset.adoption.as_mut().unwrap();
    adoption.deferred_seed = None;
    adoption.inspection_presets.clear();
    c.example_project.template_adoptions = vec![adoption.clone()];
    let ctx = TemplateSeedContext {
        time_reference: deferred.time_reference,
        window: deferred.window,
        production_tasks: tasks,
        supplies: vec![],
    };
    let ch = TemplateSeedChoices {
        scenario_id: "new-seed".to_owned().try_into().unwrap(),
        supply: true,
        demand: true,
        initial_quality: qualities,
        adopt_declared_inspection_results: false,
    };
    (c, ctx, ch)
}
fn plan(c: &TemplateCandidate) -> PreparedTemplate {
    prepare_template(c, &target(), &options(c)).unwrap()
}
fn output(
    c: &TemplateCandidate,
    context: &TemplateSeedContext,
    choices: &TemplateSeedChoices,
) -> ScenarioInput {
    prepare_template_seed_with_context(c, &plan(c), choices, Some(context)).unwrap()
}

#[test]
fn full_seed_requires_actual_context_instead_of_a_deferred_first_product_fallback() {
    let (c, _, ch) = source();
    assert_eq!(
        prepare_template_seed(&c, &plan(&c), &ch).unwrap_err()[0].code,
        "template.seed_context_required"
    );
    assert!(plan(&c).command_data(None)["explicit_new_scenario_seed"].is_null());
}

#[test]
fn multi_orders_arrivals_quantities_times_and_declared_tasks_are_preserved_with_new_ids() {
    let (c, ctx, ch) = source();
    let before = serde_json::to_value(&c.subset).unwrap();
    let s = output(&c, &ctx, &ch);
    let src = c.subset.seed.as_ref().unwrap();
    assert_eq!(s.orders.len(), 2);
    assert_eq!(s.production_tasks.len(), 2);
    assert_eq!(s.arrivals.len(), 2);
    for (new, old) in s.orders.iter().zip(&src.orders) {
        assert_ne!(new.id, old.id);
        assert_eq!(
            new.id.as_str(),
            format!("new-seed:seed:{}", old.id.as_str())
        );
        assert_eq!(
            serde_json::to_value(&new.quantity).unwrap(),
            serde_json::to_value(&old.quantity).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&new.due_at_ms).unwrap(),
            serde_json::to_value(&old.due_at_ms).unwrap()
        );
        assert_eq!(new.release_at_ms.0.value(), old.release_at_ms.0.value());
        assert_eq!(new.priority, old.priority);
        assert_eq!(new.allow_partial, old.allow_partial);
    }
    assert_eq!(s.arrivals[0].at_ms.0.value(), 20_000);
    assert_eq!(s.arrivals[0].destination.as_str(), "COMPOSE:store-shell");
    assert_eq!(
        s.arrivals[0].items[0].id.as_str(),
        "new-seed:seed:arrival-shell"
    );
    assert_eq!(
        s.production_tasks[1].order_allocations[0].order_id,
        s.orders[1].id
    );
    assert_eq!(serde_json::to_value(&c.subset).unwrap(), before);
    assert!(s.parameter_overrides.is_empty() && s.cost_inputs.is_empty());
}

#[test]
fn distinct_products_keep_their_exact_identity_instead_of_becoming_the_first_product() {
    let (mut c, ctx, ch) = source();
    let mut second = c.example_project.products[0].clone();
    second.id = id("actual-product-two");
    c.subset.seed.as_mut().unwrap().orders[1].product_ref.id = second.id.clone();
    c.example_project.products.push(second);
    let s = output(&c, &ctx, &ch);
    assert_ne!(s.orders[0].product_ref.id, s.orders[1].product_ref.id);
    assert_eq!(
        s.orders[1].product_ref.id.as_str(),
        "COMPOSE:actual-product-two"
    );
}

#[test]
fn independent_supply_demand_choices_never_copy_unselected_seed_data() {
    let (c, ctx, mut ch) = source();
    for (supply, demand) in [(false, false), (true, false), (false, true), (true, true)] {
        ch.supply = supply;
        ch.demand = demand;
        let s = output(&c, &ctx, &ch);
        assert_eq!(!s.initial_conditions.items.is_empty(), supply);
        assert_eq!(!s.arrivals.is_empty(), supply);
        assert_eq!(!s.orders.is_empty(), demand);
        assert_eq!(!s.production_tasks.is_empty(), demand);
    }
}

#[test]
fn same_location_items_require_separate_quality_choices_and_keep_links_sources_and_reuse() {
    let (mut c, ctx, mut ch) = source();
    let mut extra = c.subset.seed.as_ref().unwrap().initial_conditions.items[0].clone();
    extra.id = id("second-shell");
    c.subset
        .seed
        .as_mut()
        .unwrap()
        .initial_conditions
        .items
        .push(extra);
    ch.initial_quality.remove(&id("item-0"));
    ch.initial_quality
        .insert(id("store-shell"), QualityState::Released);
    assert_eq!(
        prepare_template_seed_with_context(&c, &plan(&c), &ch, Some(&ctx)).unwrap_err()[0].code,
        "template.initial_quality"
    );
    ch.initial_quality
        .insert(id("item-0"), QualityState::Released);
    ch.initial_quality
        .insert(id("second-shell"), QualityState::Unchecked);
    let s = output(&c, &ctx, &ch);
    assert!(matches!(
        s.initial_conditions.items.last().unwrap().quality,
        QualityState::Unchecked
    ));
    assert_eq!(
        serde_json::to_value(&s.initial_conditions.items[0].source).unwrap(),
        "template_declared_example"
    );
}

#[test]
fn missing_declared_task_and_competing_summary_are_blocking_instead_of_synthesized() {
    let (mut c, mut ctx, ch) = source();
    ctx.production_tasks.pop();
    assert_eq!(
        prepare_template_seed_with_context(&c, &plan(&c), &ch, Some(&ctx)).unwrap_err()[0].code,
        "template.seed_production_task"
    );
    c.subset.adoption.as_mut().unwrap().deferred_seed =
        builtin_template(BuiltinTemplate::PurchasedAssembly)
            .unwrap()
            .subset
            .adoption
            .unwrap()
            .deferred_seed;
    assert_eq!(
        prepare_template_seed(&c, &plan(&c), &ch).unwrap_err()[0].code,
        "template.seed_competing_sources"
    );
}

#[test]
fn allocations_bind_exact_declared_tasks_and_never_copy_committed_work_in_progress() {
    let (mut c, ctx, mut ch) = source();
    let seed = c.subset.seed.as_mut().unwrap();
    seed.initial_conditions.material_allocations = vec![MaterialAllocation {
        id: id("reservation"),
        demand_ref: ctx.production_tasks[0].id.clone(),
        item_refs: vec![seed.initial_conditions.items[0].id.clone()],
        quantity: seed.orders[0].quantity.clone().unwrap(),
        mode: AllocationMode::HardReserved,
    }];
    let s = output(&c, &ctx, &ch);
    assert_eq!(
        s.initial_conditions.material_allocations[0].demand_ref,
        s.production_tasks[0].id
    );
    assert_eq!(
        s.initial_conditions.material_allocations[0].item_refs[0],
        s.initial_conditions.items[0].id
    );
    ch.demand = false;
    assert_eq!(
        prepare_template_seed_with_context(&c, &plan(&c), &ch, Some(&ctx)).unwrap_err()[0].code,
        "template.seed_allocation_task"
    );
    ch.demand = true;
    c.subset
        .seed
        .as_mut()
        .unwrap()
        .initial_conditions
        .material_allocations[0]
        .mode = AllocationMode::Committed;
    assert_eq!(
        prepare_template_seed_with_context(&c, &plan(&c), &ch, Some(&ctx)).unwrap_err()[0].code,
        "template.seed_committed_allocation"
    );
}

#[test]
fn runtime_holders_and_unadoptable_setup_policy_do_not_silently_survive() {
    let (mut c, ctx, ch) = source();
    c.subset.seed.as_mut().unwrap().initial_conditions.items[0].holder =
        HolderRef::Job(id("old-runtime-job"));
    assert_eq!(
        prepare_template_seed_with_context(&c, &plan(&c), &ch, Some(&ctx)).unwrap_err()[0].code,
        "template.seed_runtime_holder"
    );
    // A fully mapped policy is now supported. Keep the original rejection for
    // a truly unadoptable policy; the new setup suite proves the positive path.
    c.example_project.setup_policies.push(serde_json::from_value(json!({"device_id":"missing-source-device","initial_family":"a","changeovers":[],"first_article":null})).unwrap());
    assert_eq!(
        prepare_template(&c, &target(), &options(&c)).unwrap_err()[0].code,
        "template.setup_policy_adoption"
    );
}

#[test]
fn canonical_seed_survives_real_file_move_reopen_without_overwriting_old_input() {
    let (c, ctx, ch) = source();
    let plan = plan(&c);
    let s = prepare_template_seed_with_context(&c, &plan, &ch, Some(&ctx)).unwrap();
    let original = serde_json::to_value(&s).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/模板与交接/runtime/seed-composition");
    std::fs::create_dir_all(&root).unwrap();
    let before = root.join("seed-before-move.json");
    let after = root.join("seed-reopened.json");
    let bytes = serde_json::to_vec_pretty(&Versioned {
        schema_version: SchemaVersion::V1,
        data: s,
    })
    .unwrap();
    std::fs::write(&before, &bytes).unwrap();
    std::fs::rename(&before, &after).unwrap();
    let mut reopened: Versioned<ScenarioInput> =
        serde_json::from_slice(&std::fs::read(&after).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&reopened.data).unwrap(), original);
    reopened.data.orders[0].priority += 1;
    assert_ne!(serde_json::to_value(&reopened.data).unwrap(), original);
    std::fs::write(
        root.join("prepared-project.json"),
        serde_json::to_vec_pretty(&plan.proposed_project).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("definitions.json"),
        serde_json::to_vec_pretty(&c.definitions).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("instantiate.json"),
        serde_json::to_vec_pretty(
            &plan.command_data_with_preparation(
                &"example-project.json".to_owned().try_into().unwrap(),
                &options(&c),
                None,
                Some(
                    serde_json::from_value::<Versioned<ScenarioInput>>(
                        serde_json::from_slice::<Value>(&bytes).unwrap(),
                    )
                    .unwrap()
                    .data,
                ),
            ),
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn declared_quality_selection_preserves_target_links_and_never_forges_tool_evidence() {
    let (mut c, ctx, mut ch) = source();
    let mut preset = builtin_template(BuiltinTemplate::PurchasedAssembly)
        .unwrap()
        .subset
        .adoption
        .unwrap()
        .inspection_presets
        .remove(0);
    preset.item_or_ancestor = Some(id("item-0"));
    preset.match_kind = Some(QualityMatchKind::Ancestor);
    let decision = QualityDecision {
        id: id("declared-inspection"),
        item_ref: id("item-0"),
        inspection_ref: preset.inspection_module.clone(),
        source_kind: QualitySourceKind::Preset,
        observed_result: None,
        disposition: QualityDisposition::Pending,
        effective_at_ms: SimTimeMs(10_000u64.try_into().unwrap()),
        decision_source: DecisionSource::ScheduledPreset(preset.id.clone()),
    };
    c.subset.adoption.as_mut().unwrap().inspection_presets = vec![preset];
    c.subset
        .seed
        .as_mut()
        .unwrap()
        .initial_conditions
        .declared_quality_decisions = vec![decision];
    let disabled = output(&c, &ctx, &ch);
    assert!(
        disabled.quality_presets.is_empty()
            && disabled
                .initial_conditions
                .declared_quality_decisions
                .is_empty()
    );
    ch.adopt_declared_inspection_results = true;
    let s = output(&c, &ctx, &ch);
    assert_eq!(
        s.quality_presets[0].item_or_ancestor,
        s.initial_conditions.items[0].id
    );
    assert_eq!(
        s.initial_conditions.declared_quality_decisions[0].item_ref,
        s.initial_conditions.items[0].id
    );
    if let DecisionSource::ScheduledPreset(reference) =
        &s.initial_conditions.declared_quality_decisions[0].decision_source
    {
        assert_eq!(reference, &s.quality_presets[0].id);
    } else {
        panic!("must preserve declared preset source")
    }
    c.subset
        .seed
        .as_mut()
        .unwrap()
        .initial_conditions
        .declared_quality_decisions[0]
        .source_kind = QualitySourceKind::ToolResult;
    assert_eq!(
        prepare_template_seed_with_context(&c, &plan(&c), &ch, Some(&ctx)).unwrap_err()[0].code,
        "template.seed_quality_evidence"
    );
}

#[test]
fn container_component_and_initial_family_links_remap_without_runtime_stock() {
    let (mut c, ctx, ch) = source();
    let seed = c.subset.seed.as_mut().unwrap();
    seed.initial_conditions.items[0].holder = HolderRef::Container(id("item-3"));
    seed.initial_conditions.items[3].contents = vec![id("item-0")];
    seed.initial_conditions.items[2].components = vec![id("item-1")];
    seed.initial_conditions.initial_setup_families.insert(
        c.example_project.devices[0].id.clone(),
        "declared-family".into(),
    );
    let s = output(&c, &ctx, &ch);
    assert!(
        matches!(&s.initial_conditions.items[0].holder,HolderRef::Container(id) if *id==s.initial_conditions.items[3].id)
    );
    assert_eq!(
        s.initial_conditions.items[3].contents[0],
        s.initial_conditions.items[0].id
    );
    assert_eq!(
        s.initial_conditions.items[2].components[0],
        s.initial_conditions.items[1].id
    );
    assert!(
        s.initial_conditions
            .initial_setup_families
            .keys()
            .all(|id| id.as_str().starts_with("COMPOSE:"))
    );
}

#[test]
fn declared_supply_uses_explicit_resource_reuse_and_keeps_transport_windows_and_version() {
    let (mut c, mut ctx, ch) = source();
    let mut target = target();
    let mut choices = options(&c);
    let source_worker = c.example_project.resources[0].clone();
    let mut actual = source_worker.clone();
    actual.id = id("actual-worker");
    target.resources.push(actual.clone());
    target.calendars.push(
        c.example_project
            .calendars
            .iter()
            .find(|cal| cal.id == actual.calendar_id)
            .unwrap()
            .clone(),
    );
    choices.resources.insert(
        source_worker.id.clone(),
        TemplateResourceSelection::Reuse(actual.id.clone()),
    );
    let mut transport = builtin_template_support()
        .unwrap()
        .into_iter()
        .find(|p| matches!(p.definition.payload, ModulePayload::Transport(_)))
        .unwrap()
        .definition;
    transport.id = "finite.manual-transport".to_owned().try_into().unwrap();
    if let ModulePayload::Transport(definition) = &mut transport.payload {
        definition.resources = vec![ResourceRequirement {
            role: source_worker.role.clone(),
            quantity: source_worker.capacity.clone(),
            qualifications: source_worker.qualifications.clone(),
        }];
    }
    let reference = ModuleRef {
        id: transport.id.clone(),
        version: transport.version.clone(),
    };
    c.definitions.push(transport);
    ctx.supplies.push(SupplyRelation {
        id: id("declared-supply"),
        source: c.example_project.locations[0].id.clone(),
        destination: c.example_project.locations[1].id.clone(),
        item_type_ref: c.subset.seed.as_ref().unwrap().initial_conditions.items[0]
            .item_type_ref
            .clone(),
        target_quantity: c.subset.seed.as_ref().unwrap().orders[0]
            .quantity
            .clone()
            .unwrap(),
        window: ctx.window.clone(),
        transport_ref: reference.clone(),
        resources: BTreeMap::from([(source_worker.role.clone(), source_worker.id.clone())]),
        departure_windows: vec![],
        allow_partial_load: false,
        presentation_path: Some(serde_json::from_value(json!({
            "hall_id":c.example_project.halls[0].id,
            "waypoints_m":[c.example_project.locations[0].position.translation,c.example_project.locations[1].position.translation],
            "cargo_layout":{"assignment":"item_id_order","slots":[{"item_type_ref":c.subset.seed.as_ref().unwrap().initial_conditions.items[0].item_type_ref,"anchor_role":null,"offset":c.example_project.locations[0].position}]}
        })).unwrap()),
    });
    let p = prepare_template(&c, &target, &choices).unwrap();
    let s = prepare_template_seed_with_context(&c, &p, &ch, Some(&ctx)).unwrap();
    assert_eq!(s.supplies[0].resources.values().next().unwrap(), &actual.id);
    assert_eq!(
        serde_json::to_value(&s.supplies[0].transport_ref).unwrap(),
        serde_json::to_value(reference).unwrap()
    );
    assert_eq!(s.supplies[0].window.end_ms.0.value(), 300_000);
    let display=s.supplies[0].presentation_path.as_ref().unwrap();
    assert_eq!(display.hall_id,p.proposed_project.halls[0].id);
    assert_eq!(display.cargo_layout.slots[0].item_type_ref.id,s.supplies[0].item_type_ref.id);
    assert_eq!(serde_json::to_value(&display.waypoints_m).unwrap(),serde_json::to_value(&ctx.supplies[0].presentation_path.as_ref().unwrap().waypoints_m).unwrap());
    assert!(!s.supplies[0].allow_partial_load);
    assert!(
        !p.proposed_project
            .resources
            .iter()
            .any(|r| r.id == actual.id)
    );
    assert_eq!(
        serde_json::to_value(&target.resources[0]).unwrap(),
        serde_json::to_value(&actual).unwrap()
    );
    choices.resources.remove(&source_worker.id);
    let missing = prepare_template(&c, &target, &choices).unwrap();
    assert_eq!(
        prepare_template_seed_with_context(&c, &missing, &ch, Some(&ctx)).unwrap_err()[0].code,
        "template.seed_supply_resource"
    );
}

#[test]
fn engine_input_export_keeps_explicit_quality_example_separate_from_real_measurement() {
    let (mut c, ctx, mut ch) = source();
    ch.adopt_declared_inspection_results = true;
    let legacy = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    c.subset.adoption.as_mut().unwrap().inspection_presets = legacy
        .subset
        .adoption
        .unwrap()
        .inspection_presets
        .into_iter()
        .map(|mut p| {
            p.item_or_ancestor = Some(id("item-0"));
            p.match_kind = Some(QualityMatchKind::Ancestor);
            p
        })
        .collect();
    c.example_project.template_adoptions = vec![c.subset.adoption.as_ref().unwrap().clone()];
    let p = plan(&c);
    let s = prepare_template_seed_with_context(&c, &p, &ch, Some(&ctx)).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/模板与交接/runtime/seed-composition");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("engine-scenario.json"),
        serde_json::to_vec_pretty(&s).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("engine-project.json"),
        serde_json::to_vec_pretty(&p.proposed_project).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("engine-definitions.json"),
        serde_json::to_vec_pretty(&c.definitions).unwrap(),
    )
    .unwrap();
    assert_eq!(s.quality_presets.len(), 1);
    assert_eq!(s.orders.len(), 2);
    assert_eq!(s.arrivals.len(), 2);
}

#[test]
fn actual_scenario_context_requires_exact_seed_revision_time_and_preserves_declared_tasks() {
    let (c, ctx, _) = source();
    let seed = c.subset.seed.as_ref().unwrap();
    let mut scenario:ScenarioInput=serde_json::from_value(json!({"id":"actual-source-scenario","revision":1,"project_revision":c.example_project.revision,"title":"Explicit finite source","time_reference":ctx.time_reference,"window":ctx.window,"initial_conditions":seed.initial_conditions,"orders":seed.orders,"arrivals":seed.arrivals,"production_tasks":ctx.production_tasks,"supplies":ctx.supplies,"parameter_overrides":[],"scope":[],"issues":[],"cost_inputs":[],"quality_presets":[]})).unwrap();
    let prepared = template_seed_context_from_scenario(&c, &scenario).unwrap();
    assert_eq!(
        serde_json::to_value(&prepared.production_tasks).unwrap(),
        serde_json::to_value(&ctx.production_tasks).unwrap()
    );
    scenario.arrivals[0].at_ms = SimTimeMs(99_000u64.try_into().unwrap());
    assert_eq!(
        template_seed_context_from_scenario(&c, &scenario).unwrap_err()[0].code,
        "template.seed_context_mismatch"
    );
}

#[test]
fn full_source_context_never_silently_loses_unrepresented_costs_or_partial_scope() {
    let (c, ctx, _) = source();
    let seed = c.subset.seed.as_ref().unwrap();
    let mut scenario:ScenarioInput=serde_json::from_value(json!({"id":"actual-source-scenario","revision":1,"project_revision":c.example_project.revision,"title":"Explicit finite source","time_reference":ctx.time_reference,"window":ctx.window,"initial_conditions":seed.initial_conditions,"orders":seed.orders,"arrivals":seed.arrivals,"production_tasks":ctx.production_tasks,"supplies":ctx.supplies,"parameter_overrides":[],"scope":[{"kind":"device","id":c.example_project.devices[0].id}],"issues":[],"cost_inputs":[],"quality_presets":[]})).unwrap();
    assert_eq!(
        template_seed_context_from_scenario(&c, &scenario).unwrap_err()[0].code,
        "template.seed_context_scope"
    );
    scenario.scope.clear();
    scenario.quality_presets = vec![InspectionPreset {
        id: id("source-preset"),
        item_or_ancestor: id("item-0"),
        match_kind: QualityMatchKind::Ancestor,
        inspection_module: ModuleRef {
            id: c.definitions[0].id.clone(),
            version: c.definitions[0].version.clone(),
        },
        results: vec![],
    }];
    assert_eq!(
        template_seed_context_from_scenario(&c, &scenario).unwrap_err()[0].code,
        "template.seed_context_unrepresented"
    );
}

// Exact actual Root Project/Scenario/subset bodies selected before the fix.
// Only the package shell/unchanged executable definitions use the builtin SDK
// fixture. Archive selection, captured Scenario R and authority remain Host-owned.
fn actual_root_clock_source() -> (TemplateCandidate, ScenarioInput, Value) {
    let input: Value = serde_json::from_str(include_str!(
        "../../../scenarios/模板与交接/runtime/source-clock/selected-source.json"
    ))
    .unwrap();
    let mut c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    c.example_project = serde_json::from_value(input["project"].clone()).unwrap();
    c.subset = serde_json::from_value(input["subset"].clone()).unwrap();
    let reference: ModuleRef = serde_json::from_value(input["module_ref"].clone()).unwrap();
    c.package.definition.id = reference.id;
    c.package.definition.version = reference.version;
    let scenario: ScenarioInput = serde_json::from_value(input["scenario"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&scenario.id).unwrap(),
        input["captured_scenario"]["scenario_id"]
    );
    assert_eq!(
        serde_json::to_value(&scenario.revision).unwrap(),
        input["captured_scenario"]["revision"]
    );
    (c, scenario, input)
}

#[test]
fn actual_root_source_clock_prepares_without_rewriting_either_label_or_source() {
    let (c, source, input) = actual_root_clock_source();
    let before = json!({"project":c.example_project,"scenario":source,"subset":c.subset});
    let context = template_seed_context_from_scenario(&c, &source).unwrap();
    assert_eq!(c.example_project.time_reference.label, "工程相对时间");
    assert_eq!(context.time_reference.label, "scenario_start");
    let request: Value = serde_json::from_str(include_str!(
        "../../../scenarios/模板与交接/runtime/source-clock/original-preparation-request.json"
    ))
    .unwrap();
    let original_choices = &request["source"]["data"]["choices"];
    let choices = TemplateChoices {
        namespace: original_choices["namespace"].as_str().unwrap().into(),
        resources: original_choices["resources"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(from, to)| {
                (
                    id(from),
                    TemplateResourceSelection::Reuse(id(to.as_str().unwrap())),
                )
            })
            .collect(),
        reuse_items: BTreeMap::new(),
        include_example_product: true,
    };
    let target = c.example_project.clone();
    let prepared = prepare_template(&c, &target, &choices).unwrap();
    assert_eq!(
        serde_json::to_value(&prepared.source_ref).unwrap(),
        input["module_ref"]
    );
    assert_eq!(
        serde_json::to_value(&prepared.source_subset).unwrap(),
        serde_json::to_value(&c.subset).unwrap()
    );
    assert!(
        prepared.proposed_project.resources.is_empty(),
        "explicitly reuse all five actual source resources"
    );
    let original_seed = &request["source"]["data"]["seed"];
    let mut seed_choices = TemplateSeedChoices {
        scenario_id: original_seed["scenario_id"]
            .as_str()
            .unwrap()
            .to_owned()
            .try_into()
            .unwrap(),
        supply: true,
        demand: true,
        initial_quality: serde_json::from_value(original_seed["initial_quality"].clone()).unwrap(),
        adopt_declared_inspection_results: true,
    };
    for (supply, demand) in [(false, false), (true, false), (false, true), (true, true)] {
        seed_choices.supply = supply;
        seed_choices.demand = demand;
        seed_choices.adopt_declared_inspection_results = true;
        if !supply {
            assert_eq!(
                prepare_template_seed_with_context(&c, &prepared, &seed_choices, Some(&context))
                    .unwrap_err()[0]
                    .code,
                "template.inspection_target"
            );
            seed_choices.adopt_declared_inspection_results = false;
        }
        let seed = prepare_template_seed_with_context(&c, &prepared, &seed_choices, Some(&context))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&seed.time_reference).unwrap(),
            serde_json::to_value(&source.time_reference).unwrap()
        );
        assert_eq!(
            seed.initial_conditions.items.len(),
            if supply { 4 } else { 0 }
        );
        assert_eq!(seed.orders.len(), if demand { 1 } else { 0 });
        assert_eq!(seed.production_tasks.len(), if demand { 1 } else { 0 });
        assert_eq!(seed.quality_presets.len(), if supply { 1 } else { 0 });
        if supply && demand {
            assert_eq!(
                serde_json::to_value(&seed.quality_presets[0].results).unwrap(),
                serde_json::to_value(&source.quality_presets[0].results).unwrap()
            );
            let mut merged = target.clone();
            macro_rules! append {($($field:ident),*) => {$(merged.$field.extend(prepared.proposed_project.$field.clone());)*};}
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
            if let Some(policies) = &prepared.project_setup_policies {
                merged.setup_policies = policies.clone();
            }
            for reference in &prepared.proposed_project.module_refs {
                if !merged
                    .module_refs
                    .iter()
                    .any(|r| r.id == reference.id && r.version == reference.version)
                {
                    merged.module_refs.push(reference.clone());
                }
            }
            merged.scenario_refs = vec![seed.id.clone()];
            assert_eq!(merged.resources.len(), target.resources.len());
            assert_eq!(
                serde_json::to_value(&merged.time_reference).unwrap(),
                serde_json::to_value(&target.time_reference).unwrap()
            );
            let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../scenarios/模板与交接/runtime/source-clock");
            std::fs::write(here.join("root-sdk-mapped.input.json"), serde_json::to_vec_pretty(&json!({"project":merged,"scenario":seed,"definitions":c.definitions,"source_ref":prepared.source_ref,"captured_scenario":input["captured_scenario"]})).unwrap()).unwrap();
        }
    }
    seed_choices.adopt_declared_inspection_results = false;
    let no_inspection =
        prepare_template_seed_with_context(&c, &prepared, &seed_choices, Some(&context)).unwrap();
    assert!(no_inspection.quality_presets.is_empty());
    assert_eq!(
        json!({"project":c.example_project,"scenario":source,"subset":c.subset}),
        before
    );
}

#[test]
fn source_clock_compares_exact_optional_origin_and_timezone_not_descriptive_label() {
    let (mut c, mut source, _) = actual_root_clock_source();
    for (project_origin, project_zone, scenario_origin, scenario_zone, allowed) in [
        (None, None, None, None, true),
        (
            Some("2026-10-16T00:00:00+08:00"),
            Some("Asia/Shanghai"),
            Some("2026-10-16T00:00:00+08:00"),
            Some("Asia/Shanghai"),
            true,
        ),
        (
            Some("2026-10-16T00:00:00+08:00"),
            None,
            Some("2026-10-16T00:00:00+08:00"),
            None,
            true,
        ),
        (
            None,
            Some("Asia/Shanghai"),
            None,
            Some("Asia/Shanghai"),
            true,
        ),
        (None, None, Some("2026-10-16T00:00:00+08:00"), None, false),
        (Some("2026-10-16T00:00:00+08:00"), None, None, None, false),
        (None, None, None, Some("Asia/Shanghai"), false),
        (None, Some("Asia/Shanghai"), None, None, false),
        (
            Some("2026-10-16T00:00:00+08:00"),
            Some("Asia/Shanghai"),
            Some("2026-10-16T00:00:01+08:00"),
            Some("Asia/Shanghai"),
            false,
        ),
        (
            Some("2026-10-16T00:00:00+08:00"),
            Some("Asia/Shanghai"),
            Some("2026-10-15T16:00:00Z"),
            Some("Asia/Shanghai"),
            false,
        ),
        (None, Some("Asia/Shanghai"), None, Some("UTC"), false),
        (None, Some("Asia/Shanghai"), None, Some("PRC"), false),
        (None, None, None, Some(""), false),
    ] {
        c.example_project.time_reference.origin_iso8601 = project_origin.map(str::to_owned);
        c.example_project.time_reference.timezone = project_zone.map(str::to_owned);
        source.time_reference.origin_iso8601 = scenario_origin.map(str::to_owned);
        source.time_reference.timezone = scenario_zone.map(str::to_owned);
        let before = serde_json::to_value(&source).unwrap();
        let result = template_seed_context_from_scenario(&c, &source);
        if allowed {
            assert_eq!(
                serde_json::to_value(result.unwrap().time_reference).unwrap(),
                serde_json::to_value(&source.time_reference).unwrap()
            );
        } else {
            assert_eq!(
                result.unwrap_err()[0].code,
                "template.seed_context_mismatch"
            );
        }
        assert_eq!(serde_json::to_value(&source).unwrap(), before);
    }
}

#[test]
fn actual_root_label_compatibility_keeps_revision_seed_and_scope_rejections() {
    let (c, source, _) = actual_root_clock_source();
    for field in [
        "project_revision",
        "initial_conditions",
        "orders",
        "arrivals",
        "scope",
    ] {
        let mut changed = serde_json::to_value(&source).unwrap();
        match field {
            "project_revision" => changed[field] = json!(1),
            "initial_conditions" => changed[field]["items"][0]["quality"] = json!("unchecked"),
            "orders" => changed[field][0]["due_at_ms"] = json!(299999),
            "arrivals" => {
                changed[field] = json!([{"id":"unexpected-arrival","at_ms":1,"destination":c.example_project.locations[0].id,"items":[source.initial_conditions.items[0]],"release_policy":"unchecked_allowed"}])
            }
            "scope" => {
                changed[field].as_array_mut().unwrap().pop();
            }
            _ => unreachable!(),
        }
        let changed: ScenarioInput = serde_json::from_value(changed).unwrap();
        assert_eq!(
            template_seed_context_from_scenario(&c, &changed).unwrap_err()[0].code,
            if field == "scope" {
                "template.seed_context_scope"
            } else {
                "template.seed_context_mismatch"
            },
            "{field}"
        );
    }
}
