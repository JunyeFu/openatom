//! Actual frozen Host/Engine producer for the non-GUI timeline consumers.
#[path="fixtures/production/host_support.rs"] mod support;
#[path="fixtures/whole_ix/checks.rs"] mod checks;
use support::*;
use serde_json::{Value,json};
use sf_commands::*;
use sf_domain::*;
use std::{fs,path::{Path,PathBuf},process::Command};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
fn out()->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/delivery/host")}
fn source(name:&str)->Value {serde_json::from_str(match name {
    "baseline"=>include_str!("fixtures/production/p202-ix01-baseline.json"),
    "delayed-pcb"=>include_str!("fixtures/production/p202-ix01-delayed-pcb.json"),
    "phase-calendar"=>include_str!("fixtures/production/p1-phase-calendar.json"),_=>unreachable!()}).unwrap()}
fn frozen(h:&mut Harness,r:&RunResult,kind:&str,id:&str)->Value {
    let v:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":kind,"id":id},
        "source":{"kind":"run_snapshot","data":{"run_id":r.id,"at_ms":0}},"node_index":null})));
    serde_json::to_value(v).unwrap()["data"].clone()
}
fn objects(h:&mut Harness,r:&RunResult,at:u64,limit:u64)->Value {
    let mut after=Value::Null;let mut objects=Vec::new();
    loop {
        let view:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"objects_at","data":{
            "at_ms":at,"objects":[],"page":{"after_index":after,"limit":limit}}}})));
        let RunView::Objects(page)=view else {panic!("ObjectsAt")};
        assert_eq!(page.run_id,r.id);assert_eq!(page.at_ms.0.value(),at);
        objects.extend(page.objects);match page.next_index {Some(next)=>after=json!(next),None=>break}
    }
    json!({"run_id":r.id,"at_ms":at,"objects":objects,"next_index":null})
}
fn event_pages(h:&mut Harness,r:&RunResult)->Value {
    let mut after=Value::Null;let mut events=Vec::new();
    loop {
        let view:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"events","data":{
            "interval":null,"page":{"after_index":after,"limit":7}}}})));
        let RunView::Events(page)=view else {panic!("Events")};
        events.extend(page.events);match page.next_index {Some(next)=>after=json!(next),None=>break}
    }
    assert_eq!(json!(events),json!(r.events));json!(events)
}
fn execute(h:&mut Harness,name:&str)->RunResult {
    let task=h.run(name);assert!(h.task(&task,true).result.is_none());
    h.finish_one();h.completed_run(&task)
}
fn record(h:&mut Harness,r:&RunResult,name:&str)->Value {
    let project=frozen(h,r,"project","P1");let scenario=frozen(h,r,"scenario",source(name)["scenario"]["id"].as_str().unwrap());
    let snapshot=json!(r);let draft=h.object("project","P1");
    let mut times=std::collections::BTreeSet::from([0,70000,369999,370000,419999,420000,425000,440000,570000,600000]);
    for e in &r.events {let at=e.at_ms.0.value();times.insert(at);if at>0 {times.insert(at-1);}if at<600000 {times.insert(at+1);}}
    let mut frames=Vec::new();
    // Forward, backward and repeated reads must all reproduce the same history.
    for at in times {let frame=objects(h,r,at,3);assert_eq!(frame,objects(h,r,at,10000));frames.push(frame);}
    for frame in frames.iter().rev() {assert_eq!(*frame,objects(h,r,frame["at_ms"].as_u64().unwrap(),5));}
    event_pages(h,r);assert_eq!(h.object("project","P1"),draft);assert_eq!(json!(h.full(&r.id)),snapshot);
    let value=json!({"case":name,"project":project,"scenario":scenario,"definitions":source(name)["definitions"],"run":r,"observations":frames});
    write(&out().join(format!("{name}.actual.json")),&value);value
}
#[test]
fn timeline_two_runs_complete_history_and_new_process_reopen() {
    fs::create_dir_all(out()).unwrap();
    let original=source("baseline");let mut h=Harness::from_value(original.clone());
    let base=execute(&mut h,"IX01-baseline");checks::ix01(&json!(base),"baseline");
    checks::original_materials(&json!(base),&original,&[("blank",1),("pcb",1),("fastener",4),("carton",1)]);
    let mut next=source("delayed-pcb")["scenario"].clone();next["project_revision"]=json!(1);
    let _:EditCommit=ok(h.apply(1,json!([put("scenario",next)])));
    let late=execute(&mut h,"IX01-delayed-pcb");assert_ne!(base.id,late.id);
    checks::ix01(&json!(late),"delayed_pcb");checks::original_materials(&json!(late),&source("delayed-pcb"),&[("blank",1),("pcb",1),("fastener",4),("carton",1)]);
    let a=record(&mut h,&base,"baseline");let b=record(&mut h,&late,"delayed-pcb");
    let comparison:RunComparison=ok(h.call("compare_runs",json!({"run_ids":[base.id,late.id],"metric_scope":{"keys":[],"objects":[],"acceptance":[]}})));
    write(&out().join("comparison.json"),&json!(comparison));
    // Existing Engine step producer supplies a genuine computed-boundary prefix.
    // This is explicitly a direct-Engine sample, not a Host pause capability.
    let mut engine=sf_simulation::Engine::new(sf_simulation::EngineInput {
        run_id:base.id.clone(),input_ref:base.input_ref.clone(),project:serde_json::from_value(a["project"].clone()).unwrap(),
        scenario:serde_json::from_value(a["scenario"].clone()).unwrap(),definitions:serde_json::from_value(a["definitions"].clone()).unwrap(),scope:base.scope.clone().into_iter().collect()}).unwrap();
    loop {assert_ne!(engine.step(),0);if engine.snapshot().computed_until_ms.as_ref().unwrap().0.value()>=150000 {break;}}
    let prefix=json!({"project":a["project"],"scenario":a["scenario"],"run":engine.snapshot(),"producer":"actual sf_simulation::Engine step; not Host-published pause"});
    assert!(prefix["run"]["computed_until_ms"].as_u64().unwrap()<600000);
    write(&out().join("baseline-prefix.actual.json"),&prefix);
    engine.run_to_end();assert_eq!(json!(engine.snapshot()),json!(base));
    let saved=out().join(format!("saved-{}",std::process::id()));fs::create_dir(&saved).unwrap();let revision=h.context().project.unwrap().revision.0.value();
    let task=h.save(revision,&saved);h.finish_one();assert!(matches!(h.task(&task,true).task.state,ComputationStatus::Completed));
    write(&out().join("expected-reopen.json"),&json!([a,b]));
    let mut child=Command::new(std::env::current_exe().unwrap());
    child.args(["--exact","timeline_reopen_child","--ignored","--nocapture"]).env("NONCU_TIMELINE_REOPEN","1").env("NONCU_TIMELINE_SAVED",&saved);
    #[cfg(windows)]
    child.creation_flags(0x08000000);
    let result=child.output().unwrap();
    fs::write(out().join("reopen.stdout.txt"),result.stdout).unwrap();fs::write(out().join("reopen.stderr.txt"),result.stderr).unwrap();assert!(result.status.success());
}
#[test]
#[ignore="only launched by the parent as a fresh process"]
fn timeline_reopen_child() {
    assert_eq!(std::env::var("NONCU_TIMELINE_REOPEN").unwrap(),"1");
    let saved=PathBuf::from(std::env::var("NONCU_TIMELINE_SAVED").expect("parent-selected own archive"));
    let expected=read(&out().join("expected-reopen.json"));let tree=Sandbox::new();let mut h=Harness::from_seed(tree,&saved);
    for record in expected.as_array().unwrap() {
        let r:RunResult=serde_json::from_value(record["run"].clone()).unwrap();assert_eq!(json!(h.full(&r.id)),json!(r));
        assert_eq!(frozen(&mut h,&r,"project","P1"),record["project"]);
        for frame in record["observations"].as_array().unwrap() {assert_eq!(*frame,objects(&mut h,&r,frame["at_ms"].as_u64().unwrap(),4));}
        event_pages(&mut h,&r);
    }
    write(&out().join("reopen-result.json"),&json!({"fresh_process":true,"pid":std::process::id(),"archive":saved,"both_Run_and_all_history_exact":true}));
}
#[test]
fn timeline_phase_calendar_actual_worker() {
    let value=source("phase-calendar");let mut h=Harness::from_value(value.clone());let r=execute(&mut h,value["scenario"]["id"].as_str().unwrap());
    let project=frozen(&mut h,&r,"project","P1");let scenario=frozen(&mut h,&r,"scenario",value["scenario"]["id"].as_str().unwrap());
    write(&out().join("phase-calendar.actual.json"),&json!({"project":project,"scenario":scenario,"run":r}));
}
#[test]
fn timeline_output_wait_after_last_phase_keeps_machine_and_pending_item() {
    let mut value=source("phase-calendar");
    value["project"]["locations"].as_array_mut().unwrap().iter_mut().find(|l|l["id"]=="finished").unwrap()["capacity"]["value"]["coefficient"]=json!(0);
    let mut h=Harness::from_value(value.clone());let r=execute(&mut h,"PHASE");
    let job=&r.jobs[0];assert!(job.finished_at_ms.is_none());assert!(job.phase_ref.is_none());assert_eq!(job.pending_outputs.len(),1);
    assert!(r.waits.iter().any(|w|w.interval.start_ms.0.value()==45000&&w.interval.end_ms.0.value()==100000&&w.phase_ref.is_none()));
    let mut frames=Vec::new();for at in [44999,45000,99999,100000] {frames.push(objects(&mut h,&r,at,3));}
    let project=frozen(&mut h,&r,"project","P1");
    write(&out().join("output-wait.actual.json"),&json!({"source":"p1-phase-calendar.json; only finished capacity=0, as independent phased_resources_units boundary", "project":project,"run":r,"observations":frames}));
}
#[test]
fn timeline_generated_command_real_conflict_undo_redo_and_next_run() {
    let document=read(&out().parent().unwrap().join("input-proposal.json"));let proposal=&document["proposal"];
    let mut h=Harness::from_value(source("baseline"));let accepted=h.run("IX01-baseline");
    assert!(h.task(&accepted,true).result.is_none());
    let original=h.object("scenario","IX01-baseline");
    assert_eq!(proposal["base_project_revision"],1);assert_eq!(proposal["base_scenario_revision"],1);
    let committed:EditCommit=ok(h.apply(1,json!([proposal["command"]])));assert_eq!(committed.new_revision.0.value(),2);
    // The already accepted work still executes its frozen r1 while the editable
    // Scenario is now r2. It must not borrow the new release/arrival values.
    h.finish_one();let old=h.completed_run(&accepted);assert_eq!(old.input_ref.project_revision.0.value(),1);
    checks::ix01(&json!(old),"baseline");let frozen=json!(old);
    let next=h.object("scenario","IX01-baseline");
    assert_eq!(next["arrivals"].as_array().unwrap().last().unwrap()["at_ms"].as_u64().unwrap(),original["arrivals"].as_array().unwrap().last().unwrap()["at_ms"].as_u64().unwrap()+1000);
    assert_eq!(next["orders"],original["orders"]);assert_eq!(next["initial_conditions"],original["initial_conditions"]);
    let conflict=error(h.apply(1,json!([proposal["command"]])));assert!(matches!(conflict.code,ErrorCode::RevisionConflict));assert_eq!(h.object("scenario","IX01-baseline"),next);
    let changed=execute(&mut h,"IX01-baseline");assert_ne!(changed.id,old.id);assert_eq!(changed.input_ref.project_revision.0.value(),2);
    assert_eq!(json!(h.full(&old.id)),frozen);
    let undo:EditCommit=ok(h.apply(2,json!([{"kind":"undo"}])));assert_eq!(undo.new_revision.0.value(),3);
    let restored=h.object("scenario","IX01-baseline");assert_eq!(restored["arrivals"],original["arrivals"]);assert_eq!(restored["production_tasks"],original["production_tasks"]);
    let redo:EditCommit=ok(h.apply(3,json!([{"kind":"redo"}])));assert_eq!(redo.new_revision.0.value(),4);
    let reapplied=h.object("scenario","IX01-baseline");assert_eq!(reapplied["arrivals"],next["arrivals"]);assert_eq!(reapplied["production_tasks"],next["production_tasks"]);
    assert_eq!(json!(h.full(&old.id)),frozen);assert_eq!(json!(h.full(&changed.id)),json!(changed));
    write(&out().join("input-transaction-result.json"),&json!({"actual_frontend_proposal":proposal,"accepted_r1_TaskId":accepted.id,"edited_before_worker":true,"committed_revision":2,"stale_attempt":"RevisionConflict","undo_revision":3,"redo_revision":4,"original_Run":old.id,"new_Run":changed.id,"both_Run_unchanged":true}));
}
