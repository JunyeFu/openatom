//! Existing actual ProjectHost/Commands/Engine/byte producer, isolated transport.
use serde_json::{Value,json};
use sf_commands::ProjectHost;
use std::{fs,io::{self,BufRead,Write},path::{Path,PathBuf}};
fn write(path:&Path,value:&Value){fs::write(path,serde_json::to_vec_pretty(value).unwrap()).unwrap();}
fn doc(role:&str,kind:&str,id:&str,path:&str,revision:Value)->Value{json!({"role":role,"object_ref":{"kind":kind,"id":id},"revision":revision,"module_ref":null,"file_ref":{"path":path,"kind":kind},"required":true})}
fn seed(root:&Path,directory:&Path,input:&Value){
 fs::create_dir_all(directory).unwrap();let project=&input["project"];let scenario=&input["scenario"];
 let mut entries=vec![doc("active_project","project",project["id"].as_str().unwrap(),"active.json",project["revision"].clone()),doc("scenario_input","scenario",scenario["id"].as_str().unwrap(),"scenario.json",scenario["revision"].clone())];
 write(&directory.join("active.json"),&json!({"schema_version":"1.0","data":project}));write(&directory.join("scenario.json"),&json!({"schema_version":"1.0","data":scenario}));
 let package_root=root.join("modules/devices/manual-assembly/presentation/p303-owned");let actual:Value=serde_json::from_slice(&fs::read(package_root.join(input["package_file"].as_str().unwrap_or("module.json"))).unwrap()).unwrap();
 for file in actual["data"]["files"].as_array().unwrap(){let relative=file["path"].as_str().unwrap();let target=directory.join(relative);fs::create_dir_all(target.parent().unwrap()).unwrap();fs::copy(package_root.join(relative),target).unwrap();}
 for(i,definition)in input["definitions"].as_array().unwrap().iter().enumerate(){
  let path=format!("module-{i}.json");let mut entry=doc("module_definition","module",definition["id"].as_str().unwrap(),&path,Value::Null);entry["module_ref"]=json!({"id":definition["id"],"version":definition["version"]});entries.push(entry);
  if definition["id"]==actual["data"]["definition"]["id"]{write(&directory.join(path),&actual);for file in actual["data"]["files"].as_array().unwrap(){let relative=file["path"].as_str().unwrap();let target=directory.join(relative);fs::create_dir_all(target.parent().unwrap()).unwrap();fs::copy(package_root.join(relative),target).unwrap();}}
  else{write(&directory.join(path),&json!({"schema_version":"1.0","data":{"definition":definition,"assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]}}));}
 }
 for asset in actual["data"]["assets"].as_array().unwrap(){entries.push(doc("asset","asset",asset["id"].as_str().unwrap(),asset["path"].as_str().unwrap(),Value::Null));}
 for adoption in project["template_adoptions"].as_array().unwrap(){let relative=adoption["source_archive"]["path"].as_str().unwrap();let target=directory.join(relative);fs::create_dir_all(target.parent().unwrap()).unwrap();fs::copy(root.join("modules/templates/purchased-assembly/candidate/source-project.design.json"),target).unwrap();entries.push(doc("supporting_file","asset","actual-template-source",relative,Value::Null));}
 entries.push(doc("supporting_file","asset","p303.license","LICENSE.txt",Value::Null));
 write(&directory.join("project.json"),&json!({"schema_version":"1.0","data":{"project_id":project["id"],"project_revision":project["revision"],"title":project["title"],"generation":1,"documents":entries,"module_refs":project["module_refs"],"assets":actual["data"]["rights"],"missing":[],"restart_policy":"restart_from_initial_input"}}));
}
struct Core{host:ProjectHost,seq:u64,project:String,draft:Value,formal:Value}
impl Core{
 fn request(&mut self,scope:Value,operation:&str,params:Value)->Value{self.seq+=1;let project=if scope["kind"]=="workspace"{Value::Null}else{json!(self.project)};self.raw(json!({"api_version":"0.1","request_id":format!("P303-{}",self.seq),"project_id":project,"scope":scope,"operation":operation,"params":params}))}
 fn raw(&mut self,request:Value)->Value{self.host.dispatch(&serde_json::from_value(json!("P303-ui")).unwrap(),serde_json::from_value(request.clone()).unwrap_or_else(|error|panic!("request {} scope {} params {}: {error}",request["operation"],request["scope"],request["params"])))}
 fn data(reply:Value)->Value{assert_eq!(reply["outcome"]["status"],"ok","{reply:#}");reply["outcome"]["data"].clone()}
 fn finish(&mut self){for work in self.host.drain_io_work(){self.host.finish_io(work.run());}}
 fn object(&mut self,scope:Value,kind:&str,id:&str,source:Value)->Value{Self::data(self.request(scope,"get_object",json!({"object_id":{"kind":kind,"id":id},"source":source,"node_index":null})))["data"].clone()}
 fn run(&mut self,scope:Value,scenario:Value,revision:Value,intent:Value)->Value{let h=Self::data(self.request(scope.clone(),"run_scenario",json!({"scenario_id":scenario["id"],"input_revision":revision,"observation_window":scenario["window"],"intent":intent})));self.finish();let state=Self::data(self.request(scope.clone(),"get_task",json!({"task_id":h["id"]})));assert_eq!(state["task"]["state"],"completed","{state:#}");Self::data(self.request(scope,"get_run",json!({"run_id":state["result"]["data"]["run_id"],"query":{"kind":"full_result"}})))["data"].clone()}
 fn snapshot(&mut self)->Value{let project=self.object(json!({"kind":"project"}),"project",&self.project.clone(),json!({"kind":"draft"}));let scenarios=project["scenario_refs"].as_array().unwrap().iter().map(|s|self.object(json!({"kind":"project"}),"scenario",s.as_str().unwrap(),json!({"kind":"draft"}))).collect::<Vec<_>>();json!({"project":project,"scenarios":scenarios})}
 fn reply(&mut self,query:Value)->Value{match query["kind"].as_str().unwrap(){
  "dispatch"=>{let before=self.formal.clone();let reply=self.raw(query["request"].clone());if !before.is_null(){let actual=Self::data(self.request(json!({"kind":"project"}),"get_run",json!({"run_id":before["id"],"query":{"kind":"full_result"}})))["data"].clone();assert_eq!(before,actual,"authoring never changes old formal Run");}reply},
  "await_task"=>{self.finish();self.request(query["scope"].clone(),"get_task",json!({"task_id":query["task_id"]}))},
  "new_project"=>{
   if !self.formal.is_null(){let before=self.formal.clone();let actual=Self::data(self.request(json!({"kind":"project"}),"get_run",json!({"run_id":before["id"],"query":{"kind":"full_result"}})))["data"].clone();assert_eq!(before,actual);}
   let commit=Self::data(self.request(json!({"kind":"workspace"}),"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"P303 actual switched project","time_reference":{"label":"explicit","origin_iso8601":null,"timezone":null},"departure":"discard_after_tasks_terminal"}}]})));
   assert_eq!(commit["scope"]["kind"],"project");self.project=commit["scope"]["data"].as_str().unwrap().to_owned();self.formal=Value::Null;
   let context=Self::data(self.request(json!({"kind":"project"}),"get_context",json!({"selection":[]})));json!({"snapshot":self.snapshot(),"context":context})
  },
  "snapshot"=>self.snapshot(),
  "canonical_input"=>{
   let project:sf_domain::ProjectState=serde_json::from_value(query["input"]["project"].clone()).unwrap();
   let scenario:sf_domain::ScenarioInput=serde_json::from_value(query["input"]["scenario"].clone()).unwrap();
   let definitions:Vec<sf_domain::ModuleDefinition>=serde_json::from_value(query["input"]["definitions"].clone()).unwrap();
   json!({"project":project,"scenario":scenario,"definitions":definitions})
  },
  "open_saved"=>{
   let directory=PathBuf::from(query["directory"].as_str().unwrap());
   let selected=self.host.select_directory(&serde_json::from_value(json!("P303-ui")).unwrap(),serde_json::from_value(json!("import_package")).unwrap(),directory).unwrap();
   let task=Self::data(self.request(json!({"kind":"workspace"}),"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));self.finish();
   let state=Self::data(self.request(json!({"kind":"workspace"}),"get_task",json!({"task_id":task["id"]})));assert_eq!(state["task"]["state"],"completed","saved import: {state:#}");let imported=&state["result"]["data"];
   let commit=Self::data(self.request(json!({"kind":"workspace"}),"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":imported["proposed_objects"],"module_versions":imported["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]})));
   self.project=commit["scope"]["data"].as_str().unwrap().to_owned();self.formal=Value::Null;
   let context=Self::data(self.request(json!({"kind":"project"}),"get_context",json!({"selection":[]})));json!({"project_id":self.project,"context":context})
  },
  "select_save"=>{let base=std::env::current_exe().unwrap().parent().unwrap().join(format!("saved-{}",std::process::id()));fs::create_dir_all(&base).unwrap();let selected=self.host.select_directory(&serde_json::from_value(json!("P303-ui")).unwrap(),serde_json::from_value(json!("save_project")).unwrap(),base).unwrap();json!(selected)},
  "move_fresh"=>{let parent=std::env::current_exe().unwrap().parent().unwrap().to_path_buf();let from=parent.join(format!("saved-{}",std::process::id()));let to=parent.join(format!("moved-{}",std::process::id()));assert!(from.starts_with(&parent)&&to.starts_with(&parent));fs::rename(&from,&to).unwrap();self.host=ProjectHost::new(serde_json::from_value(json!(format!("layout-fresh-{}",std::process::id()))).unwrap(),serde_json::from_value(json!("P303-ui")).unwrap());self.formal=Value::Null;let selected=self.host.select_directory(&serde_json::from_value(json!("P303-ui")).unwrap(),serde_json::from_value(json!("import_package")).unwrap(),to.clone()).unwrap();json!({"selected":selected,"from":from,"to":to})},
  "register"=>{self.host.register_bundled_module_directory(&PathBuf::from(query["directory"].as_str().unwrap())).unwrap();json!({"ok":true})},

  "port_output"=>{let result=self.host.read_run_port_output(&serde_json::from_value(json!("P303-ui")).unwrap(),&serde_json::from_value(query["project_id"].clone()).unwrap(),&serde_json::from_value(query["scope"].clone()).unwrap(),&serde_json::from_value(query["scope_revision"].clone()).unwrap(),&serde_json::from_value(query["run_id"].clone()).unwrap(),&serde_json::from_value(query["at_ms"].clone()).unwrap(),&serde_json::from_value(query["port"].clone()).unwrap());match result{Ok(data)=>json!({"ok":true,"data":data}),Err(error)=>json!({"ok":false,"error":error})}},
  "media"=>{let bytes=self.host.read_media_asset(&serde_json::from_value(json!("P303-ui")).unwrap(),&serde_json::from_value(query["project_id"].clone()).unwrap(),&serde_json::from_value(query["scope"].clone()).unwrap(),&serde_json::from_value(query["scope_revision"].clone()).unwrap(),&serde_json::from_value(query["source"].clone()).unwrap(),&serde_json::from_value(query["asset"].clone()).unwrap());match bytes{Ok(data)=>json!({"ok":true,"bytes":data}),Err(error)=>json!({"ok":false,"error":error})}},
  _=>panic!("unknown test transport operation")
 }}
}
fn main(){
 let root=PathBuf::from(std::env::args().nth(1).unwrap());let fixture=std::env::args().nth(2).unwrap_or_else(||"scenarios/图片时间轴/runtime/fixed.input.json".into());let input:Value=serde_json::from_slice(&fs::read(root.join(&fixture)).unwrap()).unwrap();let directory=root.join(format!("scenarios/媒体表现/runtime-node-inspector/core-session-{}",std::process::id()));seed(&root,&directory,&input);
 let mut core=Core{host:ProjectHost::new(serde_json::from_value(json!(format!("P303-core-{}",std::process::id()))).unwrap(),serde_json::from_value(json!("P303-ui")).unwrap()),seq:0,project:input["project"]["id"].as_str().unwrap().to_owned(),draft:Value::Null,formal:Value::Null};
 let selected=core.host.select_directory(&serde_json::from_value(json!("P303-ui")).unwrap(),serde_json::from_value(json!("import_package")).unwrap(),directory).unwrap();let task=Core::data(core.request(json!({"kind":"workspace"}),"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null})));core.finish();let state=Core::data(core.request(json!({"kind":"workspace"}),"get_task",json!({"task_id":task["id"]})));assert_eq!(state["task"]["state"],"completed","import: {state:#}");let imported=&state["result"]["data"];
 Core::data(core.request(json!({"kind":"workspace"}),"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":imported["proposed_objects"],"module_versions":imported["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":["p303.align","p303.lower","p303.hold","p303.return"],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]})));

 let snapshot=core.snapshot();let formal=core.run(json!({"kind":"project"}),snapshot["scenarios"][0].clone(),snapshot["project"]["revision"].clone(),json!({"kind":"project_scenario"}));core.formal=formal.clone();
 let context=Core::data(core.request(json!({"kind":"project"}),"get_context",json!({"selection":[]})));
 let ready=json!({"ready":true,"snapshot":snapshot,"formal_run":formal,"context":context,"capabilities":Core::data(core.request(json!({"kind":"workspace"}),"describe_capabilities",json!({}))),"producer_cut":"item-layout-producer frozen + CapturePreview frozen"});println!("{ready}");io::stdout().flush().unwrap();
 for line in io::stdin().lock().lines(){let query:Value=serde_json::from_str(&line.unwrap()).unwrap();println!("{}",core.reply(query));io::stdout().flush().unwrap();}
}
