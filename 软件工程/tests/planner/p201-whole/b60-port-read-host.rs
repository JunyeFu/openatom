//! Actual ProjectHost reception for the two explicitly declared B60 display layouts.
use serde_json::{json, Value};
use sf_commands::ProjectHost;
use std::{fs, path::{Path, PathBuf}};

struct Probe { host: ProjectHost, project: String, seq: u64 }
impl Probe {
    fn new(label: &str) -> Self {
        Self { host: ProjectHost::new(serde_json::from_value(json!(format!("B60-layout-{label}-{}", std::process::id()))).unwrap(),
            serde_json::from_value(json!("B60-layout-owner")).unwrap()), project: String::new(), seq: 0 }
    }
    fn call(&mut self, workspace: bool, operation: &str, params: Value) -> Value {
        self.seq += 1;
        let request = serde_json::from_value(json!({"api_version":"0.1","request_id":format!("layout-{}",self.seq),
            "project_id":if workspace { Value::Null } else { json!(self.project) },
            "scope":{"kind":if workspace {"workspace"} else {"project"}},"operation":operation,"params":params})).unwrap();
        let response = self.host.dispatch(&serde_json::from_value(json!("B60-layout-owner")).unwrap(), request);
        assert_eq!(response["outcome"]["status"], "ok", "{operation}: {response:#}");
        response["outcome"]["data"].clone()
    }
    fn finish(&mut self, workspace: bool, task: &Value) -> Value {
        for work in self.host.drain_io_work() { self.host.finish_io(work.run()); }
        let receipt = self.call(workspace, "get_task", json!({"task_id":task["id"]}));
        assert_eq!(receipt["task"]["state"], "completed", "{receipt:#}");
        receipt["result"]["data"].clone()
    }
    fn open(&mut self, path: &Path) {
        let selected = self.host.select_directory(&serde_json::from_value(json!("B60-layout-owner")).unwrap(),
            serde_json::from_value(json!("import_package")).unwrap(), path.to_path_buf()).unwrap();
        let task = self.call(true, "import_package", json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}));
        let draft = self.finish(true, &task);
        self.call(true, "apply_commands", json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{
            "task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],
            "module_versions":draft["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},
            "selected_assets":["p201.b60.station.surface.1.0.5"],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}));
        self.project = sf_commands::read_project_archive(path).unwrap().project.id.as_str().to_owned();
    }
    fn full(&mut self, id: &Value) -> Value {
        self.call(false, "get_run", json!({"run_id":id,"query":{"kind":"full_result"}}))["data"].clone()
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]); let variant = &args[2]; let output = PathBuf::from(&args[3]);
    let saved = root.join("saved").join(variant);
    let mut probe = Probe::new(&format!("{variant}-ports")); probe.open(&saved);
    let receipt: Value = serde_json::from_slice(&fs::read(root.join(format!("{variant}-host.json"))).unwrap()).unwrap();
    let run = probe.full(&receipt["run_id"]);
    let archive = sf_commands::read_project_archive(&saved).unwrap();
    let history_entry = archive.manifest.documents.iter().find(|entry| entry.object_ref.id.as_str() == receipt["run_id"].as_str().unwrap() && entry.file_ref.path.as_str().ends_with("/item-history.json")).unwrap();
    let history: Value = serde_json::from_slice(&archive.files[history_entry.file_ref.path.as_str()]).unwrap();
    let records = history["data"]["port_outputs"].as_array().unwrap();
    let ui = serde_json::from_value(json!("B60-layout-owner")).unwrap();
    let run_id = serde_json::from_value(receipt["run_id"].clone()).unwrap();
    let mut observations = vec![];
    for at in [600000u64, 0, 1200000, 600000, 0, 1200000] {
        // Exercise actual_values' per-timepoint reset between warm port reads.
        probe.host.read_run_observation_ui(&ui, &archive.project.id, &serde_json::from_value(json!({"kind":"project"})).unwrap(), &archive.project.revision, &run_id, &serde_json::from_value(json!(at)).unwrap()).unwrap();
        let mut elapsed_ms = 0.; let mut values = vec![];
        for device in &archive.project.devices {
            let reference = serde_json::from_value(json!({"module_ref":device.module_ref,"instance_id":device.id,"port_id":"output"})).unwrap();
            let started = std::time::Instant::now();
            let value = probe.host.read_run_port_output(&ui, &archive.project.id, &serde_json::from_value(json!({"kind":"project"})).unwrap(), &archive.project.revision, &run_id, &serde_json::from_value(json!(at)).unwrap(), &reference).unwrap();
            elapsed_ms += started.elapsed().as_secs_f64()*1000.;
            let value = serde_json::to_value(value).unwrap();
            for (transferred, field) in [(false,"completed_output"),(true,"accepted_output")] {
                let expected: u64 = records.iter().filter(|row| row[0].as_u64().unwrap() <= at && row[2]["kind"] == "project_device" && row[2]["data"]["device_id"] == json!(device.id) && row[3] == "output" && row[6] == transferred).map(|row| { assert_eq!(row[5]["value"]["decimal_places"],0); assert_eq!(row[5]["unit"],"piece"); row[5]["value"]["coefficient"].as_u64().unwrap() }).sum();
                assert_eq!(value[field]["value"]["coefficient"],expected); assert_eq!(value[field]["value"]["decimal_places"],0); assert_eq!(value[field]["unit"],"piece");
            }
            values.push(value);
        }
        observations.push(json!({"at_ms":at,"all_ports_elapsed_ms":elapsed_ms,"ports":values}));
    }
    assert_eq!(probe.full(&receipt["run_id"]),run);
    fs::write(output, serde_json::to_vec_pretty(&json!({"variant":variant,"run_id":receipt["run_id"],"project_revision":7,"port_count":60,"observations":observations,"original_Run_unchanged":true,"fresh_process":true,"Host":true,"Native":false,"formal_NFR":false})).unwrap()).unwrap();
    println!("{variant}: 360 actual port reads match original saved history, including reverse seeks");
}
