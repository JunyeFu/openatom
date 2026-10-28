//! Whole P0-03 consumers. Imported fixtures remain declarations, never producers.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use support::*;
#[path = "../../../tests/planner/acceptance/professional_evaluation_host.rs"]
mod professional_evaluation_host;

fn documents() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/production/professional-documents.json"
    ))
    .unwrap()
}

fn acknowledgment(name: &str, artifact: &str, subject: &str, scope: Value) -> Value {
    json!({"id":name,"artifact_ref":artifact,"accepted_subject":{"id":subject,"revision":"r1"},
        "accepted_scope":scope,"decision_source":{"kind":"human_statement","data":"explicit finite test purpose; not calculation evidence"},
        "conditions":[],"recorded_at_iso8601":null})
}

fn professional(with_bindings: bool) -> Harness {
    let mut value = fixture();
    let declaration: Value =
        serde_json::from_str(include_str!("../../packaging/consumer-device.design.json")).unwrap();
    value["definitions"][5]["parameters"] = json!(
        declaration["data"]["definition"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["id"] == "packaging_job_id" || p["id"] == "packaging_job_revision")
            .cloned()
            .collect::<Vec<_>>()
    );
    value["project"]["devices"][1]["configuration"] = json!({
        "packaging_job_id":{"kind":"string","value":"JOB-U"},
        "packaging_job_revision":{"kind":"integer","value":1}
    });
    let mut other = value["project"]["products"][0].clone();
    other["id"] = json!("OTHER");
    value["project"]["products"]
        .as_array_mut()
        .unwrap()
        .push(other);
    if with_bindings {
        let mut consumer = value["project"]["devices"][1].clone();
        consumer["id"] = json!("PACK-02");
        consumer["configuration"]["packaging_job_id"]["value"] = json!("JOB-OTHER");
        value["project"]["devices"]
            .as_array_mut()
            .unwrap()
            .push(consumer);
        value["project"]["acknowledgments"] = json!([
            acknowledgment(
                "IMPORTED-ACK-U",
                "ART-U",
                "PACKAGE",
                json!(["design_reference"])
            ),
            acknowledgment(
                "IMPORTED-ACK-OTHER",
                "ART-OTHER",
                "OTHER",
                json!(["design_reference"])
            )
        ]);
        value["project"]["bindings"] = json!([
            {"id":"DECLARED-U","consumer_ref":{"kind":"device","id":"PACK-01"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},"artifact_ref":"ART-U","expected_subject_revision":"r1","required_payload":[],"requested_scope":["design_reference"],"acceptance_ref":"IMPORTED-ACK-U","mode":"declared_process"},
            {"id":"DECLARED-OTHER","consumer_ref":{"kind":"device","id":"PACK-02"},"operation_ref":{"id":"fixture.pack","version":"0.1.0"},"artifact_ref":"ART-OTHER","expected_subject_revision":"r1","required_payload":[],"requested_scope":["design_reference"],"acceptance_ref":"IMPORTED-ACK-OTHER","mode":"declared_process"}
        ]);
    }
    let data = documents();
    value["project"]["artifact_refs"] = json!(["ART-U", "ART-F", "ART-OTHER"]);
    let tree = Sandbox::new();
    let directory = tree.directory("professional-seed");
    seed(&directory, &value);
    let mut manifest = read(&directory.join("project.json"));
    for job in data["jobs"].as_array().unwrap() {
        let name = job["id"].as_str().unwrap();
        let path = format!("{name}.json");
        write(
            &directory.join(&path),
            &json!({"schema_version":"1.0","data":job}),
        );
        manifest["data"]["documents"]
            .as_array_mut()
            .unwrap()
            .push(document(
                "professional_input",
                "packaging_job",
                "packaging_job",
                name,
                &path,
                json!(1),
            ));
    }
    for result in data["results"].as_array().unwrap() {
        let name = result["artifact_id"].as_str().unwrap();
        let path = format!("{name}-result.json");
        write(
            &directory.join(&path),
            &json!({"schema_version":"1.0","data":result}),
        );
        manifest["data"]["documents"]
            .as_array_mut()
            .unwrap()
            .push(document(
                "professional_result",
                "packaging_result",
                "artifact",
                name,
                &path,
                Value::Null,
            ));
    }
    for artifact in data["artifacts"].as_array().unwrap() {
        let name = artifact["id"].as_str().unwrap();
        let path = format!("{name}.json");
        write(
            &directory.join(&path),
            &json!({"schema_version":"1.0","data":artifact}),
        );
        manifest["data"]["documents"]
            .as_array_mut()
            .unwrap()
            .push(document(
                "artifact_metadata",
                "artifact_record",
                "artifact",
                name,
                &path,
                Value::Null,
            ));
    }
    write(&directory.join("project.json"), &manifest);
    let ui = id("whole-w5-professional-ui");
    let mut h = Harness {
        host: ProjectHost::new(id("whole-w5-professional"), ui.clone()),
        ui,
        tree,
    };
    let handle = h.import(&directory);
    h.finish_one();
    let draft = h.imported(&handle);
    let _: EditCommit = ok(h.adopt(&handle, &draft, true));
    h
}

fn adoption() -> Value {
    json!({"artifact_id":"ART-U","acceptance_ref":"ACK-U","consumer_ref":{"kind":"device","id":"PACK-01"},
        "operation_ref":{"id":"fixture.pack","version":"0.1.0"},
        "requested_scope":["forming","opening","linear_extraction","net_space","manufacturing"],"base_revision":1})
}

#[test]
fn unknown_and_failed_professional_objects_roundtrip_without_turning_imported_data_into_pass() {
    let mut h = professional(false);
    let expected = documents();
    for artifact in expected["artifacts"].as_array().unwrap() {
        assert_eq!(
            h.object("artifact", artifact["id"].as_str().unwrap()),
            *artifact
        );
    }
    let before = h.object("project", "P1");
    assert!(before["acknowledgments"].as_array().unwrap().is_empty());
    assert!(before["bindings"].as_array().unwrap().is_empty());
    let directory = h.tree.directory("saved");
    let copied = h.tree.directory("relocated");
    let save = h.save(1, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&directory, &copied);
    let handle = h.import(&copied);
    h.finish_one();
    let draft = h.imported(&handle);
    let _: EditCommit = ok(h.adopt(&handle, &draft, true));
    for artifact in expected["artifacts"].as_array().unwrap() {
        assert_eq!(
            h.object("artifact", artifact["id"].as_str().unwrap()),
            *artifact
        );
    }
    assert_eq!(h.object("project", "P1"), before);
}

#[test]
fn a_real_ui_acknowledgment_records_a_purpose_but_never_changes_outcome_or_creates_a_binding() {
    let mut h = professional(false);
    let before = h.object("artifact", "ART-U");
    let ack = acknowledgment("ACK-U", "ART-U", "PACKAGE", json!(["design_reference"]));
    let committed: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"record_artifact_acknowledgment","data":ack.clone()}]),
    ));
    assert_eq!(committed.new_revision.0.value(), 2);
    assert!(!committed.history.can_undo);
    let project = h.object("project", "P1");
    assert_eq!(project["acknowledgments"], json!([ack]));
    assert!(project["bindings"].as_array().unwrap().is_empty());
    assert_eq!(h.object("artifact", "ART-U"), before);
    let snapshot = h.snapshot();
    error(h.apply(2, json!([{"kind":"undo"}])));
    assert_eq!(h.snapshot(), snapshot);
    let mut request = adoption();
    request["base_revision"] = json!(2);
    let failure = error(h.call("adopt_artifact", request));
    assert!(!matches!(failure.code, ErrorCode::ComputationFailed));
    assert_eq!(h.snapshot(), snapshot);
    assert_eq!(h.object("artifact", "ART-U"), before);
}

fn caller(
    session: &ObjectId,
    branches: Vec<HostEditBranch>,
    target: Option<ObjectRef>,
) -> HostCallContext {
    HostCallContext {
        origin: CallerOrigin::ExternalAgent,
        allow_view_control: false,
        session_id: session.clone(),
        granted_project: Some(ProjectId::try_from("P1".to_owned()).unwrap()),
        allowed_operations: vec![
            OperationName::GetContext,
            OperationName::GetObject,
            OperationName::ApplyCommands,
            OperationName::AdoptArtifact,
        ],
        allowed_definition_drafts: vec![],
        selected_file_tokens: vec![],
        edit_grants: vec![HostEditGrant {
            id: id("PROFESSIONAL-GRANT"),
            scope: TaskScope::Edit(EditScope::Project(
                ProjectId::try_from("P1".to_owned()).unwrap(),
            )),
            branches,
            objects: target
                .into_iter()
                .map(|target| HostObjectEditGrant {
                    target,
                    fields: vec![],
                    can_remove: false,
                })
                .collect(),
            creates: vec![],
            source_versions: vec![],
            reusable_resources: vec![],
            created_objects: vec![],
        }],
    }
}

#[test]
fn human_statement_text_in_an_external_request_cannot_create_ui_acknowledgment_or_partial_edits() {
    let mut h = professional(false);
    let session = id("EXTERNAL-HUMAN-TEXT");
    let mut caller = caller(
        &session,
        vec![HostEditBranch::RecordArtifactAcknowledgment],
        Some(ObjectRef {
            kind: ObjectKind::Artifact,
            id: id("ART-U"),
        }),
    );
    caller.edit_grants[0].objects[0].fields = vec![HostFieldGrant {
        pointer: "/accepted_scope".into(),
        element_id: None,
    }];
    h.host.register_external_session(caller).unwrap();
    let before = h.snapshot();
    let ack = acknowledgment("ACK-FAKE", "ART-U", "PACKAGE", json!(["design_reference"]));
    let failure = error(h.call_as(
        &session,
        true,
        "apply_commands",
        json!({"base_revision":1,"commands":[{
            "kind":"record_artifact_acknowledgment","data":ack
        }]}),
    ));
    assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    assert_eq!(h.snapshot(), before);
    assert!(h.host.drain_io_work().is_empty());
}

#[test]
fn adoption_needs_its_own_branch_and_exact_consumer_without_granting_put_or_human_authority() {
    let mut h = professional(false);
    let before = h.snapshot();
    for (name, branches, target) in [
        (
            "ORDINARY-PUT",
            vec![HostEditBranch::PutObject],
            Some(ObjectRef {
                kind: ObjectKind::Device,
                id: id("PACK-01"),
            }),
        ),
        (
            "ADOPT-NO-CONSUMER",
            vec![HostEditBranch::AdoptArtifact],
            None,
        ),
        (
            "ADOPT-WRONG-CONSUMER",
            vec![HostEditBranch::AdoptArtifact],
            Some(ObjectRef {
                kind: ObjectKind::Device,
                id: id("ASM-01"),
            }),
        ),
    ] {
        let session = id(name);
        h.host
            .register_external_session(caller(&session, branches, target))
            .unwrap();
        assert!(matches!(
            error(h.call_as(&session, true, "adopt_artifact", adoption())).code,
            ErrorCode::PermissionDenied
        ));
        assert_eq!(h.snapshot(), before);
    }
    let session = id("EXACT-ADOPT-CONSUMER");
    h.host
        .register_external_session(caller(
            &session,
            vec![HostEditBranch::AdoptArtifact],
            Some(ObjectRef {
                kind: ObjectKind::Device,
                id: id("PACK-01"),
            }),
        ))
        .unwrap();
    // This passes the narrow operation/consumer grant and still cannot turn an
    // unknown imported result into a complete calculated-nominal binding.
    let failure = error(h.call_as(&session, true, "adopt_artifact", adoption()));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert_eq!(h.snapshot(), before);
    let mut device = h.object("device", "PACK-01");
    device["title"] = json!("not a Put grant");
    assert!(matches!(
        error(h.call_as(
            &session,
            true,
            "apply_commands",
            json!({"base_revision":1,
        "commands":[put("device",device)]})
        ))
        .code,
        ErrorCode::PermissionDenied
    ));
    assert_eq!(h.snapshot(), before);
}

#[test]
fn exact_professional_job_versions_are_immutable_and_a_new_version_does_not_retarget_the_consumer()
{
    let mut h = professional(false);
    let before = h.snapshot();
    let job = documents()["jobs"][0].clone();
    let mut overwritten = job.clone();
    overwritten["materials"][0]["thickness_mm"] = json!(2);
    assert!(matches!(
        error(h.apply(1, json!([{"kind":"put_packaging_job","data":overwritten}]))).code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(h.snapshot(), before);
    let mut next = job.clone();
    next["revision"] = json!(2);
    next["materials"][0]["thickness_mm"] = json!(2.0);
    let commit: EditCommit =
        ok(h.apply(1, json!([{"kind":"put_packaging_job","data":next.clone()}])));
    assert_eq!(commit.new_revision.0.value(), 2);
    assert_eq!(
        h.object("device", "PACK-01")["configuration"]["packaging_job_revision"]["value"],
        1
    );
    assert_eq!(
        h.object("artifact", "ART-U")["input_ref"]["path"],
        "JOB-U.json"
    );
    let directory = h.tree.directory("two-job-versions");
    let task = h.save(2, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let manifest = read(&directory.join("project.json"));
    let mut versions = Vec::new();
    for entry in manifest["data"]["documents"].as_array().unwrap() {
        if entry["file_ref"]["kind"] == "packaging_job" && entry["object_ref"]["id"] == "JOB-U" {
            let data =
                read(&directory.join(entry["file_ref"]["path"].as_str().unwrap()))["data"].clone();
            if data["revision"] == 1 {
                assert_eq!(data, job);
            } else {
                assert_eq!(data, next);
            }
            versions.push(data["revision"].as_u64().unwrap());
        }
    }
    versions.sort();
    assert_eq!(versions, vec![1, 2]);
}

#[test]
fn product_r2_marks_only_its_affected_declaration_while_old_artifacts_acknowledgments_and_bindings_survive()
 {
    let mut h = professional(true);
    let old_project = h.object("project", "P1");
    let old_artifact = h.object("artifact", "ART-U");
    let other_artifact = h.object("artifact", "ART-OTHER");
    let mut product = h.object("product", "PACKAGE");
    product["revision"] = json!("r2");
    let _: EditCommit = ok(h.apply(1, json!([put("product", product)])));
    let issues = h.context().issues;
    assert!(issues.iter().any(|issue| {
        issue.code == "professional_inputs.declaration_subject_changed"
            && issue
                .object_ref
                .as_ref()
                .is_some_and(|target| target.id.as_str() == "PACK-01")
    }));
    assert!(!issues.iter().any(|issue| {
        issue.code == "professional_inputs.declaration_subject_changed"
            && issue
                .object_ref
                .as_ref()
                .is_some_and(|target| target.id.as_str() == "PACK-02")
    }));
    let current = h.object("project", "P1");
    assert_eq!(current["bindings"], old_project["bindings"]);
    assert_eq!(current["acknowledgments"], old_project["acknowledgments"]);
    assert_eq!(h.object("artifact", "ART-U"), old_artifact);
    assert_eq!(h.object("artifact", "ART-OTHER"), other_artifact);
    let directory = h.tree.directory("r2-draft");
    let copied = h.tree.directory("r2-reopen");
    let task = h.save(2, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&directory, &copied);
    let task = h.import(&copied);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, true));
    assert_eq!(h.object("artifact", "ART-U"), old_artifact);
    assert_eq!(
        h.object("project", "P1")["acknowledgments"],
        old_project["acknowledgments"]
    );
    assert_eq!(
        h.object("project", "P1")["bindings"],
        old_project["bindings"]
    );
}

#[test]
fn declared_job_selection_keeps_value_basis_and_history_together_without_changing_library_defaults()
{
    let mut h = professional(false);
    let change = json!({"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},
        "parameter_id":"packaging_job_revision","value":{"kind":"integer","value":2},"basis":"user_provided"}});
    let _: EditCommit = ok(h.apply(1, json!([change])));
    let selected = h.object("device", "PACK-01");
    assert_eq!(
        selected["configuration"]["packaging_job_revision"]["value"],
        2
    );
    let project = h.object("project", "P1");
    let source = project["parameter_sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| {
            source["target"] == json!({"kind":"device","id":"PACK-01"})
                && source["field_path"]
                    .as_str()
                    .is_some_and(|path| path.starts_with("/configuration/packaging_job_revision"))
        })
        .unwrap();
    assert_eq!(source["value"], json!({"kind":"integer","value":2}));
    assert_eq!(source["basis"], "user_provided");
    assert!(source["source_ref"].is_null());
    let _: EditCommit = ok(h.apply(2, json!([{"kind":"undo"}])));
    assert_eq!(
        h.object("device", "PACK-01")["configuration"]["packaging_job_revision"]["value"],
        1
    );
    assert_eq!(
        h.object("project", "P1")
            .get("parameter_sources")
            .map(|v| v.as_array().unwrap().len())
            .unwrap_or(0),
        0
    );
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"redo"}])));
    assert_eq!(
        h.object("project", "P1")["parameter_sources"],
        project["parameter_sources"]
    );
    let directory = h.tree.directory("parameter-basis");
    let task = h.save(4, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let manifest = read(&directory.join("project.json"));
    let entry = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["role"] == "module_definition"
                && entry["object_ref"]["id"] == "fixture.pack-station"
        })
        .unwrap();
    let module = read(&directory.join(entry["file_ref"]["path"].as_str().unwrap()));
    assert_eq!(
        module["data"]["definition"]["payload"]["data"]["configuration"],
        json!({})
    );
}

#[test]
fn an_actual_packaging_product_requires_ui_purpose_binding_then_survives_factory_run_reopen_and_product_review()
 {
    let mut h = professional(false);
    let inputs: Value = serde_json::from_str(include_str!(
        "fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    let mut packaging_job = inputs["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|input| input["id"] == "COVERAGE-restricted_folded_container")
        .unwrap()["job"]["data"]
        .clone();
    packaging_job["id"] = json!("JOB-U");
    packaging_job["revision"] = json!(2);
    packaging_job["subject_ref"] = json!({"id":"PACKAGE","revision":"r1"});
    packaging_job["conditions"]["product_envelope"]["subject"] =
        packaging_job["subject_ref"].clone();
    let packaging_job =
        serde_json::to_value(serde_json::from_value::<PackageStructureJob>(packaging_job).unwrap())
            .unwrap();
    let _: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"put_packaging_job","data":packaging_job.clone()}]),
    ));
    let _: EditCommit = ok(h.apply(
        2,
        json!([{"kind":"set_declared_parameter","data":{
        "target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision",
        "value":{"kind":"integer","value":2},"basis":"user_provided"}}]),
    ));
    let physical_before = h.object("scenario", "LOCAL")["initial_conditions"].clone();
    let task: TaskHandle = ok(h.call(
        "compute_packaging",
        json!({"packaging_job":packaging_job,
        "input_revision":3,"computation_budget":packaging_job["budget"]}),
    ));
    assert!(matches!(task.state, ComputationStatus::Queued));
    assert!(h.task(&task, true).result.is_none());
    h.finish_one();
    let completed = h.task(&task, true);
    assert!(
        matches!(completed.task.state, ComputationStatus::Completed),
        "actual packaging task did not complete: {completed:?}"
    );
    let calculated = match completed.result.unwrap() {
        TaskOutput::Packaging(result) => result,
        other => panic!("expected actual PackagingResult: {other:?}"),
    };
    let result = serde_json::to_value(&calculated).unwrap();
    assert_eq!(result["evidence_origin"], "actual_software");
    assert_eq!(
        result["outcome"], "pass",
        "the full selected input must support the requested nominal binding: {result:#}"
    );
    assert_eq!(result["job_ref"], json!({"id":"JOB-U","revision":2}));
    let artifact_id = calculated.artifact_id.as_str();
    let artifact = h.object("artifact", artifact_id);
    assert_eq!(artifact["evidence_origin"], "actual_software");
    assert_eq!(artifact["actual_outcome"], "pass");
    assert_eq!(artifact["subject_ref"], packaging_job["subject_ref"]);
    assert!(!result["manufacturing_files"].as_array().unwrap().is_empty());
    assert_eq!(
        h.object("scenario", "LOCAL")["initial_conditions"],
        physical_before
    );
    let project = h.object("project", "P1");
    assert!(project["acknowledgments"].as_array().unwrap().is_empty());
    assert!(project["bindings"].as_array().unwrap().is_empty());
    let scope = json!([
        "forming",
        "opening",
        "linear_extraction",
        "net_space",
        "manufacturing"
    ]);
    let mut request = adoption();
    request["artifact_id"] = json!(artifact_id);
    request["acceptance_ref"] = json!("ACK-ACTUAL");
    request["base_revision"] = serde_json::to_value(h.context().project.unwrap().revision).unwrap();
    let before_missing_ack = h.snapshot();
    error(h.call("adopt_artifact", request.clone()));
    assert_eq!(
        h.snapshot(),
        before_missing_ack,
        "real computation alone cannot acknowledge a purpose"
    );
    let base = h.context().project.unwrap().revision.0.value();
    let ack = acknowledgment("ACK-ACTUAL", artifact_id, "PACKAGE", scope);
    let ack_commit: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"record_artifact_acknowledgment","data":ack}]),
    ));
    assert!(!ack_commit.history.can_undo);
    assert!(
        h.object("project", "P1")["bindings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(h.object("artifact", artifact_id), artifact);
    request["base_revision"] = serde_json::to_value(ack_commit.new_revision).unwrap();
    let binding: BindingCommit = ok(h.call("adopt_artifact", request));
    let binding_value = serde_json::to_value(&binding.binding).unwrap();
    assert_eq!(binding_value["mode"], "calculated_nominal");
    assert_eq!(
        binding_value["consumer_ref"],
        json!({"kind":"device","id":"PACK-01"})
    );
    assert_eq!(
        h.object("scenario", "LOCAL")["initial_conditions"],
        physical_before
    );
    let task = h.run("LOCAL");
    h.finish_one();
    let run = h.completed_run(&task);
    assert_eq!(run.acceptance_events.len(), 1);
    assert_eq!(
        run.acceptance_events[0].at_ms.0.value(),
        75000,
        "professional geometry is not a new factory processing duration"
    );
    let run_value = serde_json::to_value(&run).unwrap();
    let accepted_project = h.object("project", "P1");
    let directory = h.tree.directory("actual-packaging-bound");
    let relocated = h.tree.directory("actual-packaging-bound-reopened");
    let base = h.context().project.unwrap().revision.0.value();
    let saved = h.save(base, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&saved, true).task.state,
        ComputationStatus::Completed
    ));
    copy(&directory, &relocated);
    let imported = h.import(&relocated);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(h.object("artifact", artifact_id), artifact);
    let reopened_project = h.object("project", "P1");
    assert_eq!(reopened_project["bindings"], accepted_project["bindings"]);
    assert_eq!(
        reopened_project["acknowledgments"],
        accepted_project["acknowledgments"]
    );
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), run_value);
    let after_reopen = h.run("LOCAL");
    h.finish_one();
    assert_eq!(h.completed_run(&after_reopen).acceptance_events.len(), 1);
    let mut product_r2 = h.object("product", "PACKAGE");
    product_r2["revision"] = json!("r2");
    let base = h.context().project.unwrap().revision.0.value();
    let _: EditCommit = ok(h.apply(base, json!([put("product", product_r2)])));
    assert!(h.context().issues.iter().any(|issue| {
        issue.code == "professional_inputs.selected_subject_changed"
            && issue
                .object_ref
                .as_ref()
                .is_some_and(|target| target.id.as_str() == "PACK-01")
    }));
    assert_eq!(h.object("artifact", artifact_id), artifact);
    let changed = h.object("project", "P1");
    assert_eq!(changed["bindings"], accepted_project["bindings"]);
    assert_eq!(
        changed["acknowledgments"],
        accepted_project["acknowledgments"]
    );
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), run_value);
    let base = h.context().project.unwrap().revision.0.value();
    let failure = error(h.call(
        "run_scenario",
        json!({"scenario_id":"LOCAL","input_revision":base,
        "observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}}),
    ));
    assert!(matches!(
        failure.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert!(h.host.drain_io_work().is_empty());
    assert_eq!(
        h.object("scenario", "LOCAL")["initial_conditions"],
        physical_before
    );
    // The new product requires its own immutable Job, actual result, UI purpose and adoption.
    let mut next_job = packaging_job.clone();
    next_job["revision"] = json!(3);
    next_job["subject_ref"]["revision"] = json!("r2");
    next_job["conditions"]["product_envelope"]["subject"] = next_job["subject_ref"].clone();
    let mut next_scenario = h.object("scenario", "LOCAL");
    next_scenario["orders"][0]["product_ref"]["revision"] = json!("r2");
    let base = h.context().project.unwrap().revision.0.value();
    let _:EditCommit=ok(h.apply(base,json!([
        {"kind":"put_packaging_job","data":next_job},put("scenario",next_scenario),
        {"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":3},"basis":"user_provided"}}
    ])));
    let base = h.context().project.unwrap().revision.0.value();
    let next_task:TaskHandle=ok(h.call("compute_packaging",json!({"packaging_job":next_job,"input_revision":base,"computation_budget":next_job["budget"]})));
    h.finish_one();
    let next_result = match h.task(&next_task, true).result.unwrap() {
        TaskOutput::Packaging(result) => result,
        other => panic!("expected actual next result: {other:?}"),
    };
    assert!(matches!(next_result.outcome, CheckOutcome::Pass));
    assert_ne!(next_result.artifact_id, calculated.artifact_id);
    let next_id = next_result.artifact_id.as_str();
    let next_artifact = h.object("artifact", next_id);
    assert_eq!(next_artifact["subject_ref"]["revision"], "r2");
    let base = h.context().project.unwrap().revision.0.value();
    let blocked=error(h.call("run_scenario",json!({"scenario_id":"LOCAL","input_revision":base,"observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert!(matches!(
        blocked.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert!(h.host.drain_io_work().is_empty());
    let mut next_ack = acknowledgment(
        "ACK-ACTUAL-R2",
        next_id,
        "PACKAGE",
        json!([
            "forming",
            "opening",
            "linear_extraction",
            "net_space",
            "manufacturing"
        ]),
    );
    next_ack["accepted_subject"]["revision"] = json!("r2");
    let _: EditCommit = ok(h.apply(
        base,
        json!([{"kind":"record_artifact_acknowledgment","data":next_ack}]),
    ));
    let before_new_adoption = h.object("project", "P1");
    assert_eq!(
        before_new_adoption["bindings"],
        accepted_project["bindings"]
    );
    let mut next_request = adoption();
    next_request["artifact_id"] = json!(next_id);
    next_request["acceptance_ref"] = json!("ACK-ACTUAL-R2");
    next_request["base_revision"] =
        serde_json::to_value(h.context().project.unwrap().revision).unwrap();
    let next_binding: BindingCommit = ok(h.call("adopt_artifact", next_request));
    assert_ne!(next_binding.binding.id, binding.binding.id);
    let recovered = h.run("LOCAL");
    h.finish_one();
    let recovered = h.completed_run(&recovered);
    assert_eq!(recovered.acceptance_events.len(), 1);
    assert_eq!(recovered.acceptance_events[0].at_ms.0.value(), 75000);
    let recovered_value = serde_json::to_value(&recovered).unwrap();
    let accepted_r2 = h.object("project", "P1");
    assert_eq!(accepted_r2["bindings"][0], accepted_project["bindings"][0]);
    assert_eq!(
        accepted_r2["acknowledgments"][0],
        accepted_project["acknowledgments"][0]
    );
    assert_eq!(h.object("artifact", artifact_id), artifact);
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), run_value);

    let saved_r2 = h.tree.directory("current-r2");
    let moved_r2 = h.tree.directory("current-r2-opened");
    let base = h.context().project.unwrap().revision.0.value();
    let save = h.save(base, &saved_r2);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).task.state,
        ComputationStatus::Completed
    ));
    let manifest = read(&saved_r2.join("project.json"));
    let original_job = manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["file_ref"]["kind"] == "packaging_job")
        .map(|e| read(&saved_r2.join(e["file_ref"]["path"].as_str().unwrap()))["data"].clone())
        .find(|j| j["id"] == "JOB-U" && j["revision"] == 2)
        .unwrap();
    assert_eq!(original_job, packaging_job);
    copy(&saved_r2, &moved_r2);
    let imported = h.import(&moved_r2);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, false));
    assert_eq!(
        h.object("project", "P1")["bindings"],
        accepted_r2["bindings"]
    );
    let current = h.run("LOCAL");
    h.finish_one();
    assert_eq!(h.completed_run(&current).acceptance_events.len(), 1);

    // Selecting the historical Job after Open does not silently select its old binding.
    let mut old_product = h.object("product", "PACKAGE");
    old_product["revision"] = json!("r1");
    let mut old_scenario = h.object("scenario", "LOCAL");
    old_scenario["orders"][0]["product_ref"]["revision"] = json!("r1");
    let base = h.context().project.unwrap().revision.0.value();
    let _:EditCommit=ok(h.apply(base,json!([put("product",old_product),put("scenario",old_scenario),{"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision","value":{"kind":"integer","value":2},"basis":"user_provided"}}])));
    let base = h.context().project.unwrap().revision.0.value();
    let blocked=error(h.call("run_scenario",json!({"scenario_id":"LOCAL","input_revision":base,"observation_window":{"start_ms":0,"end_ms":240000},"intent":{"kind":"project_scenario"}})));
    assert!(matches!(
        blocked.code,
        ErrorCode::InvalidInput | ErrorCode::MissingReference
    ));
    assert!(h.host.drain_io_work().is_empty());
    let mut old_request = adoption();
    old_request["artifact_id"] = json!(artifact_id);
    old_request["acceptance_ref"] = json!("ACK-ACTUAL");
    old_request["base_revision"] = json!(base);
    let reselected: BindingCommit = ok(h.call("adopt_artifact", old_request));
    assert_ne!(reselected.binding.id, binding.binding.id);
    assert_ne!(reselected.binding.id, next_binding.binding.id);
    let selected_project = h.object("project", "P1");
    assert_eq!(selected_project["bindings"].as_array().unwrap().len(), 3);
    assert_eq!(selected_project["bindings"][0], accepted_r2["bindings"][0]);
    assert_eq!(selected_project["bindings"][1], accepted_r2["bindings"][1]);
    let historical = h.run("LOCAL");
    h.finish_one();
    assert_eq!(h.completed_run(&historical).acceptance_events.len(), 1);
    assert_eq!(h.object("artifact", artifact_id), artifact);
    assert_eq!(h.object("artifact", next_id), next_artifact);
    assert_eq!(serde_json::to_value(h.full(&run.id)).unwrap(), run_value);
    assert_eq!(
        serde_json::to_value(h.full(&recovered.id)).unwrap(),
        recovered_value
    );
}

#[test]
fn a_searched_candidate_gets_a_new_job_revision_without_overwriting_input_or_retargeting_its_consumer()
 {
    let mut h = professional(false);
    let inputs: Value = serde_json::from_str(include_str!(
        "fixtures/production/packaging-acceptance-cases.json"
    ))
    .unwrap();
    let mut input = inputs["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "SEARCH01")
        .unwrap()["job"]["data"]
        .clone();
    input["id"] = json!("JOB-U");
    input["revision"] = json!(2);
    input["subject_ref"] = json!({"id":"PACKAGE","revision":"r1"});
    let input = serde_json::to_value(serde_json::from_value::<PackageStructureJob>(input).unwrap())
        .unwrap();
    let _: EditCommit = ok(h.apply(
        1,
        json!([{"kind":"put_packaging_job","data":input.clone()}]),
    ));
    let _: EditCommit = ok(h.apply(
        2,
        json!([{"kind":"set_declared_parameter","data":{
        "target":{"kind":"device","id":"PACK-01"},"parameter_id":"packaging_job_revision",
        "value":{"kind":"integer","value":2},"basis":"user_provided"}}]),
    ));
    let physical_before = h.object("scenario", "LOCAL")["initial_conditions"].clone();
    let task: TaskHandle = ok(h.call(
        "compute_packaging",
        json!({"packaging_job":input,
        "input_revision":3,"computation_budget":input["budget"]}),
    ));
    h.finish_one();
    let completed = h.task(&task, true);
    assert!(
        matches!(completed.task.state, ComputationStatus::Completed),
        "{completed:?}"
    );
    let result = match completed.result.unwrap() {
        TaskOutput::Packaging(result) => serde_json::to_value(result).unwrap(),
        other => panic!("expected actual searched PackagingResult: {other:?}"),
    };
    assert_eq!(result["evidence_origin"], "actual_software");
    assert_eq!(result["outcome"], "pass");
    assert_eq!(result["job_ref"], json!({"id":"JOB-U","revision":3}));
    assert_eq!(
        h.object("device", "PACK-01")["configuration"]["packaging_job_revision"]["value"],
        2,
        "producing revision3 cannot silently select it for the consumer"
    );
    assert_eq!(
        h.object("scenario", "LOCAL")["initial_conditions"],
        physical_before
    );
    let directory = h.tree.directory("searched-job-versions");
    let relocated = h.tree.directory("searched-job-versions-reopened");
    let base = h.context().project.unwrap().revision.0.value();
    let task = h.save(base, &directory);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).task.state,
        ComputationStatus::Completed
    ));
    let manifest = read(&directory.join("project.json"));
    let mut versions = Vec::new();
    for entry in manifest["data"]["documents"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry["file_ref"]["kind"] == "packaging_job" && entry["object_ref"]["id"] == "JOB-U"
        })
    {
        let stored =
            read(&directory.join(entry["file_ref"]["path"].as_str().unwrap()))["data"].clone();
        let revision = stored["revision"].as_u64().unwrap();
        if revision == 2 {
            assert_eq!(stored, input, "original search domain remains immutable");
        }
        if revision == 3 {
            let mut expected = input.clone();
            expected["revision"] = json!(3);
            expected["panels"][0]["polygon_mm"][1]["x"] = json!(6.0);
            assert_eq!(
                stored, expected,
                "new Job freezes the actual approved target6 candidate"
            );
        }
        versions.push(revision);
    }
    versions.sort();
    assert_eq!(versions, vec![1, 2, 3]);
    copy(&directory, &relocated);
    let task = h.import(&relocated);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    assert_eq!(
        h.object("device", "PACK-01")["configuration"]["packaging_job_revision"]["value"],
        2
    );
    assert_eq!(
        h.object("scenario", "LOCAL")["initial_conditions"],
        physical_before
    );
}

#[test]
fn conflicting_typed_job_artifact_and_result_identities_are_invalid_input_in_both_archive_document_orders()
 {
    let mut h = professional(false);
    let original = h.tree.path.join("professional-seed");
    let project_before = h.object("project", "P1");
    let scenario_before = h.object("scenario", "LOCAL");
    for kind in ["packaging_job", "artifact_record", "packaging_result"] {
        for reverse in [false, true] {
            let directory = h.tree.directory(&format!("conflict-{kind}-{reverse}"));
            copy(&original, &directory);
            let mut manifest = read(&directory.join("project.json"));
            let entry = manifest["data"]["documents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["file_ref"]["kind"] == kind)
                .unwrap()
                .clone();
            let mut body = read(&directory.join(entry["file_ref"]["path"].as_str().unwrap()));
            let issue = json!({"code":"w5.typed_identity_content_conflict","object_ref":null,"field_path":null,"message":"different typed content under exactly the same identity","blocks":[]});
            match kind {
                "packaging_job" => body["data"]["issues"].as_array_mut().unwrap().push(issue),
                "artifact_record" => body["data"]["supported_scope"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("w5_distinct_purpose")),
                "packaging_result" => body["data"]["limitations"]
                    .as_array_mut()
                    .unwrap()
                    .push(issue),
                _ => unreachable!(),
            }
            // Prove these are typed content conflicts, not malformed JSON/schema rejection.
            match kind {
                "packaging_job" => {
                    let _: PackageStructureJob =
                        serde_json::from_value(body["data"].clone()).unwrap();
                }
                "artifact_record" => {
                    let _: ArtifactRecord = serde_json::from_value(body["data"].clone()).unwrap();
                }
                "packaging_result" => {
                    let _: PackagingResult = serde_json::from_value(body["data"].clone()).unwrap();
                }
                _ => unreachable!(),
            }
            let name = format!("same-identity-{kind}.json");
            write(&directory.join(&name), &body);
            let mut duplicate = entry;
            duplicate["file_ref"]["path"] = json!(name);
            let documents = manifest["data"]["documents"].as_array_mut().unwrap();
            documents.push(duplicate);
            if reverse {
                documents.reverse();
            }
            write(&directory.join("project.json"), &manifest);
            let imported = h.import(&directory);
            h.finish_one();
            let state = h.task(&imported, false);
            assert!(
                matches!(state.task.state, ComputationStatus::Failed),
                "{kind} reverse={reverse}: {state:?}"
            );
            assert!(
                matches!(
                    state.failure.as_ref().unwrap().code,
                    ErrorCode::InvalidInput
                ),
                "{kind} reverse={reverse}: {state:?}"
            );
            assert!(state.result.is_none());
            assert_eq!(h.object("project", "P1"), project_before);
            assert_eq!(h.object("scenario", "LOCAL"), scenario_before);
        }
    }
}
