use serde_json::json;
use sf_domain::*;
use sf_module_kit::*;
use std::collections::BTreeMap;

fn id(value: &str) -> ObjectId {
    value.to_owned().try_into().unwrap()
}

fn prepare(candidate: &TemplateCandidate) -> PreparedTemplate {
    let target = empty_template_project(
        "night-target".to_owned().try_into().unwrap(),
        "Independent template adoption".into(),
        candidate.example_project.time_reference.clone(),
    );
    let choices = TemplateChoices {
        namespace: "night-copy".into(),
        resources: candidate
            .example_project
            .resources
            .iter()
            .map(|resource| {
                (
                    resource.id.clone(),
                    TemplateResourceSelection::CreateDeclaredExample,
                )
            })
            .collect(),
        reuse_items: BTreeMap::new(),
        include_example_product: true,
    };
    prepare_template(candidate, &target, &choices).unwrap()
}

fn demand() -> TemplateSeedChoices {
    TemplateSeedChoices {
        scenario_id: "night-scenario".to_owned().try_into().unwrap(),
        supply: false,
        demand: true,
        initial_quality: BTreeMap::new(),
        adopt_declared_inspection_results: false,
    }
}

#[test]
fn deferred_seed_uses_explicit_product_identity_instead_of_first_product() {
    let mut candidate = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let selected = SubjectRevisionRef {
        id: candidate.example_project.products[0].id.clone(),
        revision: candidate.example_project.products[0].revision.clone(),
    };
    candidate
        .subset
        .adoption
        .as_mut()
        .unwrap()
        .deferred_seed
        .as_mut()
        .unwrap()
        .product_ref = Some(selected.clone());
    let mut other = candidate.example_project.products[0].clone();
    other.id = id("another-product");
    candidate.example_project.products.insert(0, other);
    let source_before = serde_json::to_value(&candidate.example_project).unwrap();
    let plan = prepare(&candidate);
    let seed = prepare_template_seed(&candidate, &plan, &demand()).unwrap();
    assert_eq!(seed.orders[0].product_ref.id, plan.id_map[&selected.id]);
    assert_eq!(seed.orders[0].product_ref.revision, selected.revision);
    assert!(seed.initial_conditions.items.is_empty());
    assert_eq!(
        serde_json::to_value(&candidate.example_project).unwrap(),
        source_before
    );
}

#[test]
fn deferred_seed_wrong_exact_product_never_substitutes_another_product() {
    for reference in [
        json!({"id":"not-selected","revision":"1"}),
        json!({"id":"night-copy:example-product","revision":"r1"}),
        json!({"id":"p401.purchased-assembly.product","revision":"999"}),
    ] {
        let mut candidate = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
        let mut selected = reference;
        if selected.get("revision") == Some(&json!("999")) {
            selected["id"] = json!(candidate.example_project.products[0].id);
        }
        candidate
            .subset
            .adoption
            .as_mut()
            .unwrap()
            .deferred_seed
            .as_mut()
            .unwrap()
            .product_ref = serde_json::from_value(selected).unwrap();
        let plan = prepare(&candidate);
        let result = prepare_template_seed(&candidate, &plan, &demand());
        assert!(
            result.is_err(),
            "missing exact selected Product must remain a gap"
        );
    }
}

#[test]
fn deferred_seed_rejects_a_mapping_from_another_exact_template_version() {
    let candidate = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let mut plan = prepare(&candidate);
    plan.source_ref.version = "99.0.0".to_owned().try_into().unwrap();
    let before = serde_json::to_value(&plan.proposed_project).unwrap();
    let result = prepare_template_seed(&candidate, &plan, &demand());
    assert!(
        result.is_err(),
        "a different source version must not supply this seed"
    );
    assert_eq!(
        serde_json::to_value(&plan.proposed_project).unwrap(),
        before
    );
}

#[test]
fn deferred_seed_without_product_reference_requires_an_unambiguous_product() {
    let mut candidate = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    assert!(
        candidate
            .subset
            .adoption
            .as_ref()
            .unwrap()
            .deferred_seed
            .as_ref()
            .unwrap()
            .product_ref
            .is_none()
    );
    let mut other = candidate.example_project.products[0].clone();
    other.id = id("another-product");
    candidate.example_project.products.push(other);
    let result = prepare_template_seed(&candidate, &prepare(&candidate), &demand());
    assert!(
        result.is_err(),
        "two products without a selected reference must not choose by vector order"
    );
}
