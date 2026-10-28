//! Preserve a real Core UI-purpose record; this is not actual human acceptance.
use super::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
#[test]
fn actual_host_design_four_formats_preserve_existing_purpose_artifact_and_draft_history_without_authorizing_unknown_result()
 {
    let mut h = professional(false);
    let ack = acknowledgment(
        "W5-REPORT-PURPOSE",
        "ART-U",
        "PACKAGE",
        json!(["design_reference"]),
    );
    let _: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"record_artifact_acknowledgment","data":ack}]),
    ));
    let old_artifact = h.object("artifact", "ART-U");
    let original_title = h.object("project", "P1")["title"].clone();
    let p = h.object("project", "P1");
    let _:EditCommit=ok(h.apply(2,json!([{"kind":"set_project_settings","data":{"title":"W5 explicit design export input","time_reference":p["time_reference"],"generation_context":p["generation_context"],"sources":p["sources"]}}])));
    let before = h.object("project", "P1");
    assert_eq!(before["acknowledgments"].as_array().unwrap().len(), 1);
    assert_eq!(before["bindings"], json!([]));
    let dirty = h.context().project.unwrap().dirty;
    let assets =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/architecture-proof/assets");
    h.host
        .set_evaluation_resources(sf_reporting::EvaluationResources {
            cjk_font: Some(fs::read(assets.join("SourceHanSansCN-Regular.otf")).unwrap()),
            font_license: Some(fs::read(assets.join("LICENSE.SourceHanSans.txt")).unwrap()),
            files: BTreeMap::new(),
        });
    let parent = h.tree.directory("design-evaluation");
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::ExportEvaluation, parent.clone())
        .unwrap();
    let task:TaskHandle=ok(h.call("export_evaluation",json!({"revision":3,"report_mode":"design_description","scenario_refs":[{"scenario_id":"LOCAL","revision":3}],"run_refs":[],"metric_scope":{"keys":[],"objects":[],"acceptance":[]},"formats":["pdf","html","csv","json"],"selected_destination_ref":selected})));
    assert!(h.task(&task, true).result.is_none());
    h.finish_one();
    let state = h.task(&task, true);
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "{state:?}"
    );
    let TaskOutput::Evaluation(output) = state.result.unwrap() else {
        panic!("actual typed report")
    };
    assert!(output.context.run_evaluations.is_empty());
    assert!(output.context.run_refs.is_empty());
    assert!(
        output
            .context
            .conclusions
            .iter()
            .any(|v| v.contains("W5-REPORT-PURPOSE"))
    );
    assert_eq!(h.object("project", "P1"), before);
    assert_eq!(h.object("artifact", "ART-U"), old_artifact);
    assert_eq!(h.context().project.unwrap().dirty, dirty);
    let report = output
        .files
        .iter()
        .find(|f| matches!(f.kind, DocumentKind::Evaluation))
        .unwrap();
    let folder = parent
        .join(report.path.as_str())
        .parent()
        .unwrap()
        .to_owned();
    assert!(
        fs::read(folder.join("report.pdf"))
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert_eq!(
        read(&folder.join("report.json"))["data"],
        serde_json::to_value(&output.context).unwrap()
    );
    let mut opened = Harness::from_seed(Sandbox::new(), &folder.join("project"));
    assert_eq!(
        opened.object("project", "P1")["acknowledgments"],
        before["acknowledgments"]
    );
    assert_eq!(opened.object("artifact", "ART-U"), old_artifact);
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"undo"}])));
    assert_eq!(h.object("project", "P1")["title"], original_title);
    assert_eq!(
        h.object("project", "P1")["acknowledgments"],
        before["acknowledgments"]
    );
    assert_eq!(h.object("artifact", "ART-U"), old_artifact);
    if let Some(base) = std::env::var_os("W5_HOST_EVALUATION_OUTPUT") {
        let target = PathBuf::from(base).join("design");
        fs::create_dir_all(&target).unwrap();
        copy(&folder, &target);
        write(
            &target.join("actual-task-output.json"),
            &serde_json::to_value(output).unwrap(),
        );
    }
}
