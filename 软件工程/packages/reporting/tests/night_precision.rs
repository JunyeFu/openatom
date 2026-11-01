//! Independent rational-oracle vectors for final subtotal/unit-cost rounding.
//! Prices are typed counterexamples, not actual industrial price evidence.
use serde_json::{Value, json};
use sf_domain::*;
use sf_reporting::build_evaluation;
type Frozen = (
    ProjectState,
    ScenarioInput,
    Vec<ModuleDefinition>,
    RunResult,
);
#[test]
fn exact_aggregate_matches_independent_fraction_oracle_across_unlike_denominators() {
    let saved: Value = serde_json::from_str(include_str!(
        "fixtures/evaluation-acceptance/actual-engine-inputs.json"
    ))
    .unwrap();
    let cases: Value = serde_json::from_str(r###"[{"terms":[[0,0,1,0]],"total_bits":0,"unit_cost_bits":0},{"terms":[[1,1,135000,0],[2,1,135000,0]],"total_bits":4599075939470750515,"unit_cost_bits":4594572339843380019},{"terms":[[1,0,3,0],[2,0,7,0]],"total_bits":4680479398052083419,"unit_cost_bits":4675975798424712923},{"terms":[[9007199254740991,0,1,9]],"total_bits":5057175182282326015,"unit_cost_bits":5052671582654955519},{"terms":[[1,9,9007199254740991,0]],"total_bits":4310421864666095755,"unit_cost_bits":4305918265038725259},{"terms":[[9007199254740114,1,9007199254740107,8],[1,3,1,9]],"total_bits":4788905364267466756,"unit_cost_bits":4784401764640096260},{"terms":[[9007199254740991,8,1,3],[1,1,3,9],[1,1,9007199254740294,0]],"total_bits":4847451708920233369,"unit_cost_bits":4842948109292862873},{"terms":[[1,2,1,9],[1,5,1,4],[1,1,9007199254740266,8],[98148,7,9007199254740467,8]],"total_bits":4788352404322762753,"unit_cost_bits":4783848804695392257},{"terms":[[3925,9,3,3],[9007199254740991,8,7,6],[1,0,7,7],[1,9,3,2],[1,8,7,9],[28538,8,3,9],[9007199254740991,8,9007199254740729,7]],"total_bits":4879680355092664888,"unit_cost_bits":4875176755465294392},{"terms":[[1,0,3,0],[9007199254740466,2,7,6],[65505,8,9007199254740222,6],[9007199254740475,1,9007199254740431,3],[1,6,3,2],[1,9,9007199254740942,2],[9007199254740708,9,7,6],[98715,3,3,9],[9007199254740293,9,7,0],[9007199254740294,0,3,3],[9007199254740991,9,7,6],[9007199254740936,4,7,7]],"total_bits":4971595073187363223,"unit_cost_bits":4967091473559992727},{"terms":[[1,3,1,0],[1,0,7,4],[12657,9,9007199254740587,9],[9007199254740991,3,9007199254740011,2],[9007199254740991,9,1,7],[82317,0,7,8],[1,5,7,3],[9007199254740099,1,9007199254740322,6],[2100,9,3,2],[9007199254740991,4,1,2],[1,6,1,2],[19962,1,3,3],[1,0,7,3],[1,4,1,9],[1,2,9007199254740239,5],[20809,0,7,0],[36254,0,9007199254739994,4],[12246,4,9007199254740716,9],[9007199254740099,2,7,5],[9007199254740991,6,3,6],[9007199254740991,9,3,1],[65306,9,9007199254740761,6],[19189,8,3,7],[9007199254740657,3,1,6]],"total_bits":4968145360008685260,"unit_cost_bits":4963641760381314764}]"###).unwrap();
    for case in cases.as_array().unwrap() {
        let mut i: Frozen = serde_json::from_value(saved["inputs"]["order_known"].clone()).unwrap();
        let template = i.1.cost_inputs[0].clone();
        i.1.cost_inputs = case["terms"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(n, term)| {
                let mut input = template.clone();
                input.id = ObjectId::try_from(format!("rational-term-{n}")).unwrap();
                input.component = format!("independent-price-component-{n}");
                input.rate = Some(
                    serde_json::from_value(json!({"coefficient":term[0],"decimal_places":term[1]}))
                        .unwrap(),
                );
                input.rate_quantity.value =
                    serde_json::from_value(json!({"coefficient":term[2],"decimal_places":term[3]}))
                        .unwrap();
                input
            })
            .collect();
        let order =
            i.1.orders
                .iter()
                .find(|o| o.product_ref.id.as_str() == "A")
                .unwrap();
        let scope = AcceptanceScope {
            product_ref: order.product_ref.clone(),
            unit: order.quantity.as_ref().unwrap().unit.clone(),
            destination: order.destination.clone(),
            stage: order.acceptance_stage.clone(),
        };
        let context=serde_json::from_value(json!({"id":"night-rational","mode":"simulation_evaluation","project_id":i.0.id,"project_revision":i.0.revision,"scenario_refs":[],"run_refs":[],"time_reference":null,"observation_window":null,"acceptance_scope":[],"question_and_scope":"价格算术类型反例","input_assumptions":[],"module_versions":[],"layout_and_schedule":[],"scenario_comparison":null,"conclusions":[],"missing_conditions":[],"supported_scope":[],"limitations":[],"editable_project_ref":null,"asset_rights":[],"generation_context":null,"run_evaluations":[]})).unwrap();
        let report = build_evaluation(context, &[i], &[], &[], &[scope]).unwrap();
        let cost = &report.data.run_evaluations[0].costs.currencies[0];
        assert_eq!(
            cost.known_subtotal.value().to_bits(),
            case["total_bits"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(
            cost.complete_total.as_ref().unwrap().value().to_bits(),
            case["total_bits"].as_u64().unwrap(),
            "{case}"
        );
        assert_eq!(
            cost.unit_cost.as_ref().unwrap().value().to_bits(),
            case["unit_cost_bits"].as_u64().unwrap(),
            "{case}"
        );
    }
}
