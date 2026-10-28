mod common;
use common::{host::*, *};
use serde_json::json;
use sf_commands::EditCommit;
use sf_domain::{ComputationStatus, RunResult};
use std::{fs, path::PathBuf};

fn main() {
    let directory = PathBuf::from(std::env::args().nth(1).unwrap());
    let packet = read(&directory.join("packet.json"));
    let case = packet["case"].as_str().unwrap();
    let scenario = packet["scenario"].as_str().unwrap();
    let original: RunResult =
        serde_json::from_value(read(&directory.join("original-run.json"))).unwrap();
    if std::env::args().nth(2).as_deref() == Some("final") {
        let mut h = Harness::from_seed(Sandbox::new(), &directory.join("three-runs"));
        let changed_case = if case == "A" { "B" } else { "A" };
        for (name, expected_case) in [("original",case),("repeated",case),("changed",changed_case)] {
            let run: RunResult = serde_json::from_value(read(&directory.join(format!("{name}-run.json")))).unwrap();
            assert_eq!(json!(h.full(&run.id)),json!(run));
            check(expected_case,&run);
            let expected = if name == "changed" {read(&directory.join("changed-history.json"))} else {packet["history"].clone()};
            assert_eq!(history(&mut h,&run),expected);
        }
        write(&directory.join("final-receipt.json"),&json!({"passed":true,"pid":std::process::id(),"runs_read":3,"history_exact":true}));
        return;
    }
    let mut h = Harness::from_seed(Sandbox::new(), &directory.join("moved"));
    assert_eq!(json!(h.full(&original.id)), json!(original));
    check(case, &original);
    assert_eq!(
        frozen(&mut h, &original, "project", "P1"),
        packet["project"]
    );
    assert_eq!(
        frozen(&mut h, &original, "scenario", scenario),
        packet["scenario_input"]
    );
    assert_eq!(history(&mut h, &original), packet["history"]);
    let repeated = execute(&mut h, scenario);
    check(case, &repeated);
    assert_ne!(repeated.id, original.id);
    assert_eq!(business(json!(repeated)), business(json!(original)));
    write(&directory.join("repeated-run.json"), &json!(repeated));
    let comparison = compare_completion(&mut h, case, &original, &repeated);
    write(&directory.join("original-comparison.json"), &comparison);

    let old_input = h.object("scenario", scenario);
    let mut changed = old_input.clone();
    let changed_case = if case == "A" { "B" } else { "A" };
    if case == "missing" {
        changed["arrivals"] = source("A")["scenario"]["arrivals"].clone();
    } else {
        for task in changed["production_tasks"].as_array_mut().unwrap() {
            task["priority"] = json!(if task["id"] == format!("TASK-{changed_case}") {
                1
            } else {
                0
            });
        }
    }
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("scenario", changed.clone())])));
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([{"kind":"undo"}])));
    same_business(h.object("scenario", scenario), old_input);
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([{"kind":"redo"}])));
    same_business(h.object("scenario", scenario), changed);
    let new = execute(&mut h, scenario);
    check(changed_case, &new);
    assert_ne!(new.id, original.id);
    assert_ne!(new.id, repeated.id);
    assert!(
        new.input_ref.scenario_revision.0.value() > original.input_ref.scenario_revision.0.value()
    );
    assert_eq!(json!(h.full(&original.id)), json!(original));
    assert_eq!(history(&mut h, &original), packet["history"]);
    assert_eq!(
        compare_completion(&mut h, case, &original, &repeated),
        comparison
    );
    write(&directory.join("changed-run.json"), &json!(new));
    let new_history = history(&mut h, &new);
    write(&directory.join("changed-history.json"), &new_history);
    let saved = directory.join("three-runs");
    fs::create_dir(&saved).unwrap();
    let base = h.context().project.unwrap().revision.0.value();
    let task = h.save(base, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    drop(h);
    let mut reopened = Harness::from_seed(Sandbox::new(), &saved);
    for run in [&original, &repeated, &new] {
        assert_eq!(json!(reopened.full(&run.id)), json!(run));
    }
    assert_eq!(history(&mut reopened, &original), packet["history"]);
    assert_eq!(history(&mut reopened, &new), new_history);
    assert_eq!(
        compare_completion(&mut reopened, case, &original, &repeated),
        comparison
    );
    assert_eq!(
        frozen(&mut reopened, &original, "scenario", scenario),
        packet["scenario_input"]
    );
    write(
        &directory.join("child-receipt.json"),
        &json!({"passed":true,"pid":std::process::id(),"case":case,"changed_case":changed_case,
        "old_run_and_history_unchanged":true,"same_input_business_equal":true,"undo_redo":true,
        "three_run_ids":[original.id,repeated.id,new.id],"public_unfinished_completion_unknown":case=="missing"}),
    );
}
