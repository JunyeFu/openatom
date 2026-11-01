mod common;
use common::{assert_fixed, business, host::*, p1};
use serde_json::json;
use sf_commands::EditCommit;
use sf_domain::{ComputationStatus, RunResult};
use std::{fs, path::PathBuf};

fn main() {
    let mut args = std::env::args().skip(1);
    let evidence = PathBuf::from(args.next().unwrap());
    let case = args.next().unwrap();
    let original: RunResult =
        serde_json::from_value(read(&evidence.join("original-run.json"))).unwrap();
    let frozen = serde_json::to_value(&original).unwrap();
    let mut h = Harness::from_seed(Sandbox::new(), &evidence.join("moved"));
    assert_eq!(serde_json::to_value(h.full(&original.id)).unwrap(), frozen);
    same_business(
        h.object("project", "P1"),
        read(&evidence.join("original-project.json")),
    );
    let scenario = h.object("scenario", &case);
    same_business(
        scenario.clone(),
        read(&evidence.join("original-scenario.json")),
    );
    assert_fixed(&case, &h.full(&original.id));

    let task = h.run(&case);
    h.finish_one();
    let rerun = h.completed_run(&task);
    assert_ne!(rerun.id, original.id);
    assert_fixed(&case, &rerun);
    let rerun_json = serde_json::to_value(&rerun).unwrap();
    assert_eq!(business(rerun_json.clone()), business(frozen.clone()));
    write(&evidence.join("fresh-rerun.json"), &rerun_json);

    // Q35: change only the declared quality inputs and start a new Run.
    // The original topology, durations, stock, orders and old Run are untouched.
    let changed_case = if case == "A" { "Q" } else { "A" };
    let mut changed = scenario.clone();
    changed["quality_presets"] = p1::parallel(changed_case)["scenario"]["quality_presets"].clone();
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("scenario", changed.clone())])));
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([{"kind":"undo"}])));
    same_business(h.object("scenario", &case), scenario);
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([{"kind":"redo"}])));
    same_business(h.object("scenario", &case), changed);
    let task = h.run(&case);
    h.finish_one();
    let changed_run = h.completed_run(&task);
    assert_ne!(changed_run.id, original.id);
    assert_ne!(changed_run.id, rerun.id);
    assert!(
        changed_run.input_ref.scenario_revision.0.value()
            > original.input_ref.scenario_revision.0.value()
    );
    let changed_summary = assert_fixed(changed_case, &changed_run);
    write(
        &evidence.join("changed-run.json"),
        &serde_json::to_value(&changed_run).unwrap(),
    );
    write(
        &evidence.join("changed-scenario.json"),
        &h.object("scenario", &case),
    );
    assert_eq!(serde_json::to_value(h.full(&original.id)).unwrap(), frozen);
    assert_eq!(serde_json::to_value(h.full(&rerun.id)).unwrap(), rerun_json);

    let saved = evidence.join("after-edit-saved");
    fs::create_dir(&saved).unwrap();
    let base = h.context().project.unwrap().revision.0.value();
    let save = h.save(base, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    drop(h);
    let mut reopened = Harness::from_seed(Sandbox::new(), &saved);
    assert_eq!(
        serde_json::to_value(reopened.full(&original.id)).unwrap(),
        frozen
    );
    assert_eq!(
        serde_json::to_value(reopened.full(&rerun.id)).unwrap(),
        rerun_json
    );
    assert_eq!(
        serde_json::to_value(reopened.full(&changed_run.id)).unwrap(),
        serde_json::to_value(&changed_run).unwrap()
    );
    same_business(
        reopened.object("scenario", &case),
        read(&evidence.join("changed-scenario.json")),
    );
    write(
        &evidence.join("fresh-process-receipt.json"),
        &json!({
            "pid":std::process::id(),"original_case":case,"changed_case":changed_case,
            "old_run_preserved":true,"same_input_rerun_business_equal":true,
            "undo_redo_quality_input_verified":true,"saved_run_count":3,
            "original_summary":assert_fixed(&case,&original),"changed_summary":changed_summary,
            "fresh_run_ids":[original.id,rerun.id,changed_run.id]
        }),
    );
}
