//! Targeted actual consumer, linked against the existing shared Cargo artifacts.
//! No second Engine, Command DTO, stock model, or Cargo/lock changes.
use serde_json::{Value, json};
use sf_domain::*;
use sf_module_kit::*;
use sf_simulation::{Engine, EngineInput};
use std::collections::BTreeMap;
use std::path::Path;

fn case(
    name: &str,
    which: BuiltinTemplate,
    expected: &[u64],
    setup90: bool,
    stock: bool,
    out: &Path,
) -> Result<Value, String> {
    let c = builtin_template(which).map_err(|e| format!("candidate {e:?}"))?;
    let target = empty_template_project(
        format!("{name}-project").try_into().unwrap(),
        name.into(),
        c.example_project.time_reference.clone(),
    );
    let options = TemplateChoices {
        namespace: name.into(),
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
    };
    let p = prepare_template(&c, &target, &options).map_err(|e| format!("prepare {e:?}"))?;
    let choices = TemplateSeedChoices {
        scenario_id: format!("{name}-scenario").try_into().unwrap(),
        supply: stock,
        demand: true,
        initial_quality: c
            .subset
            .adoption
            .as_ref()
            .unwrap()
            .deferred_seed
            .as_ref()
            .unwrap()
            .initial_stock
            .iter()
            .map(|r| (r.location_id.clone(), QualityState::Released))
            .collect(),
        adopt_declared_inspection_results: stock,
    };
    let scenario = prepare_template_seed(&c, &p, &choices).map_err(|e| format!("seed {e:?}"))?;
    // A real shared Commands consumer, not a copied schema or a pretend Host call.
    let payload = p.command_data(Some(scenario.clone()));
    let command: sf_commands::TemplateInstantiation =
        serde_json::from_value(payload.clone()).map_err(|e| format!("Commands DTO {e}"))?;
    assert_eq!(
        command.proposed_objects.len(),
        payload["proposed_objects"].as_array().unwrap().len()
    );
    let mut definitions = c.definitions.clone();
    if setup90 {
        for d in &mut definitions {
            if let ModulePayload::Tasks(t) = &mut d.payload {
                t.duration.as_mut().unwrap().value = DurationMs(90000u64.try_into().unwrap());
            }
        }
    }
    let mut project: ProjectState =
        serde_json::from_value(serde_json::to_value(&p.proposed_project).unwrap()).unwrap();
    if setup90 {
        // Explicit edited definition revision; never publish two contents under
        // the same selected exact ModuleRef identity.
        for d in &mut definitions {
            if matches!(d.payload, ModulePayload::Tasks(_)) {
                d.version = "1.0.1".to_owned().try_into().unwrap();
                for r in &mut project.module_refs {
                    if r.id == d.id {
                        r.version = d.version.clone();
                    }
                }
                for r in &mut project.routes {
                    for s in &mut r.steps {
                        if s.operation_ref.id == d.id {
                            s.operation_ref.version = d.version.clone();
                        }
                    }
                }
                for a in &mut project.template_adoptions {
                    for b in &mut a.preparation_bindings {
                        if b.task_ref.id == d.id {
                            b.task_ref.version = d.version.clone();
                        }
                    }
                }
            }
        }
    }
    let scenario: ScenarioInput =
        serde_json::from_value(serde_json::to_value(scenario).unwrap()).unwrap();
    let input_ref:RunInputRef=serde_json::from_value(json!({"project_id":project.id,"project_revision":project.revision,"scenario_id":scenario.id,"scenario_revision":scenario.revision,"frozen_project_file":{"path":"frozen/project.json","kind":"project"},"frozen_scenario_file":{"path":"frozen/scenario.json","kind":"scenario"}})).unwrap();
    let input = json!({"project":project,"scenario":scenario,"definitions":definitions,"input_ref":input_ref});
    std::fs::write(
        out.join(format!("{name}.input.json")),
        serde_json::to_vec_pretty(&input).unwrap(),
    )
    .unwrap();
    std::fs::write(
        out.join(format!("{name}.instantiate.json")),
        serde_json::to_vec_pretty(&payload).unwrap(),
    )
    .unwrap();
    let mut engine = Engine::new(EngineInput {
        run_id: format!("{name}-run").try_into().unwrap(),
        scope: scenario.scope.clone(),
        project,
        scenario,
        definitions,
        input_ref,
    })
    .map_err(|e| format!("Engine::new {e:#?}"))?;
    engine.run_to_end();
    let result = engine.snapshot();
    std::fs::write(
        out.join(format!("{name}.actual.json")),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    let actual: Vec<_> = result
        .acceptance_events
        .iter()
        .map(|e| e.at_ms.0.value())
        .collect();
    if actual != expected {
        return Err(format!(
            "acceptance expected {expected:?}, actual {actual:?}; status {:?}, limitations {:?}",
            result.computation_status, result.limitations
        ));
    }
    if !result.whole_window_computed {
        return Err("not whole window".into());
    }
    if stock && result.resource_claims.is_empty() {
        return Err("no actual resource claims".into());
    }
    if stock
        && !result
            .final_items
            .iter()
            .any(|i| matches!(i.lifecycle, Lifecycle::Transformed))
    {
        return Err("no actual input consumption".into());
    }
    let again = serde_json::to_value(engine.snapshot()).unwrap();
    assert_eq!(again, serde_json::to_value(&result).unwrap());
    Ok(
        json!({"case":name,"status":"passed","acceptance_ms":actual,"final_items":result.final_items.len(),"claims":result.resource_claims.len(),"whole_window":result.whole_window_computed,"Commands_DTO":"actual_deserialization","Host_adoption":"not_invoked"}),
    )
}
fn main() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut rows = vec![];
    let mut failed = false;
    for (name, which, expected, setup90, stock) in [
        (
            "machining",
            BuiltinTemplate::MachiningDelivery,
            vec![90000, 150000],
            false,
            true,
        ),
        (
            "purchased",
            BuiltinTemplate::PurchasedAssembly,
            vec![70000, 110000],
            false,
            true,
        ),
        (
            "parallel",
            BuiltinTemplate::ParallelElectromechanical,
            vec![160000, 200000],
            false,
            true,
        ),
        (
            "parallel-setup90",
            BuiltinTemplate::ParallelElectromechanical,
            vec![220000, 260000],
            true,
            true,
        ),
        (
            "carton",
            BuiltinTemplate::CartonFactory,
            vec![45000, 65000],
            false,
            true,
        ),
        (
            "no-supply",
            BuiltinTemplate::PurchasedAssembly,
            vec![],
            false,
            false,
        ),
    ] {
        match case(name, which, &expected, setup90, stock, out) {
            Ok(r) => rows.push(r),
            Err(e) => {
                failed = true;
                rows.push(json!({"case":name,"status":"failed","error":e}));
            }
        }
    }
    std::fs::write(
        out.join("engine-consumer-results.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
    if failed {
        std::process::exit(1);
    }
}
