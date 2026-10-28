//! Bounded P0-01/P3-01 consumer: real CC0 media contribution ZIP, move and new OS.
//! Host/program evidence; no Native picker, rendered-quality or human claim.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::json;
use sf_commands::*;
use sf_domain::*;
use std::{fs, path::{Path, PathBuf}, process::Command};
use support::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().unwrap()
}
fn reference(package: &ModulePackage) -> ModuleRef {
    ModuleRef { id: package.definition.id.clone(), version: package.definition.version.clone() }
}
fn module(h: &mut Harness, reference: &ModuleRef) -> ModulePackage {
    let object: DomainObject = ok(h.call("get_object", json!({
        "object_id":{"kind":"module","id":reference.id},
        "source":{"kind":"module_version","data":{"module_ref":reference}},"node_index":null
    })));
    let DomainObject::Module(package) = object else { panic!("exact module result") };
    *package
}
fn asset_bytes(h: &mut Harness, reference: &ModuleRef, asset: &AssetRef) -> Vec<u8> {
    let project = h.context().project.unwrap();
    h.host.read_media_asset(&h.ui, &project.id, &ApiScope::Project, &project.revision,
        &ObjectSource::ModuleVersion { module_ref: reference.clone() }, asset).unwrap()
}
fn completed_package(h: &mut Harness, task: &TaskHandle) -> ExportPackage {
    h.finish_one();
    let state = h.task(task, true);
    assert!(matches!(state.task.state, ComputationStatus::Completed), "{state:?}");
    let TaskOutput::Package(package) = state.result.unwrap() else { panic!("package output") };
    package
}

#[test]
fn cc0_media_exact_version_matches_after_archive_roundtrip() {
    let source_root = root().join("modules/media/equipment-quality-v041/installable");
    let mut writer = Harness::new();
    for name in ["assembly-simple", "assembly"] {
        writer.host.register_bundled_module_directory(&source_root.join(name)).unwrap();
    }
    let selected = ModuleRef {
        id: "p302.line-grade.assembly".to_owned().try_into().unwrap(),
        version: "0.4.1".to_owned().try_into().unwrap(),
    };
    let revision = writer.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(writer.apply(revision, json!([{"kind":"adopt_module","data":{
        "module_ref":selected,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "new_instances":[],"explicit_seed":null
    }}])));
    let expected = module(&mut writer, &selected);
    let saved = writer.tree.directory("media-exact-roundtrip");
    let revision = writer.context().project.unwrap().revision.0.value();
    let task = writer.save(revision, &saved);
    writer.finish_one();
    assert!(matches!(writer.task(&task, true).task.state, ComputationStatus::Completed));
    let ui = id("media-roundtrip-ui");
    let mut reader = Harness { host: ProjectHost::new(id("media-roundtrip-reader"), ui.clone()), ui, tree: Sandbox::new() };
    for name in ["assembly-simple", "assembly"] {
        reader.host.register_bundled_module_directory(&source_root.join(name)).unwrap();
    }
    let task = reader.import(&saved);
    reader.finish_one();
    let draft = reader.imported(&task);
    let _: EditCommit = ok(reader.adopt(&task, &draft, false));
    assert_eq!(json!(module(&mut reader, &selected)), json!(expected), "Module semantics survive save/open");
    let result: ModuleTemplateCandidates = ok(reader.call("query_module_library", json!({
        "category_filters":["media"],"industry":null,"product_family":null,"site_envelope":null,
        "source_scope":["project_packages"]
    })));
    let candidate = result.candidates.iter().find(|c| c.module_ref.id == selected.id).unwrap();
    assert!(!candidate.missing.iter().any(|issue| issue.code == "module_library.exact_version_conflict"),
        "An unchanged saved exact media module must agree with the installed library: {:?}", candidate.missing);

    for reference in std::iter::once(selected.clone()).chain(expected.dependencies.iter().cloned()) {
        for asset in module(&mut writer, &reference).assets {
            writer.host.confirm_asset_redistribution(&writer.ui, &asset.id).unwrap();
        }
    }
    let exported = writer.tree.path.join("contribution.sfproj");
    let destination = writer.host.select_file(&writer.ui, FilePurpose::ExportPackage, exported.clone()).unwrap();
    let revision = writer.context().project.unwrap().revision;
    let task: TaskHandle = ok(writer.call("export_package", json!({"revision":revision,"selected_destination_ref":destination,
        "package_scope":{"purpose":"community_module","scenarios":[],"run_ids":[],"artifact_ids":[],
            "module_refs":[selected],"include_redistributable_assets":true}})));
    let _ = completed_package(&mut writer, &task);
    let token = reader.host.select_file(&reader.ui, FilePurpose::ImportPackage, exported).unwrap();
    let task: TaskHandle = ok(reader.call_as(&reader.ui.clone(), false, "import_package", json!({
        "selected_file_ref":token,"import_policy":"open_project","base_revision":null
    })));
    reader.finish_one();
    let draft = reader.imported(&task);
    let _: EditCommit = ok(reader.adopt(&task, &draft, false));
    assert_eq!(json!(module(&mut reader, &selected)), json!(expected));
    let result: ModuleTemplateCandidates = ok(reader.call("query_module_library", json!({
        "category_filters":["media"],"industry":null,"product_family":null,"site_envelope":null,
        "source_scope":["project_packages"]
    })));
    assert_eq!(result.candidates.len(), 2, "Detailed media and its exact simplified dependency remain discoverable");
    for candidate in result.candidates {
        assert!(!candidate.missing.iter().any(|issue| issue.code == "module_library.exact_version_conflict"),
            "An unchanged contribution must not require undeclared library bookkeeping files: {:?}", candidate.missing);
    }

    let dependency = expected.dependencies.first().unwrap();
    let revision = reader.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(reader.apply(revision, json!([{"kind":"adopt_module","data":{
        "module_ref":dependency,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "new_instances":[],"explicit_seed":null
    }}])));
    let snapshot = reader.snapshot();
    assert!(snapshot["project"]["module_refs"].as_array().unwrap().contains(&json!(dependency)));
    for key in ["halls", "devices", "item_types", "resources", "connections"] {
        assert!(snapshot["project"][key].as_array().unwrap().is_empty(), "Adopting media must not create {key}");
    }
    assert!(snapshot["scenarios"].as_object().unwrap().is_empty());

    let changed = reader.tree.directory("changed-declared-source");
    let revision = reader.context().project.unwrap().revision.0.value();
    let task = reader.save(revision, &changed);
    reader.finish_one();
    assert!(matches!(reader.task(&task, true).task.state, ComputationStatus::Completed));
    let source_file = expected.files.iter().find(|file| file.path.as_str().ends_with("/source/parameters.json")).unwrap();
    let path = changed.join(source_file.path.as_str());
    let mut bytes = fs::read(&path).unwrap();
    bytes.push(b'\n');
    fs::write(path, bytes).unwrap();
    let task = reader.import(&changed);
    reader.finish_one();
    let draft = reader.imported(&task);
    let _: EditCommit = ok(reader.adopt(&task, &draft, false));
    let result: ModuleTemplateCandidates = ok(reader.call("query_module_library", json!({
        "category_filters":["media"],"industry":null,"product_family":null,"site_envelope":null,
        "source_scope":["project_packages"]
    })));
    let candidate = result.candidates.iter().find(|c| c.module_ref.id == selected.id).unwrap();
    assert!(candidate.missing.iter().any(|issue| issue.code == "module_library.exact_version_conflict"),
        "Changed declared source bytes must still be reported as an immutable version conflict");
    let before = reader.snapshot();
    let revision = reader.context().project.unwrap().revision.0.value();
    let failure = error(reader.apply(revision, json!([{"kind":"adopt_module","data":{
        "module_ref":selected,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "new_instances":[],"explicit_seed":null
    }}])));
    assert!(matches!(failure.code, ErrorCode::InvalidInput), "{failure:?}");
    assert_eq!(reader.snapshot(), before, "Rejected immutable source changes must leave the Project unchanged");
}

#[test]
#[ignore = "explicit real asset ZIP/process evidence; set SF_P001_ASSET_EVIDENCE to a new directory"]
fn p001_nonempty_cc0_contribution_zip_survives_move_and_new_os() {
    let output = PathBuf::from(std::env::var_os("SF_P001_ASSET_EVIDENCE").unwrap());
    fs::create_dir(&output).unwrap();
    let source_root = root().join("modules/media/equipment-quality-v041/installable");
    let mut h = Harness::new();
    let old_task = h.run("LOCAL");
    h.finish_one();
    let old_run = h.completed_run(&old_task);
    let mut originals = vec![];
    let mut source_files = vec![];
    for name in ["assembly-simple", "assembly"] {
        let directory = source_root.join(name);
        let raw = fs::read(directory.join("module.json")).unwrap();
        let source: ModulePackage = serde_json::from_value(read(&directory.join("module.json"))["data"].clone()).unwrap();
        assert_eq!(source.definition.contribution.license.as_deref(), Some("CC0-1.0"));
        assert!(fs::read_to_string(directory.join("LICENSE.txt")).unwrap().contains("CC0-1.0"));
        source_files.push((directory.join("module.json"), raw));
        for file in &source.files {
            let path = directory.join(file.path.as_str());
            source_files.push((path.clone(), fs::read(path).unwrap()));
        }
        assert_eq!(source.assets.len(), 2);
        assert!(source.rights.iter().all(|r| r.redistribution_confirmed && r.license.as_deref() == Some("CC0-1.0")));
        h.host.register_bundled_module_directory(&directory).unwrap();
        originals.push((directory, source));
    }
    let selected = reference(&originals[1].1);
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([{"kind":"adopt_module","data":{
        "module_ref":selected,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
        "new_instances":[],"explicit_seed":null
    }}])));
    let before = h.snapshot();
    let mut expected = vec![];
    for (directory, source) in &originals {
        let reference = reference(source);
        let actual = module(&mut h, &reference);
        assert_eq!(actual.assets.len(), source.assets.len());
        let mut source_assets = vec![];
        for (asset, original) in actual.assets.iter().zip(&source.assets) {
            assert!(asset.id.as_str().ends_with(original.id.as_str()));
            assert!(asset.path.as_str().ends_with(original.path.as_str()));
            assert_eq!(json!(asset.kind), json!(original.kind));
            let bytes = fs::read(directory.join(original.path.as_str())).unwrap();
            assert!(!bytes.is_empty());
            assert_eq!(asset_bytes(&mut h, &reference, asset), bytes);
            // Existing original CC0 declarations plus this Host's trusted test action.
            // Imported metadata alone is never treated as a live confirmation.
            h.host.confirm_asset_redistribution(&h.ui, &asset.id).unwrap();
            source_assets.push(json!({"id":asset.id,"path":original.path}));
        }
        expected.push(json!({"reference":reference,"package":actual,"original_directory":directory,
            "source_assets":source_assets,"source_files":source.files}));
    }
    let exported = output.join("contribution.sfproj");
    let destination = h.host.select_file(&h.ui, FilePurpose::ExportPackage, exported.clone()).unwrap();
    let revision = h.context().project.unwrap().revision;
    let task: TaskHandle = ok(h.call("export_package", json!({"revision":revision,"selected_destination_ref":destination,
        "package_scope":{"purpose":"community_module","scenarios":[],"run_ids":[],"artifact_ids":[],
            "module_refs":[selected],"include_redistributable_assets":true}})));
    let result = completed_package(&mut h, &task);
    write(&output.join("export-result.json"), &json!({"task_output":result}));
    assert!(!result.missing.iter().any(|i| i.code == "handoff.asset_not_redistributed"),
        "CC0 selected contribution must include its declared editable files: {:?}", result.missing);
    let after = h.snapshot();
    for key in ["project", "scenarios", "issues"] {
        assert_eq!(after[key], before[key], "export does not change live {key}");
    }
    assert_eq!(json!(h.full(&old_run.id)), json!(old_run), "old real Run stays in its original Host");
    let moved = output.join("receiver/contribution.sfproj");
    fs::create_dir(moved.parent().unwrap()).unwrap();
    fs::rename(&exported, &moved).unwrap();
    assert!(!exported.exists());
    write(&output.join("expected.json"), &json!({"parent_pid":std::process::id(),"modules":expected,"moved":moved}));
    let mut child = Command::new(std::env::current_exe().unwrap());
    child.args(["--ignored","--exact","p001_asset_contribution_child","--nocapture","--test-threads=1"])
        .env("SF_P001_ASSET_CHILD", &output);
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        child.creation_flags(0x08000000);
    }
    let child_result = child.output().unwrap();
    fs::write(output.join("child.stdout.txt"), &child_result.stdout).unwrap();
    fs::write(output.join("child.stderr.txt"), &child_result.stderr).unwrap();
    assert!(child_result.status.success(), "{}", String::from_utf8_lossy(&child_result.stderr));
    // Consume the ZIP through the real Host importer in the child. The public
    // archive reader handles its actual resaved directory, not ZIP paths.
    let archive = read_project_archive(&output.join("child-saved")).unwrap();
    assert_eq!(archive.manifest.documents.iter().filter(|d| matches!(d.role, DocumentRole::ModuleDefinition)).count(), 2,
        "selected detailed media and exact simplified dependency only");
    assert!(archive.scenarios.is_empty());
    assert!(archive.project.devices.is_empty() && archive.project.item_types.is_empty() && archive.project.resources.is_empty());
    assert!(!archive.manifest.documents.iter().any(|d| matches!(d.role, DocumentRole::RunResult | DocumentRole::FrozenRunProject | DocumentRole::FrozenRunScenario)));
    assert!(source_files.iter().all(|(path, bytes)| fs::read(path).unwrap() == *bytes));
    write(&output.join("parent-result.json"), &json!({"parent_pid":std::process::id(),"child":read(&output.join("child-result.json")),
        "real_zip_move":true,"old_zip_path_absent":!exported.exists(),"source_files_unchanged":source_files.len(),
        "old_Run_unchanged":true,"exported_documents":archive.manifest.documents.len(),"exported_files":archive.files.len(),
        "ordinary_Native":false,"human_acceptance":false}));
}

#[test]
#[ignore = "child entry only; not an additional independent scenario"]
fn p001_asset_contribution_child() {
    let Some(output) = std::env::var_os("SF_P001_ASSET_CHILD") else { return; };
    let output = PathBuf::from(output);
    let expected = read(&output.join("expected.json"));
    assert_ne!(expected["parent_pid"].as_u64().unwrap(), std::process::id() as u64);
    let ui = id("whole-w5-ui");
    let mut h = Harness { host: ProjectHost::new(id("asset-contribution-new-OS"), ui.clone()), ui, tree: Sandbox::new() };
    let token = h.host.select_file(&h.ui, FilePurpose::ImportPackage, PathBuf::from(expected["moved"].as_str().unwrap())).unwrap();
    let task: TaskHandle = ok(h.call_as(&h.ui.clone(),false,"import_package",json!({"selected_file_ref":token,"import_policy":"open_project","base_revision":null})));
    h.finish_one();
    let imported = h.imported(&task);
    assert!(imported.missing.is_empty(), "{imported:?}");
    let _: EditCommit = ok(h.adopt(&task, &imported, false));
    let mut assets = vec![];
    for row in expected["modules"].as_array().unwrap() {
        let reference: ModuleRef = serde_json::from_value(row["reference"].clone()).unwrap();
        let actual = module(&mut h, &reference);
        assert_eq!(json!(actual), row["package"], "exact module definition, dependencies and rights survive");
        for asset in &actual.assets {
            let source = row["source_assets"].as_array().unwrap().iter().find(|s| s["id"] == json!(asset.id)).unwrap();
            // Independent expected bytes; the fresh Host has no bundled module registration.
            let original = PathBuf::from(row["original_directory"].as_str().unwrap()).join(source["path"].as_str().unwrap());
            let bytes = asset_bytes(&mut h, &reference, asset);
            assert_eq!(bytes, fs::read(original).unwrap());
            assets.push(json!({"id":asset.id,"kind":asset.kind,"bytes":bytes.len(),"exact":true}));
        }
    }
    let snapshot = h.snapshot();
    assert!(snapshot["scenarios"].as_object().unwrap().is_empty());
    let resaved = output.join("child-saved");
    fs::create_dir(&resaved).unwrap();
    let revision = h.context().project.unwrap().revision.0.value();
    let save = h.save(revision, &resaved);
    h.finish_one();
    assert!(matches!(h.task(&save, true).task.state, ComputationStatus::Completed));
    let archive = read_project_archive(&resaved).unwrap();
    assert_eq!(archive.manifest.documents.iter().filter(|d| matches!(d.role, DocumentRole::ModuleDefinition)).count(), 2);
    let mut declared_files_verified = 0;
    let mut license_files_exact = 0;
    for row in expected["modules"].as_array().unwrap() {
        let package: ModulePackage = serde_json::from_value(row["package"].clone()).unwrap();
        let original_files: Vec<FileRef> = serde_json::from_value(row["source_files"].clone()).unwrap();
        assert_eq!(package.files.len(), original_files.len());
        for (file, original) in package.files.iter().zip(&original_files) {
            assert!(file.path.as_str().ends_with(original.path.as_str()));
            let source = PathBuf::from(row["original_directory"].as_str().unwrap()).join(original.path.as_str());
            assert_eq!(archive.files.get(file.path.as_str()).unwrap(), &fs::read(source).unwrap());
            declared_files_verified += 1;
            if original.path.as_str() == "LICENSE.txt" { license_files_exact += 1; }
        }
    }
    assert_eq!(license_files_exact, 2);
    write(&output.join("child-result.json"), &json!({"pid":std::process::id(),"modules":2,"assets":assets,
        "source_library_registered_in_Host":false,"saved_revision":revision,"missing":archive.manifest.missing,
        "scenarios":archive.scenarios.len(),"documents":archive.manifest.documents.len(),"files":archive.files.len(),
        "declared_files_byte_exact":declared_files_verified,"license_files_byte_exact":license_files_exact}));
}
