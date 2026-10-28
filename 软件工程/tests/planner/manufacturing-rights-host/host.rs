//! Private stdin transport to the exact accepted real Host; no alternative business state.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
};
use support::*;

fn seed_input() -> (Value, Value) {
    let mut input = fixture();
    let declaration: Value = serde_json::from_str(include_str!(
        "../../../packages/packaging/consumer-device.design.json"
    ))
    .unwrap();
    input["definitions"][5]["parameters"] = json!(
        declaration["data"]["definition"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["id"] == "packaging_job_id" || p["id"] == "packaging_job_revision")
            .cloned()
            .collect::<Vec<_>>()
    );
    input["project"]["devices"][1]["configuration"] = json!({"packaging_job_id":{"kind":"string","value":"MR-UI-JOB"},"packaging_job_revision":{"kind":"integer","value":1}});
    let cases: Value = serde_json::from_str(include_str!(
        "../../../packages/commands/tests/fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == "COVERAGE-restricted_folded_container")
        .unwrap();
    assert_eq!(case["expected"], "observed"); // Preserve the source label; UI Pass is the accepted manufacturing-consumer expectation.
    let mut job = case["job"]["data"].clone();
    job["id"] = json!("MR-UI-JOB");
    job["revision"] = json!(1);
    job["subject_ref"] = json!({"id":"PACKAGE","revision":"r1"});
    job["conditions"]["product_envelope"]["subject"] = job["subject_ref"].clone();
    (input, job)
}
fn fail(code: ErrorCode, message: String) -> ApiError {
    ApiError {
        code,
        message,
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}
fn result<T: serde::Serialize>(value: Result<T, ApiError>) -> Value {
    match value {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error}),
    }
}
fn finish_work(h: &mut Harness) {
    for work in h.host.drain_io_work() {
        h.host.finish_io(work.run());
    }
}
fn state(h: &mut Harness) -> Value {
    let context = h.context();
    let p = h.object("project", "P1");
    let modules: Vec<Value> = p["module_refs"].as_array().unwrap().iter().map(|m| {
        let value: DomainObject = ok(h.call("get_object", json!({"object_id":{"kind":"module","id":m["id"]},"source":{"kind":"module_version","data":{"module_ref":m}},"node_index":null})));
        serde_json::to_value(value).unwrap()["data"].clone()
    }).collect();
    json!({"project":p,"dirty":context.project.unwrap().dirty,"modules":modules})
}
fn verify(h: &mut Harness, out: &Path, path: &Path, artifact_id: &ArtifactId) -> Value {
    let before = state(h);
    let scope = ApiScope::Project;
    let p = h.context().project.unwrap();
    let original = h
        .host
        .read_stored_packaging_artifact(&h.ui, &p.id, &scope, &p.revision, artifact_id)
        .unwrap();
    let source = out.join("source-after-export");
    fs::create_dir_all(&source).unwrap();
    let saved = h.save(p.revision.0.value(), &source);
    finish_work(h);
    let saved_state = h.task(&saved, true);
    assert!(
        matches!(saved_state.task.state, ComputationStatus::Completed),
        "actual Save terminal: {}",
        serde_json::to_string(&saved_state).unwrap()
    );
    let source_manifest = read(&source.join("project.json"));
    let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let manifest: Value = serde_json::from_reader(zip.by_name("project.json").unwrap()).unwrap();
    let mut checks = vec![];
    for manufacturing in &original.1.manufacturing_files {
        let file = &manufacturing.file;
        let mut bytes = vec![];
        zip.by_name(file.path.as_str())
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, fs::read(source.join(file.path.as_str())).unwrap());
        let svg = std::str::from_utf8(&bytes).unwrap();
        for layer in ["cut", "fold", "glue"] {
            assert!(svg.contains(&format!("id=\"{layer}\"")));
        }
        assert!(svg.contains("mm\""));
        assert!(svg.contains(&format!(
            "data-part-ref=\"{}\"",
            manufacturing.part_id.as_str()
        )));
        assert!(svg.contains(&format!(
            "data-job-ref=\"{}\"",
            original.1.job_ref.id.as_str()
        )));
        assert!(svg.contains(&format!(
            "data-job-revision=\"{}\"",
            original.1.job_ref.revision.0.value()
        )));
        let e = source_manifest["data"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["file_ref"]["path"] == file.path.as_str())
            .unwrap();
        assert_eq!(e["role"], "supporting_file");
        assert!(
            !source_manifest["data"]["assets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["asset_ref"] == e["object_ref"]["id"])
        );
        checks.push(json!({"file":file,"actual_bytes":bytes.len(),"equal_to_ordinary_Save":true,"layers":["cut","fold","glue"],"unit":"millimeter"}));
    }
    let moved = out.join("moved");
    fs::create_dir_all(&moved).unwrap();
    let moved_zip = moved.join("manufacturing.sfproj");
    fs::copy(path, &moved_zip).unwrap();
    let tree = Sandbox::new();
    let ui = id("ui-host-fresh");
    let mut fresh = Harness {
        host: ProjectHost::new(id("ui-host-fresh-instance"), ui.clone()),
        ui,
        tree,
    };
    let selected = fresh
        .host
        .select_file(&fresh.ui, FilePurpose::ImportPackage, moved_zip.clone())
        .unwrap();
    let imported: TaskHandle = ok(fresh.call_as(
        &fresh.ui.clone(),
        false,
        "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
    ));
    finish_work(&mut fresh);
    let draft = fresh.imported(&imported);
    let _: EditCommit = ok(fresh.adopt(&imported, &draft, true));
    let p = fresh.context().project.unwrap();
    let reopened = fresh
        .host
        .read_stored_packaging_artifact(&fresh.ui, &p.id, &scope, &p.revision, artifact_id)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&reopened).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    let repeat_path = out.join("fresh-without-current-confirmation.sfproj");
    let token = fresh
        .host
        .select_file(&fresh.ui, FilePurpose::ExportPackage, repeat_path.clone())
        .unwrap();
    let task:TaskHandle=ok(fresh.call("export_package",json!({"revision":p.revision,"selected_destination_ref":token,"package_scope":{"purpose":"private_project","scenarios":[],"run_ids":[],"artifact_ids":[artifact_id],"module_refs":[],"include_redistributable_assets":true}})));
    finish_work(&mut fresh);
    assert!(matches!(
        fresh.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let mut repeated = zip::ZipArchive::new(fs::File::open(repeat_path).unwrap()).unwrap();
    for f in &original.1.manufacturing_files {
        assert!(repeated.by_name(f.file.path.as_str()).is_err());
    }
    json!({"original_zip":path,"ordinary_Save_directory":source,"moved_zip":moved_zip,"before_explicit_Save":before,"zip_manifest":manifest,"ordinary_Save_manifest":source_manifest,"files":checks,"fresh_reopen_exact_artifact_result_input":true,"imported_declarations_do_not_restore_current_confirmation":true,"artifact":original.0,"result":original.1,"input":original.2})
}
fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap());
    fs::create_dir_all(&out).unwrap();
    let (input, job) = seed_input();
    let mut h = Harness::from_value(input);
    let p = h.context().project.unwrap();
    let _: EditCommit = ok(h.apply(
        p.revision.0.value(),
        json!([{"kind":"put_packaging_job","data":job}]),
    ));
    println!("{}", json!({"ready":true,"job":job,"state":state(&mut h)}));
    io::stdout().flush().unwrap();
    let mut files = vec![];
    let mut hold_worker = false;
    for line in io::stdin().lock().lines() {
        let q: Value = serde_json::from_str(&line.unwrap()).unwrap();
        let answer = match q["kind"].as_str().unwrap() {
            "invoke" => {
                let args = &q["args"];
                match q["command"].as_str().unwrap() {
                    "project_api" => {
                        let request: ApiRequest =
                            serde_json::from_value(args["request"].clone()).unwrap();
                        let is_task = args["request"]["operation"] == "get_task";
                        let reply = h.host.dispatch(&h.ui, request);
                        if is_task && !hold_worker {
                            finish_work(&mut h);
                        }
                        result(Ok(reply))
                    }
                    "project_select_file" => {
                        let purpose: FilePurpose =
                            serde_json::from_value(args["purpose"].clone()).unwrap();
                        let path = out.join(format!("selected-{:03}.sfproj", files.len() + 1));
                        files.push(path.clone());
                        result(h.host.select_file(&h.ui, purpose, path))
                    }
                    "project_export_manufacturing_package" => {
                        let r = &args["request"];
                        result(h.host.export_manufacturing_package(
                            &h.ui,
                            &serde_json::from_value(r["project_id"].clone()).unwrap(),
                            &serde_json::from_value(r["scope"].clone()).unwrap(),
                            &serde_json::from_value(r["scope_revision"].clone()).unwrap(),
                            serde_json::from_value(r["params"].clone()).unwrap(),
                            serde_json::from_value(r["declarations"].clone()).unwrap(),
                            r["confirm_redistribution"].as_bool().unwrap(),
                        ))
                    }
                    "project_list_stored_packaging" => {
                        let r = &args["request"];
                        result(h.host.list_stored_packaging_artifacts(
                            &h.ui,
                            &serde_json::from_value(r["project_id"].clone()).unwrap(),
                            &serde_json::from_value(r["scope"].clone()).unwrap(),
                            &serde_json::from_value(r["scope_revision"].clone()).unwrap(),
                        ))
                    }
                    "project_read_stored_packaging" => {
                        let r = &args["request"];
                        let value = h.host.read_stored_packaging_artifact(
                            &h.ui,
                            &serde_json::from_value(r["project_id"].clone()).unwrap(),
                            &serde_json::from_value(r["scope"].clone()).unwrap(),
                            &serde_json::from_value(r["scope_revision"].clone()).unwrap(),
                            &serde_json::from_value(r["artifact_id"].clone()).unwrap(),
                        );
                        result(value.map(|(artifact,result,input)|json!({"artifact":artifact,"result":result,"input":input})))
                    }
                    name => result::<Value>(Err(fail(
                        ErrorCode::UnsupportedCapability,
                        format!("No test transport for {name}"),
                    ))),
                }
            }
            "state" => json!({"ok":true,"value":state(&mut h)}),
            "worker" => {
                hold_worker = q["hold"].as_bool().unwrap();
                if !hold_worker {
                    finish_work(&mut h);
                }
                json!({"ok":true,"value":null})
            }
            "remote_edit" => {
                let p = h.object("project", "P1");
                let _:EditCommit=ok(h.apply(p["revision"].as_u64().unwrap(),json!([{"kind":"set_project_settings","data":{"title":"actual concurrent edit","time_reference":p["time_reference"],"generation_context":p["generation_context"],"sources":p["sources"]}}])));
                json!({"ok":true,"value":state(&mut h)})
            }
            "create_draft" => {
                let mut d: Value = serde_json::from_str(include_str!(
                    "../../../modules/devices/template-support/professional/module.json"
                ))
                .unwrap();
                d["data"]["definition"]["id"] = json!("manufacturing.ui.actual-draft");
                let p = h.context().project.unwrap();
                let c: EditCommit = ok(h.apply(
                    p.revision.0.value(),
                    json!([{"kind":"create_definition_draft","data":d["data"]["definition"]}]),
                ));
                let EditScope::DefinitionDraft(s) = c.scope else {
                    panic!("actual draft")
                };
                json!({"ok":true,"value":{"kind":"definition_draft","data":s.id}})
            }
            "verify" => {
                let artifact = serde_json::from_value(q["artifact_id"].clone()).unwrap();
                let path = files[q["file_index"].as_u64().unwrap() as usize].clone();
                json!({"ok":true,"value":verify(&mut h,&out,&path,&artifact)})
            }
            "files" => {
                json!({"ok":true,"value":files.iter().map(|p|json!({"path":p,"exists":p.exists()})).collect::<Vec<_>>()})
            }
            "zip_summary" => {
                let path = &files[q["file_index"].as_u64().unwrap() as usize];
                let mut archive = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
                let manifest: Value =
                    serde_json::from_reader(archive.by_name("project.json").unwrap()).unwrap();
                let names = (0..archive.len())
                    .map(|n| archive.by_index(n).unwrap().name().unwrap().into_owned())
                    .collect::<Vec<_>>();
                json!({"ok":true,"value":{"manifest":manifest,"names":names}})
            }
            other => panic!("Unknown private test command {other}"),
        };
        println!("{answer}");
        io::stdout().flush().unwrap();
    }
}
