//! Actual frozen production history and portable Host saves; no Native/visual verdict.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use support::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}
fn output(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn fresh(label: &str) -> Harness {
    let ui = id(&format!("{label}-ui"));
    Harness {
        host: ProjectHost::new(id(label), ui.clone()),
        ui,
        tree: Sandbox::new(),
    }
}
fn api(h: &mut Harness, p: Option<&ProjectId>, op: &str, params: Value) -> Value {
    let request=serde_json::from_value(json!({"api_version":"0.1","request_id":"v04-production-history",
        "project_id":p,"scope":{"kind":if p.is_some(){"project"}else{"workspace"}},"operation":op,"params":params})).unwrap();
    h.host.dispatch(&h.ui, request)
}
fn context(h: &mut Harness) -> ProjectContext {
    ok(api(h, None, "get_context", json!({"selection":[]})))
}
fn object(h: &mut Harness, p: &ProjectId, kind: &str, name: &str, source: Value) -> Value {
    let obj: DomainObject = ok(api(
        h,
        Some(p),
        "get_object",
        json!({"object_id":{"kind":kind,"id":name},"source":source,"node_index":null}),
    ));
    json!(obj)["data"].clone()
}
fn project(h: &mut Harness, p: &ProjectId) -> Value {
    object(h, p, "project", p.as_str(), json!({"kind":"draft"}))
}
fn apply(h: &mut Harness, p: &ProjectId, commands: Value) {
    let base = project(h, p)["revision"].clone();
    let _: EditCommit = ok(api(
        h,
        Some(p),
        "apply_commands",
        json!({"base_revision":base,"commands":commands}),
    ));
}
fn adopt(r: &Value, targets: Value) -> Value {
    json!({"kind":"adopt_module","data":{"module_ref":r,"targets":targets,
    "bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}})
}
fn finish(h: &mut Harness, t: &TaskHandle, p: Option<&ProjectId>) -> TaskState {
    h.finish_one();
    let state: TaskState = ok(api(h, p, "get_task", json!({"task_id":t.id})));
    assert!(
        matches!(state.task.state, ComputationStatus::Completed),
        "actual task failed: {state:?}"
    );
    state
}
fn save(h: &mut Harness, p: &ProjectId, dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::SaveProject, dir.to_owned())
        .unwrap();
    let revision = project(h, p)["revision"].clone();
    let task: TaskHandle = ok(api(
        h,
        Some(p),
        "save_project",
        json!({"revision":revision,"selected_directory_ref":selected}),
    ));
    finish(h, &task, Some(p));
}
fn open(h: &mut Harness, dir: &Path) -> ProjectId {
    let task = h.import(dir);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    context(h).project.unwrap().id
}
fn refs(path: &str) -> Vec<Value> {
    read(&root().join(path))["register_order"]
        .as_array()
        .unwrap()
        .clone()
}
fn register(h: &mut Harness, entries: &[Value]) {
    for e in entries {
        h.host
            .register_bundled_module_directory(&root().join(e["directory"].as_str().unwrap()))
            .unwrap();
    }
}
fn module_ref(e: &Value) -> Value {
    json!({"id":e["id"],"version":e["version"]})
}


fn run(h: &mut Harness, p: &ProjectId, input: &Value) -> RunResult {
    let current = project(h, p)["revision"].clone();
    let t: TaskHandle = ok(api(
        h,
        Some(p),
        "run_scenario",
        json!({"scenario_id":input["scenario"]["id"],"input_revision":current,
        "observation_window":input["scenario"]["window"],"intent":{"kind":"project_scenario"}}),
    ));
    let state = finish(h, &t, Some(p));
    let Some(TaskOutput::Run(summary)) = state.result else {
        panic!("real Run")
    };
    full(h, p, &summary.run_id)
}
fn full(h: &mut Harness, p: &ProjectId, r: &RunId) -> RunResult {
    let view: RunView = ok(api(
        h,
        Some(p),
        "get_run",
        json!({"run_id":r,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(value) = view else {
        panic!("complete Run")
    };
    *value
}

#[test]
fn noncu_media_controlled_image_gif_glb_actual_host_and_portable_history() {
 let input=read(&root().join("packages/commands/tests/fixtures/production/p1-phase-calendar.json"));
 let mut h=Harness::from_value(input.clone()); let p=context(&mut h).project.unwrap().id;
 let initial=project(&mut h,&p); let mut packs=vec![];
 for name in ["image-simple","model-simple","image","gif","model"] {
  let directory=root().join("modules/media/p302-controlled/installable").join(name);
  h.host.register_bundled_module_directory(&directory).unwrap();
  let package=read(&directory.join("module.json"))["data"].clone(); packs.push((name,directory,package));
 }
 assert_eq!(project(&mut h,&p),initial,"registration does not silently adopt");
 let base=root().join(".work/delivery/controlled-host"); fs::create_dir_all(&base).unwrap();
 let mut previous:Option<Value>=None; let mut runs=vec![]; let mut bytes_seen=vec![];
 for name in ["image","gif","model"] {
  let (_,_,package)=packs.iter().find(|(n,_,_)|*n==name).unwrap();
  let reference=module_ref(&package["definition"]);
  let mut commands:Vec<_>=packs.iter().map(|(_,_,v)|adopt(&module_ref(&v["definition"]),json!([]))).collect();
  commands.push(adopt(&reference,json!([{"kind":"device","id":"CNC"}])));
  apply(&mut h,&p,json!(commands));
  let r=run(&mut h,&p,&input); let actual=json!(r);
  let job=actual["jobs"].as_array().unwrap().iter().find(|j|j["operation_ref"]["id"]=="w5.op.phased").unwrap();
  assert_eq!(job["finished_at_ms"],45000);assert_eq!(job["state"],"completed");
  if let Some(old)=&previous {for field in ["events","final_items","acceptance_events","orders","resource_claims","waits","phase_work_intervals","jobs","computed_until_ms"] {assert_eq!(actual[field],old[field],"{name}/{field}");}}
  previous=Some(actual.clone());
  let source=json!({"kind":"run_module_version","data":{"run_id":r.id,"at_ms":0,"module_ref":reference}});
  let host_package=object(&mut h,&p,"module",reference["id"].as_str().unwrap(),source.clone());
  let source:ObjectSource=serde_json::from_value(source).unwrap();let revision=context(&mut h).project.unwrap().revision;
  for (asset,original) in host_package["assets"].as_array().unwrap().iter().zip(package["assets"].as_array().unwrap()) {
   let a:AssetRef=serde_json::from_value(asset.clone()).unwrap();let bytes=h.host.read_media_asset(&h.ui,&p,&ApiScope::Project,&revision,&source,&a).unwrap();
   let expected=fs::read(packs.iter().find(|(n,_,_)|*n==name).unwrap().1.join(original["path"].as_str().unwrap())).unwrap();assert_eq!(bytes,expected);
   let path=base.join(name).join(a.path.as_str());fs::create_dir_all(path.parent().unwrap()).unwrap();fs::write(&path,bytes).unwrap();
   bytes_seen.push(json!({"variant":name,"source":source,"asset":asset,"file":path.strip_prefix(root()).unwrap()}));
  }
  output(&base.join(name).join("module.json"),&host_package);
  let frozen=object(&mut h,&p,"project",p.as_str(),json!({"kind":"run_snapshot","data":{"run_id":r.id,"at_ms":0}}));
  output(&base.join(name).join("run.json"),&json!({"run":r,"project":frozen}));runs.push(r);
 }
 let saved=base.join("saved-project");save(&mut h,&p,&saved);let moved=base.join("receiver/moved-project");fs::create_dir_all(&moved).unwrap();copy(&saved,&moved);
 let mut receiver=fresh("controlled-independent-receiver");let rp=open(&mut receiver,&moved);assert_eq!(rp,p);
 for r in &runs {assert_eq!(json!(full(&mut receiver,&p,&r.id)),json!(r));}
 for row in &bytes_seen {let source:ObjectSource=serde_json::from_value(row["source"].clone()).unwrap();let a:AssetRef=serde_json::from_value(row["asset"].clone()).unwrap();let revision=context(&mut receiver).project.unwrap().revision;assert_eq!(receiver.host.read_media_asset(&receiver.ui,&p,&ApiScope::Project,&revision,&source,&a).unwrap(),fs::read(root().join(row["file"].as_str().unwrap())).unwrap());}
 output(&base.join("result.json"),&json!({"passed":true,"actual_host":true,"variants":["image","gif","model"],"byte_reads":bytes_seen,"run_ids":runs.iter().map(|r|&r.id).collect::<Vec<_>>(),"business_unchanged":true,"relocated_fresh_host":true,"GUI":false}));
}
