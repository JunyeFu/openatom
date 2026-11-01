//! Explicit Operations mappings must survive adoption without rewriting routes.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::json;
use sf_commands::*;
use support::*;

#[test]
fn operation_adoption_keeps_explicit_mappings_and_unrelated_device_roles_after_save_open() {
    let mut value = fixture();
    let mut operation = value["definitions"][0].clone();
    operation["id"] = json!("local.explicit-operation");
    operation["payload"]["data"]["item_roles"][2] = json!("finished");
    operation["payload"]["data"]["outputs"][0]["role"] = json!("finished");
    operation["payload"]["data"]["resource_roles"][0]["role"] = json!("assembler");
    value["definitions"].as_array_mut().unwrap().push(operation);
    let reference = json!({"id":"local.explicit-operation","version":"0.1.0"});
    value["project"]["module_refs"]
        .as_array_mut()
        .unwrap()
        .push(reference.clone());
    let mut h = Harness::from_value(value);
    let original_run = h.run("LOCAL");
    h.finish_one();
    let original_run = h.completed_run(&original_run);
    let before = h.object("project", "P1");
    let mut expected = before["devices"][0].clone();
    expected["operation_binding"] = reference.clone();
    expected["role_bindings"]["item_types"]["finished"] = json!("assembly");
    expected["role_bindings"]["resources"]["assembler"] = json!("worker-packing");
    expected["role_bindings"]["targets"]["reviewed-target"] =
        json!({"kind":"device","id":"ASM-01"});
    expected["role_bindings"]["ports"]["reviewed-output"] = json!({
        "module_ref":before["devices"][0]["module_ref"],"instance_id":"ASM-01","port_id":"assembled"
    });
    let _: EditCommit = ok(h.apply(1, json!([{"kind":"adopt_module","data":{
        "module_ref":reference,"targets":[{"kind":"device","id":"ASM-01"}],
        "bindings":{"item_types":{"finished":"assembly"},"resources":{"assembler":"worker-packing"},
            "targets":{"reviewed-target":{"kind":"device","id":"ASM-01"}},
            "ports":{"reviewed-output":{"module_ref":before["devices"][0]["module_ref"],"instance_id":"ASM-01","port_id":"assembled"}}},
        "new_instances":[],"explicit_seed":null
    }}])));
    assert_eq!(
        h.object("device", "ASM-01"),
        expected,
        "adoption must store the reviewed mappings and retain device capacity/other roles"
    );
    let after = h.object("project", "P1");
    assert_eq!(after["devices"][1], before["devices"][1]);
    assert_eq!(
        after["routes"], before["routes"],
        "route adoption remains explicit"
    );
    assert_eq!(after["resources"], before["resources"]);
    assert_eq!(
        serde_json::to_value(h.full(&original_run.id)).unwrap(),
        serde_json::to_value(&original_run).unwrap()
    );
    let saved = h.tree.directory("explicit-operation");
    let save = h.save(2, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&save, true).result,
        Some(TaskOutput::Save(_))
    ));
    let open = h.import(&saved);
    h.finish_one();
    let proposal = h.imported(&open);
    // The unchanged old route now conflicts with the explicitly selected
    // operation. Reopening this incomplete draft requires that real choice.
    let _: EditCommit = ok(h.adopt(&open, &proposal, true));
    assert_eq!(h.object("device", "ASM-01"), expected);
    assert_eq!(h.object("project", "P1")["routes"], before["routes"]);
    assert_eq!(
        serde_json::to_value(h.full(&original_run.id)).unwrap(),
        serde_json::to_value(&original_run).unwrap()
    );
}
