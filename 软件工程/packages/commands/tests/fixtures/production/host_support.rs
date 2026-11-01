#![allow(dead_code)]
//! Shared setup for independent whole-package consumers, with real directories.
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(1);

pub fn id(s: &str) -> ObjectId {
    ObjectId::try_from(s.to_owned()).unwrap()
}
pub fn revision(n: u64) -> Revision {
    Revision(SafeUint::try_from(n).unwrap())
}
pub fn fixture() -> Value {
    serde_json::from_str(include_str!("local-assembly-pack.json")).unwrap()
}
pub fn put(kind: &str, data: Value) -> Value {
    json!({"kind":"put_object","data":{"kind":kind,"data":data}})
}
pub fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
pub fn ok<T: DeserializeOwned>(reply: Value) -> T {
    match serde_json::from_value::<Response<T>>(reply.clone())
        .unwrap()
        .outcome
    {
        ApiOutcome::Ok(value) => value,
        _ => panic!("expected success: {reply:#}"),
    }
}
pub fn error(reply: Value) -> ApiError {
    match serde_json::from_value::<Response<Value>>(reply.clone())
        .unwrap()
        .outcome
    {
        ApiOutcome::Error(error) => error,
        _ => panic!("expected error: {reply:#}"),
    }
}

pub struct Sandbox {
    base: PathBuf,
    pub path: PathBuf,
}
impl Sandbox {
    pub fn new() -> Self {
        let base = std::env::temp_dir().canonicalize().unwrap();
        let path = base.join(format!(
            "sf-whole-w5-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let path = path.canonicalize().unwrap();
        assert_eq!(path.parent(), Some(base.as_path()));
        Self { base, path }
    }
    pub fn directory(&self, name: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::create_dir(&path).unwrap();
        path
    }
}
impl Drop for Sandbox {
    fn drop(&mut self) {
        if self.path.parent() == Some(self.base.as_path()) {
            if let Err(e) = fs::remove_dir_all(&self.path) {
                eprintln!("fixture cleanup failed: {e}");
            }
        }
    }
}
pub fn copy(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            fs::create_dir(&dest).unwrap();
            copy(&entry.path(), &dest);
        } else {
            assert!(kind.is_file());
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

pub fn document(
    role: &str,
    kind: &str,
    object_kind: &str,
    object_id: &str,
    path: &str,
    rev: Value,
) -> Value {
    json!({"role":role,"object_ref":{"kind":object_kind,"id":object_id},"revision":rev,
        "module_ref":null,"file_ref":{"path":path,"kind":kind},"required":true})
}
pub fn seed(directory: &Path, value: &Value) {
    let mut entries = vec![
        document(
            "active_project",
            "project",
            "project",
            "P1",
            "active.json",
            json!(1),
        ),
        document(
            "scenario_input",
            "scenario",
            "scenario",
            value["scenario"]["id"].as_str().unwrap(),
            "scenario.json",
            json!(1),
        ),
    ];
    write(
        &directory.join("active.json"),
        &json!({"schema_version":"1.0","data":value["project"]}),
    );
    write(
        &directory.join("scenario.json"),
        &json!({"schema_version":"1.0","data":value["scenario"]}),
    );
    for (i, definition) in value["definitions"].as_array().unwrap().iter().enumerate() {
        let path = format!("module-{i}.json");
        let mut entry = document(
            "module_definition",
            "module",
            "module",
            definition["id"].as_str().unwrap(),
            &path,
            Value::Null,
        );
        entry["module_ref"] = json!({"id":definition["id"],"version":definition["version"]});
        entries.push(entry);
        write(
            &directory.join(path),
            &json!({"schema_version":"1.0","data":{
                "definition":definition,"assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]
            }}),
        );
    }
    write(
        &directory.join("project.json"),
        &json!({"schema_version":"1.0","data":{
            "project_id":"P1","project_revision":1,"title":value["project"]["title"],"generation":1,
            "documents":entries,"module_refs":value["project"]["module_refs"],"assets":[],"missing":[],
            "restart_policy":"restart_from_initial_input"
        }}),
    );
}

pub struct Harness {
    pub host: ProjectHost,
    pub ui: ObjectId,
    pub tree: Sandbox,
}
impl Harness {
    pub fn new() -> Self {
        Self::from_value(fixture())
    }
    pub fn from_value(value: Value) -> Self {
        let tree = Sandbox::new();
        let directory = tree.directory("seed");
        seed(&directory, &value);
        Self::from_seed(tree, &directory)
    }
    pub fn from_seed(tree: Sandbox, directory: &Path) -> Self {
        let ui = id("whole-w5-ui");
        let host = ProjectHost::new(id(&format!("whole-w5-host-{}", std::process::id())), ui.clone());
        let mut out = Self { host, ui, tree };
        let handle = out.import(directory);
        out.finish_one();
        let draft = out.imported(&handle);
        let _: EditCommit = ok(out.adopt(&handle, &draft, false));
        out
    }
    pub fn call_as(
        &mut self,
        session: &ObjectId,
        project: bool,
        operation: &str,
        params: Value,
    ) -> Value {
        let req: ApiRequest = serde_json::from_value(json!({"api_version":"0.1",
            "request_id":format!("whole-w5-{}",NEXT.fetch_add(1,Ordering::Relaxed)),
            "project_id":if project {json!("P1")} else {Value::Null},
            "scope":{"kind":if project {"project"} else {"workspace"}},"operation":operation,"params":params
        })).unwrap();
        self.host.dispatch(session, req)
    }
    pub fn call(&mut self, operation: &str, params: Value) -> Value {
        self.call_as(&self.ui.clone(), true, operation, params)
    }
    pub fn apply(&mut self, base: u64, commands: Value) -> Value {
        self.call(
            "apply_commands",
            json!({"base_revision":base,"commands":commands}),
        )
    }
    pub fn object(&mut self, kind: &str, name: &str) -> Value {
        let value: DomainObject = ok(self.call("get_object",json!({"object_id":{"kind":kind,"id":name},"source":{"kind":"draft"},"node_index":null})));
        let value = serde_json::to_value(value).unwrap();
        assert_eq!(value["kind"], kind);
        value["data"].clone()
    }
    pub fn context(&mut self) -> ProjectContext {
        ok(self.call("get_context", json!({"selection":[]})))
    }
    pub fn snapshot(&mut self) -> Value {
        let project = self.object("project", "P1");
        let mut scenarios = serde_json::Map::new();
        for name in project["scenario_refs"].as_array().unwrap() {
            let name = name.as_str().unwrap();
            scenarios.insert(name.to_owned(), self.object("scenario", name));
        }
        let context = self.context();
        json!({"project":project,"scenarios":scenarios,"issues":context.issues,"tasks":context.tasks})
    }
    pub fn finish_one(&mut self) {
        let mut work = self.host.drain_io_work();
        assert_eq!(work.len(), 1);
        self.host.finish_io(work.pop().unwrap().run());
    }
    pub fn task(&mut self, task: &TaskHandle, project: bool) -> TaskState {
        ok(self.call_as(
            &self.ui.clone(),
            project,
            "get_task",
            json!({"task_id":task.id}),
        ))
    }
    pub fn import(&mut self, directory: &Path) -> TaskHandle {
        let selected = self
            .host
            .select_directory(&self.ui, FilePurpose::ImportPackage, directory.to_owned())
            .unwrap();
        ok(self.call_as(&self.ui.clone(),false,"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})))
    }
    pub fn imported(&mut self, handle: &TaskHandle) -> ImportDraft {
        let state = self.task(handle, false);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        match state.result.unwrap() {
            TaskOutput::Import(draft) => draft,
            other => panic!("not import: {other:?}"),
        }
    }
    pub fn adopt(&mut self, handle: &TaskHandle, draft: &ImportDraft, accept: bool) -> Value {
        self.call_as(&self.ui.clone(),false,"apply_commands",json!({"base_revision":null,"commands":[{
            "kind":"adopt_imported_draft","data":{"task_id":handle.id,"choices":{
                "chosen_policy":"open_project","objects_to_create":draft.proposed_objects,"module_versions":draft.proposed_definitions,
                "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],
                "accept_missing_as_draft":accept,"departure":"discard_after_tasks_terminal"
            }}
        }]}))
    }
    pub fn run(&mut self, scenario: &str) -> TaskHandle {
        let input = self.object("scenario", scenario);
        let scope_revision = self.context().project.unwrap().revision;
        ok(self.call(
            "run_scenario",
            json!({"scenario_id":scenario,"input_revision":scope_revision,
            "observation_window":input["window"],"intent":{"kind":"project_scenario"}}),
        ))
    }
    pub fn full(&mut self, id: &RunId) -> RunResult {
        match ok(self.call(
            "get_run",
            json!({"run_id":id,"query":{"kind":"full_result"}}),
        )) {
            RunView::Full(value) => *value,
            other => panic!("not full result: {other:?}"),
        }
    }
    pub fn completed_run(&mut self, task: &TaskHandle) -> RunResult {
        let state = self.task(task, true);
        assert!(
            matches!(state.task.state, ComputationStatus::Completed),
            "{state:?}"
        );
        match state.result.unwrap() {
            TaskOutput::Run(summary) => self.full(&summary.run_id),
            other => panic!("not run: {other:?}"),
        }
    }
    pub fn save(&mut self, base: u64, directory: &Path) -> TaskHandle {
        let selected = self
            .host
            .select_directory(&self.ui, FilePurpose::SaveProject, directory.to_owned())
            .unwrap();
        ok(self.call(
            "save_project",
            json!({"revision":base,"selected_directory_ref":selected}),
        ))
    }
}

pub fn same_business(mut a: Value, mut b: Value) {
    for key in ["revision", "project_revision"] {
        a.as_object_mut().unwrap().remove(key);
        b.as_object_mut().unwrap().remove(key);
    }
    assert_eq!(a, b);
}

