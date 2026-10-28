use serde_json::{Value, json};
use sf_domain::*;

pub fn fixture() -> Value {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../packages/commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap();
    v["project"]["devices"][0]["pose"]["translation"]["x"] = json!(1);
    v["project"]["devices"][0]["pose"]["translation"]["y"] = json!(1);
    v["project"]["devices"][1]["pose"]["translation"]["x"] = json!(4);
    v["project"]["devices"][1]["pose"]["translation"]["y"] = json!(1);
    v
}
pub fn goal() -> Value {
    json!({"industry":"self-built local assembly fixture","product_family":"packaged-assembly",
        "product_specification_ref":null,"accepted_quantity_per_day":{"value":{"coefficient":2,"decimal_places":0},"unit":"piece"},
        "acceptance_scope":{"product_ref":{"id":"PACKAGE","revision":"r1"},"unit":"piece","destination":"packages-B","stage":"factory_released"},
        "production_calendar_ref":"ALL","observation_window":{"start_ms":0,"end_ms":240000},
        "site_envelope":{"regions":[{"id":"declared-usable-floor","kind":"usable","polygon_m":rectangle(0.,0.,20.,10.),"clearance_m":null}],"length_m":20,"width_m":10,"height_m":4,"usable_polygon_m":[],"clearance_m":0},
        "existing_equipment":[],"constraints":[]})
}
pub fn request() -> Value {
    json!({"revision":1,"generation_goal":goal(),"check_scope":{"objects":[{"kind":"project","id":"P1"}],"check_references":true,"check_business":true,"check_static_layout":true}})
}
pub fn typed(
    v: &Value,
) -> (
    ProjectState,
    std::collections::BTreeMap<ScenarioId, ScenarioInput>,
    Vec<ModuleDefinition>,
) {
    let p = serde_json::from_value(v["project"].clone()).unwrap();
    let s: ScenarioInput = serde_json::from_value(v["scenario"].clone()).unwrap();
    let defs = serde_json::from_value(v["definitions"].clone()).unwrap();
    (
        p,
        std::collections::BTreeMap::from([(s.id.clone(), s)]),
        defs,
    )
}
pub fn has(result: &FactoryDraftCheckResult, kind: DraftCheckKind, outcome: CheckOutcome) -> bool {
    result.checks.iter().any(|c| {
        std::mem::discriminant(&c.kind) == std::mem::discriminant(&kind)
            && std::mem::discriminant(&c.outcome) == std::mem::discriminant(&outcome)
    })
}
pub fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Value {
    json!([{"x":x,"y":y},{"x":x+w,"y":y},{"x":x+w,"y":y+h},{"x":x,"y":y+h}])
}
