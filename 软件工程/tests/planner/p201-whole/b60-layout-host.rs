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
    let root = PathBuf::from(&args[1]); let variant = &args[2]; let mode = &args[3];
    let saved = root.join("saved").join(variant);
    let mut probe = Probe::new(&format!("{variant}-{mode}"));
    if mode == "reopen" {
        probe.open(&saved);
        let receipt: Value = serde_json::from_slice(&fs::read(root.join(format!("{variant}-host.json"))).unwrap()).unwrap();
        let run = probe.full(&receipt["run_id"]);
        assert_eq!(run["final_items"].as_array().unwrap().len(), 3000);
        assert_eq!(run["input_ref"]["project_revision"], 7);
        let archive = sf_commands::read_project_archive(&saved).unwrap();
        let project = serde_json::to_value(archive.project).unwrap();
        assert!(project["locations"].as_array().unwrap().iter().all(|l| l["item_layout"]["slots"].as_array().is_some_and(|s| !s.is_empty())));
        fs::write(root.join(format!("{variant}-reopen.json")), serde_json::to_vec_pretty(&json!({"pid":std::process::id(),
            "run_id":run["id"],"revision":7,"all_location_layouts_retained":true,"items":3000,"new_OS_process":true})).unwrap()).unwrap();
        println!("{variant}: fresh process reopen passed"); return;
    }
    let input = root.join("inputs").join(variant);
    let archive = sf_commands::read_project_archive(&input).unwrap();
    let scenario = serde_json::to_value(archive.scenarios.values().next().unwrap()).unwrap();
    probe.open(&input);
    let task = probe.call(false, "run_scenario", json!({"scenario_id":scenario["id"],"input_revision":7,
        "observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}));
    let outcome = probe.finish(false, &task); let run = probe.full(&outcome["run_id"]);
    assert_eq!(run["computed_until_ms"],1200000); assert_eq!(run["jobs"].as_array().unwrap().len(),30000);
    assert_eq!(run["events"].as_array().unwrap().len(),155994); assert_eq!(run["acceptance_events"].as_array().unwrap().len(),3000);
    fs::create_dir_all(&saved).unwrap();
    let selected = probe.host.select_directory(&serde_json::from_value(json!("B60-layout-owner")).unwrap(),
        serde_json::from_value(json!("save_project")).unwrap(), saved.clone()).unwrap();
    let task = probe.call(false,"save_project",json!({"revision":7,"selected_directory_ref":selected}));
    probe.finish(false,&task);
    let saved_archive = sf_commands::read_project_archive(&saved).unwrap();
    assert_eq!(serde_json::to_value(archive.project).unwrap(), serde_json::to_value(saved_archive.project).unwrap());
    let receipt = json!({"pid":std::process::id(),"variant":variant,"run_id":run["id"],"revision":7,
        "jobs":30000,"events":155994,"accepted_items":3000,"saved_project_equal":true,"Native":false,"formal_NFR":false});
    fs::write(root.join(format!("{variant}-host.json")),serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    println!("{receipt}");
}
