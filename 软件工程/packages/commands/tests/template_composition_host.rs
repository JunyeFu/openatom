//! Owner tests of the actual public Host composition writer, not UI evidence.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;

#[test]
fn bundled_source_scenario_keeps_own_identity_through_save_and_reopen() {
    let mut host=Harness::new();
    let scenario=host.object("scenario","LOCAL");
    let mut definition=fixture()["definitions"].as_array().unwrap().iter()
        .find(|d|d["payload"]["category"]=="resources").unwrap().clone();
    definition["id"]=json!("owner.bundled-scenario-source");definition["version"]=json!("1.0.0");
    let reference=json!({"id":definition["id"],"version":definition["version"]});
    let directory=host.tree.directory("bundled-scenario-source");
    let source=json!({"schema_version":"1.0","data":scenario});
    write(&directory.join("source-scenario.json"),&source);
    write(&directory.join("module.json"),&json!({"schema_version":"1.0","data":{"definition":definition,"assets":[],"files":[{"path":"source-scenario.json","kind":"scenario"}],"dependencies":[],"rights":[],"issues":[]}}));
    host.host.register_bundled_module_directory(&directory).unwrap();
    let _:EditCommit=ok(host.apply(1,json!([{"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
    let current=host.object("scenario","LOCAL");
    let saved=host.tree.directory("bundled-scenario-saved");
    let task=host.save(2,&saved);host.finish_one();let state=host.task(&task,true);
    assert!(matches!(state.task.state,ComputationStatus::Completed),"{state:?}");
    let archive=read_project_archive(&saved).unwrap();
    let entry=archive.manifest.documents.iter().find(|entry|entry.file_ref.path.as_str().ends_with("/source-scenario.json")).unwrap();
    assert!(matches!(entry.role,DocumentRole::SupportingFile));
    assert!(matches!(entry.object_ref.kind,ObjectKind::Scenario));assert_eq!(entry.object_ref.id.as_str(),"LOCAL");
    assert_eq!(entry.revision.as_ref().unwrap().0.value(),1);
    assert_eq!(serde_json::from_slice::<Value>(&archive.files[entry.file_ref.path.as_str()]).unwrap(),source);
    assert_eq!(archive.scenarios.len(),1,"A template source is not another active Project Scenario");
    let mut reopened=Harness::from_seed(Sandbox::new(),&saved);
    assert_eq!(reopened.object("scenario","LOCAL"),current);
    assert_eq!(reopened.object("module","owner.bundled-scenario-source")["definition"]["id"],reference["id"]);
}

fn scoped(h: &mut Harness, session: &ObjectId, draft: &ModuleId, op: &str, params: Value) -> Value {
    h.host.dispatch(session,serde_json::from_value(json!({"api_version":"0.1","request_id":"owner-template-composition","project_id":"P1","scope":{"kind":"definition_draft","data":draft},"operation":op,"params":params})).unwrap())
}
fn draft_call(h: &mut Harness, draft: &ModuleId, op: &str, params: Value) -> Value {
    scoped(h, &h.ui.clone(), draft, op, params)
}
fn definition() -> Value {
    json!({"id":"owner.composition","version":"1.0.0","title":"Actual selected combination","definition_state":"design","contribution":{"author":null,"license":null,"assets":[]},"parameters":[],"attribute_schema":[],"payload":{"category":"templates","data":{"project":"template/project.subset.json","generation_profile":null}}})
}
fn create(h: &mut Harness, base: u64) -> ModuleId {
    let commit: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"create_definition_draft","data":definition()}]),
    ));
    match commit.scope {
        EditScope::DefinitionDraft(identity) => identity.id,
        _ => panic!("actual definition draft"),
    }
}
fn subset(h: &mut Harness) -> Value {
    let p = h.object("project", "P1");
    let requirements:Vec<_>=p["resources"].as_array().unwrap().iter().map(|resource|json!({"kind":resource["kind"],"role":resource["role"],"capacity":resource["capacity"],"qualifications":resource["qualifications"],"default_calendar_ms":[],"presentation":null})).collect();
    json!({"layout_scope":"region","halls":p["halls"],"lines":p["lines"],"devices":p["devices"],"connections":p["connections"],"module_refs":p["module_refs"],"resource_requirements":requirements,"item_types":p["item_types"],"role_requirements":[],"suggested_role_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"seed":null,"locations":p["locations"],"adoption":null})
}
fn put(
    h: &mut Harness,
    draft: &ModuleId,
    base: u64,
    project: u64,
    definition: Value,
    subset: Value,
) -> Value {
    let source = if subset.get("seed").is_some_and(|seed| !seed.is_null()) {
        let actual = h.object("scenario", "LOCAL");
        json!({"scenario_id":actual["id"],"revision":actual["revision"]})
    } else {
        Value::Null
    };
    draft_call(
        h,
        draft,
        "apply_commands",
        json!({"base_revision":base,"commands":[{"kind":"put_template_composition","data":{"project_revision":project,"definition":definition,"subset":subset,"source_scenario":source}}]}),
    )
}
fn read_definition(h: &mut Harness, draft: &ModuleId) -> Value {
    let object: DomainObject = ok(draft_call(
        h,
        draft,
        "get_object",
        json!({"object_id":{"kind":"definition_draft","id":draft},"source":{"kind":"draft"},"node_index":null}),
    ));
    serde_json::to_value(object).unwrap()["data"].clone()
}
fn draft_preview(h: &mut Harness, draft: &ModuleId, base: u64) -> TemplatePreview {
    let actual = read_definition(h, draft);
    let object: DomainObject = ok(draft_call(
        h,
        draft,
        "get_object",
        json!({"object_id":{"kind":"module","id":actual["id"]},"source":{"kind":"template_preview","data":{"module_ref":{"id":actual["id"],"version":actual["version"]},"base_revision":base,"choices":null,"seed":null}},"node_index":null}),
    ));
    let DomainObject::TemplatePreview(preview) = object else {
        panic!("actual captured sidecar")
    };
    *preview
}
fn freeze(h: &mut Harness, draft: &ModuleId, base: u64, version: &str) -> EditCommit {
    ok(draft_call(
        h,
        draft,
        "apply_commands",
        json!({"base_revision":base,"commands":[{"kind":"commit_definition_version","data":{"version":version}}]}),
    ))
}
fn project_preview(h: &mut Harness, reference: &ModuleRef, choices: Value) -> TemplatePreview {
    let revision = h.context().project.unwrap().revision;
    let object:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":revision,"choices":choices,"seed":null}},"node_index":null})));
    let DomainObject::TemplatePreview(preview) = object else {
        panic!("actual committed sidecar")
    };
    *preview
}
fn adopt_reference(h: &mut Harness, base: u64, reference: &ModuleRef) {
    let _:EditCommit=ok(h.apply(base,json!([{"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
}

#[test]
fn composition_retains_selected_device_and_resource_exact_media_dependencies_in_private_export() {
    let tree = Sandbox::new();
    let directory = tree.directory("media-source");
    let mut value = fixture();
    let native = json!({"kind":"native_scene","source":{"kind":"native_scene","data":"source.scene"},"unit":"millimeter","coordinate_frame":"right_handed_z_up","scale":{"x":1,"y":1,"z":1},"nodes":[],"poses":[],"actions":[],"timeline":[],"tags":[],"anchor_roles":{},"port_anchors":{},"lod":{"detailed":null,"simplified":null,"region":"device_icons"},"production_effect":"none"});
    let references = [
        "owner.device-media",
        "owner.lod-media",
        "owner.resource-media",
    ]
    .map(|id| json!({"id":id,"version":"0.1.0"}));
    for reference in &references {
        let mut media = definition();
        media["id"] = reference["id"].clone();
        media["version"] = reference["version"].clone();
        media["payload"] = json!({"category":"media","data":native});
        value["definitions"].as_array_mut().unwrap().push(media);
        value["project"]["module_refs"]
            .as_array_mut()
            .unwrap()
            .push(reference.clone());
    }
    let mut device = native.clone();
    device["source"] = json!({"kind":"media_ref","data":references[0]});
    device["lod"]["simplified"] = json!({"kind":"media_ref","data":references[1]});
    value["project"]["devices"][0]["presentation_override"] = device;
    let mut resource = native;
    resource["source"] = json!({"kind":"media_ref","data":references[2]});
    value["project"]["resources"][0]["presentation"] = resource.clone();
    seed(&directory, &value);
    let mut h = Harness::from_seed(tree, &directory);
    let original = h.object("project", "P1");
    let mut selected = subset(&mut h);
    selected["module_refs"] = json!([]);
    selected["resource_requirements"][0]["presentation"] = resource;
    let draft = create(&mut h, 1);
    let _: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), selected));
    let preview = draft_preview(&mut h, &draft, 2);
    for reference in &references {
        assert!(
            serde_json::to_value(&preview.subset.module_refs)
                .unwrap()
                .as_array()
                .unwrap()
                .contains(reference),
            "omitted actual media dependency {reference}"
        );
    }
    assert_eq!(h.object("project", "P1"), original);
    freeze(&mut h, &draft, 2, "1.1.0");
    let reference: ModuleRef =
        serde_json::from_value(json!({"id":"owner.composition","version":"1.1.0"})).unwrap();
    adopt_reference(&mut h, 1, &reference);
    let exported = h.tree.directory("media-private-export");
    let token = h
        .host
        .select_directory(&h.ui, FilePurpose::ExportPackage, exported.clone())
        .unwrap();
    let task: TaskHandle = ok(h.call("export_package", json!({"revision":2,"selected_destination_ref":token,"package_scope":{"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[reference],"include_redistributable_assets":true}})));
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let archive = read_project_archive(&exported).unwrap();
    for reference in &references {
        assert!(archive.manifest.documents.iter().any(|entry| {
            entry
                .module_ref
                .as_ref()
                .is_some_and(|actual| serde_json::to_value(actual).unwrap() == *reference)
        }));
    }
}

#[test]
fn composition_creation_history_commit_movable_archive_and_real_template_consumers_keep_old_run() {
    let mut h = Harness::new();
    let task = h.run("LOCAL");
    h.finish_one();
    let old = h.completed_run(&task);
    assert_eq!(old.acceptance_events[0].at_ms.0.value(), 75000);
    let project = h.object("project", "P1");
    let composition = subset(&mut h);
    let draft = create(&mut h, 1);
    let commit: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), composition.clone()));
    assert_eq!(commit.new_revision.0.value(), 2);
    assert_eq!(h.object("project", "P1"), project);
    let captured = draft_preview(&mut h, &draft, 2);
    assert_eq!(
        serde_json::to_value(&captured.subset.devices).unwrap(),
        composition["devices"]
    );
    assert!(captured.command.is_none() && captured.subset.seed.is_none());
    let _: EditCommit = ok(draft_call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"undo"}]}),
    ));
    let missing = error(draft_call(
        &mut h,
        &draft,
        "get_object",
        json!({"object_id":{"kind":"module","id":"owner.composition"},"source":{"kind":"template_preview","data":{"module_ref":{"id":"owner.composition","version":"1.0.0"},"base_revision":3,"choices":null,"seed":null}},"node_index":null}),
    ));
    assert!(matches!(missing.code, ErrorCode::MissingReference));
    let _: EditCommit = ok(draft_call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":3,"commands":[{"kind":"redo"}]}),
    ));
    assert_eq!(
        serde_json::to_value(&draft_preview(&mut h, &draft, 4).subset).unwrap(),
        serde_json::to_value(&captured.subset).unwrap()
    );
    let committed = freeze(&mut h, &draft, 4, "1.1.0");
    assert_eq!(committed.new_revision.0.value(), 5);
    let reference: ModuleRef =
        serde_json::from_value(json!({"id":"owner.composition","version":"1.1.0"})).unwrap();
    adopt_reference(&mut h, 1, &reference);
    let preview = project_preview(&mut h, &reference, Value::Null);
    assert_eq!(
        preview.resources.len(),
        project["resources"].as_array().unwrap().len()
    );
    assert!(
        preview
            .subset
            .adoption
            .as_ref()
            .unwrap()
            .deferred_seed
            .is_none()
    );
    let repeat = freeze(&mut h, &draft, 5, "1.1.0");
    assert_eq!(repeat.new_revision.0.value(), 6);
    assert_eq!(
        serde_json::to_value(h.full(&old.id)).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    let directory = h.tree.directory("saved-composition");
    let save = h.save(2, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let moved = h.tree.directory("moved-composition");
    copy(&directory, &moved);
    let tree = Sandbox::new();
    let imported = tree.directory("copy");
    copy(&moved, &imported);
    let mut reopened = Harness::from_seed(tree, &imported);
    let actual = project_preview(&mut reopened, &reference, Value::Null);
    assert_eq!(
        serde_json::to_value(&actual.subset).unwrap(),
        serde_json::to_value(&preview.subset).unwrap()
    );
    let resources: serde_json::Map<_, _> = actual
        .resources
        .iter()
        .map(|resource| (resource.id.as_str().to_owned(), Value::Null))
        .collect();
    let prepared = project_preview(
        &mut reopened,
        &reference,
        json!({"source_project":actual.source_project,"namespace":"explicit-copy","resources":resources,"reuse_items":{},"include_example_product":false,"site_envelope":null}),
    );
    let command = prepared.command.unwrap();
    assert!(command.explicit_new_scenario_seed.is_none());
    let _: EditCommit =
        ok(reopened.apply(2, json!([{"kind":"instantiate_template","data":command}])));
    assert_eq!(
        reopened.object("project", "P1")["devices"]
            .as_array()
            .unwrap()
            .len(),
        2 * project["devices"].as_array().unwrap().len()
    );
    assert_eq!(
        serde_json::to_value(reopened.full(&old.id)).unwrap(),
        serde_json::to_value(&old).unwrap()
    );
    let exported = reopened.tree.directory("module-export");
    let token = reopened
        .host
        .select_directory(&reopened.ui, FilePurpose::ExportPackage, exported.clone())
        .unwrap();
    let handle:TaskHandle=ok(reopened.call("export_package",json!({"revision":3,"selected_destination_ref":token,"package_scope":{"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[reference],"include_redistributable_assets":true}})));
    reopened.finish_one();
    let state = reopened.task(&handle, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let archive = read_project_archive(&exported).unwrap();
    assert!(
        archive.files.contains_key(
            preview
                .subset
                .adoption
                .as_ref()
                .unwrap()
                .source_archive
                .path
                .as_str()
        )
    );
}

#[test]
fn composition_both_revisions_source_identity_scope_and_failed_batches_are_atomic() {
    let mut h = Harness::new();
    let selected = subset(&mut h);
    let draft = create(&mut h, 1);
    for (base, project) in [(0, 1), (1, 0)] {
        let failure = error(put(
            &mut h,
            &draft,
            base,
            project,
            definition(),
            selected.clone(),
        ));
        assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    }
    let before = read_definition(&mut h, &draft);
    let mut forged = selected.clone();
    forged["devices"][0]["title"] = json!("forged object");
    let failure = error(put(&mut h, &draft, 1, 1, definition(), forged));
    assert!(matches!(failure.code, ErrorCode::InvalidInput));
    assert_eq!(read_definition(&mut h, &draft), before);
    let mut forged = selected.clone();
    forged["adoption"] = json!({"id":"claim","template_ref":{"id":"owner.composition","version":"1.0.0"},"source_archive":{"path":"secret.json","kind":"asset"},"resource_choices":[],"preparation_bindings":[],"imported_claims":[],"inspection_presets":[],"deferred_seed":null,"missing_inputs":[]});
    let failure = error(put(&mut h, &draft, 1, 1, definition(), forged));
    assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    let mut missing = definition();
    missing["payload"]["data"]["generation_profile"] = json!("scenario.json");
    let failure = error(put(&mut h, &draft, 1, 1, missing, selected.clone()));
    assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    let invalid = draft_call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":1,"definition":definition(),"subset":selected}},{"kind":"put_definition","data":{"id":"different","version":"1.0.0","title":"bad","definition_state":"design","contribution":{"author":null,"license":null,"assets":[]},"parameters":[],"attribute_schema":[],"payload":{"category":"templates","data":{"project":"no.json","generation_profile":null}}}}]}),
    );
    assert!(matches!(error(invalid).code, ErrorCode::InvalidInput));
    assert_eq!(read_definition(&mut h, &draft), before);
    let session = id("definition-only-composer");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: session.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::ApplyCommands],
            allowed_definition_drafts: vec![draft.clone()],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    let request = subset(&mut h);
    let denied = error(scoped(
        &mut h,
        &session,
        &draft,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":1,"definition":definition(),"subset":request}}]}),
    ));
    assert!(matches!(denied.code, ErrorCode::PermissionDenied));
    assert_eq!(read_definition(&mut h, &draft), before);
}

#[test]
fn composition_explicit_general_seed_is_exact_and_never_becomes_single_product_or_runtime_result() {
    let mut h = Harness::new();
    let scenario = h.object("scenario", "LOCAL");
    let mut selected = subset(&mut h);
    selected["seed"] = json!({"initial_conditions":scenario["initial_conditions"],"arrivals":scenario["arrivals"],"orders":scenario["orders"]});
    let seed = selected["seed"].clone();
    let draft = create(&mut h, 1);
    let _: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), selected));
    let preview = draft_preview(&mut h, &draft, 2);
    assert_eq!(serde_json::to_value(&preview.subset.seed).unwrap(), seed);
    assert!(
        preview
            .subset
            .adoption
            .as_ref()
            .unwrap()
            .deferred_seed
            .is_none()
    );
    assert_eq!(
        serde_json::to_value(&preview.source_scenario).unwrap(),
        json!({"scenario_id":"LOCAL","revision":1})
    );
    freeze(&mut h, &draft, 2, "1.1.0");
    let reference =
        serde_json::from_value(json!({"id":"owner.composition","version":"1.1.0"})).unwrap();
    adopt_reference(&mut h, 1, &reference);
    let directory = h.tree.directory("seed-composition");
    let save = h.save(2, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let source = project_preview(&mut h, &reference, Value::Null);
    let archive = read_project_archive(&directory).unwrap();
    let raw = read(&directory.join(source.source_project.as_str()));
    assert_eq!(raw["data"]["scenario_refs"], json!([]));
    assert_eq!(raw["data"]["acknowledgments"], json!([]));
    assert_eq!(raw["data"]["bindings"], json!([]));
    assert!(
        archive.files.contains_key(
            source
                .subset
                .adoption
                .as_ref()
                .unwrap()
                .source_archive
                .path
                .as_str()
        )
    );
    let loaded = read(
        &directory.join(
            source
                .subset
                .adoption
                .as_ref()
                .unwrap()
                .source_archive
                .path
                .as_str(),
        ),
    );
    assert!(!loaded["data"]["products"].as_array().unwrap().is_empty());
    let saved_subset = read(
        &directory.join(
            source
                .subset
                .adoption
                .as_ref()
                .unwrap()
                .source_archive
                .path
                .as_str()
                .replace("source-project.json", "project.subset.json"),
        ),
    );
    assert_eq!(saved_subset["data"]["seed"], seed);
}

#[test]
fn composition_leaf_authority_is_distinct_from_definition_title_and_project_read_authority() {
    let mut h = Harness::new();
    let selected = subset(&mut h);
    let draft = create(&mut h, 1);
    let _: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), selected));
    let preview = draft_preview(&mut h, &draft, 2);
    let actual = read_definition(&mut h, &draft);
    let session = id("actual-composition-editor");
    let grant = HostEditGrant {
        id: id("composition-only-leaf"),
        scope: TaskScope::Edit(EditScope::DefinitionDraft(DefinitionScopeIdentity {
            id: draft.clone(),
            revision: revision(2),
        })),
        branches: vec![HostEditBranch::PutDefinition],
        objects: vec![HostObjectEditGrant {
            target: ObjectRef {
                kind: ObjectKind::DefinitionDraft,
                id: id(draft.as_str()),
            },
            fields: vec![HostFieldGrant {
                pointer: "/template_composition/layout_scope".into(),
                element_id: None,
            }],
            can_remove: false,
        }],
        creates: vec![],
        source_versions: vec![],
        reusable_resources: vec![],
        created_objects: vec![],
    };
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: session.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![
                OperationName::ApplyCommands,
                OperationName::GetObject,
                OperationName::DescribeCapabilities,
            ],
            allowed_definition_drafts: vec![draft.clone()],
            selected_file_tokens: vec![],
            edit_grants: vec![grant],
        })
        .unwrap();
    let mut selected = serde_json::to_value(&preview.subset).unwrap();
    selected["layout_scope"] = json!("line");
    let mut wrong = actual.clone();
    wrong["title"] = json!("not granted");
    let denied = error(scoped(
        &mut h,
        &session,
        &draft,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"put_template_composition","data":{"project_revision":1,"definition":wrong,"subset":selected}}]}),
    ));
    assert!(matches!(denied.code, ErrorCode::PermissionDenied));
    assert_eq!(read_definition(&mut h, &draft), actual);
    let accepted: EditCommit = ok(scoped(
        &mut h,
        &session,
        &draft,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"put_template_composition","data":{"project_revision":1,"definition":actual,"subset":selected}}]}),
    ));
    assert_eq!(accepted.new_revision.0.value(), 3);
    assert!(matches!(
        draft_preview(&mut h, &draft, 3).subset.layout_scope,
        TemplateScope::Line
    ));
    assert!(
        h.host.caller_context(&session).unwrap().edit_grants[0]
            .created_objects
            .is_empty()
    );
    let mut restricted = h.host.caller_context(&session).unwrap().clone();
    restricted.allowed_operations =
        vec![OperationName::ApplyCommands, OperationName::ExportPackage];
    restricted.edit_grants[0].scope =
        TaskScope::Edit(EditScope::DefinitionDraft(DefinitionScopeIdentity {
            id: draft.clone(),
            revision: revision(3),
        }));
    restricted.edit_grants[0].branches = vec![HostEditBranch::CommitDefinitionVersion];
    restricted.edit_grants[0].objects[0].fields = vec![HostFieldGrant {
        pointer: "/version".into(),
        element_id: None,
    }];
    h.host
        .update_external_session_authority(&h.ui, restricted)
        .unwrap();
    let denied = error(scoped(
        &mut h,
        &session,
        &draft,
        "apply_commands",
        json!({"base_revision":3,"commands":[{"kind":"commit_definition_version","data":{"version":"1.1.0"}}]}),
    ));
    assert!(matches!(denied.code, ErrorCode::PermissionDenied));
    freeze(&mut h, &draft, 3, "1.1.0");
    let destination = h.tree.directory("unreadable-composition-export");
    let token = h
        .host
        .select_directory(&session, FilePurpose::ExportPackage, destination.clone())
        .unwrap();
    let denied = error(scoped(
        &mut h,
        &session,
        &draft,
        "export_package",
        json!({"revision":4,"selected_destination_ref":token,"package_scope":{"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[{"id":"owner.composition","version":"1.1.0"}],"include_redistributable_assets":true}}),
    ));
    assert!(matches!(denied.code, ErrorCode::PermissionDenied));
    assert_eq!(std::fs::read_dir(destination).unwrap().count(), 0);
    let foreign = id("foreign-composition-editor");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: foreign.clone(),
            granted_project: Some(ProjectId::try_from("OTHER-PROJECT".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::ApplyCommands, OperationName::GetObject],
            allowed_definition_drafts: vec![draft.clone()],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    let reply = scoped(
        &mut h,
        &foreign,
        &draft,
        "get_object",
        json!({"object_id":{"kind":"definition_draft","id":draft},"source":{"kind":"draft"},"node_index":null}),
    );
    assert!(reply["context"].is_null());
    assert!(matches!(error(reply).code, ErrorCode::PermissionDenied));
}

#[test]
fn composition_new_version_keeps_old_sidecars_and_missing_actual_files_reject_open_without_replacing_project()
 {
    let mut h = Harness::new();
    let selected = subset(&mut h);
    let draft = create(&mut h, 1);
    let _: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), selected));
    freeze(&mut h, &draft, 2, "1.1.0");
    let r1: ModuleRef =
        serde_json::from_value(json!({"id":"owner.composition","version":"1.1.0"})).unwrap();
    adopt_reference(&mut h, 1, &r1);
    let first = project_preview(&mut h, &r1, Value::Null);
    let mut second = serde_json::to_value(&first.subset).unwrap();
    second["layout_scope"] = json!("hall");
    let actual = read_definition(&mut h, &draft);
    let _: EditCommit = ok(put(&mut h, &draft, 3, 2, actual, second));
    freeze(&mut h, &draft, 4, "1.2.0");
    let r2: ModuleRef =
        serde_json::from_value(json!({"id":"owner.composition","version":"1.2.0"})).unwrap();
    adopt_reference(&mut h, 2, &r2);
    let latest = project_preview(&mut h, &r2, Value::Null);
    assert!(matches!(latest.subset.layout_scope, TemplateScope::Hall));
    assert!(matches!(first.subset.layout_scope, TemplateScope::Region));
    assert_ne!(first.source_project, latest.source_project);
    assert_eq!(
        serde_json::to_value(project_preview(&mut h, &r1, Value::Null).subset).unwrap(),
        serde_json::to_value(&first.subset).unwrap()
    );
    let saved = h.tree.directory("versioned-composition");
    let task = h.save(3, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let archive = read_project_archive(&saved).unwrap();
    assert!(
        archive.files.contains_key(first.source_project.as_str())
            && archive.files.contains_key(latest.source_project.as_str())
    );
    let broken = h.tree.directory("missing-sidecar");
    copy(&saved, &broken);
    std::fs::remove_file(broken.join(latest.source_project.as_str())).unwrap();
    let before = h.object("project", "P1");
    let handle = h.import(&broken);
    h.finish_one();
    let state = h.task(&handle, false);
    assert!(matches!(state.task.state, ComputationStatus::Failed));
    assert!(matches!(
        state.failure.unwrap().code,
        ErrorCode::MissingReference
    ));
    assert_eq!(h.object("project", "P1"), before);
}

#[test]
fn composition_captures_actual_asset_bytes_and_unknown_rights_without_allowing_a_rights_upgrade() {
    let tree = Sandbox::new();
    let directory = tree.directory("asset-source");
    let mut value = fixture();
    let asset = json!({"id":"owner.svg","path":"actual-owner.svg","kind":"svg"});
    let rights = json!({"asset_ref":"owner.svg","source":"actual finite owner SVG fixture","license":null,"redistribution_confirmed":false,"missing_reason":"not approved for redistribution"});
    let presentation = json!({"kind":"image","source":{"kind":"asset","data":asset},"unit":"meter","coordinate_frame":"right_handed_y_up","scale":{"x":1,"y":1,"z":1},"nodes":[],"poses":[],"actions":[],"timeline":[],"tags":[],"anchor_roles":{},"port_anchors":{},"lod":{"detailed":null,"simplified":null,"region":"device_icons"},"production_effect":"none"});
    let index = value["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|definition| definition["payload"]["category"] == "devices")
        .unwrap();
    value["definitions"][index]["payload"]["data"]["presentation"] = presentation.clone();
    value["definitions"][index]["contribution"]["assets"] = json!([rights]);
    seed(&directory, &value);
    let module_path = directory.join(format!("module-{index}.json"));
    let mut module = read(&module_path);
    module["data"]["assets"] = json!([asset]);
    module["data"]["rights"] = json!([rights]);
    module["data"]["files"] = json!([{"path":"actual-owner.svg","kind":"asset"}]);
    write(&module_path, &module);
    let mut manifest = read(&directory.join("project.json"));
    manifest["data"]["assets"] = json!([rights]);
    manifest["data"]["documents"]
        .as_array_mut()
        .unwrap()
        .push(document(
            "asset",
            "asset",
            "asset",
            "owner.svg",
            "actual-owner.svg",
            Value::Null,
        ));
    write(&directory.join("project.json"), &manifest);
    let bytes=b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"><path d=\"M0 0h1v1H0z\"/></svg>";
    std::fs::write(directory.join("actual-owner.svg"), bytes).unwrap();
    let mut h = Harness::from_seed(tree, &directory);
    let draft = create(&mut h, 1);
    let mut selected = subset(&mut h);
    selected["resource_requirements"][0]["presentation"] = presentation;
    let mut forged = definition();
    let mut upgraded = rights.clone();
    upgraded["license"] = json!("MIT");
    upgraded["redistribution_confirmed"] = json!(true);
    forged["contribution"]["assets"] = json!([upgraded]);
    let failure = error(put(&mut h, &draft, 1, 1, forged, selected.clone()));
    assert!(matches!(failure.code, ErrorCode::InvalidInput));
    let _: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), selected));
    freeze(&mut h, &draft, 2, "1.1.0");
    let reference: ModuleRef =
        serde_json::from_value(json!({"id":"owner.composition","version":"1.1.0"})).unwrap();
    adopt_reference(&mut h, 1, &reference);
    let saved = h.tree.directory("asset-composition");
    let save = h.save(2, &saved);
    h.finish_one();
    let state = h.task(&save, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let module:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null})));
    let DomainObject::Module(module) = module else {
        panic!("actual composition package")
    };
    assert_eq!(
        serde_json::to_value(&module.assets).unwrap(),
        json!([asset])
    );
    assert_eq!(
        serde_json::to_value(&module.rights).unwrap(),
        json!([rights])
    );
    assert_eq!(
        std::fs::read(saved.join("actual-owner.svg")).unwrap(),
        bytes
    );
    let moved = h.tree.directory("asset-moved");
    copy(&saved, &moved);
    let import = h.import(&moved);
    h.finish_one();
    let proposed = h.imported(&import);
    let _: EditCommit = ok(h.adopt(&import, &proposed, true));
    let actual = project_preview(&mut h, &reference, Value::Null);
    assert_eq!(
        serde_json::to_value(
            actual.subset.resource_requirements[0]
                .presentation
                .as_ref()
                .unwrap()
        )
        .unwrap()["source"]["data"],
        asset
    );
}

#[test]
fn composition_first_creation_is_available_to_the_real_ui_before_any_definition_draft_exists() {
    let ui = id("empty-composer-ui");
    let mut host = ProjectHost::new(id("empty-composer-host"), ui.clone());
    let mut call = |project: Value, scope: Value, operation: &str, params: Value| {
        host.dispatch(&ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"first-composition-capability","project_id":project,"scope":scope,"operation":operation,"params":params})).unwrap())
    };
    let created: EditCommit = ok(call(
        Value::Null,
        json!({"kind":"workspace"}),
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"real empty composition source","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}),
    ));
    let catalog: CapabilityCatalog = ok(call(
        Value::Null,
        json!({"kind":"workspace"}),
        "describe_capabilities",
        json!({}),
    ));
    assert!(
        catalog
            .available_command_kinds
            .iter()
            .any(|command| command == "create_definition_draft")
    );
    assert!(
        catalog
            .available_command_kinds
            .iter()
            .any(|command| command == "put_template_composition")
    );
    let draft: EditCommit = ok(call(
        json!(created.project_id),
        json!({"kind":"project"}),
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":definition()}]}),
    ));
    assert!(matches!(draft.scope, EditScope::DefinitionDraft(_)));
}

#[test]
fn composition_keeps_unmet_resource_requirements_editable_but_never_prepares_unqualified_actual_examples()
 {
    let mut h = Harness::new();
    let mut selected = subset(&mut h);
    selected["resource_requirements"][0]["qualifications"] =
        json!(["explicit-required-certification"]);
    let draft = create(&mut h, 1);
    let commit: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), selected));
    assert!(commit.issues.iter().any(|issue| {
        issue
            .blocks
            .iter()
            .any(|purpose| matches!(purpose, CapabilityPurpose::Adoption))
    }));
    let before = h.object("project", "P1");
    let preview = draft_preview(&mut h, &draft, 2);
    assert!(preview.command.is_none());
    let actual = read_definition(&mut h, &draft);
    let resources: serde_json::Map<_, _> = preview
        .resources
        .iter()
        .map(|resource| (resource.id.as_str().to_owned(), Value::Null))
        .collect();
    let failure = error(draft_call(
        &mut h,
        &draft,
        "get_object",
        json!({"object_id":{"kind":"module","id":actual["id"]},"source":{"kind":"template_preview","data":{"module_ref":{"id":actual["id"],"version":actual["version"]},"base_revision":2,"choices":{"source_project":preview.source_project,"namespace":"cannot-claim-qualified","resources":resources,"reuse_items":{},"include_example_product":false,"site_envelope":null},"seed":null}},"node_index":null}),
    ));
    assert!(matches!(failure.code, ErrorCode::InvalidInput));
    assert_eq!(h.object("project", "P1"), before);
}

fn capture_complete(h: &mut Harness, source_name: &str) -> (ModuleId, ModuleRef) {
    let base = h.context().project.unwrap().revision.0.value();
    let source = h.object("scenario", source_name);
    let mut selected = subset(h);
    selected["seed"] = json!({"initial_conditions":source["initial_conditions"],"arrivals":source["arrivals"],"orders":source["orders"]});
    let draft = create(h, base);
    let _: EditCommit = ok(draft_call(
        h,
        &draft,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":base,"definition":definition(),"subset":selected,"source_scenario":{"scenario_id":source["id"],"revision":source["revision"]}}}]}),
    ));
    freeze(h, &draft, 2, "1.1.0");
    let reference: ModuleRef =
        serde_json::from_value(json!({"id":"owner.composition","version":"1.1.0"})).unwrap();
    adopt_reference(h, base, &reference);
    (draft, reference)
}
fn prepare_complete(
    h: &mut Harness,
    reference: &ModuleRef,
    namespace: &str,
    scenario: Option<&str>,
    supply: bool,
    demand: bool,
) -> TemplateInstantiation {
    prepare_complete_inspection(h, reference, namespace, scenario, supply, demand, false)
}
fn prepare_complete_inspection(
    h: &mut Harness,
    reference: &ModuleRef,
    namespace: &str,
    scenario: Option<&str>,
    supply: bool,
    demand: bool,
    inspection: bool,
) -> TemplateInstantiation {
    let preview = project_preview(h, reference, Value::Null);
    let target = h.object("project", "P1");
    let resources: serde_json::Map<_, _> = preview
        .resources
        .iter()
        .map(|resource| {
            (
                resource.id.as_str().to_owned(),
                if target["resources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|actual| actual["id"] == json!(resource.id))
                {
                    json!(resource.id)
                } else {
                    Value::Null
                },
            )
        })
        .collect();
    let qualities: serde_json::Map<_, _> = preview
        .subset
        .seed
        .as_ref()
        .into_iter()
        .flat_map(|seed| {
            seed.initial_conditions
                .items
                .iter()
                .chain(seed.arrivals.iter().flat_map(|arrival| &arrival.items))
        })
        .map(|item| (item.id.as_str().to_owned(), json!("released")))
        .collect();
    let base = h.context().project.unwrap().revision;
    let object:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":base,"choices":{"source_project":preview.source_project,"namespace":namespace,"resources":resources,"reuse_items":{},"include_example_product":true,"site_envelope":null},"seed":scenario.map(|id|json!({"scenario_id":id,"supply":supply,"demand":demand,"initial_quality":qualities,"adopt_declared_inspection_results":inspection}))}},"node_index":null})));
    let DomainObject::TemplatePreview(prepared) = object else {
        panic!("actual SDK preparation")
    };
    prepared.command.unwrap()
}
fn multiple_source() -> Value {
    let mut value = fixture();
    let scenario = &mut value["scenario"];
    let order = scenario["orders"][0].clone();
    scenario["orders"] = json!([order.clone(), order]);
    for i in 0..2 {
        scenario["orders"][i]["id"] = json!(format!("ORDER-{:02}", i + 1));
        scenario["orders"][i]["quantity"]["value"]["coefficient"] = json!(1);
    }
    let task = scenario["production_tasks"][0].clone();
    scenario["production_tasks"] = json!([task.clone(), task]);
    for i in 0..2 {
        scenario["production_tasks"][i]["id"] = json!(format!("TASK-{}", i + 1));
        scenario["production_tasks"][i]["planned_quantity"]["value"]["coefficient"] = json!(1);
        scenario["production_tasks"][i]["order_allocations"][0]["order_id"] =
            json!(format!("ORDER-{:02}", i + 1));
        scenario["production_tasks"][i]["order_allocations"][0]["quantity"]["value"]["coefficient"] =
            json!(1);
    }
    let mut arrival = scenario["initial_conditions"]["items"][2].clone();
    arrival["id"] = json!("pcb-002");
    scenario["arrivals"] = json!([{"id":"second-pcb","at_ms":60000,"destination":"pcbs-A","items":[arrival],"release_policy":"unchecked_allowed"}]);
    value
}

#[test]
fn composition_seed_requires_explicit_actual_scenario_revision_and_history_restores_captured_context()
 {
    let mut h = Harness::new();
    let source = h.object("scenario", "LOCAL");
    let mut selected = subset(&mut h);
    selected["seed"] = json!({"initial_conditions":source["initial_conditions"],"orders":source["orders"],"arrivals":source["arrivals"]});
    let draft = create(&mut h, 1);
    let before = read_definition(&mut h, &draft);
    for reference in [
        Value::Null,
        json!({"scenario_id":"LOCAL","revision":9}),
        json!({"scenario_id":"MISSING","revision":1}),
    ] {
        let reply = draft_call(
            &mut h,
            &draft,
            "apply_commands",
            json!({"base_revision":1,"commands":[{"kind":"put_template_composition","data":{"project_revision":1,"definition":definition(),"subset":selected,"source_scenario":reference}}]}),
        );
        let failure = error(reply);
        assert!(matches!(
            failure.code,
            ErrorCode::InvalidInput | ErrorCode::RevisionConflict | ErrorCode::MissingReference
        ));
        assert_eq!(read_definition(&mut h, &draft), before);
    }
    let _: EditCommit = ok(put(&mut h, &draft, 1, 1, definition(), selected.clone()));
    assert_eq!(
        draft_preview(&mut h, &draft, 2)
            .source_scenario
            .unwrap()
            .revision
            .0
            .value(),
        1
    );
    let _: EditCommit = ok(draft_call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"undo"}]}),
    ));
    assert!(matches!(error(draft_call(&mut h,&draft,"get_object",json!({"object_id":{"kind":"module","id":"owner.composition"},"source":{"kind":"template_preview","data":{"module_ref":{"id":"owner.composition","version":"1.0.0"},"base_revision":3,"choices":null,"seed":null}},"node_index":null}))).code,ErrorCode::MissingReference));
    let _: EditCommit = ok(draft_call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":3,"commands":[{"kind":"redo"}]}),
    ));
    let mut changed = source.clone();
    changed["window"]["end_ms"] = json!(200000);
    let _: EditCommit = ok(h.apply(1, json!([support::put("scenario", changed)])));
    assert_eq!(
        draft_preview(&mut h, &draft, 4)
            .source_scenario
            .unwrap()
            .revision
            .0
            .value(),
        1
    );
    let actual = read_definition(&mut h, &draft);
    let failure = error(put(&mut h, &draft, 4, 1, actual, selected.clone()));
    assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    let actual = read_definition(&mut h, &draft);
    let failure = error(draft_call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":4,"commands":[{"kind":"put_template_composition","data":{"project_revision":2,"definition":actual,"subset":selected,"source_scenario":{"scenario_id":"LOCAL","revision":1}}}]}),
    ));
    assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    assert_eq!(
        draft_preview(&mut h, &draft, 4)
            .source_scenario
            .unwrap()
            .revision
            .0
            .value(),
        1
    );
    let mut partial = selected;
    partial["seed"]["orders"] = json!([]);
    let actual = read_definition(&mut h, &draft);
    let failure = error(put(&mut h, &draft, 4, 2, actual, partial));
    assert!(matches!(failure.code, ErrorCode::InvalidInput));
    assert_eq!(
        draft_preview(&mut h, &draft, 4)
            .source_scenario
            .unwrap()
            .revision
            .0
            .value(),
        1
    );
}

#[test]
fn seeded_template_definition_import_preserves_snapshot_identity_through_save_and_reopen() {
    let mut source = Harness::from_value(multiple_source());
    let (_, reference) = capture_complete(&mut source, "LOCAL");
    let exported = source.tree.directory("seeded-definition-contribution");
    let token = source
        .host
        .select_directory(&source.ui, FilePurpose::ExportPackage, exported.clone())
        .unwrap();
    let task: TaskHandle = ok(source.call("export_package", json!({"revision":2,"selected_destination_ref":token,"package_scope":{"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[reference],"include_redistributable_assets":true}})));
    source.finish_one();
    assert!(matches!(
        source.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let archive = read_project_archive(&exported).unwrap();
    let snapshot = archive
        .manifest
        .documents
        .iter()
        .find(|entry| matches!(entry.file_ref.kind, DocumentKind::Scenario))
        .unwrap();
    let original_bytes = archive.files[snapshot.file_ref.path.as_str()].clone();
    let mut target = Harness::from_value(multiple_source());
    let before = target.object("project", "P1");
    let token = target
        .host
        .select_directory(&target.ui, FilePurpose::ImportPackage, exported)
        .unwrap();
    let task: TaskHandle = ok(target.call(
        "import_package",
        json!({"selected_file_ref":token,"import_policy":"into_project","base_revision":1}),
    ));
    target.finish_one();
    assert!(matches!(
        target.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let _: EditCommit = ok(target.apply(1, json!([{"kind":"adopt_imported_draft","data":{"task_id":task.id,"choices":{"chosen_policy":"into_project","objects_to_create":[],"module_versions":[reference],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"stay"}}}])));
    same_business(target.object("project", "P1"), before);
    let saved = target.tree.directory("imported-seed-saved");
    let task = target.save(2, &saved);
    target.finish_one();
    let state = target.task(&task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let actual = read_project_archive(&saved).unwrap();
    let retained = actual
        .manifest
        .documents
        .iter()
        .find(|entry| entry.file_ref.path == snapshot.file_ref.path)
        .unwrap();
    assert_eq!(
        serde_json::to_value(retained).unwrap(),
        serde_json::to_value(snapshot).unwrap()
    );
    assert_eq!(
        actual.files[retained.file_ref.path.as_str()],
        original_bytes
    );
    assert_eq!(actual.scenarios.len(), 1);
    let tree = Sandbox::new();
    let moved = tree.directory("independent-seed-readback");
    copy(&saved, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    let preview = project_preview(&mut reopened, &reference, Value::Null);
    let exact = preview.source_scenario.unwrap();
    assert_eq!(exact.scenario_id.as_str(), "LOCAL");
    assert_eq!(exact.revision.0.value(), 1);
    let command = prepare_complete(
        &mut reopened,
        &reference,
        "ORDERS",
        Some("ONLY-ORDERS"),
        false,
        true,
    );
    let seed = command.explicit_new_scenario_seed.unwrap();
    assert_eq!(seed.orders.len(), 2);
    assert!(
        seed.initial_conditions.items.is_empty()
            && seed.arrivals.is_empty()
            && seed.supplies.is_empty()
            && seed.quality_presets.is_empty()
    );
}

#[test]
fn complete_multiorder_seed_replays_actual_arrivals_reuses_resources_and_survives_module_export_open()
 {
    let mut h = Harness::from_value(multiple_source());
    let task = h.run("LOCAL");
    h.finish_one();
    let original = h.completed_run(&task);
    assert_eq!(
        original
            .acceptance_events
            .iter()
            .map(|event| event.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 135000]
    );
    let (_, reference) = capture_complete(&mut h, "LOCAL");
    let before = h.object("project", "P1");
    let no_seed = prepare_complete(&mut h, &reference, "NO-SEED", None, false, false);
    assert!(no_seed.explicit_new_scenario_seed.is_none());
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"instantiate_template","data":no_seed}])));
    assert_eq!(
        h.object("project", "P1")["scenario_refs"],
        before["scenario_refs"]
    );
    let before = h.object("project", "P1");
    let command = prepare_complete(&mut h, &reference, "FULL", Some("FULL-SEED"), true, true);
    let seed = command.explicit_new_scenario_seed.as_ref().unwrap();
    assert_eq!(seed.orders.len(), 2);
    assert_eq!(seed.production_tasks.len(), 2);
    assert_eq!(seed.arrivals.len(), 1);
    assert_eq!(seed.initial_conditions.items.len(), 5);
    assert_ne!(
        seed.initial_conditions.items[0].id,
        seed.initial_conditions.items[1].id
    );
    assert_eq!(seed.arrivals[0].at_ms.0.value(), 60000);
    for mutate in ["window", "quantity", "arrival"] {
        let mut forged = command.clone();
        let source = forged.explicit_new_scenario_seed.as_mut().unwrap();
        if mutate == "window" {
            source.window.end_ms = SimTimeMs(SafeUint::try_from(239000u64).unwrap());
        }
        if mutate == "quantity" {
            source.orders.pop();
        }
        if mutate == "arrival" {
            source.arrivals.clear();
        }
        let failure = error(h.apply(3, json!([{"kind":"instantiate_template","data":forged}])));
        assert!(matches!(failure.code, ErrorCode::InvalidInput));
        assert_eq!(h.object("project", "P1"), before);
    }
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"instantiate_template","data":command}])));
    assert_eq!(h.object("project", "P1")["resources"], before["resources"]);
    let task = h.run("FULL-SEED");
    h.finish_one();
    let actual = h.completed_run(&task);
    assert_eq!(
        actual
            .acceptance_events
            .iter()
            .map(|event| event.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 135000]
    );
    assert_eq!(
        serde_json::to_value(h.full(&original.id)).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    let directory = h.tree.directory("full-context-save");
    let save = h.save(4, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("full-context-moved");
    copy(&directory, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    assert_eq!(
        project_preview(&mut reopened, &reference, Value::Null)
            .source_scenario
            .unwrap()
            .revision
            .0
            .value(),
        1
    );
    let again = prepare_complete(
        &mut reopened,
        &reference,
        "AGAIN",
        Some("AGAIN-SEED"),
        true,
        true,
    );
    let _: EditCommit =
        ok(reopened.apply(4, json!([{"kind":"instantiate_template","data":again}])));
    let task = reopened.run("AGAIN-SEED");
    reopened.finish_one();
    let actual = reopened.completed_run(&task);
    assert_eq!(
        actual
            .acceptance_events
            .iter()
            .map(|event| event.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 135000]
    );
    let exported = reopened.tree.directory("private-with-context");
    let token = reopened
        .host
        .select_directory(&reopened.ui, FilePurpose::ExportPackage, exported.clone())
        .unwrap();
    let task:TaskHandle=ok(reopened.call("export_package",json!({"revision":5,"selected_destination_ref":token,"package_scope":{"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[reference],"include_redistributable_assets":true}})));
    reopened.finish_one();
    assert!(matches!(
        reopened.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let exported_archive = read_project_archive(&exported).unwrap();
    assert!(
        exported_archive.scenarios.is_empty() && exported_archive.project.scenario_refs.is_empty()
    );
    assert!(
        exported_archive
            .manifest
            .documents
            .iter()
            .any(|entry| matches!(entry.role, DocumentRole::SupportingFile)
                && matches!(entry.file_ref.kind, DocumentKind::Scenario))
    );
    let tree = Sandbox::new();
    let directory = tree.directory("private-import");
    copy(&exported, &directory);
    let mut imported = Harness::from_seed(tree, &directory);
    let source = project_preview(&mut imported, &reference, Value::Null);
    assert_eq!(
        source.source_scenario.unwrap().scenario_id.as_str(),
        "LOCAL"
    );
    let next = prepare_complete(
        &mut imported,
        &reference,
        "PRIVATE",
        Some("PRIVATE-SEED"),
        true,
        true,
    );
    let base = imported.context().project.unwrap().revision.0.value();
    let _: EditCommit =
        ok(imported.apply(base, json!([{"kind":"instantiate_template","data":next}])));
    let task = imported.run("PRIVATE-SEED");
    imported.finish_one();
    let actual = imported.completed_run(&task);
    assert_eq!(
        actual
            .acceptance_events
            .iter()
            .map(|event| event.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 135000]
    );
}

#[test]
fn composed_setup_policies_merge_existing_target_reuse_actual_resources_and_require_settings_authority()
 {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/production/p104-A-first.json")).unwrap();
    value["project"]["setup_policies"][0]["first_article"] = json!({"route_ref":{"id":"ROUTE-B","revision":"r1"},"family":"B","inspection_module":{"id":"w5.op.inspect","version":"0.1.0"},"gate_id":"B-FIRST-ARTICLE"});
    let mut h = Harness::from_value(value);
    let original_project = h.object("project", "P1");
    let (_, reference) = capture_complete(&mut h, "ORDERS-A");
    let command = prepare_complete(&mut h, &reference, "SETUP", None, false, false);
    assert!(command.explicit_new_scenario_seed.is_none());
    let policies = serde_json::to_value(command.project_setup_policies.as_ref().unwrap()).unwrap();
    assert_eq!(policies.as_array().unwrap().len(), 2);
    assert_eq!(policies[0], original_project["setup_policies"][0]);
    assert_eq!(policies[1]["device_id"], "SETUP:Assembly");
    assert_eq!(
        policies[1]["changeovers"],
        original_project["setup_policies"][0]["changeovers"]
    );
    assert_eq!(
        policies[1]["first_article"]["route_ref"]["id"],
        "SETUP:ROUTE-B"
    );
    assert_eq!(
        policies[1]["first_article"]["inspection_module"],
        original_project["setup_policies"][0]["first_article"]["inspection_module"]
    );
    assert_eq!(
        policies[1]["first_article"]["gate_id"],
        "SETUP:B-FIRST-ARTICLE"
    );
    let before = h.object("project", "P1");
    for omit in [true, false] {
        let mut forged = serde_json::to_value(&command).unwrap();
        if omit {
            forged
                .as_object_mut()
                .unwrap()
                .remove("project_setup_policies");
        } else {
            forged["project_setup_policies"][1]["changeovers"][0]["duration"]["value"] = json!(1);
        }
        assert!(matches!(
            error(h.apply(2, json!([{"kind":"instantiate_template","data":forged}]))).code,
            ErrorCode::InvalidInput
        ));
        assert_eq!(h.object("project", "P1"), before);
    }
    let session = id("template-without-settings-authority");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: session.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::ApplyCommands],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![HostEditGrant {
                id: id("template-source-only"),
                scope: TaskScope::Edit(EditScope::Project(
                    ProjectId::try_from("P1".to_owned()).unwrap(),
                )),
                branches: vec![HostEditBranch::InstantiateTemplate],
                objects: vec![],
                creates: vec![],
                source_versions: vec![reference.clone()],
                reusable_resources: vec![],
                created_objects: vec![],
            }],
        })
        .unwrap();
    let failure = error(h.call_as(
        &session,
        true,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"instantiate_template","data":command}]}),
    ));
    assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    assert_eq!(h.object("project", "P1"), before);
    assert!(
        h.host.caller_context(&session).unwrap().edit_grants[0]
            .created_objects
            .is_empty()
    );
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"instantiate_template","data":command}])));
    let actual = h.object("project", "P1");
    assert_eq!(actual["setup_policies"], policies);
    assert_eq!(actual["resources"], original_project["resources"]);
    assert_eq!(actual["scenario_refs"], original_project["scenario_refs"]);
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"undo"}])));
    assert_eq!(
        h.object("project", "P1")["setup_policies"],
        before["setup_policies"]
    );
    let _: EditCommit = ok(h.apply(4, json!([{"kind":"redo"}])));
    assert_eq!(h.object("project", "P1")["setup_policies"], policies);
    let directory = h.tree.directory("setup-composition");
    let save = h.save(5, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("setup-moved");
    copy(&directory, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    assert_eq!(reopened.object("project", "P1")["setup_policies"], policies);
}

#[test]
fn composition_context_sidecar_is_required_and_multiple_matching_listed_contexts_are_never_guessed()
{
    let mut h = Harness::new();
    let (_, reference) = capture_complete(&mut h, "LOCAL");
    let directory = h.tree.directory("context-source");
    let save = h.save(2, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let archive = read_project_archive(&directory).unwrap();
    let source_entry = archive
        .manifest
        .documents
        .iter()
        .find(|entry| {
            matches!(entry.role, DocumentRole::SupportingFile)
                && matches!(entry.file_ref.kind, DocumentKind::Scenario)
        })
        .unwrap();
    let missing = h.tree.directory("missing-scenario-context");
    copy(&directory, &missing);
    let missing_path = missing.join(source_entry.file_ref.path.as_str());
    assert!(missing_path.starts_with(&missing));
    std::fs::remove_file(missing_path).unwrap();
    let before = h.object("project", "P1");
    let task = h.import(&missing);
    h.finish_one();
    assert!(matches!(
        h.task(&task, false).task.state,
        ComputationStatus::Failed
    ));
    assert_eq!(h.object("project", "P1"), before);
    let duplicate = h.tree.directory("multiple-scenario-contexts");
    copy(&directory, &duplicate);
    let mut context = read(&duplicate.join(source_entry.file_ref.path.as_str()));
    context["data"]["id"] = json!("OTHER-CONTEXT");
    context["data"]["window"]["end_ms"] = json!(239000);
    let duplicate_path = "another-source-context.json";
    write(&duplicate.join(duplicate_path), &context);
    let mut manifest = read(&duplicate.join("project.json"));
    manifest["data"]["documents"]
        .as_array_mut()
        .unwrap()
        .push(document(
            "supporting_file",
            "scenario",
            "scenario",
            "OTHER-CONTEXT",
            duplicate_path,
            json!(1),
        ));
    let module_entry = archive
        .manifest
        .documents
        .iter()
        .find(|entry| {
            entry.module_ref.as_ref().is_some_and(|actual| {
                actual.id == reference.id && actual.version == reference.version
            })
        })
        .unwrap();
    let module_path = duplicate.join(module_entry.file_ref.path.as_str());
    let mut package = read(&module_path);
    package["data"]["files"]
        .as_array_mut()
        .unwrap()
        .push(json!({"path":duplicate_path,"kind":"scenario"}));
    write(&module_path, &package);
    write(&duplicate.join("project.json"), &manifest);
    let tree = Sandbox::new();
    let moved = tree.directory("ambiguous-moved");
    copy(&duplicate, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    let base = reopened.context().project.unwrap().revision;
    let failure=error(reopened.call("get_object",json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":base,"choices":null,"seed":null}},"node_index":null})));
    assert!(matches!(failure.code, ErrorCode::MissingReference));
    assert!(failure.message.contains("template.seed_context_ambiguous"));
    assert_eq!(reopened.object("project", "P1")["revision"], 2);
}

#[test]
fn composed_first_article_uses_actual_selected_inspection_declarations_and_never_invents_pass() {
    let mut value: Value =
        serde_json::from_str(include_str!("fixtures/production/p104-B-first.json")).unwrap();
    value["scenario"]["orders"]
        .as_array_mut()
        .unwrap()
        .retain(|order| order["id"] == "B");
    value["scenario"]["production_tasks"]
        .as_array_mut()
        .unwrap()
        .retain(|task| task["id"] == "TASK-B");
    value["project"]["setup_policies"][0]["first_article"] = json!({"route_ref":{"id":"ROUTE-B","revision":"r1"},"family":"B","inspection_module":{"id":"w5.op.inspect","version":"0.1.0"},"gate_id":"B-FIRST-ARTICLE"});
    let mut h = Harness::from_value(value);
    let source_name = h.object("project", "P1")["scenario_refs"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let old = h.run(&source_name);
    h.finish_one();
    let original = h.completed_run(&old);
    let starts = |run: &RunResult| {
        serde_json::to_value(run).unwrap()["jobs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|job| {
                job["operation_ref"]["id"] == "w5.op.assembly-B"
                    && !job["input_refs"].as_array().unwrap().is_empty()
            })
            .map(|job| job["started_at_ms"].as_u64().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(starts(&original), vec![15000, 55000]);
    let (_, reference) = capture_complete(&mut h, &source_name);
    let unknown = prepare_complete(
        &mut h,
        &reference,
        "UNKNOWN-ARTICLE",
        Some("UNKNOWN-ARTICLE-SEED"),
        true,
        true,
    );
    assert!(
        unknown
            .explicit_new_scenario_seed
            .as_ref()
            .unwrap()
            .quality_presets
            .is_empty()
    );
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"instantiate_template","data":unknown}])));
    let task = h.run("UNKNOWN-ARTICLE-SEED");
    h.finish_one();
    let held = h.completed_run(&task);
    assert_eq!(starts(&held), vec![15000]);
    assert!(held.acceptance_events.is_empty());
    let known = prepare_complete_inspection(
        &mut h,
        &reference,
        "KNOWN-ARTICLE",
        Some("KNOWN-ARTICLE-SEED"),
        true,
        true,
        true,
    );
    assert_eq!(
        known
            .explicit_new_scenario_seed
            .as_ref()
            .unwrap()
            .quality_presets
            .len(),
        4
    );
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"instantiate_template","data":known}])));
    let task = h.run("KNOWN-ARTICLE-SEED");
    h.finish_one();
    let actual = h.completed_run(&task);
    assert_eq!(starts(&actual), vec![15000, 55000]);
    assert_eq!(
        serde_json::to_value(h.full(&original.id)).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    assert_eq!(
        serde_json::to_value(h.full(&held.id)).unwrap(),
        serde_json::to_value(&held).unwrap()
    );
    let directory = h.tree.directory("first-article-save");
    let save = h.save(4, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("first-article-moved");
    copy(&directory, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    assert_eq!(
        serde_json::to_value(reopened.full(&held.id)).unwrap(),
        serde_json::to_value(&held).unwrap()
    );
    let task = reopened.run("KNOWN-ARTICLE-SEED");
    reopened.finish_one();
    let repeated = reopened.completed_run(&task);
    assert_eq!(starts(&repeated), vec![15000, 55000]);
}
