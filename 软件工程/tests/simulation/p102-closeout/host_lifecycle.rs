mod common;
use common::{assert_fixed, host::*, p1, root};
use serde_json::json;
use sf_domain::ComputationStatus;
use std::{fs, process::Command};

fn lifecycle(case: &str) {
    let evidence = root()
        .join(".work/parallel-rework-closeout/delivery/evidence")
        .join(format!("{case}-{}", std::process::id()));
    fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    fs::create_dir(&evidence).unwrap();
    let input = p1::parallel(case);
    write(&evidence.join("input.json"), &input);
    let mut h = Harness::from_value(input);
    let before_project = h.object("project", "P1");
    let before_scenario = h.object("scenario", case);
    let task = h.run(case);
    assert!(matches!(task.state, ComputationStatus::Queued));
    h.finish_one();
    let run = h.completed_run(&task);
    write(
        &evidence.join("original-run.json"),
        &serde_json::to_value(&run).unwrap(),
    );
    let summary = assert_fixed(case, &run);
    write(&evidence.join("original-project.json"), &before_project);
    write(&evidence.join("original-scenario.json"), &before_scenario);
    write(&evidence.join("original-summary.json"), &summary);
    let saved = evidence.join("saved");
    let moved = evidence.join("moved");
    fs::create_dir(&saved).unwrap();
    fs::create_dir(&moved).unwrap();
    let base = h.context().project.unwrap().revision.0.value();
    let save = h.save(base, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&saved, &moved);
    assert_eq!(
        serde_json::to_value(h.full(&run.id)).unwrap(),
        read(&evidence.join("original-run.json"))
    );
    drop(h);
    let output = Command::new(env!("CARGO_BIN_EXE_p102-reopen"))
        .arg(&evidence)
        .arg(case)
        .output()
        .unwrap();
    fs::write(evidence.join("fresh-process.stdout.txt"), &output.stdout).unwrap();
    fs::write(evidence.join("fresh-process.stderr.txt"), &output.stderr).unwrap();
    assert!(
        output.status.success(),
        "fresh process failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let child = read(&evidence.join("fresh-process-receipt.json"));
    assert_ne!(child["pid"], json!(std::process::id()));
    assert_eq!(child["original_case"], case);
    assert_eq!(child["old_run_preserved"], true);
    assert_eq!(child["same_input_rerun_business_equal"], true);
    println!(
        "P102_HOST_LIFECYCLE {}",
        json!({"case":case,"parent_pid":std::process::id(),"child":child,"evidence":evidence})
    );
}

#[test]
fn a_rework_pass_145_175_survives_save_move_fresh_process_and_changed_quality() {
    lifecycle("A");
}
#[test]
fn q_cap_isolation_175_survives_save_move_fresh_process_and_changed_quality() {
    lifecycle("Q");
}
#[test]
fn u_pending_inspector_hold_survives_save_move_fresh_process_and_changed_quality() {
    lifecycle("U");
}
