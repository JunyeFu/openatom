// Actual L03 one-shot executable. Uses the real ProjectHost, never a seed archive or direct Engine.
use serde_json::{json, Value};
use sf_commands::{ApiScope, FilePurpose, ObjectSource, ProjectHost};
use sf_domain::{ModulePackage, ObjectId, ProjectId, Revision};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
type R<T> = Result<T, String>;
fn read(p: &Path) -> R<Value> {
    serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write(p: &Path, v: &Value) -> R<()> {
    fs::write(p, serde_json::to_vec_pretty(v).unwrap()).map_err(|e| e.to_string())
}
fn id(s: &str) -> ObjectId {
    ObjectId::try_from(s.to_owned()).unwrap()
}
fn data(v: Value) -> R<Value> {
    if v["outcome"]["status"] == "ok" {
        Ok(v["outcome"]["data"].clone())
    } else {
        Err(format!("actual Host rejected request: {v}"))
    }
}
struct Driver {
    host: ProjectHost,
    ui: ObjectId,
    project: Option<Value>,
    out: PathBuf,
    n: usize,
}
impl Driver {
    fn new(name: &str, out: &Path) -> Self {
        let ui = id(&format!("{name}-ui"));
        Self {
            host: ProjectHost::new(id(name), ui.clone()),
            ui,
            project: None,
            out: out.to_owned(),
            n: 0,
        }
    }
    fn raw_scope(&mut self, scope: Value, op: &str, params: Value) -> R<Value> {
        self.n += 1;
        let req = json!({"api_version":"0.1","request_id":format!("l03-{}",self.n),"project_id":self.project,
                       "scope":scope,"operation":op,"params":params});
        let typed =
            serde_json::from_value(req.clone()).map_err(|e| format!("request contract: {e}"))?;
        let reply = self.host.dispatch(&self.ui, typed);
        write(
            &self.out.join(format!("{:04}-{op}.json", self.n)),
            &json!({"request":req,"response":reply}),
        )?;
        Ok(reply)
    }
    fn raw(&mut self, op: &str, params: Value) -> R<Value> {
        self.raw_scope(
            json!({"kind":if self.project.is_some(){"project"}else{"workspace"}}),
            op,
            params,
        )
    }
    fn call(&mut self, op: &str, params: Value) -> R<Value> {
        data(self.raw(op, params)?)
    }
    fn context(&mut self) -> R<Value> {
        self.call("get_context", json!({"selection":[]}))
    }
    fn revision(&mut self) -> R<Value> {
        Ok(self.context()?["project"]["revision"].clone())
    }
    fn apply(&mut self, commands: Value) -> R<Value> {
        let rev = if self.project.is_some() {
            self.revision()?
        } else {
            Value::Null
        };
        self.call(
            "apply_commands",
            json!({"base_revision":rev,"commands":commands}),
        )
    }
    fn object(&mut self, kind: &str, name: &Value, source: Value) -> R<Value> {
        self.call(
            "get_object",
            json!({"object_id":{"kind":kind,"id":name},"source":source,"node_index":null}),
        )
    }
    fn finish(&mut self, handle: &Value) -> R<Value> {
        let mut work = self.host.drain_io_work();
        if work.len() != 1 {
            return Err(format!(
                "expected one actual Host I/O task, got {}",
                work.len()
            ));
        }
        self.host.finish_io(work.pop().unwrap().run());
        let state = self.call("get_task", json!({"task_id":handle["id"]}))?;
        if state["task"]["state"] != "completed" {
            return Err(format!("actual task did not complete: {state}"));
        }
        Ok(state["result"].clone())
    }
    fn import(&mut self, path: &Path) -> R<()> {
        let selected = self
            .host
            .select_directory(&self.ui, FilePurpose::ImportPackage, path.to_owned())
            .map_err(|e| format!("{e:?}"))?;
        let handle=self.call("import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}))?;
        let result = self.finish(&handle)?;
        if result["kind"] != "import" {
            return Err(format!("expected ImportDraft: {result}"));
        }
        let draft = &result["data"];
        self.apply(json!([{"kind":"adopt_imported_draft","data":{"task_id":handle["id"],"choices":{
            "chosen_policy":"open_project","objects_to_create":draft["proposed_objects"],"module_versions":draft["proposed_definitions"],
            "reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],
            "accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]))?;
        self.project = Some(self.context()?["project"]["id"].clone());
        Ok(())
    }
}
fn camera(x: f64, y: f64, wide: bool) -> Value {
    json!({"unit":"meter","coordinate_frame":"right_handed_z_up","zoom":1,
           "eye":if wide{json!({"x":8,"y":-17,"z":13})}else{json!({"x":x+3.5,"y":y-4.2,"z":3.2})},
           "target":if wide{json!({"x":7.5,"y":0,"z":0.8})}else{json!({"x":x,"y":y,"z":0.9})}})
}
fn render_request(
    project: &Value,
    scope_r: &Value,
    frozen_r: &Value,
    target: Value,
    cam: Value,
    lod: &str,
    run: Option<(&Value, u64)>,
) -> Value {
    let (run_id, at) = run
        .map(|(r, t)| (r.clone(), json!(t)))
        .unwrap_or((Value::Null, Value::Null));
    json!({"provenance":if run.is_some(){"actual_run"}else{"actual_saved_draft"},"project_id":project,
       "scope":{"kind":"project"},"scope_revision":scope_r,"frozen_project_revision":frozen_r,
       "hall_id":"H01","line_id":"L03","target":target,
       "operation":"render_view","params":{"revision":scope_r,"object_id":target,"preview":null,
         "delivery":"show_and_capture","module_ref":null,"comparison":null,
         "view":{"kind":if target["kind"]=="device"{"device_detail"}else{"line_canvas"},"canvas":"line_canvas",
             "hall_id":"H01","object_ref":target,"project_revision":frozen_r,"run_id":run_id,"view_time_ms":at,"lod":lod,"camera":cam}}})
}
fn run(root: &Path, out: &Path, plan_path: &Path) -> R<Value> {
    let plan = read(plan_path)?;
    let before = out.join("before");
    let after = out.join("reopened");
    fs::create_dir(&before).map_err(|e| e.to_string())?;
    fs::create_dir(&after).map_err(|e| e.to_string())?;
    let mut h = Driver::new(plan["host_id"].as_str().unwrap_or("l03-producer"), &before);
    for entry in plan["register_order"].as_array().unwrap() {
        h.host
            .register_bundled_module_directory(&root.join(entry["directory"].as_str().unwrap()))
            .map_err(|e| format!("registration {}: {e:?}", entry["id"]))?;
    }
    h.call("query_module_library",json!({"category_filters":["media"],"industry":null,"product_family":null,"site_envelope":null,"source_scope":["bundled"]}))?;
    h.apply(json!([{"kind":"create_project","data":plan["create_project"]}]))?;
    h.project = Some(h.context()?["project"]["id"].clone());
    let project = h.project.clone().unwrap();
    for def in plan["definitions"].as_array().unwrap() {
        let created = h.apply(json!([{"kind":"create_definition_draft","data":def}]))?;
        data(h.raw_scope(json!({"kind":"definition_draft","data":created["scope"]["data"]["id"]}),"apply_commands",
             json!({"base_revision":created["new_revision"],"commands":[{"kind":"commit_definition_version","data":{"version":def["version"]}}]}))?)?;
    }
    for key in [
        "pre_commands",
        "create_commands",
        "formal_media_commands",
        "media_commands",
        "business_commands",
    ] {
        // Each public EditCommit uses the actual current Project revision.
        for command in plan[key].as_array().unwrap() {
            h.apply(json!([command]))?;
        }
    }
    let mut scenario = plan["scenario"].clone();
    scenario["project_revision"] = h.revision()?;
    h.apply(json!([{"kind":"put_object","data":{"kind":"scenario","data":scenario}}]))?;
    let scope_r = h.revision()?;
    let active_before = h.object("project", &project, json!({"kind":"draft"}))?;
    write(&out.join("actual-project-before.json"), &active_before)?;
    // This is a real RunScenario call. The plan declares all source phase material effects.
    let run_reply = h.raw(
        "run_scenario",
        json!({"scenario_id":scenario["id"],"input_revision":scope_r,
                  "observation_window":scenario["window"],"intent":{"kind":"project_scenario"}}),
    )?;
    write(&out.join("actual-run-attempt.json"), &run_reply)?;
    let mut actual_run = Value::Null;
    if run_reply["outcome"]["status"] == "ok" {
        let result = h.finish(&data(run_reply.clone())?)?;
        if result["kind"] != "run" {
            return Err(format!("expected actual Run summary: {result}"));
        }
        actual_run = h.call(
            "get_run",
            json!({"run_id":result["data"]["run_id"],"query":{"kind":"full_result"}}),
        )?["data"]
            .clone();
        if plan["translation_complete"] != true {
            return Err(
                "Host unexpectedly ran the blocked migration; do not label it faithful L03".into(),
            );
        }
        write(&out.join("actual-run.json"), &actual_run)?;
    }
    let mut exact_before = Vec::new();
    let project_t: ProjectId =
        serde_json::from_value(project.clone()).map_err(|e| e.to_string())?;
    let revision_t: Revision =
        serde_json::from_value(scope_r.clone()).map_err(|e| e.to_string())?;
    let all_refs: Vec<Value> = plan["register_order"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| json!({"id":e["id"],"version":e["version"]}))
        .chain(
            plan["definitions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| json!({"id":e["id"],"version":e["version"]})),
        )
        .collect();
    let mut asset_reads = 0;
    for reference in &all_refs {
        let value = h.object(
            "module",
            &reference["id"],
            json!({"kind":"module_version","data":{"module_ref":reference}}),
        )?;
        let package: ModulePackage =
            serde_json::from_value(value["data"].clone()).map_err(|e| e.to_string())?;
        let source: ObjectSource = serde_json::from_value(
            json!({"kind":"module_version","data":{"module_ref":reference}}),
        )
        .map_err(|e| e.to_string())?;
        for asset in &package.assets {
            h.host
                .read_media_asset(
                    &h.ui,
                    &project_t,
                    &ApiScope::Project,
                    &revision_t,
                    &source,
                    asset,
                )
                .map_err(|e| format!("{e:?}"))?;
            asset_reads += 1;
        }
        exact_before.push(value);
    }
    let saved_dir = out.join("host-saved");
    fs::create_dir(&saved_dir).map_err(|e| e.to_string())?;
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::SaveProject, saved_dir.clone())
        .map_err(|e| format!("{e:?}"))?;
    let handle = h.call(
        "save_project",
        json!({"revision":scope_r,"selected_directory_ref":selected}),
    )?;
    let saved = h.finish(&handle)?;
    if saved["kind"] != "save" {
        return Err(format!("not Host SavedProject: {saved}"));
    }
    write(&out.join("actual-save.json"), &saved)?;
    // Host wrote every archive byte. Only move the completed directory, no archive writer exists here.
    let moved = out.join("project-moved");
    fs::rename(&saved_dir, &moved).map_err(|e| format!("physical move: {e}"))?;
    if !moved.join("project.json").is_file() {
        return Err("Host save did not create project.json at selected directory".into());
    }
    drop(h);
    // No bundled registration on fresh Host: the moved archive alone must supply dependencies and bytes.
    let mut fresh = Driver::new("l03-reopened", &after);
    fresh.import(&moved)?;
    if fresh.project.as_ref() != Some(&project) {
        return Err("reopened ProjectId changed".into());
    }
    let current_r = fresh.revision()?;
    let active_after = fresh.object("project", &project, json!({"kind":"draft"}))?;
    if active_before != active_after {
        return Err("project definitions/poses/revision changed on reopen".into());
    }
    write(&out.join("actual-project-reopened.json"), &active_after)?;
    let fresh_revision: Revision =
        serde_json::from_value(current_r.clone()).map_err(|e| e.to_string())?;
    for (reference, expected) in all_refs.iter().zip(&exact_before) {
        let got = fresh.object(
            "module",
            &reference["id"],
            json!({"kind":"module_version","data":{"module_ref":reference}}),
        )?;
        if &got != expected {
            return Err(format!("exact module changed on reopen: {reference}"));
        }
        let a: ModulePackage =
            serde_json::from_value(expected["data"].clone()).map_err(|e| e.to_string())?;
        let source: ObjectSource = serde_json::from_value(
            json!({"kind":"module_version","data":{"module_ref":reference}}),
        )
        .map_err(|e| e.to_string())?;
        for asset in &a.assets {
            let bytes = fresh
                .host
                .read_media_asset(
                    &fresh.ui,
                    &project_t,
                    &ApiScope::Project,
                    &fresh_revision,
                    &source,
                    asset,
                )
                .map_err(|e| format!("reopen bytes: {e:?}"))?;
            let disk = plan["register_order"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["id"] == reference["id"] && e["version"] == reference["version"])
                .ok_or("asset owner is absent from exact registration")?;
            let original = read(
                &root
                    .join(disk["directory"].as_str().unwrap())
                    .join("module.json"),
            )?;
            let old = original["data"]["assets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| asset.id.as_str().ends_with(x["id"].as_str().unwrap()))
                .ok_or("asset ID missing in exact source package")?;
            if bytes
                != fs::read(
                    root.join(disk["directory"].as_str().unwrap())
                        .join(old["path"].as_str().unwrap()),
                )
                .map_err(|e| e.to_string())?
            {
                return Err("reopened asset differs from frozen package bytes".into());
            }
        }
    }
    let mut requests = vec![];
    for mapping in plan["mapping"].as_array().unwrap() {
        let target = json!({"kind":mapping["object_kind"],"id":mapping["instance"]});
        fresh.object(
            mapping["object_kind"].as_str().unwrap(),
            &mapping["instance"],
            json!({"kind":"draft"}),
        )?;
        let (x, y) = if mapping["object_kind"] == "device" {
            (
                mapping["pose"]["translation"]["x"].as_f64().unwrap(),
                mapping["pose"]["translation"]["y"].as_f64().unwrap(),
            )
        } else {
            (14.9, 0.0)
        };
        for lod in ["detailed", "simplified"] {
            requests.push(render_request(
                &project,
                &current_r,
                &current_r,
                target.clone(),
                camera(x, y, false),
                lod,
                None,
            ));
        }
    }
    requests.push(render_request(
        &project,
        &current_r,
        &current_r,
        json!({"kind":"line","id":"L03"}),
        camera(0.0, 0.0, true),
        "detailed",
        None,
    ));
    let mut states = BTreeSet::new();
    let mut snapshots = vec![];
    if !actual_run.is_null() {
        let rid = &actual_run["id"];
        let reopened = fresh.call(
            "get_run",
            json!({"run_id":rid,"query":{"kind":"full_result"}}),
        )?;
        if reopened["data"] != actual_run {
            return Err("entire actual Run changed on reopen".into());
        }
        let mut times: BTreeSet<u64> = plan["requested_times_ms"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        for event in actual_run["events"].as_array().unwrap() {
            if let Some(t) = event["at_ms"].as_u64() {
                times.insert(t);
                if t > 0 {
                    times.insert(t - 1);
                }
            }
        }
        for t in times
            .into_iter()
            .filter(|t| *t <= actual_run["computed_until_ms"].as_u64().unwrap())
        {
            let mut objects = vec![];
            let job_selectors: Vec<Value> = actual_run["jobs"].as_array().unwrap().iter()
                .map(|job| json!({"kind":"job","id":job["id"]})).collect();
            for selectors in [json!([]), json!(job_selectors)] {
            let mut cursor = Value::Null;
            loop {
                let page=fresh.call("get_run",json!({"run_id":rid,"query":{"kind":"objects_at","data":{"at_ms":t,"objects":selectors,"page":{"after_index":cursor,"limit":100}}}}))?;
                objects.extend(
                    page["data"]["objects"]
                        .as_array()
                        .ok_or("objects_at missing objects")?
                        .clone(),
                );
                cursor = page["data"]["next_index"].clone();
                if cursor.is_null() {
                    break;
                }
            }
            }
            for object in &objects {
                if object["kind"] == "job" {
                    if let Some(s) = object["data"]["state"].as_str() {
                        states.insert(s.to_owned());
                    }
                }
            }
            snapshots.push(json!({"run_id":rid,"at_ms":t,"objects":objects}));
            requests.push(render_request(
                &project,
                &current_r,
                &actual_run["input_ref"]["project_revision"],
                json!({"kind":"line","id":"L03"}),
                camera(0.0, 0.0, true),
                "detailed",
                Some((rid, t)),
            ));
        }
        for r in &all_refs {
            fresh.object(
                "module",
                &r["id"],
                json!({"kind":"run_module_version","data":{"run_id":rid,"at_ms":0,"module_ref":r}}),
            )?;
        }
    }
    write(&out.join("actual-run-snapshots.json"), &json!(snapshots))?;
    write(&out.join("native-requests.json"), &json!(requests))?;
    let missing: Vec<_> = plan["required_states"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !states.contains(s.as_str().unwrap()))
        .cloned()
        .collect();
    Ok(
        json!({"status":if actual_run.is_null(){"actual_static_draft_saved_moved_reopened_business_blocked"}else{"actual_run_saved_moved_reopened_pending_Native"},
       "archive_absolute_path":moved,"project_id":project,"scope_revision":current_r,"draft_project_revision":current_r,
       "run_id":actual_run.get("id"),"frozen_project_revision":actual_run.get("input_ref").and_then(|v|v.get("project_revision")),
       "actual_host":true,"physical_move":true,"fresh_host_reopen":true,"exact_packages":all_refs.len(),"asset_reads_before":asset_reads,
       "actual_states":states,"missing_actual_states":missing,"business_equivalent":null,
       "business_acceptance":"Requires Root verification of original acceptance times, composition, quality, holder transfer and closure after contract gaps are resolved",
       "blocking_gaps":plan["blocking_gaps"],"native_requests":out.join("native-requests.json"),"Native_executed":false,
       "target_machine_NFR":null,"industrial_or_human_acceptance":null,"whole_task_done":false}),
    )
}
fn main() {
    let root = std::env::current_dir().unwrap();
    let plan_path = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(||root.join("scenarios/媒体表现/runtime-l03/command-plan.json"));
    let plan_path = root.join(plan_path).canonicalize().unwrap();
    let lease = plan_path.parent().unwrap();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out = lease.join(format!("execution-{stamp}"));
    fs::create_dir(&out).unwrap();
    match run(&root, &out, &plan_path) {
        Ok(v) => {
            write(&out.join("receipt.json"), &v).unwrap();
            write(&lease.join("execution-current.json"), &v).unwrap();
            write(&lease.join("native-input-index.json"), &v).unwrap();
            println!("{}", serde_json::to_string_pretty(&v).unwrap());
            if v["run_id"].is_null() {
                std::process::exit(2);
            }
        }
        Err(e) => {
            let v = json!({"status":"actual_Host_driver_failed","error":e,"output_directory":out,"archive_absolute_path":null,
            "run_id":null,"whole_task_done":false});
            write(&lease.join("execution-current.json"), &v).unwrap();
            write(&lease.join("native-input-index.json"), &v).unwrap();
            eprintln!("{v}");
            std::process::exit(1);
        }
    }
}
