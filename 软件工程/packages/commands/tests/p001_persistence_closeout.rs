//! P0-01 independent functional closeout; no GUI or performance claim.
#![cfg(windows)]
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Barrier, Mutex},
};
use support::*;

fn evidence(name: &str) -> PathBuf {
    let p = PathBuf::from(std::env::var_os("SF_P001_EVIDENCE").expect("own evidence directory"))
        .join(name);
    fs::create_dir_all(&p).unwrap();
    p
}
fn record(path: &Path, v: Value) {
    fs::write(path, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}
fn rev(h: &mut Harness) -> u64 {
    h.context().project.unwrap().revision.0.value()
}
fn set_height(h: &mut Harness, base: u64, n: f64) -> Value {
    let mut d = h.object("device", "ASM-01");
    d["height_m"] = json!(n);
    h.apply(base, json!([put("device", d)]))
}
fn save_ok(h: &mut Harness, dir: &Path) {
    let r = rev(h);
    let t = h.save(r, dir);
    h.finish_one();
    let s = h.task(&t, true);
    assert!(
        matches!(s.task.state, ComputationStatus::Completed),
        "{s:?}"
    );
    assert!(!h.context().project.unwrap().dirty);
}
fn with_run() -> (Harness, RunResult) {
    let mut h = Harness::new();
    let t = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&t);
    assert_eq!(run.acceptance_events.len(), 1);
    (h, run)
}
fn expected(h: &mut Harness, run: &RunResult) -> Value {
    json!({"project":h.object("project","P1"),"scenario":h.object("scenario","LOCAL"),"run":run,"parent_pid":std::process::id()})
}
fn child_reopen(dir: &Path, want: Value, out: &Path) {
    let expect = out.join("expected.json");
    record(&expect, want);
    let result = Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "p001_child_reopen",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("SF_P001_CHILD_SOURCE", dir)
        .env("SF_P001_CHILD_EXPECT", &expect)
        .env("SF_P001_CHILD_OUTPUT", out)
        .output()
        .unwrap();
    fs::write(out.join("child.stdout.txt"), &result.stdout).unwrap();
    fs::write(out.join("child.stderr.txt"), &result.stderr).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
#[ignore = "explicit Windows persistence evidence run; see P001_PERSISTENCE.md"]
fn p001_child_reopen() {
    let Some(dir) = std::env::var_os("SF_P001_CHILD_SOURCE") else {
        return;
    };
    let output = PathBuf::from(std::env::var_os("SF_P001_CHILD_OUTPUT").unwrap());
    let want = read(Path::new(
        &std::env::var_os("SF_P001_CHILD_EXPECT").unwrap(),
    ));
    assert_ne!(
        want["parent_pid"].as_u64().unwrap(),
        std::process::id() as u64
    );
    let archive = read_project_archive(Path::new(&dir)).unwrap();
    let mut h = Harness::new();
    let t = h.import(Path::new(&dir));
    h.finish_one();
    let draft = h.imported(&t);
    let _: EditCommit = ok(h.adopt(&t, &draft, false));
    assert_eq!(h.object("project", "P1"), want["project"]);
    assert_eq!(h.object("scenario", "LOCAL"), want["scenario"]);
    let old: RunResult = serde_json::from_value(want["run"].clone()).unwrap();
    assert_eq!(json!(h.full(&old.id)), want["run"]);
    let r = rev(&mut h);
    let _: EditCommit = ok(set_height(&mut h, r, 4.0));
    let t = h.run("LOCAL");
    h.finish_one();
    let new = h.completed_run(&t);
    assert_ne!(new.id, old.id);
    assert_eq!(new.input_ref.project_revision.0.value(), r + 1);
    assert_eq!(json!(h.full(&old.id)), want["run"]);
    let resaved = output.join("child-resaved");
    fs::create_dir_all(&resaved).unwrap();
    save_ok(&mut h, &resaved);
    let after = read_project_archive(&resaved).unwrap();
    assert_eq!(after.project.revision.0.value(), r + 1);
    record(
        &output.join("child-result.json"),
        json!({"pid":std::process::id(),"parent_pid":want["parent_pid"],"old_run":old.id,"new_run":new.id,"old_revision":r,"saved_revision":r+1,"documents_read":archive.manifest.documents.len(),"included_files_read":archive.files.len(),"old_run_unchanged":true,"local_files_only":true}),
    );
}

struct DenyCreate {
    path: PathBuf,
    sid: String,
}
impl DenyCreate {
    fn new(path: PathBuf) -> Self {
        let sid = std::env::var("SF_P001_SID").unwrap();
        let out = Command::new("icacls")
            .arg(&path)
            .arg("/deny")
            .arg(format!("*{sid}:(AD)"))
            .output()
            .unwrap();
        assert!(out.status.success(), "{:?}", out);
        Self { path, sid }
    }
}
impl Drop for DenyCreate {
    fn drop(&mut self) {
        let status = Command::new("icacls")
            .arg(&self.path)
            .arg("/remove:d")
            .arg(format!("*{}", self.sid))
            .output()
            .unwrap();
        assert!(status.status.success());
    }
}

#[test]
#[ignore = "explicit Windows persistence evidence run; see P001_PERSISTENCE.md"]
#[cfg(feature = "nfr06-save-faults")]
fn p001_four_real_io_failure_exits_preserve_complete_old_archive_and_run() {
    for branch in ["F01", "F02", "F03", "F04"] {
        let out = evidence(branch);
        let (mut h, run) = with_run();
        let directory = out.join("project");
        fs::create_dir_all(&directory).unwrap();
        save_ok(&mut h, &directory);
        let old = read_project_archive(&directory).unwrap();
        let manifest = fs::read(directory.join("project.json")).unwrap();
        let want = expected(&mut h, &run);
        let base = rev(&mut h);
        let _: EditCommit = ok(set_height(&mut h, base, 2.0));
        let edited = h.object("project", "P1");
        let guard = if branch == "F01" {
            Some(DenyCreate::new(directory.join("data")))
        } else {
            None
        };
        let held = if branch == "F02" {
            Some(
                fs::OpenOptions::new()
                    .read(true)
                    .share_mode(1)
                    .open(directory.join("project.json"))
                    .unwrap(),
            )
        } else {
            None
        };
        if branch == "F03" || branch == "F04" {
            let plan = std::env::current_exe()
                .unwrap()
                .with_file_name("nfr06-save-fault.json");
            let consumed = plan.with_file_name("nfr06-save-fault.consumed.json");
            if consumed.exists() {
                fs::remove_file(&consumed).unwrap();
            }
            record(
                &plan,
                json!({"directory":directory,"branch":branch,"expected_project_id":old.manifest.project_id,"expected_revision":old.manifest.project_revision.0.value(),"expected_generation":old.manifest.generation.value()}),
            );
        }
        let r = rev(&mut h);
        let t = h.save(r, &directory);
        h.finish_one();
        let state = h.task(&t, true);
        drop(held);
        drop(guard);
        record(&out.join("failed-task.json"), json!(state));
        assert!(
            matches!(state.task.state, ComputationStatus::Failed),
            "{branch}: {state:?}"
        );
        assert!(state.result.is_none());
        let failure = state.failure.unwrap();
        assert!(
            matches!(failure.code, ErrorCode::IoFailure),
            "{branch}: {failure:?}"
        );
        assert!(h.context().project.unwrap().dirty);
        assert_eq!(h.object("project", "P1"), edited);
        assert_eq!(json!(h.full(&run.id)), json!(run));
        for (path, bytes) in &old.files {
            assert_eq!(
                &fs::read(directory.join(path)).unwrap(),
                bytes,
                "{branch}: old file {path}"
            );
        }
        let reopen = if branch == "F04" {
            assert!(read_project_archive(&directory).is_err());
            let retained = fs::read_dir(&directory)
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| p.is_file() && fs::read(p).unwrap() == manifest)
                .expect("actual retained old manifest");
            assert!(
                failure
                    .message
                    .contains(retained.file_name().unwrap().to_str().unwrap())
            );
            let recovery = out.join("retained-copy");
            fs::create_dir_all(&recovery).unwrap();
            copy(&directory, &recovery);
            fs::remove_dir(recovery.join("project.json")).unwrap();
            fs::copy(&retained, recovery.join("project.json")).unwrap();
            recovery
        } else {
            assert_eq!(fs::read(directory.join("project.json")).unwrap(), manifest);
            directory.clone()
        };
        let reopened = read_project_archive(&reopen).unwrap();
        assert_eq!(json!(reopened.project), json!(old.project));
        assert_eq!(reopened.files, old.files);
        child_reopen(&reopen, want, &out);
        let retry = out.join("save-edited-elsewhere");
        fs::create_dir_all(&retry).unwrap();
        save_ok(&mut h, &retry);
        assert_eq!(json!(read_project_archive(&retry).unwrap().project), edited);
        record(
            &out.join("result.json"),
            json!({"branch":branch,"failure":failure,"fault_handling_status":"pass","old_content_retention_status":"pass","project_health_status":if branch=="F04"{"fail"}else{"pass"},"normal_save_succeeded":false,"branch_acceptance_status":"pass","old_file_count":old.files.len(),"reopen_path":reopen,"edited_saved_elsewhere":true}),
        );
    }
}

fn authorize(h: &mut Harness, agent: &ObjectId) {
    h.host
        .register_external_session(HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            session_id: agent.clone(),
            granted_project: Some("P1".to_owned().try_into().unwrap()),
            allowed_operations: vec![
                OperationName::ApplyCommands,
                OperationName::GetObject,
                OperationName::GetContext,
            ],
            allowed_definition_drafts: vec![],
            allow_view_control: false,
            selected_file_tokens: vec![],
            edit_grants: vec![HostEditGrant {
                id: id("p001-height-grant"),
                scope: TaskScope::Edit(EditScope::Project("P1".to_owned().try_into().unwrap())),
                branches: vec![HostEditBranch::PutObject],
                objects: vec![HostObjectEditGrant {
                    target: ObjectRef {
                        kind: ObjectKind::Device,
                        id: id("ASM-01"),
                    },
                    fields: vec![HostFieldGrant {
                        pointer: "/height_m".into(),
                        element_id: None,
                    }],
                    can_remove: false,
                }],
                creates: vec![],
                source_versions: vec![],
                reusable_resources: vec![],
                created_objects: vec![],
            }],
        })
        .unwrap();
}
#[test]
#[ignore = "explicit Windows persistence evidence run; see P001_PERSISTENCE.md"]
fn p001_concurrent_draft_requests_conflict_in_both_orders_and_keep_old_run() {
    for agent_first in [false, true] {
        let out = evidence(if agent_first {
            "agent-first"
        } else {
            "ui-first"
        });
        let (mut h, run) = with_run();
        let agent = id("p001-agent");
        authorize(&mut h, &agent);
        let base = rev(&mut h);
        let device = h.object("device", "ASM-01");
        let ui = h.ui.clone();
        let host = Arc::new(Mutex::new(h));
        let barrier = Arc::new(Barrier::new(2));
        let (sent, received) = std::sync::mpsc::channel();
        let mut received = Some(received);
        let mut handles = vec![];
        for first in [true, false] {
            let is_agent = if first { agent_first } else { !agent_first };
            let who = if is_agent { agent.clone() } else { ui.clone() };
            let host = host.clone();
            let gate = barrier.clone();
            let mut d = device.clone();
            d["height_m"] = json!(if is_agent { 3.0 } else { 2.0 });
            let done = if first { Some(sent.clone()) } else { None };
            let wait = if first { None } else { received.take() };
            handles.push(std::thread::spawn(move || {
                gate.wait();
                if let Some(rx) = wait {
                    rx.recv().unwrap();
                }
                let reply = host.lock().unwrap().call_as(
                    &who,
                    true,
                    "apply_commands",
                    json!({"base_revision":base,"commands":[put("device",d)]}),
                );
                if let Some(tx) = done {
                    tx.send(()).unwrap();
                }
                reply
            }));
            // Receiver is moved only into the second request.
            if !first {
                break;
            }
        }
        let replies: Vec<_> = handles.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(replies[0]["outcome"]["status"], "ok");
        assert_eq!(replies[1]["outcome"]["data"]["code"], "RevisionConflict");
        let mut h = Arc::try_unwrap(host).ok().unwrap().into_inner().unwrap();
        assert_eq!(rev(&mut h), base + 1);
        assert_eq!(
            h.object("device", "ASM-01")["height_m"],
            json!(if agent_first { 3.0 } else { 2.0 })
        );
        assert_eq!(json!(h.full(&run.id)), json!(run));
        let before = h.object("project", "P1");
        let mut device = h.object("device", "ASM-01");
        device["height_m"] = json!(7.0);
        let rejected=h.apply(base+1,json!([put("device",device),{"kind":"remove_object","data":{"kind":"device","id":"does-not-exist"}}]));
        assert_eq!(rejected["outcome"]["status"], "error");
        assert_eq!(h.object("project", "P1"), before);
        let save_dir = out.join("project");
        fs::create_dir_all(&save_dir).unwrap();
        save_ok(&mut h, &save_dir);
        let want = expected(&mut h, &run);
        child_reopen(&save_dir, want, &out);
        record(
            &out.join("result.json"),
            json!({"agent_first":agent_first,"replies":replies,"batch_atomic":true,"old_run_unchanged":true,"saved_revision":base+1}),
        );
    }
}

#[test]
#[ignore = "explicit Windows persistence evidence run; see P001_PERSISTENCE.md"]
fn p001_save_completion_keeps_new_draft_dirty_history_does_not_rewind_run_or_disk() {
    let out = evidence("save-history-overlap");
    let (mut h, run) = with_run();
    let dir = out.join("project");
    fs::create_dir_all(&dir).unwrap();
    save_ok(&mut h, &dir);
    let _: EditCommit = ok(set_height(&mut h, 1, 2.0));
    let pending = h.save(2, &dir);
    let work = h.host.drain_io_work().pop().unwrap();
    let _: EditCommit = ok(set_height(&mut h, 2, 3.0));
    h.host.finish_io(work.run());
    let state = h.task(&pending, true);
    assert!(matches!(state.task.state, ComputationStatus::Completed));
    assert!(h.context().project.unwrap().dirty);
    assert_eq!(
        read_project_archive(&dir)
            .unwrap()
            .project
            .revision
            .0
            .value(),
        2
    );
    let disk = fs::read(dir.join("project.json")).unwrap();
    let stale = h.apply(2, json!([{"kind":"undo"}]));
    assert_eq!(stale["outcome"]["data"]["code"], "RevisionConflict");
    for (r, cmd) in [(3, "undo"), (4, "redo"), (5, "undo")] {
        let _: EditCommit = ok(h.apply(r, json!([{"kind":cmd}])));
        assert_eq!(rev(&mut h), r + 1);
        assert_eq!(json!(h.full(&run.id)), json!(run));
        assert_eq!(fs::read(dir.join("project.json")).unwrap(), disk);
    }
    let _: EditCommit = ok(set_height(&mut h, 6, 4.0));
    let rejected = h.apply(7, json!([{"kind":"redo"}]));
    assert_eq!(rejected["outcome"]["status"], "error");
    assert_eq!(rev(&mut h), 7);
    let cancel = h.save(7, &dir);
    let _: CancellationState = ok(h.call(
        "cancel_task",
        json!({"task_id":cancel.id,"action":"cancel"}),
    ));
    for work in h.host.drain_io_work() {
        h.host.finish_io(work.run());
    }
    assert_eq!(fs::read(dir.join("project.json")).unwrap(), disk);
    assert!(h.context().project.unwrap().dirty);
    save_ok(&mut h, &dir);
    let moved = out.join("moved-project");
    fs::create_dir_all(&moved).unwrap();
    copy(&dir, &moved);
    let want = expected(&mut h, &run);
    child_reopen(&moved, want, &out);
    record(
        &out.join("result.json"),
        json!({"frozen_save_revision":2,"new_draft_stayed_dirty":true,"stale_undo":stale,"redo_invalidated":rejected,"old_run_unchanged":true,"final_revision":7,"cancel_task":h.task(&cancel,true)}),
    );
}

#[test]
#[ignore = "explicit Windows persistence evidence run; see P001_PERSISTENCE.md"]
fn p001_invalid_imports_preserve_active_draft_old_run_and_source_files() {
    let out = evidence("invalid-imports");
    let (mut h, run) = with_run();
    let valid = out.join("valid");
    fs::create_dir_all(&valid).unwrap();
    save_ok(&mut h, &valid);
    let before = h.object("project", "P1");
    for kind in ["unknown-format", "missing-module"] {
        let dir = out.join(kind);
        fs::create_dir_all(&dir).unwrap();
        copy(&valid, &dir);
        if kind == "unknown-format" {
            let mut root = read(&dir.join("project.json"));
            root["schema_version"] = json!("99.0");
            write(&dir.join("project.json"), &root);
        } else {
            let root = read(&dir.join("project.json"));
            let file = root["data"]["documents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["file_ref"]["kind"] == "module")
                .unwrap()["file_ref"]["path"]
                .as_str()
                .unwrap();
            fs::remove_file(dir.join(file)).unwrap();
        }
        let root_before = fs::read(dir.join("project.json")).unwrap();
        let t = h.import(&dir);
        h.finish_one();
        let state = h.task(&t, false);
        assert!(
            matches!(state.task.state, ComputationStatus::Failed),
            "{state:?}"
        );
        assert!(state.result.is_none());
        assert_eq!(h.object("project", "P1"), before);
        assert_eq!(json!(h.full(&run.id)), json!(run));
        assert_eq!(fs::read(dir.join("project.json")).unwrap(), root_before);
        record(&out.join(format!("{kind}.json")), json!(state));
    }
}

