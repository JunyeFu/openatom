//! Actual IntoProject definition import; archive business state is never adopted.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::path::Path;
use support::*;

fn item_definition() -> Value {
    json!({"id":"native.import.item","version":"0.1.0","title":"Local import test",
        "definition_state":"design","contribution":{"author":null,"license":"LicenseRef-Local-Test-Only","assets":[]},
        "parameters":[],"attribute_schema":[],"payload":{"category":"items","data":{
            "presentation_or_media_ref":null,"unit":"piece","tracking":null,"spec_ref":null,"attributes":{},"attribute_schema":[]}}})
}
fn source(h: &Harness, name: &str, definition: Value) -> std::path::PathBuf {
    let path = h.tree.directory(name);
    let mut value = fixture();
    value["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":definition["id"],"version":definition["version"]}));
    value["definitions"]
        .as_array_mut()
        .unwrap()
        .push(definition);
    seed(&path, &value);
    path
}
fn stage(h: &mut Harness, path: &Path) -> TaskHandle {
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::ImportPackage, path.to_owned())
        .unwrap();
    let base = h.context().project.unwrap().revision;
    ok(h.call(
        "import_package",
        json!({"selected_file_ref":selected,"import_policy":"into_project","base_revision":base}),
    ))
}
fn proposal(h: &mut Harness, task: &TaskHandle) -> ImportDraft {
    h.finish_one();
    let state = h.task(task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let Some(TaskOutput::Import(draft)) = state.result else {
        panic!("actual import proposal")
    };
    draft
}
fn adopt(h: &mut Harness, task: &TaskHandle, refs: Value) -> Value {
    let base = h.context().project.unwrap().revision.0.value();
    h.apply(
        base,
        json!([{"kind":"adopt_imported_draft","data":{"task_id":task.id,"choices":{
        "chosen_policy":"into_project","objects_to_create":[],"module_versions":refs,
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "selected_assets":[],"accept_missing_as_draft":false,"departure":"stay"}}}]),
    )
}

#[test]
fn missing_asset_contribution_requires_explicit_acceptance_and_survives_save_reopen() {
    let mut h = Harness::new();
    let path = source(&h, "missing-asset-source", item_definition());
    let mut manifest = read(&path.join("project.json"));
    let module_path = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["module_ref"]["id"] == "native.import.item")
        .unwrap()["file_ref"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let asset = json!({"id":"native-import-asset","path":"media/local.svg","kind":"svg"});
    let rights = json!({"asset_ref":"native-import-asset","source":"Self-created local fixture","license":null,"redistribution_confirmed":false,"missing_reason":"No redistribution confirmation"});
    let missing = json!({"code":"handoff.asset_not_redistributed","object_ref":{"kind":"asset","id":"native-import-asset"},"field_path":"media/local.svg","message":"Restricted asset bytes omitted","blocks":["preview"]});
    let mut entry = document(
        "asset",
        "asset",
        "asset",
        "native-import-asset",
        "media/local.svg",
        Value::Null,
    );
    entry["required"] = json!(false);
    manifest["data"]["documents"]
        .as_array_mut()
        .unwrap()
        .push(entry);
    manifest["data"]["assets"] = json!([rights]);
    manifest["data"]["missing"] = json!([missing]);
    let mut package = read(&path.join(&module_path));
    let mut presentation = read(Path::new(
        "../../modules/devices/template-support/media/module.json",
    ))["data"]["definition"]["payload"]["data"]
        .clone();
    presentation["kind"] = json!("image");
    presentation["source"] = json!({"kind":"asset","data":asset});
    presentation["lod"]["detailed"] = Value::Null;
    presentation["lod"]["simplified"] = Value::Null;
    package["data"]["definition"]["payload"] = json!({"category":"media","data":presentation});
    package["data"]["definition"]["contribution"]["assets"] = json!([rights]);
    package["data"]["assets"] = json!([asset]);
    // Actual authored packages declare media in assets, independently of files.
    package["data"]["files"] = json!([]);
    package["data"]["rights"] = json!([rights]);
    write(&path.join(&module_path), &package);
    write(&path.join("project.json"), &manifest);
    let handle = stage(&mut h, &path);
    let draft = proposal(&mut h, &handle);
    assert!(
        draft
            .missing
            .iter()
            .any(|issue| issue.code == "handoff.asset_not_redistributed")
    );
    let before = h.object("project", "P1");
    let accept = |allow| {
        json!([{"kind":"adopt_imported_draft","data":{"task_id":handle.id,"choices":{
        "chosen_policy":"into_project","objects_to_create":[],"module_versions":[{"id":"native.import.item","version":"0.1.0"}],
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "selected_assets":["native-import-asset"],"accept_missing_as_draft":allow,"departure":"stay"}}}])
    };
    error(h.apply(1, accept(false)));
    assert_eq!(h.object("project", "P1"), before);
    let _: EditCommit = ok(h.apply(1, accept(true)));
    let mut actual = h.object("project", "P1");
    assert!(actual["issues"].as_array().unwrap().contains(&missing));
    actual["issues"]
        .as_array_mut()
        .unwrap()
        .retain(|issue| issue != &missing);
    same_business(actual, before);
    let saved = h.tree.directory("missing-asset-saved");
    let save = h.save(2, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).result,
        Some(TaskOutput::Save(_))
    ));
    assert!(!saved.join("media/local.svg").exists());
    let manifest = read(&saved.join("project.json"));
    assert_eq!(manifest["data"]["assets"], json!([rights]));
    assert!(
        manifest["data"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["file_ref"]["path"] == "media/local.svg"
                && entry["required"] == false)
    );
    let open = h.import(&saved);
    h.finish_one();
    let draft = h.imported(&open);
    assert!(!draft.missing.is_empty());
    let _: EditCommit = ok(h.adopt(&open, &draft, true));
    let module: DomainObject = ok(h.call("get_object",json!({"object_id":{"kind":"module","id":"native.import.item"},"source":{"kind":"module_version","data":{"module_ref":{"id":"native.import.item","version":"0.1.0"}}},"node_index":null})));
    let expected: ModulePackage = serde_json::from_value(package["data"].clone()).unwrap();
    assert!(
        matches!(module, DomainObject::Module(value) if serde_json::to_value(&value).unwrap() == serde_json::to_value(expected).unwrap())
    );
    let saved_again = h.tree.directory("missing-asset-resaved");
    let save = h.save(2, &saved_again);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).result,
        Some(TaskOutput::Save(_))
    ));
    assert!(!saved_again.join("media/local.svg").exists());
}

#[test]
fn into_project_previews_then_imports_only_the_selected_exact_definition_and_survives_save_open() {
    let mut h = Harness::new();
    let original_run = h.run("LOCAL");
    h.finish_one();
    let original_run = h.completed_run(&original_run);
    let before = h.object("project", "P1");
    let scenario = h.object("scenario", "LOCAL");
    let definition = item_definition();
    let path = source(&h, "source", definition.clone());
    let handle = stage(&mut h, &path);
    assert_eq!(handle.input_revision.0.value(), 1);
    assert!(matches!(
        handle.scope,
        TaskScope::Edit(EditScope::Project(_))
    ));
    let draft = proposal(&mut h, &handle);
    assert!(draft.proposed_objects.is_empty());
    assert!(
        draft
            .proposed_data
            .iter()
            .all(|object| matches!(object, DomainObject::Module(_)))
    );
    assert!(draft.proposed_data.iter().any(|object| matches!(object, DomainObject::Module(package) if package.definition.id.as_str()=="native.import.item")));
    assert_eq!(h.object("project", "P1"), before);
    assert_eq!(h.object("scenario", "LOCAL"), scenario);
    let commit: EditCommit = ok(adopt(
        &mut h,
        &handle,
        json!([{"id":"native.import.item","version":"0.1.0"}]),
    ));
    assert_eq!(commit.new_revision.0.value(), 2);
    assert!(!commit.history.can_undo && !commit.history.can_redo);
    same_business(h.object("project", "P1"), before.clone());
    same_business(h.object("scenario", "LOCAL"), scenario);
    assert_eq!(
        serde_json::to_value(h.full(&original_run.id)).unwrap(),
        serde_json::to_value(&original_run).unwrap()
    );
    let saved = h.tree.directory("saved");
    let save = h.save(2, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).result,
        Some(TaskOutput::Save(_))
    ));
    let manifest = read(&saved.join("project.json"));
    let selected: Vec<_> = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["module_ref"]["id"] == "native.import.item")
        .collect();
    assert_eq!(selected.len(), 1);
    assert_eq!(
        read(&saved.join(selected[0]["file_ref"]["path"].as_str().unwrap()))["data"]["definition"],
        definition
    );
    let open = h.import(&saved);
    h.finish_one();
    let draft = h.imported(&open);
    let _: EditCommit = ok(h.adopt(&open, &draft, false));
    let module: DomainObject = ok(h.call("get_object",json!({"object_id":{"kind":"module","id":"native.import.item"},"source":{"kind":"module_version","data":{"module_ref":{"id":"native.import.item","version":"0.1.0"}}},"node_index":null})));
    assert!(
        matches!(module,DomainObject::Module(package) if serde_json::to_value(&package.definition).unwrap()==definition)
    );
}

#[test]
fn stale_import_proposal_and_unknown_selected_version_are_rejected_without_partial_adoption() {
    let mut h = Harness::new();
    let path = source(&h, "source", item_definition());
    let handle = stage(&mut h, &path);
    let _ = proposal(&mut h, &handle);
    let before = h.object("project", "P1");
    let failure = error(adopt(
        &mut h,
        &handle,
        json!([{"id":"native.import.item","version":"9.9.9"}]),
    ));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert_eq!(h.object("project", "P1"), before);
    let mut other = before["item_types"][0].clone();
    other["id"] = json!("OTHER");
    other["title"] = json!("Other");
    let _: EditCommit = ok(h.apply(1, json!([put("item_type", other)])));
    let before = h.object("project", "P1");
    let failure = error(adopt(
        &mut h,
        &handle,
        json!([{"id":"native.import.item","version":"0.1.0"}]),
    ));
    assert!(matches!(failure.code, ErrorCode::RevisionConflict));
    assert_eq!(h.object("project", "P1"), before);
}

#[test]
fn conflicting_existing_exact_version_prevents_the_entire_selected_import() {
    let mut h = Harness::new();
    let mut value = fixture();
    value["definitions"][0]["title"] = json!("Conflicting immutable bytes");
    let conflicting =
        json!({"id":value["definitions"][0]["id"],"version":value["definitions"][0]["version"]});
    let definition = item_definition();
    value["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":definition["id"],"version":definition["version"]}));
    value["definitions"]
        .as_array_mut()
        .unwrap()
        .push(definition);
    let path = h.tree.directory("conflicting");
    seed(&path, &value);
    let handle = stage(&mut h, &path);
    let draft = proposal(&mut h, &handle);
    assert!(!draft.conflicts.is_empty());
    let before = h.object("project", "P1");
    error(adopt(
        &mut h,
        &handle,
        json!([{"id":"native.import.item","version":"0.1.0"},conflicting]),
    ));
    assert_eq!(h.object("project", "P1"), before);
    error(h.call("get_object",json!({"object_id":{"kind":"module","id":"native.import.item"},"source":{"kind":"module_version","data":{"module_ref":{"id":"native.import.item","version":"0.1.0"}}},"node_index":null})));
}

#[test]
fn import_adoption_requires_the_actual_ui_and_rejects_companion_path_conflicts_atomically() {
    let mut h = Harness::new();
    let path = source(&h, "file-conflict", item_definition());
    let manifest = read(&path.join("project.json"));
    let module = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    let module_path = path.join(module["file_ref"]["path"].as_str().unwrap());
    let mut package = read(&module_path);
    package["data"]["files"] = json!([{"path":"module-0.json","kind":"module"}]);
    write(&module_path, &package);
    let handle = stage(&mut h, &path);
    let _ = proposal(&mut h, &handle);
    let before = h.object("project", "P1");
    let caller = id("module-import-agent");
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: false,
            session_id: caller.clone(),
            granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
            allowed_operations: vec![OperationName::ApplyCommands],
            allowed_definition_drafts: vec![],
            selected_file_tokens: vec![],
            edit_grants: vec![],
        })
        .unwrap();
    let denied = error(h.call_as(&caller, true, "apply_commands", json!({"base_revision":1,"commands":[{
        "kind":"adopt_imported_draft","data":{"task_id":handle.id,"choices":{
            "chosen_policy":"into_project","objects_to_create":[],"module_versions":[{"id":"native.import.item","version":"0.1.0"}],
            "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
            "selected_assets":[],"accept_missing_as_draft":false,"departure":"stay"}}}]})));
    assert!(matches!(denied.code, ErrorCode::PermissionDenied));
    let conflict = error(adopt(
        &mut h,
        &handle,
        json!([{"id":"native.import.item","version":"0.1.0"}]),
    ));
    assert!(matches!(conflict.code, ErrorCode::InvalidInput));
    assert_eq!(h.object("project", "P1"), before);
    error(h.call("get_object",json!({"object_id":{"kind":"module","id":"native.import.item"},"source":{"kind":"module_version","data":{"module_ref":{"id":"native.import.item","version":"0.1.0"}}},"node_index":null})));
}

#[test]
fn actual_asset_bytes_and_unknown_rights_are_selected_explicitly_and_preserved() {
    let mut h = Harness::new();
    let path = source(&h, "asset-source", item_definition());
    let mut manifest = read(&path.join("project.json"));
    let entries = manifest["data"]["documents"].as_array_mut().unwrap();
    let module_path = entries.last().unwrap()["file_ref"]["path"]
        .as_str()
        .unwrap()
        .to_owned();
    let rights = json!({"asset_ref":"native-import-asset","source":null,"license":null,"redistribution_confirmed":false,"missing_reason":"Local fixture; redistribution unknown"});
    entries.push(document(
        "asset",
        "asset",
        "asset",
        "native-import-asset",
        "media/local.svg",
        Value::Null,
    ));
    manifest["data"]["assets"] = json!([rights]);
    let mut package = read(&path.join(&module_path));
    package["data"]["assets"] =
        json!([{"id":"native-import-asset","path":"media/local.svg","kind":"svg"}]);
    // Host-authored media keeps AssetRef separately from companion FileRefs.
    package["data"]["files"] = json!([]);
    package["data"]["rights"] = json!([rights]);
    package["data"]["definition"]["contribution"]["assets"] = json!([rights]);
    write(&path.join(&module_path), &package);
    write(&path.join("project.json"), &manifest);
    std::fs::create_dir(path.join("media")).unwrap();
    let bytes = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"></svg>"#;
    std::fs::write(path.join("media/local.svg"), bytes).unwrap();
    let handle = stage(&mut h, &path);
    let draft = proposal(&mut h, &handle);
    let proposed = draft
        .proposed_data
        .iter()
        .find_map(|object| match object {
            DomainObject::Module(package)
                if package.definition.id.as_str() == "native.import.item" =>
            {
                Some(package)
            }
            _ => None,
        })
        .unwrap();
    assert!(!proposed.rights[0].redistribution_confirmed);
    let before = h.object("project", "P1");
    error(adopt(
        &mut h,
        &handle,
        json!([{"id":"native.import.item","version":"0.1.0"}]),
    ));
    assert_eq!(h.object("project", "P1"), before);
    let _:EditCommit=ok(h.apply(1,json!([{"kind":"adopt_imported_draft","data":{"task_id":handle.id,"choices":{
        "chosen_policy":"into_project","objects_to_create":[],"module_versions":[{"id":"native.import.item","version":"0.1.0"}],
        "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "selected_assets":["native-import-asset"],"accept_missing_as_draft":false,"departure":"stay"}}}])));
    let saved = h.tree.directory("asset-saved");
    let handle = h.save(2, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&handle, true).result,
        Some(TaskOutput::Save(_))
    ));
    assert_eq!(std::fs::read(saved.join("media/local.svg")).unwrap(), bytes);
    let manifest = read(&saved.join("project.json"));
    assert_eq!(manifest["data"]["assets"], json!([rights]));
    let module = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["module_ref"]["id"] == "native.import.item")
        .unwrap();
    assert_eq!(
        read(&saved.join(module["file_ref"]["path"].as_str().unwrap()))["data"],
        package["data"]
    );
}
