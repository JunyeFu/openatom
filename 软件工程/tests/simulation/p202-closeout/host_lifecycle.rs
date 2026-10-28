mod common;
use common::{host::*,*};
use serde_json::json;
use sf_domain::ComputationStatus;
use std::{fs,process::Command};
#[test]
fn all_original_transport_cases_save_move_and_reopen_in_fresh_processes(){
 for case in ["halls-A","halls-B","short-stock","whole-batch-short","identity","blocked","departure-missed","B13","B14","B16","phase-people","ix01-baseline","ix01-delayed-pcb","same-carrier","concurrent-unload","split-batch","closed-calendar"] {
    let dir=delivery_dir().join("evidence").join(format!("{case}-{}",std::process::id()));fs::create_dir_all(&dir).unwrap();
    let value=source(case); let scenario=value["scenario"]["id"].as_str().unwrap();
    let mut engine=engine_support::engine(&value);engine.run_to_end();let expected=engine_support::snapshot(&engine);check_variants(case,&expected);
    let mut h=Harness::from_value(value.clone());let task=h.run(scenario);h.finish_one();let run=h.completed_run(&task);
    assert_eq!(business(json!(run)),business(expected.clone()),"Host/Engine parity {case}");
    let hist=history(&mut h,&run);
    for row in hist["samples"].as_array().unwrap(){
        let at=serde_json::from_value(row["at_ms"].clone()).unwrap();
        let expected=serde_json::to_value(engine.objects_at(at).unwrap()).unwrap();
        assert_eq!(row["items"],expected,"historical Host/Engine parity {case} at {}",row["at_ms"]);
    }
    write(&dir.join("run.json"),&json!(run));write(&dir.join("engine-run.json"),&expected);write(&dir.join("history.json"),&hist);
    write(&dir.join("packet.json"),&json!({"case":case,"input":value,"parent_pid":std::process::id(),"project":frozen(&mut h,&run,"project","P1"),"scenario":frozen(&mut h,&run,"scenario",scenario)}));
    let saved=dir.join("saved");fs::create_dir(&saved).unwrap();let revision=h.context().project.unwrap().revision.0.value();let task=h.save(revision,&saved);h.finish_one();assert!(matches!(h.task(&task,true).task.state,ComputationStatus::Completed));drop(h);
    fs::rename(&saved,dir.join("project")).unwrap();
    let mut command=Command::new(env!("CARGO_BIN_EXE_p202-reopen"));command.arg(&dir);
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let out=command.output().unwrap();fs::write(dir.join("child.stdout.txt"),out.stdout).unwrap();fs::write(dir.join("child.stderr.txt"),&out.stderr).unwrap();assert!(out.status.success(),"{case}: {}",String::from_utf8_lossy(&out.stderr));
    let receipt=read(&dir.join("child-receipt.json"));assert_eq!(receipt["passed"],true);assert_ne!(receipt["pid"],std::process::id());println!("P202_CASE {}",json!({"case":case,"receipt":receipt,"directory":dir}));
 }
}
#[test]
fn original_invalid_transport_inputs_are_rejected_without_mutation(){
 for case in ["bad-unit","bad-destination","missing-duration","missing-role"]{
    let value=source(case);let before=value.clone();let result=sf_simulation::Engine::new(engine_support::input(&value));
    let Err(issues)=result else {panic!("invalid input admitted {case}")};assert_eq!(value,before);
    write(&delivery_dir().join(format!("{case}-rejection.json")),&json!({"case":case,"input":value,"issues":issues,"unchanged":true}));
 }
}
