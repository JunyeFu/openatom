//! Actual existing Host import/run/save probe. No renderer or performance claim.
use serde_json::{Value,json};
use sf_commands::ProjectHost;
use std::{fs,path::{Path,PathBuf}};

struct Probe { host:ProjectHost, project:String, seq:u64 }
impl Probe {
 fn new()->Self { Self {host:ProjectHost::new(serde_json::from_value(json!(format!("B60-surface-{}",std::process::id()))).unwrap(),serde_json::from_value(json!("B60-owner")).unwrap()),project:String::new(),seq:0} }
 fn call(&mut self,workspace:bool,operation:&str,params:Value)->Value {
  self.seq+=1;
  let value=self.host.dispatch(&serde_json::from_value(json!("B60-owner")).unwrap(),serde_json::from_value(json!({"api_version":"0.1","request_id":format!("surface-{}",self.seq),"project_id":if workspace {Value::Null}else{json!(self.project)},"scope":{"kind":if workspace {"workspace"}else{"project"}},"operation":operation,"params":params})).unwrap());
  assert_eq!(value["outcome"]["status"],"ok","{operation}: {value:#}");value["outcome"]["data"].clone()
 }
 fn finish(&mut self,workspace:bool,task:&Value)->Value {
  for work in self.host.drain_io_work(){self.host.finish_io(work.run());}
  let result=self.call(workspace,"get_task",json!({"task_id":task["id"]}));
  assert_eq!(result["task"]["state"],"completed","{result:#}");result["result"]["data"].clone()
 }
 fn open(&mut self,path:&Path) {
  let selected=self.host.select_directory(&serde_json::from_value(json!("B60-owner")).unwrap(),serde_json::from_value(json!("import_package")).unwrap(),path.to_path_buf()).unwrap();
  let task=self.call(true,"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}));
  let imported=self.finish(true,&task);
  self.call(true,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":imported["proposed_objects"],"module_versions":imported["proposed_definitions"],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":["p201.b60.station.surface.1.0.4"],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}));
  self.project=sf_commands::read_project_archive(path).unwrap().project.id.as_str().to_owned();
 }
}
fn main(){
 let root=PathBuf::from(std::env::args().nth(1).expect("input root"));
 let variant=std::env::args().nth(2).expect("variant");
 let source=root.join(&variant);
 let archive=sf_commands::read_project_archive(&source).unwrap();
 let scenario=serde_json::to_value(archive.scenarios.values().next().unwrap()).unwrap();
 let mut probe=Probe::new();probe.open(&source);
 let asset=serde_json::from_value(json!({"id":"p201.b60.station.surface.1.0.4","path":"assets/b60-station-1.0.4.glb","kind":"glb"})).unwrap();
 let bytes=probe.host.read_media_asset(&serde_json::from_value(json!("B60-owner")).unwrap(),&serde_json::from_value(json!(probe.project)).unwrap(),&serde_json::from_value(json!({"kind":"project"})).unwrap(),&serde_json::from_value(json!(5)).unwrap(),&serde_json::from_value(json!({"kind":"module_version","data":{"module_ref":{"id":"p201.b60.station","version":"1.0.4"}}})).unwrap(),&asset).unwrap();
 assert_eq!(bytes,archive.files["assets/b60-station-1.0.4.glb"]);
 let task=probe.call(false,"run_scenario",json!({"scenario_id":scenario["id"],"input_revision":5,"observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}));
 let result=probe.finish(false,&task);
 let run=probe.call(false,"get_run",json!({"run_id":result["run_id"],"query":{"kind":"full_result"}}))["data"].clone();
 assert_eq!(run["computed_until_ms"],1_200_000);
 assert_eq!(run["acceptance_events"].as_array().unwrap().len(),3000);
 assert_eq!(run["jobs"].as_array().unwrap().len(),30000);
 assert_eq!(run["events"].as_array().unwrap().len(),155994);
 let accepted600=run["acceptance_events"].as_array().unwrap().iter().filter(|event|event["at_ms"].as_u64().unwrap()<=600000).count();
 assert_eq!(accepted600,1746);
 for job in run["jobs"].as_array().unwrap(){assert_eq!(job["operation_ref"]["version"],"1.0.4");assert_eq!(job["finished_at_ms"].as_u64().unwrap()-job["started_at_ms"].as_u64().unwrap(),2000);}
 let saved=root.join(format!("host-saved-{variant}"));fs::create_dir(&saved).unwrap();
 let selected=probe.host.select_directory(&serde_json::from_value(json!("B60-owner")).unwrap(),serde_json::from_value(json!("save_project")).unwrap(),saved.clone()).unwrap();
 let task=probe.call(false,"save_project",json!({"revision":5,"selected_directory_ref":selected}));probe.finish(false,&task);
 fs::write(root.join(format!("host-{variant}-result.json")),serde_json::to_vec_pretty(&json!({"result":"passed","actual_production_Host":true,"project_id":probe.project,"project_revision":5,"run_id":run["id"],"asset_bytes_read":bytes.len(),"computed_until_ms":run["computed_until_ms"],"jobs":30000,"events":155994,"accepted600":1746,"accepted1200":3000,"saved_directory":saved,"Native_acceptance":false,"MQ_acceptance":false,"formal_NFR_acceptance":false,"benchmark_ready":false})).unwrap()).unwrap();
 println!("{}",json!({"result":"passed","variant":variant,"run_id":run["id"],"saved_directory":saved,"Native_acceptance":false}));
}
