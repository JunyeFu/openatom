//! Create a real empty editable archive and freeze the existing template inputs.
#[path = "fixtures/production/host_support.rs"] mod support;
use support::*;
use sf_commands::*;
use sf_domain::*;
use sf_module_kit::{builtin_template,BuiltinTemplate};
use serde_json::{json,Value};
use std::{fs,path::Path};
#[test]
fn prepare_actual_empty_project_for_independent_stdio_generation() {
    let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures=std::env::var_os("SF_GF_OUTPUT").map(std::path::PathBuf::from).unwrap_or_else(||root.join(".work/cu-delivery/fixtures"));let out=fixtures.join(format!("prepare-{}",std::process::id()));fs::create_dir_all(&out).unwrap();
    let ui=id("full-gf-ui");let mut h=Harness{host:ProjectHost::new(id("full-gf"),ui.clone()),ui,tree:Sandbox::new()};
    let mut source=builtin_template(BuiltinTemplate::CartonFactory).unwrap();
    let complete=std::env::var_os("SF_GF_TEMPLATE").map(std::path::PathBuf::from).unwrap_or_else(||Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/full_gf/declared-model"));
    source.example_project=serde_json::from_value(serde_json::from_slice::<Value>(&fs::read(complete.join("example-project.json")).unwrap()).unwrap()["data"].clone()).unwrap();
    source.subset=serde_json::from_value(serde_json::from_slice::<Value>(&fs::read(complete.join("template.json")).unwrap()).unwrap()["data"].clone()).unwrap();
    source.package=serde_json::from_value(serde_json::from_slice::<Value>(&fs::read(complete.join("module.json")).unwrap()).unwrap()["data"].clone()).unwrap();
    let req:Value=json!({"api_version":"0.1","request_id":"full-gf-create","project_id":null,"scope":{"kind":"workspace"},"operation":"apply_commands","params":{"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"GF explicit empty draft","time_reference":source.example_project.time_reference,"departure":"stay"}}]}});
    let _:EditCommit=ok(h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap()));
    let req=json!({"api_version":"0.1","request_id":"full-gf-context","project_id":null,"scope":{"kind":"workspace"},"operation":"get_context","params":{"selection":[]}});
    let context:ProjectContext=ok(h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap()));let mut project=context.project.unwrap();
    h.host.register_bundled_template_directory(&complete).unwrap();
    let reference=json!({"id":source.package.definition.id,"version":source.package.definition.version});
    let mut selected=source.example_project.module_refs.iter().map(|r|json!({"kind":"adopt_module","data":{"module_ref":r,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}})).collect::<Vec<_>>();
    selected.push(json!({"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}));
    let req=json!({"api_version":"0.1","request_id":"full-gf-select-input-template","project_id":project.id,"scope":{"kind":"project"},"operation":"apply_commands","params":{"base_revision":project.revision,"commands":selected}});
    let adopted:EditCommit=ok(h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap()));project.revision=adopted.new_revision;
    let seed=out.join("empty-project");fs::create_dir(&seed).unwrap();
    let token=h.host.select_directory(&h.ui,FilePurpose::SaveProject,seed).unwrap();
    let req=json!({"api_version":"0.1","request_id":"full-gf-save","project_id":project.id,"scope":{"kind":"project"},"operation":"save_project","params":{"revision":project.revision,"selected_directory_ref":token}});
    let task:TaskHandle=ok(h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap()));h.finish_one();
    let req=json!({"api_version":"0.1","request_id":"full-gf-saved","project_id":project.id,"scope":{"kind":"project"},"operation":"get_task","params":{"task_id":task.id}});
    let saved:TaskState=ok(h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap()));assert!(matches!(saved.task.state,ComputationStatus::Completed),"{saved:?}");
    let mut previews=vec![];
    for namespace in ["cu-initial","cu-shortfall","cu-budget","cu-conflict"] {
        let req=json!({"api_version":"0.1","request_id":"full-gf-preview","project_id":project.id,"scope":{"kind":"project"},"operation":"get_object","params":{"object_id":{"kind":"module","id":reference["id"]},"node_index":null,"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":project.revision,"choices":null,"seed":null}}}});
        let bare:DomainObject=ok(h.host.dispatch(&h.ui,serde_json::from_value(req.clone()).unwrap()));
        let bare=json!(bare)["data"].clone();
        let quality:serde_json::Map<String,Value>=source.subset.adoption.as_ref().unwrap().deferred_seed.as_ref().unwrap().initial_stock.iter().map(|s|(s.location_id.as_str().to_owned(),json!("released"))).collect();
        let choices=json!({"source_project":bare["source_project"],"namespace":namespace,"resources":source.example_project.resources.iter().map(|r|(r.id.as_str().to_owned(),Value::Null)).collect::<serde_json::Map<_,_>>(),"reuse_items":{},"include_example_product":true,
            "site_envelope":{"length_m":64,"width_m":32,"height_m":10,"regions":[{"id":"explicit-example-floor","kind":"usable","polygon_m":[{"x":0,"y":0},{"x":64,"y":0},{"x":64,"y":32},{"x":0,"y":32}],"clearance_m":null}],"usable_polygon_m":[],"clearance_m":0}});
        let mut req=req;req["params"]["source"]["data"]["choices"]=choices.clone();
        let unseeded:DomainObject=ok(h.host.dispatch(&h.ui,serde_json::from_value(req.clone()).unwrap()));
        let seed=json!({"scenario_id":format!("{namespace}-scenario"),"supply":true,"demand":true,"initial_quality":quality,"adopt_declared_inspection_results":true});
        req["params"]["source"]["data"]["seed"]=seed.clone();
        let seeded:DomainObject=ok(h.host.dispatch(&h.ui,serde_json::from_value(req).unwrap()));
        previews.push(json!({"namespace":namespace,"choices":choices,"seed_selection":seed,"unseeded":unseeded,"seeded":seeded}));
    }
    let packet=json!({"project":project,"source_project":source.example_project,"subset":source.subset,"previews":previews,
        "template_ref":reference,
        "expected":{"accepted_at_ms":[45000,65000],"available_quantity":2,"initial_target":1,"shortfall_target":3,
        "max_committed_repairs":3,"initial_run_counts_as_repair":false,"input_basis":"existing native carton-factory self-authored example; not industrial measurements"}});
    fs::write(out.join("template-input.json"),serde_json::to_vec_pretty(&packet).unwrap()).unwrap();
    fs::write(fixtures.join("latest.json"),serde_json::to_vec_pretty(&json!({"directory":out})).unwrap()).unwrap();
}
