use serde_json::{Value, json};
use sf_domain::*;
use sf_module_kit::*;
use std::collections::BTreeMap;
fn id(s: &str) -> ObjectId {
    s.to_owned().try_into().unwrap()
}
fn layout(child: &ItemType) -> Value {
    json!({"assignment":"item_id_order","slots":[{"item_type_ref":{"id":child.id,"revision":child.revision},"anchor_role":"workpiece","offset":{"translation":{"x":0.5,"y":0,"z":0},"rotation":{"x":0,"y":0,"z":0,"w":1},"scale":{"x":1,"y":1,"z":1},"unit":"meter","coordinate_frame":"right_handed_z_up"}}]})
}
fn candidate() -> TemplateCandidate {
    let mut c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let mut p = serde_json::to_value(&c.example_project).unwrap();
    let child = &c.example_project.item_types[0];
    p["devices"][0]["item_layout"] = layout(child);
    let mut l = layout(child);
    l["slots"][0]["anchor_role"] = Value::Null;
    p["locations"][0]["item_layout"] = l;
    p["item_types"][1]["contents_layout"] = layout(child);
    c.example_project = serde_json::from_value(p).unwrap();
    c.subset.devices = c.example_project.devices.clone();
    c.subset.locations = c.example_project.locations.clone();
    c.subset.item_types = c.example_project.item_types.clone();
    c
}
fn target() -> ProjectState {
    empty_template_project(
        "target".to_owned().try_into().unwrap(),
        "target".into(),
        TimeReference {
            label: "scenario_start".into(),
            origin_iso8601: None,
            timezone: None,
        },
    )
}
fn choices(c: &TemplateCandidate) -> TemplateChoices {
    TemplateChoices {
        namespace: "SLOTS".into(),
        resources: c
            .example_project
            .resources
            .iter()
            .map(|r| {
                (
                    r.id.clone(),
                    TemplateResourceSelection::CreateDeclaredExample,
                )
            })
            .collect(),
        reuse_items: BTreeMap::new(),
        include_example_product: true,
    }
}
fn children(p: &ProjectState) -> Vec<Value> {
    let p = serde_json::to_value(p).unwrap();
    vec![
        p["devices"][0]["item_layout"]["slots"][0]["item_type_ref"].clone(),
        p["locations"][0]["item_layout"]["slots"][0]["item_type_ref"].clone(),
        p["item_types"][1]["contents_layout"]["slots"][0]["item_type_ref"].clone(),
    ]
}
#[test]
fn each_owner_slot_maps_explicit_child_id_and_exact_revision() {
    let c = candidate();
    let before = serde_json::to_value(&c.example_project).unwrap();
    let prepared = prepare_template(&c, &target(), &choices(&c)).unwrap();
    let child = &c.example_project.item_types[0];
    for value in children(&prepared.proposed_project) {
        assert_eq!(
            value,
            json!({"id":format!("SLOTS:{}",child.id.as_str()),"revision":child.revision})
        );
    }
    assert_eq!(serde_json::to_value(&c.example_project).unwrap(), before);
}
#[test]
fn reuse_maps_slot_to_exact_selected_type_without_touching_target_layout() {
    let mut c = candidate();
    let spec: SubjectRevisionRef =
        serde_json::from_value(json!({"id":"known-spec","revision":"r1"})).unwrap();
    c.example_project.item_types[0].spec_ref = Some(spec);
    c.subset.item_types = c.example_project.item_types.clone();
    let mut t = target();
    let mut item = c.example_project.item_types[0].clone();
    let source = item.id.clone();
    item.id = id("existing-child");
    item.contents_layout = Some(serde_json::from_value(layout(&item)).unwrap());
    t.item_types.push(item);
    let before = serde_json::to_value(&t).unwrap();
    let mut options = choices(&c);
    options.reuse_items.insert(source, id("existing-child"));
    let prepared = prepare_template(&c, &t, &options).unwrap();
    let p = serde_json::to_value(&prepared.proposed_project).unwrap();
    for owner in [
        &p["devices"][0]["item_layout"],
        &p["locations"][0]["item_layout"],
        &p["item_types"][0]["contents_layout"],
    ] {
        assert_eq!(owner["slots"][0]["item_type_ref"]["id"], "existing-child");
        assert_eq!(
            owner["slots"][0]["item_type_ref"]["revision"],
            t.item_types[0].revision.as_str()
        );
    }
    assert_eq!(serde_json::to_value(&t).unwrap(), before);
    t.item_types[0].revision = "r2".to_owned().try_into().unwrap();
    assert!(
        prepare_template(&c, &t, &options)
            .unwrap_err()
            .iter()
            .any(|i| i.code == "template.item_mismatch")
    );
}
#[test]
fn missing_child_and_wrong_revision_are_not_repaired_by_same_name_or_latest() {
    for wrong in [false, true] {
        let mut c = candidate();
        let mut p = serde_json::to_value(&c.example_project).unwrap();
        p["devices"][0]["item_layout"]["slots"][0]["item_type_ref"]
            [if wrong { "revision" } else { "id" }] = json!("unknown");
        c.example_project = serde_json::from_value(p).unwrap();
        c.subset.devices = c.example_project.devices.clone();
        let e = prepare_template(&c, &target(), &choices(&c)).unwrap_err();
        assert!(e.iter().any(|i| i.code == "template.item_layout_reference"));
    }
}
