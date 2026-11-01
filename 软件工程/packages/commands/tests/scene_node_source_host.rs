//! Actual Host owner reads. Synthetic geometry proves identity, not visual quality.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::sync::atomic::{AtomicU64, Ordering};
use support::*;

static REQUEST: AtomicU64 = AtomicU64::new(1);
fn reference(version: &str) -> Value {
    json!({"id":"fixture.node.media","version":version})
}
fn presentation(mark: u64) -> Value {
    let template: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/media/module.json"
    ))
    .unwrap();
    let mut p = template["data"]["definition"]["payload"]["data"].clone();
    let mut root = p["nodes"][0].clone();
    root["id"] = json!(91);
    root["name"] = json!("same-name");
    root["parent_index"] = Value::Null;
    root["transform"]["translation"]["x"] = json!(mark as f64);
    root["geometry"] = json!({"kind":"box","data":{"x":mark as f64,"y":0.12,"z":0.8}});
    root["material"]["color_rgba"] = json!([0.1, 0.2, 0.3, 1.0]);
    let mut child = root.clone();
    child["id"] = json!(702);
    child["parent_index"] = json!(0);
    child["transform"]["translation"]["z"] = json!(0.25);
    child["material"]["metallic"] = json!(0.65);
    p["nodes"] = json!([root, child]);
    for field in ["poses", "actions", "timeline", "tags"] {
        p[field] = json!([]);
    }
    p["anchor_roles"] = json!({});
    p["port_anchors"] = json!({});
    p["lod"]["detailed"] = Value::Null;
    p["lod"]["simplified"] = Value::Null;
    serde_json::to_value(serde_json::from_value::<Presentation>(p).unwrap()).unwrap()
}
fn definition(name: &str, version: &str, category: &str, data: Value) -> Value {
    json!({"id":name,"version":version,"title":"owned node fixture","definition_state":"design",
        "contribution":{"author":null,"license":null,"assets":[]},"parameters":[],"attribute_schema":[],
        "payload":{"category":category,"data":data}})
}
fn register(h: &mut Harness, name: &str, version: &str, p: Value, dependencies: Value) {
    let path = h
        .tree
        .directory(&format!("{}-{version}", name.replace('.', "-")));
    write(
        &path.join("module.json"),
        &json!({"schema_version":"1.0","data":{
        "definition":definition(name, version,"media",p),"assets":[],"files":[],
        "dependencies":dependencies,"rights":[],"issues":[]}}),
    );
    h.host.register_bundled_module_directory(&path).unwrap();
}
fn adopt(h: &mut Harness, module: Value, target: bool) {
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base,json!([{"kind":"adopt_module","data":{
        "module_ref":module,"targets":if target {json!([{"kind":"device","id":"ASM-01"}])} else {json!([])},
        "bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
}
fn source(reference: Value) -> Value {
    json!({"kind":"module_version","data":{"module_ref":reference}})
}
fn frozen(run: &RunId, reference: Value) -> Value {
    json!({"kind":"run_module_version","data":{"run_id":run,"at_ms":0,"module_ref":reference}})
}
fn params(kind: &str, name: &str, source: Value, index: Value) -> Value {
    json!({"object_id":{"kind":kind,"id":name},"source":source,"node_index":index})
}
fn scoped(
    h: &mut Harness,
    session: &ObjectId,
    scope: Value,
    project: Value,
    operation: &str,
    params: Value,
) -> Value {
    let request = serde_json::from_value(json!({"api_version":"0.1",
        "request_id":format!("owned-node-{}",REQUEST.fetch_add(1,Ordering::Relaxed)),
        "project_id":project,"scope":scope,"operation":operation,"params":params}))
    .unwrap();
    h.host.dispatch(session, request)
}
fn draft_call(h: &mut Harness, draft: &ModuleId, operation: &str, params: Value) -> Value {
    scoped(
        h,
        &h.ui.clone(),
        json!({"kind":"definition_draft","data":draft}),
        json!("P1"),
        operation,
        params,
    )
}
fn create(h: &mut Harness, definition: Value) -> ModuleId {
    let base = h.context().project.unwrap().revision.0.value();
    let commit: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(scope) = commit.scope else {
        panic!("draft scope")
    };
    scope.id
}
fn node(reply: Value) -> Value {
    let result: DomainObject = ok(reply);
    let DomainObject::SceneNode(node) = result else {
        panic!("SceneNode expected")
    };
    serde_json::to_value(node).unwrap()
}
fn assert_node_failure(reply: Value, code: ErrorCode, pointer: &str) {
    let failure = error(reply);
    assert_eq!(
        format!("{:?}", failure.code),
        format!("{code:?}"),
        "{failure:?}"
    );
    assert!(
        failure.field_errors.iter().any(|f| f.pointer == pointer),
        "{failure:?}"
    );
}
fn external(
    name: &str,
    project: Option<&str>,
    drafts: Vec<ModuleId>,
    operations: Vec<OperationName>,
) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: id(name),
        granted_project: project.map(|p| ProjectId::try_from(p.to_owned()).unwrap()),
        allowed_operations: operations,
        allowed_definition_drafts: drafts,
        selected_file_tokens: vec![],
        edit_grants: vec![],
    }
}

#[test]
fn exact_media_array_position_is_not_node_id_or_name_and_none_is_unchanged() {
    let mut h = Harness::new();
    let p = presentation(11);
    register(&mut h, "fixture.node.media", "1.0.0", p.clone(), json!([]));
    adopt(&mut h, reference("1.0.0"), false);
    let before = h.snapshot();
    let context = serde_json::to_value(h.context()).unwrap();
    for (position, expected) in p["nodes"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            node(h.call(
                "get_object",
                params(
                    "module",
                    "fixture.node.media",
                    source(reference("1.0.0")),
                    json!(position)
                )
            )),
            *expected
        );
    }
    // Duplicate names and non-equal IDs cannot select a different node.
    for index in [2, 91, 702, 9007199254740991u64] {
        assert_node_failure(
            h.call(
                "get_object",
                params(
                    "module",
                    "fixture.node.media",
                    source(reference("1.0.0")),
                    json!(index),
                ),
            ),
            ErrorCode::MissingReference,
            "/params/node_index",
        );
    }
    let parent: DomainObject = ok(h.call(
        "get_object",
        params(
            "module",
            "fixture.node.media",
            source(reference("1.0.0")),
            Value::Null,
        ),
    ));
    let DomainObject::Module(package) = parent else {
        panic!("parent")
    };
    assert_eq!(
        serde_json::to_value(package.definition.payload).unwrap()["data"],
        p
    );
    assert_eq!(h.snapshot(), before);
    assert_eq!(serde_json::to_value(h.context()).unwrap(), context);
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn actual_bundled_table_media_reads_its_own_anchor_node_not_empty_device_shell() {
    let mut h = Harness::new();
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../modules/devices/template-support/media");
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
    let media = json!({"id":"p401.support.media","version":"1.0.0"});
    adopt(&mut h, media.clone(), true);
    let before = h.snapshot();
    let parent: DomainObject = ok(h.call(
        "get_object",
        params(
            "module",
            "p401.support.media",
            source(media.clone()),
            Value::Null,
        ),
    ));
    let DomainObject::Module(package) = parent else {
        panic!("actual package")
    };
    let ModulePayload::Media(presentation) = &package.definition.payload else {
        panic!("actual media")
    };
    let AnchorRef::SceneNode(index) =
        &presentation.anchor_roles[&RoleId::try_from("input".to_owned()).unwrap()]
    else {
        panic!("input node")
    };
    let actual = node(h.call(
        "get_object",
        params("module", "p401.support.media", source(media), json!(index)),
    ));
    assert_eq!(
        actual,
        serde_json::to_value(&presentation.nodes[index.value() as usize]).unwrap()
    );
    assert_eq!(actual["name"], json!("anchor:input"));
    assert_node_failure(
        h.call(
            "get_object",
            params("device", "ASM-01", json!({"kind":"draft"}), json!(index)),
        ),
        ErrorCode::MissingReference,
        "/params/node_index",
    );
    assert_eq!(h.snapshot(), before);
    write(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scenarios/媒体表现/runtime-scene-node-source/actual-bundled-node.json"),
        &json!({"evidence":"actual_Host_existing_bundled_table_media","module_ref":{"id":"p401.support.media","version":"1.0.0"},
        "canonical_index":index,"node":actual,"empty_device_shell_not_followed":true,"native_render_or_user_acceptance":false}),
    );
}

#[test]
fn direct_instance_override_and_resource_own_their_nodes_without_definition_fallback() {
    let mut value = fixture();
    let p = presentation(21);
    value["definitions"][4]["payload"]["data"]["presentation"] = presentation(99);
    value["project"]["devices"][0]["presentation_override"] = p.clone();
    value["project"]["resources"][0]["presentation"] = p.clone();
    let resource = value["project"]["resources"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut h = Harness::from_value(value);
    let before = h.snapshot();
    for (kind, name) in [("device", "ASM-01"), ("resource", resource.as_str())] {
        assert_eq!(
            node(h.call(
                "get_object",
                params(kind, name, json!({"kind":"draft"}), json!(1))
            )),
            p["nodes"][1]
        );
    }
    // PACK-01 has no own override; neither bound definition nor another device is substituted.
    assert_node_failure(
        h.call(
            "get_object",
            params("device", "PACK-01", json!({"kind":"draft"}), json!(0)),
        ),
        ErrorCode::MissingReference,
        "/params/node_index",
    );
    assert_node_failure(
        h.call(
            "get_object",
            params("project", "P1", json!({"kind":"draft"}), json!(0)),
        ),
        ErrorCode::InvalidInput,
        "/params/object_id",
    );
    assert_eq!(h.snapshot(), before);
}

#[test]
fn all_direct_definition_presentations_and_imported_geometry_keep_canonical_owner() {
    let mut h = Harness::new();
    let p = presentation(31);
    let mut device = fixture()["definitions"][4].clone();
    device["payload"]["data"]["presentation"] = p.clone();
    let mut resource = fixture()["definitions"][2].clone();
    resource["payload"]["data"]["presentation"] = p.clone();
    let item = definition(
        "fixture.node.item",
        "1.0.0",
        "items",
        json!({"presentation_or_media_ref":p,
        "unit":"piece","tracking":"individual","spec_ref":null,"attributes":{},"attribute_schema":[]}),
    );
    let transport = definition(
        "fixture.node.transport",
        "1.0.0",
        "transport",
        json!({"ports":[],"builtin":"transport",
        "batch_capacity":null,"resources":[],"times":{"loading":null,"outbound":null,"unloading":null,"returning":null},
        "acceptance_conditions":[],"identity_policy":"preserve_cargo_ids","presentation":p}),
    );
    let mut imported = p.clone();
    imported["nodes"][1]["geometry"] = json!({"kind":"imported_node","data":{
        "asset_ref":{"id":"fixture-glb","path":"fixture.glb","kind":"glb"},
        "node_index":53,"unsupported_extensions":[]}});
    for (i, mut d) in [
        device,
        resource,
        item,
        transport,
        definition("fixture.node.media", "1.0.0", "media", imported.clone()),
    ]
    .into_iter()
    .enumerate()
    {
        if i == 4 {
            d["contribution"]["assets"] = json!([{"asset_ref":"fixture-glb",
                "source":"self-created metadata fixture; no GLB bytes or rendering claimed",
                "license":null,"redistribution_confirmed":false,"missing_reason":"metadata-only fixture has no redistribution grant"}]);
        }
        let expected = if i == 4 {
            &imported["nodes"][1]
        } else {
            &p["nodes"][1]
        };
        let draft = create(&mut h, d.clone());
        let before = h.snapshot();
        assert_eq!(
            node(draft_call(
                &mut h,
                &draft,
                "get_object",
                params(
                    "definition_draft",
                    draft.as_str(),
                    json!({"kind":"draft"}),
                    json!(1)
                )
            )),
            *expected
        );
        assert_eq!(
            node(draft_call(
                &mut h,
                &draft,
                "get_object",
                params(
                    "module",
                    d["id"].as_str().unwrap(),
                    source(json!({"id":d["id"],"version":d["version"]})),
                    json!(1)
                )
            )),
            *expected
        );
        if i == 4 {
            assert_node_failure(
                draft_call(
                    &mut h,
                    &draft,
                    "get_object",
                    params(
                        "definition_draft",
                        draft.as_str(),
                        json!({"kind":"draft"}),
                        json!(53),
                    ),
                ),
                ErrorCode::MissingReference,
                "/params/node_index",
            );
        }
        let whole: DomainObject = ok(draft_call(
            &mut h,
            &draft,
            "get_object",
            params(
                "definition_draft",
                draft.as_str(),
                json!({"kind":"draft"}),
                Value::Null,
            ),
        ));
        let DomainObject::Definition(whole) = whole else {
            panic!("whole definition")
        };
        assert_eq!(
            serde_json::to_value(whole).unwrap(),
            serde_json::to_value(serde_json::from_value::<ModuleDefinition>(d).unwrap()).unwrap()
        );
        assert_eq!(h.snapshot(), before);
    }
    let no_presentation = create(&mut h, fixture()["definitions"][0].clone());
    assert_node_failure(
        draft_call(
            &mut h,
            &no_presentation,
            "get_object",
            params(
                "definition_draft",
                no_presentation.as_str(),
                json!({"kind":"draft"}),
                json!(0),
            ),
        ),
        ErrorCode::InvalidInput,
        "/params/object_id",
    );
}

#[test]
fn nested_media_ref_shell_is_not_an_owner_jump_and_dependencies_remain_exact() {
    let mut h = Harness::new();
    let p = presentation(41);
    register(&mut h, "fixture.node.media", "1.0.0", p.clone(), json!([]));
    let mut shell = p.clone();
    shell["nodes"] = json!([]);
    shell["source"] = json!({"kind":"media_ref","data":reference("1.0.0")});
    register(
        &mut h,
        "fixture.node.shell",
        "1.0.0",
        shell,
        json!([reference("1.0.0")]),
    );
    adopt(
        &mut h,
        json!({"id":"fixture.node.shell","version":"1.0.0"}),
        true,
    );
    let before = h.snapshot();
    assert_node_failure(
        h.call(
            "get_object",
            params("device", "ASM-01", json!({"kind":"draft"}), json!(0)),
        ),
        ErrorCode::MissingReference,
        "/params/node_index",
    );
    assert_node_failure(
        h.call(
            "get_object",
            params(
                "module",
                "fixture.node.shell",
                source(json!({"id":"fixture.node.shell","version":"1.0.0"})),
                json!(0),
            ),
        ),
        ErrorCode::MissingReference,
        "/params/node_index",
    );
    assert_eq!(
        node(h.call(
            "get_object",
            params(
                "module",
                "fixture.node.media",
                source(reference("1.0.0")),
                json!(1)
            )
        )),
        p["nodes"][1]
    );
    assert_node_failure(
        h.call(
            "get_object",
            params(
                "module",
                "fixture.node.shell",
                source(reference("1.0.0")),
                json!(1),
            ),
        ),
        ErrorCode::InvalidInput,
        "/params/source",
    );
    register(
        &mut h,
        "fixture.node.outside",
        "1.0.0",
        presentation(42),
        json!([]),
    );
    // The original Project source admits trusted registered library packages.
    assert_eq!(
        node(h.call(
            "get_object",
            params(
                "module",
                "fixture.node.outside",
                source(json!({"id":"fixture.node.outside","version":"1.0.0"})),
                json!(0)
            )
        )),
        presentation(42)["nodes"][0]
    );
    let mut linked = presentation(43);
    linked["nodes"] = json!([]);
    linked["source"] = json!({"kind":"media_ref","data":reference("1.0.0")});
    let draft = create(
        &mut h,
        definition("fixture.node.linked", "1.0.0", "media", linked),
    );
    assert_eq!(
        node(draft_call(
            &mut h,
            &draft,
            "get_object",
            params(
                "module",
                "fixture.node.media",
                source(reference("1.0.0")),
                json!(1)
            )
        )),
        p["nodes"][1]
    );
    // DefinitionDraft remains limited to its real dependency closure, even
    // though another exact package is registered in this same Host.
    assert_node_failure(
        draft_call(
            &mut h,
            &draft,
            "get_object",
            params(
                "module",
                "fixture.node.outside",
                source(json!({"id":"fixture.node.outside","version":"1.0.0"})),
                json!(0),
            ),
        ),
        ErrorCode::MissingReference,
        "/params/source/data/module_ref",
    );
    assert!(matches!(
        error(h.call(
            "get_object",
            params(
                "module",
                "fixture.node.outside",
                source(json!({"id":"fixture.node.outside","version":"9.0.0"})),
                json!(0)
            )
        ))
        .code,
        ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
}

#[test]
fn old_run_exact_nodes_survive_current_adoption_save_physical_move_and_fresh_host() {
    let mut h = Harness::new();
    let one = presentation(51);
    let two = presentation(52);
    register(
        &mut h,
        "fixture.node.media",
        "1.0.0",
        one.clone(),
        json!([]),
    );
    register(
        &mut h,
        "fixture.node.media",
        "2.0.0",
        two.clone(),
        json!([]),
    );
    adopt(&mut h, reference("1.0.0"), true);
    let task = h.run("LOCAL");
    h.finish_one();
    let old_run = h.completed_run(&task);
    assert_eq!(old_run.acceptance_events[0].at_ms.0.value(), 75000);
    adopt(&mut h, reference("2.0.0"), true);
    let before = h.snapshot();
    assert_eq!(
        node(h.call(
            "get_object",
            params(
                "module",
                "fixture.node.media",
                source(reference("2.0.0")),
                json!(1)
            )
        )),
        two["nodes"][1]
    );
    assert_eq!(
        node(h.call(
            "get_object",
            params(
                "module",
                "fixture.node.media",
                frozen(&old_run.id, reference("1.0.0")),
                json!(1)
            )
        )),
        one["nodes"][1]
    );
    assert!(matches!(
        error(h.call(
            "get_object",
            params(
                "module",
                "fixture.node.media",
                frozen(&old_run.id, reference("2.0.0")),
                json!(0)
            )
        ))
        .code,
        ErrorCode::MissingReference
    ));
    // get_object reads this Run's frozen static declaration; its empty MediaRef
    // wrapper owns no node. ObjectsAt remains the separate Item/Job query.
    let frozen_project: DomainObject = ok(h.call(
        "get_object",
        params(
            "project",
            "P1",
            json!({"kind":"run_snapshot","data":{"run_id":old_run.id,"at_ms":0}}),
            Value::Null,
        ),
    ));
    let DomainObject::Project(frozen_project) = frozen_project else {
        panic!("frozen Project")
    };
    let frozen_device: DomainObject = ok(h.call(
        "get_object",
        params(
            "device",
            "ASM-01",
            json!({"kind":"run_snapshot","data":{"run_id":old_run.id,"at_ms":0}}),
            Value::Null,
        ),
    ));
    let DomainObject::Device(frozen_device) = frozen_device else {
        panic!("frozen Device")
    };
    assert_eq!(
        serde_json::to_value(&frozen_device).unwrap(),
        serde_json::to_value(
            frozen_project
                .devices
                .iter()
                .find(|device| device.id.as_str() == "ASM-01")
                .unwrap()
        )
        .unwrap()
    );
    assert!(matches!(
        error(h.call(
            "get_object",
            params(
                "device",
                "ASM-01",
                json!({"kind":"run_snapshot","data":{"run_id":old_run.id,"at_ms":0}}),
                json!(0)
            )
        ))
        .code,
        ErrorCode::MissingReference
    ));
    assert!(matches!(
        error(h.call(
            "get_run",
            json!({"run_id":old_run.id,"query":{
        "kind":"objects_at","data":{"at_ms":0,"objects":[{"kind":"device","id":"ASM-01"}],
        "page":{"after_index":null,"limit":1}}}})
        ))
        .code,
        ErrorCode::UnsupportedCapability
    ));
    assert_eq!(h.snapshot(), before);
    let saved = h.tree.directory("saved");
    let save = h.save(3, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.path.join("moved");
    std::fs::rename(&saved, &moved).unwrap();
    assert!(!saved.exists());
    let mut reopened = Harness::from_seed(tree, &moved);
    let reopened_before = reopened.snapshot();
    assert_eq!(
        node(reopened.call(
            "get_object",
            params(
                "module",
                "fixture.node.media",
                source(reference("2.0.0")),
                json!(1)
            )
        )),
        two["nodes"][1]
    );
    assert_eq!(
        node(reopened.call(
            "get_object",
            params(
                "module",
                "fixture.node.media",
                frozen(&old_run.id, reference("1.0.0")),
                json!(1)
            )
        )),
        one["nodes"][1]
    );
    assert_eq!(
        serde_json::to_value(reopened.full(&old_run.id)).unwrap(),
        serde_json::to_value(&old_run).unwrap()
    );
    assert_eq!(reopened.object("project", "P1"), before["project"]);
    assert_eq!(reopened.snapshot(), reopened_before);
    write(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scenarios/媒体表现/runtime-scene-node-source/actual-reception.json"),
        &json!({"evidence":"actual_Host_Run_Save_physical_move_fresh_Host","current_node":two["nodes"][1],"old_run_node":one["nodes"][1],
        "frozen_run":old_run,"source_project_unchanged":true,"runtime_device_snapshots_supported":false,"visual_or_native_acceptance":false}),
    );
}

#[test]
fn definition_cycle_nodes_remain_frozen_at_exact_draft_revision_and_scope() {
    let mut h = Harness::new();
    let mut definition = fixture()["definitions"][4].clone();
    let one = presentation(61);
    let two = presentation(62);
    definition["payload"]["data"]["presentation"] = one.clone();
    let draft = create(&mut h, definition.clone());
    let scenario = h.object("scenario", "LOCAL");
    let task: TaskHandle = ok(draft_call(
        &mut h,
        &draft,
        "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":1,
        "observation_window":scenario["window"],"intent":{"kind":"definition_cycle","data":{"definition_revision":1,"test_input":scenario}}}),
    ));
    h.finish_one();
    let state: TaskState = ok(draft_call(
        &mut h,
        &draft,
        "get_task",
        json!({"task_id":task.id}),
    ));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Run(run)) = state.result else {
        panic!("sample")
    };
    definition["payload"]["data"]["presentation"] = two.clone();
    let _: EditCommit = ok(draft_call(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_definition","data":definition}]}),
    ));
    let before = h.snapshot();
    assert_eq!(
        node(draft_call(
            &mut h,
            &draft,
            "get_object",
            params(
                "definition_draft",
                draft.as_str(),
                json!({"kind":"draft"}),
                json!(1)
            )
        )),
        two["nodes"][1]
    );
    assert_eq!(
        node(draft_call(
            &mut h,
            &draft,
            "get_object",
            params(
                "definition_draft",
                draft.as_str(),
                json!({"kind":"run_snapshot","data":{"run_id":run.run_id,"at_ms":0}}),
                json!(1)
            )
        )),
        one["nodes"][1]
    );
    assert_eq!(
        node(draft_call(
            &mut h,
            &draft,
            "get_object",
            params(
                "module",
                "fixture.assembly-station",
                frozen(
                    &run.run_id,
                    json!({"id":"fixture.assembly-station","version":"0.1.0"})
                ),
                json!(1)
            )
        )),
        one["nodes"][1]
    );
    let other = create(&mut h, definition.clone());
    assert!(matches!(
        error(draft_call(
            &mut h,
            &other,
            "get_object",
            params(
                "definition_draft",
                draft.as_str(),
                json!({"kind":"run_snapshot","data":{"run_id":run.run_id,"at_ms":0}}),
                json!(1)
            )
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        error(h.call(
            "get_object",
            params(
                "module",
                "fixture.assembly-station",
                frozen(
                    &run.run_id,
                    json!({"id":"fixture.assembly-station","version":"0.1.0"})
                ),
                json!(1)
            )
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
}

#[test]
fn authorization_errors_precede_owner_details_and_reading_grants_no_run_or_edit_authority() {
    let mut h = Harness::new();
    register(
        &mut h,
        "fixture.node.media",
        "1.0.0",
        presentation(71),
        json!([]),
    );
    adopt(&mut h, reference("1.0.0"), false);
    let draft = create(
        &mut h,
        definition("fixture.node.draft", "1.0.0", "media", presentation(72)),
    );
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    for (name, project, drafts, operations) in [
        (
            "node-read",
            Some("P1"),
            vec![],
            vec![OperationName::GetObject],
        ),
        (
            "node-wrong-project",
            Some("P2"),
            vec![],
            vec![OperationName::GetObject],
        ),
        ("node-no-op", Some("P1"), vec![], vec![]),
        (
            "node-draft-only",
            None,
            vec![draft.clone()],
            vec![OperationName::GetObject],
        ),
        (
            "node-draft-read",
            Some("P1"),
            vec![draft.clone()],
            vec![OperationName::GetObject],
        ),
    ] {
        h.host
            .register_external_session(external(name, project, drafts, operations))
            .unwrap();
    }
    let before = h.snapshot();
    assert_eq!(
        node(h.call_as(
            &id("node-read"),
            true,
            "get_object",
            params(
                "module",
                "fixture.node.media",
                source(reference("1.0.0")),
                json!(0)
            )
        )),
        presentation(71)["nodes"][0]
    );
    for name in ["unregistered-node", "node-wrong-project", "node-no-op"] {
        let failure = error(h.call_as(
            &id(name),
            true,
            "get_object",
            params(
                "module",
                "does-not-exist",
                source(reference("1.0.0")),
                json!(900),
            ),
        ));
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
        assert!(failure.field_errors.is_empty() && failure.object_refs.is_empty());
    }
    for name in ["node-read", "node-draft-only"] {
        assert!(matches!(
            error(scoped(
                &mut h,
                &id(name),
                json!({"kind":"definition_draft","data":draft}),
                json!("P1"),
                "get_object",
                params(
                    "definition_draft",
                    draft.as_str(),
                    json!({"kind":"draft"}),
                    json!(0)
                )
            ))
            .code,
            ErrorCode::PermissionDenied
        ));
    }
    assert_eq!(
        node(scoped(
            &mut h,
            &id("node-draft-read"),
            json!({"kind":"definition_draft","data":draft}),
            json!("P1"),
            "get_object",
            params(
                "definition_draft",
                draft.as_str(),
                json!({"kind":"draft"}),
                json!(0)
            )
        )),
        presentation(72)["nodes"][0]
    );
    assert!(matches!(
        error(h.call_as(
            &id("node-read"),
            true,
            "get_object",
            params(
                "module",
                "fixture.node.media",
                frozen(&run.id, reference("1.0.0")),
                json!(0)
            )
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        error(h.call_as(
            &id("node-read"),
            true,
            "apply_commands",
            json!({"base_revision":2,"commands":[{"kind":"undo"}]})
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        error(h.call_as(
            &id("node-read"),
            true,
            "get_run",
            json!({"run_id":run.id,"query":{"kind":"summary"}})
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    let ui = h.ui.clone();
    assert!(matches!(
        error(scoped(
            &mut h,
            &ui,
            json!({"kind":"project"}),
            json!("P2"),
            "get_object",
            params(
                "module",
                "fixture.node.media",
                source(reference("1.0.0")),
                json!(0)
            )
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        error(h.call_as(
            &h.ui.clone(),
            false,
            "get_object",
            params(
                "module",
                "fixture.node.media",
                source(reference("1.0.0")),
                json!(0)
            )
        ))
        .code,
        ErrorCode::UnsupportedCapability
    ));
    assert_eq!(h.snapshot(), before);
    assert!(h.host.drain_io_work().is_empty());
}
