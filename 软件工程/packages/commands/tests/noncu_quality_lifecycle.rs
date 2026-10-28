//! P1-05 frozen actual quality cases, edited inputs and separate-process reopen.
#[path = "fixtures/production/host_support.rs"]
mod host;
#[path = "../../simulation/tests/noncu_quality_support.rs"]
mod quality;
use host::*;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn execute(h: &mut Harness) -> RunResult {
    let t = h.run("QUALITY-ORIGINAL");
    assert!(h.task(&t, true).result.is_none());
    h.finish_one();
    let r = h.completed_run(&t);
    assert!(r.whole_window_computed);
    assert!(matches!(r.evidence_origin, EvidenceOrigin::ActualSoftware));
    r
}
fn frozen(h: &mut Harness, r: &RunResult, kind: &str, id: &str) -> Value {
    let v:DomainObject=ok(h.call("get_object",json!({"object_id":{"kind":kind,"id":id},"source":{"kind":"run_snapshot","data":{"run_id":r.id,"at_ms":0}},"node_index":null})));
    json!(v)["data"].clone()
}
fn history(h: &mut Harness, r: &RunResult, times: &[u64]) -> Value {
    let selected = json!(
        r.jobs
            .iter()
            .map(|j| json!({"kind":"job","id":j.id}))
            .collect::<Vec<_>>()
    );
    let mut rows = vec![];
    for at in times {
        let mut objects = vec![];
        for selector in [json!([]), selected.clone()] {
            let v:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":selector,"page":{"after_index":null,"limit":1000}}}})));
            let RunView::Objects(p) = v else {
                panic!("actual history")
            };
            assert!(p.next_index.is_none());
            objects.extend(p.objects);
        }
        rows.push(json!({"at_ms":at,"objects":objects}));
    }
    json!(rows)
}
fn events(h: &mut Harness, r: &RunResult) -> Value {
    let mut after: Option<SafeUint> = None;
    let mut rows = vec![];
    loop {
        let v:RunView=ok(h.call("get_run",json!({"run_id":r.id,"query":{"kind":"events","data":{"interval":null,"page":{"after_index":after,"limit":37}}}})));
        let RunView::Events(page) = v else {
            panic!("actual events")
        };
        rows.extend(page.events);
        after = page.next_index;
        if after.is_none() {
            break;
        }
    }
    assert_eq!(json!(rows), json!(r.events));
    json!(rows)
}
fn save(h: &mut Harness, path: &Path) {
    fs::create_dir_all(path).unwrap();
    let base = h.context().project.unwrap().revision.0.value();
    let t = h.save(base, path);
    h.finish_one();
    assert!(matches!(
        h.task(&t, true).task.state,
        ComputationStatus::Completed
    ));
}
fn observed_item(history: &Value, at: u64, id: &str) -> Value {
    history
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["at_ms"] == at)
        .unwrap()["objects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "item" && o["data"]["id"] == id)
        .unwrap()["data"]
        .clone()
}
fn check(name: &str, r: &RunResult, states: &Value, edited: bool) {
    let v = json!(r);
    let blocked = !edited && ["hold", "full"].contains(&name);
    let invalid = !edited && name == "invalid";
    let packages = if blocked || invalid || name == "isolation" {
        1
    } else {
        2
    };
    let scraps = if blocked { 0 } else { 1 };
    quality::invariant(&v, packages, scraps);
    assert_eq!(quality::accepted(&v), packages as u64);
    let third = quality::assembly(2);
    let second = quality::assembly(1);
    if name == "orders" {
        assert_eq!(v["orders"].as_array().unwrap().len(), 1);
        assert_eq!(v["orders"][0]["accepted"]["value"]["coefficient"], 2);
        assert_eq!(v["orders"][0]["outstanding"]["value"]["coefficient"], 1);
        assert!(v["orders"][0]["completion_ms"].is_null());
        assert!(r.acceptance_events.iter().all(|e| e.order_id.is_some()));
        if edited {
            let at95 = observed_item(states, 95000, &third);
            assert_eq!(at95["lifecycle"], "in_process");
            assert_eq!(at95["holder"]["kind"], "job");
            assert_eq!(quality::original::named(&v, &third)["completed_reworks"], 0);
        }
    } else {
        assert!(r.acceptance_events.iter().all(|e| e.order_id.is_none()));
    }
    let pending = observed_item(states, 80000, &third);
    assert_eq!(pending["quality"], "pending");
    if blocked {
        assert_eq!(pending["holder"]["kind"], "job");
        assert_eq!(
            quality::original::intervals(&v, "W-Q"),
            vec![(20000, 30000), (40000, 50000), (60000, 120000)]
        );
    } else if name != "hold" {
        assert_eq!(
            pending["holder"],
            json!({"kind":"location","id":"pending-store"})
        );
        let recheck = observed_item(states, 80000, &second);
        assert_eq!(recheck["lifecycle"], "in_process");
        assert_eq!(recheck["holder"]["kind"], "job");
    }
    if invalid {
        assert_eq!(
            v["limitations"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| i["code"] == "simulation.quality_decision_unapplied")
                .count(),
            3
        );
        assert_eq!(quality::original::named(&v, &third)["quality"], "pending");
    }
    if !blocked && !invalid {
        assert_eq!(
            quality::original::named(&v, &second)["lifecycle"],
            "scrapped"
        );
        assert_eq!(
            quality::original::named(&v, &second)["holder"]["id"],
            "quarantine"
        );
        assert_eq!(
            quality::original::named(&v, &second)["completed_reworks"],
            1
        );
    }
    if name == "full" {
        assert_eq!(
            quality::original::named(&v, "existing-pending-slot")["quality"],
            "pending"
        );
        assert_eq!(
            quality::original::named(&v, "existing-pending-slot")["holder"]["id"],
            "pending-store"
        );
    }
    if name == "isolation" {
        let item = quality::original::named(&v, &third);
        assert_eq!(item["quality"], "rejected");
        assert_eq!(item["lifecycle"], "available");
        assert_eq!(item["completed_reworks"], 1);
        assert_eq!(
            item["holder"]["kind"],
            if edited { "location" } else { "job" }
        );
        if edited {
            assert_eq!(item["holder"]["id"], "quarantine");
        }
        assert!(
            quality::original::intervals(&v, "W-Q")
                .contains(&(115000, if edited { 125000 } else { 140000 }))
        );
    }
    if name != "isolation" && (edited || name == "original" || name == "orders") {
        let final_ms = if edited && ["original", "orders"].contains(&name) {
            110000
        } else {
            100000
        };
        assert_eq!(
            r.acceptance_events.last().unwrap().at_ms.0.value(),
            final_ms
        );
    }
}

fn host_input(name: &str) -> Value {
    if name != "invalid" {
        return quality::profile(name);
    }
    // Unknown exact module versions are a separate import rejection below.
    // This run declares only existing modules and deliberately wrong scopes.
    let mut v = quality::fixture();
    let release = v["scenario"]["initial_conditions"]["declared_quality_decisions"][0].clone();
    let scrap = v["scenario"]["initial_conditions"]["declared_quality_decisions"][1].clone();
    let mut rows = vec![scrap];
    for kind in ["early", "wrong-source", "wrong-item"] {
        let mut d = release.clone();
        d["id"] = json!(format!("invalid-{kind}"));
        match kind {
            "early" => d["effective_at_ms"] = json!(10000),
            "wrong-source" => d["decision_source"]["data"] = json!("QC-2"),
            "wrong-item" => d["item_ref"] = json!(quality::assembly(0)),
            _ => unreachable!(),
        }
        rows.push(d);
    }
    v["scenario"]["initial_conditions"]["declared_quality_decisions"] = json!(rows);
    v
}

#[test]
fn unknown_inspection_version_import_is_rejected_without_replacing_active_project() {
    let mut h = Harness::from_value(quality::fixture());
    let project = h.object("project", "P1");
    let scenario = h.object("scenario", "QUALITY-ORIGINAL");
    let bad = quality::profile("invalid");
    let directory = h.tree.directory("unknown-inspection-version");
    seed(&directory, &bad);
    let t = h.import(&directory);
    h.finish_one();
    let failure = h.task(&t, false);
    assert!(matches!(failure.task.state, ComputationStatus::Failed));
    assert!(matches!(
        failure.failure.as_ref().unwrap().code,
        ErrorCode::MissingReference
    ));
    assert_eq!(h.object("project", "P1"), project);
    assert_eq!(h.object("scenario", "QUALITY-ORIGINAL"), scenario);
    assert!(h.context().run_summaries.is_empty());
    write(
        &quality::out().join("artifacts/unknown-version-import.json"),
        &json!({"passed":true,"input":bad,"task":failure,"project_and_scenario_unchanged":true,"Run_enqueued":false}),
    );
}

#[test]
fn quality_modes_decisions_and_order_shortage_survive_saved_input_edits_and_new_processes() {
    let base = quality::out().join(format!("artifacts/host-{}", std::process::id()));
    fs::create_dir_all(&base).unwrap();
    let mut results = vec![];
    for name in ["original", "hold", "full", "orders", "invalid", "isolation"] {
        let dir = base.join(name);
        fs::create_dir(&dir).unwrap();
        let input = host_input(name);
        let before = input.clone();
        let mut h = Harness::from_value(input.clone());
        let original_project = h.object("project", "P1");
        let original_scenario = h.object("scenario", "QUALITY-ORIGINAL");
        let mut bad = original_scenario.clone();
        bad["initial_conditions"]["declared_quality_decisions"] =
            quality::fixture()["scenario"]["initial_conditions"]["declared_quality_decisions"]
                .clone();
        bad["initial_conditions"]["declared_quality_decisions"][0]["disposition"] =
            json!("automatic_pass");
        let revision = h.context().project.unwrap().revision.0.value();
        let raw = json!({"api_version":"0.1","request_id":"quality-invalid-enum","project_id":"P1","scope":{"kind":"project"},"operation":"apply_commands","params":{"base_revision":revision,"commands":[put("scenario",bad)]}});
        let refusal = serde_json::from_value::<ApiRequest>(raw.clone())
            .err()
            .expect("invalid disposition must fail at the shared API parser");
        write(
            &dir.join("invalid-enum-rejected.json"),
            &json!({"boundary":"shared_api_parse_before_Host","request":raw,"error":refusal.to_string(),"dispatched":false}),
        );
        assert_eq!(h.object("project", "P1"), original_project);
        assert_eq!(h.object("scenario", "QUALITY-ORIGINAL"), original_scenario);
        assert!(h.context().run_summaries.is_empty());
        let old = execute(&mut h);
        let mut times = vec![
            0, 19999, 20000, 30000, 40000, 49999, 50000, 60000, 69999, 70000, 75000, 79999, 80000,
            84999, 85000, 89999, 90000, 95000, 99999, 100000, 110000, 120000,
        ];
        if name == "isolation" {
            times.extend([124999, 125000, 129999, 130000, 139999, 140000]);
        }
        let old_history = history(&mut h, &old, &times);
        check(name, &old, &old_history, false);
        let old_events = events(&mut h, &old);
        let old_project = frozen(&mut h, &old, "project", "P1");
        let old_scenario = frozen(&mut h, &old, "scenario", "QUALITY-ORIGINAL");
        let next = quality::edited(name, &input);
        let mut scenario = next["scenario"].clone();
        scenario["revision"] = h.object("scenario", "QUALITY-ORIGINAL")["revision"].clone();
        let revision = h.context().project.unwrap().revision.0.value();
        scenario["project_revision"] = json!(revision);
        let mut edits = vec![put("scenario", scenario)];
        if name == "full" || name == "isolation" {
            let destination = if name == "full" {
                "pending-store"
            } else {
                "quarantine"
            };
            let location = next["project"]["locations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["id"] == destination)
                .unwrap()
                .clone();
            edits.push(put("location", location));
        }
        let _: EditCommit = ok(h.apply(revision, json!(edits)));
        assert_eq!(json!(h.full(&old.id)), json!(old));
        assert_eq!(frozen(&mut h, &old, "project", "P1"), old_project);
        assert_eq!(
            frozen(&mut h, &old, "scenario", "QUALITY-ORIGINAL"),
            old_scenario
        );
        let edited_project = h.object("project", "P1");
        let edited_scenario = h.object("scenario", "QUALITY-ORIGINAL");
        let saved = dir.join("saved");
        save(&mut h, &saved);
        let moved = dir.join("receiver/moved");
        fs::create_dir_all(&moved).unwrap();
        copy(&saved, &moved);
        assert_eq!(input, before);
        write(
            &dir.join("packet.json"),
            &json!({"name":name,"input":input,"edited_input":next,"old":old,"old_history":old_history,"old_events":old_events,"old_project":old_project,"old_scenario":old_scenario,"edited_project":edited_project,"edited_scenario":edited_scenario,"times":times,"moved":moved}),
        );
        drop(h);
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--ignored",
                "--exact",
                "noncu_quality_child_reopens",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SF_NONCU_QUALITY_PACKET", dir.join("packet.json"));
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
        let value = read(&dir.join("result.json"));
        assert_eq!(value["passed"], true);
        results.push(value);
    }
    write(
        &base.join("result.json"),
        &json!({"passed":true,"cases":results,"new_processes":6,"Native":false,"human_acceptance":null,"whole_task_done":false}),
    );
    write(
        &quality::out().join("lifecycle-result.json"),
        &json!({"passed":true,"result":base.join("result.json").strip_prefix(quality::root()).unwrap()}),
    );
}

#[test]
#[ignore = "Private new-process worker; parent provides a frozen packet"]
fn noncu_quality_child_reopens() {
    let packet = PathBuf::from(std::env::var_os("SF_NONCU_QUALITY_PACKET").unwrap());
    let dir = packet.parent().unwrap();
    let p = read(&packet);
    let name = p["name"].as_str().unwrap();
    let times: Vec<u64> = serde_json::from_value(p["times"].clone()).unwrap();
    let old: RunResult = serde_json::from_value(p["old"].clone()).unwrap();
    let mut h = Harness::from_seed(Sandbox::new(), Path::new(p["moved"].as_str().unwrap()));
    assert_eq!(h.object("project", "P1"), p["edited_project"]);
    assert_eq!(
        h.object("scenario", "QUALITY-ORIGINAL"),
        p["edited_scenario"]
    );
    assert_eq!(json!(h.full(&old.id)), p["old"]);
    assert_eq!(frozen(&mut h, &old, "project", "P1"), p["old_project"]);
    assert_eq!(
        frozen(&mut h, &old, "scenario", "QUALITY-ORIGINAL"),
        p["old_scenario"]
    );
    assert_eq!(history(&mut h, &old, &times), p["old_history"]);
    assert_eq!(events(&mut h, &old), p["old_events"]);
    let new = execute(&mut h);
    let new_history = history(&mut h, &new, &times);
    let new_events = events(&mut h, &new);
    check(name, &new, &new_history, true);
    assert_ne!(old.id, new.id);
    assert!(old.input_ref.project_revision.0.value() < new.input_ref.project_revision.0.value());
    assert_eq!(json!(h.full(&old.id)), p["old"]);
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
        frozen(&mut final_host, &old, "scenario", "QUALITY-ORIGINAL"),
        p["old_scenario"]
    );
    assert_eq!(
        frozen(&mut final_host, &new, "project", "P1"),
        p["edited_project"]
    );
    assert_eq!(
        frozen(&mut final_host, &new, "scenario", "QUALITY-ORIGINAL"),
        p["edited_scenario"]
    );
    assert_eq!(history(&mut final_host, &old, &times), p["old_history"]);
    assert_eq!(history(&mut final_host, &new, &times), new_history);
    assert_eq!(events(&mut final_host, &old), p["old_events"]);
    assert_eq!(events(&mut final_host, &new), new_events);
    write(
        &dir.join("result.json"),
        &json!({"name":name,"passed":true,"pid":std::process::id(),"old":old,"new":new,"new_history":new_history,"times":times,"two_run_project":both,"Native":false}),
    );
}
