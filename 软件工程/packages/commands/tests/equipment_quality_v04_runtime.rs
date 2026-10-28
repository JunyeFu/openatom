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

// Use the real Host to namespace/carry the old packages. No test asset remapper.
fn legacy_library(original: &Value, dir: &Path) -> Vec<Value> {
    let mut dirs = BTreeMap::new();
    for e in fs::read_dir(root().join("modules/media/p302-line-grade/installable")).unwrap() {
        let path = e.unwrap().path().join("module.json");
        if !path.is_file() {
            continue;
        }
        let package = read(&path);
        let d = &package["data"]["definition"];
        dirs.insert(
            (
                d["id"].as_str().unwrap().to_owned(),
                d["version"].as_str().unwrap().to_owned(),
            ),
            path.parent().unwrap().to_owned(),
        );
    }
    let mut entries = vec![];
    for d in original["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["payload"]["category"] == "media" && d["version"] == "0.1.0")
    {
        let path = &dirs[&(d["id"].as_str().unwrap().to_owned(), "0.1.0".to_owned())];
        assert_eq!(read(&path.join("module.json"))["data"]["definition"], *d);
        entries.push(json!({"id":d["id"],"version":d["version"],"directory":path.strip_prefix(root()).unwrap().to_str().unwrap()}));
    }
    entries.extend(refs("modules/media/p302-l03-v03/registration.json"));
    assert_eq!(entries.len(), 56);
    for e in &entries {
        let package = read(
            &root()
                .join(e["directory"].as_str().unwrap())
                .join("module.json"),
        );
        assert!(
            original["definitions"]
                .as_array()
                .unwrap()
                .contains(&package["data"]["definition"])
        );
    }
    let mut h = fresh("runtime-legacy-library");
    register(&mut h, &entries);
    let _: EditCommit = ok(api(
        &mut h,
        None,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"create_project","data":{
        "title":"Unmodified original media packages","time_reference":{"label":"Host library preparation"},"departure":"stay"}}]}),
    ));
    let p = context(&mut h).project.unwrap().id;
    apply(
        &mut h,
        &p,
        json!(
            entries
                .iter()
                .map(|e| adopt(&module_ref(e), json!([])))
                .collect::<Vec<_>>()
        ),
    );
    save(&mut h, &p, dir);
    entries
}
fn seed_from_real_library(library: &Path, dir: &Path, value: &Value) {
    fs::create_dir_all(dir).unwrap();
    copy(library, dir);
    let mut manifest = read(&dir.join("project.json"))["data"].clone();
    // Keep all original Host module/asset documents and rights; replace only the
    // preparation Project by the exact frozen business fixture and its Scenario.
    manifest["documents"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["role"] != "active_project");
    let p = &value["project"];
    let s = &value["scenario"];
    manifest["project_id"] = p["id"].clone();
    manifest["project_revision"] = p["revision"].clone();
    manifest["title"] = p["title"].clone();
    manifest["documents"].as_array_mut().unwrap().push(document(
        "active_project",
        "project",
        "project",
        p["id"].as_str().unwrap(),
        "runtime-project.json",
        p["revision"].clone(),
    ));
    manifest["documents"].as_array_mut().unwrap().push(document(
        "scenario_input",
        "scenario",
        "scenario",
        s["id"].as_str().unwrap(),
        "runtime-scenario.json",
        s["revision"].clone(),
    ));
    output(
        &dir.join("runtime-project.json"),
        &json!({"schema_version":"1.0","data":p}),
    );
    output(
        &dir.join("runtime-scenario.json"),
        &json!({"schema_version":"1.0","data":s}),
    );
    for (i, d) in value["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, d)| d["payload"]["category"] != "media")
    {
        let path = format!("runtime-business-{i}.json");
        let r = module_ref(d);
        let mut e = document(
            "module_definition",
            "module",
            "module",
            d["id"].as_str().unwrap(),
            &path,
            Value::Null,
        );
        e["module_ref"] = r.clone();
        manifest["documents"].as_array_mut().unwrap().push(e);
        manifest["module_refs"].as_array_mut().unwrap().push(r);
        output(
            &dir.join(path),
            &json!({"schema_version":"1.0","data":{"definition":d,"assets":[],"files":[],"dependencies":[],"rights":[],"issues":[]}}),
        );
    }
    output(
        &dir.join("project.json"),
        &json!({"schema_version":"1.0","data":manifest}),
    );
}
fn same_business(mut actual: Value, mut source: Value) {
    source = json!(serde_json::from_value::<ProjectState>(source).unwrap());
    for v in [&mut actual, &mut source] {
        for key in ["revision", "module_refs"] {
            v.as_object_mut().unwrap().remove(key);
        }
        for device in v["devices"].as_array_mut().unwrap() {
            device
                .as_object_mut()
                .unwrap()
                .remove("presentation_override");
        }
        for resource in v["resources"].as_array_mut().unwrap() {
            resource.as_object_mut().unwrap().remove("presentation");
        }
    }
    if actual != source {
        output(
            &root().join(".work/model-v04-runtime-reception/delivery/business-comparison-failure.json"),
            &json!({"actual":actual,"expected":source}),
        );
        let keys: Vec<_> = actual
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| actual[*k] != source[*k])
            .collect();
        panic!(
            "business declaration differs in fields {keys:?}; see runtime-delivery/business-comparison-failure.json"
        );
    }
}
fn media_setup(h: &mut Harness, p: &ProjectId, input: &Value, entries: &[Value]) {
    register(h, entries);
    let mut commands: Vec<_> = entries
        .iter()
        .map(|e| adopt(&module_ref(e), json!([])))
        .collect();
    for (device, family) in [
        ("ASM01", "assembly"),
        ("R01", "robot"),
        ("CV01", "conveyor"),
        ("V01", "inspection"),
        ("RW01", "rework"),
        ("CV02", "conveyor"),
        ("PK01", "packing"),
        ("CASE01", "sealer"),
        ("CNC", "assembly"),
    ] {
        if input["project"]["devices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["id"] == device)
        {
            commands.push(adopt(
                &json!({"id":format!("p302.line-grade.{family}"),"version":"0.4.0"}),
                json!([{"kind":"device","id":device}]),
            ));
        }
    }
    if let Some(pallet) = input["project"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "PAL01")
    {
        let mut pallet = pallet.clone();
        for key in ["source"] {
            pallet["presentation"][key] = json!({"kind":"media_ref","data":{"id":"p302.line-grade.pallet","version":"0.4.0"}});
        }
        pallet["presentation"]["lod"]["detailed"] = pallet["presentation"]["source"].clone();
        pallet["presentation"]["lod"]["simplified"] = json!({"kind":"media_ref","data":{"id":"p302.line-grade.pallet.simple","version":"0.4.0"}});
        commands.push(put("resource", pallet));
    }
    apply(h, p, json!(commands));
    let adopted = project(h, p);
    same_business(adopted.clone(), input["project"].clone());
    for r in input["project"]["module_refs"].as_array().unwrap() {
        assert!(
            adopted["module_refs"].as_array().unwrap().contains(r),
            "original exact ModuleRef lost: {r}"
        );
    }
    for e in entries {
        assert!(
            adopted["module_refs"]
                .as_array()
                .unwrap()
                .contains(&module_ref(e))
        );
    }
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
fn get_objects(h: &mut Harness, p: &ProjectId, r: &RunId, at: u64, selectors: Value) -> Vec<Value> {
    let mut after = Value::Null;
    let mut values = vec![];
    loop {
        let view: RunView = ok(api(
            h,
            Some(p),
            "get_run",
            json!({"run_id":r,"query":{"kind":"objects_at","data":{
        "at_ms":at,"objects":selectors,"page":{"after_index":after,"limit":1000}}}}),
        ));
        let RunView::Objects(page) = view else {
            panic!("actual historical objects")
        };
        values.extend(page.objects.iter().map(|o| json!(o)["data"].clone()));
        after = json!(page.next_index);
        if page.next_index.is_none() {
            break;
        }
    }
    values
}
fn at(h: &mut Harness, p: &ProjectId, r: &RunResult, t: u64) -> Value {
    let items = get_objects(h, p, &r.id, t, json!([]));
    let mut jobs = vec![];
    for chunk in r.jobs.chunks(200) {
        jobs.extend(get_objects(
            h,
            p,
            &r.id,
            t,
            json!(
                chunk
                    .iter()
                    .map(|j| json!({"kind":"job","id":j.id}))
                    .collect::<Vec<_>>()
            ),
        ));
    }
    let data = json!(r);
    let frontier = t == r.computed_until_ms.as_ref().unwrap().0.value();
    let boundary = |cause: &Value| cause == "computed_boundary" || cause == "observation_end";
    let active_claims: Vec<_> = data["resource_claims"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| {
            c["intervals"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .any(|(i, interval)| {
                    contains(interval, t)
                        || (frontier
                            && interval["end_ms"] == t
                            && boundary(&c["interval_end_causes"][i])
                            && jobs.iter().any(|j| {
                                j["id"] == c["job_id"]
                                    && ["job_resources", "phase_resources"].iter().any(|key| {
                                        j[*key].as_array().unwrap().contains(&c["resource_id"])
                                    })
                            }))
                })
        })
        .cloned()
        .collect();
    let active_waits: Vec<_> = data["waits"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| {
            contains(&w["interval"], t)
                || (frontier
                    && w["interval"]["end_ms"] == t
                    && boundary(&w["end_cause"])
                    && jobs.iter().any(|j| {
                        j["id"] == w["object_ref"]["id"]
                            && (j["state"] == "waiting_resource" || j["state"] == "waiting_output")
                            && (w["phase_ref"].is_null() || w["phase_ref"] == j["phase_ref"])
                    }))
        })
        .cloned()
        .collect();
    json!({"at_ms":t,"run_id":r.id,"scope":{"kind":"project"},"project_id":p,"current_project_revision":project(h,p)["revision"],
        "run_input_ref":r.input_ref,"jobs":jobs,"items":items,"active_resource_claims":active_claims,"active_waits":active_waits})
}
fn contains(i: &Value, t: u64) -> bool {
    i["start_ms"].as_u64().unwrap() <= t && t < i["end_ms"].as_u64().unwrap()
}
fn device_job(r: &Value, device: &str) -> Value {
    r["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|j| {
            j["execution_ref"]["data"]["device_id"] == device && !j["started_at_ms"].is_null()
        })
        .min_by_key(|j| j["started_at_ms"].as_u64().unwrap())
        .unwrap_or_else(|| panic!("started job {device}: {}", r["jobs"]))
        .clone()
}
fn item(point: &Value, key: &Value) -> Value {
    point["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == *key)
        .unwrap()
        .clone()
}
fn job(point: &Value, key: &Value) -> Value {
    point["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|j| j["id"] == *key)
        .unwrap()
        .clone()
}
fn moments(r: &RunResult) -> BTreeSet<u64> {
    let data = json!(r);
    let end = r.computed_until_ms.as_ref().unwrap().0.value();
    let mut times = BTreeSet::from([0, end]);
    let mut span = |a: u64, b: u64| {
        for t in [a, a + (b - a) / 2, b.saturating_sub(1), b] {
            if t <= end {
                times.insert(t);
            }
        }
    };
    for j in data["jobs"].as_array().unwrap() {
        if let Some(a) = j["started_at_ms"].as_u64() {
            span(a, j["finished_at_ms"].as_u64().unwrap_or(end));
        }
    }
    for w in data["phase_work_intervals"].as_array().unwrap() {
        span(
            w["interval"]["start_ms"].as_u64().unwrap(),
            w["interval"]["end_ms"].as_u64().unwrap(),
        );
    }
    for w in data["waits"].as_array().unwrap() {
        span(
            w["interval"]["start_ms"].as_u64().unwrap(),
            w["interval"]["end_ms"].as_u64().unwrap(),
        );
    }
    for a in data["acceptance_events"].as_array().unwrap() {
        let t = a["at_ms"].as_u64().unwrap();
        times.extend([t.saturating_sub(1), t]);
    }
    times
}
fn media_bytes(h: &mut Harness, p: &ProjectId, r: &RunResult, entries: &[Value]) -> Value {
    let revision = context(h).project.unwrap().revision;
    let mut checked = vec![];
    for e in entries {
        let reference: ModuleRef = serde_json::from_value(module_ref(e)).unwrap();
        let source = ObjectSource::RunModuleVersion {
            run_id: r.id.clone(),
            at_ms: SimTimeMs(0u64.try_into().unwrap()),
            module_ref: reference.clone(),
        };
        let actual: ModulePackage =
            serde_json::from_value(object(h, p, "module", reference.id.as_str(), json!(source)))
                .unwrap();
        let input = read(
            &root()
                .join(e["directory"].as_str().unwrap())
                .join("module.json"),
        )["data"]
            .clone();
        assert_eq!(json!(actual.dependencies), input["dependencies"]);
        assert_eq!(
            json!(actual.definition.contribution.license),
            input["definition"]["contribution"]["license"]
        );
        let mut bytes = 0;
        for (a, b) in actual
            .assets
            .iter()
            .zip(input["assets"].as_array().unwrap())
        {
            let actual = h
                .host
                .read_media_asset(&h.ui, p, &ApiScope::Project, &revision, &source, a)
                .unwrap();
            let expected = fs::read(
                root()
                    .join(e["directory"].as_str().unwrap())
                    .join(b["path"].as_str().unwrap()),
            )
            .unwrap();
            assert_eq!(actual, expected);
            bytes += actual.len();
            if b["kind"] == "glb" {
                let size = u32::from_le_bytes(actual[12..16].try_into().unwrap()) as usize;
                let gltf: Value = serde_json::from_slice(&actual[20..20 + size]).unwrap();
                let presentation = &input["definition"]["payload"]["data"];
                assert_eq!(
                    gltf["nodes"].as_array().unwrap().len(),
                    presentation["nodes"].as_array().unwrap().len(),
                    "GLB node count differs from its exact declaration: {}",
                    e["id"]
                );
                for node in presentation["nodes"].as_array().unwrap() {
                    assert_eq!(
                        gltf["nodes"][node["id"].as_u64().unwrap() as usize]["name"],
                        node["name"]
                    );
                }
                for clip in presentation["timeline"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|clip| clip["source"]["kind"] == "model_animation")
                {
                    assert!(
                        clip["source"]["data"]["animation_index"].as_u64().unwrap()
                            < (gltf["animations"].as_array().unwrap().len() as u64)
                    );
                }
            }
        }
        assert_eq!(actual.assets.len(), 2);
        checked.push(
            json!({"module_ref":reference,"dependencies":actual.dependencies,"asset_bytes":bytes}),
        );
    }
    json!(checked)
}
fn phase_model_audit(input: &Value, points: &BTreeMap<u64, Value>, entries: &[Value]) -> Value {
    let mut rows = vec![];
    for device in input["project"]["devices"].as_array().unwrap() {
        let family = match device["id"].as_str().unwrap() {
            "ASM01" | "CNC" => "assembly",
            "R01" => "robot",
            "CV01" | "CV02" => "conveyor",
            "V01" => "inspection",
            "RW01" => "rework",
            "PK01" => "packing",
            "CASE01" => "sealer",
            _ => unreachable!(),
        };
        let e = entries
            .iter()
            .find(|e| e["id"] == format!("p302.line-grade.{family}"))
            .unwrap();
        let package = read(
            &root()
                .join(e["directory"].as_str().unwrap())
                .join("module.json"),
        );
        let media = &package["data"]["definition"]["payload"]["data"];
        let actual: BTreeSet<_> = points
            .values()
            .flat_map(|point| point["jobs"].as_array().unwrap())
            .filter(|j| j["execution_ref"]["data"]["device_id"] == device["id"])
            .filter_map(|j| j["phase_ref"].as_str().map(str::to_owned))
            .collect();
        let declared: BTreeSet<_> = media["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|clip| clip["phase_ref"].as_str().map(str::to_owned))
            .collect();
        let missing_phases: Vec<_> = actual.difference(&declared).cloned().collect();
        let anchors: Vec<_> = device["item_layout"]["slots"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|slot| slot["anchor_role"].as_str())
            .collect();
        let missing_anchors: Vec<_> = anchors
            .into_iter()
            .filter(|role| media["anchor_roles"][*role].is_null())
            .collect();
        rows.push(json!({"device":device["id"],"media_ref":module_ref(e),"observed_actual_phases":actual,"declared_model_phases":declared,
            "unused_model_phases":declared.difference(&actual).collect::<Vec<_>>(),"missing_actual_phases":missing_phases,"missing_item_anchors":missing_anchors,
            "passed":missing_phases.is_empty()&&missing_anchors.is_empty(),"visual_acceptance":false}));
    }
    json!({"passed":rows.iter().all(|row|row["passed"]==true),"devices":rows,
        "rule":"Only actual phase_ref from public Host history is matched; extra declared clips do not introduce business phases. GLB nodes and referenced animation indices are checked on exact Host-read bytes."})
}
fn assert_history(
    label: &str,
    h: &mut Harness,
    p: &ProjectId,
    r: &RunResult,
    times: &mut BTreeSet<u64>,
) -> Value {
    let data = json!(r);
    let mut identities = json!({});
    match label {
        "l03-complete" => {
            assert_eq!(
                data["acceptance_events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| a["at_ms"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                [90000, 130000, 240000, 255000, 290000, 330000]
            );
            let robot = device_job(&data, "R01");
            let cargo = robot["input_refs"][0].clone();
            assert_eq!(robot["started_at_ms"], 40000);
            assert_eq!(robot["finished_at_ms"], 52000);
            let downstream = device_job(&data, "CV01");
            assert_eq!(downstream["started_at_ms"], 49000);
            for (t, kind, holder) in [
                (42999, "location", json!("out-ASM01")),
                (43000, "job", robot["id"].clone()),
                (48999, "job", robot["id"].clone()),
                (49000, "job", downstream["id"].clone()),
            ] {
                times.insert(t);
                let point = at(h, p, r, t);
                assert_eq!(
                    item(&point, &cargo)["holder"],
                    json!({"kind":kind,"id":holder})
                );
            }
            let sealer = device_job(&data, "CASE01");
            let package = sealer["input_refs"]
                .as_array()
                .unwrap()
                .iter()
                .find(|key| {
                    data["final_items"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|i| i["id"] == **key && i["item_type_ref"]["id"] == "package")
                })
                .unwrap();
            for (t, state) in [
                (87999, "open"),
                (88000, "sealed"),
                (89999, "sealed"),
                (90000, "sealed"),
            ] {
                times.insert(t);
                let point = at(h, p, r, t);
                let pack = item(&point, package);
                assert_eq!(pack["attributes"]["closure"]["value"], state);
                if t < 90000 {
                    assert_eq!(pack["holder"], json!({"kind":"job","id":sealer["id"]}));
                } else {
                    assert_eq!(
                        pack["holder"],
                        json!({"kind":"location","id":"PAL01-store"})
                    );
                }
            }
            let mut products = BTreeSet::new();
            let mut packages = vec![];
            for acceptance in data["acceptance_events"].as_array().unwrap() {
                let pack = data["final_items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|i| i["id"] == acceptance["item_refs"][0])
                    .unwrap();
                assert_eq!(pack["contents"].as_array().unwrap().len(), 1);
                let content = &pack["contents"][0];
                assert!(products.insert(content.to_string()));
                let content = data["final_items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|i| i["id"] == *content)
                    .unwrap();
                assert_eq!(
                    content["holder"],
                    json!({"kind":"container","id":pack["id"]})
                );
                assert_eq!(content["composition_refs"].as_array().unwrap().len(), 6);
                if content["composition_refs"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("shell-003"))
                {
                    assert_eq!(content["completed_reworks"], 1);
                }
                packages.push(json!({"package":pack["id"],"product":content["id"],"composition":content["composition_refs"],"accepted_at_ms":acceptance["at_ms"],"holder":pack["holder"]}));
            }
            assert_eq!(products.len(), 6);
            let tape_inputs: BTreeSet<_> = data["jobs"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|j| {
                    j["execution_ref"]["data"]["device_id"] == "CASE01"
                        && !j["started_at_ms"].is_null()
                })
                .flat_map(|j| j["input_refs"].as_array().unwrap())
                .map(Value::to_string)
                .collect();
            let tapes: Vec<_> = data["final_items"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|i| {
                    i["item_type_ref"]["id"] == "sealing_tape"
                        && i["lifecycle"] == "transformed"
                        && tape_inputs.contains(&i["id"].to_string())
                })
                .collect();
            assert_eq!(tapes.len(), 6);
            assert_eq!(
                tapes
                    .iter()
                    .map(|i| i["quantity"]["value"]["coefficient"].as_u64().unwrap())
                    .sum::<u64>(),
                36
            );
            assert!(
                tapes
                    .iter()
                    .all(|i| i["quantity"]["value"]["decimal_places"] == 1)
            );
            let eight: Vec<_> = [
                "ASM01", "R01", "CV01", "V01", "RW01", "CV02", "PK01", "CASE01",
            ]
            .into_iter()
            .map(|d| json!({"device":d,"first_job":device_job(&data,d)}))
            .collect();
            identities = json!({"robot_job":robot,"robot_cargo":cargo,"sealer_job":sealer,"first_package":package,"accepted_packages":packages,"eight_device_first_jobs":eight});
        }
        "l03-missing-pcb" => {
            let first = device_job(&data, "ASM01");
            assert_eq!(first["started_at_ms"], 120000);
            times.extend([0, 60000, 119999, 120000, 140000, 360000]);
            for t in [0, 60000, 119999] {
                let point = at(h, p, r, t);
                assert!(
                    point["jobs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|j| j["started_at_ms"].is_null())
                );
                assert!(
                    point["items"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|i| i["holder"]["kind"] == "location")
                );
                assert!(point["active_waits"].as_array().unwrap().iter().any(|w| {
                    w["reasons"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|reason| reason["kind"] == "missing_material")
                }));
            }
            assert_eq!(job(&at(h, p, r, 120000), &first["id"])["state"], "working");
            identities = json!({"first_started_job":first,"missing_pcb_interval_ms":[0,120000],"original_future_arrival_preserved":"ARR-PCB-001"});
        }
        "phase-resource-wait" | "phase-interrupted" => {
            let target = device_job(&data, "CNC");
            let key = &target["id"];
            let checks = if label == "phase-resource-wait" {
                vec![
                    (2500, "load", 2500),
                    (20000, "auto", 15000),
                    (35000, "unload", 0),
                    (37500, "unload", 0),
                    (42500, "unload", 2500),
                ]
            } else {
                vec![
                    (10000, "auto", 5000),
                    (20000, "auto", 15000),
                    (30000, "auto", 15000),
                    (39999, "auto", 15000),
                    (45000, "auto", 20000),
                ]
            };
            for (t, phase, progress) in checks {
                times.insert(t);
                let point = at(h, p, r, t);
                let j = job(&point, key);
                assert_eq!(j["phase_ref"], phase);
                assert_eq!(j["effective_progress_ms"], progress);
                assert!(j["finished_at_ms"].is_null());
                let waiting = if label == "phase-resource-wait" {
                    (35000..40000).contains(&t)
                } else {
                    (20000..40000).contains(&t)
                };
                assert_eq!(
                    j["state"],
                    if waiting {
                        "waiting_resource"
                    } else {
                        "working"
                    }
                );
                assert_eq!(j["job_resources"], json!(["CAP-CNC"]));
                if waiting {
                    assert!(!point["active_waits"].as_array().unwrap().is_empty());
                    assert!(
                        point["active_resource_claims"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|c| c["resource_id"] == "CAP-CNC")
                    );
                }
            }
            assert_eq!(
                target["finished_at_ms"],
                if label == "phase-resource-wait" {
                    45000
                } else {
                    60000
                }
            );
            identities = json!({"job":target});
        }
        "l03-robot-output-wait" => {
            let target = device_job(&data, "R01");
            assert_eq!(target["state"], "waiting_output");
            assert_eq!(target["phase_ref"], "release");
            assert!(target["finished_at_ms"].is_null());
            let cargo = target["input_refs"][0].clone();
            times.extend([47999, 48000, 60000, 360000]);
            for t in [48000, 60000, 360000] {
                let point = at(h, p, r, t);
                assert_eq!(
                    item(&point, &cargo)["holder"],
                    json!({"kind":"job","id":target["id"]})
                );
                assert_eq!(job(&point, &target["id"])["phase_ref"], "release");
            }
            assert!(
                !r.phase_work_intervals
                    .iter()
                    .any(|w| json!(w.job_id) == target["id"] && w.phase_id.as_str() == "release")
            );
            identities = json!({"job":target,"cargo":cargo});
        }
        "l03-pallet-output-wait" => {
            let target = device_job(&data, "CASE01");
            assert_eq!(target["state"], "waiting_output");
            assert!(target["finished_at_ms"].is_null());
            assert_eq!(target["pending_outputs"].as_array().unwrap().len(), 1);
            let package = &target["pending_outputs"][0];
            times.extend([87999, 88000, 89999, 90000, 360000]);
            let begin = item(&at(h, p, r, 90000), package);
            let end = item(&at(h, p, r, 360000), package);
            assert_eq!(begin["contents"], end["contents"]);
            assert_eq!(end["attributes"]["closure"]["value"], "sealed");
            assert_eq!(end["holder"], json!({"kind":"job","id":target["id"]}));
            identities = json!({"job":target,"retained_package":end});
        }
        _ => unreachable!(),
    }
    identities
}
fn variants(original: &Value) -> Vec<(&'static str, Value, Value)> {
    let mut out = vec![("l03-complete", original.clone(), json!([]))];
    let mut missing = original.clone();
    missing["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .retain(|i| i["item_type_ref"]["id"] != "pcb");
    out.push(("l03-missing-pcb",missing,json!(["Remove only initial pcb Items; preserve the original ARR-PCB-001 at 120000ms and all other business input. Assert no start before real arrival and real start at 120000ms"])));
    let phased =
        read(&root().join("packages/commands/tests/fixtures/production/p1-phase-calendar.json"));
    out.push(("phase-resource-wait",phased.clone(),json!(["Unmodified existing PHASE input: load 0-5s, auto 5-35s, worker unavailable 20-40s, unload 40-45s"])));
    let mut gap = phased;
    gap["project"]["calendars"].as_array_mut().unwrap().push(json!({"id":"MACHINE-GAP","title":"explicit off shift","source":"user_provided","intervals":[{"start_ms":0,"end_ms":20000},{"start_ms":40000,"end_ms":100000}]}));
    for r in gap["project"]["resources"].as_array_mut().unwrap() {
        if r["id"] == "CAP-CNC" {
            r["calendar_id"] = json!("MACHINE-GAP")
        }
        if r["id"] == "W-C" {
            r["calendar_id"] = json!("ALL")
        }
    }
    gap["definitions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["id"] == "w5.op.phased")
        .unwrap()["payload"]["data"]["phases"][1]["interruption_policy"] = json!("pause_resume");
    out.push(("phase-interrupted",gap,json!(["Exact named pause_resume machine_gap variant already exercised by phased_resources_acceptance.rs; machine off 20-40s, worker ALL; unchanged work durations"])));
    let mut blocked = original.clone();
    let mut blocker = blocked["scenario"]["initial_conditions"]["items"][0].clone();
    blocker["id"] = json!("R01-destination-pending-item");
    blocker["item_type_ref"] = json!({"id":"assembly","revision":"r1"});
    blocker["holder"] = json!({"kind":"location","id":"out-R01"});
    blocker["quality"] = json!("pending");
    blocked["scenario"]["initial_conditions"]["items"]
        .as_array_mut()
        .unwrap()
        .push(blocker);
    out.push(("l03-robot-output-wait",blocked,json!(["Exact existing full_robot_destination variant: one named pending assembly in capacity-one out-R01"])));
    let mut pallet = original.clone();
    for i in 0..12 {
        let mut blocker = original["scenario"]["initial_conditions"]["items"][0].clone();
        blocker["id"] = json!(format!("PAL01-unreleased-{i:02}"));
        blocker["item_type_ref"] = json!({"id":"package","revision":"r1"});
        blocker["holder"] = json!({"kind":"location","id":"PAL01-store"});
        blocker["quality"] = json!("pending");
        pallet["scenario"]["initial_conditions"]["items"]
            .as_array_mut()
            .unwrap()
            .push(blocker);
    }
    out.push(("l03-pallet-output-wait",pallet,json!(["Exact existing full_pallet variant: twelve named unreleased packages fill capacity twelve; sealed new package stays with its Job"])));
    out
}

#[test]
fn actual_l03_and_four_wait_classes_keep_identity_history_exact_media_and_fresh_host_replay() {
    let original = read(&root().join("scenarios/媒体表现/runtime-l03/engine-input.json"));
    let base = root().join(format!(
        ".work/model-v04-runtime-reception/artifacts/attempt-{}",
        std::process::id()
    ));
    fs::create_dir(&base).unwrap();
    let library = base.join("original-media-library");
    let legacy = legacy_library(&original, &library);
    let entries = refs("modules/media/equipment-quality-v04/registration.json");
    assert_eq!(entries.len(), 28);
    let mut results = vec![];
    for (label, input, deltas) in variants(&original) {
        println!("actual Host case: {label}");
        output(
            &root().join(format!(".work/model-v04-runtime-reception/inputs/{label}.json")),
            &json!({"kind":"named_actual_engine_input","deltas":deltas,"input":input}),
        );
        let case = base.join(label);
        fs::create_dir(&case).unwrap();
        let seed = case.join("input-project");
        seed_from_real_library(&library, &seed, &input);
        let mut h = fresh(&format!("runtime-{label}"));
        let p = open(&mut h, &seed);
        assert_eq!(json!(p), input["project"]["id"]);
        media_setup(&mut h, &p, &input, &entries);
        let frozen = project(&mut h, &p);
        for d in input["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["payload"]["category"] != "media")
        {
            let package = object(
                &mut h,
                &p,
                "module",
                d["id"].as_str().unwrap(),
                json!({"kind":"module_version","data":{"module_ref":module_ref(d)}}),
            );
            assert_eq!(
                package["definition"],
                json!(serde_json::from_value::<ModuleDefinition>(d.clone()).unwrap())
            );
        }
        let r = run(&mut h, &p, &input);
        assert!(matches!(r.evidence_origin, EvidenceOrigin::ActualSoftware));
        assert!(r.whole_window_computed);
        assert!(matches!(r.computation_status, ComputationStatus::Completed));
        assert_eq!(json!(r.input_ref.project_revision), frozen["revision"]);
        output(&case.join("actual-run.json"), &json!({"run":r}));
        let source = json!({"kind":"run_snapshot","data":{"run_id":r.id,"at_ms":0}});
        assert_eq!(
            object(&mut h, &p, "project", p.as_str(), source.clone()),
            frozen
        );
        let mut times = moments(&r);
        let identities = assert_history(label, &mut h, &p, &r, &mut times);
        let live = project(&mut h, &p);
        let media = media_bytes(&mut h, &p, &r, &entries);
        let mut expected = BTreeMap::new();
        for t in &times {
            expected.insert(*t, at(&mut h, &p, &r, *t));
        }
        let model_audit = phase_model_audit(&input, &expected, &entries);
        output(&case.join("model-phase-audit.json"), &model_audit);
        for t in times.iter().rev() {
            assert_eq!(
                at(&mut h, &p, &r, *t),
                expected[t],
                "reverse seek {label} {t}"
            );
        }
        assert_eq!(project(&mut h, &p), live);
        assert_eq!(json!(full(&mut h, &p, &r.id)), json!(r));
        let saved = case.join("saved-project");
        save(&mut h, &p, &saved);
        let moved = case.join("receiver/moved-project");
        fs::create_dir_all(&moved).unwrap();
        copy(&saved, &moved);
        let mut reopened = fresh(&format!("reopened-{label}"));
        let reopened_id = open(&mut reopened, &moved);
        assert_eq!(reopened_id, p);
        assert_eq!(json!(full(&mut reopened, &p, &r.id)), json!(r));
        assert_eq!(project(&mut reopened, &p), live);
        assert_eq!(
            object(&mut reopened, &p, "project", p.as_str(), source),
            frozen
        );
        assert_eq!(media_bytes(&mut reopened, &p, &r, &entries), media);
        // Current revision is equal after open; stored-history observations must
        // agree in both directions with the original Engine-backed Host.
        for t in times.iter().rev().chain(times.iter()) {
            assert_eq!(
                at(&mut reopened, &p, &r, *t),
                expected[t],
                "fresh Host history {label} {t}"
            );
        }
        assert_eq!(json!(full(&mut reopened, &p, &r.id)), json!(r));
        assert_eq!(project(&mut reopened, &p), live);
        let resources: Vec<_> = input["project"]["resources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|resource| {
                object(
                    &mut reopened,
                    &p,
                    "resource",
                    resource["id"].as_str().unwrap(),
                    json!({"kind":"run_snapshot","data":{"run_id":r.id,"at_ms":0}}),
                )
            })
            .collect();
        output(&case.join("actual-run.json"), &json!({"run":r}));
        output(
            &case.join("timepoints.json"),
            &json!({"label":label,"project_id":p,"current_revision":live["revision"],"run_id":r.id,"run_input_ref":r.input_ref,
            "scope":{"kind":"project"},"resource_declarations":resources,"identity_assertions":identities,"points":expected.values().collect::<Vec<_>>(),
            "verified_orders":["forward original","reverse original","reverse new Host","forward new Host"],"expected_source":"actual public Host historical reads, plus explicit task assertions; not final Job substitution"}),
        );
        let summary = json!({"label":label,"input_deltas":deltas,"project_id":p,"run_id":r.id,"input_ref":r.input_ref,"current_revision":live["revision"],
            "input_project":seed.strip_prefix(root()).unwrap(),"saved_project":saved.strip_prefix(root()).unwrap(),"moved_project":moved.strip_prefix(root()).unwrap(),
            "timepoints":case.join("timepoints.json").strip_prefix(root()).unwrap(),"point_count":times.len(),"job_count":r.jobs.len(),"acceptance_count":r.acceptance_events.len(),
            "exact_v04_packages":media,"model_phase_audit":model_audit,"passed":true,"Native":false,"MQ07_MQ08_whole":false,"human_acceptance":null});
        output(&case.join("case-result.json"), &summary);
        results.push(summary);
    }
    let result = json!({"passed":true,"model_contracts_passed":results.iter().all(|r|r["model_phase_audit"]["passed"]==true),"case_count":results.len(),"cases":results,"legacy_exact_versions":legacy.iter().map(module_ref).collect::<Vec<_>>(),
        "waiting_classes":["missing input before start","phase resource wait","pause_resume interruption","output capacity"],
        "production_sources_modified":false,"Native":false,"MQ07_MQ08_whole":false,"human_acceptance":null,"whole_task_done":false});
    output(&base.join("runtime-result.json"), &result);
    output(
        &root().join(".work/model-v04-runtime-reception/delivery/latest-runtime-result.json"),
        &json!({"result":base.join("runtime-result.json").strip_prefix(root()).unwrap(),"case_count":results.len(),"passed":true}),
    );
}
