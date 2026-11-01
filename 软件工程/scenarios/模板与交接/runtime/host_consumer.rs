//! Actual source-selected template adoption and full bounded acceptance.
#[path = "../../../packages/commands/tests/fixtures/production/host_support.rs"]
mod support;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use sf_module_kit::*;
use std::collections::BTreeMap;
use std::path::Path;
use support::*;

fn new_project(c: &TemplateCandidate, reuse: bool) -> ProjectState {
    let mut p = empty_template_project(
        "P1".to_owned().try_into().unwrap(),
        "P4-01 source-selected acceptance".into(),
        c.example_project.time_reference.clone(),
    );
    p.module_refs = c
        .definitions
        .iter()
        .map(|d| ModuleRef {
            id: d.id.clone(),
            version: d.version.clone(),
        })
        .collect();
    p.module_refs.push(ModuleRef {
        id: c.package.definition.id.clone(),
        version: c.package.definition.version.clone(),
    });
    p.scenario_refs.push("LOCAL".to_owned().try_into().unwrap());
    if reuse {
        let source = c
            .example_project
            .resources
            .iter()
            .find(|r| r.id.as_str() == "W-B")
            .unwrap();
        let mut worker = source.clone();
        worker.id = id("actual-existing-worker");
        worker.title = "existing worker, explicitly selected".into();
        worker.calendar_id = id("actual-existing-calendar");
        worker.home_location = Some(id("actual-existing-home"));
        worker.qualifications.push("existing-qualification".into());
        let mut calendar = c
            .example_project
            .calendars
            .iter()
            .find(|x| x.id == source.calendar_id)
            .unwrap()
            .clone();
        calendar.id = worker.calendar_id.clone();
        calendar.source = SourceBasis::UserProvided;
        let mut home = c.example_project.locations[0].clone();
        home.id = id("actual-existing-home");
        home.hall_id = None;
        home.accepted_item_types.clear();
        home.capacity = None;
        p.locations.push(home);
        p.resources.push(worker);
        p.calendars.push(calendar);
    }
    p
}
fn harness(
    name: &str,
    candidate: &TemplateCandidate,
    project: ProjectState,
    root: &Path,
) -> Harness {
    let mut empty_scenario = fixture()["scenario"].clone();
    for k in [
        "orders",
        "production_tasks",
        "arrivals",
        "scope",
        "quality_presets",
    ] {
        empty_scenario[k] = json!([]);
    }
    empty_scenario["initial_conditions"]["items"] = json!([]);
    let tree = Sandbox::new();
    let directory = tree.directory("selected-template-source");
    seed(
        &directory,
        &json!({"project":project,"scenario":empty_scenario,"definitions":candidate.definitions}),
    );
    let mut manifest = read(&directory.join("project.json"));
    let entries = manifest["data"]["documents"].as_array_mut().unwrap();
    let source = root.join("modules/templates").join(name).join("candidate");
    for f in ["template.json", "source-project.design.json"] {
        std::fs::copy(source.join(f), directory.join(f)).unwrap();
        entries.push(document(
            if f == "template.json" {
                "template_data"
            } else {
                "supporting_file"
            },
            if f == "template.json" {
                "template"
            } else {
                "asset"
            },
            "module",
            candidate.package.definition.id.as_str(),
            f,
            Value::Null,
        ));
    }
    // All nested source bytes are included so Save/Open has the actual companions.
    fn copy_files(src: &Path, dst: &Path, base: &Path, entries: &mut Vec<Value>) {
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            let p = e.path();
            if e.file_type().unwrap().is_dir() {
                copy_files(&p, dst, base, entries);
            } else {
                let mut rel = p
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel == "project.json" {
                    rel = "example-blueprint.json".into();
                }
                if matches!(
                    rel.as_str(),
                    "template.json" | "source-project.design.json" | "module.json"
                ) {
                    continue;
                }
                std::fs::create_dir_all(dst.join(&rel).parent().unwrap()).unwrap();
                std::fs::copy(&p, dst.join(&rel)).unwrap();
                entries.push(document(
                    if rel == "generation-profile.json" {
                        "generation_profile"
                    } else {
                        "supporting_file"
                    },
                    if rel == "generation-profile.json" {
                        "generation_profile"
                    } else {
                        "asset"
                    },
                    "module",
                    "p401-source",
                    &rel,
                    Value::Null,
                ));
            }
        }
    }
    copy_files(&source, &directory, &source, entries);
    write(
        &directory.join("template.json"),
        &json!({"schema_version":"1.0","data":candidate.subset}),
    );
    write(
        &directory.join("example-project.json"),
        &json!({"schema_version":"1.0","data":candidate.example_project}),
    );
    let outer = candidate.package.clone();
    // Keep the actual selected ModulePackage file list intact.
    write(
        &directory.join("template-module.json"),
        &json!({"schema_version":"1.0","data":outer}),
    );
    let mut entry = document(
        "module_definition",
        "module",
        "module",
        candidate.package.definition.id.as_str(),
        "template-module.json",
        Value::Null,
    );
    entry["module_ref"] = json!({"id":candidate.package.definition.id,"version":candidate.package.definition.version});
    entries.push(entry);
    write(&directory.join("project.json"), &manifest);
    Harness::from_seed(tree, &directory)
}
fn choices_for(c: &TemplateCandidate, namespace: &str, reuse: bool) -> TemplateChoices {
    let mut resources: BTreeMap<_, _> = c
        .example_project
        .resources
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                TemplateResourceSelection::CreateDeclaredExample,
            )
        })
        .collect();
    if reuse {
        resources.insert(
            id("W-B"),
            TemplateResourceSelection::Reuse(id("actual-existing-worker")),
        );
    }
    TemplateChoices {
        namespace: namespace.into(),
        resources,
        reuse_items: BTreeMap::new(),
        include_example_product: true,
    }
}
fn explicit_site() -> SiteEnvelope {
    let mut s = carton_generation_profile().unwrap().site_applicability;
    s.length_m = Some(32.0.try_into().unwrap());
    s.width_m = Some(20.0.try_into().unwrap());
    s
}
fn seed_for(c: &TemplateCandidate, p: &PreparedTemplate, sid: &str) -> ScenarioInput {
    prepare_template_seed(
        c,
        p,
        &TemplateSeedChoices {
            scenario_id: sid.to_owned().try_into().unwrap(),
            supply: true,
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
            adopt_declared_inspection_results: true,
        },
    )
    .unwrap()
}
fn expected_metadata(project: &Value, plan: &PreparedTemplate) {
    assert_eq!(
        project["template_adoptions"],
        serde_json::to_value(&plan.proposed_project.template_adoptions).unwrap(),
        "all adoption references/resource choices must equal the independently prepared remap"
    );
    assert_eq!(
        project["sources"],
        serde_json::to_value(&plan.proposed_project.sources).unwrap(),
        "trusted source records must be exact, not merely nonempty"
    );
    assert_eq!(
        project
            .get("parameter_sources")
            .cloned()
            .unwrap_or(json!([])),
        serde_json::to_value(&plan.proposed_project.parameter_sources).unwrap(),
        "parameter value/basis/source/target must exactly match preparation"
    );
}
fn actual_run(h: &mut Harness, expected: &[u64]) -> RunResult {
    let handle = h.run("P401");
    h.finish_one();
    let r = h.completed_run(&handle);
    assert!(r.whole_window_computed);
    assert_eq!(
        r.acceptance_events
            .iter()
            .map(|a| a.at_ms.0.value())
            .collect::<Vec<_>>(),
        expected
    );
    r
}
fn run_case(
    name: &str,
    candidate: TemplateCandidate,
    expected: &[u64],
    root: &Path,
    explicit: bool,
    reuse: bool,
    label: &str,
) -> Value {
    assert!(
        candidate.package.definition.contribution.license.is_none(),
        "do not upgrade unknown rights in the acceptance fixture"
    );
    let mut h = harness(name, &candidate, new_project(&candidate, reuse), root);
    let before = h.object("project", "P1");
    let actual: ProjectState = serde_json::from_value(before.clone()).unwrap();
    let choices = choices_for(
        &candidate,
        if explicit {
            "EXPLICIT:source-bound"
        } else {
            "ADOPTED"
        },
        reuse,
    );
    let mut plan = prepare_template(&candidate, &actual, &choices).unwrap();
    let site = explicit.then(explicit_site);
    if let Some(site) = &site {
        fill_template_site(&mut plan, site).unwrap();
    }
    let seed = seed_for(&candidate, &plan, "P401");
    let source_project = "example-project.json".to_owned().try_into().unwrap();
    let payload = if explicit {
        plan.command_data_with_preparation(&source_project, &choices, site.as_ref(), Some(seed))
    } else {
        plan.command_data(Some(seed))
    };
    let parsed: TemplateInstantiation = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(parsed.preparation.is_some(), explicit);
    let reply = h.apply(1, json!([{"kind":"instantiate_template","data":payload}]));
    let out = Path::new(env!("CARGO_MANIFEST_DIR"));
    write(
        &out.join(format!("host-{label}.instantiate-reply.json")),
        &reply,
    );
    let commit: EditCommit = ok(reply);
    assert_eq!(commit.new_revision.0.value(), 2);
    let after = h.object("project", "P1");
    expected_metadata(&after, &plan);
    if reuse {
        let source = before["resources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "actual-existing-worker")
            .unwrap();
        assert_eq!(
            h.object("resource", "actual-existing-worker"),
            *source,
            "reuse may not change capacity/calendar/qualification/home"
        );
        assert_eq!(
            after["resources"].as_array().unwrap().len(),
            candidate.example_project.resources.len()
        );
    }
    if explicit {
        assert_eq!(after["halls"][0]["width_m"], json!(32.0));
        assert_eq!(after["halls"][0]["depth_m"], json!(20.0));
        assert!(after["halls"][0]["height_m"].is_null());
    }
    // Undo/Redo must include source/adoption metadata and the explicitly chosen seed.
    let undo: EditCommit = ok(h.apply(2, json!([{"kind":"undo"}])));
    assert_eq!(undo.new_revision.0.value(), 3);
    same_business(h.object("project", "P1"), before.clone());
    assert!(matches!(error(h.call("get_object",json!({"object_id":{"kind":"scenario","id":"P401"},"source":{"kind":"draft"},"node_index":null}))).code,ErrorCode::MissingReference));
    let redo: EditCommit = ok(h.apply(3, json!([{"kind":"redo"}])));
    assert_eq!(redo.new_revision.0.value(), 4);
    same_business(h.object("project", "P1"), after.clone());
    expected_metadata(&h.object("project", "P1"), &plan);
    let result = actual_run(&mut h, expected);
    assert_eq!(result.input_ref.project_revision.0.value(), 4);
    let first = after["devices"][0]["id"].as_str().unwrap();
    let mut edited = h.object("device", first);
    edited["title"] = json!("actual edited device");
    edited["pose"]["translation"]["x"] =
        json!(edited["pose"]["translation"]["x"].as_f64().unwrap() + 1.0);
    let edit: EditCommit = ok(h.apply(4, json!([put("device", edited.clone())])));
    assert_eq!(edit.new_revision.0.value(), 5);
    assert_eq!(h.object("device", first), edited);
    assert_eq!(
        serde_json::to_value(h.full(&result.id)).unwrap(),
        serde_json::to_value(&result).unwrap(),
        "editing current draft must not rewrite retained Run"
    );
    let current = h.object("project", "P1");
    expected_metadata(&current, &plan);
    let saved = h.tree.directory("saved-original-location");
    let handle = h.save(5, &saved);
    h.finish_one();
    assert!(matches!(
        h.task(&handle, true).task.state,
        ComputationStatus::Completed
    ));
    let moved = h.tree.directory("moved-editable-package");
    copy(&saved, &moved);
    let archive = read_project_archive(&moved).unwrap();
    let module_entry = archive
        .manifest
        .documents
        .iter()
        .find(|e| {
            e.module_ref.as_ref().is_some_and(|r| {
                r.id == candidate.package.definition.id
                    && String::from(r.version.clone())
                        == String::from(candidate.package.definition.version.clone())
            })
        })
        .unwrap();
    let persisted: Versioned<ModulePackage> =
        serde_json::from_slice(&archive.files[module_entry.file_ref.path.as_str()]).unwrap();
    assert_eq!(
        serde_json::to_value(&persisted.data.definition.contribution).unwrap(),
        serde_json::to_value(&candidate.package.definition.contribution).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&persisted.data.rights).unwrap(),
        serde_json::to_value(&candidate.package.rights).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&persisted.data.issues).unwrap(),
        serde_json::to_value(&candidate.package.issues).unwrap(),
        "save/move may not upgrade unknown rights or remove their restrictions"
    );
    let raw_path = plan.proposed_project.template_adoptions[0]
        .source_archive
        .path
        .as_str();
    assert_eq!(
        archive.files[raw_path],
        std::fs::read(
            root.join("modules/templates")
                .join(name)
                .join("candidate/source-project.design.json")
        )
        .unwrap(),
        "retain original source archive bytes after save/move"
    );
    let imported = h.import(&moved);
    h.finish_one();
    let draft = h.imported(&imported);
    let _: EditCommit = ok(h.adopt(&imported, &draft, true));
    let reopened = h.object("project", "P1");
    assert_eq!(reopened, current);
    expected_metadata(&reopened, &plan);
    assert_eq!(
        serde_json::to_value(h.full(&result.id)).unwrap(),
        serde_json::to_value(&result).unwrap(),
        "moved package must retain old Run with its original frozen revision"
    );
    let rerun = actual_run(&mut h, expected);
    assert_eq!(rerun.input_ref.project_revision.0.value(), 5);
    write(&out.join(format!("host-{label}.project.json")), &reopened);
    json!({"case":label,"status":"passed","actual_import":true,"actual_source_adoption":true,"actual_instantiate":true,"actual_single_Host_Engine":true,"acceptance_ms":expected,"source_archive_bytes_preserved_after_move":true,
 "actual_Save_Move_Open":true,"actual_device_edit_and_reopen":true,"actual_Undo_Redo":true,"adoption_metadata_matches_prepared_remap":true,"SourceRecord_and_ParameterSource_exact":true,
 "retained_Run_immutable_after_edit_and_move":true,"actual_rerun_after_moved_Open":true,"explicit_preparation":explicit,"explicit_reuse":reuse,"explicit_site":explicit,"namespace":choices.namespace,"selected_template_version":String::from(candidate.package.definition.version.clone()),"unknown_rights_preserved":true})
}
fn rejected_atomically(h: &mut Harness, data: Value, expected: ErrorCode, label: &str) {
    let before = h.snapshot();
    let mut marker = builtin_template(BuiltinTemplate::MachiningDelivery)
        .unwrap()
        .example_project
        .item_types[0]
        .clone();
    marker.id = id("rollback-marker");
    let reply=h.apply(1,json!([put("item_type",serde_json::to_value(marker).unwrap()),{"kind":"instantiate_template","data":data}]));
    write(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("rejected-{label}.json")),
        &reply,
    );
    assert_eq!(format!("{:?}", error(reply).code), format!("{expected:?}"));
    assert_eq!(
        h.snapshot(),
        before,
        "{label}: reject the whole edit batch, including earlier object and metadata"
    );
    assert!(h.host.drain_io_work().is_empty());
}
fn error_cases(root: &Path) -> Value {
    let c = builtin_template(BuiltinTemplate::ParallelElectromechanical).unwrap();
    let mut h = harness(
        "parallel-electromechanical",
        &c,
        new_project(&c, false),
        root,
    );
    let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
    let choices = choices_for(&c, "ATOMIC", false);
    let plan = prepare_template(&c, &target, &choices).unwrap();
    let path = "example-project.json".to_owned().try_into().unwrap();
    let payload = plan.command_data_with_preparation(
        &path,
        &choices,
        None,
        Some(seed_for(&c, &plan, "P401")),
    );
    let mut bad = payload.clone();
    bad["template_ref"]["version"] = json!("99.0.0");
    rejected_atomically(&mut h, bad, ErrorCode::MissingReference, "wrong-version");
    let mut bad = payload.clone();
    bad["subset"]["halls"][0]["width_m"] = json!(999);
    rejected_atomically(&mut h, bad, ErrorCode::MissingReference, "changed-subset");
    let mut bad = payload.clone();
    bad["preparation"]["source_project"] = json!("unlisted-project.json");
    rejected_atomically(&mut h, bad, ErrorCode::InvalidInput, "unlisted-source");
    let mut bad = payload.clone();
    bad["preparation"]["namespace"] = json!("forged");
    rejected_atomically(&mut h, bad, ErrorCode::InvalidInput, "namespace-mismatch");
    let mut bad = payload.clone();
    bad["preparation"]["resources"]["W-B"] = json!("missing-existing-person");
    rejected_atomically(&mut h, bad, ErrorCode::InvalidInput, "missing-reuse");
    let mut bad = payload.clone();
    bad["preparation"]["resources"]
        .as_object_mut()
        .unwrap()
        .remove("W-B");
    rejected_atomically(
        &mut h,
        bad,
        ErrorCode::InvalidInput,
        "absent-key-not-new-resource",
    );
    let mut bad = payload.clone();
    bad["proposed_objects"][0]["data"]["title"] = json!("caller-substituted source object");
    rejected_atomically(&mut h, bad, ErrorCode::InvalidInput, "changed-object");
    let mut bad = payload.clone();
    bad["explicit_new_scenario_seed"]["id"] = json!("LOCAL");
    rejected_atomically(
        &mut h,
        bad,
        ErrorCode::InvalidInput,
        "existing-scenario-seed",
    );
    let before = h.snapshot();
    let stale: ApiError = error(h.apply(
        0,
        json!([{"kind":"instantiate_template","data":payload.clone()}]),
    ));
    assert!(matches!(stale.code, ErrorCode::RevisionConflict));
    assert_eq!(h.snapshot(), before);
    let mut bad = payload;
    bad["sources"] = json!([]);
    let request = json!({"api_version":"0.1","request_id":"forged-record","project_id":"P1","scope":{"kind":"project"},"operation":"apply_commands","params":{"base_revision":1,"commands":[{"kind":"instantiate_template","data":bad}]}});
    assert!(
        serde_json::from_value::<ApiRequest>(request).is_err(),
        "caller-supplied metadata is not a new command field"
    );
    assert_eq!(h.snapshot(), before);
    json!({"case":"error-atomicity","status":"passed","actual_Host_rejections":9,"unknown_metadata_rejected_by_actual_API":true,"rollback_includes_earlier_objects_sources_and_seed":true})
}
fn unresolved_case(root: &Path) -> Value {
    let c = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let mut h = harness("purchased-assembly", &c, new_project(&c, false), root);
    let target: ProjectState = serde_json::from_value(h.object("project", "P1")).unwrap();
    let mut choices = choices_for(&c, "UNRESOLVED", false);
    choices.resources.remove(&id("W-B"));
    choices.include_example_product = false;
    let plan = prepare_template(&c, &target, &choices).unwrap();
    assert!(!plan.issues.is_empty());
    let path = "example-project.json".to_owned().try_into().unwrap();
    let _:EditCommit=ok(h.apply(1,json!([{"kind":"instantiate_template","data":plan.command_data_with_preparation(&path,&choices,None,None)}])));
    let actual = h.object("project", "P1");
    expected_metadata(&actual, &plan);
    assert!(actual["products"].as_array().unwrap().is_empty());
    assert_eq!(actual["scenario_refs"], json!(["LOCAL"]));
    assert!(
        actual["template_adoptions"][0]["resource_choices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["source_id"] == "W-B" && r["resource_id"].is_null())
    );
    json!({"case":"unresolved-role-draft","status":"passed","missing_key_is_unresolved":true,"no_implicit_supply_or_demand_or_product":true,"exact_metadata":true})
}
fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .unwrap();
    let mut rows = vec![];
    let mut failed = false;
    for (name, which, expected) in [
        (
            "machining-delivery",
            BuiltinTemplate::MachiningDelivery,
            vec![90000, 150000],
        ),
        (
            "purchased-assembly",
            BuiltinTemplate::PurchasedAssembly,
            vec![70000, 110000],
        ),
        (
            "parallel-electromechanical",
            BuiltinTemplate::ParallelElectromechanical,
            vec![160000, 200000],
        ),
        (
            "carton-factory",
            BuiltinTemplate::CartonFactory,
            vec![45000, 65000],
        ),
    ] {
        for explicit in [false, true] {
            let label = format!(
                "{name}-{}",
                if explicit {
                    "explicit-reuse-site"
                } else {
                    "legacy"
                }
            );
            match std::panic::catch_unwind(|| {
                run_case(
                    name,
                    builtin_template(which).unwrap(),
                    &expected,
                    root,
                    explicit,
                    explicit,
                    &label,
                )
            }) {
                Ok(r) => rows.push(r),
                Err(e) => {
                    failed = true;
                    rows.push(failure(&label, e));
                }
            }
        }
    }
    for (label, f) in [
        ("error-atomicity", error_cases as fn(&Path) -> Value),
        (
            "unresolved-role-draft",
            unresolved_case as fn(&Path) -> Value,
        ),
    ] {
        match std::panic::catch_unwind(|| f(root)) {
            Ok(r) => rows.push(r),
            Err(e) => {
                failed = true;
                rows.push(failure(label, e));
            }
        }
    }
    let mut versioned = builtin_template(BuiltinTemplate::PurchasedAssembly).unwrap();
    let newer: VersionId = "1.1.0".to_owned().try_into().unwrap();
    versioned.package.definition.version = newer.clone();
    versioned
        .package
        .definition
        .title
        .push_str(" / explicit new template version");
    versioned
        .subset
        .adoption
        .as_mut()
        .unwrap()
        .template_ref
        .version = newer.clone();
    for a in &mut versioned.example_project.template_adoptions {
        a.template_ref.version = newer.clone();
    }
    match std::panic::catch_unwind(|| {
        run_case(
            "purchased-assembly",
            versioned,
            &[70000, 110000],
            root,
            true,
            true,
            "new-source-version",
        )
    }) {
        Ok(r) => rows.push(r),
        Err(e) => {
            failed = true;
            rows.push(failure("new-source-version", e));
        }
    }
    write(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("host-consumer-results.json"),
        &json!(rows),
    );
    println!("{}", serde_json::to_string_pretty(&rows).unwrap());
    if failed {
        std::process::exit(1);
    }
}
fn failure(label: &str, e: Box<dyn std::any::Any + Send>) -> Value {
    json!({"case":label,"status":"failed","error":e.downcast_ref::<String>().cloned().or_else(||e.downcast_ref::<&str>().map(|s|s.to_string())).unwrap_or("panic".into())})
}
