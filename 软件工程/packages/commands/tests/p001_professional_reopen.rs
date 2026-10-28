//! Explicit P0-01 new-process consumer of the existing full professional archive.
//! Programmatic Host evidence, not ordinary Native or human acceptance.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{fs, path::Path, process::Command};
use support::*;

fn snapshot(h: &mut Harness, identities: &Value) -> Value {
    let project = h.object("project", "P1");
    let scenarios: Vec<_> = project["scenario_refs"].as_array().unwrap().iter()
        .map(|id| h.object("scenario", id.as_str().unwrap())).collect();
    let runs: Vec<_> = identities["runs"].as_array().unwrap().iter().map(|id| {
        let id: RunId = serde_json::from_value(id.clone()).unwrap();
        serde_json::to_value(h.full(&id)).unwrap()
    }).collect();
    let mut artifacts = vec![];
    let mut manufacturing = vec![];
    let revision = h.context().project.unwrap().revision;
    let project_id: ProjectId = "P1".to_owned().try_into().unwrap();
    for id in identities["artifacts"].as_array().unwrap() {
        let id: ArtifactId = serde_json::from_value(id.clone()).unwrap();
        let triple = h.host.read_stored_packaging_artifact(
            &h.ui, &project_id, &ApiScope::Project, &revision, &id,
        ).unwrap();
        for file in &triple.1.manufacturing_files {
            let bytes = h.host.read_stored_manufacturing_file(
                &h.ui, &project_id, &ApiScope::Project, &revision, &id, &file.file,
            ).unwrap();
            let svg = String::from_utf8(bytes).unwrap();
            for layer in ["cut", "fold", "glue"] {
                assert!(svg.contains(&format!("id=\"{layer}\"")));
            }
            manufacturing.push(json!({"artifact":id,"file":file.file,"svg":svg}));
        }
        artifacts.push(serde_json::to_value(triple).unwrap());
    }
    json!({"project":project,"scenarios":scenarios,"runs":runs,
        "artifacts":artifacts,"manufacturing":manufacturing})
}
fn save(h: &mut Harness, directory: &Path) {
    fs::create_dir(directory).unwrap();
    let revision = h.context().project.unwrap().revision.0.value();
    let task = h.save(revision, directory);
    h.finish_one();
    assert!(matches!(h.task(&task, true).task.state, ComputationStatus::Completed));
}

#[test]
#[ignore = "explicit process/persistence evidence; set SF_P001_PROFESSIONAL_EVIDENCE to a new directory"]
fn p001_professional_saved_archive_survives_move_and_new_os_process() {
    let output = std::path::PathBuf::from(std::env::var_os("SF_P001_PROFESSIONAL_EVIDENCE").unwrap());
    fs::create_dir(&output).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/evaluation/r12-publication/saved-project");
    let original = read_project_archive(&source).unwrap();
    let original_root = fs::read(source.join("project.json")).unwrap();
    let identities = json!({
        "runs":original.manifest.documents.iter()
            .filter(|d| matches!(d.role, DocumentRole::RunResult))
            .map(|d| json!(d.object_ref.id)).collect::<Vec<_>>(),
        "artifacts":original.project.artifact_refs,
    });
    let mut h = Harness::from_seed(Sandbox::new(), &source);
    let before = snapshot(&mut h, &identities);
    assert_eq!(before["project"]["acknowledgments"].as_array().unwrap().len(), 2);
    assert_eq!(before["project"]["bindings"].as_array().unwrap().len(), 2);
    assert_eq!(before["runs"].as_array().unwrap().len(), 2);
    assert_eq!(before["artifacts"].as_array().unwrap().len(), 2);
    assert!(!before["manufacturing"].as_array().unwrap().is_empty());
    write(&output.join("expected.json"), &json!({
        "parent_pid":std::process::id(),"identities":identities,"snapshot":before,
        "origin":"existing actual r12 publication saved-project fixture; no new compute or simulation",
    }));
    let saved = output.join("saved");
    let moved = output.join("moved");
    save(&mut h, &saved);
    assert_eq!(snapshot(&mut h, &identities), before);
    fs::rename(&saved, &moved).unwrap();
    assert!(!saved.exists(), "the newly saved test directory really moved");
    let moved_archive = read_project_archive(&moved).unwrap();
    assert_eq!(json!(moved_archive.project), json!(original.project));
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--ignored", "--exact", "p001_professional_child", "--nocapture", "--test-threads=1"])
        .env("SF_P001_PROFESSIONAL_CHILD", &moved)
        .env("SF_P001_PROFESSIONAL_OUTPUT", &output);
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let result = command.output().unwrap();
    fs::write(output.join("child.stdout.txt"), &result.stdout).unwrap();
    fs::write(output.join("child.stderr.txt"), &result.stderr).unwrap();
    assert!(result.status.success(), "{}", String::from_utf8_lossy(&result.stderr));
    let source_unchanged = fs::read(source.join("project.json")).unwrap() == original_root
        && read_project_archive(&source).unwrap().files == original.files;
    assert!(source_unchanged, "the source fixture must not be rewritten by Save/Open");
    write(&output.join("parent-result.json"), &json!({
        "parent_pid":std::process::id(),"child":read(&output.join("child-result.json")),
        "real_move":true,"saved_path_absent":!saved.exists(),
        "original_fixture_unchanged":source_unchanged,
        "documents":moved_archive.manifest.documents.len(),"files":moved_archive.files.len(),
        "ordinary_Native":false,"human_acceptance":false,
    }));
}

#[test]
#[ignore = "child entry only; not an additional independent coverage case"]
fn p001_professional_child() {
    let Some(source) = std::env::var_os("SF_P001_PROFESSIONAL_CHILD") else { return; };
    let output = std::path::PathBuf::from(std::env::var_os("SF_P001_PROFESSIONAL_OUTPUT").unwrap());
    let want = read(&output.join("expected.json"));
    assert_ne!(want["parent_pid"].as_u64().unwrap(), std::process::id() as u64);
    let mut h = Harness::from_seed(Sandbox::new(), Path::new(&source));
    let actual = snapshot(&mut h, &want["identities"]);
    assert_eq!(actual, want["snapshot"], "all saved inputs/results/purposes/bindings/Runs/SVG bytes survive");
    // Persisted purposes/bindings do not recreate the previous live authority.
    let base = h.context().project.unwrap().revision.0.value();
    let denied = error(h.call("run_scenario", json!({"scenario_id":"LOCAL","input_revision":base,
        "observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert!(matches!(denied.code, ErrorCode::PermissionDenied));
    assert!(h.host.drain_io_work().is_empty());
    let project = &actual["project"];
    let _: EditCommit = ok(h.apply(base, json!([{"kind":"set_project_settings","data":{
        "title":format!("{} · new OS readback", project["title"].as_str().unwrap()),
        "time_reference":project["time_reference"],"generation_context":project["generation_context"],
        "sources":project["sources"]
    }}])));
    let edited = snapshot(&mut h, &want["identities"]);
    assert_eq!(edited["project"]["revision"], base + 1);
    for key in ["acknowledgments", "bindings"] {
        assert_eq!(edited["project"][key], actual["project"][key]);
    }
    for key in ["runs", "artifacts", "manufacturing"] {
        assert_eq!(edited[key], actual[key]);
    }
    save(&mut h, &output.join("child-resaved"));
    let saved = read_project_archive(&output.join("child-resaved")).unwrap();
    assert_eq!(saved.project.revision.0.value(), base + 1);
    write(&output.join("child-result.json"), &json!({
        "pid":std::process::id(),"parent_pid":want["parent_pid"],"readback_equal":true,
        "old_revision":base,"edited_and_saved_revision":base+1,
        "Runs":actual["runs"].as_array().unwrap().len(),
        "artifacts":actual["artifacts"].as_array().unwrap().len(),
        "manufacturing_files":actual["manufacturing"].as_array().unwrap().len(),
        "serialized_authority_denied":denied.code,"old_results_unchanged_after_edit":true,
    }));
}
