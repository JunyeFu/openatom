//! Actual shared startup/public Host/Engine consumer; no Tauri, browser, cloud or services.
use serde_json::{json, Value};
use sf_commands::*;
use sf_domain::*;
use std::{fs, path::Path, sync::atomic::{AtomicU64, Ordering}};
static REQUEST: AtomicU64 = AtomicU64::new(1);
fn read(path:&Path)->Value { serde_json::from_slice(&fs::read(path).unwrap()).unwrap() }
fn write(path:&Path,value:&Value){fs::create_dir_all(path.parent().unwrap()).unwrap();fs::write(path,serde_json::to_vec_pretty(value).unwrap()).unwrap();}
fn id(value:&str)->ObjectId {value.to_owned().try_into().unwrap()}
fn data(reply:Value)->Value {let response:Response<Value>=serde_json::from_value(reply.clone()).unwrap();match response.outcome{ApiOutcome::Ok(value)=>value,other=>panic!("Actual Host failed: {other:?}\n{reply:#}")}}
struct Host { host:ProjectHost, ui:ObjectId, project:Value }
impl Host {
 fn new()->Self {let ui=id(&format!("resource-ui-{}",std::process::id()));Self{host:ProjectHost::new(id(&format!("resource-host-{}",std::process::id())),ui.clone()),ui,project:Value::Null}}
 fn call(&mut self,operation:&str,params:Value)->Value {
  let request=serde_json::from_value(json!({"api_version":"0.1","request_id":format!("resource-{}",REQUEST.fetch_add(1,Ordering::Relaxed)),"project_id":self.project,"scope":{"kind":if self.project.is_null(){"workspace"}else{"project"}},"operation":operation,"params":params})).unwrap();
  data(self.host.dispatch(&self.ui,request))
 }
 fn finish(&mut self){let mut work=self.host.drain_io_work();assert_eq!(work.len(),1);let completed=work.pop().unwrap().run();self.host.finish_io(completed);}
 fn task(&mut self,handle:&Value)->Value{self.call("get_task",json!({"task_id":handle["id"]}))}
 fn open(&mut self,root:&Path){
  let selected=self.host.select_directory(&self.ui,FilePurpose::ImportPackage,root.to_owned()).unwrap();
  let handle=self.call("import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}));self.finish();let state=self.task(&handle);
  assert_eq!(state["task"]["state"],"completed","{state:#}");let draft=&state["result"]["data"];
  let commit=self.call("apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":handle["id"],"choices":{"chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}));self.project=commit["project_id"].clone();assert!(!self.project.is_null());
 }
 fn context(&mut self)->Value{self.call("get_context",json!({"selection":[]}))}
 fn object(&mut self,kind:&str,object:&str)->Value{self.call("get_object",json!({"object_id":{"kind":kind,"id":object},"source":{"kind":"draft"},"node_index":null}))["data"].clone()}
 fn current_project(&mut self)->Value{let project=self.project.as_str().unwrap().to_owned();self.object("project",&project)}
 fn save(&mut self,root:&Path){fs::create_dir(root).unwrap();let selected=self.host.select_directory(&self.ui,FilePurpose::SaveProject,root.to_owned()).unwrap();let revision=self.context()["project"]["revision"].clone();let handle=self.call("save_project",json!({"revision":revision,"selected_directory_ref":selected}));self.finish();let state=self.task(&handle);assert_eq!(state["task"]["state"],"completed","{state:#}");}
 fn full(&mut self,run:&Value)->Value{let value=self.call("get_run",json!({"run_id":run,"query":{"kind":"full_result"}}));assert_eq!(value["kind"],"full");value["data"].clone()}
}
fn build(root:&Path,output:&Path,expected:&Value){
 assert!(!output.exists());fs::create_dir_all(output).unwrap();let mut h=Host::new();
 h.host.register_bundled_library(&root.join("modules")).unwrap();
 let profile=read(&root.join("modules/startup-library.json"));let reference=profile["template_ref"].clone();assert_eq!(reference,expected["template_ref"]);
 let source=read(&root.join("modules/templates/paper-packaging-factory/candidate/example-project.json"))["data"].clone();let source_scenario=read(&root.join("modules/templates/paper-packaging-factory/candidate/source-scenario.json"))["data"].clone();
 let commit=h.call("apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"actual specialized factory","time_reference":{"label":"declared factory timeline","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}));h.project=commit["project_id"].clone();
 let revision=h.context()["project"]["revision"].clone();h.call("apply_commands",json!({"base_revision":revision,"commands":[{"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}]}));
 let revision=h.context()["project"]["revision"].clone();let mut params=json!({"object_id":{"kind":"module","id":reference["id"]},"node_index":null,"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":revision,"choices":null,"seed":null}}});
 let bare=h.call("get_object",params.clone());assert_eq!(bare["kind"],"template_preview","{bare:#}");
 params["source"]["data"]["choices"]=json!({"source_project":bare["data"]["source_project"],"namespace":"factory","resources":source["resources"].as_array().unwrap().iter().map(|r|(r["id"].as_str().unwrap().to_owned(),Value::Null)).collect::<serde_json::Map<_,_>>(),"reuse_items":{},"include_example_product":true,"site_envelope":null});
 params["source"]["data"]["seed"]=json!({"scenario_id":"FACTORY-PRODUCTION","supply":true,"demand":true,"initial_quality":source_scenario["initial_conditions"]["items"].as_array().unwrap().iter().map(|i|(i["id"].as_str().unwrap().to_owned(),i["quality"].clone())).collect::<serde_json::Map<_,_>>(),"adopt_declared_inspection_results":true});
 let preview=h.call("get_object",params);write(&output.join("actual-preview.json"),&preview);let command=preview["data"]["command"].clone();assert!(!command.is_null());
 let revision=h.context()["project"]["revision"].clone();h.call("apply_commands",json!({"base_revision":revision,"commands":[{"kind":"instantiate_template","data":command}]}));
 let project=h.current_project();assert_eq!(project["devices"].as_array().unwrap().len(),92);assert_eq!(project["setup_policies"].as_array().unwrap().len(),92);
 let scenario=h.object("scenario","FACTORY-PRODUCTION");assert_eq!(scenario["supplies"].as_array().unwrap().len() as u64,expected["supplies"].as_u64().unwrap());
 let revision=h.context()["project"]["revision"].clone();let handle=h.call("run_scenario",json!({"scenario_id":"FACTORY-PRODUCTION","input_revision":revision,"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}));h.finish();let state=h.task(&handle);assert_eq!(state["task"]["state"],"completed","{state:#}");
 let run=h.full(&state["result"]["data"]["run_id"]);let typed:RunResult=serde_json::from_value(run.clone()).unwrap();
 let jobs=typed.jobs.iter().filter(|job|matches!(job.execution_ref,JobExecutionRef::ProjectDevice{..})).count();assert_eq!(jobs as u64,expected["manufacturing_jobs"].as_u64().unwrap());assert_eq!(typed.transport.len() as u64,expected["transport_trips"].as_u64().unwrap());assert!(typed.jobs.iter().all(|j|matches!(j.state,JobStatus::Completed)));assert!(typed.transport.iter().all(|t|matches!(t.phase,TransportPhase::Returned)));
 let orders=expected["accepted_orders"].as_object().unwrap();
 for (identity,count) in orders {let order=typed.orders.iter().find(|o|o.order_id.as_str().ends_with(identity)).unwrap();assert_eq!(order.accepted.value.coefficient.value(),count.as_u64().unwrap());assert_eq!(order.outstanding.value.coefficient.value(),0);}
 assert_eq!(typed.orders.len(),orders.len());write(&output.join("actual-project.json"),&project);write(&output.join("actual-scenario.json"),&scenario);write(&output.join("actual-run.json"),&run);h.save(&output.join("saved-project"));
 write(&output.join("receipt.json"),&json!({"status":"actual_shared_startup_public_template_Host_Engine_saved","pid":std::process::id(),"template_ref":reference,"manufacturing":jobs,"transport":typed.transport.len(),"accepted":orders,"accepted_total":typed.orders.iter().map(|o|o.accepted.value.coefficient.value()).sum::<u64>(),"connected_devices":expected["connected_devices"],"independent_normal_cells":expected["independent_cells"],"Native_verified":false}));
}
fn reopen(resource:&Path,saved:&Path,expected:&Path,output:&Path){
 let mut h=Host::new();h.host.register_bundled_library(&resource.join("modules")).unwrap();h.open(saved);
 assert_eq!(h.current_project(),read(&expected.join("actual-project.json")));let run=read(&expected.join("actual-run.json"));assert_eq!(h.full(&run["id"]),run);
 let first=read(&expected.join("receipt.json"));assert_ne!(first["pid"].as_u64().unwrap(),u64::from(std::process::id()));
 write(output,&json!({"status":"new_OS_process_reopen_exact","pid":std::process::id(),"builder_pid":first["pid"],"project_and_Run_exact":true,"Native_verified":false}));
}
fn legacy(source:&Path,output:&Path){
 assert!(!output.exists());fs::create_dir_all(output).unwrap();let manifest=read(&source.join("project.json"))["data"].clone();let expected=read_project_archive(source).unwrap();let mut h=Host::new();h.open(source);assert_eq!(h.current_project(),serde_json::to_value(&expected.project).unwrap());
 let runs=manifest["documents"].as_array().unwrap().iter().filter(|d|d["role"]=="run_result").map(|d|read(&source.join(d["file_ref"]["path"].as_str().unwrap()))["data"].clone()).collect::<Vec<_>>();for run in &runs{assert_eq!(h.full(&run["id"]),*run);}let project=h.current_project();write(&output.join("actual-project.json"),&project);write(&output.join("actual-runs.json"),&json!(runs));h.save(&output.join("saved-project"));write(&output.join("receipt.json"),&json!({"pid":std::process::id(),"source":"original self-contained R1 archive","run_count":runs.len(),"source_saved_or_modified":false,"new_versions_substituted":false}));
}
fn legacy_reopen(expected:&Path,original:&Path,output:&Path){
 let mut h=Host::new();h.open(&expected.join("saved-project"));assert_eq!(h.current_project(),read(&expected.join("actual-project.json")));
 let manifest=read(&original.join("project.json"))["data"].clone();let mut frozen_checks=0;
 for run in read(&expected.join("actual-runs.json")).as_array().unwrap(){
  assert_eq!(h.full(&run["id"]),*run);
  let doc=manifest["documents"].as_array().unwrap().iter().find(|d|d["role"]=="run_result" && d["object_ref"]["id"]==run["id"]).unwrap();
  let directory=original.join(doc["file_ref"]["path"].as_str().unwrap()).parent().unwrap().to_owned();
  for (kind,file) in [("project","project.json"),("scenario","scenario.json")]{
   let expected=read(&directory.join(file))["data"].clone();
   let value=h.call("get_object",json!({"object_id":{"kind":kind,"id":expected["id"]},"source":{"kind":"run_snapshot","data":{"run_id":run["id"],"at_ms":0}},"node_index":null}));
   assert_eq!(value["data"],expected);frozen_checks+=1;
  }
 }
 let old=read(&expected.join("receipt.json"));assert_ne!(old["pid"].as_u64().unwrap(),u64::from(std::process::id()));
 write(output,&json!({"status":"old_R1_new_OS_process_reopen_exact","pid":std::process::id(),"prior_pid":old["pid"],"Run_count":old["run_count"],"frozen_Project_Scenario_checks":frozen_checks,"original_modules_and_Run_retained":true}));
}
fn negative(root:&Path,output:&Path){let mut h=Host::new();let failure=h.host.register_bundled_library(&root.join("modules")).unwrap_err();assert!(failure.contains("Required startup"),"{failure}");assert!(!failure.contains("Bundled template purchased"));write(output,&json!({"required_missing_failed":true,"fallback_to_83_legacy_directories":false,"actual_production_Host_error":failure,"pid":std::process::id()}));}
fn main(){let args=std::env::args().collect::<Vec<_>>();match args[1].as_str(){
 "build"=>{let expected=if let Some(path)=args.get(4){read(Path::new(path))}else{json!({"template_ref":{"id":"community.template-paper-packaging-factory","version":"1.3.0"},"supplies":26,"manufacturing_jobs":145,"transport_trips":78,"accepted_orders":{"PF-ORDER-CASE":3,"PF-ORDER-SLEEVE":3,"PF-ORDER-TUBE":6,"PF-ORDER-LABEL":18,"PF-ORDER-BAG":3},"connected_devices":19,"independent_cells":73})};build(Path::new(&args[2]),Path::new(&args[3]),&expected)},
 "reopen"=>reopen(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4]),Path::new(&args[5])),
 "legacy"=>legacy(Path::new(&args[2]),Path::new(&args[3])),
 "legacy-reopen"=>legacy_reopen(Path::new(&args[2]),Path::new(&args[3]),Path::new(&args[4])),
 "missing"=>negative(Path::new(&args[2]),Path::new(&args[3])),
 _=>panic!("build/reopen/legacy/legacy-reopen/missing required")
}}
