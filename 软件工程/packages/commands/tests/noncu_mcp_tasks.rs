//! Deterministic real worker/Host cancellation boundaries behind the MCP tools.
#[path="fixtures/production/host_support.rs"] mod support;
use support::*;
use serde_json::json;
use sf_commands::*;
use sf_domain::*;

#[test]
fn admission_is_not_output_and_cancel_before_worker_prevents_directory_commit() {
    let mut h=Harness::new();
    let destination=h.tree.directory("noncu-cancel-before-worker");
    let handle=h.save(1,&destination);
    let admitted=h.task(&handle,true);
    assert!(admitted.result.is_none());
    assert!(!destination.join("project.json").exists());
    let cancellation:CancellationState=ok(h.call("cancel_task",json!({"task_id":handle.id,"action":"cancel"})));
    assert!(cancellation.action_accepted&&!cancellation.terminal_won);
    h.finish_one();
    let final_state=h.task(&handle,true);
    assert!(matches!(final_state.task.state,ComputationStatus::Cancelled));
    assert!(final_state.result.is_none());
    assert!(!destination.join("project.json").exists());
    println!("{}",json!({"case":"cancel_before_worker","TaskId":handle.id,"admission_was_output":false,"terminal":"cancelled","directory_committed":false}));
}

#[test]
fn actual_io_commit_wins_cancel_between_worker_output_and_Host_completion_publication() {
    let mut h=Harness::new();
    let destination=h.tree.directory("noncu-commit-before-completion");
    let handle=h.save(1,&destination);
    let mut work=h.host.drain_io_work();
    assert_eq!(work.len(),1);
    let completed_work=work.pop().unwrap().run();
    assert!(destination.join("project.json").is_file());
    let cancellation:CancellationState=ok(h.call("cancel_task",json!({"task_id":handle.id,"action":"cancel"})));
    h.host.finish_io(completed_work);
    let state=h.task(&handle,true);
    assert!(matches!(state.task.state,ComputationStatus::Completed),"{state:?}");
    assert!(matches!(state.result,Some(TaskOutput::Save(_))));
    assert!(destination.join("project.json").is_file());
    let late:CancellationState=ok(h.call("cancel_task",json!({"task_id":handle.id,"action":"cancel"})));
    assert!(late.terminal_won&&!late.action_accepted);
    println!("{}",json!({"case":"commit_before_Host_completion_publication","TaskId":handle.id,
        "cancel_race_response":cancellation,"terminal":"completed","late_cancel_terminal_won":late.terminal_won}));
}
