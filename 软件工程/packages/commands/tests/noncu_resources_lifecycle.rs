//! P1-03 actual Host/Engine lifecycle. Run with local TMP/TEMP and isolated target.
#[path = "../../simulation/tests/noncu_resources_support.rs"]
mod cases;
#[path = "fixtures/whole_ix/checks.rs"]
mod frozen_checks;
#[path = "fixtures/production/host_support.rs"]
mod host;
use host::*;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn result(h: &mut Harness, name: &str) -> RunResult {
    let task = h.run(name);
    assert!(h.task(&task, true).result.is_none());
    h.finish_one();
    let r = h.completed_run(&task);
    assert!(r.whole_window_computed);
    assert!(matches!(r.evidence_origin, EvidenceOrigin::ActualSoftware));
    r
}
fn frozen(h: &mut Harness, run: &RunResult, kind: &str, name: &str) -> Value {
    let v:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":kind,"id":name},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":0}},"node_index":null})));
    json!(v)["data"].clone()
}
fn events(h: &mut Harness, r: &RunResult) -> Value {
    let mut cursor: Option<SafeUint> = None;
    let mut rows = vec![];
    loop {
        let page:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"events","data":{"interval":null,"page":{"after_index":cursor,"limit":100}}}})));
        let RunView::Events(page) = page else {
            panic!("actual events")
        };
        rows.extend(page.events);
        cursor = page.next_index;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(json!(rows), json!(r.events));
    json!(rows)
}
fn history(h: &mut Harness, r: &RunResult, moments: &[u64]) -> Value {
    let mut rows = vec![];
    // Frozen ObjectsAt supports Items and Jobs. Transport states remain in the
    // actual FullResult/events; cargo holder history uses the same Item history.
    let selectors = vec![
        json!([]),
        json!(
            r.jobs
                .iter()
                .map(|j| json!({"kind":"job","id":j.id}))
                .collect::<Vec<_>>()
        ),
    ];
    for at in moments {
        let mut objects = vec![];
        for selected in &selectors {
            let page:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":selected,"page":{"after_index":null,"limit":1000}}}})));
            let RunView::Objects(page) = page else {
                panic!("actual objects")
            };
            assert!(page.next_index.is_none());
            objects.extend(page.objects);
        }
        rows.push(json!({"at_ms":at,"objects":objects}));
    }
    json!(rows)
}
fn save(h: &mut Harness, path: &Path) {
    fs::create_dir_all(path).unwrap();
    let rev = h.context().project.unwrap().revision.0.value();
    let task = h.save(rev, path);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
}
fn check(name: &str, r: &RunResult, edited: bool) {
    let v = json!(r);
    if name == "logistics" {
        frozen_checks::ix01(&v, if edited { "delayed_pcb" } else { "baseline" });
        return;
    }
    assert!(r.transport.is_empty());
    assert!(
        r.acceptance_events.is_empty(),
        "local no-order profile is not an invented acceptance goal"
    );
    let j = &v["jobs"][0];
    assert_eq!(cases::original::held_ms(&v, "W-C"), 10000);
    if name == "original" {
        let unload = if edited { 50000 } else { 40000 };
        assert_eq!(j["finished_at_ms"], unload + 5000);
        assert_eq!(
            cases::work(&v),
            vec![
                ("load".into(), 0, 5000),
                ("auto".into(), 5000, 35000),
                ("unload".into(), unload, unload + 5000)
            ]
        );
        assert_eq!(
            cases::original::intervals(&v, "CAP-CNC"),
            vec![(0, unload + 5000)]
        );
    } else {
        let policy = if name == "pause" {
            "pause_resume"
        } else {
            "non_preemptive"
        };
        assert_eq!(
            cases::work(&v),
            cases::expected_work(policy, if edited { 90000 } else { 80000 })
        );
        assert_eq!(
            j["finished_at_ms"],
            if edited { json!(95000) } else { Value::Null }
        );
        assert_eq!(cases::arrival_count(&v), 1);
        assert_eq!(
            cases::original::intervals(&v, "CAP-CNC"),
            vec![(10000, if edited { 95000 } else { 100000 })]
        );
        assert_eq!(
            j["state"],
            if edited {
                "completed"
            } else {
                "waiting_output"
            }
        );
        assert_eq!(cases::original::physical(&v, "part").len(), 1);
    }
}
fn source(name: &str) -> Value {
    match name {
        "original" => cases::original::phased(),
        "pause" => cases::combined("pause_resume", 0),
        "nonpreemptive" => cases::combined("non_preemptive", 0),
        "logistics" => {
            serde_json::from_str(include_str!("fixtures/production/p202-ix01-baseline.json"))
                .unwrap()
        }
        _ => unreachable!(),
    }
}

#[test]
fn arrivals_calendars_phase_waits_and_logistics_survive_actual_host_edit_and_fresh_process() {
    let output = cases::root().join(format!(
        ".work/delivery/artifacts/host-{}",
        std::process::id()
    ));
    fs::create_dir_all(&output).unwrap();
    let mut results = vec![];
    for name in ["original", "pause", "nonpreemptive", "logistics"] {
        let dir = output.join(name);
        fs::create_dir(&dir).unwrap();
        let input = source(name);
        let source_before = input.clone();
        let mut h = Harness::from_value(input.clone());
        let scenario = input["scenario"]["id"].as_str().unwrap();
        let old = result(&mut h, scenario);
        check(name, &old, false);
        let old_project = frozen(&mut h, &old, "project", "P1");
        let old_scenario = frozen(&mut h, &old, "scenario", scenario);
        let moments: Vec<u64> = if name == "logistics" {
            vec![
                0, 60000, 240000, 260000, 369999, 370000, 419999, 420000, 425000, 440000, 570000,
                600000,
            ]
        } else {
            vec![
                0, 4999, 5000, 9999, 10000, 15000, 19999, 20000, 30000, 35000, 39999, 40000, 45000,
                55000, 65000, 70000, 79999, 80000, 84999, 85000, 90000, 95000, 100000,
            ]
        };
        let old_history = history(&mut h, &old, &moments);
        let old_events = events(&mut h, &old);
        if name == "logistics" {
            let later: Value = serde_json::from_str(include_str!(
                "fixtures/production/p202-ix01-delayed-pcb.json"
            ))
            .unwrap();
            let mut next = later["scenario"].clone();
            next["id"] = json!(scenario);
            next["revision"] = old_scenario["revision"].clone();
            let base = h.context().project.unwrap().revision.0.value();
            next["project_revision"] = json!(base);
            let _: EditCommit = ok(h.apply(base, json!([put("scenario", next)])));
        } else {
            let mut worker = h.object("calendar", "WORKER");
            worker["intervals"][1]["start_ms"] =
                json!(if name == "original" { 50000 } else { 90000 });
            let mut changes = vec![put("calendar", worker)];
            if name != "original" {
                let mut location = h.object("location", "finished");
                location["capacity"]["value"]["coefficient"] = json!(1);
                changes.push(put("location", location));
            }
            let base = h.context().project.unwrap().revision.0.value();
            let _: EditCommit = ok(h.apply(base, json!(changes)));
        }
        assert_eq!(json!(h.full(&old.id)), json!(old));
        assert_eq!(frozen(&mut h, &old, "project", "P1"), old_project);
        assert_eq!(frozen(&mut h, &old, "scenario", scenario), old_scenario);
        let edited_project = h.object("project", "P1");
        let edited_scenario = h.object("scenario", scenario);
        let saved = dir.join("saved");
        save(&mut h, &saved);
        let moved = dir.join("receiver/moved");
        fs::create_dir_all(&moved).unwrap();
        copy(&saved, &moved);
        assert_eq!(input, source_before);
        write(
            &dir.join("packet.json"),
            &json!({"name":name,"input":input,"scenario":scenario,"old":old,"old_project":old_project,"old_scenario":old_scenario,"history":old_history,"events":old_events,"moments":moments,"edited_project":edited_project,"edited_scenario":edited_scenario,"moved":moved}),
        );
        drop(h);
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--ignored",
                "--exact",
                "noncu_resources_child_reopens",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SF_NONCU_RESOURCES_PACKET", dir.join("packet.json"));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            child.creation_flags(0x08000000);
        }
        let done = child.output().unwrap();
        fs::write(dir.join("child.stdout.txt"), done.stdout).unwrap();
        fs::write(dir.join("child.stderr.txt"), &done.stderr).unwrap();
        assert!(
            done.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&done.stderr)
        );
        let child = read(&dir.join("result.json"));
        assert_eq!(child["passed"], true);
        results.push(child);
    }
    write(
        &output.join("result.json"),
        &json!({"passed":true,"cases":results,"new_processes":4,"Native":false,"whole_task_done":false,"human_acceptance":null}),
    );
    write(
        &cases::root().join(".work/delivery/lifecycle-result.json"),
        &json!({"passed":true,"result":output.join("result.json").strip_prefix(cases::root()).unwrap()}),
    );
}

#[test]
#[ignore = "Private fresh-process entry; parent supplies a frozen packet"]
fn noncu_resources_child_reopens() {
    let packet_path = PathBuf::from(std::env::var_os("SF_NONCU_RESOURCES_PACKET").unwrap());
    let dir = packet_path.parent().unwrap();
    let p = read(&packet_path);
    let name = p["name"].as_str().unwrap();
    let mut h = Harness::from_seed(Sandbox::new(), Path::new(p["moved"].as_str().unwrap()));
    let old: RunResult = serde_json::from_value(p["old"].clone()).unwrap();
    let scenario = p["scenario"].as_str().unwrap();
    let moments: Vec<u64> = serde_json::from_value(p["moments"].clone()).unwrap();
    assert_eq!(h.object("project", "P1"), p["edited_project"]);
    assert_eq!(h.object("scenario", scenario), p["edited_scenario"]);
    assert_eq!(json!(h.full(&old.id)), p["old"]);
    assert_eq!(frozen(&mut h, &old, "project", "P1"), p["old_project"]);
    assert_eq!(
        frozen(&mut h, &old, "scenario", scenario),
        p["old_scenario"]
    );
    assert_eq!(history(&mut h, &old, &moments), p["history"]);
    assert_eq!(events(&mut h, &old), p["events"]);
    let new = result(&mut h, scenario);
    check(name, &new, true);
    let new_history = history(&mut h, &new, &moments);
    if name == "logistics" {
        let cargo = &new
            .transport
            .iter()
            .max_by_key(|t| t.departed_ms.as_ref().map(|s| s.0.value()))
            .unwrap()
            .cargo[0];
        for (at, kind) in [(419999, "transport"), (420000, "location")] {
            let point = new_history
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["at_ms"] == at)
                .unwrap();
            let item = point["objects"]
                .as_array()
                .unwrap()
                .iter()
                .find(|o| o["kind"] == "item" && o["data"]["id"] == json!(cargo))
                .unwrap();
            assert_eq!(item["data"]["holder"]["kind"], kind);
            if at == 420000 {
                assert_eq!(item["data"]["holder"]["id"], "shell-buffer");
            }
        }
    }
    assert_ne!(old.id, new.id);
    assert!(old.input_ref.project_revision.0.value() < new.input_ref.project_revision.0.value());
    assert_eq!(json!(h.full(&old.id)), p["old"]);
    assert_eq!(frozen(&mut h, &old, "project", "P1"), p["old_project"]);
    let both = dir.join("two-runs");
    save(&mut h, &both);
    drop(h);
    let mut final_host = Harness::from_seed(Sandbox::new(), &both);
    assert_eq!(json!(final_host.full(&old.id)), p["old"]);
    assert_eq!(json!(final_host.full(&new.id)), json!(new));
    assert_eq!(
        frozen(&mut final_host, &old, "project", "P1"),
        p["old_project"]
    );
    assert_eq!(
        frozen(&mut final_host, &old, "scenario", scenario),
        p["old_scenario"]
    );
    assert_eq!(
        frozen(&mut final_host, &new, "project", "P1"),
        p["edited_project"]
    );
    assert_eq!(
        frozen(&mut final_host, &new, "scenario", scenario),
        p["edited_scenario"]
    );
    assert_eq!(history(&mut final_host, &old, &moments), p["history"]);
    assert_eq!(history(&mut final_host, &new, &moments), new_history);
    assert_eq!(events(&mut final_host, &old), p["events"]);
    write(
        &dir.join("result.json"),
        &json!({"name":name,"passed":true,"pid":std::process::id(),"old":old,"new":new,"moments":moments,"new_history":new_history,"two_run_project":both,"Native":false}),
    );
}
