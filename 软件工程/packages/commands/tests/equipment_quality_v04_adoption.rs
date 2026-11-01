//! Frozen 0.4.0 media reception through the real ProjectHost. No renderer/Run proof.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde::Deserialize;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use support::*;

#[derive(Deserialize)]
struct Registration {
    register_order: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    id: ModuleId,
    version: VersionId,
    directory: String,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn fresh(label: &str) -> Harness {
    let ui = id(&format!("{label}-ui"));
    Harness {
        host: ProjectHost::new(id(label), ui.clone()),
        ui,
        tree: Sandbox::new(),
    }
}
fn request(
    h: &mut Harness,
    project: Option<&ProjectId>,
    scope: ApiScope,
    operation: &str,
    params: Value,
) -> Value {
    let message: ApiRequest = serde_json::from_value(
        json!({"api_version":"0.1","request_id":"equipment-v04-independent",
        "project_id":project,"scope":scope,"operation":operation,"params":params}),
    )
    .unwrap();
    h.host.dispatch(&h.ui, message)
}
fn context(h: &mut Harness) -> ProjectContext {
    ok(request(
        h,
        None,
        ApiScope::Workspace,
        "get_context",
        json!({"selection":[]}),
    ))
}
fn project(h: &mut Harness, identity: &ProjectId) -> ProjectState {
    let object: DomainObject = ok(request(
        h,
        Some(identity),
        ApiScope::Project,
        "get_object",
        json!({"object_id":{"kind":"project","id":identity},"source":{"kind":"draft"},"node_index":null}),
    ));
    let DomainObject::Project(value) = object else {
        panic!("actual Project")
    };
    *value
}
fn apply(h: &mut Harness, identity: &ProjectId, commands: Value) {
    let base = project(h, identity).revision;
    let _: EditCommit = ok(request(
        h,
        Some(identity),
        ApiScope::Project,
        "apply_commands",
        json!({"base_revision":base,"commands":commands}),
    ));
}
fn source(reference: &ModuleRef) -> ObjectSource {
    ObjectSource::ModuleVersion {
        module_ref: reference.clone(),
    }
}
fn module(h: &mut Harness, identity: &ProjectId, reference: &ModuleRef) -> ModulePackage {
    let object: DomainObject = ok(request(
        h,
        Some(identity),
        ApiScope::Project,
        "get_object",
        json!({"object_id":{"kind":"module","id":reference.id},"source":source(reference),"node_index":null}),
    ));
    let DomainObject::Module(value) = object else {
        panic!("actual exact ModulePackage")
    };
    *value
}
fn package(directory: &Path) -> ModulePackage {
    let value: Versioned<ModulePackage> =
        serde_json::from_slice(&fs::read(directory.join("module.json")).unwrap()).unwrap();
    value.data
}
fn reference(package: &ModulePackage) -> ModuleRef {
    ModuleRef {
        id: package.definition.id.clone(),
        version: package.definition.version.clone(),
    }
}
fn adoption(reference: &ModuleRef, target: Option<&str>) -> Value {
    json!({"kind":"adopt_module","data":{"module_ref":reference,
        "targets":target.map(|id| vec![json!({"kind":"device","id":id})]).unwrap_or_default(),
        "bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}})
}
fn entries() -> Vec<Entry> {
    let value: Registration = serde_json::from_slice(
        &fs::read(root().join("modules/media/equipment-quality-v04/registration.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(value.register_order.len(), 28);
    let families: BTreeSet<_> = [
        "assembly",
        "carton",
        "control-box",
        "conveyor",
        "fastener",
        "inspection",
        "packing",
        "pallet",
        "pcb",
        "rework",
        "robot",
        "sealer",
        "sealing-tape",
        "shell",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    let mut actual = BTreeSet::new();
    for i in 0..14 {
        let simple = &value.register_order[i];
        let detailed = &value.register_order[i + 14];
        assert_eq!(String::from(simple.version.clone()), "0.4.0");
        assert_eq!(String::from(detailed.version.clone()), "0.4.0");
        assert_eq!(
            simple.id.as_str(),
            format!("{}.simple", detailed.id.as_str())
        );
        actual.insert(
            detailed
                .id
                .as_str()
                .strip_prefix("p302.line-grade.")
                .unwrap()
                .to_owned(),
        );
    }
    assert_eq!(actual, families);
    value.register_order
}
fn verify_bytes(
    h: &mut Harness,
    identity: &ProjectId,
    directory: &Path,
    input: &ModulePackage,
) -> (ModulePackage, usize) {
    let reference = reference(input);
    let actual = module(h, identity, &reference);
    assert_eq!(actual.definition.id, input.definition.id);
    assert_eq!(actual.definition.version, input.definition.version);
    assert_eq!(
        serde_json::to_value(&actual.dependencies).unwrap(),
        serde_json::to_value(&input.dependencies).unwrap()
    );
    assert_eq!(actual.rights.len(), input.rights.len());

    assert_eq!(
        actual.definition.contribution.license,
        input.definition.contribution.license
    );
    let ModulePayload::Media(presentation) = &actual.definition.payload else {
        panic!("actual Media")
    };
    let ModulePayload::Media(original) = &input.definition.payload else {
        panic!("source Media")
    };
    assert_eq!(presentation.nodes.len(), original.nodes.len());
    assert_eq!(
        serde_json::to_value(&presentation.actions).unwrap(),
        serde_json::to_value(&original.actions).unwrap()
    );
    assert_eq!(actual.assets.len(), 2);
    let mut kinds = BTreeSet::new();
    let revision = project(h, identity).revision;
    let mut total = 0;
    for (asset, original) in actual.assets.iter().zip(&input.assets) {
        // Host namespaces both asset IDs and paths. Follow its returned source;
        // the bytes must still be those of the exact registered input package.
        let prefix = asset
            .id
            .as_str()
            .strip_suffix(original.id.as_str())
            .unwrap();
        assert_eq!(
            asset.path.as_str(),
            format!("{prefix}{}", original.path.as_str())
        );
        let mut expected_right = input
            .rights
            .iter()
            .find(|right| right.asset_ref == original.id)
            .unwrap()
            .clone();
        expected_right.asset_ref = asset.id.clone();
        let actual_right = actual
            .rights
            .iter()
            .find(|right| right.asset_ref == asset.id)
            .unwrap();
        assert_eq!(
            serde_json::to_value(actual_right).unwrap(),
            serde_json::to_value(&expected_right).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&asset.kind).unwrap(),
            serde_json::to_value(&original.kind).unwrap()
        );
        let expected = fs::read(directory.join(original.path.as_str())).unwrap();
        let bytes = h
            .host
            .read_media_asset(
                &h.ui,
                identity,
                &ApiScope::Project,
                &revision,
                &source(&reference),
                asset,
            )
            .unwrap();
        assert!(
            bytes == expected,
            "Host bytes differ: {} {} {}",
            reference.id.as_str(),
            String::from(reference.version.clone()),
            asset.id.as_str()
        );
        match asset.kind {
            AssetKind::Glb => {
                assert_eq!(&bytes[..4], b"glTF");
                kinds.insert("glb");
            }
            AssetKind::Png => {
                assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
                kinds.insert("png");
            }
            _ => panic!("unchanged GLB/PNG package contract"),
        }
        total += bytes.len();
    }
    assert_eq!(kinds, BTreeSet::from(["glb", "png"]));
    (actual, total)
}
fn unchanged_business(project: &ProjectState) {
    assert!(project.scenario_refs.is_empty() && project.artifact_refs.is_empty());
    assert!(project.products.is_empty() && project.routes.is_empty() && project.boms.is_empty());
    assert!(project.resources.is_empty() && project.acknowledgments.is_empty());
    assert!(
        project.item_types.is_empty(),
        "no runtime item or holder simulated for a media-only reception"
    );
}

#[test]
fn all_28_v04_packages_adopt_save_relocate_and_reopen_with_exact_host_bytes_and_old_version() {
    let entries = entries();
    let mut h = fresh("v04-source");
    let _: EditCommit = ok(request(
        &mut h,
        None,
        ApiScope::Workspace,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"Independent 28 media package reception","time_reference":{"label":"media reception only"},"departure":"stay"}}]}),
    ));
    let identity = context(&mut h).project.unwrap().id;
    let empty = serde_json::to_value(project(&mut h, &identity)).unwrap();
    let legacy_simple = root().join("modules/media/p302-line-grade/installable/assembly-simple");
    let legacy_detailed = root().join("modules/media/p302-line-grade/installable/assembly");
    let base_dir = root().join("modules/devices/template-support/devices");
    let base = package(&base_dir);
    h.host.register_bundled_module_directory(&base_dir).unwrap();
    let legacy: Vec<_> = [legacy_simple, legacy_detailed]
        .into_iter()
        .map(|directory| {
            let package = package(&directory);
            assert_eq!(String::from(package.definition.version.clone()), "0.1.0");
            h.host
                .register_bundled_module_directory(&directory)
                .unwrap();
            (directory, package)
        })
        .collect();
    let mut inputs = Vec::new();
    for (index, entry) in entries.iter().enumerate() {
        let directory = root().join(&entry.directory);
        let input = package(&directory);
        assert_eq!(input.definition.id, entry.id);
        assert_eq!(input.definition.version, entry.version);
        if index < 14 {
            assert!(input.dependencies.is_empty());
        } else {
            let expected = ModuleRef {
                id: entries[index - 14].id.clone(),
                version: entries[index - 14].version.clone(),
            };
            assert_eq!(
                serde_json::to_value(&input.dependencies).unwrap(),
                json!([expected])
            );
            let ModulePayload::Media(presentation) = &input.definition.payload else {
                panic!("media source")
            };
            assert_eq!(
                serde_json::to_value(&presentation.lod.simplified).unwrap(),
                json!({"kind":"media_ref","data":expected})
            );
        }
        assert_eq!(
            input.definition.contribution.license.as_deref(),
            Some("CC0-1.0")
        );
        assert!(
            input
                .rights
                .iter()
                .all(|right| right.license.as_deref() == Some("CC0-1.0")
                    && right.redistribution_confirmed)
        );
        h.host
            .register_bundled_module_directory(&directory)
            .unwrap();
        inputs.push((directory, input));
    }
    assert_eq!(
        serde_json::to_value(project(&mut h, &identity)).unwrap(),
        empty,
        "registration must not silently adopt or edit the Project"
    );
    let library: ModuleTemplateCandidates = ok(request(
        &mut h,
        None,
        ApiScope::Workspace,
        "query_module_library",
        json!({"category_filters":["media"],"industry":null,"product_family":null,"site_envelope":null,"source_scope":["bundled"]}),
    ));
    for (_, input) in inputs.iter().chain(&legacy) {
        let reference = reference(input);
        assert!(
            library
                .candidates
                .iter()
                .any(|candidate| candidate.module_ref.id == reference.id
                    && candidate.module_ref.version == reference.version)
        );
    }
    let mut commands = vec![adoption(&reference(&base), None)];
    commands.extend(
        legacy
            .iter()
            .map(|(_, input)| adoption(&reference(input), None)),
    );
    commands.extend(
        inputs
            .iter()
            .map(|(_, input)| adoption(&reference(input), None)),
    );
    apply(&mut h, &identity, json!(commands));
    let fixture = fixture();
    let mut hall = fixture["project"]["halls"][0].clone();
    hall["id"] = json!("v04-hall");
    hall["input_locations"] = json!([]);
    hall["output_locations"] = json!([]);
    let mut layout = vec![put("hall", hall)];
    let equipment = [
        "assembly",
        "conveyor",
        "inspection",
        "packing",
        "rework",
        "robot",
        "sealer",
    ];
    for (i, family) in equipment
        .iter()
        .chain(std::iter::once(&"legacy-assembly"))
        .enumerate()
    {
        let mut device = fixture["project"]["devices"][0].clone();
        device["id"] = json!(format!("visual-{family}"));
        device["title"] = json!(format!("Media reception display {family}"));
        device["hall_id"] = json!("v04-hall");
        device["line_id"] = Value::Null;
        device["module_ref"] = json!(reference(&base));
        device["role_bindings"] = json!({"item_types":{},"resources":{},"targets":{},"ports":{}});
        device["operation_binding"] = Value::Null;
        device["presentation_override"] = Value::Null;
        device["configuration"] = json!({});
        device["pose"]["translation"]["x"] = json!((i % 4) * 3);
        device["pose"]["translation"]["y"] = json!((i / 4) * 3);
        layout.push(put("device", device));
    }
    apply(&mut h, &identity, json!(layout));
    apply(
        &mut h,
        &identity,
        json!([adoption(
            &reference(&legacy[1].1),
            Some("visual-legacy-assembly")
        )]),
    );
    let legacy_before = project(&mut h, &identity)
        .devices
        .into_iter()
        .find(|d| d.id.as_str() == "visual-legacy-assembly")
        .unwrap();
    let bindings: Vec<_> = equipment
        .iter()
        .map(|family| {
            let input = inputs
                .iter()
                .find(|(_, input)| {
                    input.definition.id.as_str() == format!("p302.line-grade.{family}")
                })
                .unwrap();
            adoption(&reference(&input.1), Some(&format!("visual-{family}")))
        })
        .collect();
    apply(&mut h, &identity, json!(bindings));
    let adopted = project(&mut h, &identity);
    unchanged_business(&adopted);
    let expected_refs: BTreeSet<_> = inputs
        .iter()
        .chain(&legacy)
        .map(|(_, input)| {
            (
                input.definition.id.as_str().to_owned(),
                String::from(input.definition.version.clone()),
            )
        })
        .chain(std::iter::once((
            base.definition.id.as_str().to_owned(),
            String::from(base.definition.version.clone()),
        )))
        .collect();
    let actual_refs: BTreeSet<_> = adopted
        .module_refs
        .iter()
        .map(|r| (r.id.as_str().to_owned(), String::from(r.version.clone())))
        .collect();
    assert_eq!(actual_refs, expected_refs);
    let legacy_after = adopted
        .devices
        .iter()
        .find(|d| d.id.as_str() == "visual-legacy-assembly")
        .unwrap();
    assert_eq!(
        serde_json::to_value(legacy_after).unwrap(),
        serde_json::to_value(&legacy_before).unwrap()
    );
    for family in equipment {
        let device = adopted
            .devices
            .iter()
            .find(|d| d.id.as_str() == format!("visual-{family}"))
            .unwrap();
        let locator = &device.presentation_override.as_ref().unwrap().source;
        let MediaLocator::MediaRef(value) = locator else {
            panic!("exact media binding")
        };
        assert_eq!(value.id.as_str(), format!("p302.line-grade.{family}"));
        assert_eq!(String::from(value.version.clone()), "0.4.0");
    }
    let mut actual_packages = BTreeMap::new();
    let mut rows = Vec::new();
    let mut asset_bytes = 0;
    for (directory, input) in inputs.iter().chain(&legacy) {
        let (actual, bytes) = verify_bytes(&mut h, &identity, directory, input);
        asset_bytes += bytes;
        actual_packages.insert(
            (
                input.definition.id.as_str().to_owned(),
                String::from(input.definition.version.clone()),
            ),
            serde_json::to_value(&actual).unwrap(),
        );
        if input.definition.version == entries[0].version {
            rows.push(json!({"id":input.definition.id,"version":input.definition.version,"dependencies":input.dependencies,"asset_count":actual.assets.len(),"host_bytes":bytes}));
        }
    }
    let attempt = root()
        .join(".work/model-v04-host-reception/artifacts")
        .join(format!("attempt-{}", std::process::id()));
    let saved = attempt.join("source/saved-project");
    let moved = attempt.join("receiver/moved-project");
    fs::create_dir_all(&saved).unwrap();
    fs::create_dir_all(&moved).unwrap();
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::SaveProject, saved.clone())
        .unwrap();
    let task: TaskHandle = ok(request(
        &mut h,
        Some(&identity),
        ApiScope::Project,
        "save_project",
        json!({"revision":adopted.revision,"selected_directory_ref":selected}),
    ));
    h.finish_one();
    let state: TaskState = ok(request(
        &mut h,
        Some(&identity),
        ApiScope::Project,
        "get_task",
        json!({"task_id":task.id}),
    ));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    copy(&saved, &moved);
    let mut reopened = fresh("v04-independent-receiver");
    assert!(context(&mut reopened).project.is_none());
    let handle = reopened.import(&moved);
    reopened.finish_one();
    let draft = reopened.imported(&handle);
    let _: EditCommit = ok(reopened.adopt(&handle, &draft, false));
    let new_identity = context(&mut reopened).project.unwrap().id;
    assert_eq!(new_identity, identity);
    let readback = project(&mut reopened, &new_identity);
    assert_eq!(
        serde_json::to_value(&readback).unwrap(),
        serde_json::to_value(&adopted).unwrap()
    );
    unchanged_business(&readback);
    for (directory, input) in inputs.iter().chain(&legacy) {
        let (actual, _) = verify_bytes(&mut reopened, &new_identity, directory, input);
        let key = (
            input.definition.id.as_str().to_owned(),
            String::from(input.definition.version.clone()),
        );
        assert!(
            serde_json::to_value(&actual).unwrap() == actual_packages[&key],
            "exact package metadata changed after move/reopen: {key:?}"
        );
    }
    let input = &inputs[14].1;
    let current = module(&mut reopened, &new_identity, &reference(input));
    let asset = &current.assets[0];
    let revision = readback.revision.clone();
    let actual_source = source(&reference(input));
    let forbidden = reopened
        .host
        .read_media_asset(
            &id("unregistered-session"),
            &new_identity,
            &ApiScope::Project,
            &revision,
            &actual_source,
            asset,
        )
        .unwrap_err();
    assert!(matches!(forbidden.code, ErrorCode::PermissionDenied));
    let workspace = reopened
        .host
        .read_media_asset(
            &reopened.ui,
            &new_identity,
            &ApiScope::Workspace,
            &revision,
            &actual_source,
            asset,
        )
        .unwrap_err();
    assert!(matches!(workspace.code, ErrorCode::InvalidInput));
    let stale = reopened
        .host
        .read_media_asset(
            &reopened.ui,
            &new_identity,
            &ApiScope::Project,
            &support::revision(0),
            &actual_source,
            asset,
        )
        .unwrap_err();
    assert!(matches!(stale.code, ErrorCode::RevisionConflict));
    let mut wrong = asset.clone();
    wrong.path = "not-declared.glb".to_owned().try_into().unwrap();
    let absent = reopened
        .host
        .read_media_asset(
            &reopened.ui,
            &new_identity,
            &ApiScope::Project,
            &revision,
            &actual_source,
            &wrong,
        )
        .unwrap_err();
    assert!(matches!(absent.code, ErrorCode::MissingReference));
    let mut unknown_version = reference(input);
    unknown_version.version = "0.4.1".to_owned().try_into().unwrap();
    let missing_version = reopened
        .host
        .read_media_asset(
            &reopened.ui,
            &new_identity,
            &ApiScope::Project,
            &revision,
            &source(&unknown_version),
            asset,
        )
        .unwrap_err();
    assert!(matches!(missing_version.code, ErrorCode::MissingReference));
    assert_eq!(
        serde_json::to_value(project(&mut reopened, &new_identity)).unwrap(),
        serde_json::to_value(&readback).unwrap()
    );
    let result = json!({"passed":true,"evidence_layer":"real ProjectHost command, file-token save/import/adopt and admitted media byte projection; no Native renderer",
        "registered_order":rows,"v04_packages":28,"families":14,"v04_assets_per_read":56,"legacy_versions_preserved":2,
        "equipment_media_bindings":7,"legacy_device_binding_preserved":true,"save_project":saved,"moved_project":moved,"new_host_no_registration":true,
        "exact_Project_and_packages_after_reopen":true,"all_GLB_PNG_bytes_equal":true,"asset_bytes_per_read_including_legacy":asset_bytes,
        "negative_scope_checks":4,"negative_source_version_checks":1,"source_assets_modified":false,"Native":false,"actual_holder":false,"actual_waiting":false,
        "MQ07_MQ08":false,"whole_task_done":false,"human_acceptance":null});
    fs::write(
        attempt.join("host-result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!(
        "V04_HOST_RESULT {}",
        attempt.join("host-result.json").display()
    );
}

#[test]
fn detailed_v04_requires_its_declared_exact_simplified_dependency() {
    let entries = entries();
    let mut h = fresh("v04-dependency-negative");
    let error = h
        .host
        .register_bundled_module_directory(&root().join(&entries[14].directory))
        .unwrap_err();
    assert!(matches!(error.code, ErrorCode::MissingReference));
    assert_eq!(error.field_errors.len(), 1);
    h.host
        .register_bundled_module_directory(&root().join(&entries[0].directory))
        .unwrap();
    h.host
        .register_bundled_module_directory(&root().join(&entries[14].directory))
        .unwrap();
}
