//! Real Host reads beginning at the device binding used by media/load.ts.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{collections::BTreeSet, fs, path::Path};
use support::*;

const FIRST: &[u8] =
    b"<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1' fill='red'/></svg>";
const SECOND: &[u8] =
    b"<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1' fill='blue'/></svg>";
fn reference(version: &str) -> ModuleRef {
    serde_json::from_value(json!({"id":"fixture.adopted.media","version":version})).unwrap()
}
fn glb() -> Vec<u8> {
    // A real minimal glTF 2 GLB JSON chunk; this test does not decode/render it.
    let mut json =
        br#"{"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[]}],"nodes":[]}"#.to_vec();
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend((20u32 + json.len() as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(json);
    out
}
fn package(dir: &Path, version: &str) {
    let assets = json!([
        {"id":"model","path":"assets/model.glb","kind":"glb"},
        {"id":"image-a","path":"assets/a.svg","kind":"svg"},
        {"id":"image-b","path":"assets/b.svg","kind":"svg"}]);
    let template: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/media/module.json"
    ))
    .unwrap();
    let mut presentation = template["data"]["definition"]["payload"]["data"].clone();
    presentation["kind"] = json!("model");
    presentation["source"] = json!({"kind":"asset","data":assets[1]});
    presentation["lod"]["detailed"] = json!({"kind":"asset","data":assets[0]});
    presentation["lod"]["simplified"] =
        json!({"kind":"image_sequence","data":[assets[1],assets[2]]});
    let rights:Vec<_>=assets.as_array().unwrap().iter().map(|a|json!({"asset_ref":a["id"],"source":"self-created controlled media adoption fixture","license":null,"redistribution_confirmed":false,"missing_reason":"not licensed for redistribution"})).collect();
    write(
        &dir.join("module.json"),
        &json!({"schema_version":"1.0","data":{
        "definition":{"id":"fixture.adopted.media","version":version,"title":"owned Media source",
            "definition_state":"design","contribution":{"author":null,"license":null,"assets":rights},
            "parameters":[],"attribute_schema":[],"payload":{"category":"media","data":presentation}},
        "assets":assets,"files":[{"path":"assets/model.glb","kind":"asset"},{"path":"assets/a.svg","kind":"asset"},{"path":"assets/b.svg","kind":"asset"}],
        "dependencies":[],"rights":rights,"issues":[]}}),
    );
    fs::create_dir_all(dir.join("assets")).unwrap();
    fs::write(dir.join("assets/model.glb"), glb()).unwrap();
    fs::write(
        dir.join("assets/a.svg"),
        if version == "1.0.0" { FIRST } else { SECOND },
    )
    .unwrap();
    fs::write(dir.join("assets/b.svg"), SECOND).unwrap();
}
fn module_request(reference: &ModuleRef, source: ObjectSource) -> Value {
    json!({"object_id":{"kind":"module","id":reference.id},"source":source,"node_index":null})
}
fn get_module(
    h: &mut Harness,
    scope: &ApiScope,
    source: ObjectSource,
    reference: &ModuleRef,
) -> ModulePackage {
    let response = call(
        h,
        &h.ui.clone(),
        scope,
        "get_object",
        module_request(reference, source),
    );
    let DomainObject::Module(module) = ok(response) else {
        panic!("exact module")
    };
    *module
}
fn call(
    h: &mut Harness,
    session: &ObjectId,
    scope: &ApiScope,
    operation: &str,
    params: Value,
) -> Value {
    h.host.dispatch(
        session,
        serde_json::from_value(
            json!({"api_version":"0.1","request_id":"media-adoption-owner",
        "project_id":"P1","scope":scope,"operation":operation,"params":params}),
        )
        .unwrap(),
    )
}
fn source(reference: &ModuleRef, run: Option<&RunId>) -> ObjectSource {
    if let Some(run) = run {
        ObjectSource::RunModuleVersion {
            run_id: run.clone(),
            at_ms: SimTimeMs(SafeUint::try_from(0u64).unwrap()),
            module_ref: reference.clone(),
        }
    } else {
        ObjectSource::ModuleVersion {
            module_ref: reference.clone(),
        }
    }
}
fn adopt(h: &mut Harness, version: &str) {
    let base = h.context().project.unwrap().revision.0.value();
    let _:EditCommit=ok(h.apply(base,json!([{"kind":"adopt_module","data":{"module_ref":reference(version),
        "targets":[{"kind":"device","id":"ASM-01"}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "new_instances":[],"explicit_seed":null}}])));
}
fn new_draft(h: &mut Harness, media: &ModuleRef) -> ModuleId {
    let mut definition = fixture()["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == "fixture.assembly-station")
        .unwrap()
        .clone();
    let template: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/media/module.json"
    ))
    .unwrap();
    let mut presentation = template["data"]["definition"]["payload"]["data"].clone();
    for pointer in ["/source", "/lod/detailed", "/lod/simplified"] {
        *presentation.pointer_mut(pointer).unwrap() = json!({"kind":"media_ref","data":media});
    }
    definition["payload"]["data"]["presentation"] = presentation;
    let base = h.context().project.unwrap().revision.0.value();
    let commit: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(identity) = commit.scope else {
        panic!("draft")
    };
    identity.id
}

// The existing load.ts path: start at the Device's bound exact package, select
// its override, and only change source after following an actual MediaRef.
// It is a Host boundary consumer, not TS/runtime-renderer acceptance evidence.
fn device_media(
    h: &mut Harness,
    project: &Value,
    run: Option<&RunId>,
    simplified: bool,
) -> Result<Vec<Vec<u8>>, ApiError> {
    let device = &project["devices"][0];
    let original: ModuleRef = serde_json::from_value(device["module_ref"].clone()).unwrap();
    let package = get_module(h, &ApiScope::Project, source(&original, run), &original);
    let ModulePayload::Devices(definition) = package.definition.payload else {
        panic!("device")
    };
    let mut presentation: Presentation = if device["presentation_override"].is_null() {
        definition.presentation.unwrap()
    } else {
        serde_json::from_value(device["presentation_override"].clone()).unwrap()
    };
    let mut actual_source = source(&original, run);
    let mut seen = BTreeSet::new();
    let locator = loop {
        let locator = if simplified {
            presentation
                .lod
                .simplified
                .clone()
                .expect("explicit simplified")
        } else {
            presentation
                .lod
                .detailed
                .clone()
                .unwrap_or(presentation.source.clone())
        };
        let MediaLocator::MediaRef(reference) = locator else {
            break locator;
        };
        assert!(seen.insert(serde_json::to_string(&reference).unwrap()));
        actual_source = source(&reference, run);
        let package = get_module(h, &ApiScope::Project, actual_source.clone(), &reference);
        let ModulePayload::Media(value) = package.definition.payload else {
            panic!("media")
        };
        presentation = value;
    };
    let assets = match locator {
        MediaLocator::Asset(asset) => vec![asset],
        MediaLocator::ImageSequence(assets) => assets,
        _ => panic!("controlled bytes"),
    };
    let current = h.context().project.unwrap().revision;
    assets
        .iter()
        .map(|asset| {
            h.host.read_media_asset(
                &h.ui,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &ApiScope::Project,
                &current,
                &actual_source,
                asset,
            )
        })
        .collect()
}

#[test]
fn real_media_adoption_resolves_from_the_original_device_binding_for_both_lods() {
    let mut h = Harness::new();
    let original = h.snapshot();
    let dir = h.tree.directory("media1");
    package(&dir, "1.0.0");
    h.host.register_bundled_module_directory(&dir).unwrap();
    let package1 = get_module(
        &mut h,
        &ApiScope::Project,
        source(&reference("1.0.0"), None),
        &reference("1.0.0"),
    );
    adopt(&mut h, "1.0.0");
    let project = h.object("project", "P1");
    assert_eq!(
        device_media(&mut h, &project, None, false).unwrap(),
        vec![glb()]
    );
    assert_eq!(
        device_media(&mut h, &project, None, true).unwrap(),
        vec![FIRST.to_vec(), SECOND.to_vec()]
    );
    let linked = &project["devices"][0]["presentation_override"];
    for pointer in ["/source", "/lod/detailed", "/lod/simplified"] {
        assert_eq!(
            linked.pointer(pointer).unwrap(),
            &json!({"kind":"media_ref","data":reference("1.0.0")})
        );
    }
    assert!(linked["nodes"].as_array().unwrap().is_empty());
    assert_eq!(
        project["devices"][0]["module_ref"],
        original["project"]["devices"][0]["module_ref"]
    );
    assert_eq!(project["item_types"], original["project"]["item_types"]);
    let device_ref: ModuleRef =
        serde_json::from_value(project["devices"][0]["module_ref"].clone()).unwrap();
    assert!(matches!(
        h.host
            .read_media_asset(
                &h.ui,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &ApiScope::Project,
                &revision(2),
                &source(&device_ref, None),
                &package1.assets[0]
            )
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"undo"}])));
    assert_eq!(
        h.object("device", "ASM-01"),
        original["project"]["devices"][0]
    );
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"redo"}])));
    assert_eq!(h.object("device", "ASM-01"), project["devices"][0]);
    let run_task = h.run("LOCAL");
    h.finish_one();
    let run1 = h.completed_run(&run_task);
    assert_eq!(run1.acceptance_events[0].at_ms.0.value(), 75000);
    let frozen:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"project","id":"P1"},
        "source":ObjectSource::RunSnapshot{run_id:run1.id.clone(),at_ms:SimTimeMs(SafeUint::try_from(0u64).unwrap())},"node_index":null})));
    let DomainObject::Project(frozen) = frozen else {
        panic!("real frozen input")
    };
    let frozen = serde_json::to_value(frozen).unwrap();
    let two = h.tree.directory("media2");
    package(&two, "2.0.0");
    h.host.register_bundled_module_directory(&two).unwrap();
    assert_eq!(h.object("device", "ASM-01"), project["devices"][0]);
    adopt(&mut h, "2.0.0");
    let current = h.object("project", "P1");
    assert_eq!(
        device_media(&mut h, &current, None, true).unwrap(),
        vec![SECOND.to_vec(), SECOND.to_vec()]
    );
    assert_eq!(
        device_media(&mut h, &frozen, Some(&run1.id), true).unwrap(),
        vec![FIRST.to_vec(), SECOND.to_vec()]
    );
    let run_task = h.run("LOCAL");
    h.finish_one();
    let run2 = h.completed_run(&run_task);
    let mut business1 = serde_json::to_value(&run1).unwrap();
    let mut business2 = serde_json::to_value(&run2).unwrap();
    for value in [&mut business1, &mut business2] {
        value.as_object_mut().unwrap().remove("id");
        value.as_object_mut().unwrap().remove("input_ref");
    }
    assert_eq!(
        business1, business2,
        "changing Media cannot change any business result field"
    );
    let before = h.snapshot();
    let saved = h.tree.directory("saved");
    let task = h.save(5, &saved);
    h.finish_one();
    let state = h.task(&task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:#?}"
    );
    let tree = Sandbox::new();
    let moved = tree.path.join("moved");
    fs::rename(&saved, &moved).unwrap();
    assert!(!saved.exists());
    let mut reopened = Harness::from_seed(tree, &moved);
    let current = reopened.object("project", "P1");
    assert_eq!(current, before["project"]);
    for simplified in [false, true] {
        assert_eq!(
            device_media(&mut reopened, &current, None, simplified).unwrap(),
            if simplified {
                vec![SECOND.to_vec(), SECOND.to_vec()]
            } else {
                vec![glb()]
            }
        );
        assert_eq!(
            device_media(&mut reopened, &frozen, Some(&run1.id), simplified).unwrap(),
            if simplified {
                vec![FIRST.to_vec(), SECOND.to_vec()]
            } else {
                vec![glb()]
            }
        );
    }
    assert_eq!(
        serde_json::to_value(reopened.full(&run1.id)).unwrap(),
        serde_json::to_value(&run1).unwrap()
    );
    let old = get_module(
        &mut reopened,
        &ApiScope::Project,
        source(&reference("1.0.0"), Some(&run1.id)),
        &reference("1.0.0"),
    );
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(&package1).unwrap()
    );
    assert!(
        old.rights
            .iter()
            .all(|right| right.license.is_none() && !right.redistribution_confirmed)
    );
    write(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scenarios/媒体表现/runtime-media-adoption-source/actual-adoption.json"),
        &json!({"current_project":current,"frozen_project":frozen,"old_module":old,"run1":run1,"run2":run2,
            "physical_move_fresh_Host":true,"source_derived_from_Device_binding":true,"business_all_fields_equal_except_Run_input_identity":true,
            "actual_TS_or_GPU_or_Native_acceptance":false}),
    );
}

#[test]
fn draft_dependency_metadata_and_bytes_have_the_same_exact_scope() {
    let mut h = Harness::new();
    let dir = h.tree.directory("media1");
    package(&dir, "1.0.0");
    h.host.register_bundled_module_directory(&dir).unwrap();
    let reference = reference("1.0.0");
    let package = get_module(
        &mut h,
        &ApiScope::Project,
        source(&reference, None),
        &reference,
    );
    let draft = new_draft(&mut h, &reference);
    let scope = ApiScope::DefinitionDraft(draft);
    let before = h.snapshot();
    assert_eq!(
        h.host
            .read_media_asset(
                &h.ui,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &scope,
                &revision(1),
                &source(&reference, None),
                &package.assets[0]
            )
            .unwrap(),
        glb()
    );
    let actual = get_module(&mut h, &scope, source(&reference, None), &reference);
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(package).unwrap()
    );
    assert_eq!(h.snapshot(), before);
}

#[test]
fn image_source_without_a_detailed_override_is_resolved_through_the_same_exact_media_link() {
    let mut h = Harness::new();
    let dir = h.tree.directory("image");
    package(&dir, "1.0.0");
    let mut document = read(&dir.join("module.json"));
    document["data"]["definition"]["payload"]["data"]["kind"] = json!("image");
    document["data"]["definition"]["payload"]["data"]["lod"]["detailed"] = Value::Null;
    document["data"]["definition"]["payload"]["data"]["lod"]["simplified"] = Value::Null;
    write(&dir.join("module.json"), &document);
    h.host.register_bundled_module_directory(&dir).unwrap();
    adopt(&mut h, "1.0.0");
    let project = h.object("project", "P1");
    let before = h.snapshot();
    assert_eq!(
        device_media(&mut h, &project, None, false).unwrap(),
        vec![FIRST.to_vec()]
    );
    let actual = get_module(
        &mut h,
        &ApiScope::Project,
        source(&reference("1.0.0"), None),
        &reference("1.0.0"),
    );
    let ModulePayload::Media(media) = actual.definition.payload else {
        panic!("exact image media")
    };
    assert!(
        media.lod.simplified.is_none(),
        "a link cannot invent a simplified LOD on its real source"
    );
    assert_eq!(h.snapshot(), before);
}

#[test]
fn draft_exact_self_transitive_metadata_and_byte_permissions_do_not_fall_back_to_other_scopes() {
    let mut h = Harness::new();
    let dependency = h.tree.directory("dependency");
    package(&dependency, "1.0.0");
    let mut document = read(&dependency.join("module.json"));
    document["data"]["definition"]["id"] = json!("fixture.media.dependency");
    write(&dependency.join("module.json"), &document);
    h.host
        .register_bundled_module_directory(&dependency)
        .unwrap();
    let dep: ModuleRef =
        serde_json::from_value(json!({"id":"fixture.media.dependency","version":"1.0.0"})).unwrap();
    let one = h.tree.directory("one");
    package(&one, "1.0.0");
    let mut document = read(&one.join("module.json"));
    document["data"]["dependencies"] = json!([dep]);
    write(&one.join("module.json"), &document);
    h.host.register_bundled_module_directory(&one).unwrap();
    let two = h.tree.directory("two");
    package(&two, "2.0.0");
    h.host.register_bundled_module_directory(&two).unwrap();
    let draft = new_draft(&mut h, &reference("1.0.0"));
    let scope = ApiScope::DefinitionDraft(draft.clone());
    let unrelated = new_draft(&mut h, &reference("2.0.0"));
    let other = ApiScope::DefinitionDraft(unrelated);
    let own: ModuleRef =
        serde_json::from_value(fixture()["project"]["devices"][0]["module_ref"].clone()).unwrap();
    let own_package = get_module(&mut h, &scope, source(&own, None), &own);
    assert!(matches!(
        own_package.definition.payload,
        ModulePayload::Devices(_)
    ));
    let before = h.snapshot();
    let context = call(
        &mut h,
        &id("whole-w5-ui"),
        &scope,
        "get_context",
        json!({"selection":[]}),
    );
    let package = get_module(&mut h, &scope, source(&dep, None), &dep);
    assert_eq!(
        h.host
            .read_media_asset(
                &h.ui,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &scope,
                &revision(1),
                &source(&dep, None),
                &package.assets[0]
            )
            .unwrap(),
        glb()
    );
    for (scope, reference) in [
        (&scope, reference("2.0.0")),
        (&other, reference("1.0.0")),
        (
            &scope,
            serde_json::from_value(json!({"id":"fixture.adopted.media","version":"9.0.0"}))
                .unwrap(),
        ),
    ] {
        let reply = call(
            &mut h,
            &id("whole-w5-ui"),
            scope,
            "get_object",
            module_request(&reference, source(&reference, None)),
        );
        let failure = error(reply);
        assert!(matches!(failure.code, ErrorCode::MissingReference));
        assert_eq!(
            failure.field_errors[0].pointer,
            "/params/source/data/module_ref"
        );
    }
    let mut wrong = module_request(&dep, source(&dep, None));
    wrong["object_id"]["id"] = json!("other-id");
    assert!(matches!(
        error(call(
            &mut h,
            &id("whole-w5-ui"),
            &scope,
            "get_object",
            wrong
        ))
        .code,
        ErrorCode::InvalidInput
    ));
    assert!(matches!(
        h.host
            .read_media_asset(
                &h.ui,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &scope,
                &revision(2),
                &source(&dep, None),
                &package.assets[0]
            )
            .unwrap_err()
            .code,
        ErrorCode::RevisionConflict
    ));
    for (name, project, allowed, get_object) in [
        ("allowed", "P1", true, true),
        ("no-draft", "P1", false, true),
        ("other-project", "P2", true, true),
        ("no-operation", "P1", true, false),
    ] {
        h.host
            .register_external_session(HostCallContext {
                origin: CallerOrigin::ExternalAgent,
                allow_view_control: false,
                session_id: id(name),
                granted_project: Some(ProjectId::try_from(project.to_owned()).unwrap()),
                allowed_operations: if get_object {
                    vec![OperationName::GetObject]
                } else {
                    vec![]
                },
                allowed_definition_drafts: if allowed { vec![draft.clone()] } else { vec![] },
                selected_file_tokens: vec![],
                edit_grants: vec![],
            })
            .unwrap();
        let reply = call(
            &mut h,
            &id(name),
            &scope,
            "get_object",
            module_request(&dep, source(&dep, None)),
        );
        if name == "allowed" {
            let value: DomainObject = ok(reply);
            assert!(matches!(value, DomainObject::Module(_)));
        } else {
            assert!(matches!(error(reply).code, ErrorCode::PermissionDenied));
        }
        assert!(matches!(
            h.host
                .read_media_asset(
                    &id(name),
                    &ProjectId::try_from("P1".to_owned()).unwrap(),
                    &scope,
                    &revision(1),
                    &source(&dep, None),
                    &package.assets[0]
                )
                .unwrap_err()
                .code,
            ErrorCode::PermissionDenied
        ));
    }
    assert_eq!(
        call(
            &mut h,
            &id("whole-w5-ui"),
            &scope,
            "get_context",
            json!({"selection":[]})
        ),
        context
    );
    assert_eq!(h.snapshot(), before);
}

#[test]
fn explicit_media_upgrade_requires_exact_source_and_all_three_lod_reference_leaves_atomically() {
    let mut h = Harness::new();
    let one = h.tree.directory("one");
    let two = h.tree.directory("two");
    package(&one, "1.0.0");
    package(&two, "2.0.0");
    h.host.register_bundled_module_directory(&one).unwrap();
    h.host.register_bundled_module_directory(&two).unwrap();
    adopt(&mut h, "1.0.0");
    let fields = [
        "/presentation_override/source/data/version",
        "/presentation_override/lod/detailed/data/version",
        "/presentation_override/lod/simplified/data/version",
    ];
    for (name, count, source_grant) in [
        ("partial", 2, true),
        ("no-source", 3, false),
        ("all", 3, true),
    ] {
        h.host
            .register_external_session(HostCallContext {
                origin: CallerOrigin::ExternalAgent,
                allow_view_control: false,
                session_id: id(name),
                granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
                allowed_operations: vec![OperationName::ApplyCommands],
                allowed_definition_drafts: vec![],
                selected_file_tokens: vec![],
                edit_grants: vec![HostEditGrant {
                    id: id("media-upgrade"),
                    scope: TaskScope::Edit(EditScope::Project(
                        ProjectId::try_from("P1".to_owned()).unwrap(),
                    )),
                    branches: vec![HostEditBranch::AdoptModule],
                    objects: vec![HostObjectEditGrant {
                        target: ObjectRef {
                            kind: ObjectKind::Device,
                            id: id("ASM-01"),
                        },
                        fields: fields[..count]
                            .iter()
                            .map(|pointer| HostFieldGrant {
                                pointer: (*pointer).to_owned(),
                                element_id: None,
                            })
                            .collect(),
                        can_remove: false,
                    }],
                    creates: vec![],
                    source_versions: if source_grant {
                        vec![reference("2.0.0")]
                    } else {
                        vec![]
                    },
                    reusable_resources: vec![],
                    created_objects: vec![],
                }],
            })
            .unwrap();
    }
    let command = json!({"kind":"adopt_module","data":{"module_ref":reference("2.0.0"),"targets":[{"kind":"device","id":"ASM-01"}],
        "bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}});
    let before = h.snapshot();
    for session in ["partial", "no-source"] {
        let reply = call(
            &mut h,
            &id(session),
            &ApiScope::Project,
            "apply_commands",
            json!({"base_revision":2,"commands":[command]}),
        );
        assert!(matches!(error(reply).code, ErrorCode::PermissionDenied));
        assert_eq!(h.snapshot(), before);
    }
    let mut forbidden = h.object("device", "PACK-01");
    forbidden["title"] = json!("not granted");
    let reply = call(
        &mut h,
        &id("all"),
        &ApiScope::Project,
        "apply_commands",
        json!({"base_revision":2,"commands":[command,
        {"kind":"put_object","data":{"kind":"device","data":forbidden}}]}),
    );
    assert!(matches!(error(reply).code, ErrorCode::PermissionDenied));
    assert_eq!(h.snapshot(), before);
    let _: EditCommit = ok(call(
        &mut h,
        &id("all"),
        &ApiScope::Project,
        "apply_commands",
        json!({"base_revision":2,"commands":[command]}),
    ));
    let current = h.object("project", "P1");
    assert_eq!(
        device_media(&mut h, &current, None, true).unwrap(),
        vec![SECOND.to_vec(), SECOND.to_vec()]
    );
    assert!(
        h.host.caller_context(&id("all")).unwrap().edit_grants[0]
            .created_objects
            .is_empty()
    );
}
