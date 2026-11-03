//! H10/H11 use the original Host Task registry and managed cloud reader. Workers
//! freeze/read/encode/write, then the Host alone admits the no-replace commit.
use super::*;
use crate::project_store::StagedExportDirectory;
use sf_domain::cloud_execution::*;
use sf_reporting::{
    CloudDataExportInput, ExecutionReportInput, ExecutionReportInputSession, PredictionInput,
};
use std::io::Write;
use std::sync::atomic::AtomicU64;

static EXPORT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub(super) struct OriginalExecutionRequest {
    payload: Value,
    operation: OperationName,
    task_id: TaskId,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread::JoinHandle;

    fn id(s: &str) -> ObjectId {
        s.to_owned().try_into().unwrap()
    }
    fn call(
        host: &mut ProjectHost,
        caller: &ObjectId,
        request: &str,
        operation: &str,
        params: Value,
    ) -> Value {
        let workspace = host.project.is_none();
        host.dispatch(caller,serde_json::from_value(json!({"api_version":"0.1","request_id":request,
            "project_id":host.project.as_ref().map(|p|p.id.clone()),"scope":{"kind":if workspace{"workspace"}else{"project"}},
            "operation":operation,"params":params})).unwrap())
    }
    fn ok(response: Value) -> Value {
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        response["outcome"]["data"].clone()
    }
    fn run_one(host: &mut ProjectHost) {
        let mut work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        host.finish_io(work.pop().unwrap().run());
    }
    fn directory(label: &str) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.work/v2-implementation-20261026/host-exports/evidence")
            .join(format!("{label}-{}", export_id().as_str()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }
    fn fixture() -> (
        ProjectHost,
        ObjectId,
        ExecutionGatewayLease,
        PredictionRunReference,
    ) {
        let ui = id("export-test-ui");
        let mut host = ProjectHost::new(id("export-test-real-host"), ui.clone());
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources/v2-demo/geometry-v4/project");
        let selected = host
            .select_directory(&ui, FilePurpose::ImportPackage, source)
            .unwrap();
        let task = ok(call(
            &mut host,
            &ui,
            "open-r4",
            "import_package",
            json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
        ));
        run_one(&mut host);
        ok(call(
            &mut host,
            &ui,
            "adopt-r4",
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task["id"],"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}),
        ));
        let project = host.project.as_ref().unwrap().clone();
        assert_eq!(project.revision.0.value(), 4);
        assert_eq!(host.runs.len(), 3);
        let scenario = host
            .scenarios
            .values()
            .find(|s| s.revision.0.value() == 4)
            .unwrap()
            .clone();
        let run = host
            .runs
            .values()
            .find(|r| r.result.input_ref.project_revision.0.value() == 4)
            .unwrap()
            .result
            .clone();
        let mut required: Vec<_> = project.module_refs.clone();
        required.extend(
            project
                .devices
                .iter()
                .flat_map(|d| d.route_operation_refs.clone()),
        );
        let mut modules = vec![];
        let mut seen = BTreeSet::new();
        while let Some(reference) = required.pop() {
            if seen.insert(module_key(&reference)) {
                let package = host.modules[&module_key(&reference)].clone();
                required.extend(package.dependencies.clone());
                modules.push(package);
            }
        }
        let definitions: Vec<_> = modules.iter().map(|m| m.definition.clone()).collect();
        let bindings: Vec<CandidateBinding> = serde_json::from_slice(
            &std::fs::read(
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../resources/v2-demo/geometry-v4/candidate-bindings.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let frozen:ExecutionFrozenInputs=typed(json!({"document_kind":"execution_frozen_inputs","contract_version":EXECUTION_CONTRACT_VERSION,
            "identity":{"session_id":"export-original-session","execution_epoch_id":"export-original-epoch","project_id":project.id,"host_context_session_id":host.host_instance_id},
            "project_input_ref":run.input_ref,"project":project,"scenario":scenario,"modules":modules,"execution_definitions":definitions,
            "scope":scenario.scope,"candidate_bindings":bindings,"asset_bindings":[]})).unwrap();
        let policy=typed(json!({"clock_mode":"host_monotonic","report_cutoff_ms":120000,"stop_new_dispatch_on_cutoff":true,
            "finish_policy":"drain_admitted_then_finish","target_quantity":{"value":{"coefficient":6,"decimal_places":0},"unit":"piece"},
            "source_prediction_window":{"start_ms":0,"end_ms":scenario.window.end_ms},"observation_max_age_ms":2000})).unwrap();
        let prediction = PredictionRunReference {
            run_id: run.id.clone(),
            input_ref: ReportRunInputRef {
                project_id: project.id.clone(),
                project_revision: project.revision,
                scenario_id: scenario.id,
                scenario_revision: scenario.revision,
            },
        };
        let lease = host
            .register_native_execution(
                &ui,
                frozen,
                policy,
                "export-local-dataset".to_owned().try_into().unwrap(),
                id("export-private-connection"),
            )
            .unwrap();
        (host, ui, lease, prediction)
    }
    fn report_params(
        host: &mut ProjectHost,
        caller: &ObjectId,
        lease: &ExecutionGatewayLease,
        prediction: Option<PredictionRunReference>,
        path: &std::path::Path,
        formats: Value,
    ) -> Value {
        let selected = host
            .select_directory(caller, FilePurpose::ExportExecutionReport, path.to_owned())
            .unwrap();
        let source = json!({"read_origin":"host","identity":lease.identity,"execution_revision":host.executions[&lease.identity.session_id].core.revision});
        json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"revision":host.project.as_ref().unwrap().revision,"sources":[source],
            "formats":formats,"selected_destination_ref":selected,"prediction":prediction.map(|p|json!({"prediction":p,"execution":source}))})
    }
    fn state(host: &mut ProjectHost, ui: &ObjectId, task: &Value) -> Value {
        ok(call(
            host,
            ui,
            "get-export-task",
            "get_task",
            json!({"task_id":task["id"]}),
        ))
    }
    fn fonts() -> sf_reporting::EvaluationResources {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/architecture-proof/assets");
        sf_reporting::EvaluationResources {
            cjk_font: Some(std::fs::read(root.join("SourceHanSansCN-Regular.otf")).unwrap()),
            font_license: Some(std::fs::read(root.join("LICENSE.SourceHanSans.txt")).unwrap()),
            files: BTreeMap::new(),
        }
    }
    #[test]
    fn h10_real_r4_host_prediction_four_formats_and_original_request_recovery() {
        let (mut host, ui, lease, prediction) = fixture();
        let path = directory("h10-four-formats");
        host.set_evaluation_resources(fonts());
        let params = report_params(
            &mut host,
            &ui,
            &lease,
            Some(prediction.clone()),
            &path,
            json!(["pdf", "html", "json", "csv"]),
        );
        let task = ok(call(
            &mut host,
            &ui,
            "same-original-report",
            "export_execution_report",
            params.clone(),
        ));
        let repeated = ok(call(
            &mut host,
            &ui,
            "same-original-report",
            "export_execution_report",
            params.clone(),
        ));
        assert_eq!(task["id"], repeated["id"]);
        let lookup = ok(call(
            &mut host,
            &ui,
            "lookup-original-report",
            "read_execution_view",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"request","originating_request_id":"same-original-report"}}),
        ));
        assert_eq!(lookup["data"]["operation"], "export_execution_report");
        assert_eq!(lookup["data"]["task"]["id"], task["id"]);
        // Current draft changes never replace the original frozen report inputs.
        host.project.as_mut().unwrap().title =
            "later draft title must not become frozen input".into();
        host.stop_native_execution_preparation(&lease, id("future-stop-after-report-pin"))
            .unwrap();
        assert!(host.executions[&lease.identity.session_id].core.revision > 0);
        run_one(&mut host);
        let s = state(&mut host, &ui, &task);
        assert_eq!(s["task"]["state"], "completed", "{s}");
        let strict: TaskState = serde_json::from_value(s.clone()).unwrap();
        assert_eq!(
            strict.originating_request_id,
            Some(id("same-original-report"))
        );
        let result = &s["result"]["data"];
        let report_path = path.join(result["report_file"]["path"].as_str().unwrap());
        let report: ExecutionReport =
            serde_json::from_slice(&std::fs::read(&report_path).unwrap()).unwrap();
        assert_eq!(report.export_project_revision.value(), 4);
        assert_eq!(report.sessions[0].snapshot.execution_revision.value(), 0);
        assert_eq!(
            report
                .prediction_comparison
                .as_ref()
                .unwrap()
                .prediction
                .source
                .run_id,
            prediction.run_id
        );
        assert_ne!(
            report.sessions[0].project_title,
            host.project.as_ref().unwrap().title
        );
        for f in result["files"].as_array().unwrap() {
            assert!(path.join(f["path"].as_str().unwrap()).is_file());
        }
        let parent = report_path.parent().unwrap();
        assert!(
            std::fs::read(parent.join("report.pdf"))
                .unwrap()
                .starts_with(b"%PDF")
        );
        assert!(
            std::fs::read_to_string(parent.join("report.html"))
                .unwrap()
                .contains("预测")
        );
        assert_eq!(std::fs::read_dir(parent.join("csv")).unwrap().count(), 17);
        let mut changed = params.clone();
        changed["prediction"] = Value::Null;
        assert_eq!(
            call(
                &mut host,
                &ui,
                "same-original-report",
                "export_execution_report",
                changed
            )["outcome"]["data"]["code"],
            "InvalidInput"
        );
        let cross = call(
            &mut host,
            &ui,
            "same-original-report",
            "read_cloud_production",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"configuration","base_revision":4,"scenario_id":prediction.input_ref.scenario_id,"scenario_revision":4}}),
        );
        assert_eq!(cross["outcome"]["data"]["code"], "InvalidInput");
        let core = &host.executions[&lease.identity.session_id].core;
        let start = json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"base_revision":4,
            "scenario_id":core.frozen.scenario.id,"scenario_revision":4,"candidate_bindings":core.frozen.candidate_bindings,"policy":core.policy});
        assert_eq!(
            call(
                &mut host,
                &ui,
                "same-original-report",
                "start_execution_session",
                start
            )["outcome"]["data"]["code"],
            "InvalidInput"
        );
        let data = data_params(&mut host, &ui, &path, 0, vec![]);
        assert_eq!(
            call(
                &mut host,
                &ui,
                "same-original-report",
                "export_cloud_data",
                data
            )["outcome"]["data"]["code"],
            "InvalidInput"
        );
        std::fs::write(
            path.join("task-result.json"),
            serde_json::to_vec_pretty(&s).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn h10_rejects_stale_revision_wrong_purpose_owner_and_missing_font_without_commit() {
        let (mut host, ui, lease, prediction) = fixture();
        let path = directory("h10-rejections");
        let params = report_params(
            &mut host,
            &ui,
            &lease,
            Some(prediction),
            &path,
            json!(["pdf"]),
        );
        let mut stale = params.clone();
        stale["revision"] = json!(3);
        assert_eq!(
            call(&mut host, &ui, "stale", "export_execution_report", stale)["outcome"]["status"],
            "error"
        );
        let old = host
            .select_directory(&ui, FilePurpose::ExportEvaluation, path.clone())
            .unwrap();
        let mut wrong = params.clone();
        wrong["selected_destination_ref"]["selection_id"] = json!(old.selection_id);
        assert_eq!(
            call(
                &mut host,
                &ui,
                "wrong-purpose",
                "export_execution_report",
                wrong
            )["outcome"]["data"]["code"],
            "PermissionDenied"
        );
        let mut outside = host.sessions[&ui].clone();
        outside.origin = CallerOrigin::ExternalAgent;
        outside.session_id = id("other-export-owner");
        outside.granted_project = host.project.as_ref().map(|p| p.id.clone());
        outside.selected_file_tokens.clear();
        host.register_external_session(outside.clone()).unwrap();
        assert_eq!(
            call(
                &mut host,
                &outside.session_id,
                "wrong-owner",
                "export_execution_report",
                params.clone()
            )["outcome"]["data"]["code"],
            "PermissionDenied"
        );
        let task = ok(call(
            &mut host,
            &ui,
            "missing-font",
            "export_execution_report",
            params,
        ));
        run_one(&mut host);
        let s = state(&mut host, &ui, &task);
        assert_eq!(s["task"]["state"], "failed");
        assert!(s["result"].is_null());
        assert!(std::fs::read_dir(path).unwrap().next().is_none());
    }
    #[test]
    fn h10_cancel_before_and_after_stage_late_cancel_and_nonoverwrite() {
        for mode in ["before", "staged", "existing", "late"] {
            let (mut host, ui, lease, _) = fixture();
            let path = directory(mode);
            let params = report_params(&mut host, &ui, &lease, None, &path, json!(["json"]));
            let task = ok(call(
                &mut host,
                &ui,
                "publication-race",
                "export_execution_report",
                params,
            ));
            if mode == "before" {
                ok(call(
                    &mut host,
                    &ui,
                    "cancel-before",
                    "cancel_task",
                    json!({"task_id":task["id"],"action":"cancel"}),
                ));
            }
            let mut work = host.drain_io_work();
            let completed = work.pop().unwrap().run();
            let target = if let Ok(WorkOutput::ExecutionExport(prepared)) = &completed.result {
                let TaskOutput::ExecutionReport(r) = &prepared.output else {
                    panic!()
                };
                Some(
                    path.join(r.report_file.path.as_str())
                        .parent()
                        .unwrap()
                        .to_owned(),
                )
            } else {
                None
            };
            if mode == "staged" {
                ok(call(
                    &mut host,
                    &ui,
                    "cancel-after-stage",
                    "cancel_task",
                    json!({"task_id":task["id"],"action":"cancel"}),
                ));
            }
            if mode == "existing" {
                std::fs::create_dir(target.as_ref().unwrap()).unwrap();
                std::fs::write(
                    target.as_ref().unwrap().join("keep.txt"),
                    b"original user directory",
                )
                .unwrap();
            }
            host.finish_io(completed);
            let s = state(&mut host, &ui, &task);
            match mode {
                "before" | "staged" => {
                    assert_eq!(s["task"]["state"], "cancelled", "{s}");
                    assert!(s["result"].is_null());
                    if let Some(target) = target {
                        assert!(!target.exists());
                    }
                }
                "existing" => {
                    assert_eq!(s["task"]["state"], "failed");
                    assert!(s["result"].is_null());
                    assert_eq!(
                        std::fs::read(target.unwrap().join("keep.txt")).unwrap(),
                        b"original user directory"
                    );
                }
                "late" => {
                    assert_eq!(s["task"]["state"], "completed");
                    let late = ok(call(
                        &mut host,
                        &ui,
                        "late-cancel",
                        "cancel_task",
                        json!({"task_id":task["id"],"action":"cancel"}),
                    ));
                    assert_eq!(late["terminal_won"], true);
                    assert!(target.unwrap().join("report.json").exists());
                }
                _ => unreachable!(),
            }
        }
    }
    #[test]
    fn h10_final_commit_rechecks_revoked_grant_token_and_host_epoch() {
        for mode in ["operation", "read", "token", "epoch"] {
            let (mut host, ui, lease, _) = fixture();
            let path = directory(mode);
            let mut agent = host.sessions[&ui].clone();
            agent.origin = CallerOrigin::ExternalAgent;
            agent.session_id = id("export-authorized-agent");
            agent.granted_project = host.project.as_ref().map(|p| p.id.clone());
            agent.selected_file_tokens.clear();
            host.register_external_session(agent.clone()).unwrap();
            let params = report_params(
                &mut host,
                &agent.session_id,
                &lease,
                None,
                &path,
                json!(["json"]),
            );
            let task = ok(call(
                &mut host,
                &agent.session_id,
                "revoked-export",
                "export_execution_report",
                params,
            ));
            let completed = host.drain_io_work().pop().unwrap().run();
            let mut next = host.sessions[&agent.session_id].clone();
            match mode {
                "operation" => next
                    .allowed_operations
                    .retain(|o| !matches!(o, OperationName::ExportExecutionReport)),
                "read" => next
                    .allowed_operations
                    .retain(|o| !matches!(o, OperationName::ReadExecutionView)),
                "token" => next.selected_file_tokens.clear(),
                "epoch" => host.epoch += 1,
                _ => unreachable!(),
            }
            if mode != "epoch" {
                host.update_external_session_authority(&ui, next).unwrap();
            }
            host.finish_io(completed);
            let actual =
                &host.tasks[&serde_json::from_value::<TaskId>(task["id"].clone()).unwrap()].state;
            assert!(matches!(actual.task.state, ComputationStatus::Failed));
            assert!(actual.result.is_none());
            assert!(std::fs::read_dir(path).unwrap().all(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sf-execution-export-")
            }));
        }
    }

    struct Server {
        url: String,
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
        requests: Arc<Mutex<Vec<(String, Value)>>>,
    }
    impl Server {
        fn new(mut reply: impl FnMut(&str, &Value) -> (u16, Value) + Send + 'static) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let done = stop.clone();
            let requests = Arc::new(Mutex::new(vec![]));
            let seen = requests.clone();
            let thread = std::thread::spawn(move || {
                for connection in listener.incoming() {
                    let mut stream = connection.unwrap();
                    if done.load(Ordering::Acquire) {
                        break;
                    }
                    stream
                        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                        .unwrap();
                    let mut bytes = vec![];
                    let end = loop {
                        let mut b = [0; 1];
                        stream.read_exact(&mut b).unwrap();
                        bytes.push(b[0]);
                        if bytes.ends_with(b"\r\n\r\n") {
                            break bytes.len();
                        }
                    };
                    let headers = String::from_utf8(bytes[..end].to_vec()).unwrap();
                    let first = headers.lines().next().unwrap();
                    let path = first.split_whitespace().nth(1).unwrap().to_owned();
                    let length = headers
                        .lines()
                        .find_map(|s| {
                            s.split_once(':').and_then(|(k, v)| {
                                k.eq_ignore_ascii_case("content-length")
                                    .then(|| v.trim().parse::<usize>().unwrap())
                            })
                        })
                        .unwrap_or(0);
                    let mut body = vec![0; length];
                    stream.read_exact(&mut body).unwrap();
                    let body = if body.is_empty() {
                        Value::Null
                    } else {
                        serde_json::from_slice(&body).unwrap()
                    };
                    seen.lock().unwrap().push((path.clone(), body.clone()));
                    let (status, result) = reply(&path, &body);
                    let body = serde_json::to_vec(&result).unwrap();
                    write!(stream,"HTTP/1.1 {status} result\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
                    stream.write_all(&body).unwrap();
                }
            });
            Self {
                url: format!("http://{addr}/"),
                stop,
                thread: Some(thread),
                requests,
            }
        }
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            let _ =
                TcpStream::connect(self.url.trim_start_matches("http://").trim_end_matches('/'));
            self.thread.take().unwrap().join().unwrap();
        }
    }
    fn response(kind: &str, data: Value) -> Value {
        json!({"contract_version":EXECUTION_CONTRACT_VERSION,"result":{"kind":kind,"data":data}})
    }
    fn connect(host: &mut ProjectHost, ui: &ObjectId, url: &str, token: &str) {
        let project = host.project.as_ref().unwrap().id.clone();
        let connection = CloudReadConnection::local_loopback(
            id("export-read-connection"),
            project,
            "example".to_owned().try_into().unwrap(),
            url.into(),
            token.into(),
            vec![],
        )
        .unwrap();
        host.install_cloud_read_connection(ui, connection).unwrap();
    }
    fn data_params(
        host: &mut ProjectHost,
        ui: &ObjectId,
        path: &std::path::Path,
        frontier: u64,
        devices: Vec<String>,
    ) -> Value {
        let selected = host
            .select_directory(ui, FilePurpose::ExportCloudData, path.to_owned())
            .unwrap();
        json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"revision":host.project.as_ref().unwrap().revision,
            "query":{"contract_version":EXECUTION_CONTRACT_VERSION,"dataset":"telemetry","cloud_project_id":"example","dataset_id":"load-fixture","device_ids":[],
                "received_from":"2020-01-01T00:00:00Z","received_until":"2030-01-01T00:00:00Z","aggregation":"rows","page":{"limit":200,"cursor":null}},
            "selection":{"dataset":"telemetry","cloud_project_id":"example","dataset_id":"load-fixture","selected_visible_frontier":frontier,"selected_device_ids":devices},"selected_destination_ref":selected})
    }
    fn empty_data_server(wrong: bool) -> Server {
        Server::new(move |path, _| {
            if path.contains("datasets") {
                return (
                    200,
                    response(
                        "dataset_catalog",
                        json!({"cloud_project_id":"example","datasets":[{"dataset_id":"load-fixture","cloud_project_id":"example","source_mode":"generated_load","session_id":null,"visible_frontier":0,"purpose":"generated_load"}]}),
                    ),
                );
            }
            (
                200,
                response(
                    "query_result",
                    json!({"contract_version":EXECUTION_CONTRACT_VERSION,"dataset":"telemetry","selected_epoch":null,"selected_execution_revision":null,"rows":[],"next_cursor":null,"data_quality_issues":[],"dataset_id":"load-fixture","selected_visible_frontier":if wrong{1}else{0},"selected_device_ids":[]}),
                ),
            )
        })
    }
    #[test]
    fn h11_real_host_http_empty_two_files_request_recovery_and_strict_task_markers() {
        let (mut host, ui, _, _) = fixture();
        let server = empty_data_server(false);
        connect(&mut host, &ui, &server.url, "component-reader");
        let path = directory("h11-empty");
        let params = data_params(&mut host, &ui, &path, 0, vec![]);
        let task = ok(call(
            &mut host,
            &ui,
            "original-data-export",
            "export_cloud_data",
            params.clone(),
        ));
        assert_eq!(
            ok(call(
                &mut host,
                &ui,
                "original-data-export",
                "export_cloud_data",
                params
            ))["id"],
            task["id"]
        );
        let lookup = ok(call(
            &mut host,
            &ui,
            "find-data-export",
            "read_execution_view",
            json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"request","originating_request_id":"original-data-export"}}),
        ));
        assert_eq!(lookup["data"]["operation"], "export_cloud_data");
        run_one(&mut host);
        let s = state(&mut host, &ui, &task);
        assert_eq!(s["task"]["state"], "completed", "{s}");
        assert_eq!(s["result"]["kind"], "cloud_data_export");
        serde_json::from_value::<TaskState>(s.clone()).unwrap();
        let r = &s["result"]["data"];
        assert_eq!(r["row_count"], 0);
        assert_eq!(r["page_count"], 1);
        let jsonl =
            std::fs::read_to_string(path.join(r["files"][1]["path"].as_str().unwrap())).unwrap();
        let lines: Vec<Value> = jsonl
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[2]["complete"], true);
        for key in ["originating_request_id", "result", "failure"] {
            let mut bad = s.clone();
            bad.as_object_mut().unwrap().remove(key);
            assert!(serde_json::from_value::<TaskState>(bad).is_err());
        }
        for (pointer, value) in [
            ("/originating_operation", json!("export_evaluation")),
            ("/task/kind", json!("simulation")),
            ("/allowed_controls", json!(["pause_calculation"])),
            ("/result/kind", json!("execution_report")),
        ] {
            let mut bad = s.clone();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(
                serde_json::from_value::<TaskState>(bad.clone()).is_err(),
                "{bad}"
            );
        }
        assert_eq!(server.requests.lock().unwrap().len(), 2);
        std::fs::write(
            path.join("task-result.json"),
            serde_json::to_vec_pretty(&s).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn h11_pin_mismatch_fails_and_cancelled_staged_files_never_publish() {
        for wrong in [true, false] {
            let (mut host, ui, _, _) = fixture();
            let server = empty_data_server(wrong);
            connect(&mut host, &ui, &server.url, "component-reader");
            let path = directory("h11-not-committed");
            let params = data_params(&mut host, &ui, &path, 0, vec![]);
            let task = ok(call(
                &mut host,
                &ui,
                "data-failure",
                "export_cloud_data",
                params,
            ));
            let completed = host.drain_io_work().pop().unwrap().run();
            if !wrong {
                ok(call(
                    &mut host,
                    &ui,
                    "cancel-data-stage",
                    "cancel_task",
                    json!({"task_id":task["id"],"action":"cancel"}),
                ));
            }
            host.finish_io(completed);
            let s = state(&mut host, &ui, &task);
            assert_eq!(
                s["task"]["state"],
                if wrong { "failed" } else { "cancelled" }
            );
            assert!(s["result"].is_null());
            assert!(std::fs::read_dir(path).unwrap().all(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sf-execution-export-")
            }));
        }
    }
    #[test]
    fn h11_execution_query_keeps_exact_revision_and_no_telemetry_descriptor() {
        let (mut host, ui, lease, _) = fixture();
        let identity = lease.identity.clone();
        let server = Server::new(move |path, query| {
            assert_eq!(path, "/v2/data/query");
            assert_eq!(
                query["selection"],
                json!({"kind":"revision","execution_revision":0})
            );
            (
                200,
                response(
                    "query_result",
                    json!({"contract_version":EXECUTION_CONTRACT_VERSION,"dataset":"execution",
                "selected_epoch":identity.execution_epoch_id,"selected_execution_revision":0,"rows":[],"next_cursor":null,"data_quality_issues":[]}),
                ),
            )
        });
        connect(&mut host, &ui, &server.url, "component-reader");
        let path = directory("h11-execution-r0");
        let selected = host
            .select_directory(&ui, FilePurpose::ExportCloudData, path.clone())
            .unwrap();
        let params = json!({"execution_contract_version":EXECUTION_CONTRACT_VERSION,"revision":4,
            "query":{"contract_version":EXECUTION_CONTRACT_VERSION,"dataset":"execution","identity":lease.identity,
                "selection":{"kind":"revision","execution_revision":0},"host_from_ms":0,"host_until_ms":0,"aggregation":"rows","page":{"limit":200,"cursor":null}},
            "selection":{"dataset":"execution","identity":lease.identity,"execution_revision":0},"selected_destination_ref":selected});
        let task = ok(call(
            &mut host,
            &ui,
            "execution-data-export",
            "export_cloud_data",
            params,
        ));
        run_one(&mut host);
        let s = state(&mut host, &ui, &task);
        assert_eq!(s["task"]["state"], "completed", "{s}");
        let result = &s["result"]["data"];
        assert!(result["dataset_descriptor"].is_null());
        assert_eq!(result["row_count"], 0);
        assert_eq!(result["query"]["selection"]["execution_revision"], 0);
        assert_eq!(server.requests.lock().unwrap().len(), 1);
    }

    #[test]
    fn h11_cloud_permission_and_transport_errors_keep_their_actual_task_failure() {
        for status in [403_u16, 503_u16] {
            let (mut host, ui, _, _) = fixture();
            let server = Server::new(move |path, _| {
                if path.contains("datasets") {
                    return (
                        200,
                        response(
                            "dataset_catalog",
                            json!({"cloud_project_id":"example","datasets":[{"dataset_id":"load-fixture","cloud_project_id":"example","source_mode":"generated_load","session_id":null,"visible_frontier":0,"purpose":"generated_load"}]}),
                        ),
                    );
                }
                (status, json!({"error":"fixture rejected this page"}))
            });
            connect(&mut host, &ui, &server.url, "component-reader");
            let path = directory("h11-page-rejected");
            let params = data_params(&mut host, &ui, &path, 0, vec![]);
            let task = ok(call(
                &mut host,
                &ui,
                "rejected-data-page",
                "export_cloud_data",
                params,
            ));
            run_one(&mut host);
            let s = state(&mut host, &ui, &task);
            assert_eq!(s["task"]["state"], "failed");
            assert!(s["result"].is_null());
            assert_eq!(
                s["failure"]["code"],
                if status == 403 {
                    "PermissionDenied"
                } else {
                    "IoFailure"
                }
            );
            assert!(std::fs::read_dir(path).unwrap().all(|e| {
                e.unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".sf-execution-export-")
            }));
        }
    }

    #[test]
    fn h10_cloud_source_uses_real_http_c16_c07_and_paginated_c18_from_actual_host_facts() {
        let (mut host, ui, lease, _) = fixture();
        let source =
            typed(json!({"read_origin":"host","identity":lease.identity,"execution_revision":0}))
                .unwrap();
        let input = host.local_report_source(&source).unwrap();
        let snapshot = input.snapshot;
        let inputs = input.inputs;
        let history = input.history;
        assert!(history.records.len() > 1);
        let expected = history.records.len();
        let server = Server::new(move |path, query| {
            let path = path.split('?').next().unwrap();
            if path.ends_with("inputs") {
                return (200, response("inputs", json!(inputs)));
            }
            if path.contains("snapshots/") {
                return (
                    200,
                    response(
                        "snapshot",
                        json!({"snapshot":snapshot,"access":{"ownership":"other_host","allowed_session_controls":[],"device_acceptance_control_ids":[]},"delivery":{"read_origin":"cloud","local_execution_revision":null,"cloud_published_revision":0,"relation":"synced"}}),
                    ),
                );
            }
            assert!(path.ends_with("history/query"));
            assert_eq!(query["execution_revision"], 0);
            let first = query["page"]["cursor"].is_null();
            let records = if first {
                history.records[..1].to_vec()
            } else {
                history.records[1..].to_vec()
            };
            (
                200,
                response(
                    "history",
                    json!({"identity":history.identity,"execution_revision":0,"observation_frontier":history.observation_frontier,
                "host_from_ms":0,"host_until_ms":0,"records":records,"next_cursor":if first{json!("bounded-history-page-2")}else{Value::Null}}),
                ),
            )
        });
        connect(&mut host, &ui, &server.url, "component-reader");
        let path = directory("h10-cloud-transport");
        let mut params = report_params(&mut host, &ui, &lease, None, &path, json!(["json"]));
        params["sources"][0]["read_origin"] = json!("cloud");
        let task = ok(call(
            &mut host,
            &ui,
            "cloud-report-source",
            "export_execution_report",
            params,
        ));
        run_one(&mut host);
        let s = state(&mut host, &ui, &task);
        assert_eq!(s["task"]["state"], "completed", "{s}");
        let report: ExecutionReport = serde_json::from_slice(
            &std::fs::read(path.join(s["result"]["data"]["report_file"]["path"].as_str().unwrap()))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(report.sessions[0].history.records.len(), expected);
        assert_eq!(server.requests.lock().unwrap().len(), 4);
    }
    #[test]
    #[ignore = "requires Root-approved existing retained PG read-only HTTP endpoint and reader env"]
    fn h11_actual_retained_pg_bounded_400_rows_two_pages() {
        let (mut host, ui, _, _) = fixture();
        let path = directory("h11-actual-pg-400");
        let endpoint =
            std::env::var("SF_HOST_EXPORTS_HTTP_ENDPOINT").expect("actual local service endpoint");
        let token = std::env::var("SF_HOST_EXPORTS_READER_TOKEN").expect("private reader token");
        connect(&mut host, &ui, &endpoint, &token);
        let params = data_params(
            &mut host,
            &ui,
            &path,
            400,
            (0..100).map(|n| format!("G{n:03}")).collect(),
        );
        let task = ok(call(
            &mut host,
            &ui,
            "actual-pg-data-export",
            "export_cloud_data",
            params,
        ));
        run_one(&mut host);
        let s = state(&mut host, &ui, &task);
        assert_eq!(s["task"]["state"], "completed", "{s}");
        let r = &s["result"]["data"];
        assert_eq!(r["row_count"], 400);
        assert_eq!(r["page_count"], 2);
        let jsonl =
            std::fs::read_to_string(path.join(r["files"][1]["path"].as_str().unwrap())).unwrap();
        let lines: Vec<Value> = jsonl
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0]["query"], r["query"]);
        assert_eq!(lines[3]["complete"], true);
        assert_eq!(lines[3]["row_count"], 400);
        assert_eq!(lines[1]["result"]["selected_visible_frontier"], 400);
        assert_eq!(lines[2]["result"]["selected_visible_frontier"], 400);
        std::fs::write(
            path.join("task-result.json"),
            serde_json::to_vec_pretty(&s).unwrap(),
        )
        .unwrap();
    }
}
pub(super) enum ReportSourceInput {
    Host(Box<ExecutionReportInputSession>),
    Cloud(ExecutionReportSource, Arc<CloudReadConnection>),
}
pub(super) enum ExecutionExportWork {
    Report {
        id: ObjectId,
        project_id: ProjectId,
        revision: SafeUint,
        sources: Vec<ReportSourceInput>,
        prediction: Option<PredictionInput>,
        formats: Vec<HostExportExecutionReportRequestParamsFormatsItem>,
        destination: ExecutionReportDestination,
        directory: PathBuf,
        resources: Arc<sf_reporting::EvaluationResources>,
    },
    Data {
        id: ObjectId,
        project_id: ProjectId,
        query: HostExportQuery,
        selection: CloudDataExportSelection,
        destination: CloudDataExportDestination,
        directory: PathBuf,
        connection: Arc<CloudReadConnection>,
    },
}
pub(super) struct PreparedExecutionExport {
    stage: StagedExportDirectory,
    output: TaskOutput,
}

fn typed<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, ApiError> {
    serde_json::from_value(value).map_err(|_| {
        error(
            ErrorCode::InvalidInput,
            "Export source does not match the required typed contract",
            None,
        )
    })
}
fn export_failure(issues: Vec<Issue>) -> ApiError {
    let mut failure = engine_failure(issues);
    failure.code = ErrorCode::InvalidInput;
    failure.message = "Execution export failed; no delivery directory was committed".into();
    failure
}
fn cancelled(flag: &AtomicBool) -> Result<(), ApiError> {
    if flag.load(Ordering::Acquire) {
        Err(error(
            ErrorCode::Cancelled,
            "Execution export cancelled before publication",
            None,
        ))
    } else {
        Ok(())
    }
}
fn timestamp() -> Result<CloudTimestamp, ApiError> {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            error(
                ErrorCode::InvalidInput,
                "UTC clock predates the supported export timestamp",
                None,
            )
        })?;
    let epoch = CloudTimestamp::try_from("1970-01-01T00:00:00Z".to_owned()).unwrap();
    let micros = std::time::Duration::new(elapsed.as_secs(), elapsed.subsec_micros() * 1000);
    (*epoch.as_datetime() + micros)
        .to_rfc3339()
        .try_into()
        .map_err(|_| {
            error(
                ErrorCode::InvalidInput,
                "UTC export timestamp is outside the supported range",
                None,
            )
        })
}
fn export_id() -> ObjectId {
    ObjectId::try_from(format!(
        "export-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        EXPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
    .unwrap()
}
fn progress(sink: &Mutex<Option<TaskProgress>>, phase: &str, rows: u64) {
    *sink.lock().expect("export progress") = Some(TaskProgress {
        phase: phase.into(),
        processed: rows.try_into().expect("producer SafeUint count"),
        total: None,
        message: None,
    });
}
fn report_stamp(
    source: &ExecutionReportSource,
    snapshot: &ExecutionSnapshot,
    delivery: Value,
) -> Result<ExecutionReportSourceStamp, ApiError> {
    typed(
        serde_json::json!({"read_origin":source.read_origin,"identity":source.identity,
        "execution_revision":source.execution_revision,"observation_frontier":snapshot.observation_frontier,"delivery":delivery}),
    )
}

fn cloud_report_source(
    source: &ExecutionReportSource,
    connection: &CloudReadConnection,
    cancel: &AtomicBool,
) -> Result<ExecutionReportInputSession, ApiError> {
    cancelled(cancel)?;
    let CloudReadResult::Inputs(inputs) = connection.read(&CloudReadSelection::Inputs {
        identity: source.identity.clone(),
    })?
    else {
        unreachable!("managed reader validates variant")
    };
    cancelled(cancel)?;
    let CloudReadResult::Snapshot(envelope) = connection.read(&CloudReadSelection::Snapshot {
        identity: source.identity.clone(),
        selection: SnapshotSelection::Revision {
            execution_revision: source.execution_revision.clone(),
        },
    })?
    else {
        unreachable!("managed reader validates variant")
    };
    let snapshot = envelope.snapshot;
    let stamp = report_stamp(
        source,
        &snapshot,
        serde_json::to_value(envelope.delivery).unwrap(),
    )?;
    let mut query: ExecutionHistoryQuery = typed(
        serde_json::json!({"contract_version":EXECUTION_CONTRACT_VERSION,
        "identity":source.identity,"execution_revision":source.execution_revision,"host_from_ms":0,
        "host_until_ms":snapshot.coverage.observed_until_host_ms,"objects":[],"page":{"limit":200,"cursor":null}}),
    )?;
    let mut records = vec![];
    loop {
        cancelled(cancel)?;
        let CloudReadResult::History(page) = connection.read(&CloudReadSelection::History {
            query: query.clone(),
        })?
        else {
            unreachable!("managed reader validates variant")
        };
        if page.observation_frontier != snapshot.observation_frontier
            || page.host_from_ms != query.host_from_ms
            || page.host_until_ms != query.host_until_ms
            || page.records.len() > 200
        {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud report history changed its fixed R/F/time selection",
                None,
            ));
        }
        for record in &page.records {
            if record.record_sequence.value() != records.len() as u64 + 1
                || record.record_sequence.value() > snapshot.observation_frontier.value()
            {
                return Err(error(
                    ErrorCode::IoFailure,
                    "Cloud report history is missing, repeated or beyond fixed F",
                    None,
                ));
            }
            records.push(record.clone());
        }
        match page.next_cursor {
            Some(next)
                if !page.records.is_empty()
                    && !exact_data(&query.page.cursor, &Some(next.clone()))
                    && (records.len() as u64) < snapshot.observation_frontier.value() =>
            {
                query.page.cursor = Some(next)
            }
            Some(_) => {
                return Err(error(
                    ErrorCode::IoFailure,
                    "Cloud report history cursor made no valid prefix progress",
                    None,
                ));
            }
            None => break,
        }
    }
    let history = typed(
        serde_json::json!({"document_kind":"execution_observation_history","contract_version":EXECUTION_CONTRACT_VERSION,
        "identity":source.identity,"observation_frontier":snapshot.observation_frontier,"records":records}),
    )?;
    Ok(ExecutionReportInputSession {
        source: stamp,
        inputs,
        snapshot,
        history,
    })
}

impl ExecutionExportWork {
    pub(super) fn run(
        self,
        cancel: &AtomicBool,
        sink: &Mutex<Option<TaskProgress>>,
    ) -> Result<PreparedExecutionExport, ApiError> {
        cancelled(cancel)?;
        match self {
            Self::Report {
                id,
                project_id,
                revision,
                sources,
                prediction,
                formats,
                destination,
                directory,
                resources,
            } => {
                progress(sink, "读取固定执行来源", 0);
                let mut sessions = vec![];
                for source in sources {
                    sessions.push(match source {
                        ReportSourceInput::Host(session) => *session,
                        ReportSourceInput::Cloud(source, connection) => {
                            cloud_report_source(&source, &connection, cancel)?
                        }
                    });
                    cancelled(cancel)?;
                }
                let report = sf_reporting::build_execution_report(ExecutionReportInput {
                    report_id: id.clone(),
                    project_id,
                    export_project_revision: revision,
                    generated_at_utc: timestamp()?,
                    sessions,
                    prediction,
                })
                .map_err(export_failure)?;
                let requested: Vec<_> = formats
                    .iter()
                    .map(|f| match f {
                        HostExportExecutionReportRequestParamsFormatsItem::Pdf => {
                            sf_reporting::EvaluationOutputFormat::Pdf
                        }
                        HostExportExecutionReportRequestParamsFormatsItem::Html => {
                            sf_reporting::EvaluationOutputFormat::Html
                        }
                        HostExportExecutionReportRequestParamsFormatsItem::Csv => {
                            sf_reporting::EvaluationOutputFormat::Csv
                        }
                        HostExportExecutionReportRequestParamsFormatsItem::Json => {
                            sf_reporting::EvaluationOutputFormat::Json
                        }
                    })
                    .collect();
                progress(sink, "编码同源执行报告", 0);
                let mut files =
                    sf_reporting::render_execution_report(&report, &requested, &resources, cancel)
                        .map_err(|issues| {
                            if cancel.load(Ordering::Acquire) {
                                error(
                                    ErrorCode::Cancelled,
                                    "Report export cancelled before publication",
                                    None,
                                )
                            } else {
                                export_failure(issues)
                            }
                        })?;
                // Compact Task receipts always point to the canonical report, even
                // when JSON was not selected as an additional presentation format.
                if !files.iter().any(|f| f.relative_path == "report.json") {
                    files.push(sf_reporting::EvaluationFile {
                        relative_path: "report.json".into(),
                        media_type: "application/json".into(),
                        bytes: serde_json::to_vec_pretty(&report)
                            .expect("canonical report serializes"),
                    });
                }
                let name = format!("execution-report-{}", id.as_str());
                let stage = StagedExportDirectory::create(&directory, &name)?;
                let mut refs = vec![];
                for file in files {
                    cancelled(cancel)?;
                    stage.write(&file.relative_path, &file.bytes)?;
                    refs.push(file_ref(
                        &format!("{name}/{}", file.relative_path),
                        DocumentKind::Asset,
                    ));
                }
                let report_file = file_ref(&format!("{name}/report.json"), DocumentKind::Asset);
                let prediction =
                    report
                        .prediction_comparison
                        .as_ref()
                        .map(|p| PredictionExecutionLink {
                            prediction: p.prediction.source.clone(),
                            execution: p.execution.clone(),
                        });
                let output = ExportExecutionReportResult {
                    destination,
                    report_file,
                    report_id: id,
                    sources: report.sessions.iter().map(|s| s.source.clone()).collect(),
                    files: refs,
                    limitations: report.limitations.clone(),
                    prediction,
                };
                progress(sink, "等待宿主提交报告目录", output.files.len() as u64);
                Ok(PreparedExecutionExport {
                    stage,
                    output: TaskOutput::ExecutionReport(Box::new(output)),
                })
            }
            Self::Data {
                id,
                project_id,
                query,
                selection,
                destination,
                directory,
                connection,
            } => {
                let mut canonical = serde_json::to_value(&query).unwrap();
                canonical["page"] = serde_json::json!({"limit":200,"cursor":null});
                let query: HostExportQuery = typed(canonical.clone())?;
                let query_domain: DaaSQuery = typed(canonical)?;
                let descriptor = match &query_domain {
                    DaaSQuery::Telemetry { dataset_id, .. } => {
                        cancelled(cancel)?;
                        let CloudReadResult::DatasetCatalog(catalog) =
                            connection.read(&CloudReadSelection::DatasetCatalog {})?
                        else {
                            unreachable!("managed reader validates variant")
                        };
                        let selected: Vec<_> = catalog
                            .datasets
                            .into_iter()
                            .filter(|d| &d.dataset_id == dataset_id)
                            .collect();
                        if selected.len() != 1 {
                            return Err(error(
                                ErrorCode::MissingReference,
                                "Exact dataset descriptor is unavailable or ambiguous",
                                None,
                            ));
                        }
                        selected.into_iter().next()
                    }
                    DaaSQuery::Execution { .. } => None,
                };
                let input = CloudDataExportInput {
                    export_id: id.clone(),
                    project_id,
                    query: query_domain,
                    selection: selection.clone(),
                    dataset_descriptor: descriptor.clone(),
                    generated_at_utc: timestamp()?,
                };
                let name = format!("cloud-data-{}", id.as_str());
                let stage = StagedExportDirectory::create(&directory, &name)?;
                let mut csv = stage.create_file("data.csv")?;
                let mut jsonl = stage.create_file("data.jsonl")?;
                let mut read_failure = None;
                let counts = sf_reporting::write_cloud_data_export(
                    &input,
                    &mut csv,
                    &mut jsonl,
                    cancel,
                    |rows| progress(sink, "逐页写入固定筛选数据", rows),
                    |query| {
                        let result = connection
                            .read(&CloudReadSelection::DataQuery {
                                query: query.clone(),
                            })
                            .map_err(|e| {
                                let issue = Issue {
                                    code: format!("host.cloud_export.{:?}", e.code),
                                    message: e.message.clone(),
                                    object_ref: None,
                                    field_path: None,
                                    blocks: vec![],
                                };
                                read_failure = Some(e);
                                issue
                            })?;
                        let CloudReadResult::QueryResult(page) = result else {
                            unreachable!("managed reader validates variant")
                        };
                        Ok(page)
                    },
                )
                .map_err(|issue| {
                    if cancel.load(Ordering::Acquire) {
                        error(
                            ErrorCode::Cancelled,
                            "Data export cancelled before publication",
                            None,
                        )
                    } else if let Some(failure) = read_failure {
                        failure
                    } else {
                        let io = issue.code == "cloud_data_export_io";
                        let mut failure = export_failure(vec![issue]);
                        if io {
                            failure.code = ErrorCode::IoFailure;
                        }
                        failure
                    }
                })?;
                csv.flush()
                    .and_then(|_| csv.sync_all())
                    .map_err(|_| error(ErrorCode::IoFailure, "CSV stage flush failed", None))?;
                jsonl
                    .flush()
                    .and_then(|_| jsonl.sync_all())
                    .map_err(|_| error(ErrorCode::IoFailure, "JSONL stage flush failed", None))?;
                drop((csv, jsonl));
                cancelled(cancel)?;
                let mut result_query = serde_json::to_value(query).unwrap();
                match &selection {
                    CloudDataExportSelection::Telemetry {
                        selected_device_ids,
                        ..
                    } => {
                        result_query["device_ids"] =
                            serde_json::to_value(selected_device_ids).unwrap()
                    }
                    CloudDataExportSelection::Execution {
                        execution_revision, ..
                    } => {
                        result_query["selection"] = serde_json::json!({"kind":"revision","execution_revision":execution_revision})
                    }
                }
                let query = typed(result_query)?;
                let output = ExportCloudDataResult {
                    export_id: id,
                    destination,
                    query,
                    selection,
                    page_count: typed(serde_json::json!(counts.page_count))?,
                    row_count: counts.row_count.try_into().unwrap(),
                    quality_issue_occurrence_count: counts
                        .quality_issue_occurrence_count
                        .try_into()
                        .unwrap(),
                    files: [
                        file_ref(&format!("{name}/data.csv"), DocumentKind::Asset),
                        file_ref(&format!("{name}/data.jsonl"), DocumentKind::Asset),
                    ],
                    dataset_descriptor: descriptor,
                };
                progress(sink, "等待宿主提交数据目录", counts.row_count);
                Ok(PreparedExecutionExport {
                    stage,
                    output: TaskOutput::CloudDataExport(Box::new(output)),
                })
            }
        }
    }
}

impl ProjectHost {
    /// Shared original-request identity covers H01/H08/H10/H11 without changing
    /// H01's frozen-media preparation implementation or its Native interfaces.
    pub(super) fn dispatch_execution_original(
        &mut self,
        session: &ObjectId,
        request: ApiRequest,
    ) -> Value {
        let payload = serde_json::to_value(&request).expect("typed original request");
        let (id, project, scope, operation) = request_header(request.clone());
        let admitted = (|| {
            let caller = self.sessions.get(session).ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Unknown or revoked session",
                    None,
                )
            })?;
            self.admit(caller, &project, &scope, &operation)?;
            if let Some(original) = self
                .original_execution_requests
                .get(&(session.clone(), id.clone()))
            {
                if original.payload != payload || !same(&original.operation, &operation) {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Original request_id already belongs to a different operation or payload",
                        Some("/request_id"),
                    ));
                }
                return Ok(Some(
                    serde_json::to_value(
                        current_task_state(self.owned_task(caller, &scope, &original.task_id)?)
                            .task,
                    )
                    .unwrap(),
                ));
            }
            Ok(None)
        })();
        match admitted {
            Err(failure) => self.failure(id, session, &scope, failure),
            Ok(Some(task)) => self.respond(id, session, &scope, Ok(task)),
            Ok(None) => {
                let response = self.dispatch_once(session, request);
                if let Some(raw_id) = response.pointer("/outcome/data/id").and_then(Value::as_str) {
                    if let Ok(task_id) = TaskId::try_from(raw_id.to_owned()) {
                        if self.tasks.get(&task_id).is_some_and(|t| {
                            t.owner == *session && same(&t.state.originating_operation, &operation)
                        }) {
                            self.original_execution_requests.insert(
                                (session.clone(), id),
                                OriginalExecutionRequest {
                                    payload,
                                    operation,
                                    task_id,
                                },
                            );
                        }
                    }
                }
                response
            }
        }
    }
    pub(super) fn registered_execution_export(
        &self,
        caller: &HostCallContext,
        id: &ObjectId,
    ) -> Result<Option<ExecutionRequestStatus>, ApiError> {
        let Some(original) = self
            .original_execution_requests
            .get(&(caller.session_id.clone(), id.clone()))
        else {
            return Ok(None);
        };
        if !matches!(
            original.operation,
            OperationName::ExportExecutionReport | OperationName::ExportCloudData
        ) {
            return Ok(None);
        }
        let task =
            current_task_state(self.owned_task(caller, &ApiScope::Project, &original.task_id)?)
                .task;
        typed(
            serde_json::json!({"host_context_session_id":self.host_instance_id,"state":"registered",
            "operation":original.operation,"task":task,"originating_request_id":id}),
        )
        .map(Some)
    }
    fn export_caller(
        &self,
        session: &ObjectId,
        project: &Option<ProjectId>,
        scope: &ApiScope,
        operation: OperationName,
        revision: &SafeUint,
    ) -> Result<HostCallContext, ApiError> {
        let caller = self.sessions.get(session).cloned().ok_or_else(|| {
            error(
                ErrorCode::PermissionDenied,
                "Unknown or revoked session",
                None,
            )
        })?;
        self.admit(&caller, project, scope, &operation)?;
        if !matches!(scope, ApiScope::Project) {
            return Err(unsupported("Execution exports require Project scope"));
        }
        check_revision(
            &Revision(revision.clone()),
            &self.project.as_ref().unwrap().revision,
            "/params/revision",
        )?;
        Ok(caller)
    }
    fn require_export_read(
        &self,
        caller: &HostCallContext,
        operation: OperationName,
    ) -> Result<(), ApiError> {
        self.admit(
            caller,
            &self.project.as_ref().map(|p| p.id.clone()),
            &ApiScope::Project,
            &operation,
        )
    }
    fn local_report_source(
        &self,
        source: &ExecutionReportSource,
    ) -> Result<ExecutionReportInputSession, ApiError> {
        let local = self.local_execution_source(&source.identity)?;
        let snapshot = local.snapshot(&SnapshotSelection::Revision {execution_revision:source.execution_revision.clone()})?.into_owned();
        let mut history=local.history_document()?;
        history.records.truncate(snapshot.observation_frontier.value() as usize);
        history.observation_frontier=snapshot.observation_frontier.clone();
        let published=local.published_revision();
        let relation=match &local {
            host_execution::LocalExecutionSource::Archive(_)=>"unknown",
            host_execution::LocalExecutionSource::Live(_)=>if published.as_ref().is_some_and(|r|r.value()>=source.execution_revision.value()) {"synced"}else{"publication_pending"},
        };
        let stamp=report_stamp(source,&snapshot,serde_json::json!({"read_origin":"host","local_execution_revision":source.execution_revision,
            "cloud_published_revision":published,"relation":relation}))?;
        Ok(ExecutionReportInputSession {source:stamp,inputs:local.frozen().clone(),snapshot,history})
    }

    fn export_prediction(
        &self,
        caller: &HostCallContext,
        link: &PredictionExecutionLink,
        sources: &[ExecutionReportSource],
    ) -> Result<PredictionInput, ApiError> {
        self.require_export_read(caller, OperationName::GetRun)?;
        if !sources.iter().any(|s| exact_data(s, &link.execution)) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Prediction target must be one selected exact execution source",
                Some("/params/prediction"),
            ));
        }
        let run = self.runs.get(&link.prediction.run_id).ok_or_else(|| {
            error(
                ErrorCode::MissingReference,
                "Prediction Run is absent",
                None,
            )
        })?;
        let project = self.project.as_ref().unwrap();
        if !matches!(&run.scope,EditScope::Project(id) if id==&project.id)
            || run.result.input_ref.project_id != project.id
            || !matches!(run.result.evidence_origin, EvidenceOrigin::ActualSoftware)
            || !terminal(&run.result.computation_status)
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Prediction requires an actual terminal Project-scoped Run",
                None,
            ));
        }
        let summary = run_summary(&run.result);
        let frozen = self.frozen_run_project(&summary)?;
        let scenario = self.frozen_run_scenario(&summary)?;
        let expected = ReportRunInputRef {
            project_id: frozen.id.clone(),
            project_revision: frozen.revision.clone(),
            scenario_id: scenario.id.clone(),
            scenario_revision: scenario.revision.clone(),
        };
        if !exact_data(&expected, &link.prediction.input_ref) {
            return Err(error(
                ErrorCode::InvalidInput,
                "Prediction frozen input reference conflicts with original Run",
                Some("/params/prediction"),
            ));
        }
        let mut seen = BTreeSet::new();
        let definitions = frozen
            .module_refs
            .iter()
            .chain(frozen.devices.iter().flat_map(|d| &d.route_operation_refs))
            .filter(|r| seen.insert(module_key(r)))
            .map(|r| self.frozen_run_module(&summary, r).map(|m| m.definition))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PredictionInput {
            reference: link.prediction.clone(),
            project: frozen,
            scenario,
            definitions,
            run: run.result.as_ref().clone(),
            execution: link.execution.clone(),
        })
    }
    pub(super) fn dispatch_execution_report(
        &mut self,
        session: &ObjectId,
        request: HostExportExecutionReportRequest,
    ) -> Value {
        let r = request.into_inner();
        let result = (|| {
            let p = r.params;
            let caller = self.export_caller(
                session,
                &r.project_id,
                &r.scope,
                OperationName::ExportExecutionReport,
                &p.revision,
            )?;
            let selected = SelectedFileRef {
                selection_id: p.selected_destination_ref.selection_id.clone(),
                purpose: FilePurpose::ExportExecutionReport,
            };
            let directory =
                self.selected_directory(&caller, &selected, &FilePurpose::ExportExecutionReport)?;
            let mut identities = BTreeSet::new();
            let mut sources = vec![];
            for source in &p.sources {
                if !identities.insert((
                    source.identity.session_id.clone(),
                    source.identity.execution_epoch_id.clone(),
                )) {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Report sources must name distinct session/epoch identities",
                        Some("/params/sources"),
                    ));
                }
                match source.read_origin {
                    ExecutionReportSourceReadOrigin::Host => {
                        self.require_export_read(&caller, OperationName::GetExecutionSnapshot)?;
                        self.require_export_read(&caller, OperationName::ReadExecutionView)?;
                        sources.push(ReportSourceInput::Host(Box::new(
                            self.local_report_source(source)?,
                        )));
                    }
                    ExecutionReportSourceReadOrigin::Cloud => {
                        self.require_export_read(&caller, OperationName::ReadCloudProduction)?;
                        if Some(&source.identity.project_id) != r.project_id.as_ref() {
                            return Err(error(
                                ErrorCode::PermissionDenied,
                                "Cloud report source is outside Project",
                                None,
                            ));
                        }
                        let connection = self
                            .cloud_connections
                            .get(&source.identity.project_id)
                            .cloned()
                            .ok_or_else(|| {
                                unsupported("No managed cloud reader for report source")
                            })?;
                        sources.push(ReportSourceInput::Cloud(source.clone(), connection));
                    }
                }
            }
            let prediction = p
                .prediction
                .as_ref()
                .map(|link| self.export_prediction(&caller, link, &p.sources))
                .transpose()?;
            let project = self.project.as_ref().unwrap();
            let work = ExecutionExportWork::Report {
                id: export_id(),
                project_id: project.id.clone(),
                revision: p.revision,
                sources,
                prediction,
                formats: p.formats,
                destination: p.selected_destination_ref,
                directory,
                resources: self.evaluation_resources.clone(),
            };
            let task = self.enqueue_io(
                &caller,
                TaskScope::Edit(EditScope::Project(project.id.clone())),
                OperationName::ExportExecutionReport,
                project.revision.clone(),
                Some(selected),
                IoInput::ExecutionExport(Box::new(work)),
            );
            self.tasks
                .get_mut(&task.id)
                .unwrap()
                .state
                .originating_request_id = Some(r.request_id.clone());
            typed::<ExportingTaskHandle>(serde_json::to_value(task).unwrap())
        })();
        self.respond(r.request_id, session, &r.scope, result)
    }
    pub(super) fn dispatch_cloud_data_export(
        &mut self,
        session: &ObjectId,
        request: HostExportCloudDataRequest,
    ) -> Value {
        let r = request.into_inner();
        let result = (|| {
            let p = r.params;
            let caller = self.export_caller(
                session,
                &r.project_id,
                &r.scope,
                OperationName::ExportCloudData,
                &p.revision,
            )?;
            self.require_export_read(&caller, OperationName::ReadCloudProduction)?;
            let selected = SelectedFileRef {
                selection_id: p.selected_destination_ref.selection_id.clone(),
                purpose: FilePurpose::ExportCloudData,
            };
            let directory =
                self.selected_directory(&caller, &selected, &FilePurpose::ExportCloudData)?;
            let project = self.project.as_ref().unwrap();
            let connection = self
                .cloud_connections
                .get(&project.id)
                .cloned()
                .ok_or_else(|| unsupported("No managed cloud reader for data export"))?;
            let work = ExecutionExportWork::Data {
                id: export_id(),
                project_id: project.id.clone(),
                query: p.query,
                selection: p.selection,
                destination: p.selected_destination_ref,
                directory,
                connection,
            };
            let task = self.enqueue_io(
                &caller,
                TaskScope::Edit(EditScope::Project(project.id.clone())),
                OperationName::ExportCloudData,
                project.revision.clone(),
                Some(selected),
                IoInput::ExecutionExport(Box::new(work)),
            );
            self.tasks
                .get_mut(&task.id)
                .unwrap()
                .state
                .originating_request_id = Some(r.request_id.clone());
            typed::<ExportingTaskHandle>(serde_json::to_value(task).unwrap())
        })();
        self.respond(r.request_id, session, &r.scope, result)
    }
    pub(super) fn finish_execution_export(
        &mut self,
        id: TaskId,
        prepared: PreparedExecutionExport,
    ) {
        let result = (|| {
            let task = self.tasks.get(&id).ok_or_else(|| {
                error(
                    ErrorCode::MissingReference,
                    "Original export task is absent",
                    None,
                )
            })?;
            if terminal(&task.state.task.state) {
                return Err(error(
                    ErrorCode::Cancelled,
                    "Export task already ended before publication",
                    None,
                ));
            }
            cancelled(&task.cancelled)?;
            if task.epoch != self.epoch {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Export Host project context changed before publication",
                    None,
                ));
            }
            let caller = self.sessions.get(&task.owner).ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Export caller was revoked before publication",
                    None,
                )
            })?;
            let TaskScope::Edit(EditScope::Project(project)) = &task.state.task.scope else {
                return Err(unsupported("Export requires Project task scope"));
            };
            self.admit(
                caller,
                &Some(project.clone()),
                &ApiScope::Project,
                &task.state.originating_operation,
            )?;
            if let TaskOutput::ExecutionReport(report) = &prepared.output {
                if report
                    .sources
                    .iter()
                    .any(|s| matches!(s.read_origin, ExecutionReportSourceStampReadOrigin::Host))
                {
                    self.require_export_read(caller, OperationName::GetExecutionSnapshot)?;
                    self.require_export_read(caller, OperationName::ReadExecutionView)?;
                }
                if report.prediction.is_some() {
                    self.require_export_read(caller, OperationName::GetRun)?;
                }
            }
            if matches!(
                task.state.originating_operation,
                OperationName::ExportCloudData
            ) || matches!(&prepared.output,TaskOutput::ExecutionReport(result) if result.sources.iter().any(|s|matches!(s.read_origin,ExecutionReportSourceStampReadOrigin::Cloud)))
            {
                self.require_export_read(caller, OperationName::ReadCloudProduction)?;
            }
            let selection = task
                .selection
                .as_ref()
                .expect("export has selected destination");
            let actual = self.selected_directory(caller, selection, &selection.purpose)?;
            if actual != prepared.stage.parent() {
                return Err(error(
                    ErrorCode::PermissionDenied,
                    "Export directory token changed before publication",
                    None,
                ));
            }
            prepared.stage.commit(&task.cancelled)?;
            Ok(prepared.output)
        })();
        match result {
            Ok(output) => {
                let result_ref = match &output {
                    TaskOutput::ExecutionReport(r) => Some(r.report_file.clone()),
                    TaskOutput::CloudDataExport(r) => Some(r.files[0].clone()),
                    _ => unreachable!(),
                };
                let task = self.tasks.get_mut(&id).expect("committed original task");
                task.state.task.state = ComputationStatus::Completed;
                task.state.task.result_ref = result_ref;
                task.state.result = Some(output);
                task.state.allowed_controls.clear();
            }
            Err(failure) => self.finish_io(ProjectIoCompletion {
                task_id: id,
                result: Err(failure),
            }),
        }
    }
}
