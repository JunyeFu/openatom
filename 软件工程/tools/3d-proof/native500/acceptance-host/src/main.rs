#[path="../../../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use support::*;
use serde_json::{Value,json};
use sf_commands::*;
use sf_domain::*;
use std::path::PathBuf;
fn main(){
 let root=PathBuf::from(std::env::args_os().nth(1).expect("root"));
 let input=PathBuf::from(std::env::args_os().nth(2).expect("input JSON"));
 let out=PathBuf::from(std::env::args_os().nth(3).expect("owned output directory"));std::fs::create_dir_all(&out).unwrap();
 let data=read(&input);let mut h=Harness::from_value(data.clone());
 let review=root.join(std::env::args_os().nth(4).unwrap_or_else(|| "modules/media/native-equipment-500/reviews/B02-Q01".into()));
 let regs=read(&review.join("registration.json"));
 let version=regs["register_order"][0]["version"].as_str().unwrap();
 for e in regs["register_order"].as_array().unwrap(){h.host.register_bundled_module_directory(&review.join(e["directory"].as_str().unwrap())).unwrap();}
 let item_regs=read(&review.join("items/registration.json"));
 for e in item_regs["register_order"].as_array().unwrap(){h.host.register_bundled_module_directory(&root.join(e["directory"].as_str().unwrap())).unwrap();}
 let mut commands=vec![];
 for e in regs["register_order"].as_array().unwrap().iter().filter(|e|e["id"].as_str().unwrap().contains(".device.")){
  let key=e["id"].as_str().unwrap().split_once(".device.").unwrap().1;
  let targets:Vec<_>=data["project"]["devices"].as_array().unwrap().iter().filter(|d|d["module_ref"]["id"]==format!("qa.seed.{key}")).map(|d|json!({"kind":"device","id":d["id"]})).collect();
  commands.push(json!({"kind":"adopt_module","data":{"module_ref":{"id":e["id"],"version":e["version"]},"targets":targets,"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}));
 }
 for e in item_regs["register_order"].as_array().unwrap().iter().filter(|e|e["category"]=="items"){
  commands.push(json!({"kind":"adopt_module","data":{"module_ref":{"id":e["id"],"version":e["version"]},"targets":[{"kind":"item_type","id":e["item_type"]}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}));
 }
 // Adopt first, then retain explicit instance bindings and frozen item layouts.
 let commit:EditCommit=ok(h.apply(1,json!(commands)));
 fn retarget(v:&mut Value,version:&str,regs:&Value){if let Some(id)=v.get("id").and_then(Value::as_str){if let Some(key)=id.strip_prefix("qa.seed."){v["id"]=regs["register_order"].as_array().unwrap().iter().find(|e|e["id"].as_str().unwrap().ends_with(&format!(".device.{key}"))).unwrap()["id"].clone();v["version"]=json!(version);}}if let Some(map)=v.as_object_mut(){for child in map.values_mut(){retarget(child,version,regs);}}else if let Some(rows)=v.as_array_mut(){for child in rows{retarget(child,version,regs);}}}
 let jogger_path=review.join("devices/paper-jogger/module.json");let jogger=jogger_path.is_file().then(||read(&jogger_path));
 let mut restores=vec![];for d in data["project"]["devices"].as_array().unwrap(){let mut d=d.clone();retarget(&mut d,version,&regs);if d["module_ref"]["id"]=="qa.output-drainer" {if let Some(pkg)=&jogger {d["presentation_override"]=pkg["data"]["definition"]["payload"]["data"]["presentation"].clone();}}restores.push(put("device",d));}
 for d in data["project"]["routes"].as_array().unwrap(){let mut d=d.clone();retarget(&mut d,version,&regs);restores.push(put("route",d));}
 for d in data["project"]["connections"].as_array().unwrap(){let mut d=d.clone();retarget(&mut d,version,&regs);restores.push(put("connection",d));}
 // Quality presets and declared decisions must follow the same adopted exact module.
 let mut scenario=h.object("scenario",data["scenario"]["id"].as_str().unwrap());scenario["project_revision"]=json!(commit.new_revision.0.value());retarget(&mut scenario,version,&regs);restores.push(put("scenario",scenario));
 for t in data["project"]["item_types"].as_array().unwrap(){let mut t=t.clone();if let Some(e)=item_regs["register_order"].as_array().unwrap().iter().find(|e|e["category"]=="items"&&e["item_type"]==t["id"]){t["module_ref"]=json!({"id":e["id"],"version":e["version"]});}restores.push(put("item_type",t));}
 let _:EditCommit=ok(h.apply(commit.new_revision.0.value(),json!(restores)));
 let p=h.object("project","P1");let mut runs=vec![];
 for s in p["scenario_refs"].as_array().unwrap(){let task=h.run(s.as_str().unwrap());h.finish_one();let run=h.completed_run(&task);let value=serde_json::to_value(&run).unwrap();
  write(&out.join(format!("{}.run.json",s.as_str().unwrap())),&value);runs.push(value);}
 let revision=h.context().project.unwrap().revision.0.value();let saved=out.join("saved");std::fs::create_dir(&saved).unwrap();let task=h.save(revision,&saved);h.finish_one();assert!(matches!(h.task(&task,true).task.state,ComputationStatus::Completed));
 let project=h.object("project","P1");
 // A fresh Host with no local library registrations reopens the portable exact assets/Runs.
 let mut reopened=Harness::from_seed(Sandbox::new(),&saved);assert_eq!(reopened.object("project","P1"),project);
 for value in &runs{let id:RunId=serde_json::from_value(value["id"].clone()).unwrap();assert_eq!(serde_json::to_value(reopened.full(&id)).unwrap(),*value);}
 write(&out.join("host-receipt.json"),&json!({"passed":true,"project":project,"runs":runs.iter().map(|r|json!({"id":r["id"],"input_ref":r["input_ref"],"jobs":r["jobs"].as_array().unwrap().len()})).collect::<Vec<_>>(),"fresh_Host_save_reopen":true,"Native":false,"saved":saved}));
 println!("real Host runs, portable media and fresh Host reopen passed: {}",out.display());
}
