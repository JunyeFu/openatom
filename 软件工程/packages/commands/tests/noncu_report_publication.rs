//! Frozen R12 packaging report through the current public Host and directory writer.
#[path = "fixtures/production/host_support.rs"]
mod host;
use serde_json::json;
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn input() -> PathBuf {
    std::env::var_os("NONCU_PACKAGING_INPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            root().join("packages/commands/tests/fixtures/evaluation/r12-publication")
        })
}
fn fonts() -> sf_reporting::EvaluationResources {
    let at = root().join("tools/architecture-proof/assets");
    sf_reporting::EvaluationResources {
        cjk_font: Some(fs::read(at.join("SourceHanSansCN-Regular.otf")).unwrap()),
        font_license: Some(fs::read(at.join("LICENSE.SourceHanSans.txt")).unwrap()),
        files: BTreeMap::new(),
    }
}
fn evidence(name: &str) -> Option<PathBuf> {
    std::env::var_os("NONCU_REPORT_OUTPUT").map(|base| {
        let at = PathBuf::from(base).join(name);
        fs::create_dir_all(&at).unwrap();
        at
    })
}
fn frozen_host() -> host::Harness {
    let tree = host::Sandbox::new();
    let input = tree.directory("copied-r12-project");
    host::copy(&self::input().join("saved-project"), &input);
    let mut h = host::Harness::from_seed(tree, &input);
    h.host.set_evaluation_resources(fonts());
    h
}
fn admit(h: &mut host::Harness, parent: &Path) -> TaskHandle {
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::ExportEvaluation, parent.to_owned())
        .unwrap();
    let source = host::read(&input().join("reports/.sf-evaluation-74804-24/report.json"));
    let revision = h.context().project.unwrap().revision;
    host::ok(h.call(
        "export_evaluation",
        json!({"revision":revision,
        "report_mode":"simulation_evaluation", "scenario_refs":[],
        "run_refs":source["data"]["run_refs"],
        "metric_scope":{"keys":[],"objects":[],"acceptance":[]},
        "formats":["pdf","html","csv","json"],"selected_destination_ref":selected}),
    ))
}
fn design(h: &mut host::Harness, parent: &Path) -> TaskHandle {
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::ExportEvaluation, parent.to_owned())
        .unwrap();
    let revision = h.context().project.unwrap().revision;
    host::ok(h.call(
        "export_evaluation",
        json!({"revision":revision,
        "report_mode":"design_description", "scenario_refs":[], "run_refs":[],
        "metric_scope":{"keys":[],"objects":[],"acceptance":[]},
        "formats":["json"],"selected_destination_ref":selected}),
    ))
}
fn retain(name: &str, parent: &Path, states: &[TaskState]) {
    if let Some(at) = evidence(name) {
        host::write(
            &at.join("tasks.json"),
            &serde_json::to_value(states).unwrap(),
        );
        let reports = at.join("reports");
        fs::create_dir_all(&reports).unwrap();
        host::copy(parent, &reports);
    }
}
fn files(at: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, at: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, out)
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(at, at, &mut out);
    out
}

#[test]
fn published_report_progress_does_not_replace_the_host_task_completion() {
    let mut h = frozen_host();
    let parent = h.tree.directory("actual-report-progress");
    let task = design(&mut h, &parent);
    let mut work = h.host.drain_io_work();
    assert_eq!(work.len(), 1);
    let completion = work.pop().unwrap().run();
    let working = h.task(&task, true);
    assert!(matches!(working.task.state, ComputationStatus::Running));
    assert!(working.result.is_none());
    let progress = working.task.progress.expect("actual report work publishes progress");
    assert!(!progress.phase.is_empty());
    assert!(progress.processed.value() > 0);
    assert_eq!(Some(progress.processed), progress.total);
    h.host.finish_io(completion);
    let finished = h.task(&task, true);
    assert!(matches!(finished.task.state, ComputationStatus::Completed));
    assert!(matches!(finished.result, Some(TaskOutput::Evaluation(_))));
}

#[test]
fn r12_saved_actual_packaging_runs_publish_all_formats_and_reopen_without_changing_frozen_inputs() {
    let mut h = frozen_host();
    let before = files(&h.tree.path.join("copied-r12-project"));
    let parent = h
        .tree
        .path
        .join("lifecycle/sleeve-part_relations_or_repeat_order/reports");
    fs::create_dir_all(&parent).unwrap();
    let task = admit(&mut h, &parent);
    assert!(matches!(task.state, ComputationStatus::Queued));
    assert!(h.task(&task, true).result.is_none());
    assert_eq!(fs::read_dir(&parent).unwrap().count(), 0);
    h.finish_one();
    let state = h.task(&task, true);
    if let Some(at) = evidence("r12") {
        host::write(
            &at.join("publication-path.json"),
            &json!({"selected_parent":parent,"parent_utf16_units":parent.as_os_str().to_string_lossy().encode_utf16().count(),"project_local_temp":std::env::temp_dir(),"candidate_and_target_are_siblings":true}),
        );
        host::write(
            &at.join("task.json"),
            &serde_json::to_value(&state).unwrap(),
        );
        let reports = at.join("reports");
        fs::create_dir_all(&reports).unwrap();
        host::copy(&parent, &reports);
    }
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let TaskOutput::Evaluation(output) = state.result.unwrap() else {
        panic!("report result required")
    };
    let report_ref = output
        .files
        .iter()
        .find(|f| matches!(f.kind, DocumentKind::Evaluation))
        .unwrap();
    let folder = parent
        .join(report_ref.path.as_str())
        .parent()
        .unwrap()
        .to_owned();
    let actual = host::read(&folder.join("report.json"));
    assert_eq!(
        actual["data"],
        serde_json::to_value(&output.context).unwrap()
    );
    let original = host::read(&input().join("reports/.sf-evaluation-74804-24/report.json"));
    assert_eq!(actual["data"]["run_refs"], original["data"]["run_refs"]);
    assert_eq!(
        actual["data"]["run_evaluations"],
        original["data"]["run_evaluations"]
    );
    assert_eq!(
        actual["data"]["scenario_comparison"],
        original["data"]["scenario_comparison"]
    );
    assert_eq!(fs::read_dir(folder.join("csv")).unwrap().count(), 15);
    for f in &output.files {
        assert!(
            parent.join(f.path.as_str()).is_file(),
            "{}",
            f.path.as_str()
        );
    }
    assert!(
        fs::read(folder.join("report.pdf"))
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert!(folder.join("licenses/SourceHanSans-LICENSE.txt").is_file());
    let manifest = host::read(&h.tree.path.join("copied-r12-project/project.json"));
    let mut professional_files = 0;
    let mut svgs = 0;
    for entry in manifest["data"]["documents"].as_array().unwrap() {
        let path = entry["file_ref"]["path"].as_str().unwrap();
        if matches!(
            entry["role"].as_str(),
            Some("artifact_metadata" | "professional_input" | "professional_result")
        ) || path.starts_with("documents/packaging-")
        {
            assert_eq!(
                fs::read(folder.join("project").join(path)).unwrap(),
                before[Path::new(path)]
            );
            professional_files += 1;
            svgs += usize::from(path.ends_with(".svg"));
        }
    }
    assert_eq!(svgs, 3);
    assert!(professional_files >= 9);
    for id in ["whole-w5-host:artifact:1:2", "whole-w5-host:artifact:1:4"] {
        assert!(output.context.conclusions.iter().any(|s| s.contains(id)));
    }
    let moved = h.tree.directory("moved-report-project");
    host::copy(&folder.join("project"), &moved);
    let mut reopened = host::Harness::from_seed(host::Sandbox::new(), &moved);
    for id in &output.context.run_refs {
        assert_eq!(
            serde_json::to_value(h.full(id)).unwrap(),
            serde_json::to_value(reopened.full(id)).unwrap()
        );
    }
    assert_eq!(files(&h.tree.path.join("copied-r12-project")), before);
}

#[test]
fn two_independent_hosts_race_for_same_final_name_without_overwrite_or_mixed_files() {
    let mut a = host::Harness::new();
    let mut alternative = host::fixture();
    alternative["project"]["title"] = json!("并发B：独立冻结工程");
    let mut b = host::Harness::from_value(alternative);
    let parent = a.tree.directory("shared-selected-parent");
    let ta = design(&mut a, &parent);
    let tb = design(&mut b, &parent);
    let wa = a.host.drain_io_work().pop().unwrap();
    let wb = b.host.drain_io_work().pop().unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let gate = barrier.clone();
    let worker = std::thread::spawn(move || {
        gate.wait();
        wa.run()
    });
    barrier.wait();
    let cb = wb.run();
    let ca = worker.join().unwrap();
    a.host.finish_io(ca);
    b.host.finish_io(cb);
    let states = [a.task(&ta, true), b.task(&tb, true)];
    retain("concurrent-hosts", &parent, &states);
    assert_eq!(
        states
            .iter()
            .filter(|s| matches!(s.task.state, ComputationStatus::Completed))
            .count(),
        1
    );
    assert_eq!(
        states
            .iter()
            .filter(|s| matches!(s.task.state, ComputationStatus::Failed))
            .count(),
        1
    );
    let targets = fs::read_dir(&parent)
        .unwrap()
        .map(Result::unwrap)
        .filter(|e| e.file_name().to_string_lossy().starts_with("evaluation-"))
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 1);
    let report = host::read(&targets[0].path().join("report.json"));
    let winner = states
        .iter()
        .find(|s| matches!(s.task.state, ComputationStatus::Completed))
        .unwrap();
    let TaskOutput::Evaluation(output) = winner.result.as_ref().unwrap() else {
        panic!("evaluation")
    };
    assert_eq!(
        report["data"],
        serde_json::to_value(&output.context).unwrap()
    );
    let archive = targets[0].path().join("project");
    let manifest = host::read(&archive.join("project.json"));
    let active = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["role"] == "active_project")
        .unwrap();
    let archived = host::read(&archive.join(active["file_ref"]["path"].as_str().unwrap()));
    let expected = if matches!(states[0].task.state, ComputationStatus::Completed) {
        a.object("project", "P1")
    } else {
        b.object("project", "P1")
    };
    assert_eq!(archived["data"], expected);
    assert!(
        output
            .context
            .question_and_scope
            .contains(expected["title"].as_str().unwrap())
    );
    assert!(
        states
            .iter()
            .find(|s| matches!(s.task.state, ComputationStatus::Failed))
            .unwrap()
            .result
            .is_none()
    );
}

#[cfg(windows)]
#[test]
fn actual_open_descendant_blocks_move_and_retains_candidate_without_exposing_a_partial_final_directory()
 {
    use std::os::windows::fs::OpenOptionsExt;
    let mut h = frozen_host();
    let parent = h.tree.directory("held-descendant");
    let task = admit(&mut h, &parent);
    let watched = parent.clone();
    let (send, observed) = std::sync::mpsc::channel();
    let (release, receive) = std::sync::mpsc::channel();
    let holder = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while std::time::Instant::now() < deadline {
            for e in fs::read_dir(&watched).unwrap() {
                let p = e.unwrap().path().join("project");
                if let Ok(handle) = fs::OpenOptions::new()
                    .read(true)
                    .share_mode(3)
                    .custom_flags(0x02200000)
                    .open(&p)
                {
                    send.send(p).unwrap();
                    receive.recv().unwrap();
                    drop(handle);
                    return;
                }
            }
            std::thread::yield_now();
        }
        panic!("actual staging child was never observed");
    });
    // Two one-way channels keep the known holder alive through the real failing API.
    let mut work = h.host.drain_io_work();
    let completion = work.pop().unwrap().run();
    h.host.finish_io(completion);
    let state = h.task(&task, true);
    let held = observed
        .recv_timeout(std::time::Duration::from_secs(1))
        .unwrap();
    retain("held-descendant", &parent, std::slice::from_ref(&state));
    if let Some(at) = evidence("held-descendant") {
        host::write(
            &at.join("holding-condition.json"),
            &json!({"path":held,"holder_pid":std::process::id(),"share_read":true,"share_write":true,"share_delete":false,"API":"MoveFileW"}),
        );
    }
    release.send(()).unwrap();
    holder.join().unwrap();
    assert!(
        matches!(state.task.state, ComputationStatus::Failed),
        "{state:?}"
    );
    assert!(state.result.is_none());
    assert!(matches!(
        state.failure.as_ref().unwrap().code,
        ErrorCode::IoFailure
    ));
    assert!(
        state
            .failure
            .as_ref()
            .unwrap()
            .message
            .contains("os error 5"),
        "{state:?}"
    );
    assert!(
        state
            .failure
            .as_ref()
            .unwrap()
            .message
            .contains("MoveFileW")
    );
    assert!(
        state
            .failure
            .as_ref()
            .unwrap()
            .message
            .contains(".sf-evaluation-")
    );
    assert!(
        state
            .failure
            .as_ref()
            .unwrap()
            .message
            .contains("evaluation-00000002")
    );
    assert_eq!(
        fs::read_dir(&parent)
            .unwrap()
            .map(Result::unwrap)
            .filter(|e| e.file_name().to_string_lossy().starts_with("evaluation-"))
            .count(),
        0
    );
    assert!(held.parent().unwrap().join("report.json").is_file());
    let failed_stage = held.parent().unwrap().to_owned();
    let failed_bytes = files(&failed_stage);
    // A new explicit request gets a new candidate; the failed Task is not retried.
    let next = admit(&mut h, &parent);
    assert_ne!(next.id, task.id);
    h.finish_one();
    let recovered = h.task(&next, true);
    assert!(
        matches!(recovered.task.state, ComputationStatus::Completed),
        "{recovered:?}"
    );
    let TaskOutput::Evaluation(output) = recovered.result.as_ref().unwrap() else {
        panic!("new request must return its committed evaluation")
    };
    let report_file = output
        .files
        .iter()
        .find(|file| matches!(file.kind, DocumentKind::Evaluation))
        .unwrap();
    assert!(
        report_file
            .path
            .as_str()
            .starts_with("evaluation-00000003/")
    );
    assert_eq!(
        host::read(&parent.join(report_file.path.as_str()))["data"],
        serde_json::to_value(&output.context).unwrap()
    );
    assert_eq!(output.context.run_refs.len(), 2);
    assert_eq!(files(&failed_stage), failed_bytes);
    assert_eq!(
        serde_json::to_value(h.task(&task, true)).unwrap(),
        serde_json::to_value(&state).unwrap()
    );
    retain("held-descendant-recovery", &parent, &[state, recovered]);
}

#[cfg(windows)]
#[test]
fn cancellation_during_actual_staging_has_no_final_directory_and_no_success_result() {
    let mut h = frozen_host();
    let parent = h.tree.directory("cancel-during-stage");
    let task = admit(&mut h, &parent);
    let work = h.host.drain_io_work().pop().unwrap();
    let worker = std::thread::spawn(move || work.run());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let stage = loop {
        let candidate = fs::read_dir(&parent)
            .unwrap()
            .map(Result::unwrap)
            .map(|e| e.path())
            .find(|p| p.join("project/project.json").is_file());
        if let Some(p) = candidate {
            break p;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "actual stage not observed before commit"
        );
        std::thread::yield_now();
    };
    let _: CancellationState =
        host::ok(h.call("cancel_task", json!({"task_id":task.id,"action":"cancel"})));
    h.host.finish_io(worker.join().unwrap());
    let state = h.task(&task, true);
    retain("cancel-during-stage", &parent, std::slice::from_ref(&state));
    assert!(
        matches!(state.task.state, ComputationStatus::Cancelled),
        "{state:?}"
    );
    assert!(state.result.is_none());
    assert!(stage.exists());
    assert_eq!(
        fs::read_dir(&parent)
            .unwrap()
            .map(Result::unwrap)
            .filter(|e| e.file_name().to_string_lossy().starts_with("evaluation-"))
            .count(),
        0
    );
}

#[cfg(windows)]
#[test]
#[ignore = "only executed as this test's hidden crash subprocess"]
fn abrupt_worker_helper() {
    use std::os::windows::fs::OpenOptionsExt;
    let parent = PathBuf::from(std::env::var_os("NONCU_ABORT_PARENT").expect("helper-only parent"));
    let mut h = frozen_host();
    let task = admit(&mut h, &parent);
    host::write(
        &parent.join("queued-task.json"),
        &serde_json::to_value(h.task(&task, true)).unwrap(),
    );
    let watched = parent.clone();
    let observer = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while std::time::Instant::now() < deadline {
            for e in fs::read_dir(&watched).unwrap() {
                let p = e.unwrap().path();
                if !p
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".sf-evaluation-")
                {
                    continue;
                }
                if let Ok(_guard) = fs::OpenOptions::new()
                    .read(true)
                    .share_mode(3)
                    .custom_flags(0x02200000)
                    .open(&p)
                {
                    while !p.join("project/project.json").is_file() {
                        assert!(std::time::Instant::now() < deadline);
                        std::thread::yield_now();
                    }
                    std::process::exit(91);
                }
            }
            std::thread::yield_now();
        }
        panic!("actual stage not observed");
    });
    h.finish_one();
    observer.join().unwrap();
    panic!("worker should have aborted");
}

#[cfg(windows)]
#[test]
fn abrupt_own_worker_exit_keeps_old_valid_output_and_never_exposes_partial_final_directory() {
    use std::os::windows::process::CommandExt;
    let tree = host::Sandbox::new();
    let parent = tree.directory("aborted-report");
    let old = parent.join("evaluation-00000001");
    fs::create_dir(&old).unwrap();
    fs::write(old.join("old-valid.txt"), b"prior committed output").unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "abrupt_worker_helper",
            "--ignored",
            "--nocapture",
        ])
        .env("NONCU_ABORT_PARENT", &parent)
        .creation_flags(0x08000000)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(91));
    assert_eq!(
        fs::read(old.join("old-valid.txt")).unwrap(),
        b"prior committed output"
    );
    assert!(!parent.join("evaluation-00000002").exists());
    let stages = fs::read_dir(&parent)
        .unwrap()
        .map(Result::unwrap)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".sf-evaluation-")
        })
        .collect::<Vec<_>>();
    assert_eq!(stages.len(), 1);
    assert!(stages[0].join("project/project.json").is_file());
    if let Some(at) = evidence("abrupt-worker") {
        host::write(
            &at.join("exit.json"),
            &json!({"code":status.code(),"own_child_waited":true,"old_output_preserved":true,"final_target_absent":true,"stage":stages[0]}),
        );
        let reports = at.join("reports");
        fs::create_dir_all(&reports).unwrap();
        host::copy(&parent, &reports);
    }
}
