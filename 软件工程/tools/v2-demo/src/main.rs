//! Repeatable real Host archive/run producer. No Native, desktop, cloud or ports.
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::cloud_execution as wire;
use sf_domain::*;
use sf_simulation::{Engine, EngineInput};
use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

const PROJECT: &str = "Task8-two-device-six-units";
const SCENARIO: &str = "Task8-six-units";
mod geometry;
mod geometry_projection;
mod multistage;
mod full_factory;
static REQUEST: AtomicU64 = AtomicU64::new(1);
fn typed<T: DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}
fn value<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).unwrap()
}
fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn read(p: &Path) -> Value {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: &Path, v: &Value) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, serde_json::to_vec_pretty(v).unwrap()).unwrap();
}
fn data(reply: Value) -> Value {
    let response: Response<Value> = typed(reply.clone());
    match response.outcome {
        ApiOutcome::Ok(v) => v,
        other => panic!("Host call failed: {other:?}\n{reply:#}"),
    }
}
struct Host {
    host: ProjectHost,
    ui: ObjectId,
}
impl Host {
    fn new(label: &str) -> Self {
        let ui = id(&format!("task8-{label}-ui-{}", std::process::id()));
        Self {
            host: ProjectHost::new(
                id(&format!("task8-{label}-host-{}", std::process::id())),
                ui.clone(),
            ),
            ui,
        }
    }
    fn call(&mut self, project: bool, operation: &str, params: Value) -> Value {
        let request = typed(
            json!({"api_version":"0.1","request_id":format!("task8-demo-{}",REQUEST.fetch_add(1,Ordering::Relaxed)),"project_id":if project{json!(PROJECT)}else{Value::Null},"scope":{"kind":if project{"project"}else{"workspace"}},"operation":operation,"params":params}),
        );
        data(self.host.dispatch(&self.ui, request))
    }
    fn finish(&mut self) {
        let mut work = self.host.drain_io_work();
        assert_eq!(work.len(), 1);
        let completion = work.pop().unwrap().run();
        self.host.finish_io(completion);
    }
    fn task(&mut self, project: bool, handle: &Value) -> Value {
        self.call(project, "get_task", json!({"task_id":handle["id"]}))
    }
    fn open(&mut self, path: &Path) {
        let selected = self
            .host
            .select_directory(&self.ui, FilePurpose::ImportPackage, path.to_owned())
            .unwrap();
        let handle=self.call(false,"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}));
        self.finish();
        let state = self.task(false, &handle);
        assert_eq!(state["task"]["state"], "completed", "{state:#}");
        let draft = &state["result"]["data"];
        let commit=self.call(false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":handle["id"],"choices":{"chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}));
        assert_eq!(commit["project_id"], PROJECT);
    }
    fn object(&mut self, kind: &str, object_id: &str) -> Value {
        self.call(true,"get_object",json!({"object_id":{"kind":kind,"id":object_id},"source":{"kind":"draft"},"node_index":null}))["data"].clone()
    }
    fn context(&mut self) -> Value {
        self.call(true, "get_context", json!({"selection":[]}))
    }
    fn run(&mut self) -> Value {
        let context = self.context();
        let scenario = self.object("scenario", SCENARIO);
        let handle=self.call(true,"run_scenario",json!({"scenario_id":SCENARIO,"input_revision":context["project"]["revision"],"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}));
        self.finish();
        let state = self.task(true, &handle);
        assert_eq!(state["task"]["state"], "completed", "{state:#}");
        assert_eq!(state["result"]["kind"], "run");
        let run_id = state["result"]["data"]["run_id"].clone();
        let run = self.call(
            true,
            "get_run",
            json!({"run_id":run_id,"query":{"kind":"full_result"}}),
        );
        assert_eq!(run["kind"], "full");
        run["data"].clone()
    }
    fn save(&mut self, path: &Path) {
        fs::create_dir(path).unwrap();
        let selection = self
            .host
            .select_directory(&self.ui, FilePurpose::SaveProject, path.to_owned())
            .unwrap();
        let context = self.context();
        let handle = self.call(
            true,
            "save_project",
            json!({"revision":context["project"]["revision"],"selected_directory_ref":selection}),
        );
        self.finish();
        let state = self.task(true, &handle);
        assert_eq!(state["task"]["state"], "completed", "{state:#}");
        assert_eq!(state["result"]["kind"], "save");
    }
}
fn bindings(project: &ProjectState) -> Vec<wire::CandidateBinding> {
    project.devices.iter().map(|device|{let ms=value(&device.configuration["processing_sim_seconds"])["value"].as_u64().unwrap();typed(json!({"binding_id":format!("binding-{}",device.id.as_str()),"device_id":device.id,"device_module_ref":device.module_ref,"operation_ref":device.module_ref,"execution_capacity_ref":{"kind":"resource","id":format!("{}-CAP",device.id.as_str())},"input_ports":[{"role_id":"workpiece","port_id":"input","location_id":"L1-SOURCE","item_type_id":"benchmark-item-v1","unit_id":"piece"}],"output_ports":[{"role_id":"workpiece","port_id":"output","location_id":"L1-SINK","item_type_id":"benchmark-item-v1","unit_id":"piece"}],"resources":[],"phase_id":"cycle","nominal_work_ms":ms}))}).collect()
}
fn modules(archive: &ProjectArchive) -> Vec<ModulePackage> {
    archive
        .manifest
        .documents
        .iter()
        .filter(|e| matches!(e.file_ref.kind, DocumentKind::Module))
        .map(|e| {
            let v: Versioned<ModulePackage> =
                serde_json::from_slice(&archive.files[e.file_ref.path.as_str()]).unwrap();
            v.data
        })
        .collect()
}
fn check_business(run: &RunResult) {
    assert!(matches!(
        run.computation_status,
        ComputationStatus::Completed
    ));
    assert!(run.whole_window_computed);
    assert_eq!(run.jobs.len(), 6);
    assert_eq!(run.acceptance_events.len(), 6);
    assert_eq!(run.final_items.len(), 6);
    for job in &run.jobs {
        assert_eq!(value(&job.execution_ref)["data"]["device_id"], "L1-D01");
        assert_eq!(
            job.finished_at_ms.as_ref().unwrap().0.value()
                - job.started_at_ms.as_ref().unwrap().0.value(),
            10000
        );
    }
    assert_eq!(run.phase_work_intervals.len(), 6);
    for interval in &run.phase_work_intervals {
        assert_eq!(
            interval.interval.end_ms.0.value() - interval.interval.start_ms.0.value(),
            10000
        );
        assert_eq!(
            interval.effective_progress_end_ms.0.value()
                - interval.effective_progress_start_ms.0.value(),
            10000
        );
    }
    let times: Vec<_> = run
        .acceptance_events
        .iter()
        .map(|e| e.at_ms.0.value())
        .collect();
    assert_eq!(times, vec![10000, 20000, 30000, 40000, 50000, 60000]);
}
fn build(source: &Path, output: &Path, receipt: &Path) {
    assert!(
        !output.exists(),
        "new output directory required; never overwrite an existing sample"
    );
    for name in ["candidate-bindings.json", "profile-bindings.json"] {
        assert!(
            !output.parent().unwrap().join(name).exists(),
            "fresh bundle parent required; existing mappings are preserved"
        );
    }
    fs::create_dir_all(output.parent().unwrap()).unwrap();
    let seed = read_project_archive(source).unwrap();
    let mut h = Host::new("build");
    h.open(source);
    let context = h.context();
    h.call(true,"apply_commands",json!({"base_revision":context["project"]["revision"],"commands":[{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"L1-D01"},"parameter_id":"processing_sim_seconds","value":{"kind":"integer","value":10000},"basis":"user_provided"}},{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"L1-D02"},"parameter_id":"processing_sim_seconds","value":{"kind":"integer","value":15000},"basis":"user_provided"}}]}));
    let run: RunResult = typed(h.run());
    write(
        &receipt.with_file_name("actual-prediction.json"),
        &value(&run),
    );
    check_business(&run);
    h.save(output);
    let saved = read_project_archive(output).unwrap();
    assert_eq!(saved.project.devices.len(), 2);
    assert_eq!(saved.project.parameter_sources.len(), 2);
    for e in seed
        .manifest
        .documents
        .iter()
        .filter(|e| matches!(e.file_ref.kind, DocumentKind::Module | DocumentKind::Asset))
    {
        assert_eq!(
            seed.files[e.file_ref.path.as_str()],
            saved.files[e.file_ref.path.as_str()],
            "original module/asset bytes changed"
        );
    }
    let candidates = bindings(&saved.project);
    write(
        &output.parent().unwrap().join("candidate-bindings.json"),
        &value(&candidates),
    );
    let profiles = json!({"source_mode":"software_in_loop","local_project_id":saved.project.id,"scenario_id":SCENARIO,"target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},"execution_profiles":[{"profile_id":"processor-a","device_id":"L1-D01","binding_id":"binding-L1-D01","processing_time_ms":10000},{"profile_id":"processor-b","device_id":"L1-D02","binding_id":"binding-L1-D02","processing_time_ms":15000}],"note":"Non-secret mapping only. Real H01 creates session/epoch; Native supplies credentials privately."});
    write(
        &output.parent().unwrap().join("profile-bindings.json"),
        &profiles,
    );
    write(
        receipt,
        &json!({"passed":true,"builder_pid":std::process::id(),"project_id":saved.project.id,"project_revision":saved.project.revision,"scenario_id":SCENARIO,"prediction_run_id":run.id,"prediction_input_ref":run.input_ref,"computation_status":run.computation_status,"whole_window_computed":run.whole_window_computed,"goal_met":run.goal_met,"accepted_quantity":6,"acceptance_times_ms":run.acceptance_events.iter().map(|e|e.at_ms.0.value()).collect::<Vec<_>>(),"prediction_actual_device":"L1-D01","original_module_and_asset_bytes_unchanged":true,"module_refs":saved.project.module_refs,"parameter_sources":saved.project.parameter_sources,"manifest_documents":saved.manifest.documents.len(),"candidate_bindings":candidates}),
    );
    println!("Real Host build/save passed; run_id={}", run.id.as_str());
}
fn verify(archive_path: &Path, output: &Path) {
    let archive = read_project_archive(archive_path).unwrap();
    let mut h = Host::new("reopen");
    h.open(archive_path);
    let project: ProjectState = typed(h.object("project", PROJECT));
    let scenario: ScenarioInput = typed(h.object("scenario", SCENARIO));
    assert_eq!(value(&project), value(&archive.project));
    assert_eq!(project.devices.len(), 2);
    assert_eq!(scenario.initial_conditions.items.len(), 6);
    assert_eq!(
        scenario.production_tasks[0]
            .planned_quantity
            .value
            .coefficient
            .value(),
        6
    );
    let packages = modules(&archive);
    let run_entry = archive
        .manifest
        .documents
        .iter()
        .find(|e| matches!(e.file_ref.kind, DocumentKind::RunResult))
        .unwrap();
    let stored: Versioned<RunResult> =
        serde_json::from_slice(&archive.files[run_entry.file_ref.path.as_str()]).unwrap();
    let run_id = stored.data.id.clone();
    let public_run = h.call(
        true,
        "get_run",
        json!({"run_id":run_id,"query":{"kind":"full_result"}}),
    );
    let run: RunResult = typed(public_run["data"].clone());
    assert_eq!(value(&run), value(&stored.data));
    check_business(&run);
    let frozen_project: Versioned<ProjectState> =
        serde_json::from_slice(&archive.files[run.input_ref.frozen_project_file.path.as_str()])
            .unwrap();
    let frozen_scenario: Versioned<ScenarioInput> =
        serde_json::from_slice(&archive.files[run.input_ref.frozen_scenario_file.path.as_str()])
            .unwrap();
    assert_eq!(value(&frozen_project.data), value(&project));
    assert_eq!(value(&frozen_scenario.data), value(&scenario));
    let candidates = bindings(&project);
    let identity: wire::ExecutionIdentity = typed(
        json!({"session_id":format!("private-verify-session-{}",std::process::id()),"execution_epoch_id":format!("private-verify-epoch-{}",std::process::id()),"project_id":project.id,"host_context_session_id":format!("private-verify-host-{}",std::process::id())}),
    );
    let policy: wire::ExecutionPolicy = typed(
        json!({"clock_mode":"host_monotonic","report_cutoff_ms":40000,"stop_new_dispatch_on_cutoff":true,"finish_policy":"drain_admitted_then_finish","target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},"source_prediction_window":{"start_ms":0,"end_ms":scenario.window.end_ms},"observation_max_age_ms":1000}),
    );
    let input = EngineInput {
        run_id: run.id.clone(),
        input_ref: run.input_ref.clone(),
        project: project.clone(),
        scope: scenario.scope.clone(),
        scenario: scenario.clone(),
        definitions: packages.iter().map(|m| m.definition.clone()).collect(),
    };
    let mut wrong = candidates.clone();
    wrong[1].nominal_work_ms = 10000u64.try_into().unwrap();
    let rejected =
        match Engine::new_external(input.clone(), identity.clone(), policy.clone(), wrong) {
            Err(error) => error.code,
            Ok(_) => panic!("wrong B nominal duration was admitted"),
        };
    assert_eq!(rejected, "external.binding");
    let engine = Engine::new_external(input, identity, policy, candidates.clone()).unwrap();
    let prepared = engine.external_observation().unwrap();
    assert_eq!(prepared.queued_logical_jobs.len(), 6);
    assert_eq!(prepared.items.len(), 6);
    assert!(
        prepared.jobs.is_empty()
            && prepared.events.is_empty()
            && prepared.resource_claims.is_empty()
    );
    assert!(engine.snapshot().acceptance_events.is_empty());
    fs::create_dir_all(output).unwrap();
    let mut assets = vec![];
    for package in &packages {
        for asset in &package.assets {
            let bytes = h
                .host
                .read_media_asset(
                    &h.ui,
                    &project.id,
                    &ApiScope::Project,
                    &project.revision,
                    &ObjectSource::ModuleVersion {
                        module_ref: ModuleRef {
                            id: package.definition.id.clone(),
                            version: package.definition.version.clone(),
                        },
                    },
                    asset,
                )
                .unwrap();
            assert_eq!(bytes, archive.files[asset.path.as_str()]);
            let path = output.join(asset.path.as_str());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, bytes).unwrap();
            assets.push(json!({"asset":asset,"module_ref":{"id":package.definition.id,"version":package.definition.version},"read_by_actual_host":true}));
        }
    }
    write(
        &output.join("host-input.json"),
        &json!({"project":project,"scenario":scenario,"modules":packages,"run":run,"asset_reads":assets}),
    );
    write(
        &output.join("receipt.json"),
        &json!({"passed":true,"reopen_pid":std::process::id(),"project_id":project.id,"scenario_id":scenario.id,"run_id":run.id,"run_frozen_inputs_exactly_restored":true,"prepared":{"logical_work":6,"inputs":6,"jobs":0,"events":0,"accepted":0,"resource_claims":0},"wrong_b_nominal_rejected":true,"wrong_b_nominal_issue":rejected,"nominal_work_ms":candidates.iter().map(|b|value(&b.nominal_work_ms)).collect::<Vec<_>>(),"asset_reads":assets,"module_count":packages.len(),"required_document_count":archive.manifest.documents.iter().filter(|e|e.required).count()}),
    );
    println!("New Host reopen/actual media read/new_external prepared passed");
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    match args[1].as_str() {
        "build" => build(
            Path::new(&args[2]),
            Path::new(&args[3]),
            Path::new(&args[4]),
        ),
        "verify" => verify(Path::new(&args[2]), Path::new(&args[3])),
        "geometry-build" => geometry::build_geometry(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4]), Path::new(&args[5])),
        "geometry-verify" => geometry::verify_geometry(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "multistage-build" => multistage::build(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4]), Path::new(&args[5])),
        "multistage-verify" => multistage::verify(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "full-factory-build" => full_factory::build(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "full-factory-connect" => full_factory::connect(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "full-factory-families" => full_factory::families(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "full-factory-bags" => full_factory::bags(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "full-factory-materials" => full_factory::materials(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "full-factory-checks" => full_factory::checks(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        "full-factory-template" => full_factory::template(Path::new(&args[2]), Path::new(&args[3])),
        "full-factory-template-families" => full_factory::template_families(Path::new(&args[2]), Path::new(&args[3])),
        "full-factory-template-bags" => full_factory::template_bags(Path::new(&args[2]), Path::new(&args[3])),
        "full-factory-observe" => full_factory::observe(Path::new(&args[2]), Path::new(&args[3]), Path::new(&args[4])),
        _ => panic!("build seed new-project receipt | verify project private-output"),
    }
}
