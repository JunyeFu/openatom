// Included in the existing H01 test module. Real Host/Engine and files, with
// explicitly finite typed C01/publication acknowledgements; no HTTPS/device proof.
fn archive_ok(value:Value)->Value {assert_eq!(value["outcome"]["status"],"ok","{value}");value["outcome"]["data"].clone()}
fn archive_root(label:&str)->PathBuf {
    let path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.work/v2-implementation-20261026/execution-archive/evidence")
        .join(format!("{label}-{}-{}",std::process::id(),DIRECTORIES.fetch_add(1,Ordering::Relaxed)));
    fs::create_dir_all(&path).unwrap();path
}
fn archive_open(host:&mut ProjectHost,ui:&ObjectId,path:&Path) {
    let selected=host.select_directory(ui,FilePurpose::ImportPackage,path.to_owned()).unwrap();
    let task=archive_ok(invoke(host,ui,"archive-open","import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),true));
    let mut work=host.drain_io_work();assert_eq!(work.len(),1);host.finish_io(work.pop().unwrap().run());
    archive_ok(invoke(host,ui,"archive-adopt","apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}),true));
}
fn archive_r4()->(ProjectHost,ObjectId,ExecutionPreparationJob,ExecutionGatewayLease,PathBuf) {
    let ui=id("archive-original-ui");let mut host=ProjectHost::new(id("archive-original-host"),ui.clone());
    let source=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../resources/v2-demo/geometry-v4/project");archive_open(&mut host,&ui,&source);
    assert_eq!(host.project.as_ref().unwrap().revision.0.value(),4);assert_eq!(host.runs.len(),3);
    let project=host.project.as_ref().unwrap().id.clone();
    let candidates=serde_json::from_slice(&fs::read(source.parent().unwrap().join("candidate-bindings.json")).unwrap()).unwrap();
    host.install_cloud_read_connection(&ui,CloudReadConnection::local_loopback(id("archive-private-connection"),project.clone(),"archive-cloud-project".to_owned().try_into().unwrap(),"http://127.0.0.1:9".into(),"archive-never-export-reader-secret".into(),candidates).unwrap()).unwrap();
    let (sender,receiver)=mpsc::channel();host.install_execution_preparation_runtime(&ui,project,ExecutionPreparationRuntime::new(sender,id("archive-private-connection"))).unwrap();
    let task=start(&mut host,&ui,"archive-original-H01");let job=receiver.try_recv().unwrap();host.execution_preparation_started(job.lease()).unwrap();
    job.prepare_engine().unwrap();let evidence=archive_root("saved-source");job.persist_frozen_inputs(&evidence.join("original-private-freeze")).unwrap();
    host.execution_preparation_external_started(job.lease()).unwrap();let gateway=host.register_execution_preparation_core(job.lease(),cloud_ack(&job)).unwrap();
    for binding in &job.frozen.candidate_bindings {host.execution_gateway_register_boot(&gateway,&binding.device_id,format!("archive-boot-{}",binding.device_id.as_str()).try_into().unwrap(),std::time::Instant::now()).unwrap();}
    acknowledge(&mut host,&gateway);host.finish_execution_preparation(job.lease(),&gateway).unwrap();
    assert_eq!(task_state(&mut host,&ui,&task.id)["task"]["state"],"completed");
    let revision=host.executions[&gateway.identity.session_id].core.revision;
    archive_ok(invoke(&mut host,&ui,"archive-run","control_execution_session",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":gateway.identity,"expected_execution_revision":revision,"action":{"kind":"run"}}),false));
    assert!(host.execution_gateway_snapshot(&gateway).unwrap().coverage.clock_started);
    (host,ui,job,gateway,evidence)
}
fn archive_queue_save(host:&mut ProjectHost,ui:&ObjectId,directory:&Path)->(TaskId,ProjectIoWork) {
    fs::create_dir_all(directory).unwrap();let selected=host.select_directory(ui,FilePurpose::SaveProject,directory.to_owned()).unwrap();
    let revision=host.project.as_ref().unwrap().revision.clone();
    let task=archive_ok(invoke(host,ui,"archive-save","save_project",json!({"revision":revision,"selected_directory_ref":selected}),false));
    let mut work=host.drain_io_work();assert_eq!(work.len(),1);(serde_json::from_value(task["id"].clone()).unwrap(),work.pop().unwrap())
}
#[test]
fn execution_archive_real_h01_save_prefix_generation_and_new_os_readonly_reopen() {
    let (mut host,ui,job,gateway,evidence)=archive_r4();let original=host.backing.clone().unwrap();let saved=evidence.join("project");
    let (first,work)=archive_queue_save(&mut host,&ui,&saved);
    std::thread::sleep(std::time::Duration::from_millis(5));host.execution_gateway_tick(&gateway,std::time::Instant::now()).unwrap();
    host.finish_io(work.run());let state=task_state(&mut host,&ui,&first);assert_eq!(state["task"]["state"],"completed","{state}");assert_eq!(state["result"]["data"]["still_dirty"],true);assert!(host.dirty);
    let first_archive=host.backing.clone().unwrap();let original_source_files:Vec<_>=first_archive.manifest.documents.iter().filter(|e|matches!(e.file_ref.kind,DocumentKind::ExecutionArchive)).map(|e|(e.file_ref.path.as_str().to_owned(),first_archive.files[e.file_ref.path.as_str()].clone())).collect();assert_eq!(original_source_files.len(),1);
    let (second,work)=archive_queue_save(&mut host,&ui,&saved);host.finish_io(work.run());let state=task_state(&mut host,&ui,&second);assert_eq!(state["task"]["state"],"completed","{state}");assert_eq!(state["result"]["data"]["still_dirty"],false);assert!(!host.dirty);
    let archive=read_project_archive(&saved).unwrap();assert_eq!(archive.execution_sources.len(),1);assert_eq!(archive.run_results().unwrap().len(),3);
    for (path,bytes) in &original_source_files {assert_eq!(&archive.files[path],bytes,"old immutable source document replaced");}
    for run in original.run_results().unwrap() {for file in [&run.input_ref.frozen_project_file,&run.input_ref.frozen_scenario_file] {assert_eq!(archive.files[file.path.as_str()],original.files[file.path.as_str()]);}}
    let before_sources=archive.manifest.documents.iter().filter(|e|matches!(e.file_ref.kind,DocumentKind::ExecutionArchive)).count();
    let (third,work)=archive_queue_save(&mut host,&ui,&saved);host.finish_io(work.run());assert_eq!(task_state(&mut host,&ui,&third)["task"]["state"],"completed");
    let final_archive=read_project_archive(&saved).unwrap();assert_eq!(before_sources,final_archive.manifest.documents.iter().filter(|e|matches!(e.file_ref.kind,DocumentKind::ExecutionArchive)).count(),"unchanged source was copied again");
    for bytes in final_archive.files.values(){assert!(!String::from_utf8_lossy(bytes).contains("archive-never-export-reader-secret"));}
    let snapshot=host.execution_gateway_snapshot(&gateway).unwrap();let meta=json!({"project_directory":saved,"evidence":evidence,"parent_pid":std::process::id(),"identity":gateway.identity,"snapshot":snapshot,"source_frozen_directory":evidence.join("original-private-freeze"),"asset_binding":job.frozen.asset_bindings[0]});
    let metadata=evidence.join("child-input.json");fs::write(&metadata,serde_json::to_vec_pretty(&meta).unwrap()).unwrap();
    let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","project_host::host_execution_preparation::tests::execution_archive_child_process","--ignored","--nocapture"]).env("SUPERFACTORY_ARCHIVE_REOPEN_CHILD",&metadata).output().unwrap();
    fs::write(evidence.join("child.stdout.log"),&output.stdout).unwrap();fs::write(evidence.join("child.stderr.log"),&output.stderr).unwrap();assert!(output.status.success(),"{}\n{}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
    let child:Value=serde_json::from_slice(&fs::read(evidence.join("child-result.json")).unwrap()).unwrap();assert_ne!(child["pid"],meta["parent_pid"]);assert_eq!(child["native_cloud_or_physical_execution"],false);
    fs::write(evidence.join("parent-result.json"),serde_json::to_vec_pretty(&json!({"status":"actual_Host_save_new_OS_reopen_passed","parent_pid":std::process::id(),"child_pid":child["pid"],"project_directory":saved,"source":snapshot.identity,"latest_R":snapshot.execution_revision,"latest_F":snapshot.observation_frontier,"latest_T":snapshot.coverage.observed_until_host_ms,"saved_runs":3,"old_run_and_source_bytes_preserved":true,"save_race_stays_dirty":true,"secret_excluded":true,"native_cloud_or_physical_execution":false})).unwrap()).unwrap();
    println!("ARCHIVE_ACTUAL_PARENT_RESULT={}",evidence.join("parent-result.json").display());
}
#[test]
#[ignore="spawned by real archive parent with an existing saved project and original bytes"]
fn execution_archive_child_process() {
    let input=std::env::var_os("SUPERFACTORY_ARCHIVE_REOPEN_CHILD").expect("actual parent metadata required");let meta:Value=serde_json::from_slice(&fs::read(input).unwrap()).unwrap();assert_ne!(json!(std::process::id()),meta["parent_pid"]);
    let saved=PathBuf::from(meta["project_directory"].as_str().unwrap());let evidence=PathBuf::from(meta["evidence"].as_str().unwrap());let ui=id("archive-reopened-ui");let mut host=ProjectHost::new(id(&format!("archive-reopened-{}",std::process::id())),ui.clone());archive_open(&mut host,&ui,&saved);
    assert!(host.executions.is_empty());assert!(host.execution_blockers().is_empty());assert!(!host.dirty);assert_eq!(host.runs.len(),3);
    let identity:ExecutionIdentity=serde_json::from_value(meta["identity"].clone()).unwrap();let expected:ExecutionSnapshot=serde_json::from_value(meta["snapshot"].clone()).unwrap();
    let catalog=archive_ok(invoke(&mut host,&ui,"archive-catalog","read_execution_view",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"catalog"}}),false));assert_eq!(catalog["data"]["entries"].as_array().unwrap().len(),1);assert_eq!(catalog["data"]["entries"][0]["active_owner"],false);
    let params=json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"selection":{"kind":"revision","execution_revision":expected.execution_revision}});
    let envelope=archive_ok(invoke(&mut host,&ui,"archive-latest-exact","get_execution_snapshot",params,false));assert_eq!(envelope["snapshot"],meta["snapshot"]);assert_eq!(envelope["access"]["ownership"],"other_host");assert_eq!(envelope["access"]["allowed_session_controls"],json!([]));assert_eq!(envelope["delivery"]["relation"],"unknown");
    let r0=archive_ok(invoke(&mut host,&ui,"archive-r0","get_execution_snapshot",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"selection":{"kind":"revision","execution_revision":0}}),false));assert_eq!(r0["snapshot"]["coverage"]["clock_started"],false);
    let frozen=archive_ok(invoke(&mut host,&ui,"archive-inputs","read_execution_view",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"inputs","identity":identity}}),false));assert_eq!(frozen["data"]["project_input_ref"],envelope["snapshot"]["project_input_ref"]);
    for (kind,extra) in [("observations",json!({"at_host_ms":expected.coverage.observed_until_host_ms})),("history",json!({"host_from_ms":0,"host_until_ms":expected.coverage.observed_until_host_ms}))] {
        let mut cursor:Value=Value::Null;let mut count=0;
        loop {let mut query=json!({"contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"execution_revision":expected.execution_revision,"objects":[],"page":{"limit":3,"cursor":cursor}});for (k,v) in extra.as_object().unwrap(){query[k]=v.clone();}
            let page=archive_ok(invoke(&mut host,&ui,"archive-pages","read_execution_view",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":kind,"query":query}}),false));assert_eq!(page["data"]["observation_frontier"],json!(expected.observation_frontier));count+=page["data"][if kind=="history"{"records"}else{"objects"}].as_array().unwrap().len();cursor=page["data"]["next_cursor"].clone();if cursor.is_null(){break;}}
        if kind=="history"{assert_eq!(count as u64,expected.observation_frontier.value());}else{assert!(count>=6);}
    }
    let binding:ExecutionAssetBinding=serde_json::from_value(meta["asset_binding"].clone()).unwrap();let request=ExecutionMediaAssetReadRequest {project_id:identity.project_id.clone(),scope_revision:host.project.as_ref().unwrap().revision.clone(),identity:identity.clone(),execution_revision:expected.execution_revision.clone(),module_ref:binding.module_ref.clone(),asset:binding.asset.clone()};
    let original=fs::read(PathBuf::from(meta["source_frozen_directory"].as_str().unwrap()).join(binding.archive_file.path.as_str())).unwrap();assert_eq!(host.read_execution_media_asset(&ui,&request).unwrap(),original);
    let modules=std::mem::take(&mut host.modules);assert_eq!(host.read_execution_media_asset(&ui,&request).unwrap(),original);host.modules=modules;
    let mut bad=request.clone();bad.execution_revision=100000u64.try_into().unwrap();assert!(host.read_execution_media_asset(&ui,&bad).is_err());bad=request.clone();bad.identity.execution_epoch_id="missing-original-epoch".to_owned().try_into().unwrap();assert!(host.read_execution_media_asset(&ui,&bad).is_err());bad=request.clone();bad.module_ref.version="99.0.0".to_owned().try_into().unwrap();assert!(host.read_execution_media_asset(&ui,&bad).is_err());assert!(host.read_execution_media_asset(&id("unregistered-native"),&request).is_err());
    let denied=invoke(&mut host,&ui,"archive-cannot-resume","control_execution_session",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"expected_execution_revision":expected.execution_revision,"action":{"kind":"run"}}),false);assert_eq!(denied["outcome"]["status"],"error");assert!(host.executions.is_empty());assert!(!host.dirty);
    let same_ui=id("archive-same-string-ui");let mut same=ProjectHost::new(identity.host_context_session_id.clone(),same_ui.clone());archive_open(&mut same,&same_ui,&saved);let actual=archive_ok(invoke(&mut same,&same_ui,"same-string-has-no-engine","get_execution_snapshot",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"selection":{"kind":"latest"}}),false));assert_eq!(actual["access"]["ownership"],"unverified");assert!(same.executions.is_empty());assert!(same.execution_blockers().is_empty());
    let fonts=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/architecture-proof/assets");host.set_evaluation_resources(sf_reporting::EvaluationResources {cjk_font:Some(fs::read(fonts.join("SourceHanSansCN-Regular.otf")).unwrap()),font_license:Some(fs::read(fonts.join("LICENSE.SourceHanSans.txt")).unwrap()),files:BTreeMap::new()});
    let report_dir=evidence.join("reopened-report");fs::create_dir_all(&report_dir).unwrap();let destination=host.select_directory(&ui,FilePurpose::ExportExecutionReport,report_dir.clone()).unwrap();let source=json!({"read_origin":"host","identity":identity,"execution_revision":expected.execution_revision});
    let predicted=host.runs.values().find(|r|r.result.input_ref.project_revision.0.value()==4).unwrap().result.clone();let predicted_ref=&predicted.input_ref;
    let task=archive_ok(invoke(&mut host,&ui,"archive-report-original","export_execution_report",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"revision":4,"sources":[source.clone()],"formats":["pdf","html","csv","json"],"selected_destination_ref":destination,"prediction":{"prediction":{"run_id":predicted.id,"input_ref":{"project_id":predicted_ref.project_id,"project_revision":predicted_ref.project_revision,"scenario_id":predicted_ref.scenario_id,"scenario_revision":predicted_ref.scenario_revision}},"execution":source}}),false));
    let mut work=host.drain_io_work();assert_eq!(work.len(),1);host.finish_io(work.pop().unwrap().run());let task_id=serde_json::from_value(task["id"].clone()).unwrap();let state=task_state(&mut host,&ui,&task_id);assert_eq!(state["task"]["state"],"completed","{state}");let report=&state["result"]["data"];let files=report["files"].as_array().unwrap();for file in files{assert!(report_dir.join(file["path"].as_str().unwrap()).is_file());}
    for suffix in [".pdf",".html",".csv",".json"]{assert!(files.iter().any(|f|f["path"].as_str().unwrap().ends_with(suffix)));}
    let project=host.project.as_ref().unwrap().clone();
    archive_ok(invoke(&mut host,&ui,"edit-current-draft-only","apply_commands",json!({"base_revision":project.revision,"commands":[{"kind":"set_project_settings","data":{"title":"Draft edited after readonly execution reopen","time_reference":project.time_reference,"generation_context":project.generation_context,"sources":project.sources}}]}),false));
    assert_eq!(host.project.as_ref().unwrap().revision.0.value(),5);
    let old_inputs=archive_ok(invoke(&mut host,&ui,"old-execution-inputs-stay-r4","read_execution_view",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"inputs","identity":identity}}),false));
    assert_eq!(old_inputs["data"]["project"]["revision"],4);assert_ne!(old_inputs["data"]["project"]["title"],json!("Draft edited after readonly execution reopen"));
    let again=evidence.join("reopened-saved");let (task,work)=archive_queue_save(&mut host,&ui,&again);host.finish_io(work.run());assert_eq!(task_state(&mut host,&ui,&task)["task"]["state"],"completed");assert_eq!(read_project_archive(&again).unwrap().run_results().unwrap().len(),3);assert!(!host.dirty);
    let last_ui=id("archive-reopened-again-ui");let mut last=ProjectHost::new(id("archive-reopened-again-host"),last_ui.clone());archive_open(&mut last,&last_ui,&again);assert_eq!(last.project.as_ref().unwrap().revision.0.value(),5);assert_eq!(last.runs.len(),3);assert!(last.executions.is_empty());
    let kept=archive_ok(invoke(&mut last,&last_ui,"old-r4-after-resave-r5","get_execution_snapshot",json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"identity":identity,"selection":{"kind":"revision","execution_revision":expected.execution_revision}}),false));assert_eq!(kept["snapshot"],meta["snapshot"]);
    fs::write(evidence.join("child-result.json"),serde_json::to_vec_pretty(&json!({"pid":std::process::id(),"source":identity,"snapshot":envelope,"archive_catalog":catalog,"saved_run_count":3,"report_id":report["report_id"],"report_directory":report_dir,"report_files":files.len(),"same_host_string_is_unverified":true,"no_engine_restored":host.executions.is_empty(),"native_cloud_or_physical_execution":false})).unwrap()).unwrap();
}
#[test]
fn execution_archive_corruption_has_no_latest_draft_or_cloud_fallback() {
    let (mut host,ui,_job,_gateway,evidence)=archive_r4();let saved=evidence.join("project");let (task,work)=archive_queue_save(&mut host,&ui,&saved);host.finish_io(work.run());assert_eq!(task_state(&mut host,&ui,&task)["task"]["state"],"completed");let archive=read_project_archive(&saved).unwrap();let source=archive.manifest.documents.iter().find(|e|matches!(e.file_ref.kind,DocumentKind::ExecutionArchive)).unwrap().file_ref.path.as_str().to_owned();
    for variant in ["wrong_epoch","wrong_R","wrong_version","missing_file","truncated_bytes"] {
        let mut corrupt=archive.clone();let mut value:Value=serde_json::from_slice(&corrupt.files[&source]).unwrap();
        match variant {"wrong_epoch"=>value["data"]["frozen_inputs"]["identity"]["execution_epoch_id"]=json!("wrong-epoch"),"wrong_R"=>value["data"]["snapshots"][0]["execution_revision"]=json!(99999),"wrong_version"=>value["schema_version"]=json!("2.0"),"missing_file"=>{let f=value["data"]["files"][0]["stored_file"]["path"].as_str().unwrap();corrupt.files.remove(f);},"truncated_bytes"=>{let f=value["data"]["files"][0]["stored_file"]["path"].as_str().unwrap();corrupt.files.get_mut(f).unwrap().pop();},_=>unreachable!()}
        corrupt.files.insert(source.clone(),serde_json::to_vec_pretty(&value).unwrap());let path=evidence.join(variant);fs::create_dir_all(&path).unwrap();for (relative,bytes) in &corrupt.files {let file=path.join(relative);fs::create_dir_all(file.parent().unwrap()).unwrap();fs::write(file,bytes).unwrap();}fs::write(path.join("project.json"),encode_document(&corrupt.manifest)).unwrap();assert!(read_project_archive(&path).is_err(),"{variant} incorrectly loaded");
    }
}

#[test]
fn execution_archive_project_export_keeps_closed_source_and_module_export_excludes_it() {
    let (mut host,ui,_job,_gateway,evidence)=archive_r4();
    let saved=evidence.join("project");let (task,work)=archive_queue_save(&mut host,&ui,&saved);host.finish_io(work.run());assert_eq!(task_state(&mut host,&ui,&task)["task"]["state"],"completed");
    let rights:Vec<_>=host.backing.as_ref().unwrap().manifest.assets.iter().map(|right|right.asset_ref.clone()).collect();
    for asset in rights {host.confirm_asset_redistribution(&ui,&asset).unwrap();}
    let complete=json!({"purpose":"private_project","scenarios":host.scenarios.keys().collect::<Vec<_>>(),"run_ids":host.runs.keys().collect::<Vec<_>>(),"artifact_ids":[],"module_refs":[],"include_redistributable_assets":true});
    let one_module=host.project.as_ref().unwrap().module_refs[0].clone();
    for (label,selection,expected_sources) in [("complete-project",complete,1),("standalone-module",json!({"purpose":"private_module","scenarios":[],"run_ids":[],"artifact_ids":[],"module_refs":[one_module],"include_redistributable_assets":true}),0)] {
        let destination=evidence.join(label);fs::create_dir_all(&destination).unwrap();let selected=host.select_directory(&ui,FilePurpose::ExportPackage,destination.clone()).unwrap();
        let task=archive_ok(invoke(&mut host,&ui,label,"export_package",json!({"revision":4,"selected_destination_ref":selected,"package_scope":selection}),false));let mut work=host.drain_io_work();assert_eq!(work.len(),1);host.finish_io(work.pop().unwrap().run());let state=task_state(&mut host,&ui,&serde_json::from_value(task["id"].clone()).unwrap());assert_eq!(state["task"]["state"],"completed","{state}");
        let reopened=read_project_archive(&destination).unwrap();assert_eq!(reopened.execution_sources.len(),expected_sources);
        if expected_sources==1 {assert_eq!(reopened.run_results().unwrap().len(),3);assert!(reopened.execution_sources.values().all(|source|source.reader.is_some()));}
        else {assert!(!reopened.manifest.documents.iter().any(|entry|matches!(entry.file_ref.kind,DocumentKind::ExecutionArchive)));assert!(!reopened.files.keys().any(|path|path.contains("/execution-")));}
    }
    fs::write(evidence.join("export-result.json"),serde_json::to_vec_pretty(&json!({"status":"actual_project_and_module_export_scope_passed","project_export_contains_readonly_source":true,"module_export_has_no_execution_data":true,"native_cloud_or_physical_execution":false})).unwrap()).unwrap();
}
