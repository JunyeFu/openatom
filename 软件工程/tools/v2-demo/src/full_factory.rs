//! Actual Host constructor for the common 92-device layout. It does not assert
//! that independent normal-use cells are connected product routes.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod proof;
use proof::*;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::path::Path;

/// Read the named existing Run; no new simulation, edit, or source save.
pub(super) fn observe(project_path: &Path, plan_path: &Path, output: &Path) {
    assert!(!output.exists(), "Retain earlier observations; use a new output");
    std::fs::create_dir_all(output).unwrap();
    let plan = read(plan_path);
    let run_id: RunId = serde_json::from_value(plan["run_id"].clone()).unwrap();
    let mut host = Harness::from_seed(Sandbox::new(), project_path);
    let run = host.full(&run_id);
    let at_zero = json!({"kind":"run_snapshot","data":{"run_id":run_id,"at_ms":0}});
    let frozen: DomainObject = ok(host.call("get_object", json!({"object_id":{"kind":"project","id":"P1"},"source":at_zero,"node_index":null})));
    let DomainObject::Project(project) = frozen else { panic!("Frozen Project required") };
    let scenario: DomainObject = ok(host.call("get_object", json!({"object_id":{"kind":"scenario","id":run.input_ref.scenario_id},"source":at_zero,"node_index":null})));
    let DomainObject::Scenario(scenario) = scenario else { panic!("Frozen Scenario required") };
    assert_eq!(project.revision.0.value(), run.input_ref.project_revision.0.value());
    assert_eq!(scenario.revision.0.value(), run.input_ref.scenario_revision.0.value());
    let mut frames = vec![];
    for time in plan["times"].as_array().unwrap() {
        let at = time.as_u64().unwrap();
        let mut objects = vec![];
        let mut selectors = vec![json!([])];
        selectors.extend(run.jobs.chunks(200).map(|jobs| json!(jobs.iter().map(|j|json!({"kind":"job","id":j.id})).collect::<Vec<_>>())));
        for selected in selectors {
            let mut after: Option<usize> = None;
            loop {
                let value: RunView = ok(host.call("get_run",json!({"run_id":run_id,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":selected,"page":{"after_index":after,"limit":200}}}})));
                let RunView::Objects(page) = value else { panic!("Actual object page required") };
                objects.extend(page.objects); after=page.next_index.map(|x|x.value() as usize);
                if after.is_none() { break }
            }
        }
        frames.push(json!({"run_id":run_id,"at_ms":at,"objects":objects,"next_index":null}));
    }
    let mut modules=vec![];
    for reference in &project.module_refs {
        let value: DomainObject=ok(host.call("get_object",json!({"object_id":{"kind":"module","id":reference.id},"source":{"kind":"run_module_version","data":{"run_id":run_id,"at_ms":0,"module_ref":reference}},"node_index":null})));
        let DomainObject::Module(package)=value else {panic!("Exact frozen module required")}; modules.push(package);
    }
    assert_eq!(serde_json::to_value(host.full(&run_id)).unwrap(),serde_json::to_value(&run).unwrap());
    write(&output.join("actual-frozen-project.json"), &serde_json::to_value(&project).unwrap());
    write(&output.join("actual-scenario.json"), &serde_json::to_value(&scenario).unwrap());
    write(&output.join("actual-connected-run.json"), &serde_json::to_value(&run).unwrap());
    write(&output.join("actual-frozen-modules.json"), &serde_json::to_value(&modules).unwrap());
    write(&output.join("actual-observation-frames.json"), &json!(frames));
    write(&output.join("sampling-plan.json"), &plan);
    write(&output.join("receipt.json"), &json!({"source_archive":project_path,"run_id":run_id,"actual_Host_observations":frames.len(),"run_exact":true,"source_saved_or_edited":false,"ordinary_Native_verified":false,"whole_factory_accepted":false}));
}

pub(super) fn build(root: &Path, staged: &Path, output: &Path) {
    assert!(!output.exists(), "Use a new output, retain previous build evidence");
    std::fs::create_dir_all(output).unwrap();
    let seed = read(&staged.join("seed.json"));
    let adoption = read(&staged.join("adoption.json"));
    let tree=Sandbox::new();
    let seed_directory=tree.directory("seed");
    proof::seed(&seed_directory,&seed);
    let ui=id("full-factory-ui");
    let mut host=Harness {host:ProjectHost::new(id(&format!("full-factory-host-{}",std::process::id())),ui.clone()),ui,tree};
    let imported=host.import(&seed_directory);host.finish_one();
    let draft=host.imported(&imported);
    write(&output.join("actual-import-draft.json"),&serde_json::to_value(&draft).unwrap());
    let _:EditCommit=ok(host.adopt(&imported,&draft,false));
    let catalog = read(&root.join(adoption["catalog"].as_str().unwrap()));
    for entry in catalog["register_order"].as_array().unwrap() {
        host.host.register_bundled_module_directory(&root.join(entry["directory"].as_str().unwrap())).unwrap();
    }
    for registry in adoption["item_registrations"].as_array().unwrap() {
        let registry = read(&root.join(registry.as_str().unwrap()));
        for entry in registry["register_order"].as_array().unwrap() {
            host.host.register_bundled_module_directory(&root.join(entry["directory"].as_str().unwrap())).unwrap();
        }
    }
    let mut commands = vec![];
    for device in adoption["devices"].as_array().unwrap() {
        commands.push(json!({"kind":"adopt_module","data":{"module_ref":device["module_ref"],"targets":[{"kind":"device","id":device["target_id"]}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}));
    }
    for item in adoption["items"].as_array().unwrap() {
        commands.push(json!({"kind":"adopt_module","data":{"module_ref":{"id":item["id"],"version":item["version"]},"targets":[{"kind":"item_type","id":item["item_type"]}],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}));
    }
    let first: EditCommit = ok(host.apply(1, json!(commands)));
    let mut commands = vec![];
    for (key, kind) in [("restore_devices","device"),("restore_routes","route"),("restore_connections","connection")] {
        for row in adoption[key].as_array().unwrap() { commands.push(put(kind, row.clone())); }
    }
    let mut scenario=adoption["restore_scenario"].clone();
    let current=host.object("scenario","PF-ALL-NORMAL");
    scenario["revision"]=current["revision"].clone();
    scenario["project_revision"]=serde_json::to_value(&first.new_revision).unwrap();
    commands.push(put("scenario", scenario));
    let _: EditCommit = ok(host.apply(first.new_revision.0.value(), json!(commands)));
    let project = host.object("project", "P1");
    assert_eq!(project["devices"].as_array().unwrap().len(),92);
    write(&output.join("actual-project.json"), &project);
    let task = host.run("PF-ALL-NORMAL"); host.finish_one();
    let run = host.completed_run(&task);
    let actual: Value = serde_json::to_value(&run).unwrap();
    write(&output.join("actual-normal-run.json"), &actual);
    let revision = host.context().project.unwrap().revision.0.value();
    let saved=output.join("project");std::fs::create_dir(&saved).unwrap();
    let task=host.save(revision,&saved);host.finish_one();
    assert!(matches!(host.task(&task,true).task.state,ComputationStatus::Completed));
    let mut reopened=Harness::from_seed(Sandbox::new(), &saved);
    assert_eq!(reopened.object("project","P1"),project);
    assert_eq!(serde_json::to_value(reopened.full(&run.id)).unwrap(),actual);
    write(&output.join("receipt.json"), &json!({"status":"actual_Host_layout_and_normal_cells_saved_reopened","devices":92,"halls":1,"run_id":run.id,"jobs":run.jobs.len(),"acceptance_events":run.acceptance_events.len(),"actual_project_saved":saved,"fresh_Host_reopen_exact":true,"connected_factory_accepted":false,"Native_verified":false,"remaining":"Connect actual compatible product routes and transport; build template adoption consumer; Native scene and cloud-phase acceptance."}));
    println!("Actual Host 92-device layout and normal-use cells saved/reopened: {}",output.display());
}

pub(super) fn connect(project_path: &Path, staged: &Path, output: &Path) {
    connect_routes(project_path, staged, output, false, false)
}

pub(super) fn families(project_path: &Path, staged: &Path, output: &Path) {
    connect_routes(project_path, staged, output, true, false)
}

pub(super) fn bags(project_path: &Path, staged: &Path, output: &Path) {
    connect_routes(project_path, staged, output, true, true)
}

pub(super) fn materials(project_path: &Path, staged: &Path, output: &Path) {
    connect_routes(project_path, staged, output, true, true)
}

/// Reuse the already saved positive factory; changed scenarios need no module
/// re-adoption or repeated full frozen-module export.
pub(super) fn checks(project_path: &Path, staged: &Path, output: &Path) {
    assert!(!output.exists(), "Retain prior checks");
    std::fs::create_dir_all(output).unwrap();
    let plan=read(&staged.join("commands.json"));
    let mut host=Harness::from_seed(Sandbox::new(),project_path);
    let mut results=vec![];
    for check in plan["quality_checks"].as_array().unwrap() {
        let mut check=check.clone();
        let revision=host.context().project.unwrap().revision.0.value();
        check["project_revision"]=json!(revision);
        let _:EditCommit=ok(host.apply(revision,json!([put("scenario",check.clone())])));
        let identity=check["id"].as_str().unwrap();
        let task=host.run(identity);host.finish_one();
        let run=host.completed_run(&task);
        write(&output.join(format!("actual-{identity}-run.json")),&serde_json::to_value(&run).unwrap());
        for order in &run.orders {
            let expected=plan["material_checks"][identity].get(order.order_id.as_str())
                .unwrap_or(&plan["expected"]["accepted_orders"][order.order_id.as_str()]).as_u64().unwrap();
            assert_eq!(order.accepted.value.coefficient.value(),expected,"Affected material boundary or unaffected order changed");
        }
        results.push(json!({"scenario":identity,"orders":run.orders,"run_id":run.id}));
    }
    write(&output.join("receipt.json"),&json!({"state":"actual_host_scenario_checks_passed","checks":results,"source_saved_or_edited":false}));
    println!("Actual Host scenario checks passed: {}",output.display());
}

fn connect_routes(project_path: &Path, staged: &Path, output: &Path, families: bool, bags: bool) {
    assert!(!output.exists(), "Retain previous attempts; choose a new output");
    std::fs::create_dir_all(output).unwrap();
    let plan=read(&staged.join("commands.json"));
    let mut host=Harness::from_seed(Sandbox::new(),project_path);
    let old_context=host.context();
    let old_project=host.object("project","P1");
    let mut old_runs=vec![];
    for summary in &old_context.run_summaries {
        old_runs.push(host.full(&summary.run_id));
    }
    let mut adopt=vec![];
    for module in plan["new_modules"].as_array().unwrap() {
        host.host.register_bundled_module_directory(Path::new(module["directory"].as_str().unwrap())).unwrap();
        adopt.push(json!({"kind":"adopt_module","data":{"module_ref":{"id":module["id"],"version":module["version"]},"targets":[],"new_instances":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"explicit_seed":null}}));
    }
    let base=host.context().project.unwrap().revision.0.value();
    assert_eq!(base,plan["base_revision"].as_u64().unwrap());
    let adopted:EditCommit=ok(host.apply(base,json!(adopt)));
    let mut commands=plan["commands"].as_array().unwrap().clone();
    for command in &mut commands {
        if command["kind"]=="put_object" && command["data"]["kind"]=="scenario" {
            command["data"]["data"]["project_revision"]=serde_json::to_value(&adopted.new_revision).unwrap();
        }
    }
    commands.push(json!({"kind":"set_project_settings","data":{"title":"纸包装多产品工厂 · 连续生产与物流","time_reference":old_project["time_reference"],"generation_context":old_project["generation_context"],"sources":old_project["sources"],"setup_policies":old_project["setup_policies"]}}));
    let _:EditCommit=ok(host.apply(adopted.new_revision.0.value(),json!(commands)));
    let project=host.object("project","P1");
    let scenario=plan["scenario_id"].as_str().unwrap();
    write(&output.join("actual-project.json"),&project);
    write(&output.join("actual-scenario.json"),&host.object("scenario",scenario));
    let task=host.run(scenario);host.finish_one();
    let run=host.completed_run(&task);
    let actual=serde_json::to_value(&run).unwrap();
    write(&output.join("actual-connected-run.json"),&actual);
    let manufacturing_jobs=run.jobs.iter().filter(|j|matches!(j.execution_ref,JobExecutionRef::ProjectDevice {..})).count();
    let expected_jobs=plan["expected"]["manufacturing_jobs"].as_u64().map(|v|v as usize).unwrap_or(if bags {145} else if families {125} else {108});
    assert_eq!(manufacturing_jobs,expected_jobs,"Independent cells plus exact declared product-cycle yields; transport cycles counted separately");
    assert!(run.jobs.iter().all(|j|matches!(j.state,JobStatus::Completed)),"Every declared manufacturing job must complete");
    let expected_orders = if plan["expected"]["accepted_orders"].is_object() {plan["expected"]["accepted_orders"].clone()}
        else if bags {json!({"PF-ORDER-CASE":3,"PF-ORDER-SLEEVE":3,"PF-ORDER-TUBE":6,"PF-ORDER-LABEL":18,"PF-ORDER-BAG":3})}
        else if families {json!({"PF-ORDER-CASE":3,"PF-ORDER-SLEEVE":3,"PF-ORDER-TUBE":6,"PF-ORDER-LABEL":18})}
        else {json!({"PF-ORDER-CASE":3,"PF-ORDER-SLEEVE":3})};
    assert_eq!(run.orders.len(),expected_orders.as_object().unwrap().len());
    for order in &run.orders {assert_eq!(order.accepted.value.coefficient.value(),expected_orders[order.order_id.as_str()].as_u64().unwrap());assert_eq!(order.outstanding.value.coefficient.value(),0);}
    let expected_trips=plan["expected"]["transport_trips"].as_u64().map(|v|v as usize).unwrap_or(if bags {78} else if families {54} else {27});
    assert_eq!(run.transport.len(),expected_trips,"Declared per-leg quantities and full tray capacities include split products");
    assert!(run.transport.iter().all(|t|matches!(t.phase,TransportPhase::Returned)));
    let frozen:DomainObject=ok(host.call("get_object",json!({"object_id":{"kind":"project","id":"P1"},"source":{"kind":"run_snapshot","data":{"run_id":run.id,"at_ms":0}},"node_index":null})));
    write(&output.join("actual-frozen-project.json"),&serde_json::to_value(&frozen).unwrap()["data"]);
    let mut times=std::collections::BTreeSet::from([0_u64,100000,500000,1000000,run.window.end_ms.0.value()]);
    for trip in run.transport.iter().take(8) {
        if let (Some(start),Some(end))=(&trip.departed_ms,&trip.arrived_ms) {times.insert((start.0.value()+end.0.value())/2);}
    }
    let mut frames=vec![];
    for at in times {
        let mut objects=vec![];
        let mut selectors=vec![json!([])];
        selectors.extend(run.jobs.chunks(200).map(|chunk|json!(chunk.iter().map(|j|json!({"kind":"job","id":j.id})).collect::<Vec<_>>())));
        for selected in selectors {
            let mut after:Option<usize>=None;
            loop {
                let view:RunView=ok(host.call("get_run",json!({"run_id":run.id,"query":{"kind":"objects_at","data":{"at_ms":at,"objects":selected,"page":{"after_index":after,"limit":200}}}})));
                let RunView::Objects(page)=view else {panic!("Actual observation page required")};
                objects.extend(page.objects);after=page.next_index.map(|v|v.value() as usize);
                if after.is_none(){break}
            }
        }
        frames.push(json!({"run_id":run.id,"at_ms":at,"objects":objects,"next_index":null}));
    }
    write(&output.join("actual-observation-frames.json"),&json!(frames));
    let mut modules=vec![];
    for reference in project["module_refs"].as_array().unwrap() {
        let value:DomainObject=ok(host.call("get_object",json!({"object_id":{"kind":"module","id":reference["id"]},"source":{"kind":"run_module_version","data":{"run_id":run.id,"at_ms":0,"module_ref":reference}},"node_index":null})));
        let DomainObject::Module(package)=value else {panic!("Exact frozen module required")};modules.push(package);
    }
    write(&output.join("actual-frozen-modules.json"),&serde_json::to_value(&modules).unwrap());
    let mut quality_checks=vec![];
    for check in plan["quality_checks"].as_array().unwrap() {
        let mut check=check.clone();
        let rev=host.context().project.unwrap().revision.0.value();
        check["project_revision"]=json!(rev);
        let _:EditCommit=ok(host.apply(rev,json!([put("scenario",check.clone())])));
        let check_id=check["id"].as_str().unwrap();
        let task=host.run(check_id);host.finish_one();
        let checked=host.completed_run(&task);
        write(&output.join(format!("actual-{check_id}-run.json")),&serde_json::to_value(&checked).unwrap());
        if let Some(overrides)=plan["material_checks"][check_id].as_object() {
            for order in &checked.orders {
                let expected=overrides.get(order.order_id.as_str()).unwrap_or(&expected_orders[order.order_id.as_str()]).as_u64().unwrap();
                assert_eq!(order.accepted.value.coefficient.value(),expected,"Material boundary applies only to the actual affected products");
            }
            quality_checks.push(json!({"scenario":check_id,"orders":checked.orders,"run_id":checked.id}));
            continue;
        }
        let target=if bags {"PF-ORDER-BAG"} else if families {"PF-ORDER-LABEL"} else {"PF-ORDER-CASE"};
        let order=checked.orders.iter().find(|o|o.order_id.as_str()==target).unwrap();
        let accepted=order.accepted.value.coefficient.value();
        if bags {assert_eq!(accepted,if check_id=="PF-BAG-SHORT-PATCHES" {2} else {0},"A bag requires actual webs, two cords, four patches and attachment adhesive");}
        else if check_id.ends_with("FAIL") { assert_eq!(accepted,if families {12} else {2},"Failed stock must never reach final acceptance"); }
        else { assert_eq!(accepted,0,"Missing preset holds the first item and must not yield accepted products"); }
        for unaffected in checked.orders.iter().filter(|o|o.order_id.as_str()!=target) {
            assert_eq!(unaffected.accepted.value.coefficient.value(),expected_orders[unaffected.order_id.as_str()].as_u64().unwrap());
        }
        quality_checks.push(json!({"scenario":check_id,"order":target,"accepted":accepted,"run_id":checked.id}));
    }
    let project=host.object("project","P1");
    write(&output.join("actual-project.json"),&project);
    let saved=output.join("project");std::fs::create_dir(&saved).unwrap();
    let rev=host.context().project.unwrap().revision.0.value();let task=host.save(rev,&saved);host.finish_one();
    let save_result=host.task(&task,true);
    write(&output.join("actual-save-task.json"),&serde_json::to_value(&save_result).unwrap());
    assert!(matches!(save_result.task.state,ComputationStatus::Completed),"{}",serde_json::to_value(&save_result).unwrap());
    let mut reopen=Harness::from_seed(Sandbox::new(),&saved);
    assert_eq!(reopen.object("project","P1"),project);
    assert_eq!(serde_json::to_value(reopen.full(&run.id)).unwrap(),actual);
    for prior in &old_runs {assert_eq!(serde_json::to_value(reopen.full(&prior.id)).unwrap(),serde_json::to_value(prior).unwrap());}
    write(&output.join("receipt.json"),&json!({"status":"actual_connected_product_routes_transport_saved_reopened","devices":92,"manufacturing_jobs":manufacturing_jobs,"all_jobs_including_transport":run.jobs.len(),"transport_trips":run.transport.len(),"orders":run.orders,"acceptance_events":run.acceptance_events,"quality_checks":quality_checks,"prior_Run_exactly_retained":true,"fresh_Host_reopen_exact":true,"actual_project_saved":saved,"Native_verified":false,"whole_factory_accepted":false,"remaining":"Other product routes, complete template adoption, Native transport visual projection, cloud phases and original gap list"}));
    println!("Connected product routes,{} real trips,{} accepted products and exact reopen verified: {}",run.transport.len(),run.orders.iter().map(|o|o.accepted.value.coefficient.value()).sum::<u64>(),output.display());
}

/// Select the real bundled template through public Host preview/adoption APIs,
/// then execute its remapped source and reopen it in a fresh Host.
pub(super) fn template(root: &Path, output: &Path) {
    template_routes(root, output, false, false)
}

pub(super) fn template_families(root: &Path, output: &Path) {
    template_routes(root, output, true, false)
}

pub(super) fn template_bags(root: &Path, output: &Path) {
    template_routes(root, output, true, true)
}

fn template_routes(root: &Path, output: &Path, families: bool, bags: bool) {
    assert!(!output.exists(), "Retain prior adoption attempts");
    std::fs::create_dir_all(output).unwrap();
    let modules=root.join("modules");
    let factory=modules.join("templates/paper-packaging-factory");
    let registry=read(&factory.join("registration.json"));
    let source=read(&factory.join("candidate/example-project.json"))["data"].clone();
    let source_scenario=read(&factory.join("candidate/source-scenario.json"))["data"].clone();
    let mut project=source.clone();
    for value in project.as_object_mut().unwrap().values_mut() {
        if value.is_array(){*value=json!([]);}
    }
    project["id"]=json!("P1");project["title"]=json!("从模板创建的纸包装工厂");
    project["views"]["selected_hall"]=Value::Null;project["views"]["selected_objects"]=json!([]);
    project["scenario_refs"]=json!(["EMPTY"]);
    let mut empty_scenario=source_scenario.clone();
    for value in empty_scenario.as_object_mut().unwrap().values_mut(){if value.is_array(){*value=json!([]);}}
    empty_scenario["id"]=json!("EMPTY");empty_scenario["initial_conditions"]=json!({"items":[],"initial_setup_families":{},"declared_quality_decisions":[],"material_allocations":[]});
    let mut host=Harness::from_value(json!({"project":project,"scenario":empty_scenario,"definitions":[]}));
    for entry in registry["register_order"].as_array().unwrap(){
        host.host.register_bundled_module_directory(&modules.join(entry["directory"].as_str().unwrap())).unwrap();
    }
    host.host.register_bundled_template_directory(&factory.join("candidate")).unwrap();
    let reference=&registry["template_ref"];
    let base=host.context().project.unwrap().revision.0.value();
    let _:EditCommit=ok(host.apply(base,json!([{"kind":"adopt_module","data":{"module_ref":reference,"targets":[],"bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}}])));
    let base=host.context().project.unwrap().revision;
    let mut params=json!({"object_id":{"kind":"module","id":reference["id"]},"node_index":null,"source":{"kind":"template_preview","data":{"module_ref":reference,"base_revision":base,"choices":null,"seed":null}}});
    let bare:DomainObject=ok(host.call("get_object",params.clone()));
    let DomainObject::TemplatePreview(bare)=bare else {panic!("Template preview required")};
    params["source"]["data"]["choices"]=json!({"source_project":bare.source_project,"namespace":"factory","resources":source["resources"].as_array().unwrap().iter().map(|r|(r["id"].as_str().unwrap().to_owned(),Value::Null)).collect::<serde_json::Map<_,_>>(),"reuse_items":{},"include_example_product":true,"site_envelope":null});
    params["source"]["data"]["seed"]=json!({"scenario_id":"FACTORY-PRODUCTION","supply":true,"demand":true,"initial_quality":source_scenario["initial_conditions"]["items"].as_array().unwrap().iter().map(|i|(i["id"].as_str().unwrap().to_owned(),i["quality"].clone())).collect::<serde_json::Map<_,_>>(),"adopt_declared_inspection_results":true});
    let preview:DomainObject=ok(host.call("get_object",params));
    write(&output.join("actual-template-preview.json"),&serde_json::to_value(&preview).unwrap());
    let DomainObject::TemplatePreview(preview)=preview else {panic!("Prepared preview required")};
    let command=preview.command.expect("Actual template must produce an adoption command");
    let base=host.context().project.unwrap().revision.0.value();
    let _:EditCommit=ok(host.apply(base,json!([{"kind":"instantiate_template","data":command}])));
    let project=host.object("project","P1");assert_eq!(project["devices"].as_array().unwrap().len(),92);
    assert_eq!(project["setup_policies"].as_array().unwrap().len(),92);
    assert!(project["devices"].as_array().unwrap().iter().all(|d|d["id"].as_str().unwrap().starts_with("factory:")));
    let scenario=host.object("scenario","FACTORY-PRODUCTION");
    assert_eq!(scenario["supplies"].as_array().unwrap().len(),if bags {26} else if families {18} else {9});
    let task=host.run("FACTORY-PRODUCTION");host.finish_one();let run=host.completed_run(&task);
    let expected=if bags {[("CASE",3),("SLEEVE",3),("TUBE",6),("LABEL",18),("BAG",3)].to_vec()}
        else if families {[("CASE",3),("SLEEVE",3),("TUBE",6),("LABEL",18)].to_vec()} else {[("CASE",3),("SLEEVE",3)].to_vec()};
    assert_eq!(run.orders.len(),expected.len());
    for (name,count) in expected {
        let order=run.orders.iter().find(|o|o.order_id.as_str().ends_with(&format!("PF-ORDER-{name}"))).unwrap();
        assert_eq!(order.accepted.value.coefficient.value(),count);assert_eq!(order.outstanding.value.coefficient.value(),0);
    }
    assert_eq!(run.transport.len(),if bags {78} else if families {54} else {27});assert!(run.transport.iter().all(|t|matches!(t.phase,TransportPhase::Returned)));
    let jobs=run.jobs.iter().filter(|j|matches!(j.execution_ref,JobExecutionRef::ProjectDevice {..})).count();assert_eq!(jobs,if bags {145} else if families {125} else {108});
    assert!(run.jobs.iter().all(|j|matches!(j.state,JobStatus::Completed)));
    let actual=serde_json::to_value(&run).unwrap();
    write(&output.join("actual-project.json"),&project);write(&output.join("actual-scenario.json"),&scenario);write(&output.join("actual-run.json"),&actual);
    let saved=output.join("project");std::fs::create_dir(&saved).unwrap();
    let revision=host.context().project.unwrap().revision.0.value();let task=host.save(revision,&saved);host.finish_one();
    let state=host.task(&task,true);write(&output.join("actual-save-task.json"),&serde_json::to_value(&state).unwrap());
    assert!(matches!(state.task.state,ComputationStatus::Completed),"{state:?}");
    let mut reopen=Harness::from_seed(Sandbox::new(),&saved);
    assert_eq!(reopen.object("project","P1"),project);assert_eq!(serde_json::to_value(reopen.full(&run.id)).unwrap(),actual);
    write(&output.join("receipt.json"),&json!({"status":"actual_public_Host_template_adopted_run_saved_reopened","template_ref":reference,"devices":92,"remapped_setup_policies":92,"manufacturing_jobs":jobs,"transport_trips":run.transport.len(),"accepted_products":run.orders.iter().map(|o|o.accepted.value.coefficient.value()).sum::<u64>(),"fresh_Host_reopen_exact":true,"project":saved,"ordinary_Native_verified":false,"whole_factory_accepted":false,"remaining":"Other product routes and current ordinary Native; independent cells are not complete factory flows"}));
    println!("Actual factory template adopted, run and reopened: {}",output.display());
}
