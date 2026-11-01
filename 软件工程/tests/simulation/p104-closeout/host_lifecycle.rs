mod common;
use common::{host::*, *};
use serde_json::json;
use sf_domain::ComputationStatus;
use std::{fs, process::Command};

fn lifecycle(case: &str) {
    let directory = root()
        .join(".work/p104-closeout/delivery/evidence")
        .join(format!("{case}-{}", std::process::id()));
    fs::create_dir_all(directory.parent().unwrap()).unwrap();
    fs::create_dir(&directory).unwrap();
    let input = source(case);
    let scenario = input["scenario"]["id"].as_str().unwrap();
    let mut h = Harness::from_value(input.clone());
    let run = execute(&mut h, scenario);
    write(&directory.join("original-run.json"), &json!(run));
    check(case, &run);
    let history = history(&mut h, &run);
    write(
        &directory.join("packet.json"),
        &json!({"case":case,"scenario":scenario,"input":input,
        "project":frozen(&mut h,&run,"project","P1"),"scenario_input":frozen(&mut h,&run,"scenario",scenario),"history":history}),
    );
    let saved = directory.join("saved");
    let moved = directory.join("moved");
    fs::create_dir(&saved).unwrap();
    
    let base = h.context().project.unwrap().revision.0.value();
    let task = h.save(base, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    fs::rename(&saved, &moved).unwrap();
    drop(h);
    let mut command = Command::new(env!("CARGO_BIN_EXE_p104-reopen"));
    command.arg(&directory);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().unwrap();
    fs::write(directory.join("child.stdout.txt"), output.stdout).unwrap();
    fs::write(directory.join("child.stderr.txt"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let child = read(&directory.join("child-receipt.json"));
    assert_eq!(child["passed"], true);
    assert_ne!(child["pid"], std::process::id());
    let mut final_read = Command::new(env!("CARGO_BIN_EXE_p104-reopen"));
    final_read.arg(&directory).arg("final");
    #[cfg(windows)] { use std::os::windows::process::CommandExt; final_read.creation_flags(0x08000000); }
    let final_output = final_read.output().unwrap();
    fs::write(directory.join("final.stdout.txt"), &final_output.stdout).unwrap();
    fs::write(directory.join("final.stderr.txt"), &final_output.stderr).unwrap();
    assert!(final_output.status.success(), "{}", String::from_utf8_lossy(&final_output.stderr));
    println!(
        "P104_LIFECYCLE {}",
        json!({"case":case,"parent_pid":std::process::id(),"child":child,"evidence":directory})
    );
}
#[test]
fn a_first_ix02_205_late5_worker135_new_process_priority_edit_and_history() {
    lifecycle("A");
}
#[test]
fn b_first_ix02_205_late45_worker160_new_process_priority_edit_and_history() {
    lifecycle("B");
}
#[test]
fn missing_pcb_keeps_completion_unknown_in_public_metrics_then_explicit_supply_edit_reruns() {
    lifecycle("missing");
}
