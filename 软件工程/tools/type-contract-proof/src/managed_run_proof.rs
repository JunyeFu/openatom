//! AQ-01: canonical native carrier and oversize fixture, not a product Host.
use super::{api::*, contract::*};
use serde_json::{Value, json};
use std::{fs, path::Path};

pub fn run(root: &Path) {
    let out = root.join("output");
    let mut fixture: Value = serde_json::from_slice(&fs::read(root.join("fixtures/run.json")).unwrap()).unwrap();
    let run = &mut fixture["data"];
    run["computed_until_ms"] = json!(0);
    run["final_items"] = Value::Array((0..3000).map(|i| json!({
        "id":format!("I{i:04}"), "item_type_ref":{"id":"TYPE1","revision":"r1"},
        "quantity":{"value":{"coefficient":1,"decimal_places":0},"unit":"piece"},
        "holder":{"kind":"location","id":"L1"}, "lifecycle":"available", "quality":"released",
        "allocation_refs":[], "composition_refs":[], "contents":[], "completed_reworks":0,
        "split_from_ref":null
    })).collect());
    run["events"] = Value::Array((0..3000).map(|i| json!({
        "sequence":i,"at_ms":0,"subject":{"kind":"item","id":format!("I{i:04}")},
        "payload":{"kind":"arrival_accepted","data":format!("I{i:04}")}
    })).collect());
    let typed: Versioned<RunResult> = serde_json::from_value(fixture.clone()).unwrap();
    let bytes = serde_json::to_vec(&typed).unwrap();
    assert!(bytes.len() > 1_048_576);
    assert_eq!(fixture, serde_json::from_slice::<Value>(&bytes).unwrap());
    fs::write(out.join("aq01-large-run.json"), &bytes).unwrap();

    let mut reply: Value = serde_json::from_slice(&fs::read(root.join("api-fixtures/get_run.response.json")).unwrap()).unwrap();
    reply["outcome"]["data"] = json!({"kind":"full","data":fixture["data"]});
    let full: Response<RunView> = serde_json::from_value(reply.clone()).unwrap();
    let full_bytes = serde_json::to_vec(&full).unwrap();
    assert!(full_bytes.len() > 1_048_576);
    fs::write(out.join("aq01-full-response.json"), &full_bytes).unwrap();
    reply["outcome"] = json!({"status":"managed","data":{
        "result_id":"AQ01-R1-0", "response_schema_ref":"tools/type-contract-proof/output/api-schemas/get_run.response.schema.json", "byte_length":full_bytes.len()
    }});
    let response: Response<RunView> = serde_json::from_value(reply.clone()).unwrap();
    assert_eq!(reply, serde_json::to_value(response).unwrap());
    let frame = json!({"wire_version":"1","host_instance_id":"TYPE-FIXTURE",
        "client_session_id":"TYPE-FIXTURE-SESSION", "originating_operation":"get_run", "reply":reply,
        "managed_assets":null,"managed_results":[{"result_id":"AQ01-R1-0","handle_value":8}]});
    let typed_frame: IpcResponseFrame<RunView> = serde_json::from_value(frame.clone()).unwrap();
    assert_eq!(frame, serde_json::to_value(typed_frame).unwrap());
    let frame_bytes = serde_json::to_vec(&frame).unwrap();
    assert!(frame_bytes.len() <= 1_048_576);
    fs::write(out.join("aq01-managed-run.response.json"), serde_json::to_vec_pretty(&reply).unwrap()).unwrap();
    fs::write(out.join("aq01-managed-run.frame.json"), &frame_bytes).unwrap();
    let mut negatives = Vec::new();
    for (id, field, value) in [
        ("unknown_path", "path", json!("C:/private.json")),
        ("empty_result_id", "result_id", json!("")),
        ("unsafe_byte_length", "byte_length", json!(9_007_199_254_740_992u64)),
    ] {
        let mut bad = reply.clone(); bad["outcome"]["data"][field] = value;
        assert!(serde_json::from_value::<Response<RunView>>(bad.clone()).is_err());
        negatives.push(json!({"id":id,"response":bad}));
    }
    fs::write(out.join("aq01-negative-responses.json"), serde_json::to_vec_pretty(&negatives).unwrap()).unwrap();
    fs::write(out.join("aq01-native-result.json"), serde_json::to_vec_pretty(&json!({
        "status":"passed_actual_native_decode_roundtrip_and_frame_sizing", "items":3000,"events":3000,
        "result_bytes":full_bytes.len(),"frame_bytes":frame_bytes.len(),"frame_cap":1_048_576,
        "native_positive_carriers":3,"native_negative_carriers":negatives.len(),
        "actual_product_runtime":null,"actual_Host_authorization":false
    })).unwrap()).unwrap();
}
