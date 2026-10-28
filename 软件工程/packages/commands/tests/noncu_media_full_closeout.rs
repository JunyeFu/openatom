//! Formal Items authoring/adoption and immutable old/new production Runs.
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
fn scoped(
    h: &mut Harness,
    p: Option<&ProjectId>,
    scope: Value,
    operation: &str,
    params: Value,
) -> Value {
    let request =
        serde_json::from_value(json!({"api_version":"0.1","request_id":"v04-items-formal",
        "project_id":p,"scope":scope,"operation":operation,"params":params}))
        .unwrap();
    h.host.dispatch(&h.ui, request)
}
fn api(h: &mut Harness, p: Option<&ProjectId>, op: &str, params: Value) -> Value {
    scoped(
        h,
        p,
        json!({"kind":if p.is_some(){"project"}else{"workspace"}}),
        op,
        params,
    )
}
fn context(h: &mut Harness) -> ProjectContext {
    ok(api(h, None, "get_context", json!({"selection":[]})))
}
fn object(h: &mut Harness, p: &ProjectId, kind: &str, name: &str, source: Value) -> Value {
    let value: DomainObject = ok(api(
        h,
        Some(p),
        "get_object",
        json!({"object_id":{"kind":kind,"id":name},"source":source,"node_index":null}),
    ));
    json!(value)["data"].clone()
}
fn project(h: &mut Harness, p: &ProjectId) -> Value {
    object(h, p, "project", p.as_str(), json!({"kind":"draft"}))
}
fn frozen(h: &mut Harness, p: &ProjectId, r: &RunId) -> Value {
    object(
        h,
        p,
        "project",
        p.as_str(),
        json!({"kind":"run_snapshot","data":{"run_id":r,"at_ms":0}}),
    )
}
fn run_source(r: &RunId, m: &Value) -> Value {
    json!({"kind":"run_module_version","data":{"run_id":r,"at_ms":0,"module_ref":m}})
}
fn module(h: &mut Harness, p: &ProjectId, m: &Value, r: Option<&RunId>) -> Value {
    object(
        h,
        p,
        "module",
        m["id"].as_str().unwrap(),
        r.map(|r| run_source(r, m))
            .unwrap_or_else(|| json!({"kind":"module_version","data":{"module_ref":m}})),
    )
}
fn apply(h: &mut Harness, p: &ProjectId, commands: Value) -> Value {
    let base = project(h, p)["revision"].clone();
    api(
        h,
        Some(p),
        "apply_commands",
        json!({"base_revision":base,"commands":commands}),
    )
}
fn adoption(m: &Value, target: &str) -> Value {
    json!({"kind":"adopt_module","data":{"module_ref":m,"targets":[{"kind":"item_type","id":target}],
    "bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"new_instances":[],"explicit_seed":null}})
}
fn open(h: &mut Harness, path: &Path) -> ProjectId {
    let task = h.import(path);
    h.finish_one();
    let draft = h.imported(&task);
    let _: EditCommit = ok(h.adopt(&task, &draft, false));
    context(h).project.unwrap().id
}
fn finish(h: &mut Harness, p: &ProjectId, t: &TaskHandle) -> TaskState {
    h.finish_one();
    let s: TaskState = ok(api(h, Some(p), "get_task", json!({"task_id":t.id})));
    assert!(
        matches!(s.task.state, ComputationStatus::Completed),
        "{s:?}"
    );
    s
}
fn full(h: &mut Harness, p: &ProjectId, r: &RunId) -> RunResult {
    let v: RunView = ok(api(
        h,
        Some(p),
        "get_run",
        json!({"run_id":r,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(v) = v else {
        panic!("full Run")
    };
    *v
}
fn run(h: &mut Harness, p: &ProjectId, old: &RunResult) -> RunResult {
    let revision = project(h, p)["revision"].clone();
    let task: TaskHandle = ok(api(
        h,
        Some(p),
        "run_scenario",
        json!({"scenario_id":old.input_ref.scenario_id,"input_revision":revision,
        "observation_window":old.window,"intent":{"kind":"project_scenario"}}),
    ));
    let state = finish(h, p, &task);
    let Some(TaskOutput::Run(s)) = state.result else {
        panic!("computed Run")
    };
    full(h, p, &s.run_id)
}
fn save(h: &mut Harness, p: &ProjectId, path: &Path) {
    fs::create_dir_all(path).unwrap();
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::SaveProject, path.to_owned())
        .unwrap();
    let revision = project(h, p)["revision"].clone();
    let task: TaskHandle = ok(api(
        h,
        Some(p),
        "save_project",
        json!({"revision":revision,"selected_directory_ref":selected}),
    ));
    finish(h, p, &task);
}
fn differences(a: &Value, b: &Value, path: &str, out: &mut Vec<Value>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
            for k in keys {
                differences(
                    a.get(k).unwrap_or(&Value::Null),
                    b.get(k).unwrap_or(&Value::Null),
                    &format!("{path}/{k}"),
                    out,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                differences(a, b, &format!("{path}/{i}"), out)
            }
        }
        _ => out.push(json!({"path":path,"before":a,"after":b})),
    }
}
fn exact_run_business(a: &RunResult, b: &RunResult, path: &Path) -> Value {
    let mut delta = vec![];
    differences(&json!(a), &json!(b), "", &mut delta);
    let allowed: BTreeSet<_> = [
        "/id",
        "/input_ref/project_revision",
        "/input_ref/scenario_revision",
        "/input_ref/frozen_project_file/path",
        "/input_ref/frozen_scenario_file/path",
    ]
    .into_iter()
    .collect();
    output(
        path,
        &json!({"comparison":"all canonical RunResult fields; only these exact identity/revision/file-address leaves may differ","allowed":allowed,"differences":delta}),
    );
    assert!(
        delta
            .iter()
            .all(|d| allowed.contains(d["path"].as_str().unwrap())),
        "unallowed full Run business difference; see {}",
        path.display()
    );
    assert_ne!(a.id, b.id);
    json!(delta)
}
fn history(h: &mut Harness, p: &ProjectId, r: &RunResult, t: u64) -> Value {
    let mut objects = vec![];
    let mut selectors = vec![json!([])];
    selectors.extend(r.jobs.chunks(200).map(|chunk| {
        json!(
            chunk
                .iter()
                .map(|j| json!({"kind":"job","id":j.id}))
                .collect::<Vec<_>>()
        )
    }));
    for selected in selectors {
        let mut after = Value::Null;
        loop {
            let v: RunView = ok(api(
                h,
                Some(p),
                "get_run",
                json!({"run_id":r.id,"query":{"kind":"objects_at","data":{"at_ms":t,"objects":selected,"page":{"after_index":after,"limit":1000}}}}),
            ));
            let RunView::Objects(page) = v else {
                panic!("historical state")
            };
            objects.extend(page.objects.iter().map(|o| json!(o)));
            after = json!(page.next_index);
            if page.next_index.is_none() {
                break;
            }
        }
    }
    json!({"at_ms":t,"objects":objects})
}
fn mappings(h: &mut Harness, p: &ProjectId, r: &RunResult) -> Vec<Value> {
    let source = frozen(h, p, &r.id);
    let mut rows = vec![];
    for item in source["item_types"].as_array().unwrap() {
        if item["module_ref"].is_null() {
            continue;
        }
        let package = module(h, p, &item["module_ref"], Some(&r.id));
        assert_eq!(package["definition"]["payload"]["category"], "items");
        let presentation = &package["definition"]["payload"]["data"]["presentation_or_media_ref"];
        if presentation["source"]["kind"] != "media_ref" {
            continue;
        }
        rows.push(json!({"item_type":item,"items_ref":item["module_ref"],"items_package":package,"media_ref":presentation["source"]["data"]}));
    }
    rows
}
fn media_bytes(
    h: &mut Harness,
    p: &ProjectId,
    r: &RunResult,
    rows: &[Value],
    expected_version: &str,
) -> Value {
    let revision = context(h).project.unwrap().revision;
    let mut checked = BTreeMap::new();
    for row in rows {
        assert_eq!(row["items_ref"]["version"], expected_version);
        assert_eq!(row["media_ref"]["version"], expected_version);
        let detailed = module(h, p, &row["media_ref"], Some(&r.id));
        let presentation = &detailed["definition"]["payload"]["data"];
        let simplified = presentation["lod"]["simplified"]["data"].clone();
        assert_eq!(presentation["lod"]["simplified"]["kind"], "media_ref");
        assert_eq!(simplified["version"], expected_version);
        assert_eq!(detailed["dependencies"], json!([simplified]));
        if expected_version == "0.4.0" {
            let item_presentation =
                &row["items_package"]["definition"]["payload"]["data"]["presentation_or_media_ref"];
            assert_eq!(
                item_presentation["lod"]["detailed"]["data"],
                row["media_ref"]
            );
            assert_eq!(item_presentation["lod"]["simplified"]["data"], simplified);
            let actual: BTreeSet<_> = row["items_package"]["dependencies"]
                .as_array()
                .unwrap()
                .iter()
                .map(Value::to_string)
                .collect();
            assert_eq!(
                actual,
                BTreeSet::from([row["media_ref"].to_string(), simplified.to_string()])
            );
        }
        for m in [&row["media_ref"], &simplified] {
            if checked.contains_key(&m.to_string()) {
                continue;
            }
            let package = module(h, p, m, Some(&r.id));
            let input = if expected_version == "0.4.0" {
                "equipment-quality-v04"
            } else {
                "p302-l03-v03"
            };
            let registry = read(&root().join(format!("modules/media/{input}/registration.json")));
            let entry = registry["register_order"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["id"] == m["id"] && e["version"] == m["version"])
                .unwrap();
            let dir = root().join(entry["directory"].as_str().unwrap());
            let original = read(&dir.join("module.json"))["data"].clone();
            assert_eq!(package["dependencies"], original["dependencies"]);
            let source: ObjectSource = serde_json::from_value(run_source(&r.id, m)).unwrap();
            let assets: Vec<AssetRef> = serde_json::from_value(package["assets"].clone()).unwrap();
            assert_eq!(assets.len(), 2);
            let mut bytes = 0;
            for (asset, original) in assets.iter().zip(original["assets"].as_array().unwrap()) {
                let actual = h
                    .host
                    .read_media_asset(&h.ui, p, &ApiScope::Project, &revision, &source, asset)
                    .unwrap();
                let expected = fs::read(dir.join(original["path"].as_str().unwrap())).unwrap();
                assert_eq!(actual, expected);
                let cache = root()
                    .join(".work/full-closeout-20261021/delivery/host-media")
                    .join(m["id"].as_str().unwrap())
                    .join(m["version"].as_str().unwrap());
                fs::create_dir_all(&cache).unwrap();
                let asset_path = cache.join(asset.path.as_str());
                fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
                fs::write(asset_path, &actual).unwrap();
                output(&cache.join("module.json"), &package);
                bytes += actual.len();
            }
            checked.insert(m.to_string(),json!({"module_ref":m,"dependencies":package["dependencies"],"GLB_PNG_bytes":bytes}));
        }
    }
    json!(checked.values().collect::<Vec<_>>())
}
fn author_and_adopt(h: &mut Harness, p: &ProjectId, old: &[Value], out: &Path) -> Vec<Value> {
    let original = project(h, p);
    let mut planned = vec![];
    let mut trace = vec![];
    for row in old {
        assert_eq!(row["items_ref"]["version"], "0.3.0");
        let before = row["items_package"]["definition"].clone();
        let mut definition = before.clone();
        let media_id = row["media_ref"]["id"].as_str().unwrap();
        let detail = json!({"id":media_id,"version":"0.4.0"});
        let simple = json!({"id":format!("{media_id}.simple"),"version":"0.4.0"});
        let presentation = &mut definition["payload"]["data"]["presentation_or_media_ref"];
        presentation["source"] = json!({"kind":"media_ref","data":detail});
        presentation["lod"]["detailed"] = presentation["source"].clone();
        presentation["lod"]["simplified"] = json!({"kind":"media_ref","data":simple});
        let commands = json!([{"kind":"create_definition_draft","data":before}]);
        let response = apply(h, p, commands.clone());
        let commit: EditCommit = ok(response.clone());
        let EditScope::DefinitionDraft(draft) = commit.scope else {
            panic!("formal definition draft")
        };
        trace.push(json!({"scope":{"kind":"project"},"commands":commands,"response":response}));
        let put = json!([{"kind":"put_definition","data":definition}]);
        let response = scoped(
            h,
            Some(p),
            json!({"kind":"definition_draft","data":draft.id}),
            "apply_commands",
            json!({"base_revision":draft.revision,"commands":put}),
        );
        let updated: EditCommit = ok(response.clone());
        trace.push(json!({"scope":{"kind":"definition_draft","data":draft.id},"commands":put,"response":response}));
        let commit_command =
            json!([{"kind":"commit_definition_version","data":{"version":"0.4.0"}}]);
        let response = scoped(
            h,
            Some(p),
            json!({"kind":"definition_draft","data":draft.id}),
            "apply_commands",
            json!({"base_revision":updated.new_revision,"commands":commit_command}),
        );
        let _: EditCommit = ok(response.clone());
        trace.push(json!({"scope":{"kind":"definition_draft","data":draft.id},"commands":commit_command,"response":response}));
        let reference = json!({"id":row["items_ref"]["id"],"version":"0.4.0"});
        let published = module(h, p, &reference, None);
        definition["version"] = json!("0.4.0");
        assert_eq!(published["definition"], definition);
        let mut business_definition = definition.clone();
        business_definition["version"] = before["version"].clone();
        business_definition["payload"]["data"]["presentation_or_media_ref"] =
            before["payload"]["data"]["presentation_or_media_ref"].clone();
        assert_eq!(business_definition, before);
        planned.push(json!({"item_type_id":row["item_type"]["id"],"old_items_ref":row["items_ref"],"new_items_ref":reference,"old_media_ref":row["media_ref"],"new_detailed_ref":detail,"new_simplified_ref":simple,"published_dependencies":published["dependencies"]}));
    }
    assert_eq!(
        project(h, p),
        original,
        "creating/publishing definitions must not implicitly adopt them"
    );
    let commands = json!(
        planned
            .iter()
            .map(|row| adoption(&row["new_items_ref"], row["item_type_id"].as_str().unwrap()))
            .collect::<Vec<_>>()
    );
    let response = apply(h, p, commands.clone());
    let _: EditCommit = ok(response.clone());
    trace.push(json!({"scope":{"kind":"project"},"commands":commands,"response":response}));
    let actual = project(h, p);
    let mut expected = original.clone();
    expected["revision"] = json!(original["revision"].as_u64().unwrap() + 1);
    for row in &planned {
        let target = expected["item_types"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item["id"] == row["item_type_id"])
            .unwrap();
        target["module_ref"] = row["new_items_ref"].clone();
        expected["module_refs"]
            .as_array_mut()
            .unwrap()
            .push(row["new_items_ref"].clone());
    }
    assert_eq!(
        actual, expected,
        "adoption may only add exact versions, change explicit ItemType ModuleRefs and advance the Project revision"
    );
    output(
        out,
        &json!({"formal_commands":trace,"mapping":planned,"type_revision_contract":"AdoptModule preserves ItemType revision and every business field; exact Items ModuleRef plus Project/Scenario revision changes; BOM/route ItemTypeRef stays original"}),
    );
    planned
}

#[test]
#[ignore = "requires retained P3-02 actual Host archives; run explicitly with --ignored"]
fn noncu_media_complete_remaining_item_persistence() {
    let latest =
        read(&root().join(".work/delivery/host-runtime/delivery/latest-runtime-result.json"));
    let catalog = read(&root().join(latest["result"].as_str().unwrap()));
    let prior = fs::read_dir(root().join(".work/delivery/items-reception/artifacts"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.join("l03-complete/case-result.json").is_file()
                && p.join("l03-missing-pcb/case-result.json").is_file()
                && read(&p.join("l03-complete/case-result.json"))["passed"] == true
                && read(&p.join("l03-missing-pcb/case-result.json"))["passed"] == true
        })
        .expect("actual persisted Items prerequisite");
    let proven = read(&prior.join("l03-complete/media-mapping.json"));
    assert_eq!(
        read(&prior.join("l03-complete/case-result.json"))["passed"],
        true
    );
    assert_eq!(
        read(&prior.join("l03-missing-pcb/case-result.json"))["passed"],
        true
    );
    let base = root().join(format!(
        ".work/full-closeout-20261021/delivery/items-persistence/attempt-{}",
        std::process::id()
    ));
    fs::create_dir_all(&base).unwrap();
    let mut results = vec![];
    for entry in catalog["cases"].as_array().unwrap() {
        let label = entry["label"].as_str().unwrap();
        if prior.join(label).join("case-result.json").is_file() {
            let passed = read(&prior.join(label).join("case-result.json"));
            assert_eq!(passed["passed"], true);
            results.push(json!({"label":label,"reused_actual_complete_case":prior.join(label),"passed":true}));
            continue;
        }
        let case = base.join(label);
        fs::create_dir_all(&case).unwrap();
        let mut h = fresh(&format!("residual-{label}"));
        let p = open(
            &mut h,
            &root().join(entry["moved_project"].as_str().unwrap()),
        );
        let old_id: RunId = serde_json::from_value(entry["run_id"].clone()).unwrap();
        let old = full(&mut h, &p, &old_id);
        let before = json!(old);
        let old_project = frozen(&mut h, &p, &old_id);
        let old_rows = mappings(&mut h, &p, &old);
        let mut times = BTreeMap::new();
        let history_input = read(&root().join(entry["timepoints"].as_str().unwrap()));
        for point in history_input["points"].as_array().unwrap() {
            let t = point["at_ms"].as_u64().unwrap();
            times.insert(t, history(&mut h, &p, &old, t));
        }
        let planned = if !old_rows.is_empty() {
            author_and_adopt(&mut h, &p, &old_rows, &case.join("formal-authoring.json"))
        } else {
            vec![]
        };
        let new = run(&mut h, &p, &old);
        let delta =
            exact_run_business(&old, &new, &case.join("full-run-business-differences.json"));
        let new_rows = mappings(&mut h, &p, &new);
        for row in &new_rows {
            let existing = proven["new_types"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["item_type"]["id"] == row["item_type"]["id"])
                .unwrap();
            assert_eq!(
                row["items_package"], existing["items_package"],
                "only already saved/reopened exact Items packages can reuse persistence evidence"
            );
        }
        for (t, expected) in times.iter().rev() {
            assert_eq!(history(&mut h, &p, &old, *t), *expected);
            assert_eq!(history(&mut h, &p, &new, *t), *expected);
        }
        let media = media_bytes(&mut h, &p, &new, &new_rows, "0.4.0");
        assert_eq!(json!(full(&mut h, &p, &old.id)), before);
        assert_eq!(frozen(&mut h, &p, &old.id), old_project);
        output(&case.join("runs.json"), &json!({"old":old,"new":new}));
        output(
            &case.join("frozen-projects.json"),
            &json!({"old":old_project,"new":frozen(&mut h,&p,&new.id),"current":project(&mut h,&p)}),
        );
        output(
            &case.join("media-mapping.json"),
            &json!({"old_types":old_rows,"new_types":new_rows,"formal_adoption":planned,"new_media_bytes":media}),
        );
        output(
            &case.join("timepoints.json"),
            &json!({"project_id":p,"scope":{"kind":"project"},"current_revision":context(&mut h).project.unwrap().revision,"points":times.values().collect::<Vec<_>>()}),
        );
        let old_media = media_bytes(&mut h, &p, &old, &old_rows, "0.3.0");
        let live = project(&mut h, &p);
        let new_project = frozen(&mut h, &p, &new.id);
        let saved = case.join("saved-project");
        save(&mut h, &p, &saved);
        let moved = case.join("receiver/moved-project");
        fs::create_dir_all(moved.parent().unwrap()).unwrap();
        // Both absolute targets are owned generated outputs below this exact task.
        assert!(saved.starts_with(&base) && moved.starts_with(&base));
        assert!(!moved.exists());
        fs::rename(&saved, &moved).expect("move owned saved archive without a second large copy");
        let child_expected = case.join("fresh-process-expected.json");
        output(
            &child_expected,
            &json!({"moved":moved,"project_id":p,"project":live,
   "old_run":old,"new_run":new,"old_project":old_project,"new_project":new_project,
   "old_rows":old_rows,"new_rows":new_rows,"old_media":old_media,"new_media":media,"points":times}),
        );
        drop(h);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "noncu_media_complete_item_reopen_child",
                "--nocapture",
            ])
            .env("NONCU_FULL_ITEM_CHILD", &child_expected)
            .output()
            .unwrap();
        fs::write(case.join("fresh-process.stdout.log"), &child.stdout).unwrap();
        fs::write(case.join("fresh-process.stderr.log"), &child.stderr).unwrap();
        assert!(
            child.status.success(),
            "independent Items reopen failed: {}",
            String::from_utf8_lossy(&child.stdout)
        );
        let result = json!({"label":label,"passed":true,"point_count":times.len(),"business_differences":delta,
   "same_exact_items_package_as_persisted_complete_case":true,"new_case_saved":true,"moved_project":moved,
   "fresh_OS_process":true,"parent_Host_destroyed":true,"GUI":false});
        output(&case.join("case-result.json"), &result);
        results.push(result);
    }
    output(
        &base.join("result.json"),
        &json!({"passed":true,"cases":results,"remaining_original_persistence_cases_now_completed":true,"original_failure":"disk space, retained in items-controlled-second.log","GUI":false}),
    );
    output(
        &root().join(".work/full-closeout-20261021/delivery/items-persistence/latest.json"),
        &json!({"result":base.join("result.json")}),
    );
}

#[test]
fn noncu_media_complete_item_reopen_child() {
    let Some(path) = std::env::var_os("NONCU_FULL_ITEM_CHILD") else {
        return;
    };
    let expected = read(Path::new(&path));
    let mut h = fresh("full-item-independent-OS");
    assert!(context(&mut h).project.is_none());
    let p = open(&mut h, Path::new(expected["moved"].as_str().unwrap()));
    assert_eq!(json!(p), expected["project_id"]);
    assert_eq!(project(&mut h, &p), expected["project"]);
    let old: RunResult = serde_json::from_value(expected["old_run"].clone()).unwrap();
    let new: RunResult = serde_json::from_value(expected["new_run"].clone()).unwrap();
    let old_rows: Vec<Value> = serde_json::from_value(expected["old_rows"].clone()).unwrap();
    let new_rows: Vec<Value> = serde_json::from_value(expected["new_rows"].clone()).unwrap();
    assert_eq!(json!(full(&mut h, &p, &old.id)), json!(old));
    assert_eq!(json!(full(&mut h, &p, &new.id)), json!(new));
    assert_eq!(frozen(&mut h, &p, &old.id), expected["old_project"]);
    assert_eq!(frozen(&mut h, &p, &new.id), expected["new_project"]);
    assert_eq!(mappings(&mut h, &p, &old), old_rows);
    assert_eq!(mappings(&mut h, &p, &new), new_rows);
    assert_eq!(
        media_bytes(&mut h, &p, &old, &old_rows, "0.3.0"),
        expected["old_media"]
    );
    assert_eq!(
        media_bytes(&mut h, &p, &new, &new_rows, "0.4.0"),
        expected["new_media"]
    );
    for (t, point) in expected["points"].as_object().unwrap() {
        let t = t.parse().unwrap();
        assert_eq!(history(&mut h, &p, &old, t), *point);
        assert_eq!(history(&mut h, &p, &new, t), *point);
    }
    assert_eq!(project(&mut h, &p), expected["project"]);
    output(
        &Path::new(&path)
            .parent()
            .unwrap()
            .join("fresh-process.actual.json"),
        &json!({"passed":true,"pid":std::process::id(),"fresh_Host":true,"external_registration":false,
   "old_new_Run_and_input_exact":true,"old_new_media_bytes_exact":true,
   "historical_points":expected["points"].as_object().unwrap().len()}),
    );
}

#[test]
#[ignore = "requires retained P3-02 actual Host archives; run explicitly with --ignored"]
fn noncu_media_reopen_previously_persisted_items_in_independent_processes() {
    let prior = fs::read_dir(root().join(".work/delivery/items-reception/artifacts"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.join("l03-complete/case-result.json").is_file()
                && p.join("phase-resource-wait/case-result.json").is_file()
                && read(&p.join("l03-complete/case-result.json"))["passed"] == true
        })
        .unwrap();
    let base = root().join(".work/full-closeout-20261021/delivery/reused-item-os");
    let mut results = vec![];
    for label in ["l03-complete", "l03-missing-pcb", "phase-resource-wait"] {
        let source = prior.join(label);
        let result = read(&source.join("case-result.json"));
        assert_eq!(result["passed"], true);
        let runs = read(&source.join("runs.json"));
        let projects = read(&source.join("frozen-projects.json"));
        let mappings = read(&source.join("media-mapping.json"));
        let points = read(&source.join("timepoints.json"));
        let times: BTreeMap<u64, Value> = points["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (p["at_ms"].as_u64().unwrap(), p.clone()))
            .collect();
        let case = base.join(label);
        let expected = case.join("fresh-process-expected.json");
        output(
            &expected,
            &json!({"moved":root().join(result["moved_project"].as_str().unwrap()),
   "project_id":points["project_id"],"project":projects["current"],"old_run":runs["old"],"new_run":runs["new"],
   "old_project":projects["old"],"new_project":projects["new"],"old_rows":mappings["old_types"],
   "new_rows":mappings["new_types"],"old_media":mappings["old_media_bytes"],"new_media":mappings["new_media_bytes"],"points":times}),
        );
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "noncu_media_complete_item_reopen_child",
                "--nocapture",
            ])
            .env("NONCU_FULL_ITEM_CHILD", &expected)
            .output()
            .unwrap();
        fs::write(case.join("fresh-process.stdout.log"), &child.stdout).unwrap();
        fs::write(case.join("fresh-process.stderr.log"), &child.stderr).unwrap();
        assert!(
            child.status.success(),
            "saved previous Items child failed for {label}: {}",
            String::from_utf8_lossy(&child.stdout)
        );
        results.push(
            json!({"label":label,"passed":true,"original_saved_archive":source,
   "fresh_process_result":case.join("fresh-process.actual.json"),"historical_points":times.len()}),
        );
    }
    output(
        &base.join("result.json"),
        &json!({"passed":true,"cases":results,"old_delivery_unmodified":true,"Native":false}),
    );
}
