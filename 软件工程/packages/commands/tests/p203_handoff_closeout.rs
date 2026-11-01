//! P2-03 actual export, relocation and two generations of OS-process receivers.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use support::*;
fn out() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.work/cu-closeout-20261023/root-P203-reception/delivery")
}
fn run(h: &mut Harness, s: &str) -> RunResult {
    let t = h.run(s);
    h.finish_one();
    h.completed_run(&t)
}
fn compare(h: &mut Harness, a: &RunId, b: &RunId) -> Value {
    let c:RunComparison=ok(h.call("compare_runs",json!({"run_ids":[a,b],"metric_scope":{"keys":["accepted_quantity","completion_time","waiting_time","unit_cost"],"objects":[],"acceptance":[]}})));
    json!(c)
}
fn export(h: &mut Harness, path: &Path, runs: &[RunId], scenarios: Value, zip: bool) -> Value {
    if !zip {
        fs::create_dir_all(path).unwrap();
    }
    let token = if zip {
        h.host
            .select_file(&h.ui, FilePurpose::ExportPackage, path.to_owned())
            .unwrap()
    } else {
        h.host
            .select_directory(&h.ui, FilePurpose::ExportPackage, path.to_owned())
            .unwrap()
    };
    let r = h.context().project.unwrap().revision;
    let t:TaskHandle=ok(h.call("export_package",json!({"revision":r,"selected_destination_ref":token,"package_scope":{"purpose":"handoff_project","scenarios":scenarios,"run_ids":runs,"artifact_ids":[],"module_refs":[],"include_redistributable_assets":false}})));
    h.finish_one();
    let state = h.task(&t, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    json!(state)
}
fn open(path: &Path, zip: bool) -> Harness {
    if !zip {
        return Harness::from_seed(Sandbox::new(), path);
    }
    let ui = id("p203-receiver");
    let mut h = Harness {
        host: ProjectHost::new(id("p203-host"), ui.clone()),
        ui,
        tree: Sandbox::new(),
    };
    let selected = h
        .host
        .select_file(&h.ui, FilePurpose::ImportPackage, path.to_owned())
        .unwrap();
    let t: TaskHandle = ok(h.call_as(
        &h.ui.clone(),
        false,
        "import_package",
        json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
    ));
    h.finish_one();
    let draft = h.imported(&t);
    let _: EditCommit = ok(h.adopt(&t, &draft, false));
    h
}
fn child(mode: &str, packet: &Path) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--ignored",
        "--exact",
        "p203_receiver_worker",
        "--nocapture",
        "--test-threads=1",
    ])
    .env("P203_PACKET", packet)
    .env("P203_MODE", mode);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let r = cmd.output().unwrap();
    let p = packet.parent().unwrap();
    fs::write(p.join(format!("{mode}.stdout.txt")), &r.stdout).unwrap();
    fs::write(p.join(format!("{mode}.stderr.txt")), &r.stderr).unwrap();
    assert!(
        r.status.success(),
        "{mode}: {} {}",
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr)
    );
}
#[test]
fn actual_directory_and_zip_handoff_have_new_process_edit_save_and_readback() {
    let root = out().join(format!("handoff-{}", std::process::id()));
    assert!(
        !root.exists(),
        "preserve earlier evidence; choose a fresh output for rerun"
    );
    fs::create_dir_all(&root).unwrap();
    write(&out().join("latest.json"), &json!({"directory":root}));
    let mut h = Harness::new();
    let a = run(&mut h, "LOCAL");
    assert_eq!(a.acceptance_events.len(), 1);
    let same = run(&mut h, "LOCAL");
    let strict = compare(&mut h, &a.id, &same.id);
    assert_eq!(strict["pairs"][0]["status"], "strict");
    let mut s = h.object("scenario", "LOCAL");
    s["id"] = json!("WITH-PCB");
    s["revision"] = json!(1);
    s["project_revision"] = json!(h.context().project.unwrap().revision);
    let mut pcb = s["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("pcb-002");
    s["arrivals"] = json!([{"id":"PCB-ARRIVAL","at_ms":90000,"destination":"pcbs-A","items":[pcb],"release_policy":"unchecked_allowed"}]);
    let rev = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(rev, json!([put("scenario", s)])));
    let b = run(&mut h, "WITH-PCB");
    assert_eq!(
        b.acceptance_events
            .iter()
            .map(|x| x.at_ms.0.value())
            .collect::<Vec<_>>(),
        [75000, 165000]
    );
    let controlled = compare(&mut h, &a.id, &b.id);
    assert_eq!(controlled["pairs"][0]["status"], "controlled_changes");
    write(
        &root.join("comparisons.json"),
        &json!({"strict":strict,"controlled":controlled}),
    );
    export(
        &mut h,
        &root.join("design-only"),
        &[],
        json!(["LOCAL"]),
        false,
    );
    let manifest = read(&root.join("design-only/project.json"));
    assert_eq!(
        manifest["data"]["restart_policy"],
        "restart_from_initial_input"
    );
    assert!(
        !manifest["data"]["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["file_ref"]["kind"] == "run_result")
    );
    let runs = vec![a.id.clone(), same.id.clone(), b.id.clone()];
    for zip in [false, true] {
        let case = root.join(if zip { "zip" } else { "directory" });
        fs::create_dir(&case).unwrap();
        let src = case.join(if zip { "author.sfproj" } else { "author" });
        let state = export(&mut h, &src, &runs, json!(["LOCAL", "WITH-PCB"]), zip);
        let moved = case.join(if zip {
            "receiver-renamed.sfproj"
        } else {
            "receiver-moved"
        });
        assert!(src.starts_with(&root) && moved.starts_with(&root));
        fs::rename(&src, &moved).unwrap();
        assert!(!src.exists());
        let packet = case.join("packet.json");
        write(
            &packet,
            &json!({"parent_pid":std::process::id(),"zip":zip,"moved":moved,"author":src,"a":a,"same":same,"b":b,"comparison":controlled,"export_state":state,"project":h.object("project","P1")}),
        );
        child("edit", &packet);
        child("read", &packet);
    }
    assert_eq!(json!(h.full(&a.id)), json!(a));
    assert_eq!(json!(h.full(&b.id)), json!(b));
    write(
        &root.join("result.json"),
        &json!({"passed":true,"new_OS_processes":4,"paths":["directory/edited","zip/edited"],"Native":false,"human_acceptance":null}),
    );
}
#[test]
#[ignore = "Explicit subprocess worker only; ordinary cargo test needs no environment"]
fn p203_receiver_worker() {
    let packet = PathBuf::from(std::env::var_os("P203_PACKET").unwrap());
    let mode = std::env::var("P203_MODE").unwrap();
    let p = read(&packet);
    assert_ne!(p["parent_pid"], json!(std::process::id()));
    let base = packet.parent().unwrap();
    let a: RunResult = serde_json::from_value(p["a"].clone()).unwrap();
    let b: RunResult = serde_json::from_value(p["b"].clone()).unwrap();
    let same: RunResult = serde_json::from_value(p["same"].clone()).unwrap();
    if mode == "edit" {
        let mut h = open(
            Path::new(p["moved"].as_str().unwrap()),
            p["zip"].as_bool().unwrap(),
        );
        assert_eq!(json!(h.full(&a.id)), p["a"]);
        assert_eq!(json!(h.full(&same.id)), p["same"]);
        assert_eq!(json!(h.full(&b.id)), p["b"]);
        assert_eq!(compare(&mut h, &a.id, &b.id), p["comparison"]);
        let mut s = h.object("scenario", "WITH-PCB");
        s["arrivals"][0]["at_ms"] = json!(120000);
        let rev = h.context().project.unwrap().revision.0.value();
        let _: EditCommit = ok(h.apply(rev, json!([put("scenario", s)])));
        let c = run(&mut h, "WITH-PCB");
        assert_ne!(c.id, b.id);
        assert_eq!(c.window.start_ms.0.value(), 0);
        assert_eq!(
            c.acceptance_events
                .iter()
                .map(|x| x.at_ms.0.value())
                .collect::<Vec<_>>(),
            [75000, 195000]
        );
        assert_eq!(json!(h.full(&a.id)), p["a"]);
        assert_eq!(json!(h.full(&b.id)), p["b"]);
        let target = base.join("edited");
        fs::create_dir(&target).unwrap();
        let rev = h.context().project.unwrap().revision.0.value();
        let t = h.save(rev, &target);
        h.finish_one();
        let saved = h.task(&t, true);
        assert!(
            matches!(saved.task.state, ComputationStatus::Completed),
            "{saved:?}"
        );
        let ccomparison = compare(&mut h, &b.id, &c.id);
        let exported = export(
            &mut h,
            &base.join("edited.sfproj"),
            &[a.id.clone(), same.id.clone(), b.id.clone(), c.id.clone()],
            json!(["LOCAL", "WITH-PCB"]),
            true,
        );
        write(
            &base.join("edited-result.json"),
            &json!({"pid":std::process::id(),"new":c,"comparison":ccomparison,"saved":saved,"exported":exported,"project":h.object("project","P1"),"passed":true}),
        );
    } else {
        assert_eq!(mode, "read");
        let e = read(&base.join("edited-result.json"));
        assert_ne!(e["pid"], json!(std::process::id()));
        let c: RunResult = serde_json::from_value(e["new"].clone()).unwrap();
        for zip in [false, true] {
            let path = base.join(if zip { "edited.sfproj" } else { "edited" });
            let mut h = open(&path, zip);
            assert_eq!(json!(h.full(&a.id)), p["a"]);
            assert_eq!(json!(h.full(&same.id)), p["same"]);
            assert_eq!(json!(h.full(&b.id)), p["b"]);
            assert_eq!(json!(h.full(&c.id)), e["new"]);
            assert_eq!(
                h.object("scenario", "WITH-PCB")["arrivals"][0]["at_ms"],
                120000
            );
            assert_eq!(h.object("project", "P1"), e["project"]);
            assert_eq!(compare(&mut h, &b.id, &c.id), e["comparison"]);
        }
        write(
            &base.join("read-result.json"),
            &json!({"passed":true,"pid":std::process::id(),"runs_verified":4,"directory_and_ZIP":true,"Native":false}),
        );
    }
}
