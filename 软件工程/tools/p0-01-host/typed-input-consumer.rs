//! Actual Core directory consumer; deliberately does not claim Native picker or UI evidence.
use serde_json::{json, Value};
use sf_commands::{FilePurpose, ProjectHost};
use sf_domain::{ObjectId, ProjectState};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn id(value: &str) -> ObjectId {
    ObjectId::try_from(value.to_owned()).unwrap()
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
struct Consumer {
    host: ProjectHost,
    ui: ObjectId,
    project: Option<Value>,
    out: PathBuf,
    count: usize,
}
impl Consumer {
    fn new(name: &str, out: &Path) -> Self {
        let ui = id(&format!("{name}-ui"));
        Self {
            host: ProjectHost::new(id(name), ui.clone()),
            ui,
            project: None,
            out: out.to_owned(),
            count: 0,
        }
    }
    fn raw(&mut self, operation: &str, params: Value) -> Value {
        self.count += 1;
        let request = json!({"api_version":"0.1","request_id":format!("p001-{}",self.count),
            "project_id":self.project,"scope":{"kind":if self.project.is_some(){"project"}else{"workspace"}},"operation":operation,"params":params});
        let response = self
            .host
            .dispatch(&self.ui, serde_json::from_value(request.clone()).unwrap());
        write(
            &self.out.join(format!("{:03}-{operation}.json", self.count)),
            &json!({"request":request,"response":response}),
        );
        response
    }
    fn call(&mut self, operation: &str, params: Value) -> Value {
        let response = self.raw(operation, params);
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        response["outcome"]["data"].clone()
    }
    fn context(&mut self) -> Value {
        self.call("get_context", json!({"selection":[]}))
    }
    fn project(&mut self) -> Value {
        self.call("get_object", json!({"object_id":{"kind":"project","id":self.project},"source":{"kind":"draft"},"node_index":null}))["data"].clone()
    }
    fn edit(&mut self, base: Value, command: Value) -> Value {
        self.call(
            "apply_commands",
            json!({"base_revision":base,"commands":[command]}),
        )
    }
    fn finish(&mut self, handle: &Value) -> Value {
        let mut work = self.host.drain_io_work();
        assert_eq!(work.len(), 1);
        self.host.finish_io(work.pop().unwrap().run());
        let task = self.call("get_task", json!({"task_id":handle["id"]}));
        assert_eq!(task["task"]["state"], "completed", "{task}");
        task["result"].clone()
    }
    fn open(&mut self, directory: &Path) {
        let selected = self
            .host
            .select_directory(&self.ui, FilePurpose::ImportPackage, directory.to_owned())
            .unwrap();
        let task = self.call("import_package", json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}));
        let result = self.finish(&task);
        assert_eq!(result["kind"], "import");
        let draft = &result["data"];
        assert!(draft["conflicts"].as_array().unwrap().is_empty(), "{draft}");
        assert_eq!(draft["missing"].as_array().unwrap().len(), 1, "{draft}");
        assert_eq!(
            draft["missing"][0]["code"], "professional_inputs.nonhuman_acknowledgment",
            "{draft}"
        );
        self.edit(Value::Null, json!({"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{
            "chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],
            "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],
            "accept_missing_as_draft":true,"departure":"discard_after_tasks_terminal"}}}));
        self.project = Some(self.context()["project"]["id"].clone());
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input = fs::canonicalize(&args[1]).unwrap();
    let out = PathBuf::from(&args[2]);
    fs::create_dir(&out).unwrap();
    let out = fs::canonicalize(out).unwrap();
    let mut consumer = Consumer::new("p001-typed-core", &out);
    consumer.open(&input);
    let actual = consumer.project();
    let expected: ProjectState =
        serde_json::from_value(read(&input.join("payload/project-state.json"))["data"].clone())
            .unwrap();
    let expected = serde_json::to_value(expected).unwrap();
    assert_eq!(actual["issues"].as_array().unwrap().len(), 1);
    assert_eq!(
        actual["issues"][0]["code"],
        "professional_inputs.nonhuman_acknowledgment"
    );
    assert_eq!(
        actual["issues"][0]["blocks"],
        json!(["execution", "complete_conclusion"])
    );
    for (key, value) in expected.as_object().unwrap() {
        if key != "issues" {
            assert_eq!(
                &actual[key], value,
                "Actual Core must retain typed Project field: {key}"
            );
        }
    }
    let saved = out.join("saved");
    fs::create_dir(&saved).unwrap();
    let selected = consumer
        .host
        .select_directory(&consumer.ui, FilePurpose::SaveProject, saved.clone())
        .unwrap();
    let task = consumer.call(
        "save_project",
        json!({"revision":actual["revision"],"selected_directory_ref":selected}),
    );
    consumer.finish(&task);
    assert_eq!(consumer.context()["project"]["dirty"], false);
    let saved_manifest = fs::read(saved.join("project.json")).unwrap();
    let mut edited = actual["halls"][0].clone();
    edited["title"] = json!("Native typed consumer branch");
    let base = actual["revision"].as_u64().unwrap();
    consumer.edit(
        json!(base),
        json!({"kind":"put_object","data":{"kind":"hall","data":edited}}),
    );
    consumer.edit(json!(base + 1), json!({"kind":"undo"}));
    assert_eq!(consumer.project()["halls"], actual["halls"]);
    consumer.edit(json!(base + 2), json!({"kind":"redo"}));
    assert_eq!(consumer.project()["halls"][0], edited);
    let current = consumer.project();
    let conflict = consumer.raw(
        "apply_commands",
        json!({"base_revision":base,"commands":[{"kind":"undo"}]}),
    );
    assert_ne!(conflict["outcome"]["status"], "ok");
    assert_eq!(consumer.project(), current);
    consumer.edit(json!(base + 3), json!({"kind":"undo"}));
    edited["title"] = json!("Native typed consumer new branch");
    consumer.edit(
        json!(base + 4),
        json!({"kind":"put_object","data":{"kind":"hall","data":edited}}),
    );
    let before_redo = consumer.project();
    let redo = consumer.raw(
        "apply_commands",
        json!({"base_revision":base+5,"commands":[{"kind":"redo"}]}),
    );
    assert_ne!(redo["outcome"]["status"], "ok");
    assert_eq!(consumer.project(), before_redo);
    assert_eq!(
        fs::read(saved.join("project.json")).unwrap(),
        saved_manifest
    );
    // Both paths are fixed descendants of this newly created, canonical owned output folder.
    let moved = out.join("moved");
    fs::rename(&saved, &moved).unwrap();
    let reopened_out = out.join("fresh-host");
    fs::create_dir(&reopened_out).unwrap();
    let mut fresh = Consumer::new("p001-typed-fresh-core", &reopened_out);
    fresh.open(&moved);
    assert_eq!(fresh.project(), actual);
    let old_base = fresh.project()["revision"].clone();
    fresh.edit(
        old_base,
        json!({"kind":"put_object","data":{"kind":"hall","data":edited}}),
    );
    let result = json!({"kind":"actual_Core_directory_roundtrip_not_Native_UI","passed":true,
        "typed_project_fields_retained":true,"actual_save_and_move_and_fresh_host_reopen":true,
        "new_revisions_for_undo_redo":true,"stale_revision_retained":true,"invalid_redo_retained":true,
        "saved_manifest_unchanged_by_history":true,"reopened_project_editable":true,
        "input_professional_outcome":"undetermined","human_acceptance":false,"whole_task_done":false});
    write(&out.join("result.json"), &result);
    println!("{result}");
}
