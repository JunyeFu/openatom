use super::*;
use crate::execution_api::{
    CloudProductionConfiguration, CloudReadResult, CloudReadSelection, CloudReadTaskHandle,
    CloudSourceMode, HostCloudReadRequest,
};
use reqwest::{Method, Url, blocking::Client};
use sf_domain::cloud_execution::*;

/// Native-owned connection. No serializer or Debug implementation exposes a
/// credential, and no public request accepts this configuration.
#[derive(Clone)]
pub struct CloudReadConnection {
    pub(super) connection_ref: ObjectId,
    project_id: ProjectId,
    cloud_project_id: CloudProjectId,
    endpoint: Url,
    reader_token: String,
    pub(super) candidates: Vec<CandidateBinding>,
    client: Client,
}
impl CloudReadConnection {
    pub fn https(
        connection_ref: ObjectId,
        project_id: ProjectId,
        cloud_project_id: CloudProjectId,
        endpoint: String,
        reader_token: String,
        candidates: Vec<CandidateBinding>,
    ) -> Result<Self, ApiError> {
        Self::build(
            connection_ref,
            project_id,
            cloud_project_id,
            endpoint,
            reader_token,
            candidates,
            false,
        )
    }
    /// Explicit local integration mode; public internet HTTP is never enabled.
    pub fn local_loopback(
        connection_ref: ObjectId,
        project_id: ProjectId,
        cloud_project_id: CloudProjectId,
        endpoint: String,
        reader_token: String,
        candidates: Vec<CandidateBinding>,
    ) -> Result<Self, ApiError> {
        Self::build(
            connection_ref,
            project_id,
            cloud_project_id,
            endpoint,
            reader_token,
            candidates,
            true,
        )
    }
    fn build(
        connection_ref: ObjectId,
        project_id: ProjectId,
        cloud_project_id: CloudProjectId,
        endpoint: String,
        reader_token: String,
        candidates: Vec<CandidateBinding>,
        local: bool,
    ) -> Result<Self, ApiError> {
        let endpoint = Url::parse(&endpoint).map_err(|_| {
            error(
                ErrorCode::InvalidInput,
                "Invalid private cloud endpoint",
                None,
            )
        })?;
        let local_http = local
            && endpoint.scheme() == "http"
            && matches!(endpoint.host_str(), Some("127.0.0.1" | "[::1]" | "::1"));
        if (endpoint.scheme() != "https" && !local_http)
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
            || endpoint.host_str().is_none()
            || reader_token.is_empty()
            || reader_token.contains(['\r', '\n'])
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "A fixed HTTPS origin and private reader credential are required",
                None,
            ));
        }
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| {
                error(
                    ErrorCode::IoFailure,
                    "Cloud TLS client initialization failed",
                    None,
                )
            })?;
        Ok(Self {
            connection_ref,
            project_id,
            cloud_project_id,
            endpoint,
            reader_token,
            candidates,
            client,
        })
    }
    fn url(&self, segments: &[&str]) -> Url {
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .expect("fixed HTTP origin is a base URL")
            .clear()
            .extend(segments);
        url
    }
    fn identity(&self, identity: &ExecutionIdentity) -> Result<(), ApiError> {
        if identity.project_id != self.project_id {
            Err(error(
                ErrorCode::PermissionDenied,
                "Execution identity is outside the configured local project",
                None,
            ))
        } else {
            Ok(())
        }
    }
    fn read_request(&self, selection: &CloudReadSelection) -> Result<reqwest::blocking::RequestBuilder, ApiError> {
        let mut query: Vec<(&str, String)> = vec![];
        let (method, segments, body): (Method, Vec<String>, Option<Value>) = match selection {
            CloudReadSelection::DatasetCatalog {} => (
                Method::GET,
                vec![
                    "v2".into(),
                    "projects".into(),
                    self.cloud_project_id.as_str().into(),
                    "datasets".into(),
                ],
                None,
            ),
            CloudReadSelection::ActiveExecution {} => (
                Method::GET,
                vec![
                    "v2".into(),
                    "projects".into(),
                    self.cloud_project_id.as_str().into(),
                    "active-execution".into(),
                ],
                None,
            ),
            CloudReadSelection::DataQuery { query: q } => {
                match q {
                    DaaSQuery::Telemetry {
                        cloud_project_id, ..
                    } if cloud_project_id != &self.cloud_project_id => {
                        return Err(error(
                            ErrorCode::PermissionDenied,
                            "Dataset query is outside the configured cloud project",
                            None,
                        ));
                    }
                    DaaSQuery::Execution { identity, .. } => self.identity(identity)?,
                    _ => {}
                }
                (
                    Method::POST,
                    vec!["v2".into(), "data".into(), "query".into()],
                    Some(serde_json::to_value(q).unwrap()),
                )
            }
            CloudReadSelection::Snapshot {
                identity,
                selection,
            } => {
                self.identity(identity)?;
                query.push(("identity", serde_json::to_string(identity).unwrap()));
                let selector = match selection {
                    SnapshotSelection::Latest {} => "latest".into(),
                    SnapshotSelection::Cutoff {} => "cutoff".into(),
                    SnapshotSelection::Revision { execution_revision } => {
                        execution_revision.value().to_string()
                    }
                };
                (
                    Method::GET,
                    vec![
                        "v2".into(),
                        "sessions".into(),
                        identity.session_id.as_str().into(),
                        "snapshots".into(),
                        selector,
                    ],
                    None,
                )
            }
            CloudReadSelection::Command {
                identity,
                command_id,
            } => {
                self.identity(identity)?;
                (
                    Method::GET,
                    vec!["v2".into(), "commands".into(), command_id.as_str().into()],
                    None,
                )
            }
            CloudReadSelection::CommandReceipts { query: q } => {
                self.identity(&q.identity)?;
                query.push(("query", serde_json::to_string(q).unwrap()));
                (
                    Method::GET,
                    vec![
                        "v2".into(),
                        "commands".into(),
                        q.command_id.as_str().into(),
                        "receipts".into(),
                    ],
                    None,
                )
            }
            CloudReadSelection::Inputs { identity } => {
                self.identity(identity)?;
                query.push(("identity", serde_json::to_string(identity).unwrap()));
                (
                    Method::GET,
                    vec![
                        "v2".into(),
                        "sessions".into(),
                        identity.session_id.as_str().into(),
                        "inputs".into(),
                    ],
                    None,
                )
            }
            CloudReadSelection::Observations { query: q } => {
                self.identity(&q.identity)?;
                (
                    Method::POST,
                    vec![
                        "v2".into(),
                        "sessions".into(),
                        q.identity.session_id.as_str().into(),
                        "observations".into(),
                        "query".into(),
                    ],
                    Some(serde_json::to_value(q).unwrap()),
                )
            }
            CloudReadSelection::History { query: q } => {
                self.identity(&q.identity)?;
                (
                    Method::POST,
                    vec![
                        "v2".into(),
                        "sessions".into(),
                        q.identity.session_id.as_str().into(),
                        "history".into(),
                        "query".into(),
                    ],
                    Some(serde_json::to_value(q).unwrap()),
                )
            }
            CloudReadSelection::Configuration { .. } => {
                unreachable!("configuration is frozen inside Host admission")
            }
        };
        let parts: Vec<_> = segments.iter().map(String::as_str).collect();
        let mut request = self
            .client
            .request(method, self.url(&parts))
            .bearer_auth(&self.reader_token)
            .query(&query);
        if let Some(body) = body {
            request = request.json(&body);
        }
        Ok(request)
    }
    pub(super) fn read(&self, selection: &CloudReadSelection) -> Result<CloudReadResult, ApiError> {
        let request = self.read_request(selection)?;
        let response = request.send().map_err(|_| {
            error(
                ErrorCode::IoFailure,
                "Cloud connection or TLS request failed",
                None,
            )
        })?;
        let status = response.status();
        if status.is_redirection() {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud redirects are not accepted",
                None,
            ));
        }
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Cloud reader scope was rejected",
                None,
            ));
        }
        if !status.is_success() {
            return Err(error(
                ErrorCode::IoFailure,
                &format!("Cloud read failed with HTTP {}", status.as_u16()),
                None,
            ));
        }
        let decoded: CloudResponse = response.json().map_err(|_| {
            error(
                ErrorCode::IoFailure,
                "Cloud response does not match the execution contract",
                None,
            )
        })?;
        let CloudResponse::Variant0(success) = decoded else {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud returned an error in a successful HTTP response",
                None,
            ));
        };
        let value = serde_json::to_value(success.result).expect("typed cloud result serializes");
        let result = serde_json::from_value::<CloudReadResult>(value).map_err(|_| {
            error(
                ErrorCode::IoFailure,
                "Cloud response is not the requested read result",
                None,
            )
        })?;
        self.check_result(selection, &result)?;
        Ok(result)
    }
    fn check_result(
        &self,
        selection: &CloudReadSelection,
        result: &CloudReadResult,
    ) -> Result<(), ApiError> {
        let valid = match (selection, result) {
            (CloudReadSelection::DatasetCatalog {}, CloudReadResult::DatasetCatalog(d)) => {
                d.cloud_project_id == self.cloud_project_id
            }
            (CloudReadSelection::ActiveExecution {}, CloudReadResult::ActiveExecution(d)) => {
                d.cloud_project_id == self.cloud_project_id
            }
            (
                CloudReadSelection::Snapshot {
                    identity,
                    selection,
                },
                CloudReadResult::Snapshot(d),
            ) => {
                exact_data(identity, &d.snapshot.identity)
                    && match selection {
                        SnapshotSelection::Revision { execution_revision } => {
                            execution_revision.value() == d.snapshot.execution_revision.value()
                        }
                        SnapshotSelection::Cutoff {} => d
                            .snapshot
                            .coverage
                            .cutoff_snapshot_revision
                            .as_ref()
                            .is_some_and(|revision| {
                                revision.value() == d.snapshot.execution_revision.value()
                            }),
                        _ => true,
                    }
            }
            (
                CloudReadSelection::Command {
                    identity,
                    command_id,
                },
                CloudReadResult::Command(d),
            ) => {
                exact_data(identity, &d.command.identity)
                    && command_id == &d.command.attempt.command_id
            }
            (
                CloudReadSelection::CommandReceipts { query },
                CloudReadResult::CommandReceipts(d),
            ) => exact_data(&query.identity, &d.identity) && query.command_id == d.command_id,
            (CloudReadSelection::Inputs { identity }, CloudReadResult::Inputs(d)) => {
                exact_data(identity, &d.identity)
            }
            (CloudReadSelection::Observations { query }, CloudReadResult::Observations(d)) => {
                exact_data(&query.identity, &d.identity)
                    && query.execution_revision.value() == d.execution_revision.value()
            }
            (CloudReadSelection::History { query }, CloudReadResult::History(d)) => {
                exact_data(&query.identity, &d.identity)
                    && query.execution_revision.value() == d.execution_revision.value()
            }
            (
                CloudReadSelection::DataQuery {
                    query: DaaSQuery::Telemetry { dataset_id, .. },
                },
                CloudReadResult::QueryResult(DaaSQueryResult::Telemetry {
                    dataset_id: actual, ..
                }),
            ) => dataset_id == actual,
            (
                CloudReadSelection::DataQuery {
                    query:
                        DaaSQuery::Execution {
                            identity,
                            selection,
                            ..
                        },
                },
                CloudReadResult::QueryResult(DaaSQueryResult::Execution {
                    selected_epoch,
                    selected_execution_revision,
                    ..
                }),
            ) => {
                identity.execution_epoch_id == *selected_epoch
                    && match selection {
                        SnapshotSelection::Revision { execution_revision } => {
                            execution_revision.value() == selected_execution_revision.value()
                        }
                        _ => true,
                    }
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(error(
                ErrorCode::IoFailure,
                "Cloud response identity or selection differs from the original request",
                None,
            ))
        }
    }
}

pub(super) enum CloudReadWork {
    Configuration(CloudProductionConfiguration),
    Remote {
        connection: Arc<CloudReadConnection>,
        selection: CloudReadSelection,
    },
}
impl CloudReadWork {
    pub(super) fn run(self, cancelled: &AtomicBool) -> Result<CloudReadResult, ApiError> {
        let result = match self {
            Self::Configuration(c) => Ok(CloudReadResult::Configuration(c)),
            Self::Remote {
                connection,
                selection,
            } => connection.read(&selection),
        };
        if cancelled.load(Ordering::Acquire) {
            Err(error(ErrorCode::Cancelled, "Cloud read cancelled", None))
        } else {
            result
        }
    }
}
pub(super) struct RegisteredCloudRead {
    payload: Value,
    pub(super) task_id: TaskId,
}
impl ProjectHost {
    /// Trusted Native configuration only; arbitrary URLs and credentials are
    /// absent from the public request schema.
    pub fn install_cloud_read_connection(
        &mut self,
        authorizer_ui: &ObjectId,
        connection: CloudReadConnection,
    ) -> Result<(), ApiError> {
        self.require_registered_ui(authorizer_ui)?;
        if self
            .project
            .as_ref()
            .is_none_or(|p| p.id != connection.project_id)
        {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Private cloud connection must name the active project",
                None,
            ));
        }
        self.cloud_connections
            .insert(connection.project_id.clone(), Arc::new(connection));
        Ok(())
    }
    pub(super) fn dispatch_cloud_read(
        &mut self,
        session_id: &ObjectId,
        request: HostCloudReadRequest,
    ) -> Value {
        let request = request.into_inner();
        let scope = request.scope.clone();
        let request_id = request.request_id.clone();
        let result = (|| {
            let caller = self.sessions.get(session_id).cloned().ok_or_else(|| {
                error(
                    ErrorCode::PermissionDenied,
                    "Unknown or revoked session",
                    None,
                )
            })?;
            self.admit(
                &caller,
                &request.project_id,
                &scope,
                &OperationName::ReadCloudProduction,
            )?;
            if !matches!(scope, ApiScope::Project) {
                return Err(unsupported("Cloud reads require Project scope"));
            }
            let key = (caller.session_id.clone(), request_id.clone());
            if self.has_execution_start_request(&key) {
                return Err(error(ErrorCode::InvalidInput, "Original request ID belongs to an execution preparation", None));
            }
            let payload =
                serde_json::to_value(&request).expect("validated cloud request serializes");
            if let Some(registered) = self.cloud_read_requests.get(&key) {
                if registered.payload != payload {
                    return Err(error(
                        ErrorCode::InvalidInput,
                        "Original cloud request ID already has a different request",
                        None,
                    ));
                }
                return CloudReadTaskHandle::try_from(
                    current_task_state(self.owned_task(&caller, &scope, &registered.task_id)?).task,
                )
                .map_err(|_| {
                    error(
                        ErrorCode::InvalidInput,
                        "Original cloud request task no longer matches",
                        None,
                    )
                });
            }
            let project = self.project.as_ref().expect("admitted project");
            let revision = project.revision.clone();
            let project_id = project.id.clone();
            let work = match request.params.selection {
                CloudReadSelection::Configuration {
                    base_revision,
                    scenario_id,
                    scenario_revision,
                } => {
                    check_revision(&base_revision, &revision, "/params/selection/base_revision")?;
                    let scenario = self.scenarios.get(&scenario_id).ok_or_else(|| {
                        error(
                            ErrorCode::MissingReference,
                            "Scenario is not registered in this Host",
                            None,
                        )
                    })?;
                    check_revision(
                        &scenario_revision,
                        &scenario.revision,
                        "/params/selection/scenario_revision",
                    )?;
                    let configured = self.cloud_connections.get(&project_id);
                    let issues = if configured.is_none() {
                        vec![Issue {
                            code: "MissingCloudConfiguration".into(),
                            object_ref: None,
                            field_path: None,
                            message: "尚未配置云连接与设备绑定".into(),
                            blocks: vec![],
                        }]
                    } else {
                        vec![]
                    };
                    CloudReadWork::Configuration(CloudProductionConfiguration {
                        project_revision: revision.clone(),
                        scenario_id,
                        scenario_revision,
                        cloud_connection_ref: configured.map(|c| c.connection_ref.clone()),
                        cloud_project_id: configured.map(|c| c.cloud_project_id.clone()),
                        source_mode: CloudSourceMode::SoftwareInLoop,
                        candidate_bindings: configured
                            .map(|c| c.candidates.clone())
                            .unwrap_or_default(),
                        issues,
                    })
                }
                selection => CloudReadWork::Remote {
                    connection: self
                        .cloud_connections
                        .get(&project_id)
                        .cloned()
                        .ok_or_else(|| {
                            unsupported("The active project has no private cloud connection")
                        })?,
                    selection,
                },
            };
            let task = self.enqueue_io(
                &caller,
                TaskScope::Edit(EditScope::Project(project_id)),
                OperationName::ReadCloudProduction,
                revision,
                None,
                IoInput::CloudRead(work),
            );
            self.tasks
                .get_mut(&task.id)
                .unwrap()
                .state
                .originating_request_id = Some(request_id.clone());
            self.cloud_read_requests.insert(
                key,
                RegisteredCloudRead {
                    payload,
                    task_id: task.id.clone(),
                },
            );
            CloudReadTaskHandle::try_from(task).map_err(|_| {
                error(
                    ErrorCode::InvalidInput,
                    "Cloud read task registration failed",
                    None,
                )
            })
        })();
        self.respond(request_id, session_id, &scope, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn id(s: &str) -> ObjectId {
        ObjectId::try_from(s.to_owned()).unwrap()
    }
    #[test]
    fn snapshot_and_inputs_get_requests_carry_the_exact_original_execution_identity() {
        let connection = CloudReadConnection::https(id("connection"),
            ProjectId::try_from("example".to_owned()).unwrap(),
            CloudProjectId::try_from("cloud".to_owned()).unwrap(),
            "https://cloud-read-component.invalid/".into(), "test-reader".into(), vec![]).unwrap();
        let identity: ExecutionIdentity = serde_json::from_value(json!({
            "project_id":"example","host_context_session_id":"original-host",
            "session_id":"original:session","execution_epoch_id":"original:epoch"
        })).unwrap();
        for selection in [
            CloudReadSelection::Snapshot {identity:identity.clone(),selection:SnapshotSelection::Latest {}},
            CloudReadSelection::Snapshot {identity:identity.clone(),selection:SnapshotSelection::Cutoff {}},
            CloudReadSelection::Snapshot {identity:identity.clone(),selection:SnapshotSelection::Revision {execution_revision:7u64.try_into().unwrap()}},
            CloudReadSelection::Inputs {identity:identity.clone()},
        ] {
            let request = connection.read_request(&selection).unwrap().build().unwrap();
            assert_eq!(request.method(), Method::GET);
            let values: Vec<_> = request.url().query_pairs().filter(|(key,_)|key=="identity").collect();
            assert_eq!(values.len(),1,"GET must not omit or duplicate identity");
            assert_eq!(serde_json::from_str::<Value>(&values[0].1).unwrap(),serde_json::to_value(&identity).unwrap());
        }
    }
    #[test]
    fn private_connection_accepts_only_fixed_origins_and_encodes_opaque_path_ids() {
        let make = |endpoint: &str, local: bool| {
            let args = (
                id("connection"),
                ProjectId::try_from("example".to_owned()).unwrap(),
                CloudProjectId::try_from("cloud".to_owned()).unwrap(),
                endpoint.to_owned(),
                "test-reader".to_owned(),
                vec![],
            );
            if local {
                CloudReadConnection::local_loopback(args.0, args.1, args.2, args.3, args.4, args.5)
            } else {
                CloudReadConnection::https(args.0, args.1, args.2, args.3, args.4, args.5)
            }
        };
        for endpoint in [
            "http://example.invalid/",
            "https://user:password@example.invalid/",
            "https://example.invalid/path",
            "https://example.invalid/?token=secret",
        ] {
            assert!(make(endpoint, false).is_err(), "{endpoint}");
        }
        assert!(make("http://localhost:23270/", true).is_err());
        let local = make("http://127.0.0.1:23270/", true).unwrap();
        assert_eq!(
            local.url(&["v2", "commands", "one/two"]).path(),
            "/v2/commands/one%2Ftwo"
        );
    }
    #[test]
    fn response_selection_cannot_substitute_another_execution_revision_or_unproved_cutoff() {
        let cases: Value = serde_json::from_str(include_str!(
            "../../domain/tests/fixtures/cloud_wire_cases.json"
        ))
        .unwrap();
        let sample = |name: &str, label: &str| {
            cases
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["definition"] == name && v["label"] == label)
                .unwrap()["value"]
                .clone()
        };
        let connection = CloudReadConnection::local_loopback(
            id("connection"),
            ProjectId::try_from("example".to_owned()).unwrap(),
            CloudProjectId::try_from("cloud".to_owned()).unwrap(),
            "http://127.0.0.1:23270/".into(),
            "test-reader".into(),
            vec![],
        )
        .unwrap();
        let mut query = sample("DaaSQuery", "union-branch-1");
        query["selection"] = json!({"kind":"revision","execution_revision":7});
        let selection = CloudReadSelection::DataQuery {
            query: serde_json::from_value(query).unwrap(),
        };
        let mut data = sample("DaaSQueryResult", "union-branch-1");
        data["selected_execution_revision"] = json!(7);
        let result = CloudReadResult::QueryResult(serde_json::from_value(data.clone()).unwrap());
        assert!(connection.check_result(&selection, &result).is_ok());
        data["selected_execution_revision"] = json!(8);
        let wrong = CloudReadResult::QueryResult(serde_json::from_value(data).unwrap());
        assert!(connection.check_result(&selection, &wrong).is_err());
        let mut envelope = sample("ExecutionReadEnvelope", "positive");
        let identity = serde_json::from_value(envelope["snapshot"]["identity"].clone()).unwrap();
        let cutoff = CloudReadSelection::Snapshot {
            identity,
            selection: SnapshotSelection::Cutoff {},
        };
        let missing = CloudReadResult::Snapshot(serde_json::from_value(envelope.clone()).unwrap());
        assert!(connection.check_result(&cutoff, &missing).is_err());
        envelope["snapshot"]["coverage"]["cutoff_snapshot_revision"] =
            envelope["snapshot"]["execution_revision"].clone();
        let pinned = CloudReadResult::Snapshot(serde_json::from_value(envelope).unwrap());
        assert!(connection.check_result(&cutoff, &pinned).is_ok());
    }
    fn setup() -> (ProjectHost, ObjectId, ProjectId, ScenarioId) {
        let ui = id("cloud-ui");
        let mut host = ProjectHost::new(id("cloud-host"), ui.clone());
        let response=host.dispatch(&ui,serde_json::from_value(json!({"api_version":"0.1","request_id":"create-cloud-project","project_id":null,"scope":{"kind":"workspace"},"operation":"apply_commands","params":{"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"cloud-read host component","time_reference":{"label":"component","origin_iso8601":null,"timezone":null},"departure":"stay"}}]}})).unwrap());
        assert_eq!(response["outcome"]["status"], "ok", "{response}");
        let project = host.project.as_ref().unwrap().id.clone();
        let cases: Value = serde_json::from_str(include_str!(
            "../../domain/tests/fixtures/cloud_wire_cases.json"
        ))
        .unwrap();
        let case = cases
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["definition"] == "ExecutionFrozenInputs" && x["label"] == "positive")
            .unwrap();
        let mut scenario: ScenarioInput =
            serde_json::from_value(case["value"]["scenario"].clone()).unwrap();
        scenario.id = ScenarioId::try_from("cloud-metadata-scenario".to_owned()).unwrap();
        scenario.project_revision = host.project.as_ref().unwrap().revision.clone();
        let scenario_id = scenario.id.clone();
        host.scenarios.insert(scenario_id.clone(), scenario);
        (host, ui, project, scenario_id)
    }
    fn configuration_request(
        host: &ProjectHost,
        project: &ProjectId,
        scenario: &ScenarioId,
        request_id: &str,
    ) -> Value {
        json!({"api_version":"0.1","request_id":request_id,"project_id":project,"scope":{"kind":"project"},"operation":"read_cloud_production","params":{"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":{"kind":"configuration","base_revision":host.project.as_ref().unwrap().revision,"scenario_id":scenario,"scenario_revision":host.scenarios[scenario].revision}}})
    }
    fn task_state(
        host: &mut ProjectHost,
        ui: &ObjectId,
        project: &ProjectId,
        task: &TaskId,
    ) -> Value {
        let request=serde_json::from_value(json!({"api_version":"0.1","request_id":"read-original-task","project_id":project,"scope":{"kind":"project"},"operation":"get_task","params":{"task_id":task}})).unwrap();
        host.dispatch(ui, request)["outcome"]["data"].clone()
    }
    #[test]
    fn configuration_read_uses_real_task_registry_and_exact_original_request() {
        let (mut host, ui, project, scenario) = setup();
        let payload = configuration_request(&host, &project, &scenario, "read-config");
        let first = host.dispatch(&ui, serde_json::from_value(payload.clone()).unwrap());
        let handle: CloudReadTaskHandle =
            serde_json::from_value(first["outcome"]["data"].clone()).unwrap();
        let repeated = host.dispatch(&ui, serde_json::from_value(payload).unwrap());
        assert_eq!(first, repeated);
        let work = host.drain_io_work();
        assert_eq!(work.len(), 1);
        let revision = host.project.as_ref().unwrap().revision.0.value();
        let generation = host.content_generation;
        host.finish_io(work.into_iter().next().unwrap().run());
        let value = task_state(&mut host, &ui, &project, &handle.id);
        let strict: crate::execution_api::CloudReadTaskState =
            serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(strict.task.state, ComputationStatus::Completed));
        assert_eq!(strict.originating_request_id, id("read-config"));
        assert_eq!(value["result"]["kind"], "cloud_read");
        assert_eq!(value["result"]["data"]["kind"], "configuration");
        assert_eq!(
            value["result"]["data"]["data"]["cloud_project_id"],
            Value::Null
        );
        assert_eq!(
            value["result"]["data"]["data"]["issues"][0]["code"],
            "MissingCloudConfiguration"
        );
        assert_eq!(host.project.as_ref().unwrap().revision.0.value(), revision);
        assert_eq!(host.content_generation, generation);
        let mut changed = configuration_request(&host, &project, &scenario, "read-config");
        changed["params"]["selection"] = json!({"kind":"dataset_catalog"});
        let result = host.dispatch(&ui, serde_json::from_value(changed).unwrap());
        assert_eq!(result["outcome"]["data"]["code"], "InvalidInput");
        assert!(host.drain_io_work().is_empty());
    }
    #[test]
    fn new_tasks_cannot_escape_strict_view_through_legacy_operation_or_missing_nulls() {
        let (mut host, ui, project, scenario) = setup();
        let payload = configuration_request(&host, &project, &scenario, "strict-task");
        let response = host.dispatch(&ui, serde_json::from_value(payload).unwrap());
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let state = task_state(&mut host, &ui, &project, &task.id);
        assert!(serde_json::from_value::<TaskState>(state.clone()).is_ok());
        for field in ["originating_request_id", "result", "failure"] {
            let mut bad = state.clone();
            bad.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<TaskState>(bad).is_err(), "{field}");
        }
        let mut bad = state.clone();
        bad["originating_operation"] = json!("run_scenario");
        assert!(serde_json::from_value::<TaskState>(bad.clone()).is_err());
        bad["task"]["kind"] = json!("simulation");
        bad.as_object_mut()
            .unwrap()
            .remove("originating_request_id");
        assert!(
            serde_json::from_value::<TaskState>(bad).is_ok(),
            "legacy task still accepts its old shape"
        );
        let mut missing = state;
        missing["task"].as_object_mut().unwrap().remove("progress");
        assert!(serde_json::from_value::<TaskState>(missing).is_err());
    }
    #[test]
    fn request_scope_and_caller_authority_are_checked_before_any_read_work() {
        let (mut host, ui, project, scenario) = setup();
        let mut payload = configuration_request(&host, &project, &scenario, "read-scope");
        payload["scope"] = json!({"kind":"workspace"});
        assert!(serde_json::from_value::<ApiRequest>(payload.clone()).is_err());
        payload["scope"] = json!({"kind":"project"});
        let result = host.dispatch(
            &id("unknown-agent"),
            serde_json::from_value(payload.clone()).unwrap(),
        );
        assert_eq!(result["outcome"]["data"]["code"], "PermissionDenied");
        assert!(host.drain_io_work().is_empty());
        payload["project_id"] = json!("another-project");
        let result = host.dispatch(&ui, serde_json::from_value(payload).unwrap());
        assert_eq!(result["outcome"]["data"]["code"], "PermissionDenied");
        assert!(host.drain_io_work().is_empty());
    }
    #[test]
    fn queued_cloud_read_cancellation_preserves_original_request_and_has_no_result() {
        let (mut host, ui, project, scenario) = setup();
        let payload = configuration_request(&host, &project, &scenario, "cancel-read");
        let response = host.dispatch(&ui, serde_json::from_value(payload).unwrap());
        let task: TaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
        let cancel=serde_json::from_value(json!({"api_version":"0.1","request_id":"cancel","project_id":project,"scope":{"kind":"project"},"operation":"cancel_task","params":{"task_id":task.id,"action":"cancel"}})).unwrap();
        let result = host.dispatch(&ui, cancel);
        assert_eq!(result["outcome"]["status"], "ok");
        for work in host.drain_io_work() {
            host.finish_io(work.run());
        }
        let state = task_state(&mut host, &ui, &project, &task.id);
        assert_eq!(state["task"]["state"], "cancelled");
        assert_eq!(state["result"], Value::Null);
        assert_eq!(state["originating_request_id"], "cancel-read");
        assert!(serde_json::from_value::<TaskState>(state).is_ok());
    }

    #[test]
    #[ignore = "requires the owned real HTTP service and private scoped reader environment"]
    fn actual_loopback_cloud_reads_complete_in_the_original_host_task_registry() {
        let endpoint = std::env::var("SF_V2_HOST_READ_ENDPOINT").unwrap();
        let token = std::env::var("SF_V2_HOST_READER_TOKEN").unwrap();
        let (mut host, ui, project, _) = setup();
        let connection = CloudReadConnection::local_loopback(id("actual-local-cloud"),
            project.clone(), CloudProjectId::try_from("example".to_owned()).unwrap(),
            endpoint, token.clone(), vec![]).unwrap();
        host.install_cloud_read_connection(&ui, connection).unwrap();
        let revision = host.project.as_ref().unwrap().revision.clone();
        let generation = host.content_generation;
        let selections = [json!({"kind":"dataset_catalog"}), json!({"kind":"active_execution"}),
            json!({"kind":"data_query","query":{"dataset":"telemetry","contract_version":EXECUTION_CONTRACT_VERSION,
                "cloud_project_id":"example","dataset_id":"load-fixture","device_ids":["G000"],
                "received_from":"2020-01-01T00:00:00.000000Z","received_until":"2090-01-01T00:00:00.000000Z",
                "aggregation":"rows","page":{"limit":200,"cursor":null}}})];
        for (n, selection) in selections.into_iter().enumerate() {
            let request_id = format!("actual-cloud-read-{n}");
            let payload = json!({"api_version":"0.1","request_id":request_id,"project_id":project,
                "scope":{"kind":"project"},"operation":"read_cloud_production",
                "params":{"execution_contract_version":EXECUTION_CONTRACT_VERSION,"selection":selection}});
            let response = host.dispatch(&ui, serde_json::from_value(payload).unwrap());
            let task: CloudReadTaskHandle = serde_json::from_value(response["outcome"]["data"].clone()).unwrap();
            let work = host.drain_io_work();
            assert_eq!(work.len(), 1);
            host.finish_io(work.into_iter().next().unwrap().run());
            let state = task_state(&mut host, &ui, &project, &task.id);
            assert_eq!(state["task"]["state"], "completed", "{state}");
            assert_eq!(state["originating_request_id"], request_id);
            assert!(!state.to_string().contains(&token));
            let data = &state["result"]["data"]["data"];
            if n == 0 { assert!(data["datasets"].as_array().unwrap().iter().any(|d| d["dataset_id"] == "load-fixture")); }
            if n == 2 {
                assert_eq!(data["selected_visible_frontier"], 1_000_000);
                assert_eq!(data["rows"].as_array().unwrap().len(), 200);
                assert!(data["next_cursor"].is_string());
            }
            assert_eq!(host.project.as_ref().unwrap().revision.0.value(), revision.0.value());
            assert_eq!(host.content_generation, generation);
        }
    }
}
