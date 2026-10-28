//! Private stdin/stdout adapter to the frozen public Host. No listener or Engine copy.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use std::{io::{self, BufRead, Write}, path::Path};
use support::*;

struct Session { harness: Harness, project: String, sequence: u64 }
impl Session {
    fn call(&mut self, operation: &str, params: Value) -> Value {
        self.sequence += 1;
        let request: ApiRequest = serde_json::from_value(json!({"api_version":"0.1",
            "request_id":format!("p205-{}",self.sequence),"project_id":self.project,
            "scope":{"kind":"project"},"operation":operation,"params":params})).unwrap();
        self.harness.host.dispatch(&self.harness.ui, request)
    }
    fn object(&mut self, kind: &str, id: &str) -> Value {
        let value: Value = ok(self.call("get_object", json!({"object_id":{"kind":kind,"id":id},
            "source":{"kind":"draft"},"node_index":null})));
        value["data"].clone()
    }
    fn snapshot(&mut self) -> Value {
        let project = self.object("project", &self.project.clone());
        let scenarios: Vec<_> = project["scenario_refs"].as_array().unwrap().iter()
            .map(|id|self.object("scenario",id.as_str().unwrap())).collect();
        let context: Value = ok(self.call("get_context",json!({"selection":[]})));
        json!({"project":project,"scenarios":scenarios,"issues":context["issues"],"tasks":context["tasks"]})
    }
    fn full(&mut self, id: Value) -> Value {
        let view: RunView = ok(self.call("get_run", json!({"run_id":id,"query":{"kind":"full_result"}})));
        match view {RunView::Full(run)=>serde_json::to_value(run).unwrap(),other=>panic!("full result required: {other:?}")}
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let (harness, project) = match args[1].as_str() {
        "--open" => {
            let path = Path::new(&args[2]);
            let manifest = read(&path.join("project.json"));
            (Harness::from_seed(Sandbox::new(), path),manifest["data"]["project_id"].as_str().unwrap().to_owned())
        }
        "--fixture" => {
            let value=read(Path::new(&args[2]));let project=value["project"]["id"].as_str().unwrap().to_owned();
            let tree=Sandbox::new();let directory=tree.directory("seed");seed(&directory,&value);
            // The original support's seed label is P1. For the explicit second
            // project fixture, make its manifest identity match its real DTO.
            let mut manifest=read(&directory.join("project.json"));manifest["data"]["project_id"]=json!(project);
            for document in manifest["data"]["documents"].as_array_mut().unwrap() {
                if document["object_ref"]["kind"]=="project" {document["object_ref"]["id"]=json!(project);}
            }
            write(&directory.join("project.json"),&manifest);
            (Harness::from_seed(tree,&directory),project)
        }
        _ => panic!("--open or --fixture required"),
    };
    let mut session=Session{harness,project,sequence:0};
    for line in io::stdin().lock().lines() {
        let request:Value=serde_json::from_str(&line.unwrap()).unwrap();
        let result=match request["op"].as_str().unwrap() {
            "snapshot"=>session.snapshot(),
            "call"=>session.call(request["operation"].as_str().unwrap(),request["params"].clone()),
            "full"=>session.full(request["run"].clone()),
            "run"=>{
                let scenario=session.object("scenario",request["scenario"].as_str().unwrap());
                let current:Value=ok(session.call("get_context",json!({"selection":[]})));
                let task:Value=ok(session.call("run_scenario",json!({"scenario_id":scenario["id"],
                    "input_revision":current["project"]["revision"],"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}})));
                session.harness.finish_one();
                let state:Value=ok(session.call("get_task",json!({"task_id":task["id"]})));
                assert_eq!(state["task"]["state"],"completed");
                session.full(state["result"]["data"]["run_id"].clone())
            }
            "save"=>{
                let current:Value=ok(session.call("get_context",json!({"selection":[]})));
                let selected=session.harness.host.select_directory(&session.harness.ui,FilePurpose::SaveProject,Path::new(request["directory"].as_str().unwrap()).to_owned()).unwrap();
                let task:Value=ok(session.call("save_project",json!({"revision":current["project"]["revision"],"selected_directory_ref":selected})));
                session.harness.finish_one();
                ok::<Value>(session.call("get_task",json!({"task_id":task["id"]})))
            }
            _=>panic!("unknown private verification operation"),
        };
        println!("{}",serde_json::to_string(&result).unwrap());io::stdout().flush().unwrap();
    }
}
