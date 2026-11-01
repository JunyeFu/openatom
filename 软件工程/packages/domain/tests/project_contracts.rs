use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sf_domain::*;
use std::{fs, path::PathBuf};

fn fixture(name: &str) -> Value {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/type-contract-proof/fixtures");
    serde_json::from_slice(&fs::read(root.join(format!("{name}.json"))).unwrap()).unwrap()
}

fn roundtrip<T: DeserializeOwned + Serialize>(value: Value) {
    let typed: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(value, serde_json::to_value(typed).unwrap());
}

#[test]
fn full_project_scenario_and_nine_module_categories_roundtrip_without_loss() {
    roundtrip::<Versioned<ProjectState>>(fixture("project"));
    roundtrip::<Versioned<ScenarioInput>>(fixture("scenario"));
    for category in [
        "items",
        "devices",
        "resources",
        "tasks",
        "operations",
        "transport",
        "templates",
        "professional",
        "media",
    ] {
        roundtrip::<Versioned<ModulePackage>>(fixture(&format!("module-{category}")));
    }
}

#[test]
fn project_boundary_rejects_missing_unknown_and_invalid_fields() {
    let original = fixture("project");
    for key in [
        "id",
        "revision",
        "views",
        "item_types",
        "module_refs",
        "time_reference",
    ] {
        let mut value = original.clone();
        value["data"].as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<Versioned<ProjectState>>(value).is_err(),
            "{key}"
        );
    }
    for (key, bad) in [
        ("runtime_inventory", json!([])),
        ("revision", json!(9007199254740992u64)),
        ("revision", json!("r1")),
        ("revision", json!(1.25)),
    ] {
        let mut value = original.clone();
        value["data"][key] = bad;
        assert!(
            serde_json::from_value::<Versioned<ProjectState>>(value).is_err(),
            "{key}"
        );
    }
}

#[test]
fn project_and_definition_scopes_keep_their_separate_identity() {
    let project = json!({"kind":"project","data":"P1"});
    let definition = json!({"kind":"definition_draft","data":{"id":"draft-1","revision":2}});
    roundtrip::<EditScope>(project.clone());
    roundtrip::<EditScope>(definition);
    assert!(serde_json::from_value::<EditScope>(json!({"kind":"project","data":1})).is_err());
    assert!(
        serde_json::from_value::<EditScope>(json!({"kind":"definition_draft","data":"P1"}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<EditScope>(json!({"kind":"project","data":"P1","revision":1}))
            .is_err()
    );
    assert!(serde_json::from_value::<TaskScope>(project).is_err());
}
