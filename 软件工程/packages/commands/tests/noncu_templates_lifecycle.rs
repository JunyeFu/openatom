//! P4-01 CLI-only four-entry lifecycle, including a genuinely new OS process.
#[path = "fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use sf_module_kit::*;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
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
fn fresh(name: &str) -> Harness {
    let ui = id(&format!("{name}-ui"));
    Harness {
        host: ProjectHost::new(id(name), ui.clone()),
        ui,
        tree: Sandbox::new(),
    }
}
fn api(h: &mut Harness, p: Option<&ProjectId>, op: &str, params: Value) -> Value {
    let request=serde_json::from_value(json!({"api_version":"0.1","request_id":"noncu-p401","project_id":p,
        "scope":{"kind":if p.is_some(){"project"}else{"workspace"}},"operation":op,"params":params})).unwrap();
    h.host.dispatch(&h.ui, request)
}
fn context(h: &mut Harness) -> ProjectContext {
    ok(api(h, None, "get_context", json!({"selection":[]})))
}
fn object(h: &mut Harness, p: &ProjectId, kind: &str, name: &str) -> Value {
    let v: DomainObject = ok(api(
        h,
        Some(p),
        "get_object",
        json!({"object_id":{"kind":kind,"id":name},"source":{"kind":"draft"},"node_index":null}),
    ));
    json!(v)["data"].clone()
}
fn project(h: &mut Harness, p: &ProjectId) -> ProjectState {
    serde_json::from_value(object(h, p, "project", p.as_str())).unwrap()
}
fn frozen_project(h: &mut Harness, p: &ProjectId, r: &RunId) -> Value {
    let value: DomainObject = ok(api(
        h,
        Some(p),
        "get_object",
        json!({"object_id":{"kind":"project","id":p},"source":{"kind":"run_snapshot","data":{"run_id":r,"at_ms":0}},"node_index":null}),
    ));
    json!(value)["data"].clone()
}
fn apply(h: &mut Harness, p: &ProjectId, commands: Value) {
    let base = project(h, p).revision;
    let _: EditCommit = ok(api(
        h,
        Some(p),
        "apply_commands",
        json!({"base_revision":base,"commands":commands}),
    ));
}
fn full(h: &mut Harness, p: &ProjectId, r: &RunId) -> RunResult {
    let v: RunView = ok(api(
        h,
        Some(p),
        "get_run",
        json!({"run_id":r,"query":{"kind":"full_result"}}),
    ));
    let RunView::Full(v) = v else {
        panic!("actual full Run")
    };
    *v
}
fn finish(h: &mut Harness, p: Option<&ProjectId>, t: &TaskHandle) -> TaskState {
    h.finish_one();
    let s: TaskState = ok(api(h, p, "get_task", json!({"task_id":t.id})));
    assert!(
        matches!(s.task.state, ComputationStatus::Completed),
        "{s:?}"
    );
    s
}
fn run(h: &mut Harness, p: &ProjectId, scenario: &str) -> RunResult {
    let s = object(h, p, "scenario", scenario);
    let revision = project(h, p).revision;
    let task: TaskHandle = ok(api(
        h,
        Some(p),
        "run_scenario",
        json!({"scenario_id":scenario,"input_revision":revision,"observation_window":s["window"],"intent":{"kind":"project_scenario"}}),
    ));
    let state = finish(h, Some(p), &task);
    let Some(TaskOutput::Run(s)) = state.result else {
        panic!("actual Run")
    };
    full(h, p, &s.run_id)
}
fn accepted(r: &RunResult) -> Vec<u64> {
    r.acceptance_events
        .iter()
        .map(|e| e.at_ms.0.value())
        .collect()
}
fn save(h: &mut Harness, p: &ProjectId, path: &Path) {
    fs::create_dir_all(path).unwrap();
    let selected = h
        .host
        .select_directory(&h.ui, FilePurpose::SaveProject, path.to_owned())
        .unwrap();
    let revision = project(h, p).revision;
    let t: TaskHandle = ok(api(
        h,
        Some(p),
        "save_project",
        json!({"revision":revision,"selected_directory_ref":selected}),
    ));
    finish(h, Some(p), &t);
}
fn open(h: &mut Harness, path: &Path) -> ProjectId {
    let t = h.import(path);
    h.finish_one();
    let draft = h.imported(&t);
    let _: EditCommit = ok(h.adopt(&t, &draft, false));
    context(h).project.unwrap().id
}
fn history(h: &mut Harness, p: &ProjectId, r: &RunResult, t: u64) -> Value {
    let mut objects = vec![];
    let selectors = [
        json!([]),
        json!(
            r.jobs
                .iter()
                .map(|j| json!({"kind":"job","id":j.id}))
                .collect::<Vec<_>>()
        ),
    ];
    for selected in selectors {
        let v: RunView = ok(api(
            h,
            Some(p),
            "get_run",
            json!({"run_id":r.id,"query":{"kind":"objects_at","data":{"at_ms":t,"objects":selected,"page":{"after_index":null,"limit":1000}}}}),
        ));
        let RunView::Objects(page) = v else {
            panic!("canonical history")
        };
        assert!(page.next_index.is_none());
        objects.extend(page.objects);
    }
    json!({"at_ms":t,"objects":objects})
}
fn all() -> [(BuiltinTemplate, &'static str, [u64; 2]); 4] {
    [
        (
            BuiltinTemplate::MachiningDelivery,
            "machining-delivery",
            [90000, 150000],
        ),
        (
            BuiltinTemplate::PurchasedAssembly,
            "purchased-assembly",
            [70000, 110000],
        ),
        (
            BuiltinTemplate::ParallelElectromechanical,
            "parallel-electromechanical",
            [160000, 200000],
        ),
        (
            BuiltinTemplate::CartonFactory,
            "carton-factory",
            [45000, 65000],
        ),
    ]
}

#[test]
fn all_four_templates_adopt_run_edit_save_destroy_and_new_process_reopen() {
    let output_root = root().join(format!(
        ".work/delivery/artifacts/noncu-{}",
        std::process::id()
    ));
    fs::create_dir_all(&output_root).unwrap();
    let mut results = vec![];
    for (which, name, expected) in all() {
        let directory = output_root.join(name);
        fs::create_dir(&directory).unwrap();
        let candidate = builtin_template(which).unwrap();
        let source = json!(candidate.example_project);
        let subset = json!(candidate.subset);
        let mut h = fresh(&format!("noncu-{name}"));
        for (_, label, _) in all() {
            h.host
                .register_bundled_template_directory(
                    &root()
                        .join("modules/templates")
                        .join(label)
                        .join("candidate"),
                )
                .unwrap();
        }
        let _: EditCommit = ok(api(
            &mut h,
            None,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":format!("P4-01 {name} independent lifecycle"),"time_reference":candidate.example_project.time_reference,"departure":"stay"}}]}),
        ));
        let p = context(&mut h).project.unwrap().id;
        let library: ModuleTemplateCandidates = ok(api(
            &mut h,
            Some(&p),
            "query_module_library",
            json!({"category_filters":["templates"],"industry":null,"product_family":null,"site_envelope":null,"source_scope":["bundled"]}),
        ));
        let reference = ModuleRef {
            id: candidate.package.definition.id.clone(),
            version: candidate.package.definition.version.clone(),
        };
        assert!(
            library
                .candidates
                .iter()
                .any(|c| json!(c.module_ref) == json!(reference)),
            "explicit exact source must appear in the real library"
        );
        // Existing worker is an explicit actual object, not automatic staffing.
        let worker = candidate
            .example_project
            .resources
            .iter()
            .find(|r| matches!(r.kind, ResourceKind::Worker))
            .unwrap();
        let mut existing = worker.clone();
        existing.id = id("existing-worker");
        existing.home_location = None;
        let mut calendar = candidate
            .example_project
            .calendars
            .iter()
            .find(|c| c.id == worker.calendar_id)
            .unwrap()
            .clone();
        calendar.id = id("existing-worker-calendar");
        existing.calendar_id = calendar.id.clone();
        let source_item = candidate
            .example_project
            .item_types
            .iter()
            .find(|i| {
                candidate
                    .example_project
                    .boms
                    .iter()
                    .any(|bom| bom.rows.iter().any(|r| r.item_type_ref.id == i.id))
            })
            .unwrap();
        let mut existing_item = source_item.clone();
        existing_item.id = id("existing-material-type");
        apply(
            &mut h,
            &p,
            json!([
                put("calendar", json!(calendar)),
                put("resource", json!(existing)),
                put("item_type", json!(existing_item))
            ]),
        );
        let before = project(&mut h, &p);
        let mut choices = TemplateChoices {
            namespace: format!("new-{name}"),
            resources: candidate
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
        choices.resources.insert(
            worker.id.clone(),
            TemplateResourceSelection::Reuse(existing.id.clone()),
        );
        let mut unknown_reuse = choices.clone();
        unknown_reuse
            .reuse_items
            .insert(source_item.id.clone(), existing_item.id.clone());
        let rejected = prepare_template(&candidate, &before, &unknown_reuse).unwrap_err();
        assert!(
            rejected.iter().any(|e| e.code == "template.item_mismatch"
                && e.message.contains("known exact specification")),
            "unknown source specifications must not be guessed: {rejected:?}"
        );
        output(
            &directory.join("unknown-material-reuse-rejected.json"),
            &json!(rejected),
        );
        let mut plan = prepare_template(&candidate, &before, &choices).unwrap();
        let site: Option<SiteEnvelope> = if name == "carton-factory" {
            Some(serde_json::from_value(json!({"regions":[],"length_m":31,"width_m":13,"height_m":5,"usable_polygon_m":[],"clearance_m":0})).unwrap())
        } else {
            None
        };
        if let Some(site) = &site {
            fill_template_site(&mut plan, site).unwrap();
        }
        let command = plan.command_data_with_preparation(
            &"example-project.json".to_owned().try_into().unwrap(),
            &choices,
            site.as_ref(),
            None,
        );
        apply(
            &mut h,
            &p,
            json!([{"kind":"instantiate_template","data":command}]),
        );
        let adopted = project(&mut h, &p);
        assert!(
            adopted.scenario_refs.is_empty(),
            "template adoption must not copy orders, stock or existing runs"
        );
        assert!(context(&mut h).run_summaries.is_empty());
        assert_eq!(
            json!(
                adopted
                    .resources
                    .iter()
                    .find(|r| r.id == existing.id)
                    .unwrap()
            ),
            json!(existing)
        );
        assert_eq!(
            adopted.resources.len(),
            before.resources.len() + candidate.example_project.resources.len() - 1
        );
        assert!(plan.id_map.iter().all(|(old, new)| old != new));
        assert_eq!(json!(candidate.example_project), source);
        assert_eq!(json!(candidate.subset), subset);
        let scenario_id: ScenarioId = format!("{name}-explicit-supply-demand").try_into().unwrap();
        let mut seed = prepare_template_seed(
            &candidate,
            &plan,
            &TemplateSeedChoices {
                scenario_id: scenario_id.clone(),
                supply: true,
                demand: true,
                initial_quality: candidate
                    .subset
                    .adoption
                    .as_ref()
                    .unwrap()
                    .deferred_seed
                    .as_ref()
                    .unwrap()
                    .initial_stock
                    .iter()
                    .map(|s| (s.location_id.clone(), QualityState::Released))
                    .collect(),
                adopt_declared_inspection_results: true,
            },
        )
        .unwrap();
        assert_eq!(seed.orders.len(), 1);
        assert_eq!(
            candidate.example_project.products.len(),
            1,
            "unambiguous exact source product; reordered multi-product cases remain covered by existing SDK regressions"
        );
        assert_eq!(
            seed.orders[0].product_ref.id,
            plan.id_map[&candidate.example_project.products[0].id]
        );
        seed.project_revision = adopted.revision.clone();
        let settings = json!({"title":adopted.title,"time_reference":seed.time_reference,"generation_context":adopted.generation_context,"sources":adopted.sources,"setup_policies":adopted.setup_policies});
        apply(
            &mut h,
            &p,
            json!([{"kind":"set_project_settings","data":settings},put("scenario",json!(seed))]),
        );
        let original_scenario = object(&mut h, &p, "scenario", scenario_id.as_str());
        let old = run(&mut h, &p, scenario_id.as_str());
        let old_frozen = frozen_project(&mut h, &p, &old.id);
        assert_eq!(accepted(&old), expected);
        let expected_item = &candidate.example_project.products[0].item_type_ref;
        for event in &old.acceptance_events {
            for item_id in &event.item_refs {
                let item = old.final_items.iter().find(|i| i.id == *item_id).unwrap();
                assert_eq!(item.item_type_ref.id, plan.id_map[&expected_item.id]);
                assert_eq!(item.item_type_ref.revision, expected_item.revision);
            }
        }
        assert!(!old.resource_claims.is_empty());
        assert!(
            old.resource_claims
                .iter()
                .all(|claim| adopted.resources.iter().any(|r| r.id == claim.resource_id))
        );
        assert!(matches!(
            old.evidence_origin,
            EvidenceOrigin::ActualSoftware
        ));
        assert!(old.whole_window_computed);
        let times = [
            0,
            10000,
            expected[0] / 2,
            expected[0] - 1,
            expected[0],
            old.computed_until_ms.as_ref().unwrap().0.value(),
        ];
        let snapshots: Vec<_> = times
            .iter()
            .map(|t| history(&mut h, &p, &old, *t))
            .collect();
        // Delay all newly adopted availability by 10 s. A single parallel-branch
        // worker can have slack, so its delay alone need not delay the product.
        let calendar_edits: Vec<_> = project(&mut h, &p)
            .calendars
            .iter()
            .filter(|c| c.id != calendar.id)
            .map(|c| {
                let mut edited = c.clone();
                assert_eq!(edited.intervals[0].start_ms.0.value(), 0);
                edited.intervals[0].start_ms = SimTimeMs(10000u64.try_into().unwrap());
                put("calendar", json!(edited))
            })
            .collect();
        assert!(!calendar_edits.is_empty());
        apply(&mut h, &p, json!(calendar_edits));
        assert_eq!(json!(full(&mut h, &p, &old.id)), json!(old));
        assert_eq!(frozen_project(&mut h, &p, &old.id), old_frozen);
        let live = project(&mut h, &p);
        assert_eq!(
            json!(live.calendars.iter().find(|c| c.id == calendar.id).unwrap()),
            json!(calendar),
            "target resource availability is preserved"
        );
        let scenario = object(&mut h, &p, "scenario", scenario_id.as_str());
        for (key, value) in original_scenario.as_object().unwrap() {
            if !["revision", "project_revision"].contains(&key.as_str()) {
                assert_eq!(
                    scenario[key], *value,
                    "calendar edit must preserve scenario field {key}"
                );
            }
        }
        let saved = directory.join("saved-project");
        save(&mut h, &p, &saved);
        let moved = directory.join("receiver/moved-project");
        fs::create_dir_all(&moved).unwrap();
        // Move only this test's newly saved directory inside its own output root.
        fs::remove_dir(&moved).unwrap(); // empty directory just created above
        assert!(saved.starts_with(&output_root) && moved.starts_with(&output_root));
        fs::rename(&saved, &moved).unwrap();
        assert!(!saved.exists());
        let packet = json!({"name":name,"project":live,"scenario_id":scenario_id,"scenario":scenario,"old_run":old,"old_frozen_project":old_frozen,"history":snapshots,"expected_new_times":expected.map(|t|t+15000),"parent_pid":std::process::id(),"reference":reference,"existing_worker":existing,
            "source_template":source,"source_subset":subset,"moved_project":moved,"site":site,"new_ids":plan.id_map});
        output(&directory.join("packet.json"), &packet);
        drop(h);
        let mut process = Command::new(std::env::current_exe().unwrap());
        process
            .args([
                "--ignored",
                "--exact",
                "noncu_templates_child_reopens",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("SF_NONCU_TEMPLATE_CASE", directory.join("packet.json"));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            process.creation_flags(0x08000000);
        }
        let result = process.output().unwrap();
        fs::write(directory.join("child.stdout.txt"), &result.stdout).unwrap();
        fs::write(directory.join("child.stderr.txt"), &result.stderr).unwrap();
        assert!(
            result.status.success(),
            "new process failed for {name}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let child = read(&directory.join("child-result.json"));
        assert_eq!(child["passed"], true);
        results.push(child);
    }
    output(
        &output_root.join("result.json"),
        &json!({"passed":true,"cases":results,"new_processes":4,"Native":false,"human_acceptance":null,"whole_task_done":false}),
    );
    output(
        &root().join(".work/delivery/lifecycle-result.json"),
        &json!({"passed":true,"result":output_root.join("result.json").strip_prefix(root()).unwrap()}),
    );
}

#[test]
#[ignore = "Fresh OS process worker; only the parent supplies its private packet"]
fn noncu_templates_child_reopens() {
    let packet_path = PathBuf::from(std::env::var_os("SF_NONCU_TEMPLATE_CASE").unwrap());
    let directory = packet_path.parent().unwrap();
    let packet = read(&packet_path);
    let mut h = fresh(&format!("noncu-child-{}", packet["name"].as_str().unwrap()));
    let p = open(&mut h, Path::new(packet["moved_project"].as_str().unwrap()));
    assert_eq!(json!(project(&mut h, &p)), packet["project"]);
    let old: RunResult = serde_json::from_value(packet["old_run"].clone()).unwrap();
    assert_eq!(
        frozen_project(&mut h, &p, &old.id),
        packet["old_frozen_project"]
    );
    assert_eq!(json!(full(&mut h, &p, &old.id)), json!(old));
    assert_eq!(
        object(
            &mut h,
            &p,
            "scenario",
            packet["scenario_id"].as_str().unwrap()
        ),
        packet["scenario"]
    );
    for point in packet["history"].as_array().unwrap().iter().rev() {
        assert_eq!(
            history(&mut h, &p, &old, point["at_ms"].as_u64().unwrap()),
            *point
        );
    }
    assert_ne!(json!(std::process::id()), packet["parent_pid"]);
    // The receiver edits actual availability AFTER reopening in a new OS process.
    let before_receiver_edit = project(&mut h, &p);
    let edits: Vec<_> = before_receiver_edit.calendars.iter()
        .filter(|c| c.id.as_str() != "existing-worker-calendar")
        .map(|c| {
            let mut changed = c.clone();
            assert_eq!(changed.intervals[0].start_ms.0.value(), 10000);
            changed.intervals[0].start_ms = SimTimeMs(15000u64.try_into().unwrap());
            put("calendar", json!(changed))
        }).collect();
    assert!(!edits.is_empty());
    apply(&mut h, &p, json!(edits));
    assert_ne!(json!(project(&mut h, &p).revision), json!(before_receiver_edit.revision));
    assert_eq!(json!(full(&mut h, &p, &old.id)), json!(old));
    let new = run(&mut h, &p, packet["scenario_id"].as_str().unwrap());
    assert_eq!(json!(accepted(&new)), packet["expected_new_times"]);
    assert_eq!(json!(full(&mut h, &p, &old.id)), json!(old));
    assert_eq!(
        frozen_project(&mut h, &p, &old.id),
        packet["old_frozen_project"]
    );
    assert_eq!(
        project(&mut h, &p)
            .resources
            .iter()
            .find(|r| r.id.as_str() == "existing-worker")
            .map(|r| json!(r))
            .unwrap(),
        packet["existing_worker"]
    );
    let both = directory.join("reopened-with-two-runs");
    save(&mut h, &p, &both);
    drop(h);
    let mut final_host = fresh("noncu-final-read");
    assert_eq!(open(&mut final_host, &both), p);
    assert_eq!(json!(full(&mut final_host, &p, &old.id)), json!(old));
    assert_eq!(
        frozen_project(&mut final_host, &p, &old.id),
        packet["old_frozen_project"]
    );
    assert_eq!(json!(full(&mut final_host, &p, &new.id)), json!(new));
    for point in packet["history"].as_array().unwrap() {
        assert_eq!(
            history(&mut final_host, &p, &old, point["at_ms"].as_u64().unwrap()),
            *point
        );
    }
    output(
        &directory.join("child-result.json"),
        &json!({"name":packet["name"],"passed":true,"pid":std::process::id(),"parent_pid":packet["parent_pid"],"edited_after_new_process_reopen":true,"availability_start_ms":15000,"old_run":old,"new_run":new,"final_project":project(&mut final_host,&p),"saved_two_run_project":both,"history_points":packet["history"].as_array().unwrap().len(),"Native":false}),
    );
}

#[test]
fn nine_declared_categories_are_actual_editable_definition_drafts() {
    let mut h = fresh("noncu-nine-categories");
    let _: EditCommit = ok(api(
        &mut h,
        None,
        "apply_commands",
        json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"Actual nine-category authoring","time_reference":{"label":"declared examples only"},"departure":"stay"}}]}),
    ));
    let p = context(&mut h).project.unwrap().id;
    let before = json!(project(&mut h, &p));
    let mut categories = std::collections::BTreeSet::new();
    let mut records = vec![];
    for package in builtin_template_support().unwrap() {
        let base = project(&mut h, &p).revision;
        let definition = json!(package.definition);
        let commit: EditCommit = ok(api(
            &mut h,
            Some(&p),
            "apply_commands",
            json!({"base_revision":base,"commands":[{"kind":"create_definition_draft","data":definition}]}),
        ));
        let EditScope::DefinitionDraft(draft) = commit.scope else {
            panic!("actual draft")
        };
        let mut edited = definition.clone();
        edited["title"] = json!(format!(
            "{} / independently editable",
            definition["title"].as_str().unwrap()
        ));
        let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"noncu-category-edit","project_id":p,"scope":{"kind":"definition_draft","data":draft.id},"operation":"apply_commands",
            "params":{"base_revision":draft.revision,"commands":[{"kind":"put_definition","data":edited}]}})).unwrap();
        let _: EditCommit = ok(h.host.dispatch(&h.ui, request));
        let request:ApiRequest=serde_json::from_value(json!({"api_version":"0.1","request_id":"noncu-category-read","project_id":p,"scope":{"kind":"definition_draft","data":draft.id},"operation":"get_object",
            "params":{"object_id":{"kind":"definition_draft","id":draft.id},"source":{"kind":"draft"},"node_index":null}})).unwrap();
        let readback: DomainObject = ok(h.host.dispatch(&h.ui, request));
        assert_eq!(json!(readback)["data"], edited);
        let category = definition["payload"]["category"]
            .as_str()
            .unwrap()
            .to_owned();
        categories.insert(category.clone());
        records.push(json!({"category":category,"definition_id":definition["id"],"draft_id":draft.id,"edited_and_read":true,"solver_run_claimed":false}));
    }
    assert_eq!(
        categories,
        std::collections::BTreeSet::from(
            [
                "items",
                "devices",
                "resources",
                "tasks",
                "operations",
                "transport",
                "templates",
                "professional",
                "media"
            ]
            .map(str::to_owned)
        )
    );
    assert_eq!(json!(project(&mut h, &p)), before);
    assert!(context(&mut h).run_summaries.is_empty());
    output(
        &root().join(".work/delivery/nine-category-result.json"),
        &json!({"passed":true,"categories":records,"project_unchanged":true,"professional_solver_or_production_not_claimed":true,"Native":false}),
    );
}

