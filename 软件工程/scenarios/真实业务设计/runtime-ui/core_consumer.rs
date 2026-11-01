//! Isolated real ProjectHost/Commands/Engine consumer, no replacement model.
use serde_json::{Value,json};
use sf_commands::ProjectHost;
use std::{fs,io::{self,BufRead,Write},path::{Path,PathBuf}};
fn write(path:&Path,value:&Value){fs::write(path,serde_json::to_vec_pretty(value).unwrap()).unwrap();}
fn doc(role:&str,kind:&str,id:&str,path:&str,revision:Value)->Value{json!({"role":role,"object_ref":{"kind":kind,"id":id},"revision":revision,"module_ref":null,"file_ref":{"path":path,"kind":kind},"required":true})}
fn seed(directory:&Path,input:&Value){
    fs::create_dir_all(directory).unwrap();let project=&input["project"];let scenario=&input["scenario"];
    let mut entries=vec![doc("active_project","project",project["id"].as_str().unwrap(),"active.json",project["revision"].clone()),doc("scenario_input","scenario",scenario["id"].as_str().unwrap(),"scenario.json",scenario["revision"].clone())];
    write(&directory.join("active.json"),&json!({"schema_version":"1.0","data":project}));write(&directory.join("scenario.json"),&json!({"schema_version":"1.0","data":scenario}));
    for (i,definition) in input["definitions"].as_array().unwrap().iter().enumerate(){let path=format!("module-{i}.json");let mut entry=doc("module_definition","module",definition["id"].as_str().unwrap(),&path,Value::Null);entry["module_ref"]=json!({"id":definition["id"],"version":definition["version"]});entries.push(entry);write(&directory.join(path),&json!({"schema_version":"1.0","data":{"definition":definition,"assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]}}));}
    write(&directory.join("project.json"),&json!({"schema_version":"1.0","data":{"project_id":project["id"],"project_revision":project["revision"],"title":project["title"],"generation":1,"documents":entries,"module_refs":project["module_refs"],"assets":[],"missing":[],"restart_policy":"restart_from_initial_input"}}));
}
struct Core{host:ProjectHost,seq:u64,history:Value,project_id:String,baseline:Option<Value>}
impl Core{
 fn call(&mut self,project:bool,operation:&str,params:Value)->Value{
    let id=project.then(||self.project_id.clone());self.for_project(id.as_deref(),operation,params)
 }
 fn for_project(&mut self,project_id:Option<&str>,operation:&str,params:Value)->Value{
    self.seq+=1;let request=serde_json::from_value(json!({"api_version":"0.1","request_id":format!("P205-consumer-{}",self.seq),"project_id":project_id,"scope":{"kind":if project_id.is_some(){"project"}else{"workspace"}},"operation":operation,"params":params})).unwrap();
    self.host.dispatch(&serde_json::from_value(json!("P205-ui")).unwrap(),request)
 }
 fn data(reply:Value)->Value{assert_eq!(reply["outcome"]["status"],"ok","{reply:#}");reply["outcome"]["data"].clone()}
 fn finish(&mut self){let work=self.host.drain_io_work();assert!(!work.is_empty());for work in work{self.host.finish_io(work.run());}}
 fn object(&mut self,kind:&str,id:&str,source:Value)->Value{Self::data(self.call(true,"get_object",json!({"object_id":{"kind":kind,"id":id},"source":source,"node_index":null})))["data"].clone()}
 fn snapshot(&mut self)->Value{
    let id=self.project_id.clone();let project=self.object("project",&id,json!({"kind":"draft"}));
    let scenarios=project["scenario_refs"].as_array().unwrap().iter().map(|s|self.object("scenario",s.as_str().unwrap(),json!({"kind":"draft"}))).collect::<Vec<_>>();
    let modules=project["module_refs"].as_array().unwrap().iter().map(|m|self.object("module",m["id"].as_str().unwrap(),json!({"kind":"module_version","data":{"module_ref":m}}))).collect::<Vec<_>>();
    json!({"snapshot":{"project":project,"scenarios":scenarios},"modules":modules,"history":self.history})
 }
 fn run(&mut self,scenario_id:&str,revision:Value)->Value{
    let scenario=self.object("scenario",scenario_id,json!({"kind":"draft"}));
    let handle=Self::data(self.call(true,"run_scenario",json!({"scenario_id":scenario_id,"input_revision":revision,"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}})));
    self.finish();let task=Self::data(self.call(true,"get_task",json!({"task_id":handle["id"]})));assert_eq!(task["task"]["state"],"completed","{task:#}");let id=task["result"]["data"]["run_id"].clone();
    Self::data(self.call(true,"get_run",json!({"run_id":id,"query":{"kind":"full_result"}})))["data"].clone()
 }
 fn context(&mut self)->Value{Self::data(self.call(true,"get_context",json!({"selection":[]})))}
 fn reply(&mut self,query:&Value)->Value{
    match query["kind"].as_str().unwrap(){
      "refresh"=>self.snapshot(),
      "read_run"=>Self::data(self.call(true,"get_run",json!({"run_id":query["run_id"],"query":{"kind":"full_result"}})))["data"].clone(),
      "commit"=>{
        let proposal=&query["proposal"];let before=self.baseline.clone();
        let reply=self.for_project(proposal["project_id"].as_str(),"apply_commands",json!({"base_revision":proposal["base_project_revision"],"commands":proposal["commands"]}));
        if reply["outcome"]["status"]!="ok"{return json!({"ok":false,"error":reply["outcome"]["data"]});}
        let commit=Self::data(reply);self.history=commit["history"].clone();let state=self.snapshot();
        if let Some(old)=before{let actual=Self::data(self.call(true,"get_run",json!({"run_id":old["id"],"query":{"kind":"full_result"}})))["data"].clone();assert_eq!(old,actual,"draft Commands must preserve frozen Run");}
        json!({"ok":true,"commit":commit,"snapshot":state["snapshot"],"modules":state["modules"],"history":self.history,"old_Run_unchanged":true})
      },
      "run"=>{
        let request=&query["request"];let old=self.baseline.clone();let result=self.run(request["scenario_id"].as_str().unwrap(),request["base_project_revision"].clone());
        if let Some(old)=old{let actual=Self::data(self.call(true,"get_run",json!({"run_id":old["id"],"query":{"kind":"full_result"}})))["data"].clone();assert_eq!(old,actual);assert_ne!(old["id"],result["id"]);}
        let at=result["computed_until_ms"].clone();let frozen_project=self.object("project",&self.project_id.clone(),json!({"kind":"run_snapshot","data":{"run_id":result["id"],"at_ms":at}}));let frozen_scenario=self.object("scenario",result["input_ref"]["scenario_id"].as_str().unwrap(),json!({"kind":"run_snapshot","data":{"run_id":result["id"],"at_ms":at}}));
        json!({"run":result,"frozenProject":frozen_project,"frozenScenario":frozen_scenario,"state":self.snapshot(),"old_Run_unchanged":true})
      },
      "concurrent_edit"=>{
        let id=query["scenario_id"].as_str().unwrap();let mut scenario=self.object("scenario",id,json!({"kind":"draft"}));if query["field"]=="due"{scenario["orders"][0]["due_at_ms"]=json!(175000);}else{scenario["title"]=json!("来自另一次真实编辑的方案名称");}let revision=self.context()["project"]["revision"].clone();
        let commit=Self::data(self.call(true,"apply_commands",json!({"base_revision":revision,"commands":[{"kind":"put_object","data":{"kind":"scenario","data":scenario}}]})));self.history=commit["history"].clone();self.snapshot()
      },
      "concurrent_parameter"=>{
        let revision=self.context()["project"]["revision"].clone();
        let commit=Self::data(self.call(true,"apply_commands",json!({"base_revision":revision,"commands":[{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":query["device_id"]},"parameter_id":"unload_ms","value":{"kind":"integer","value":9000},"basis":"user_provided"}}]})));self.history=commit["history"].clone();self.snapshot()
      },
      "observe"=>Self::data(self.call(true,"get_run",json!({"run_id":query["run_id"],"query":{"kind":"objects_at","data":{"at_ms":query["at_ms"],"objects":[],"page":{"after_index":null,"limit":500}}}})))["data"].clone(),
      "new_project"=>{
        let commit=Self::data(self.call(false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"P205明确切换后的独立工程","time_reference":{"label":"explicit relative milliseconds","origin_iso8601":null,"timezone":null},"departure":"discard_after_tasks_terminal"}}]})));
        self.project_id=commit["project_id"].as_str().unwrap().to_owned();self.history=commit["history"].clone();self.baseline=None;self.snapshot()
      },
      _=>panic!("unknown explicit consumer action")
    }
 }
}
fn main(){
    let root=PathBuf::from(std::env::args().nth(1).unwrap());let fixture=std::env::args().nth(2).unwrap_or_else(||"packages/commands/tests/fixtures/production/p104-A-first.json".into());let input:Value=serde_json::from_slice(&fs::read(root.join(&fixture)).unwrap()).unwrap();
    let directory=root.join(format!("scenarios/真实业务设计/runtime-ui/core-session-{}",std::process::id()));seed(&directory,&input);
    let mut core=Core{host:ProjectHost::new(serde_json::from_value(json!(format!("P205-core-{}",std::process::id()))).unwrap(),serde_json::from_value(json!("P205-ui")).unwrap()),seq:0,history:Value::Null,project_id:input["project"]["id"].as_str().unwrap().to_owned(),baseline:None};
    let selected=core.host.select_directory(&serde_json::from_value(json!("P205-ui")).unwrap(),serde_json::from_value(json!("import_package")).unwrap(),directory).unwrap();
    let task=Core::data(core.call(false,"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));core.finish();let state=Core::data(core.call(false,"get_task",json!({"task_id":task["id"]})));let draft=&state["result"]["data"];
    let commit=Core::data(core.call(false,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]})));core.history=commit["history"].clone();
    let initial=core.snapshot();let scenario_id=initial["snapshot"]["scenarios"][0]["id"].as_str().unwrap();let run=core.run(scenario_id,initial["snapshot"]["project"]["revision"].clone());core.baseline=Some(run.clone());
    let settings:sf_commands::ProjectSettings=serde_json::from_value(json!({"title":"schema carrier probe only","time_reference":input["project"]["time_reference"],"generation_context":null,"sources":[],"setup_policies":input["project"]["setup_policies"]})).unwrap();
    let supported=serde_json::to_value(settings).unwrap().get("setup_policies").is_some();
    let frozen_project=core.object("project",&core.project_id.clone(),json!({"kind":"run_snapshot","data":{"run_id":run["id"],"at_ms":run["computed_until_ms"]}}));
    let frozen_scenario=core.object("scenario",run["input_ref"]["scenario_id"].as_str().unwrap(),json!({"kind":"run_snapshot","data":{"run_id":run["id"],"at_ms":run["computed_until_ms"]}}));
    let ready=json!({"ready":true,"producer":"actual sf_commands::ProjectHost + existing Engine","fixture":fixture,"state":core.snapshot(),"run":run,"frozenProject":frozen_project,"frozenScenario":frozen_scenario,"setup_policy_write_supported":supported});
    let name=if fixture.ends_with("p104-A-first.json"){"core-initial.actual.json".to_owned()}else{format!("core-initial-{}.actual.json",Path::new(&fixture).file_stem().unwrap().to_string_lossy())};
    write(&root.join("scenarios/真实业务设计/runtime-ui").join(name),&ready);println!("{ready}");io::stdout().flush().unwrap();
    for line in io::stdin().lock().lines(){let query:Value=serde_json::from_str(&line.unwrap()).unwrap();let reply=core.reply(&query);println!("{reply}");io::stdout().flush().unwrap();}
}
