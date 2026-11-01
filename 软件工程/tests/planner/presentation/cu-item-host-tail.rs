// Appended by cu-prepare.py to the unchanged frozen noncu_media_full_closeout
// test helpers. Only production ProjectHost readers/writers are called here.
#[test]
fn cu_item_save_move_and_new_process() {
    let input = PathBuf::from(std::env::var_os("CU_ITEM_EXPECTED").unwrap());
    let mut expected = read(&input);
    let label = std::env::var("CU_ITEM_LABEL").unwrap();
    let base = root().join(".work/cu-delivery/persistence").join(&label);
    fs::create_dir_all(&base).unwrap();
    let mut h = fresh("cu-item-save");
    let p = open(&mut h, Path::new(expected["moved"].as_str().unwrap()));
    assert_eq!(project(&mut h, &p), expected["project"]);
    let saved = base.join("saved-project");
    save(&mut h, &p, &saved);
    drop(h);
    let moved = base.join("receiver/moved-project");
    fs::create_dir_all(moved.parent().unwrap()).unwrap();
    let owned = root().join(".work/cu-delivery").canonicalize().unwrap();
    assert!(saved.canonicalize().unwrap().starts_with(&owned));
    assert!(moved.parent().unwrap().canonicalize().unwrap().starts_with(&owned));
    assert!(!moved.exists());
    fs::rename(&saved, &moved).unwrap();
    expected["moved"] = json!(moved);
    expected["label"] = json!(label);
    let expectation = base.join("expected.json");
    output(&expectation, &expected);
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "cu_item_export_fresh_host", "--nocapture"])
        .env("CU_ITEM_CHILD", &expectation).output().unwrap();
    fs::write(base.join("child.stdout.log"), &child.stdout).unwrap();
    fs::write(base.join("child.stderr.log"), &child.stderr).unwrap();
    assert!(child.status.success(), "fresh actual Host: {}", String::from_utf8_lossy(&child.stdout));
    let actual = read(&base.join("actual.json"));
    assert_eq!(actual["passed"], true);
    assert_ne!(actual["pid"], json!(std::process::id()));
    output(&base.join("save-move.json"), &json!({"passed":true,"parent_pid":std::process::id(),
        "child_pid":actual["pid"],"saved_then_physically_moved":true,"source_preserved":input,
        "moved":moved,"new_process_without_external_registration":true}));
}

#[test]
fn cu_item_export_fresh_host() {
    let Some(input) = std::env::var_os("CU_ITEM_CHILD") else { return; };
    let expected = read(Path::new(&input));
    let mut h = fresh("cu-item-independent-host");
    assert!(context(&mut h).project.is_none());
    let p = open(&mut h, Path::new(expected["moved"].as_str().unwrap()));
    assert_eq!(project(&mut h, &p), expected["project"]);
    let old:RunResult = serde_json::from_value(expected["old_run"].clone()).unwrap();
    let run:RunResult = serde_json::from_value(expected["new_run"].clone()).unwrap();
    assert_eq!(json!(full(&mut h, &p, &old.id)), expected["old_run"]);
    assert_eq!(json!(full(&mut h, &p, &run.id)), expected["new_run"]);
    let old_project = frozen(&mut h, &p, &old.id);
    let frozen_project = frozen(&mut h, &p, &run.id);
    assert_eq!(old_project, expected["old_project"]);
    assert_eq!(frozen_project, expected["new_project"]);
    let rev = context(&mut h).project.unwrap().revision;
    let mut points = vec![];
    for (time, point) in expected["points"].as_object().unwrap() {
        let t:u64 = time.parse().unwrap();
        assert_eq!(history(&mut h, &p, &old, t), *point);
        assert_eq!(history(&mut h, &p, &run, t), *point);
        let observation = h.host.read_run_observation_ui(&h.ui, &p, &ApiScope::Project, &rev, &run.id,
            &SimTimeMs(SafeUint::try_from(t).unwrap())).unwrap();
        assert!(observation.next_index.is_none());
        let mut actual:Vec<_> = observation.objects.iter().filter(|x|matches!(x,DomainObject::Item(_)|DomainObject::Job(_))).map(|x|json!(x).to_string()).collect();
        let mut known:Vec<_> = point["objects"].as_array().unwrap().iter().map(Value::to_string).collect();
        actual.sort();known.sort();assert_eq!(actual,known);
        points.push(json!(observation));
    }
    let mut queue:Vec<Value> = frozen_project["module_refs"].as_array().unwrap().clone();
    let mut modules = BTreeMap::new();
    let mut asset_count = 0;
    while let Some(reference) = queue.pop() {
        if modules.contains_key(&reference.to_string()) {continue;}
        let package = module(&mut h, &p, &reference, Some(&run.id));
        queue.extend(package["dependencies"].as_array().unwrap().iter().cloned());
        let source:ObjectSource = serde_json::from_value(run_source(&run.id,&reference)).unwrap();
        for a in package["assets"].as_array().unwrap() {
            let asset:AssetRef = serde_json::from_value(a.clone()).unwrap();
            let bytes = h.host.read_media_asset(&h.ui,&p,&ApiScope::Project,&rev,&source,&asset).unwrap();
            let file = root().join(".work/cu-delivery/host-media")
                .join(reference["id"].as_str().unwrap()).join(reference["version"].as_str().unwrap()).join(asset.path.as_str());
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            if file.exists() {assert_eq!(fs::read(&file).unwrap(),bytes);} else {fs::write(&file,&bytes).unwrap();}
            asset_count += 1;
        }
        modules.insert(reference.to_string(),package);
    }
    assert_eq!(project(&mut h,&p),expected["project"]);
    let label=expected["label"].as_str().unwrap();
    let value=json!({"label":label,"pid":std::process::id(),"passed":true,"current_project":expected["project"],
        "project":frozen_project,"old_project":old_project,"run":run,"old_run":old,"points":points,
        "modules":modules.values().collect::<Vec<_>>(),"scope_revision":rev,"project_id":p,
        "assets_read":asset_count,"external_registration":false});
    output(&root().join(".work/cu-delivery/host").join(format!("{label}.json")),&value);
    output(&Path::new(&input).parent().unwrap().join("actual.json"),&json!({"passed":true,
        "pid":std::process::id(),"actual_observation_count":points.len(),"assets_read":asset_count,
        "modules":modules.len(),"whole_task_done":false,"ordinary_Native_acceptance":false}));
}
