//! Frozen real Host/Engine consumers of all nine explicit matrix edits.
//! UI-origin calls below are programmatic test purposes, not human acceptance.
#[path = "fixtures/production/host_support.rs"]
mod support;
#[path = "../../packaging/tests/support/noncu_cases.rs"]
mod cases;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};
use support::*;

fn normalized(job: Value) -> Value {
    serde_json::to_value(serde_json::from_value::<PackageStructureJob>(job).unwrap()).unwrap()
}
fn revision_of(h: &mut Harness) -> u64 { h.context().project.unwrap().revision.0.value() }
fn edit(h: &mut Harness, commands: Value) {
    let base = revision_of(h);
    let _: EditCommit = ok(h.apply(base, commands));
}
fn host(job_id: &str) -> Harness {
    let mut input = fixture();
    let declaration: Value = serde_json::from_str(include_str!("../../packaging/consumer-device.design.json")).unwrap();
    input["definitions"][5]["parameters"] = json!(declaration["data"]["definition"]["parameters"].as_array().unwrap().iter().filter(|p|p["id"]=="packaging_job_id" || p["id"]=="packaging_job_revision").cloned().collect::<Vec<_>>());
    input["project"]["devices"][1]["configuration"] = json!({"packaging_job_id":{"kind":"string","value":job_id},"packaging_job_revision":{"kind":"integer","value":1}});
    Harness::from_value(input)
}
fn completed(h: &mut Harness, task: &TaskHandle, root: &Path, label: &str) -> TaskOutput {
    let state = h.task(task, true);
    write(&root.join(format!("{label}-task.json")), &serde_json::to_value(&state).unwrap());
    assert!(matches!(state.task.state, ComputationStatus::Completed), "{label}: {state:?}");
    state.result.unwrap()
}
fn produce(h: &mut Harness, job: &Value, root: &Path, label: &str) -> PackagingResult {
    edit(h, json!([{"kind":"put_packaging_job","data":job}]));
    let base = revision_of(h);
    let task: TaskHandle = ok(h.call("compute_packaging", json!({"packaging_job":job,"input_revision":base,"computation_budget":job["budget"]})));
    h.finish_one();
    match completed(h, &task, root, label) {
        TaskOutput::Packaging(result) => {
            assert!(matches!(result.outcome, CheckOutcome::Pass), "{label}: {result:?}");
            result
        },
        other => panic!("wrong packaging result {other:?}"),
    }
}
fn bind(h: &mut Harness, result: &PackagingResult, ack_id: &str) -> ConsumerBinding {
    let request = |base| json!({"artifact_id":result.artifact_id,"acceptance_ref":ack_id,"consumer_ref":{"kind":"device","id":"PACK-01"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},"requested_scope":["forming","opening","linear_extraction","net_space","manufacturing"],"base_revision":base});
    let before = h.snapshot();
    let base = revision_of(h);
    error(h.call("adopt_artifact", request(base)));
    assert_eq!(h.snapshot(), before, "calculation cannot authorize its own purpose");
    edit(h, json!([{"kind":"record_artifact_acknowledgment","data":{
        "id":ack_id,"artifact_ref":result.artifact_id,"accepted_subject":result.subject_ref,
        "accepted_scope":["forming","opening","linear_extraction","net_space","manufacturing"],
        "decision_source":{"kind":"human_statement","data":"Programmatic test of existing trusted UI-origin purpose gate; not actual human acceptance or industrial approval"},"conditions":[],"recorded_at_iso8601":null
    }}]));
    let base = revision_of(h);
    let adopted: BindingCommit = ok(h.call("adopt_artifact", request(base)));
    let value = serde_json::to_value(&adopted.binding).unwrap();
    assert_eq!(value["mode"],"calculated_nominal");
    assert_eq!(value["consumer_ref"],json!({"kind":"device","id":"PACK-01"}));
    adopted.binding
}
fn stored(h: &mut Harness, result: &PackagingResult) -> (ArtifactRecord, PackagingResult, PackageStructureJob) {
    let revision = h.context().project.unwrap().revision;
    h.host.read_stored_packaging_artifact(&h.ui, &"P1".to_owned().try_into().unwrap(), &ApiScope::Project, &revision, &result.artifact_id).unwrap()
}
fn svg_bytes(h: &mut Harness, result: &PackagingResult) -> Vec<Vec<u8>> {
    let revision = h.context().project.unwrap().revision;
    result.manufacturing_files.iter().map(|f|h.host.read_stored_manufacturing_file(&h.ui, &"P1".to_owned().try_into().unwrap(), &ApiScope::Project, &revision, &result.artifact_id, &f.file).unwrap()).collect()
}
fn factory(h: &mut Harness, root: &Path, label: &str) -> RunResult {
    let task = h.run("LOCAL");
    h.finish_one();
    completed(h, &task, root, label);
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events.len(),1);
    assert_eq!(run.acceptance_events[0].at_ms.0.value(),75000,"geometry cannot invent a factory duration");
    run
}

fn lifecycle(mechanism: &str, change: &str) {
    let key = format!("{mechanism}-{change}");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/delivery/evidence").join(std::env::var("NONCU_RUN_LABEL").unwrap_or("manual".into())).join("lifecycle").join(&key);
    fs::create_dir_all(&root).unwrap();
    let mut first = cases::seed(mechanism);
    first["id"] = json!(format!("NC-{mechanism}-{change}"));
    first["revision"] = json!(1);
    first["subject_ref"] = json!({"id":"PACKAGE","revision":"r1"});
    first["conditions"]["product_envelope"]["subject"] = first["subject_ref"].clone();
    let first = normalized(first);
    let mut h = host(first["id"].as_str().unwrap());
    let physical = h.object("scenario","LOCAL")["initial_conditions"].clone();
    let initial_project = h.object("project","P1");
    let old = produce(&mut h, &first, &root, "old-calculation");
    let after_calculation = h.object("project","P1");
    assert_eq!(after_calculation["acknowledgments"],initial_project["acknowledgments"]);
    assert_eq!(after_calculation["bindings"],initial_project["bindings"]);
    assert_eq!(h.object("scenario","LOCAL")["initial_conditions"],physical);
    let old_triple = serde_json::to_value(stored(&mut h,&old)).unwrap();
    let old_files = svg_bytes(&mut h,&old);
    let old_binding = bind(&mut h,&old,"ACK-NONCU-OLD");
    let accepted = h.object("project","P1");
    let old_run = factory(&mut h,&root,"old-factory");
    let old_run_json = serde_json::to_value(&old_run).unwrap();

    let mut changed = cases::matrix_job(mechanism,change);
    changed["subject_ref"]["id"] = json!("PACKAGE");
    changed["conditions"]["product_envelope"]["subject"] = changed["subject_ref"].clone();
    let changed = normalized(changed);
    let mut commands = vec![];
    if change=="product_function" {
        let mut product = h.object("product","PACKAGE");
        product["revision"] = json!("r2");
        let mut scenario = h.object("scenario","LOCAL");
        scenario["orders"][0]["product_ref"]["revision"] = json!("r2");
        commands.extend([put("product",product),put("scenario",scenario)]);
    }
    commands.push(json!({"kind":"put_packaging_job","data":changed}));
    commands.push(json!({"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":2},"basis":"user_provided"}}));
    edit(&mut h,json!(commands));
    assert_eq!(h.object("project","P1")["bindings"],accepted["bindings"]);
    let base = revision_of(&mut h);
    let rejected = error(h.call("run_scenario",json!({"scenario_id":"LOCAL","input_revision":base,"observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert!(matches!(rejected.code,ErrorCode::InvalidInput|ErrorCode::MissingReference));
    assert!(h.host.drain_io_work().is_empty());
    write(&root.join("changed-old-binding-rejected.json"),&serde_json::to_value(rejected).unwrap());
    let current = produce(&mut h,&changed,&root,"current-calculation");
    assert_ne!(current.artifact_id,old.artifact_id);
    assert_eq!(h.object("project","P1")["bindings"],accepted["bindings"]);
    assert_eq!(h.object("project","P1")["acknowledgments"],accepted["acknowledgments"]);
    assert_eq!(serde_json::to_value(stored(&mut h,&old)).unwrap(),old_triple);
    assert_eq!(svg_bytes(&mut h,&old),old_files);
    let current_binding = bind(&mut h,&current,"ACK-NONCU-CURRENT");
    assert_ne!(current_binding.id,old_binding.id);
    let current_run = factory(&mut h,&root,"current-factory");
    assert_eq!(serde_json::to_value(h.full(&old_run.id)).unwrap(),old_run_json);
    assert_eq!(h.object("scenario","LOCAL")["initial_conditions"],physical);
    let current_triple = serde_json::to_value(stored(&mut h,&current)).unwrap();
    let current_files = svg_bytes(&mut h,&current);
    assert_eq!(current_files.len(),cases::expected_parts(mechanism,change));
    for (i,bytes) in current_files.iter().enumerate(){fs::write(root.join(format!("current-part-{i}.svg")),bytes).unwrap();}

    let saved = root.join("saved-project");
    let relocated = root.join("relocated-project");
    fs::create_dir(&saved).unwrap();
    fs::create_dir(&relocated).unwrap();
    let base = revision_of(&mut h);
    let save = h.save(base,&saved);
    h.finish_one();
    completed(&mut h,&save,&root,"save");
    copy(&saved,&relocated);
    let mut reopened = Harness::from_seed(Sandbox::new(),&relocated);
    assert_eq!(serde_json::to_value(stored(&mut reopened,&old)).unwrap(),old_triple);
    assert_eq!(serde_json::to_value(stored(&mut reopened,&current)).unwrap(),current_triple);
    assert_eq!(svg_bytes(&mut reopened,&old),old_files);
    assert_eq!(svg_bytes(&mut reopened,&current),current_files);
    assert_eq!(serde_json::to_value(reopened.full(&old_run.id)).unwrap(),old_run_json);
    assert_eq!(serde_json::to_value(reopened.full(&current_run.id)).unwrap(),serde_json::to_value(&current_run).unwrap());
    let base = revision_of(&mut reopened);
    let imported_authority = error(reopened.call("run_scenario",json!({"scenario_id":"LOCAL","input_revision":base,"observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert!(matches!(imported_authority.code,ErrorCode::PermissionDenied),"serialized claims cannot create a live producer/UI authority: {imported_authority:?}");
    assert!(reopened.host.drain_io_work().is_empty());
    write(&root.join("new-Host-serialized-authority-denied.json"),&serde_json::to_value(&imported_authority).unwrap());
    // The original producer Host retains its separately established runtime
    // authority. Reopen the relocated bytes explicitly, then actually rerun.
    let imported = h.import(&relocated);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported,&draft,false));
    let reopened_run = factory(&mut h,&root,"reopened-factory");
    assert_eq!(serde_json::to_value(h.full(&old_run.id)).unwrap(),old_run_json);

    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/architecture-proof/assets");
    reopened.host.set_evaluation_resources(sf_reporting::EvaluationResources{cjk_font:Some(fs::read(assets.join("SourceHanSansCN-Regular.otf")).unwrap()),font_license:Some(fs::read(assets.join("LICENSE.SourceHanSans.txt")).unwrap()),files:BTreeMap::new()});
    let parent = root.join("reports");
    fs::create_dir(&parent).unwrap();
    let selected = reopened.host.select_directory(&reopened.ui,FilePurpose::ExportEvaluation,parent.clone()).unwrap();
    let base = revision_of(&mut reopened);
    let scenario = reopened.object("scenario","LOCAL");
    let report_task: TaskHandle = ok(reopened.call("export_evaluation",json!({"revision":base,"report_mode":"simulation_evaluation","scenario_refs":[{"scenario_id":"LOCAL","revision":scenario["revision"]}],"run_refs":[old_run.id,current_run.id],"metric_scope":{"keys":[],"objects":[],"acceptance":[]},"formats":["pdf","html","csv","json"],"selected_destination_ref":selected})));
    reopened.finish_one();
    let report = match completed(&mut reopened,&report_task,&root,"four-format-report") {TaskOutput::Evaluation(output)=>output,other=>panic!("expected evaluation {other:?}")};
    write(&root.join("actual-task-output.json"),&serde_json::to_value(&report).unwrap());
    assert_eq!(report.context.run_refs.len(),2);
    assert_eq!(report.context.run_evaluations.len(),2);
    let report_file = report.files.iter().find(|f|matches!(f.kind,DocumentKind::Evaluation)).unwrap();
    let report_directory: PathBuf = parent.join(report_file.path.as_str()).parent().unwrap().to_owned();
    assert_eq!(read(&report_directory.join("report.json"))["data"],serde_json::to_value(&report.context).unwrap());
    assert!(fs::read(report_directory.join("report.pdf")).unwrap().starts_with(b"%PDF-"));
    assert!(report_directory.join("index.html").is_file());
    assert_eq!(fs::read_dir(report_directory.join("csv")).unwrap().filter(|f|f.as_ref().unwrap().path().extension().is_some_and(|x|x=="csv")).count(),15);
    for f in &report.files{assert!(parent.join(f.path.as_str()).is_file(),"missing declared export {}",f.path.as_str());}
    write(&root.join("lifecycle.json"),&json!({"mechanism":mechanism,"change":change,"old_job":first,"new_job":changed,"old":old_triple,"current":current_triple,"old_binding":old_binding,"new_binding":current_binding,"old_run":old_run,"new_run":current_run,"reopened_run":reopened_run,"reopened_run_origin":"original producer Host after explicit relocated import","new_Host_rejects_serialized_execution_authority":true,"physical_initial_conditions_unchanged":true,"old_job_result_svg_binding_ack_and_Run_preserved":true,"reopened_in_new_Host":true,"report_directory":report_directory,"human_acceptance":false,"rights_confirmation_performed":false}));
    eprintln!("NONCU_HOST {key} complete: old/new actual result, separate purpose/binding, old Run, new Run, new Host and four formats");
}

macro_rules! row {($name:ident,$mechanism:literal,$change:literal)=>{#[test]fn $name(){lifecycle($mechanism,$change);}};}
row!(container_product,"container","product_function");
row!(container_material,"container","material_approved");
row!(container_repeat,"container","part_relations_or_repeat_order");
row!(outer_liner_product,"outer_liner","product_function");
row!(outer_liner_material,"outer_liner","material_approved");
row!(outer_liner_repeat,"outer_liner","part_relations_or_repeat_order");
row!(cover_product,"cover","product_function");
row!(cover_material,"cover","material_approved");
row!(cover_repeat,"cover","part_relations_or_repeat_order");
row!(sleeve_product,"sleeve","product_function");
row!(sleeve_material,"sleeve","material_approved");
row!(sleeve_repeat,"sleeve","part_relations_or_repeat_order");
