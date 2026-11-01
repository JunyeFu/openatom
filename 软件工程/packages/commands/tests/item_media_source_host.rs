//! Exact Items adoption is exercised through the real Host and real archives.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{fs, path::Path};
use support::*;

const RED: &[u8] =
    b"<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1' fill='red'/></svg>";
const BLUE: &[u8] =
    b"<svg xmlns='http://www.w3.org/2000/svg'><rect width='1' height='1' fill='blue'/></svg>";

fn reference(version: &str) -> ModuleRef {
    serde_json::from_value(json!({"id":"fixture.exact.item","version":version})).unwrap()
}
fn fixture_package(directory: &Path, version: &str, bytes: &[u8]) {
    let asset = json!({"id":"item-image","path":"assets/item.svg","kind":"svg"});
    let rights = json!([{"asset_ref":"item-image","source":"self-created controlled Item media fixture","license":null,"redistribution_confirmed":false,"missing_reason":"no redistribution grant"}]);
    let template: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/media/module.json"
    ))
    .unwrap();
    let mut presentation = template["data"]["definition"]["payload"]["data"].clone();
    presentation["kind"] = json!("image");
    presentation["source"] = json!({"kind":"asset","data":asset});
    presentation["lod"]["detailed"] = Value::Null;
    presentation["lod"]["simplified"] = Value::Null;
    let dependency = json!({"id":"fixture.item.dependency","version":"1.0.0"});
    write(
        &directory.join("dependency/module.json"),
        &json!({"schema_version":"1.0","data":{
        "definition":{"id":"fixture.item.dependency","version":"1.0.0","title":"owned exact dependency",
            "definition_state":"design","contribution":{"author":null,"license":null,"assets":rights},
            "parameters":[],"attribute_schema":[],"payload":{"category":"media","data":presentation}},
        "assets":[asset],"files":[{"path":"assets/item.svg","kind":"asset"}],
        "dependencies":[],"rights":rights,"issues":[]}}),
    );
    fs::create_dir_all(directory.join("dependency/assets")).unwrap();
    fs::write(directory.join("dependency/assets/item.svg"), RED).unwrap();
    write(
        &directory.join("module.json"),
        &json!({"schema_version":"1.0","data":{
        "definition":{"id":"fixture.exact.item","version":version,"title":"shell",
            "definition_state":"design","contribution":{"author":null,"license":null,"assets":rights},
            "parameters":[],"attribute_schema":[],"payload":{"category":"items","data":{
                "presentation_or_media_ref":presentation,"unit":"piece","tracking":"individual",
                "spec_ref":null,"attributes":{},"attribute_schema":[]}}},
        "assets":[asset],"files":[{"path":"assets/item.svg","kind":"asset"},{"path":"dependency/module.json","kind":"asset"}],
        "dependencies":[dependency],"rights":rights,"issues":[]}}),
    );
    fs::create_dir_all(directory.join("assets")).unwrap();
    fs::write(directory.join("assets/item.svg"), bytes).unwrap();
}
fn adoption(reference: &ModuleRef, targets: Value, new_instances: Value) -> Value {
    json!({"kind":"adopt_module","data":{"module_ref":reference,"targets":targets,
        "bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "new_instances":new_instances,"explicit_seed":null}})
}
fn adopt(h: &mut Harness, version: &str) {
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(
        base,
        json!([adoption(
            &reference(version),
            json!([{"kind":"item_type","id":"shell"}]),
            json!([])
        )]),
    ));
}
fn module(h: &mut Harness, source: ObjectSource, reference: &ModuleRef) -> ModulePackage {
    let value: DomainObject = ok(h.call(
        "get_object",
        json!({"object_id":{"kind":"module","id":reference.id},
        "source":source,"node_index":null}),
    ));
    let DomainObject::Module(package) = value else {
        panic!("exact module")
    };
    *package
}
fn media(h: &mut Harness, source: &ObjectSource, package: &ModulePackage) -> Vec<u8> {
    let current = h.context().project.unwrap().revision;
    h.host
        .read_media_asset(
            &h.ui,
            &ProjectId::try_from("P1".to_owned()).unwrap(),
            &ApiScope::Project,
            &current,
            source,
            &package.assets[0],
        )
        .unwrap()
}
fn source(reference: &ModuleRef) -> ObjectSource {
    ObjectSource::ModuleVersion {
        module_ref: reference.clone(),
    }
}
fn frozen_project(h: &mut Harness, run: &RunId) -> Value {
    let value:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"project","id":"P1"},
        "source":ObjectSource::RunSnapshot{run_id:run.clone(),at_ms:SimTimeMs(SafeUint::try_from(0u64).unwrap())},"node_index":null})));
    let DomainObject::Project(project) = value else {
        panic!("frozen project")
    };
    serde_json::to_value(project).unwrap()
}

#[test]
fn adopted_item_exact_version_history_and_media_survive_move_and_fresh_host() {
    let mut h = Harness::new();
    let old_project = h.object("project", "P1");
    assert!(h.object("item_type", "shell").get("module_ref").is_none());
    let one = h.tree.directory("item-v1");
    let two = h.tree.directory("item-v2");
    fixture_package(&one, "1.0.0", RED);
    fixture_package(&two, "2.0.0", BLUE);
    h.host.register_bundled_module_directory(&one).unwrap();
    h.host.register_bundled_module_directory(&two).unwrap();
    adopt(&mut h, "1.0.0");
    let first_item = h.object("item_type", "shell");
    assert_eq!(first_item["module_ref"], json!(reference("1.0.0")));
    assert!(first_item.get("presentation_or_media_ref").is_none());
    for field in [
        "revision",
        "unit",
        "tracking",
        "spec_ref",
        "attributes",
        "attribute_schema",
    ] {
        assert_eq!(first_item[field], old_project["item_types"][0][field]);
    }
    let first = module(&mut h, source(&reference("1.0.0")), &reference("1.0.0"));
    assert_eq!(
        serde_json::to_value(&first.dependencies).unwrap(),
        json!([{"id":"fixture.item.dependency","version":"1.0.0"}])
    );
    assert_eq!(media(&mut h, &source(&reference("1.0.0")), &first), RED);
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"undo"}])));
    assert!(h.object("item_type", "shell").get("module_ref").is_none());
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"redo"}])));
    assert_eq!(h.object("item_type", "shell"), first_item);
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 75000);
    let frozen = frozen_project(&mut h, &run.id);
    assert_eq!(
        frozen["item_types"][0]["module_ref"],
        json!(reference("1.0.0"))
    );
    assert!(
        run.final_items
            .iter()
            .any(|item| item.item_type_ref.id.as_str() == "shell")
    );
    adopt(&mut h, "2.0.0");
    assert_eq!(
        h.object("item_type", "shell")["module_ref"],
        json!(reference("2.0.0"))
    );
    let second = module(&mut h, source(&reference("2.0.0")), &reference("2.0.0"));
    assert_eq!(media(&mut h, &source(&reference("2.0.0")), &second), BLUE);
    let frozen_source = ObjectSource::RunModuleVersion {
        run_id: run.id.clone(),
        at_ms: SimTimeMs(SafeUint::try_from(0u64).unwrap()),
        module_ref: reference("1.0.0"),
    };
    assert_eq!(media(&mut h, &frozen_source, &first), RED);
    assert_eq!(frozen_project(&mut h, &run.id), frozen);
    let before = h.snapshot();
    let saved = h.tree.directory("saved");
    let task = h.save(5, &saved);
    h.finish_one();
    let saved_state = h.task(&task, true);
    assert!(
        matches!(saved_state.task.state, ComputationStatus::Completed),
        "{saved_state:#?}"
    );
    let tree = Sandbox::new();
    let moved = tree.path.join("physically-moved");
    fs::rename(&saved, &moved).unwrap();
    assert!(!saved.exists());
    let mut reopened = Harness::from_seed(tree, &moved);
    assert_eq!(
        reopened.object("item_type", "shell")["module_ref"],
        json!(reference("2.0.0"))
    );
    assert_eq!(frozen_project(&mut reopened, &run.id), frozen);
    assert_eq!(
        serde_json::to_value(reopened.full(&run.id)).unwrap(),
        serde_json::to_value(&run).unwrap()
    );
    let read1 = module(&mut reopened, frozen_source.clone(), &reference("1.0.0"));
    let read2 = module(
        &mut reopened,
        source(&reference("2.0.0")),
        &reference("2.0.0"),
    );
    assert_eq!(
        serde_json::to_value(&read1).unwrap(),
        serde_json::to_value(&first).unwrap()
    );
    assert_eq!(media(&mut reopened, &frozen_source, &read1), RED);
    assert_eq!(
        media(&mut reopened, &source(&reference("2.0.0")), &read2),
        BLUE
    );
    assert!(read1.rights[0].license.is_none() && !read1.rights[0].redistribution_confirmed);
    assert_eq!(reopened.object("project", "P1"), before["project"]);
    let dependency: ModuleRef =
        serde_json::from_value(json!({"id":"fixture.item.dependency","version":"1.0.0"})).unwrap();
    let dependency_package = module(&mut reopened, source(&dependency), &dependency);
    assert_eq!(
        media(&mut reopened, &source(&dependency), &dependency_package),
        RED
    );
    let evidence = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/媒体表现/runtime-item-media-source/actual-reception.json");
    write(
        &evidence,
        &json!({"evidence":"actual_same_Host_worker_Save_physical_rename_fresh_Host_Open",
        "current_item_type":reopened.object("item_type","shell"),"frozen_run_project":frozen,
        "current_module":read2,"frozen_module":read1,"dependency":dependency_package,
        "run":run,"media_bytes_exact":{"v1":true,"v2":true,"dependency":true},
        "rendered_or_human_accepted":false}),
    );
}

#[test]
fn new_item_adoption_and_failed_batch_are_atomic_without_media_inference() {
    let mut h = Harness::new();
    let original = h.object("item_type", "shell");
    let legacy: ItemType = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(serde_json::to_value(&legacy).unwrap(), original);
    let mut explicit_null = original.clone();
    explicit_null["module_ref"] = Value::Null;
    let legacy: ItemType = serde_json::from_value(explicit_null).unwrap();
    assert!(legacy.module_ref.is_none());
    let directory = h.tree.directory("owned-item");
    fixture_package(&directory, "1.0.0", RED);
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
    // Selecting a module alone never assigns media to a same-name manual type.
    let _: EditCommit = ok(h.apply(
        1,
        json!([adoption(&reference("1.0.0"), json!([]), json!([]))]),
    ));
    assert_eq!(h.object("item_type", "shell"), original);
    let mut new = original.clone();
    new["id"] = json!("new-item");
    new["title"] = json!("shell");
    new["module_ref"] = json!({"id":"forged.other","version":"9.0.0"});
    let _: EditCommit = ok(h.apply(
        2,
        json!([adoption(
            &reference("1.0.0"),
            json!([]),
            json!([{"kind":"item_type","data":new}])
        )]),
    ));
    assert_eq!(
        h.object("item_type", "new-item")["module_ref"],
        json!(reference("1.0.0"))
    );
    assert_eq!(h.object("item_type", "shell"), original);
    let before = h.snapshot();
    let failed = h.apply(
        3,
        json!([
            adoption(
                &reference("1.0.0"),
                json!([{"kind":"item_type","id":"shell"}]),
                json!([])
            ),
            adoption(
                &reference("1.0.0"),
                json!([{"kind":"device","id":"D1"}]),
                json!([])
            )
        ]),
    );
    assert!(matches!(
        error(failed).code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
    let failed = h.apply(
        3,
        json!([adoption(
            &reference("1.0.0"),
            json!([]),
            json!([{"kind":"item_type","data":new}])
        )]),
    );
    assert!(matches!(error(failed).code, ErrorCode::InvalidInput));
    assert_eq!(h.snapshot(), before);
    let failed = h.apply(
        2,
        json!([adoption(
            &reference("1.0.0"),
            json!([{"kind":"item_type","id":"shell"}]),
            json!([])
        )]),
    );
    assert!(matches!(error(failed).code, ErrorCode::RevisionConflict));
    assert_eq!(h.snapshot(), before);
}

fn external(
    session: &str,
    project: &str,
    fields: &[&str],
    source_granted: bool,
) -> HostCallContext {
    let project: ProjectId = ProjectId::try_from(project.to_owned()).unwrap();
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: id(session),
        granted_project: Some(project.clone()),
        allowed_operations: vec![OperationName::GetObject, OperationName::ApplyCommands],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![HostEditGrant {
            id: id("exact-item-grant"),
            scope: TaskScope::Edit(EditScope::Project(project)),
            branches: vec![HostEditBranch::AdoptModule],
            objects: vec![HostObjectEditGrant {
                target: ObjectRef {
                    kind: ObjectKind::ItemType,
                    id: id("shell"),
                },
                fields: fields
                    .iter()
                    .map(|p| HostFieldGrant {
                        pointer: (*p).to_owned(),
                        element_id: None,
                    })
                    .collect(),
                can_remove: false,
            }],
            creates: vec![],
            source_versions: if source_granted {
                vec![reference("1.0.0")]
            } else {
                vec![]
            },
            reusable_resources: vec![],
            created_objects: vec![],
        }],
    }
}

#[test]
fn item_exact_source_changes_require_current_project_source_and_both_module_leaves() {
    let mut h = Harness::new();
    let directory = h.tree.directory("grant-item");
    fixture_package(&directory, "1.0.0", RED);
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
    let before = h.snapshot();
    for (session, project, fields, grant) in [
        (
            "no-source",
            "P1",
            vec!["/module_ref/id", "/module_ref/version"],
            false,
        ),
        ("id-only", "P1", vec!["/module_ref/id"], true),
        ("parent", "P1", vec!["/module_ref"], true),
        (
            "wrong-project",
            "P2",
            vec!["/module_ref/id", "/module_ref/version"],
            true,
        ),
    ] {
        h.host
            .register_external_session(external(session, project, &fields, grant))
            .unwrap();
        let reply = h.call_as(
            &id(session),
            true,
            "apply_commands",
            json!({"base_revision":1,"commands":[
            adoption(&reference("1.0.0"),json!([{"kind":"item_type","id":"shell"}]),json!([]))]}),
        );
        assert!(matches!(error(reply).code, ErrorCode::PermissionDenied));
        assert_eq!(h.snapshot(), before);
        assert!(
            h.host.caller_context(&id(session)).unwrap().edit_grants[0]
                .created_objects
                .is_empty()
        );
    }
    h.host
        .register_external_session(external(
            "exact",
            "P1",
            &["/module_ref/id", "/module_ref/version"],
            true,
        ))
        .unwrap();
    let _: EditCommit = ok(h.call_as(
        &id("exact"),
        true,
        "apply_commands",
        json!({"base_revision":1,"commands":[
        adoption(&reference("1.0.0"),json!([{"kind":"item_type","id":"shell"}]),json!([]))]}),
    ));
    assert_eq!(
        h.object("item_type", "shell")["module_ref"],
        json!(reference("1.0.0"))
    );
    let exact = module(&mut h, source(&reference("1.0.0")), &reference("1.0.0"));
    let project = ProjectId::try_from("P1".to_owned()).unwrap();
    assert!(matches!(
        h.host
            .read_media_asset(
                &id("exact"),
                &project,
                &ApiScope::Project,
                &revision(2),
                &source(&reference("1.0.0")),
                &exact.assets[0]
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
}
