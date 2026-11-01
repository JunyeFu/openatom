//! Native IPC adapter over the single Tauri-managed ProjectApiState.
//! The main window authorizes a verified OS peer; requests cannot carry grants.
use crate::project_api::ProjectApiState;
use serde::Deserialize;
use serde_json::{Value, json};
use sf_commands::*;
use sf_domain::*;
use std::{
    collections::BTreeMap,
    io,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri_plugin_dialog::DialogExt;
use tokio::io::AsyncReadExt;
use tokio::{
    net::windows::named_pipe::NamedPipeServer,
    sync::{Notify, oneshot},
};

#[path = "../../../../integrations/agent/runtime/contract.rs"]
mod contract;
#[path = "../../../../integrations/agent/runtime/framing.rs"]
mod framing;
#[path = "../../../../integrations/agent/runtime/managed_io.rs"]
mod managed_io;
#[path = "../../../../integrations/agent/runtime/pipe_os.rs"]
mod pipe_os;

struct Connection {
    pid: u32,
    os_session: u32,
    authorized: bool,
    revoked: bool,
    authorize: Option<oneshot::Sender<()>>,
    close: Arc<Notify>,
}
#[derive(Clone)]
pub struct AgentPipeState {
    pub(super) project: ProjectApiState,
    endpoint: String,
    connections: Arc<Mutex<BTreeMap<String, Connection>>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldChoice {
    pointer: String,
    element_id: Option<ObjectId>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectChoice {
    target: ObjectRef,
    fields: Vec<FieldChoice>,
    can_remove: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateChoice {
    kind: ObjectKind,
    max_new_objects: SafeUint,
    fields: Vec<FieldChoice>,
}
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchChoice {
    AdoptArtifact,
    CreateProject,
    PutObject,
    RemoveObject,
    SetProjectSettings,
    CreateDefinitionDraft,
    PutDefinition,
    SetDeclaredParameter,
    AdoptModule,
    InstantiateTemplate,
    PutPackagingJob,
    RecordArtifactAcknowledgment,
    AdoptImportedDraft,
    CommitDefinitionVersion,
    Undo,
    Redo,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantChoice {
    scope: TaskScope,
    branches: Vec<BranchChoice>,
    objects: Vec<ObjectChoice>,
    creates: Vec<CreateChoice>,
    source_versions: Vec<ModuleRef>,
    reusable_resources: Vec<ObjectId>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityChoice {
    project_id: Option<ProjectId>,
    operations: Vec<OperationName>,
    definition_drafts: Vec<ModuleId>,
    #[serde(default)]
    allow_view_control: bool,
    edit: Vec<GrantChoice>,
}
impl AuthorityChoice {
    fn into_context(self, session_id: ObjectId) -> HostCallContext {
        let session = session_id.as_str();
        let edit_grants = self
            .edit
            .into_iter()
            .enumerate()
            .map(|(index, g)| HostEditGrant {
                id: ObjectId::try_from(format!("{session}-grant-{index}")).unwrap(),
                scope: g.scope,
                branches: g.branches.into_iter().map(branch).collect(),
                objects: g
                    .objects
                    .into_iter()
                    .map(|o| HostObjectEditGrant {
                        target: o.target,
                        fields: fields(o.fields),
                        can_remove: o.can_remove,
                    })
                    .collect(),
                creates: g
                    .creates
                    .into_iter()
                    .map(|c| HostCreateAllowance {
                        kind: c.kind,
                        max_new_objects: c.max_new_objects,
                        fields: fields(c.fields),
                    })
                    .collect(),
                source_versions: g.source_versions,
                reusable_resources: g.reusable_resources,
                created_objects: vec![],
            })
            .collect();
        HostCallContext {
            origin: CallerOrigin::ExternalAgent,
            allow_view_control: self.allow_view_control,
            session_id,
            granted_project: self.project_id,
            allowed_operations: self.operations,
            allowed_definition_drafts: self.definition_drafts,
            selected_file_tokens: vec![],
            edit_grants,
        }
    }
}

fn fields(choices: Vec<FieldChoice>) -> Vec<HostFieldGrant> {
    choices
        .into_iter()
        .map(|f| HostFieldGrant {
            pointer: f.pointer,
            element_id: f.element_id,
        })
        .collect()
}
fn branch(choice: BranchChoice) -> HostEditBranch {
    match choice {
        BranchChoice::AdoptArtifact => HostEditBranch::AdoptArtifact,
        BranchChoice::CreateProject => HostEditBranch::CreateProject,
        BranchChoice::PutObject => HostEditBranch::PutObject,
        BranchChoice::RemoveObject => HostEditBranch::RemoveObject,
        BranchChoice::SetProjectSettings => HostEditBranch::SetProjectSettings,
        BranchChoice::CreateDefinitionDraft => HostEditBranch::CreateDefinitionDraft,
        BranchChoice::PutDefinition => HostEditBranch::PutDefinition,
        BranchChoice::SetDeclaredParameter => HostEditBranch::SetDeclaredParameter,
        BranchChoice::AdoptModule => HostEditBranch::AdoptModule,
        BranchChoice::InstantiateTemplate => HostEditBranch::InstantiateTemplate,
        BranchChoice::PutPackagingJob => HostEditBranch::PutPackagingJob,
        BranchChoice::RecordArtifactAcknowledgment => HostEditBranch::RecordArtifactAcknowledgment,
        BranchChoice::AdoptImportedDraft => HostEditBranch::AdoptImportedDraft,
        BranchChoice::CommitDefinitionVersion => HostEditBranch::CommitDefinitionVersion,
        BranchChoice::Undo => HostEditBranch::Undo,
        BranchChoice::Redo => HostEditBranch::Redo,
    }
}
fn failure(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}

impl AgentPipeState {
    pub fn start(project: ProjectApiState) -> io::Result<Self> {
        let identity = pipe_os::current_identity()?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let state = Self {
            project,
            endpoint: format!(r"\\.\pipe\superfactory-{}-{nonce}", std::process::id()),
            connections: Arc::new(Mutex::new(BTreeMap::new())),
        };
        let server = tauri::async_runtime::block_on(async {
            pipe_os::create_server(&state.endpoint, true, &identity)
        })?;
        let listener = state.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = listener.listen(server, identity).await {
                eprintln!("Agent pipe listener stopped: {error}");
            }
        });
        Ok(state)
    }
    async fn listen(
        self,
        mut server: NamedPipeServer,
        identity: pipe_os::Identity,
    ) -> io::Result<()> {
        loop {
            server.connect().await?;
            // Keep a listening instance before dispatching this connection.
            let next = pipe_os::create_server(&self.endpoint, false, &identity)?;
            let state = self.clone();
            let local = identity.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = state.serve(server, local).await {
                    eprintln!("Agent pipe connection closed: {error}");
                }
            });
            server = next;
        }
    }
    async fn serve(
        &self,
        mut pipe: NamedPipeServer,
        identity: pipe_os::Identity,
    ) -> io::Result<()> {
        let peer = pipe_os::pipe_peer(&pipe, true, &identity)?;
        let hello: IpcHello = serde_json::from_value(framing::read_frame(&mut pipe).await?)
            .map_err(io::Error::other)?;
        if serde_json::to_value(hello.expected_host_instance_id).map_err(io::Error::other)?
            != json!(self.project.host_instance_id)
        {
            return Err(failure("Expected Host instance differs"));
        }
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let session = format!(
            "agent-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        let (authorize, authorized) = oneshot::channel();
        let close = Arc::new(Notify::new());
        self.connections
            .lock()
            .map_err(|_| io::Error::other("Connection registry poisoned"))?
            .insert(
                session.clone(),
                Connection {
                    pid: peer.identity.pid,
                    os_session: peer.identity.session,
                    authorized: false,
                    revoked: false,
                    authorize: Some(authorize),
                    close: close.clone(),
                },
            );
        let result=async {
            let mut premature=[0u8;1];
            tokio::select! {
                result=authorized=>result.map_err(io::Error::other)?,
                _=close.notified()=>return Err(failure("Desktop authorization revoked")),
                result=pipe.read(&mut premature)=>{
                    result?;
                    return Err(failure("Peer disconnected or sent a request before desktop authorization"));
                }
            }
            framing::write_frame(&mut pipe,&json!({"wire_version":"1","host_instance_id":self.project.host_instance_id,"client_session_id":session,"api_version":"0.1","max_frame_bytes":framing::MAX_FRAME_BYTES})).await?;
            loop {
                let value=tokio::select! {result=framing::read_frame(&mut pipe)=>result?,_=close.notified()=>return Err(failure("Desktop authorization revoked"))};
                let frame:IpcRequestFrame=serde_json::from_value(value.clone()).map_err(io::Error::other)?;
                if value["host_instance_id"]!=json!(self.project.host_instance_id) || value["client_session_id"]!=session {return Err(failure("IPC request session differs"));}
                let reply={
                    let connections=self.connections.lock().map_err(|_|io::Error::other("Connection registry poisoned"))?;
                    let connection=connections.get(&session).ok_or_else(||failure("Session no longer connected"))?;
                    if !connection.authorized || connection.revoked {return Err(failure("Session has no current desktop authority"));}
                    self.project.dispatch_session(&frame.client_session_id,frame.request)
                };
                let operation:OperationName=serde_json::from_value(value["request"]["operation"].clone()).map_err(io::Error::other)?;
                contract::validate_response(&operation,&reply)?;
                let image=self.project.rendered_task_image(&ObjectId::try_from(session.clone()).map_err(io::Error::other)?,&value["request"],&reply).map_err(|e|failure(&e.message))?;
                let mut response=json!({"wire_version":"1","host_instance_id":self.project.host_instance_id,"client_session_id":session,"originating_operation":operation,"reply":reply,"managed_assets":null});
                if let Some((task,output,image))=image {
                    let handle=managed_io::publish_readonly(&image.bytes,&peer,&identity)?;
                    response["managed_assets"]=json!([{"task_id":task.task.id,"asset_id":image.asset.id,"scope":output.requested_scope,"scope_revision":output.requested_scope_revision,"mime_type":image.mime_type,"byte_length":image.bytes.len(),"handle_value":handle}]);
                }
                if serde_json::to_vec(&response).map_err(io::Error::other)?.len()>framing::MAX_FRAME_BYTES as usize {
                    let bytes=serde_json::to_vec(&response["reply"]).map_err(io::Error::other)?;
                    let handle=managed_io::publish_readonly(&bytes,&peer,&identity)?;
                    let id=format!("{session}-result-{}",NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed));
                    response["reply"]["outcome"]=json!({"status":"managed","data":{"result_id":id,"response_schema_ref":contract::response_schema_ref(&operation),"byte_length":bytes.len()}});
                    response["managed_results"]=json!([{"result_id":id,"handle_value":handle}]);
                }
                framing::write_frame(&mut pipe,&response).await?;
            }
        }.await;
        let mut connections = self
            .connections
            .lock()
            .map_err(|_| io::Error::other("Connection registry poisoned"))?;
        if let Some(connection) = connections.remove(&session) {
            if connection.authorized && !connection.revoked {
                self.project
                    .revoke_external(&ObjectId::try_from(session).map_err(io::Error::other)?)
                    .map_err(|error| failure(&error.message))?;
            }
        }
        result
    }
    pub(super) fn info(&self) -> io::Result<Value> {
        let connections = self
            .connections
            .lock()
            .map_err(|_| io::Error::other("Connection registry poisoned"))?;
        Ok(
            json!({"endpoint":self.endpoint,"host_instance_id":self.project.host_instance_id,"connections":connections.iter().map(|(id,c)|json!({"session_id":id,"pid":c.pid,"os_session":c.os_session,"authorized":c.authorized,"revoked":c.revoked})).collect::<Vec<_>>()}),
        )
    }
    pub(super) fn authorize(&self, session: &str, choice: AuthorityChoice) -> io::Result<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|_| io::Error::other("Connection registry poisoned"))?;
        let connection = connections
            .get_mut(session)
            .ok_or_else(|| failure("Verified pending peer not found"))?;
        if connection.authorized || connection.revoked {
            return Err(failure(
                "Existing authority requires explicit revoke and a fresh connection",
            ));
        }
        let session_id = ObjectId::try_from(session.to_owned()).map_err(io::Error::other)?;
        self.project
            .register_external(choice.into_context(session_id))
            .map_err(|e| failure(&e.message))?;
        connection.authorized = true;
        connection
            .authorize
            .take()
            .ok_or_else(|| failure("Pending authorization missing"))?
            .send(())
            .map_err(|_| failure("Peer disconnected during authorization"))
    }
    pub(super) fn update_authority(
        &self,
        session: &str,
        choice: AuthorityChoice,
    ) -> io::Result<()> {
        let connections = self
            .connections
            .lock()
            .map_err(|_| io::Error::other("Connection registry poisoned"))?;
        let connection = connections
            .get(session)
            .ok_or_else(|| failure("Connected peer not found"))?;
        if !connection.authorized || connection.revoked {
            return Err(failure("Connection has no current native authorization"));
        }
        self.project
            .update_external_authority(
                choice.into_context(
                    ObjectId::try_from(session.to_owned()).map_err(io::Error::other)?,
                ),
            )
            .map_err(|error| failure(&error.message))
    }
    pub(super) fn grant_task_access(
        &self,
        session: &str,
        task: &TaskId,
        allow_controls: bool,
    ) -> io::Result<()> {
        let connections = self
            .connections
            .lock()
            .map_err(|_| io::Error::other("Connection registry poisoned"))?;
        let connection = connections
            .get(session)
            .ok_or_else(|| failure("Connected peer not found"))?;
        if !connection.authorized || connection.revoked {
            return Err(failure("Connection has no current native authorization"));
        }
        self.project
            .grant_external_task(
                &ObjectId::try_from(session.to_owned()).map_err(io::Error::other)?,
                task,
                allow_controls,
            )
            .map_err(|error| failure(&error.message))
    }
    fn revoke(&self, session: &str) -> io::Result<()> {
        let mut connections = self
            .connections
            .lock()
            .map_err(|_| io::Error::other("Connection registry poisoned"))?;
        let connection = connections
            .get_mut(session)
            .ok_or_else(|| failure("Connected peer not found"))?;
        if connection.authorized && !connection.revoked {
            self.project
                .revoke_external(&ObjectId::try_from(session.to_owned()).map_err(io::Error::other)?)
                .map_err(|error| failure(&error.message))?;
        }
        connection.revoked = true;
        connection.close.notify_one();
        Ok(())
    }
}
fn main_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("Only the main desktop window may administer Agent connections".to_owned())
    }
}
#[tauri::command]
pub fn agent_connection_info(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AgentPipeState>,
) -> Result<Value, String> {
    main_window(&window)?;
    state.info().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn agent_authorize_connection(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AgentPipeState>,
    session_id: String,
    choice: AuthorityChoice,
) -> Result<(), String> {
    main_window(&window)?;
    state
        .authorize(&session_id, choice)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn agent_update_connection_authority(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AgentPipeState>,
    session_id: String,
    choice: AuthorityChoice,
) -> Result<(), String> {
    main_window(&window)?;
    state
        .update_authority(&session_id, choice)
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub fn agent_grant_task_access(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AgentPipeState>,
    session_id: String,
    task_id: TaskId,
    allow_controls: bool,
) -> Result<(), String> {
    main_window(&window)?;
    state
        .grant_task_access(&session_id, &task_id, allow_controls)
        .map_err(|error| error.to_string())
}
#[tauri::command]
pub fn agent_revoke_connection(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, AgentPipeState>,
    session_id: String,
) -> Result<(), String> {
    main_window(&window)?;
    state.revoke(&session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_select_directory(
    window: tauri::WebviewWindow,
    app: tauri::AppHandle,
    state: tauri::State<'_, AgentPipeState>,
    session_id: String,
    purpose: FilePurpose,
) -> Result<SelectedFileRef, String> {
    main_window(&window)?;
    let title = crate::project_api::directory_dialog_title(window.label(), &purpose)
        .map_err(|error| error.message)?;
    #[cfg(all(debug_assertions, feature = "nfr-devtools"))]
    let configured = crate::project_api::configured_test_directory(&purpose).map_err(|error| error.message)?;
    #[cfg(not(all(debug_assertions, feature = "nfr-devtools")))]
    let configured: Option<std::path::PathBuf> = None;
    let path = if let Some(path) = configured { path } else {
    let (sender, receiver) = oneshot::channel();
    app.dialog()
        .file()
        .set_parent(&window)
        .set_title(format!("为此 Agent：{title}"))
        .pick_folder(move |path| {
            let _ = sender.send(path);
        });
    receiver
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Directory selection cancelled".to_owned())?
        .into_path()
        .map_err(|e| e.to_string())?
    };
    let connections = state
        .connections
        .lock()
        .map_err(|_| "Connection registry poisoned".to_owned())?;
    let connection = connections
        .get(&session_id)
        .ok_or_else(|| "Verified connection no longer exists".to_owned())?;
    if !connection.authorized || connection.revoked {
        return Err("Connection has no current authorization".to_owned());
    }
    state
        .project
        .select_external_directory(
            &ObjectId::try_from(session_id).map_err(|e| e.to_string())?,
            purpose,
            path,
        )
        .map_err(|e| e.message)
}

/// Startup-only actual-process consumer of this Tauri-managed state. No model,
/// network port, global Codex configuration or second Host is involved.
pub fn smoke(state: &AgentPipeState, app: tauri::AppHandle) -> io::Result<bool> {
    use std::os::windows::process::CommandExt;
    use std::{
        io::Read,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    use tauri::Manager;
    let root = std::env::current_dir()?;
    let output = root.join("tests/delivery/mcp/output").join(format!(
        "native-process-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos()
    ));
    std::fs::create_dir_all(&output)?;
    let request = |id: &str, project: Value, operation: &str, params: Value| json!({"api_version":"0.1","request_id":id,"project_id":project,"scope":{"kind":if project.is_null(){"workspace"}else{"project"}},"operation":operation,"params":params});
    let created=state.project.dispatch_ui("main",request("mcp-native-create",Value::Null,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"L".repeat(1_200_000),"time_reference":{"label":"actual MCP process","origin_iso8601":null,"timezone":"Asia/Shanghai"},"departure":"stay"}}]})));
    if created["outcome"]["status"] != "ok" {
        return Err(io::Error::other(format!(
            "Native UI seed failed: {created}"
        )));
    }
    let project = created["outcome"]["data"]["project_id"].clone();
    let python = std::env::var_os("SUPERFACTORY_TEST_PYTHON").ok_or_else(|| {
        io::Error::other("SUPERFACTORY_TEST_PYTHON must name the already installed test runtime")
    })?;
    let driver = root.join("tests/delivery/mcp/process_client.py");
    let bridge = std::env::current_exe()?.with_file_name("superfactory-mcp-bridge.exe");
    let mut child = Command::new(python)
        .arg("-B")
        .arg("-X")
        .arg("utf8")
        .arg(driver)
        .arg(bridge)
        .arg(&state.endpoint)
        .arg(state.project.host_instance_id.as_str())
        .arg(project.as_str().unwrap())
        .arg(&output)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000)
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout = std::thread::spawn(move || {
        let mut text = String::new();
        let mut stream = stdout;
        stream.read_to_string(&mut text).map(|_| text)
    });
    let stderr = std::thread::spawn(move || {
        let mut text = String::new();
        let mut stream = stderr;
        stream.read_to_string(&mut text).map(|_| text)
    });
    let start = Instant::now();
    let mut admitted = 0;
    let mut administration = Vec::new();
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| io::Error::other("Managed main window missing"))?;
    let secondary = tauri::WebviewWindowBuilder::new(
        &app,
        "agent-permission-probe",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .visible(false)
    .build()
    .map_err(io::Error::other)?;
    let mut initial_choices = BTreeMap::<String, Value>::new();
    let mut native_actions = Vec::new();
    let mut action_index = 1u64;
    let mut switched_project = None;
    let status = loop {
        let pending: Vec<String> = state
            .connections
            .lock()
            .map_err(|_| io::Error::other("Registry poisoned"))?
            .iter()
            .filter(|(_, c)| !c.authorized && !c.revoked)
            .map(|(id, _)| id.clone())
            .collect();
        for session in pending {
            let editable = admitted == 1;
            let fields = |names: &[&str]| {
                names
                    .iter()
                    .map(|name| json!({"pointer":name,"element_id":null}))
                    .collect::<Vec<_>>()
            };
            let edit = if editable {
                json!([{"scope":{"kind":"edit","data":{"kind":"project","data":project}},"branches":["set_project_settings","put_object"],"objects":[{"target":{"kind":"project","id":project},"fields":fields(&["/title"]),"can_remove":false}],"creates":[{"kind":"item_type","max_new_objects":1,"fields":fields(&["/revision","/title","/unit","/tracking","/spec_ref"])},{"kind":"scenario","max_new_objects":1,"fields":fields(&["/revision","/project_revision","/title","/time_reference/label","/time_reference/origin_iso8601","/time_reference/timezone","/window/start_ms","/window/end_ms"])}],"source_versions":[],"reusable_resources":[]}])
            } else {
                json!([])
            };
            let choice_value = json!({"project_id":project,"operations":["describe_capabilities","get_context","get_object","apply_commands","save_project","get_task","cancel_task","query_module_library","check_factory_draft","run_scenario","get_run","compare_runs"],"definition_drafts":[],"edit":edit});
            let choice: AuthorityChoice =
                serde_json::from_value(choice_value.clone()).map_err(io::Error::other)?;
            state.authorize(&session, choice)?;
            initial_choices.insert(session.clone(), choice_value);
            if editable {
                std::fs::create_dir_all(output.join("saved"))?;
                let selected = state
                    .project
                    .select_external_directory(
                        &ObjectId::try_from(session.clone()).unwrap(),
                        FilePurpose::SaveProject,
                        output.join("saved"),
                    )
                    .map_err(|e| io::Error::other(e.message))?;
                std::fs::write(
                    output.join("selection.json"),
                    serde_json::to_vec(&json!({"session_id":session,"selection":selected}))
                        .map_err(io::Error::other)?,
                )?;
            }
            administration.push(json!({"session_id":session,"editable":editable}));
            admitted += 1;
        }
        let action_path = output.join(format!("admin-request-{action_index}.json"));
        if action_path.exists() {
            let action: Value =
                serde_json::from_slice(&std::fs::read(&action_path)?).map_err(io::Error::other)?;
            let session = action["session_id"]
                .as_str()
                .ok_or_else(|| io::Error::other("Test action session missing"))?
                .to_owned();
            let kind = action["kind"]
                .as_str()
                .ok_or_else(|| io::Error::other("Test action kind missing"))?;
            let response = if kind == "probe_eof_revoked" {
                let request = serde_json::from_value(request(
                    "native-EOF-negative-probe",
                    project.clone(),
                    "get_context",
                    json!({"selection":[]}),
                ))
                .map_err(io::Error::other)?;
                let reply = state.project.dispatch_session(
                    &ObjectId::try_from(session.clone()).map_err(io::Error::other)?,
                    request,
                );
                json!({"ok":true,"host_denied":reply["outcome"]["data"]["code"]=="PermissionDenied","context_null":reply["context"].is_null()})
            } else if kind == "switch_project" {
                let before = state.project.dispatch_ui("main", request(
                    "native-before-project-switch", project.clone(), "get_object",
                    json!({"object_id":{"kind":"project","id":project},"source":{"kind":"draft"},"node_index":null}),
                ));
                let switched = state.project.dispatch_ui("main", request(
                    "native-project-switch", Value::Null, "apply_commands",
                    json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"NFR08 post-switch project","time_reference":{"label":"actual external switch"},"departure":"discard_after_tasks_terminal"}}]}),
                ));
                if switched["outcome"]["status"] == "ok" {
                    switched_project = Some(switched["outcome"]["data"]["project_id"].clone());
                }
                json!({"ok":switched["outcome"]["status"]=="ok","before":before,"switch":switched})
            } else {
                let result = if kind == "task_readonly"
                    || kind == "task_controls"
                    || kind == "secondary_task"
                {
                    let task: TaskId = serde_json::from_value(action["task_id"].clone())
                        .map_err(io::Error::other)?;
                    agent_grant_task_access(
                        if kind == "secondary_task" {
                            secondary.clone()
                        } else {
                            main.clone()
                        },
                        app.state::<AgentPipeState>(),
                        session.clone(),
                        task,
                        kind == "task_controls",
                    )
                } else {
                    let mut choice = initial_choices
                        .get(&session)
                        .ok_or_else(|| io::Error::other("Test session initial choice missing"))?
                        .clone();
                    match kind {
                        "restore" | "secondary_update" => {}
                        "remove_fields" => {
                            for grant in choice["edit"].as_array_mut().unwrap() {
                                grant["objects"] = json!([]);
                            }
                        }
                        "drop_operations" => {
                            choice["operations"].as_array_mut().unwrap().retain(|op| {
                                !matches!(
                                    op.as_str(),
                                    Some(
                                        "get_object"
                                            | "apply_commands"
                                            | "get_task"
                                            | "cancel_task"
                                    )
                                )
                            });
                        }
                        "wrong_project" => {
                            choice["project_id"] = json!("ungranted-project");
                        }
                        _ => return Err(io::Error::other("Unknown native test action")),
                    }
                    agent_update_connection_authority(
                        if kind == "secondary_update" {
                            secondary.clone()
                        } else {
                            main.clone()
                        },
                        app.state::<AgentPipeState>(),
                        session.clone(),
                        serde_json::from_value(choice).map_err(io::Error::other)?,
                    )
                };
                match result {
                    Ok(()) => json!({"ok":true}),
                    Err(error) => json!({"ok":false,"error":error}),
                }
            };
            let response =
                json!({"index":action_index,"kind":kind,"session_id":session,"result":response});
            let temporary = output.join(format!("admin-response-{action_index}.tmp"));
            std::fs::write(
                &temporary,
                serde_json::to_vec(&response).map_err(io::Error::other)?,
            )?;
            std::fs::rename(
                temporary,
                output.join(format!("admin-response-{action_index}.json")),
            )?;
            native_actions.push(response);
            action_index += 1;
        }
        let revoke = output.join("request-revoke.json");
        if revoke.exists() {
            let session: String =
                serde_json::from_slice(&std::fs::read(&revoke)?).map_err(io::Error::other)?;
            state.revoke(&session)?;
            std::fs::remove_file(revoke)?;
            std::fs::write(
                output.join("revoked.json"),
                serde_json::to_vec(&session).map_err(io::Error::other)?,
            )?;
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(55) {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Actual MCP subprocess test exceeded 55 seconds",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout
        .join()
        .map_err(|_| io::Error::other("stdout reader failed"))??;
    let stderr = stderr
        .join()
        .map_err(|_| io::Error::other("stderr reader failed"))??;
    std::fs::write(output.join("driver.stdout.txt"), &stdout)?;
    std::fs::write(output.join("driver.stderr.txt"), &stderr)?;
    let active_project = switched_project.clone().unwrap_or(project);
    let read=state.project.dispatch_ui("main",request("mcp-native-readback",active_project.clone(),"get_object",json!({"object_id":{"kind":"project","id":active_project},"source":{"kind":"draft"},"node_index":null})));
    let context = state.project.dispatch_ui(
        "main",
        request(
            "mcp-native-context",
            active_project,
            "get_context",
            json!({"selection":[]}),
        ),
    );
    let readback = switched_project.is_some()
        && read["outcome"]["data"]["data"]["title"] == "NFR08 post-switch project"
        && context["outcome"]["data"]["project"]["revision"] == 1;
    let producer_catalog = state.project.dispatch_ui(
        "main",
        request(
            "mcp-native-provider-catalog",
            Value::Null,
            "describe_capabilities",
            json!({}),
        ),
    );
    let title = main.title().map_err(io::Error::other)?;
    let passed = status.success() && readback && title == "超级工厂 · 工业工作台";
    let report = json!({"kind":"P3_04_actual_MCP_native_process","passed":passed,"driver_exit":status.code(),"driver_stdout":stdout,"driver_stderr":stderr,"desktop_readback":readback,"desktop_tasks":context["outcome"]["data"]["tasks"],"producer_catalog":producer_catalog,"sessions":administration,"native_actions":native_actions,"window_title":title,"output_directory":output,"project_revision":context["outcome"]["data"]["project"]["revision"],"scope":"actual native desktop managed state / OS authenticated pipe / separate stdio bridge / Python MCP JSON-RPC client; native main/secondary window authorization probes; not Codex client or human/full GF acceptance"});
    std::fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report).map_err(io::Error::other)?,
    )?;
    println!("{report}");
    Ok(passed)
}
