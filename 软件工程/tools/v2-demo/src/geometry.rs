use super::*;

pub(super) fn run_entries(archive: &ProjectArchive) -> Vec<RunResult> {
    archive.manifest.documents.iter().filter(|e| matches!(e.file_ref.kind, DocumentKind::RunResult)).map(|e| {
        let v: Versioned<RunResult> = serde_json::from_slice(&archive.files[e.file_ref.path.as_str()]).unwrap(); v.data
    }).collect()
}
pub(super) fn retained(before: &ProjectArchive, after: &ProjectArchive) {
    for entry in &before.manifest.documents {
        if matches!(entry.file_ref.kind, DocumentKind::RunResult | DocumentKind::Module | DocumentKind::Asset)
            || entry.file_ref.path.as_str().contains("run-") {
            assert_eq!(before.files[entry.file_ref.path.as_str()], after.files[entry.file_ref.path.as_str()], "old Run/frozen input/module/asset changed: {}", entry.file_ref.path.as_str());
        }
    }
}
pub(super) fn build_geometry(original_path: &Path, staged: &Path, output: &Path, receipt: &Path) {
    assert!(!output.exists(), "never overwrite saved r2 or another delivery");
    let before = read_project_archive(original_path).unwrap(); let mut h = Host::new("geometry-build"); h.open(staged);
    let changes = read(&staged.parent().unwrap().join("geometry-edit.json"));
    let commit = h.call(true, "apply_commands", json!({"base_revision":changes["base_revision"],"commands":changes["commands"]}));
    assert!(commit["new_revision"].as_u64().unwrap() > before.project.revision.0.value());
    let project: ProjectState = typed(h.object("project", PROJECT)); let scenario: ScenarioInput = typed(h.object("scenario", SCENARIO));
    assert!(scenario.revision.0.value() > 2); assert_eq!(project.devices.len(), 2);
    for d in &project.devices { assert_eq!(value(&d.module_ref.version), json!("1.0.6")); assert_eq!(d.item_layout.as_ref().unwrap().slots.len(), 1); }
    for l in &project.locations { assert_eq!(l.item_layout.as_ref().unwrap().slots.len(), 6); }
    let run: RunResult = typed(h.run()); check_business(&run); h.save(output);
    let saved = read_project_archive(output).unwrap(); retained(&before, &saved);
    let old = run_entries(&before); let all = run_entries(&saved); assert_eq!(all.len(), old.len() + 1);
    for prior in &old { assert!(all.iter().any(|r| r.id == prior.id && value(r) == value(prior))); }
    let candidates = bindings(&saved.project);
    write(&output.parent().unwrap().join("candidate-bindings.json"), &value(&candidates));
    let profiles = read(&original_path.parent().unwrap().join("profile-bindings.json"));
    write(&output.parent().unwrap().join("profile-bindings.json"), &profiles);
    write(&receipt.with_file_name("new-prediction.json"), &value(&run));
    write(receipt,&json!({"passed":true,"pid":std::process::id(),"project_revision":saved.project.revision,"scenario_revision":scenario.revision,
        "new_run":run.id,"old_run_ids":old.iter().map(|r|r.id.clone()).collect::<Vec<_>>(),"total_saved_runs":all.len(),"old_run_and_frozen_bytes_unchanged":true,
        "new_modules":changes["new_modules"],"location_slots":6,"device_slots":1,"nominal_ms":[10000,15000],"business_check":"six actual accepted pieces; original fixed-A prediction remains 10/20/30/40/50/60s"}));
}
pub(super) fn verify_geometry(current_path: &Path, original_path: &Path, output: &Path) {
    let archive = read_project_archive(current_path).unwrap(); let original = read_project_archive(original_path).unwrap(); retained(&original, &archive);
    let mut h = Host::new("geometry-reopen"); h.open(current_path);
    let project: ProjectState = typed(h.object("project", PROJECT)); let scenario: ScenarioInput = typed(h.object("scenario", SCENARIO));
    assert_eq!(value(&project), value(&archive.project));
    let all = run_entries(&archive); let prior = run_entries(&original);
    let run = all.iter().find(|r| !prior.iter().any(|old| old.id == r.id)).unwrap().clone();
    for saved in &all { let reply = h.call(true,"get_run",json!({"run_id":saved.id,"query":{"kind":"full_result"}})); assert_eq!(reply["data"],value(saved)); check_business(saved); }
    let frozen_project: Versioned<ProjectState> = serde_json::from_slice(&archive.files[run.input_ref.frozen_project_file.path.as_str()]).unwrap();
    let frozen_scenario: Versioned<ScenarioInput> = serde_json::from_slice(&archive.files[run.input_ref.frozen_scenario_file.path.as_str()]).unwrap();
    assert_eq!(value(&frozen_project.data), value(&project)); assert_eq!(value(&frozen_scenario.data), value(&scenario));
    // Read the new Run's actual frozen module closure via the real Host, not all historical versions.
    let refs = project.module_refs.clone();
    let mut packages = vec![]; let mut asset_reads = vec![];
    for reference in refs {
        let reply = h.call(true,"get_object",json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"run_module_version","data":{"run_id":run.id,"at_ms":0,"module_ref":reference}},"node_index":null}));
        assert_eq!(reply["kind"],"module"); let package:ModulePackage=typed(reply["data"].clone());
        for asset in &package.assets {
            let source=ObjectSource::RunModuleVersion{run_id:run.id.clone(),at_ms:typed(json!(0)),module_ref:reference.clone()};
            let bytes=h.host.read_media_asset(&h.ui,&project.id,&ApiScope::Project,&project.revision,&source,asset).unwrap();
            assert_eq!(bytes,archive.files[asset.path.as_str()]); let path=output.join(asset.path.as_str());fs::create_dir_all(path.parent().unwrap()).unwrap();fs::write(path,bytes).unwrap();
            asset_reads.push(json!({"module_ref":reference,"asset":asset,"actual_host_frozen_read":true}));
        }
        packages.push(package);
    }
    fs::create_dir_all(output).unwrap();
    write(&output.join("host-input.json"),&json!({"project":project,"scenario":scenario,"modules":packages,"run":run,"asset_reads":asset_reads}));
    super::geometry_projection::produce(&project,&scenario,&run,&packages,&value(&asset_reads),output);
    write(&output.join("receipt.json"),&json!({"passed":true,"new_process_pid":std::process::id(),"project_revision":project.revision,"scenario_revision":scenario.revision,
      "new_run":run.id,"all_run_ids":all.iter().map(|r|r.id.clone()).collect::<Vec<_>>(),"old_run_and_frozen_bytes_unchanged":true,"new_frozen_inputs_match_current":true,
      "new_frozen_module_versions":packages.iter().map(|p|json!({"id":p.definition.id,"version":p.definition.version})).collect::<Vec<_>>(),"actual_host_asset_reads":asset_reads}));
}
