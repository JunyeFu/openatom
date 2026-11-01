//! Actual Host media-byte producer checks. This is not decoding/rendering evidence.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::path::{Path, PathBuf};
use support::*;

const A:&[u8]=b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><path fill='red' d='M0 0h1v1H0z'/></svg>";
const B:&[u8]=b"<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><path fill='blue' d='M0 0h1v1H0z'/></svg>";
fn reference(version: &str) -> ModuleRef {
    serde_json::from_value(json!({"id":"fixture.media.bytes","version":version})).unwrap()
}
fn presentation(asset: &AssetRef) -> Value {
    let document: Value = serde_json::from_str(include_str!(
        "../../../modules/devices/template-support/media/module.json"
    ))
    .unwrap();
    let mut body = document["data"]["definition"]["payload"]["data"].clone();
    body["kind"] = json!("image");
    body["source"] = json!({"kind":"asset","data":asset});
    body["lod"]["detailed"] = Value::Null;
    body["lod"]["simplified"] = Value::Null;
    body
}
fn package(directory: &Path, version: &str) -> PathBuf {
    let assets:Vec<AssetRef>=serde_json::from_value(json!([{"id":"red","path":"assets/red.svg","kind":"svg"},{"id":"blue","path":"assets/blue.svg","kind":"svg"}])).unwrap();
    let mut body = presentation(&assets[0]);
    body["lod"]["detailed"] = json!({"kind":"image_sequence","data":assets});
    body["nodes"][0]["material"]["texture"] = json!(assets[1]);
    let rights:Vec<_>=assets.iter().map(|asset|json!({"asset_ref":asset.id,"source":"tiny self-created byte fixture","license":null,"redistribution_confirmed":false,"missing_reason":"fixture has no product-rights confirmation"})).collect();
    let files: Vec<_> = assets
        .iter()
        .map(|asset| json!({"path":asset.path,"kind":"asset"}))
        .collect();
    write(
        &directory.join("module.json"),
        &json!({"schema_version":"1.0","data":{"definition":{"id":"fixture.media.bytes","version":version,"title":"Tiny controlled byte fixture","definition_state":"design","contribution":{"author":null,"license":null,"assets":rights},"parameters":[],"attribute_schema":[],"payload":{"category":"media","data":body}},"assets":assets,"files":files,"dependencies":[],"rights":rights,"issues":[]}}),
    );
    std::fs::create_dir_all(directory.join("assets")).unwrap();
    std::fs::write(directory.join("assets/red.svg"), A).unwrap();
    std::fs::write(directory.join("assets/blue.svg"), B).unwrap();
    directory.to_owned()
}
fn adopt(h: &mut Harness, reference: &ModuleRef) {
    let base = h.context().project.unwrap().revision;
    let _:EditCommit=ok(h.apply(base.0.value(),json!([{"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
}
fn module(h: &mut Harness, reference: &ModuleRef) -> ModulePackage {
    let value:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null})));
    let DomainObject::Module(package) = value else {
        panic!("actual module")
    };
    *package
}
fn source(reference: &ModuleRef) -> ObjectSource {
    ObjectSource::ModuleVersion {
        module_ref: reference.clone(),
    }
}
fn bytes(
    h: &mut Harness,
    scope: &ApiScope,
    revision: &Revision,
    source: &ObjectSource,
    asset: &AssetRef,
) -> Result<Vec<u8>, ApiError> {
    h.host.read_media_asset(
        &h.ui,
        &ProjectId::try_from("P1".to_owned()).unwrap(),
        scope,
        revision,
        source,
        asset,
    )
}
fn current(h: &mut Harness, reference: &ModuleRef, asset: &AssetRef) -> Result<Vec<u8>, ApiError> {
    let revision = h.context().project.unwrap().revision;
    bytes(h, &ApiScope::Project, &revision, &source(reference), asset)
}
fn scoped(h: &mut Harness, draft: &ModuleId, op: &str, params: Value) -> Value {
    h.host.dispatch(&h.ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"media-owner","project_id":"P1","scope":{"kind":"definition_draft","data":draft},"operation":op,"params":params})).unwrap())
}
fn create(h: &mut Harness, definition: Value) -> ModuleId {
    let base = h.context().project.unwrap().revision.0.value();
    let commit: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"create_definition_draft","data":definition}]),
    ));
    let EditScope::DefinitionDraft(identity) = commit.scope else {
        panic!("actual draft")
    };
    identity.id
}

#[test]
fn media_bundle_payload_asset_sequence_texture_and_unadopted_exact_preview_use_real_bytes() {
    let mut h = Harness::new();
    let directory = h.tree.directory("bundle");
    package(&directory, "1.0.0");
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
    let reference = reference("1.0.0");
    adopt(&mut h, &reference);
    let actual = module(&mut h, &reference);
    let body = serde_json::to_value(&actual.definition.payload).unwrap()["data"].clone();
    assert_eq!(
        body["source"]["data"],
        serde_json::to_value(&actual.assets[0]).unwrap()
    );
    assert_eq!(
        body["lod"]["detailed"]["data"],
        serde_json::to_value(&actual.assets).unwrap()
    );
    assert_eq!(
        body["nodes"][0]["material"]["texture"],
        serde_json::to_value(&actual.assets[1]).unwrap()
    );
    assert_eq!(current(&mut h, &reference, &actual.assets[0]).unwrap(), A);
    let mut preview = Harness::new();
    preview
        .host
        .register_bundled_module_directory(&directory)
        .unwrap();
    let before = preview.snapshot();
    let context_before = serde_json::to_value(preview.context()).unwrap();
    assert_eq!(
        serde_json::to_value(module(&mut preview, &reference)).unwrap(),
        serde_json::to_value(&actual).unwrap()
    );
    assert_eq!(
        current(&mut preview, &reference, &actual.assets[0]).unwrap(),
        A
    );
    assert_eq!(
        current(&mut preview, &reference, &actual.assets[1]).unwrap(),
        B
    );
    assert_eq!(preview.snapshot(), before);
    assert_eq!(
        serde_json::to_value(preview.context()).unwrap(),
        context_before
    );
    assert!(
        actual
            .rights
            .iter()
            .all(|right| right.license.is_none() && !right.redistribution_confirmed)
    );
}

#[test]
fn media_bytes_require_real_ui_scope_revision_and_all_three_asset_fields_without_mutation() {
    let mut h = Harness::new();
    let directory = h.tree.directory("bundle");
    package(&directory, "1.0.0");
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
    let reference = reference("1.0.0");
    adopt(&mut h, &reference);
    let actual = module(&mut h, &reference);
    let before = h.snapshot();
    let context_before = serde_json::to_value(h.context()).unwrap();
    let asset = actual.assets[0].clone();
    for field in ["id", "path", "kind"] {
        let mut wrong = serde_json::to_value(&asset).unwrap();
        wrong[field] = json!(match field {
            "id" => "other",
            "path" => "assets/unknown.svg",
            _ => "png",
        });
        let wrong: AssetRef = serde_json::from_value(wrong).unwrap();
        assert!(matches!(
            current(&mut h, &reference, &wrong).unwrap_err().code,
            ErrorCode::MissingReference
        ));
    }
    let revoked = id("unknown");
    assert!(matches!(
        h.host
            .read_media_asset(
                &revoked,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &ApiScope::Project,
                &revision(2),
                &source(&reference),
                &asset
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    let session = id("external-media");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: session.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::GetObject],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    assert!(matches!(
        h.host
            .read_media_asset(
                &session,
                &ProjectId::try_from("P1".to_owned()).unwrap(),
                &ApiScope::Project,
                &revision(2),
                &source(&reference),
                &asset
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        bytes(
            &mut h,
            &ApiScope::Project,
            &revision(1),
            &source(&reference),
            &asset
        )
        .unwrap_err()
        .code,
        ErrorCode::RevisionConflict
    ));
    assert!(matches!(
        h.host
            .read_media_asset(
                &h.ui,
                &ProjectId::try_from("OTHER".to_owned()).unwrap(),
                &ApiScope::Project,
                &revision(2),
                &source(&reference),
                &asset
            )
            .unwrap_err()
            .code,
        ErrorCode::PermissionDenied
    ));
    assert!(matches!(
        bytes(
            &mut h,
            &ApiScope::Project,
            &revision(2),
            &ObjectSource::Draft,
            &asset
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidInput
    ));
    for path in ["../secret", "C:/secret", "assets\\secret.svg", "/secret"] {
        assert!(RelativeFilePath::try_from(path.to_owned()).is_err());
    }
    assert_eq!(h.snapshot(), before);
    assert_eq!(serde_json::to_value(h.context()).unwrap(), context_before);
}

#[test]
fn media_exact_formal_run_save_move_open_and_private_module_import_keep_original_bytes() {
    let mut h = Harness::new();
    let one = h.tree.directory("one");
    let two = h.tree.directory("two");
    package(&one, "1.0.0");
    package(&two, "2.0.0");
    h.host.register_bundled_module_directory(&one).unwrap();
    h.host.register_bundled_module_directory(&two).unwrap();
    let reference1 = reference("1.0.0");
    let reference2 = reference("2.0.0");
    adopt(&mut h, &reference1);
    let first = module(&mut h, &reference1);
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(), 75000);
    adopt(&mut h, &reference2);
    let second = module(&mut h, &reference2);
    let old = ObjectSource::RunModuleVersion {
        run_id: run.id.clone(),
        at_ms: SimTimeMs(SafeUint::try_from(0u64).unwrap()),
        module_ref: reference1.clone(),
    };
    assert_eq!(
        bytes(
            &mut h,
            &ApiScope::Project,
            &revision(3),
            &old,
            &first.assets[0]
        )
        .unwrap(),
        A
    );
    let absent = ObjectSource::RunModuleVersion {
        run_id: run.id.clone(),
        at_ms: SimTimeMs(SafeUint::try_from(0u64).unwrap()),
        module_ref: reference2.clone(),
    };
    assert!(matches!(
        bytes(
            &mut h,
            &ApiScope::Project,
            &revision(3),
            &absent,
            &second.assets[0]
        )
        .unwrap_err()
        .code,
        ErrorCode::MissingReference
    ));
    let future = ObjectSource::RunModuleVersion {
        run_id: run.id.clone(),
        at_ms: SimTimeMs(SafeUint::try_from(240001u64).unwrap()),
        module_ref: reference1.clone(),
    };
    assert!(matches!(
        bytes(
            &mut h,
            &ApiScope::Project,
            &revision(3),
            &future,
            &first.assets[0]
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidInput
    ));
    let directory = h.tree.directory("saved");
    let task = h.save(3, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("moved");
    copy(&directory, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    assert_eq!(
        bytes(
            &mut reopened,
            &ApiScope::Project,
            &revision(3),
            &old,
            &first.assets[0]
        )
        .unwrap(),
        A
    );
    assert_eq!(
        serde_json::to_value(reopened.full(&run.id)).unwrap(),
        serde_json::to_value(&run).unwrap()
    );
    let exported = reopened.tree.directory("module");
    let token = reopened
        .host
        .select_directory(&reopened.ui, FilePurpose::ExportPackage, exported.clone())
        .unwrap();
    let task:TaskHandle=ok(reopened.call("export_package",json!({"revision":3,"selected_destination_ref":token,"package_scope":{"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[reference1],"include_redistributable_assets":true}})));
    reopened.finish_one();
    assert!(matches!(
        reopened.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("private");
    copy(&exported, &moved);
    let ui = id("whole-w5-ui");
    let mut imported = Harness {
        host: ProjectHost::new(id("whole-w5-host"), ui.clone()),
        ui,
        tree,
    };
    let handle = imported.import(&moved);
    imported.finish_one();
    let draft = imported.imported(&handle);
    // Private-module export retains unresolved rights. Opening that draft is
    // an explicit choice, never an implicit improvement of those rights.
    assert!(!draft.missing.is_empty());
    let _: EditCommit = ok(imported.adopt(&handle, &draft, true));
    assert!(matches!(
        current(&mut imported, &reference1, &first.assets[0])
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    assert!(
        module(&mut imported, &reference1)
            .rights
            .iter()
            .all(|right| !right.redistribution_confirmed && right.license.is_none())
    );

    // Separate, explicitly released owner-created fixture. The previous
    // unresolved version is immutable and remains unresolved above.
    let released = reopened.tree.directory("released");
    package(&released, "3.0.0");
    let mut document = read(&released.join("module.json"));
    for right in document["data"]["rights"].as_array_mut().unwrap() {
        right["license"] = json!("CC0-1.0");
        right["redistribution_confirmed"] = json!(true);
        right["missing_reason"] = Value::Null;
    }
    document["data"]["definition"]["contribution"]["assets"] = document["data"]["rights"].clone();
    write(&released.join("module.json"), &document);
    reopened
        .host
        .register_bundled_module_directory(&released)
        .unwrap();
    let released_ref = reference("3.0.0");
    adopt(&mut reopened, &released_ref);
    let released_package = module(&mut reopened, &released_ref);
    for asset in &released_package.assets {
        reopened
            .host
            .confirm_asset_redistribution(&reopened.ui, &asset.id)
            .unwrap();
    }
    let exported = reopened.tree.directory("released-private");
    let token = reopened
        .host
        .select_directory(&reopened.ui, FilePurpose::ExportPackage, exported.clone())
        .unwrap();
    let handle:TaskHandle=ok(reopened.call("export_package",json!({"revision":4,"selected_destination_ref":token,"package_scope":{"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[released_ref],"include_redistributable_assets":true}})));
    reopened.finish_one();
    assert!(matches!(
        reopened.task(&handle, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("released-private-moved");
    copy(&exported, &moved);
    let mut released_import = Harness::from_seed(tree, &moved);
    assert_eq!(
        current(
            &mut released_import,
            &released_ref,
            &released_package.assets[0]
        )
        .unwrap(),
        A
    );
    assert_eq!(
        current(
            &mut released_import,
            &released_ref,
            &released_package.assets[1]
        )
        .unwrap(),
        B
    );
}

#[test]
fn media_actual_draft_self_dependency_and_definition_cycle_read_exact_frozen_assets() {
    let mut h = Harness::new();
    let directory = h.tree.directory("media");
    package(&directory, "1.0.0");
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
    let media = reference("1.0.0");
    adopt(&mut h, &media);
    let actual = module(&mut h, &media);
    let mut definition = fixture()["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|definition| definition["id"] == "fixture.assembly-station")
        .unwrap()
        .clone();
    definition["payload"]["data"]["presentation"] = presentation(&actual.assets[0]);
    definition["contribution"]["assets"] = serde_json::to_value(&actual.rights).unwrap();
    let own: ModuleRef =
        serde_json::from_value(json!({"id":definition["id"],"version":definition["version"]}))
            .unwrap();
    let draft = create(&mut h, definition.clone());
    let scope = ApiScope::DefinitionDraft(draft.clone());
    assert!(matches!(
        bytes(
            &mut h,
            &scope,
            &revision(2),
            &source(&own),
            &actual.assets[0]
        )
        .unwrap_err()
        .code,
        ErrorCode::RevisionConflict
    ));
    assert_eq!(
        bytes(
            &mut h,
            &scope,
            &revision(1),
            &source(&own),
            &actual.assets[0]
        )
        .unwrap(),
        A
    );
    assert!(matches!(
        bytes(
            &mut h,
            &scope,
            &revision(1),
            &source(&media),
            &actual.assets[0]
        )
        .unwrap_err()
        .code,
        ErrorCode::MissingReference
    ));
    let scenario = h.object("scenario", "LOCAL");
    let task: TaskHandle = ok(scoped(
        &mut h,
        &draft,
        "run_scenario",
        json!({"scenario_id":scenario["id"],"input_revision":1,"observation_window":scenario["window"],"intent":{"kind":"definition_cycle","data":{"definition_revision":1,"test_input":scenario}}}),
    ));
    h.finish_one();
    let state: TaskState = ok(scoped(
        &mut h,
        &draft,
        "get_task",
        json!({"task_id":task.id}),
    ));
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    let Some(TaskOutput::Run(run)) = state.result else {
        panic!("actual sample")
    };
    let frozen = ObjectSource::RunModuleVersion {
        run_id: run.run_id,
        at_ms: SimTimeMs(SafeUint::try_from(0u64).unwrap()),
        module_ref: own.clone(),
    };
    definition["payload"]["data"]["presentation"] = presentation(&actual.assets[1]);
    let _: EditCommit = ok(scoped(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_definition","data":definition}]}),
    ));
    assert_eq!(
        bytes(
            &mut h,
            &scope,
            &revision(2),
            &source(&own),
            &actual.assets[1]
        )
        .unwrap(),
        B
    );
    assert_eq!(
        bytes(&mut h, &scope, &revision(2), &frozen, &actual.assets[0]).unwrap(),
        A
    );
    assert!(matches!(
        bytes(&mut h, &scope, &revision(2), &frozen, &actual.assets[1])
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    definition["payload"]["data"]["presentation"] = presentation(&actual.assets[0]);
    definition["payload"]["data"]["presentation"]["source"] =
        json!({"kind":"media_ref","data":media});
    let _: EditCommit = ok(scoped(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"put_definition","data":definition}]}),
    ));
    assert_eq!(
        bytes(
            &mut h,
            &scope,
            &revision(3),
            &source(&media),
            &actual.assets[0]
        )
        .unwrap(),
        A
    );
    let mut missing = serde_json::to_value(&actual.assets[0]).unwrap();
    missing["id"] = json!("explicit-missing");
    missing["path"] = json!("assets/explicit-missing.svg");
    let missing: AssetRef = serde_json::from_value(missing).unwrap();
    definition["payload"]["data"]["presentation"] = presentation(&missing);
    let mut right = serde_json::to_value(&actual.rights[0]).unwrap();
    right["asset_ref"] = serde_json::to_value(&missing.id).unwrap();
    definition["contribution"]["assets"] = json!([right]);
    let _: EditCommit = ok(scoped(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":3,"commands":[{"kind":"put_definition","data":definition}]}),
    ));
    let before = h.snapshot();
    assert!(matches!(
        bytes(&mut h, &scope, &revision(4), &source(&own), &missing)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
}

#[test]
fn committed_local_media_is_readable_without_adoption_and_new_editing_copy_keeps_old_run() {
    let mut h = Harness::new();
    let run_task = h.run("LOCAL");
    h.finish_one();
    let old_run = h.completed_run(&run_task);
    assert_eq!(old_run.acceptance_events[0].at_ms.0.value(), 75000);
    let directory = h.tree.directory("local-source");
    package(&directory, "1.0.0");
    h.host
        .register_bundled_module_directory(&directory)
        .unwrap();
    adopt(&mut h, &reference("1.0.0"));
    let original = module(&mut h, &reference("1.0.0"));
    let mut definition = serde_json::to_value(&original.definition).unwrap();
    definition["id"] = json!("local.committed.media");
    let draft = create(&mut h, definition);
    let _: EditCommit = ok(scoped(
        &mut h,
        &draft,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"commit_definition_version","data":{"version":"1.1.0"}}]}),
    ));
    let reference: ModuleRef =
        serde_json::from_value(json!({"id":"local.committed.media","version":"1.1.0"})).unwrap();
    let before = h.snapshot();
    assert!(
        !before["project"]["module_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|member| member == &json!(reference))
    );
    let committed = module(&mut h, &reference);
    assert_eq!(current(&mut h, &reference, &original.assets[0]).unwrap(), A);
    assert_eq!(current(&mut h, &reference, &original.assets[1]).unwrap(), B);
    assert_eq!(h.snapshot(), before);
    let editing = create(&mut h, serde_json::to_value(&committed.definition).unwrap());
    assert_eq!(h.snapshot(), before);
    let mut edited = serde_json::to_value(&committed.definition).unwrap();
    edited["title"] = json!("Explicit new editing copy");
    let _: EditCommit = ok(scoped(
        &mut h,
        &editing,
        "apply_commands",
        json!({"base_revision":1,"commands":[{"kind":"put_definition","data":edited}]}),
    ));
    assert_eq!(
        serde_json::to_value(module(&mut h, &reference)).unwrap(),
        serde_json::to_value(&committed).unwrap()
    );
    let _: EditCommit = ok(scoped(
        &mut h,
        &editing,
        "apply_commands",
        json!({"base_revision":2,"commands":[{"kind":"commit_definition_version","data":{"version":"1.2.0"}}]}),
    ));
    assert_eq!(h.object("project", "P1"), before["project"]);
    assert_eq!(
        serde_json::to_value(h.full(&old_run.id)).unwrap(),
        serde_json::to_value(&old_run).unwrap()
    );
    let saved = h.tree.directory("local-saved");
    let save = h.save(2, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let tree = Sandbox::new();
    let moved = tree.directory("local-moved");
    copy(&saved, &moved);
    let mut reopened = Harness::from_seed(tree, &moved);
    assert_eq!(
        serde_json::to_value(module(&mut reopened, &reference)).unwrap(),
        serde_json::to_value(&committed).unwrap()
    );
    assert_eq!(
        current(&mut reopened, &reference, &original.assets[0]).unwrap(),
        A
    );
    assert_eq!(
        serde_json::to_value(reopened.full(&old_run.id)).unwrap(),
        serde_json::to_value(&old_run).unwrap()
    );
    let no_read = id("local-no-read");
    reopened
        .host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: no_read.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::GetContext],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    let request = json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null});
    assert!(matches!(
        error(reopened.call_as(&no_read, true, "get_object", request.clone())).code,
        ErrorCode::PermissionDenied
    ));
    let mut wrong = request.clone();
    wrong["source"]["data"]["module_ref"]["version"] = json!("9.9.9");
    assert!(matches!(
        error(reopened.call("get_object", wrong)).code,
        ErrorCode::MissingReference
    ));
    let created:EditCommit=ok(reopened.call_as(&reopened.ui.clone(),false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"New distinct project","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"discard_after_tasks_terminal"}}]})));
    let req:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"local-after-switch","project_id":created.project_id,"scope":{"kind":"project"},"operation":"get_object","params":request})).unwrap();
    assert!(matches!(
        error(reopened.host.dispatch(&reopened.ui, req)).code,
        ErrorCode::MissingReference
    ));
}
