#[path = "../../commands/tests/fixtures/production/p1_engine_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_domain::*;
use sf_simulation::{Engine, EngineInput, PortOutputRecord};
use support::{engine, parallel, phased};

fn p303() -> Engine {
    let declared: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.input.json"
    ))
    .unwrap();
    let ready: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.bootstrap.actual.json"
    ))
    .unwrap();
    let scenario: ScenarioInput =
        serde_json::from_value(ready["snapshot"]["scenarios"][0].clone()).unwrap();
    Engine::new(EngineInput {
        run_id: serde_json::from_value(ready["formal_run"]["id"].clone()).unwrap(),
        input_ref: serde_json::from_value(ready["formal_run"]["input_ref"].clone()).unwrap(),
        project: serde_json::from_value(ready["snapshot"]["project"].clone()).unwrap(),
        scope: scenario.scope.clone(),
        scenario,
        definitions: serde_json::from_value(declared["definitions"].clone()).unwrap(),
    })
    .unwrap()
}
fn rows(engine: &Engine, device: &str) -> Vec<PortOutputRecord> {
    engine.port_output_history().into_iter().filter(|r|matches!(&r.2,JobExecutionRef::ProjectDevice{device_id,..} if device_id.as_str()==device)).collect()
}
fn counts(rows: &[PortOutputRecord], at: u64) -> (u64, u64) {
    let total = |transferred| {
        rows.iter()
            .filter(|r| r.0.0.value() <= at && r.6 == transferred)
            .map(|r| r.5.value.coefficient.value())
            .sum()
    };
    (total(false), total(true))
}
fn out() -> std::path::PathBuf {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios/真实业务设计/runtime-port-output");
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn actual_output75_transfer85_freezes_quantity_without_recounting_wait_or_future_stock() {
    let mut e = p303();
    assert!(e.port_output_history().is_empty());
    while e
        .snapshot()
        .computed_until_ms
        .as_ref()
        .is_none_or(|v| v.0.value() < 75000)
    {
        e.step();
    }
    let prefix = rows(&e, "CNC");
    assert_eq!(counts(&prefix, 75000), (1, 0));
    assert_eq!(prefix.len(), 1);
    e.pause();
    assert_eq!(e.step(), 0);
    assert_eq!(rows(&e, "CNC").len(), 1);
    e.resume();
    e.run_to_end();
    let history = rows(&e, "CNC");
    assert_eq!(history.len(), 2);
    for (time, want) in [
        (74999, (0, 0)),
        (75000, (1, 0)),
        (80000, (1, 0)),
        (85000, (1, 1)),
    ] {
        assert_eq!(counts(&history, time), want);
    }
    assert_eq!(history[0].0.0.value(), 75000);
    assert_eq!(history[1].0.0.value(), 85000);
    assert_eq!(history[0].7, history[1].7);
    assert_eq!(history[0].3.as_str(), "output");
    let frozen = serde_json::to_value(e.port_output_history()).unwrap();
    let mut returned = e.port_output_history();
    returned[0].5.value.coefficient = 9u64.try_into().unwrap();
    assert_eq!(
        serde_json::to_value(e.port_output_history()).unwrap(),
        frozen
    );
    let original: Value = serde_json::from_str(include_str!(
        "../../../scenarios/图片时间轴/runtime/phase.bootstrap.actual.json"
    ))
    .unwrap();
    assert_eq!(
        serde_json::to_value(e.snapshot()).unwrap(),
        original["formal_run"]
    );
    std::fs::write(out().join("p303-engine.actual.json"),serde_json::to_vec_pretty(&json!({"history":e.port_output_history(),"Run":e.snapshot(),"original_entire_Run_equal":true})).unwrap()).unwrap();
}

#[test]
fn multiple_actual_jobs_same_device_record_each_physical_output_once() {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../commands/tests/fixtures/production/local-assembly-pack.json"
    ))
    .unwrap();
    let mut pcb = v["scenario"]["initial_conditions"]["items"][2].clone();
    pcb["id"] = json!("output-second-pcb");
    v["scenario"]["arrivals"] = json!([{"id":"output-arrival","at_ms":90000,"destination":"pcbs-A","items":[pcb],"release_policy":"unchecked_allowed"}]);
    let mut e = engine(&v);
    e.run_to_end();
    let history = rows(&e, "ASM-01");
    assert_eq!(history.len(), 4);
    assert_eq!(counts(&history, 59999), (0, 0));
    assert_eq!(counts(&history, 60000), (1, 1));
    assert_eq!(counts(&history, 150000), (2, 2));
    let records = e.port_output_history();
    let mut seen = std::collections::BTreeSet::new();
    for row in records {
        assert!(seen.insert((row.1, row.3, row.6, row.7)));
    }
    assert_eq!(
        e.snapshot()
            .acceptance_events
            .iter()
            .map(|v| v.at_ms.0.value())
            .collect::<Vec<_>>(),
        vec![75000, 165000]
    );
}

#[test]
fn original_inspection_branches_rework_preserve_ids_and_packages_do_not_count_contents_as_outputs()
{
    for (case, pass, fail, isolated, packages) in [("A", 2, 1, 0, 2), ("Q", 1, 1, 1, 1)] {
        let v = parallel(case);
        let policy = &v["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == "w5.op.inspect")
            .unwrap()["payload"]["data"]["quality_and_routes"];
        let mut e = engine(&v);
        e.run_to_end();
        let run = e.snapshot();
        let records = e.port_output_history();
        let inspection: Vec<_> = records
            .iter()
            .filter(|r| {
                !r.6 && run
                    .jobs
                    .iter()
                    .any(|j| j.id == r.1 && j.operation_ref.id.as_str() == "w5.op.inspect")
            })
            .collect();
        assert_eq!(
            inspection
                .iter()
                .filter(|r| r.3.as_str() == policy["pass_port"].as_str().unwrap())
                .count(),
            pass
        );
        assert_eq!(
            inspection
                .iter()
                .filter(|r| r.3.as_str() == policy["fail_port"].as_str().unwrap())
                .count(),
            fail
        );
        assert_eq!(inspection.len(), pass + fail + isolated);
        for row in inspection {
            let job = run.jobs.iter().find(|j| j.id == row.1).unwrap();
            assert!(job.input_refs.contains(&row.7));
            assert_eq!(row.5.value.coefficient.value(), 1);
        }
        let packing: Vec<_> = records
            .iter()
            .filter(|r| {
                !r.6 && run
                    .jobs
                    .iter()
                    .any(|j| j.id == r.1 && j.operation_ref.id.as_str() == "w5.op.pack")
            })
            .collect();
        assert_eq!(packing.len(), packages);
        assert!(
            packing
                .iter()
                .all(|r| r.5.value.coefficient.value() == 1 && r.4.id.as_str() == "package")
        );
    }
}

#[test]
fn declared_two_output_ports_and_bulk_units_keep_actual_row_port_and_quantity() {
    let mut v = phased();
    for typ in v["project"]["item_types"].as_array_mut().unwrap() {
        typ["tracking"] = json!("batch");
    }
    let def_index = v["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|d| d["id"] == "w5.op.phased")
        .unwrap();
    let device_ref = v["project"]["devices"][0]["module_ref"].clone();
    let device_index = v["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .position(|d| d["id"] == device_ref["id"])
        .unwrap();
    let mut second = v["definitions"][def_index]["payload"]["data"]["outputs"][0].clone();
    second["port"] = json!("output-secondary");
    second["role"] = json!("secondary");
    second["quantity"]["value"]["coefficient"] = json!(2);
    v["definitions"][def_index]["payload"]["data"]["outputs"][0]["quantity"]["value"]["coefficient"] =
        json!(3);
    v["definitions"][def_index]["payload"]["data"]["outputs"]
        .as_array_mut()
        .unwrap()
        .push(second);
    let ports = &mut v["definitions"][device_index]["payload"]["data"]["ports"];
    let mut p = ports
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "output")
        .unwrap()
        .clone();
    p["id"] = json!("output-secondary");
    p["payload"]["data"]["item_role"] = json!("secondary");
    ports.as_array_mut().unwrap().push(p);
    v["project"]["devices"][0]["role_bindings"]["item_types"]["secondary"] = json!("part");
    let mut connection = v["project"]["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["data"]["source"]["kind"] == "port")
        .unwrap()
        .clone();
    connection["data"]["id"] = json!("secondary-output");
    connection["data"]["source"]["data"]["port_id"] = json!("output-secondary");
    v["project"]["connections"]
        .as_array_mut()
        .unwrap()
        .push(connection);
    v["project"]["locations"][1]["capacity"]["value"]["coefficient"] = json!(10);
    let mut e = engine(&v);
    e.run_to_end();
    let history = rows(&e, "CNC");
    for (port, quantity) in [("output", 3), ("output-secondary", 2)] {
        let found: Vec<_> = history.iter().filter(|r| r.3.as_str() == port).collect();
        assert_eq!(found.len(), 2);
        assert!(
            found
                .iter()
                .all(|r| r.5.value.coefficient.value() == quantity && r.5.unit.as_str() == "piece")
        );
    }
    assert_eq!(history.len(), 4);
    assert_eq!(counts(&history, 45000), (5, 5));
}
