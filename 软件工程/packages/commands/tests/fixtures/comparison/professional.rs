//! Whole P0-03 consumers. Imported fixtures remain declarations, never producers.
use crate::support::*;
use serde_json::{Value, json};
use sf_commands::*;

fn documents() -> Value {
    serde_json::from_str(include_str!("../production/professional-documents.json")).unwrap()
}

fn acknowledgment(name: &str, artifact: &str, subject: &str, scope: Value) -> Value {
    json!({"id":name,"artifact_ref":artifact,"accepted_subject":{"id":subject,"revision":"r1"},
        "accepted_scope":scope,"decision_source":{"kind":"human_statement","data":"explicit finite test purpose; not calculation evidence"},
        "conditions":[],"recorded_at_iso8601":null})
}

pub fn professional(with_bindings: bool) -> Harness {
    let mut value = fixture();
    let declaration: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../packaging/consumer-device.design.json"
    )))
    .unwrap();
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
