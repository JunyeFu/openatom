//! Real production Host runs, explicit phase expectations and new-process project readback.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod host;
#[path = "../../../packages/commands/tests/fixtures/whole_ix/checks.rs"]
mod original_ix;
use host::*;
use serde_json::{json, Value};
use sf_commands::*;
use sf_domain::*;
use std::{fs,path::{Path,PathBuf},process::Command};
fn inputs()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("inputs/native-phases")}
fn output()->PathBuf {PathBuf::from(std::env::var_os("SF_P103_DELIVERY").expect("explicit private output"))}
fn history(h:&mut Harness,r:&RunResult,moments:&[u64])->Value {
 let mut rows=vec![];
 for t in moments {
  let mut objects=vec![];
  for selector in [json!([]),json!(r.jobs.iter().map(|j|json!({"kind":"job","id":j.id})).collect::<Vec<_>>())] {
   let v:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"objects_at","data":{"at_ms":t,"objects":selector,"page":{"after_index":null,"limit":1000}}}})));
   let RunView::Objects(page)=v else {panic!("objects")};assert!(page.next_index.is_none());objects.extend(page.objects);
  }
  rows.push(json!({"at_ms":t,"objects":objects}));
 }
 json!(rows)
}
fn events(h:&mut Harness,r:&RunResult)->Value {
 let mut rows=vec![];let mut cursor=None::<SafeUint>;
 loop {
  let v:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"events","data":{"interval":null,"page":{"after_index":cursor,"limit":100}}}})));
  let RunView::Events(p)=v else {panic!("events")};rows.extend(p.events);cursor=p.next_index;if cursor.is_none(){break}
 }
 assert_eq!(json!(rows),json!(r.events));json!(rows)
}
fn verify(c:&Value,r:&RunResult,hist:&Value) {
 let v=json!(r);assert!(r.whole_window_computed);assert_eq!(v["evidence_origin"],"actual_software");
 assert_eq!(v["computation_status"],"completed");
 if c["logistics"]==true {
  original_ix::ix01(&v,c["expectation"].as_str().unwrap());
  // Delayed transport keeps the same actual cargo while waiting for acceptance.
  if c["expectation"]=="delayed_pcb" {
   let trip=v["transport"].as_array().unwrap().iter().max_by_key(|x|x["departed_ms"].as_u64()).unwrap();
   let cargo=&trip["cargo"][0];
   for (at,kind) in [(419999,"transport"),(420000,"location")] {
    let row=hist.as_array().unwrap().iter().find(|x|x["at_ms"]==at).unwrap();
    let item=row["objects"].as_array().unwrap().iter().find(|x|x["kind"]=="item" && x["data"]["id"]==*cargo).unwrap();
    assert_eq!(item["data"]["holder"]["kind"],kind);
    if at==420000 {assert_eq!(item["data"]["holder"]["id"],"shell-buffer");}
   }
  }
  return
 }
 let job=v["jobs"].as_array().unwrap().iter().find(|j|j["execution_ref"]["data"]["device_id"]=="CNC").unwrap();
 let jid=&job["id"];
 let work:Vec<Value>=v["phase_work_intervals"].as_array().unwrap().iter().filter(|x|x["job_id"]==*jid).map(|x|json!([x["phase_id"],x["interval"]["start_ms"],x["interval"]["end_ms"]])).collect();
 assert_eq!(json!(work),c["expected_work"],"{} phase work",c["name"]);
 assert_eq!(job["finished_at_ms"],c["finished_at_ms"]);
 let claims:Vec<&Value>=v["resource_claims"].as_array().unwrap().iter().filter(|x|x["job_id"]==*jid).collect();
 let spans:Vec<Value>=claims.iter().filter(|x|x["resource_id"]=="CAP-CNC").flat_map(|x|x["intervals"].as_array().unwrap()).map(|x|json!([x["start_ms"],x["end_ms"]])).collect();
 assert_eq!(json!(spans),c["machine_intervals"]);
 let worker:u64=claims.iter().filter(|x|x["resource_id"]=="W-C").flat_map(|x|x["intervals"].as_array().unwrap()).map(|x|x["end_ms"].as_u64().unwrap()-x["start_ms"].as_u64().unwrap()).sum();
 assert_eq!(worker,c["worker_ms"].as_u64().unwrap());
 for expect in c["waits"].as_array().unwrap() {
  // Conditions can be split at a boundary while their total wait remains contiguous.
  let waits:Vec<&Value>=v["waits"].as_array().unwrap().iter().filter(|w|w["object_ref"]["id"]==*jid && w["phase_ref"]==expect[3] && w["interval"]["start_ms"].as_u64().unwrap()>=expect[0].as_u64().unwrap() && w["interval"]["end_ms"].as_u64().unwrap()<=expect[1].as_u64().unwrap()).collect();
  assert!(!waits.is_empty(),"missing wait {expect}");let mut end=expect[0].as_u64().unwrap();
  for w in waits {assert_eq!(w["interval"]["start_ms"],end);end=w["interval"]["end_ms"].as_u64().unwrap();assert!(w["reasons"].as_array().unwrap().iter().any(|x|x["kind"]==expect[2]),"reason {w}");}
  assert_eq!(end,expect[1].as_u64().unwrap());
 }
 for claim in &claims {assert_eq!(claim["intervals"].as_array().unwrap().len(),claim["interval_end_causes"].as_array().unwrap().len());}
 if c["name"]=="pause-interphase-output" {
  assert_eq!(job["state"],"waiting_output");
  let cap=claims.iter().find(|x|x["resource_id"]=="CAP-CNC").unwrap();assert_eq!(cap["interval_end_causes"],json!(["observation_end"]));
  assert!(v["final_items"].as_array().unwrap().iter().any(|i|i["item_type_ref"]["id"]=="part" && i["holder"]["id"]==*jid));
 } else {assert_eq!(job["state"],"completed");}
 if c["name"].as_str().unwrap().starts_with("pause") || c["name"]=="nonpreemptive-control" {
  assert_eq!(v["events"].as_array().unwrap().iter().filter(|e|e["payload"]["kind"]=="arrival_accepted" && e["payload"]["data"]=="phase-arrival").count(),1);
  for at in [9999,10000] {
   let point=hist.as_array().unwrap().iter().find(|x|x["at_ms"]==at).unwrap();
   let count=point["objects"].as_array().unwrap().iter().filter(|x|x["kind"]=="item" && x["data"]["id"]=="phase-future-raw").count();assert_eq!(count,usize::from(at==10000));
  }
 }
 if c["name"]=="pause-aux-release" {
  let auto:Vec<&Value>=v["phase_work_intervals"].as_array().unwrap().iter().filter(|w|w["job_id"]==*jid && w["phase_id"]=="auto").collect();
  assert_eq!(auto[0]["end_cause"],"interrupted");assert_eq!(auto[0]["effective_progress_end_ms"],5000);assert_eq!(auto[1]["effective_progress_start_ms"],5000);
  let aux=claims.iter().find(|x|x["resource_id"]=="W-C" && x["phase_ref"]=="auto").unwrap();
  assert_eq!(aux["intervals"],json!([{"start_ms":15000,"end_ms":20000},{"start_ms":40000,"end_ms":65000}]));
  assert_eq!(aux["interval_end_causes"],json!(["released","released"]));
  for at in [20000,30000,39999] {
   let point=hist.as_array().unwrap().iter().find(|x|x["at_ms"]==at).unwrap();
   let j=&point["objects"].as_array().unwrap().iter().find(|x|x["kind"]=="job" && x["data"]["id"]==*jid).unwrap()["data"];
   assert!(j["job_resources"].as_array().unwrap().contains(&json!("CAP-CNC")));
   assert!(!j["phase_resources"].as_array().unwrap().contains(&json!("W-C")));
   assert_eq!(j["effective_progress_ms"],5000);
   assert!(point["objects"].as_array().unwrap().iter().any(|x|x["kind"]=="item" && x["data"]["holder"]["id"]==*jid && x["data"]["lifecycle"]=="in_process"));
  }
 }
 assert!(r.acceptance_events.is_empty(),"local model must not invent order acceptance");
}
#[test]
#[ignore = "requires explicit fresh SF_P103_DELIVERY directory; see NATIVE_PROJECTS.md"]
fn phase_projects_are_actual_host_runs_saved_and_reopened_in_new_os_processes() {
 let all=read(&inputs().join("expectations.json"));fs::create_dir_all(output().join("projects")).unwrap();let mut results=vec![];
 for c in all["cases"].as_array().unwrap() {
  let name=c["name"].as_str().unwrap();let dir=output().join("evidence").join(name);fs::create_dir_all(&dir).unwrap();
  let input=read(&inputs().join(c["input"].as_str().unwrap()));let mut h=Harness::from_value(input.clone());
  let task=h.run(input["scenario"]["id"].as_str().unwrap());h.finish_one();let r=h.completed_run(&task);
  let moments:Vec<u64>=serde_json::from_value(c["moments"].clone()).unwrap();let hist=history(&mut h,&r,&moments);let ev=events(&mut h,&r);
  write(&dir.join("actual-before-assertions.json"),&json!({"run":r,"history":hist,"expectation":c}));verify(c,&r,&hist);
  let project=output().join("projects").join(name);fs::create_dir(&project).unwrap();let rev=h.context().project.unwrap().revision.0.value();let task=h.save(rev,&project);h.finish_one();assert!(matches!(h.task(&task,true).task.state,ComputationStatus::Completed));
  let expected_project=h.object("project","P1");let expected_scenario=h.object("scenario",input["scenario"]["id"].as_str().unwrap());
  let packet=json!({"case":c,"run":r,"history":hist,"events":ev,"input":input,"project_directory":project,"project":expected_project,"scenario":expected_scenario,"parent_pid":std::process::id()});write(&dir.join("packet.json"),&packet);drop(h);
  let mut child=Command::new(std::env::current_exe().unwrap());child.args(["--ignored","--exact","phase_project_new_process_readback","--nocapture","--test-threads=1"]).env("SF_P103_PACKET",dir.join("packet.json"));
  #[cfg(windows)] {use std::os::windows::process::CommandExt;child.creation_flags(0x08000000);}
  let child=child.output().unwrap();fs::write(dir.join("child.stdout.log"),&child.stdout).unwrap();fs::write(dir.join("child.stderr.log"),&child.stderr).unwrap();assert!(child.status.success(),"{name}: {}",String::from_utf8_lossy(&child.stderr));results.push(read(&dir.join("readback.json")));
 }
 write(&output().join("host-result.json"),&json!({"passed":true,"cases":results,"fresh_OS_processes":results.len(),"ordinary_Native":false,"whole_task_done":false}));
}
#[test]
#[ignore="child OS entry invoked by parent for every case"]
fn phase_project_new_process_readback(){
 let p=PathBuf::from(std::env::var_os("SF_P103_PACKET").unwrap());let packet=read(&p);let c=&packet["case"];let r:RunResult=serde_json::from_value(packet["run"].clone()).unwrap();
 assert_ne!(packet["parent_pid"],std::process::id());let mut h=Harness::from_seed(Sandbox::new(),Path::new(packet["project_directory"].as_str().unwrap()));
 assert_eq!(h.object("project","P1"),packet["project"]);assert_eq!(h.object("scenario",packet["scenario"]["id"].as_str().unwrap()),packet["scenario"]);assert_eq!(json!(h.full(&r.id)),packet["run"]);
 let moments:Vec<u64>=serde_json::from_value(c["moments"].clone()).unwrap();let hist=history(&mut h,&r,&moments);assert_eq!(hist,packet["history"]);assert_eq!(events(&mut h,&r),packet["events"]);verify(c,&r,&hist);
 write(&p.parent().unwrap().join("readback.json"),&json!({"passed":true,"case":c["name"],"project_directory":packet["project_directory"],"parent_pid":packet["parent_pid"],"child_pid":std::process::id(),"run_id":r.id,"events":r.events.len(),"jobs":r.jobs.len(),"history_points":moments.len(),"old_Run_recomputed":false,"exact_saved_project_scenario_Run_events_history_equal":true}));
}

