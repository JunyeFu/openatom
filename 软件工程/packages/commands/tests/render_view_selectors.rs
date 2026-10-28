use serde_json::{Value, json};
use sf_commands::{ApiRequest, RenderViewParams};

fn original() -> Value {
    serde_json::from_str(include_str!(
        "../../../tools/type-contract-proof/api-fixtures/render_view.request.json"
    ))
    .unwrap()
}

#[test]
fn original_render_request_keeps_its_wire_shape() {
    let original = original();
    let request: ApiRequest = serde_json::from_value(original.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), original);
}

#[test]
fn render_selectors_keep_exact_module_version_and_comparison_inputs() {
    let mut request = original();
    request["params"]["module_ref"] = json!({"id":"module-exact", "version":"2.7.0"});
    request["params"]["comparison"] = json!({
        "run_ids":["frozen-A", "frozen-B"],
        "metric_scope":{"keys":["accepted_quantity"], "objects":[], "acceptance":[]}
    });
    let actual: ApiRequest = serde_json::from_value(request.clone()).unwrap();
    assert_eq!(serde_json::to_value(actual).unwrap(), request);
    for selector in ["module_ref", "comparison"] {
        let mut only_one = request["params"].clone();
        only_one.as_object_mut().unwrap().remove(selector);
        let typed: RenderViewParams = serde_json::from_value(only_one.clone()).unwrap();
        assert_eq!(serde_json::to_value(typed).unwrap(), only_one);
    }
}

#[test]
fn request_cannot_supply_display_authority() {
    let mut request = original();
    request["params"]["allow_view_control"] = json!(true);
    assert!(serde_json::from_value::<ApiRequest>(request).is_err());
}
