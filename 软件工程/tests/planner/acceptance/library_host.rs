//! Independent consumers of the actual public Host, never Native UI evidence.
use super::support::{Sandbox, copy, error, id, ok};
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
fn call(
    h: &mut ProjectHost,
    ui: &ObjectId,
    p: Option<&ProjectId>,
    op: &str,
    params: Value,
) -> Value {
    h.dispatch(ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"w5-library-consumer","project_id":p,"scope":{"kind":if p.is_some(){"project"}else{"workspace"}},"operation":op,"params":params})).unwrap())
}
fn fresh() -> (ProjectHost, ObjectId) {
    let ui = id("w5-library-ui");
    let mut h = ProjectHost::new(id("w5-library-host"), ui.clone());
    for name in [
        "machining-delivery",
        "purchased-assembly",
        "parallel-electromechanical",
        "carton-factory",
    ] {
        h.register_bundled_template_directory(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/templates")
                .join(name)
                .join("candidate"),
        )
        .unwrap();
    }
    for name in [
        "items",
        "devices",
        "resources",
        "tasks",
        "operations",
        "transport",
        "professional",
        "media",
    ] {
        h.register_bundled_module_directory(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../modules/devices/template-support")
                .join(name),
        )
        .unwrap();
    }
    (h, ui)
}
fn create(h: &mut ProjectHost, ui: &ObjectId) -> ProjectId {
    let c =
        sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::PurchasedAssembly).unwrap();
    let _: EditCommit = ok(call(
        h,
        ui,
        None,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"W5 actual library consumer","time_reference":c.example_project.time_reference,"departure":"discard_after_tasks_terminal"}}]}),
    ));
    let context: ProjectContext = ok(call(h, ui, None, "get_context", json!({"selection":[]})));
    context.project.unwrap().id
}
fn project(h: &mut ProjectHost, ui: &ObjectId, p: &ProjectId) -> Value {
    let value: DomainObject = ok(call(
        h,
        ui,
        Some(p),
        "get_object",
        json!({"object_id":{"kind":"project","id":p},"source":{"kind":"draft"},"node_index":null}),
    ));
    serde_json::to_value(value).unwrap()["data"].clone()
}
fn query(
    h: &mut ProjectHost,
    ui: &ObjectId,
    p: Option<&ProjectId>,
    category: Value,
    sources: Value,
) -> ModuleTemplateCandidates {
    ok(call(
        h,
        ui,
        p,
        "query_module_library",
        json!({"category_filters":category,"industry":null,"product_family":null,"site_envelope":null,"source_scope":sources}),
    ))
}
fn preview(
    h: &mut ProjectHost,
    ui: &ObjectId,
    p: &ProjectId,
    r: &ModuleRef,
    choices: Value,
    seed: Value,
) -> TemplatePreview {
    let rev = project(h, ui, p)["revision"].clone();
    let object: DomainObject = ok(call(
        h,
        ui,
        Some(p),
        "get_object",
        json!({"object_id":{"kind":"module","id":r.id},"source":{"kind":"template_preview","data":{"module_ref":r,"base_revision":rev,"choices":choices,"seed":seed}},"node_index":null}),
    ));
    let DomainObject::TemplatePreview(value) = object else {
        panic!("actual template preview required")
    };
    *value
}
fn choices(meta: &TemplatePreview, namespace: &str) -> Value {
    json!({"source_project":meta.source_project,"namespace":namespace,"resources":meta.resources.iter().map(|r|(r.id.clone(),Option::<ObjectId>::None)).collect::<BTreeMap<_,_>>(),"reuse_items":{},"include_example_product":true,"site_envelope":null})
}
fn done(
    h: &mut ProjectHost,
    ui: &ObjectId,
    p: Option<&ProjectId>,
    task: &TaskHandle,
) -> TaskOutput {
    let mut work = h.drain_io_work();
    assert_eq!(work.len(), 1);
    h.finish_io(work.pop().unwrap().run());
    let state: TaskState = ok(call(h, ui, p, "get_task", json!({"task_id":task.id})));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    state.result.unwrap()
}
#[test]
fn p401_library_registration_and_query_are_readonly_exact_and_cover_nine_categories() {
    let (mut h, ui) = fresh();
    let c: ProjectContext = ok(call(
        &mut h,
        &ui,
        None,
        "get_context",
        json!({"selection":[]}),
    ));
    assert!(c.project.is_none());
    let result = query(&mut h, &ui, None, json!([]), json!(["bundled"]));
    let categories: BTreeSet<_> = result
        .candidates
        .iter()
        .map(|c| {
            serde_json::to_value(&c.category)
                .unwrap()
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    let templates = query(&mut h, &ui, None, json!(["templates"]), json!(["bundled"]));
    assert_eq!(templates.candidates.len(), 4);
    let p = create(&mut h, &ui);
    let before = project(&mut h, &ui, &p);
    let _: ModuleTemplateCandidates = query(
        &mut h,
        &ui,
        Some(&p),
        json!([]),
        json!(["bundled", "user_granted_libraries"]),
    );
    assert_eq!(project(&mut h, &ui, &p), before);
    assert_eq!(before["module_refs"], json!([]));
    assert_eq!(before["devices"], json!([]));
    assert_eq!(before["scenario_refs"], json!([]));
    let second = create(&mut h, &ui);
    assert_ne!(p, second);
    assert_eq!(
        query(
            &mut h,
            &ui,
            Some(&second),
            json!(["templates"]),
            json!(["bundled"])
        )
        .candidates
        .len(),
        4
    );
    let expected = BTreeSet::from_iter(
        [
            "items",
            "devices",
            "resources",
            "tasks",
            "operations",
            "transport",
            "templates",
            "professional",
            "media",
        ]
        .map(str::to_owned),
    );
    assert_eq!(
        categories, expected,
        "Original nine-category samples must reach the actual bundled library consumer"
    );
}
#[test]
fn p401_all_four_previews_are_readonly_and_omit_seed_until_selected() {
    let (mut h, ui) = fresh();
    let p = create(&mut h, &ui);
    let before = project(&mut h, &ui, &p);
    for (i, c) in query(
        &mut h,
        &ui,
        Some(&p),
        json!(["templates"]),
        json!(["bundled"]),
    )
    .candidates
    .into_iter()
    .enumerate()
    {
        let meta = preview(&mut h, &ui, &p, &c.module_ref, Value::Null, Value::Null);
        assert!(meta.command.is_none());
        assert!(!meta.resources.is_empty());
        let ready = preview(
            &mut h,
            &ui,
            &p,
            &c.module_ref,
            choices(&meta, &format!("w5-preview-{i}")),
            Value::Null,
        );
        let command = ready.command.unwrap();
        assert!(command.explicit_new_scenario_seed.is_none());
        assert!(command.preparation.is_some());
        let v = serde_json::to_value(command).unwrap();
        assert!(
            v["proposed_objects"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["kind"] == "device")
        );
        assert!(
            v["proposed_objects"]
                .as_array()
                .unwrap()
                .iter()
                .all(|o| o["kind"] != "scenario")
        );
        assert_eq!(project(&mut h, &ui, &p), before);
    }
}
#[test]
fn p401_real_prepared_seed_runs_original_times_and_saved_archive_keeps_exact_source_without_registry()
 {
    let (mut h, ui) = fresh();
    let p = create(&mut h, &ui);
    let source =
        sf_module_kit::builtin_template(sf_module_kit::BuiltinTemplate::PurchasedAssembly).unwrap();
    let r = ModuleRef {
        id: source.package.definition.id.clone(),
        version: source.package.definition.version.clone(),
    };
    let meta = preview(&mut h, &ui, &p, &r, Value::Null, Value::Null);
    let seed = json!({"scenario_id":"W5-P401-SEED","supply":true,"demand":true,"initial_quality":source.subset.adoption.as_ref().unwrap().deferred_seed.as_ref().unwrap().initial_stock.iter().map(|s|(s.location_id.clone(),QualityState::Released)).collect::<BTreeMap<_,_>>(),"adopt_declared_inspection_results":true});
    let ready = preview(&mut h, &ui, &p, &r, choices(&meta, "w5-real-library"), seed);
    let command = ready.command.unwrap();
    assert!(command.explicit_new_scenario_seed.is_some());
    let _: EditCommit = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "apply_commands",
        json!({"base_revision":ready.base_revision,"commands":[{"kind":"instantiate_template","data":command}]}),
    ));
    let before = project(&mut h, &ui, &p);
    let scenario: DomainObject = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "get_object",
        json!({"object_id":{"kind":"scenario","id":"W5-P401-SEED"},"source":{"kind":"draft"},"node_index":null}),
    ));
    let scenario = serde_json::to_value(scenario).unwrap()["data"].clone();
    let task: TaskHandle = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "run_scenario",
        json!({"scenario_id":"W5-P401-SEED","input_revision":before["revision"],"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}),
    ));
    assert!(matches!(task.state, ComputationStatus::Queued));
    let TaskOutput::Run(summary) = done(&mut h, &ui, Some(&p), &task) else {
        panic!("real Run required")
    };
    let view: RunView = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "get_run",
        json!({"run_id":summary.run_id,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(run) = view else {
        panic!("full Run")
    };
    assert_eq!(
        run.acceptance_events
            .iter()
            .map(|a| a.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![70000, 110000]
    );
    let frozen = serde_json::to_value(&run).unwrap();
    let tree = Sandbox::new();
    let original = tree.directory("p401-save");
    let moved = tree.directory("p401-moved");
    let selected = h
        .select_directory(&ui, FilePurpose::SaveProject, original.clone())
        .unwrap();
    let task: TaskHandle = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "save_project",
        json!({"revision":before["revision"],"selected_directory_ref":selected}),
    ));
    let _ = done(&mut h, &ui, Some(&p), &task);
    copy(&original, &moved);
    // Fresh actual Host has no bundled registry: archive is its sole authority.
    let mut reopened = ProjectHost::new(id("w5-reopened-library"), ui.clone());
    let selected = reopened
        .select_directory(&ui, FilePurpose::ImportPackage, moved)
        .unwrap();
    let task: TaskHandle = ok(call(
        &mut reopened,
        &ui,
        None,
        "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
    ));
    let TaskOutput::Import(draft) = done(&mut reopened, &ui, None, &task) else {
        panic!("real import")
    };
    let _: EditCommit = ok(call(
        &mut reopened,
        &ui,
        None,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task.id,"choices":{"chosen_policy":"open_project","objects_to_create":draft.proposed_objects,"module_versions":draft.proposed_definitions,"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"stay"}}}]}),
    ));
    assert_eq!(project(&mut reopened, &ui, &p), before);
    let reread: RunView = ok(call(
        &mut reopened,
        &ui,
        Some(&p),
        "get_run",
        json!({"run_id":run.id,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(reread) = reread else {
        panic!("full saved Run")
    };
    assert_eq!(serde_json::to_value(reread).unwrap(), frozen);
    let meta = preview(&mut reopened, &ui, &p, &r, Value::Null, Value::Null);
    assert!(!meta.resources.is_empty());
}
#[test]
fn p401_stale_preview_and_forged_source_reject_without_overwriting_current_project() {
    let (mut h, ui) = fresh();
    let p = create(&mut h, &ui);
    let c = query(
        &mut h,
        &ui,
        Some(&p),
        json!(["templates"]),
        json!(["bundled"]),
    )
    .candidates
    .remove(0);
    let meta = preview(&mut h, &ui, &p, &c.module_ref, Value::Null, Value::Null);
    let ready = preview(
        &mut h,
        &ui,
        &p,
        &c.module_ref,
        choices(&meta, "w5-stale-library"),
        Value::Null,
    );
    let before = project(&mut h, &ui, &p);
    let _: EditCommit = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "apply_commands",
        json!({"base_revision":before["revision"],"commands":[{"kind":"set_project_settings","data":{"title":"Real concurrent project edit","time_reference":before["time_reference"],"generation_context":null,"sources":[]}}]}),
    ));
    let current = project(&mut h, &ui, &p);
    let failure = error(call(
        &mut h,
        &ui,
        Some(&p),
        "apply_commands",
        json!({"base_revision":ready.base_revision,"commands":[{"kind":"instantiate_template","data":ready.command.unwrap()}]}),
    ));
    assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    assert_eq!(project(&mut h, &ui, &p), current);
    let mut forged = choices(&meta, "w5-forged-source");
    forged["source_project"] = json!("not-a-listed-source.json");
    let response = call(
        &mut h,
        &ui,
        Some(&p),
        "get_object",
        json!({"object_id":{"kind":"module","id":c.module_ref.id},"source":{"kind":"template_preview","data":{"module_ref":c.module_ref,"base_revision":current["revision"],"choices":forged,"seed":null}},"node_index":null}),
    );
    let _ = error(response);
    assert_eq!(project(&mut h, &ui, &p), current);
}
#[test]
fn p401_two_explicit_copies_get_disjoint_ids_without_implicit_inventory_orders_or_run() {
    let (mut h, ui) = fresh();
    let p = create(&mut h, &ui);
    let c = query(
        &mut h,
        &ui,
        Some(&p),
        json!(["templates"]),
        json!(["bundled"]),
    )
    .candidates
    .remove(0);
    let mut previous = BTreeSet::new();
    for namespace in ["w5-library-copy-a", "w5-library-copy-b"] {
        let meta = preview(&mut h, &ui, &p, &c.module_ref, Value::Null, Value::Null);
        let ready = preview(
            &mut h,
            &ui,
            &p,
            &c.module_ref,
            choices(&meta, namespace),
            Value::Null,
        );
        let command = ready.command.unwrap();
        assert!(command.explicit_new_scenario_seed.is_none());
        let objects = serde_json::to_value(&command.proposed_objects).unwrap();
        let ids: BTreeSet<_> = objects
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|o| o["data"]["id"].as_str().map(str::to_owned))
            .collect();
        assert!(previous.is_disjoint(&ids));
        previous.extend(ids);
        let _: EditCommit = ok(call(
            &mut h,
            &ui,
            Some(&p),
            "apply_commands",
            json!({"base_revision":ready.base_revision,"commands":[{"kind":"instantiate_template","data":command}]}),
        ));
    }
    let state = project(&mut h, &ui, &p);
    assert_eq!(state["scenario_refs"], json!([]));
    let context: ProjectContext = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "get_context",
        json!({"selection":[]}),
    ));
    assert!(context.run_summaries.is_empty());
    assert!(!state["devices"].as_array().unwrap().is_empty());
}

#[test]
fn p401_all_eight_real_support_definitions_adopt_save_open_and_exact_registered_source_is_immutable()
 {
    let (mut h, ui) = fresh();
    let p = create(&mut h, &ui);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../modules/devices/template-support");
    let mut exact = Vec::new();
    for category in [
        "items",
        "devices",
        "resources",
        "tasks",
        "operations",
        "transport",
        "professional",
        "media",
    ] {
        let package: Versioned<ModulePackage> = serde_json::from_slice(
            &std::fs::read(root.join(category).join("module.json")).unwrap(),
        )
        .unwrap();
        let reference = ModuleRef {
            id: package.data.definition.id,
            version: package.data.definition.version,
        };
        let base = project(&mut h, &ui, &p)["revision"].clone();
        let _: EditCommit = ok(call(
            &mut h,
            &ui,
            Some(&p),
            "apply_commands",
            json!({"base_revision":base,"commands":[{"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}]}),
        ));
        let object: DomainObject = ok(call(
            &mut h,
            &ui,
            Some(&p),
            "get_object",
            json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null}),
        ));
        exact.push((reference, serde_json::to_value(object).unwrap()));
    }
    let before = project(&mut h, &ui, &p);
    for field in ["devices", "resources", "scenario_refs"] {
        assert_eq!(
            before[field],
            json!([]),
            "adopting definitions must not create {field}"
        );
    }
    let tree = Sandbox::new();
    let altered = tree.directory("changed-exact-support");
    copy(&root.join("items"), &altered);
    let mut file: Value =
        serde_json::from_slice(&std::fs::read(altered.join("module.json")).unwrap()).unwrap();
    file["data"]["definition"]["title"] =
        json!("Changed bytes under the same exact source version");
    std::fs::write(
        altered.join("module.json"),
        serde_json::to_vec(&file).unwrap(),
    )
    .unwrap();
    let registry =
        serde_json::to_value(query(&mut h, &ui, Some(&p), json!([]), json!(["bundled"]))).unwrap();
    assert!(h.register_bundled_module_directory(&altered).is_err());
    assert_eq!(
        serde_json::to_value(query(&mut h, &ui, Some(&p), json!([]), json!(["bundled"]))).unwrap(),
        registry
    );
    assert_eq!(project(&mut h, &ui, &p), before);
    let saved = tree.directory("support-save");
    let token = h
        .select_directory(&ui, FilePurpose::SaveProject, saved.clone())
        .unwrap();
    let task: TaskHandle = ok(call(
        &mut h,
        &ui,
        Some(&p),
        "save_project",
        json!({"revision":before["revision"],"selected_directory_ref":token}),
    ));
    let _ = done(&mut h, &ui, Some(&p), &task);
    let moved = tree.directory("support-moved");
    copy(&saved, &moved);
    let mut opened = ProjectHost::new(id("w5-support-reopened"), ui.clone());
    let token = opened
        .select_directory(&ui, FilePurpose::ImportPackage, moved)
        .unwrap();
    let task: TaskHandle = ok(call(
        &mut opened,
        &ui,
        None,
        "import_package",
        json!({"selected_file_ref":token,"import_policy":"open_project","base_revision":null}),
    ));
    let TaskOutput::Import(draft) = done(&mut opened, &ui, None, &task) else {
        panic!("actual import")
    };
    let _: EditCommit = ok(call(
        &mut opened,
        &ui,
        None,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task.id,"choices":{"chosen_policy":"open_project","objects_to_create":draft.proposed_objects,"module_versions":draft.proposed_definitions,"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"stay"}}}]}),
    ));
    assert_eq!(project(&mut opened, &ui, &p), before);
    for (reference, original) in exact {
        let object: DomainObject = ok(call(
            &mut opened,
            &ui,
            Some(&p),
            "get_object",
            json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null}),
        ));
        assert_eq!(serde_json::to_value(object).unwrap(), original);
    }
}
