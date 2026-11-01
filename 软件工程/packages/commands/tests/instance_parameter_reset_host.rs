//! Removing an explicit instance override must also remove only its source.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::json;
use sf_commands::*;
use support::*;

#[test]
fn explicit_parameter_removal_is_atomic_undoable_and_survives_save_open() {
    let mut value = fixture();
    let reference = value["project"]["devices"][0]["module_ref"].clone();
    let definition = value["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|definition| definition["id"] == reference["id"])
        .unwrap();
    for name in ["speed", "auxiliary"] {
        definition["payload"]["data"]["configuration"][name] = json!({"kind":"number","value":1});
        definition["parameters"].as_array_mut().unwrap().push(
            json!({"id":name,"value_type":"number",
            "binding":format!("/payload/data/configuration/{name}/value"),"label":null,"unit":null,
            "minimum":0,"maximum":10,"integer":false,"options":[],"basis":"module_declared"}),
        );
    }
    let mut h = Harness::from_value(value);
    let run = h.run("LOCAL");
    h.finish_one();
    let original_run = h.completed_run(&run);
    let device_id = h.object("project", "P1")["devices"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let _: EditCommit = ok(h.apply(1, json!([
        {"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":device_id},"parameter_id":"speed","value":{"kind":"number","value":3},"basis":"user_provided"}},
        {"kind":"set_declared_parameter","data":{"target":{"kind":"device","id":device_id},"parameter_id":"auxiliary","value":{"kind":"number","value":4},"basis":"module_declared"}}
    ])));
    let before = h.object("project", "P1");
    let mut changed = h.object("device", &device_id);
    changed["configuration"]["speed"]["value"] = json!(5);
    assert!(matches!(
        error(h.apply(2, json!([put("device", changed.clone())]))).code,
        ErrorCode::InvalidInput
    ));
    assert_eq!(
        h.object("project", "P1"),
        before,
        "a sourced value cannot be silently replaced"
    );
    changed["configuration"]
        .as_object_mut()
        .unwrap()
        .remove("speed");
    let _: EditCommit = ok(h.apply(2, json!([put("device", changed.clone())])));
    let after = h.object("project", "P1");
    assert_eq!(after["parameter_sources"].as_array().unwrap().len(), 1);
    let auxiliary = before["parameter_sources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|source| source["field_path"] == "/configuration/auxiliary/value")
        .unwrap();
    assert_eq!(&after["parameter_sources"][0], auxiliary);
    assert_eq!(
        after["parameter_sources"][0]["field_path"],
        "/configuration/auxiliary/value"
    );
    assert_eq!(after["devices"][1], before["devices"][1]);
    let _: EditCommit = ok(h.apply(3, json!([{"kind":"undo"}])));
    assert_eq!(
        h.object("project", "P1")["parameter_sources"],
        before["parameter_sources"]
    );
    let _: EditCommit = ok(h.apply(4, json!([{"kind":"redo"}])));
    assert_eq!(h.object("device", &device_id), changed);
    assert_eq!(
        h.object("project", "P1")["parameter_sources"],
        after["parameter_sources"]
    );
    let saved = h.tree.directory("parameter-reset");
    let task = h.save(5, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&task, true).result,
        Some(TaskOutput::Save(_))
    ));
    let open = h.import(&saved);
    h.finish_one();
    let proposal = h.imported(&open);
    let _: EditCommit = ok(h.adopt(&open, &proposal, false));
    assert_eq!(h.object("device", &device_id), changed);
    assert_eq!(
        h.object("project", "P1")["parameter_sources"],
        after["parameter_sources"]
    );
    assert_eq!(
        serde_json::to_value(h.full(&original_run.id)).unwrap(),
        serde_json::to_value(original_run).unwrap()
    );
}
