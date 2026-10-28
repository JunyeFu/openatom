use rmcp::{
    ServiceExt,
    handler::server::wrapper::Parameters,
    model::CallToolRequestParams,
    tool, tool_router,
    transport::{ConfigureCommandExt, TokioChildProcess, stdio},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    ffi::c_void,
    io,
    mem::size_of,
    os::windows::fs::OpenOptionsExt,
    path::Path,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions},
    sync::{Mutex, oneshot},
};
use ts_rs::TS;
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree},
    Security::{
        Authorization::{
            ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        },
        GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_GROUPS, TOKEN_QUERY, TokenGroups,
    },
    Storage::FileSystem::ReplaceFileW,
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

type TrialResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
mod geometry_report;

struct LabJob {
    cancel: AtomicBool,
    terminal: std::sync::Mutex<Option<&'static str>>,
}
impl LabJob {
    fn new() -> Self {
        Self {
            cancel: AtomicBool::new(false),
            terminal: std::sync::Mutex::new(None),
        }
    }
    fn finish(&self, status: &'static str) {
        *self.terminal.lock().unwrap() = Some(status);
    }
    fn request_cancel(&self) -> &'static str {
        match *self.terminal.lock().unwrap() {
            Some("completed") => "already_completed",
            Some("cancelled") => "already_cancelled",
            Some(_) => unreachable!("Fixed lab terminal states"),
            None => {
                self.cancel.store(true, Ordering::Relaxed);
                "cancel_requested"
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, TS, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum FixturePhase {
    Manual { duration_ms: u32, resource: String },
    Automatic { duration_ms: u32 },
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, TS, PartialEq)]
struct FixtureRun {
    source_revision: u32,
    duration_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, TS, PartialEq)]
struct Fixture {
    revision: u32,
    duration_ms: u32,
    frozen_run: FixtureRun,
    note: Option<String>,
    phases: Vec<FixturePhase>,
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct Edit {
    base_revision: u32,
    duration_ms: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct PipeRequest {
    scope: String,
    op: String,
    edit: Option<Edit>,
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn logon_sid() -> io::Result<String> {
    // Windows owns token/SID layout; buffer alignment is usize, not Vec<u8>.
    unsafe {
        let mut token = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut length = 0;
        GetTokenInformation(token, TokenGroups, ptr::null_mut(), 0, &mut length);
        if length == 0 {
            CloseHandle(token);
            return Err(io::Error::last_os_error());
        }
        let mut buffer = vec![0usize; (length as usize).div_ceil(size_of::<usize>())];
        let ok = GetTokenInformation(
            token,
            TokenGroups,
            buffer.as_mut_ptr().cast(),
            length,
            &mut length,
        );
        let error = io::Error::last_os_error();
        CloseHandle(token);
        if ok == 0 {
            return Err(error);
        }
        let groups = &*(buffer.as_ptr().cast::<TOKEN_GROUPS>());
        let entries =
            std::slice::from_raw_parts(groups.Groups.as_ptr(), groups.GroupCount as usize);
        let login = entries
            .iter()
            .find(|g| g.Attributes & 0xC0000000 == 0xC0000000)
            .ok_or_else(|| io::Error::other("No current logon SID"))?;
        let mut text = ptr::null_mut();
        if ConvertSidToStringSidW(login.Sid, &mut text) == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut n = 0;
        while *text.add(n) != 0 {
            n += 1;
        }
        let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, n));
        LocalFree(text.cast());
        Ok(sid)
    }
}

fn make_pipe(name: &str, sddl: &str) -> io::Result<NamedPipeServer> {
    let encoded = wide(sddl);
    unsafe {
        let mut descriptor = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            encoded.as_ptr(),
            1,
            &mut descriptor,
            ptr::null_mut(),
        ) == 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut attrs = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor,
            bInheritHandle: 0,
        };
        let pipe = ServerOptions::new()
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(
                name,
                (&mut attrs as *mut SECURITY_ATTRIBUTES).cast::<c_void>(),
            );
        LocalFree(descriptor);
        pipe
    }
}

async fn serve_host(
    name: String,
    state: Arc<Mutex<Fixture>>,
    ready: oneshot::Sender<()>,
) -> TrialResult<()> {
    let sid = logon_sid()?;
    // Creator/session read-write and SYSTEM. No Everyone/anonymous ACL entry.
    let acl = format!("D:P(A;;GA;;;SY)(A;;GRGW;;;{sid})");
    let mut server = make_pipe(&name, &acl)?;
    ready
        .send(())
        .map_err(|_| io::Error::other("Trial coordinator closed"))?;
    loop {
        server.connect().await?;
        let connection = server;
        server = make_pipe(&name, &acl)?; // next accept exists before replying to this request
        let shared = state.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(connection);
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            let request: PipeRequest = serde_json::from_str(&line)?;
            let response = if request.scope != "S05-05-isolated-fixture" {
                json!({"status":"ScopeDenied"})
            } else {
                let mut fixture = shared.lock().await;
                match request.op.as_str() {
                    "context" => json!({"status":"ok","fixture":&*fixture}),
                    "edit" => {
                        let edit = request
                            .edit
                            .ok_or_else(|| io::Error::other("Missing trial edit"))?;
                        if edit.base_revision != fixture.revision {
                            json!({"status":"RevisionConflict","current_revision":fixture.revision})
                        } else {
                            fixture.duration_ms = edit.duration_ms;
                            fixture.revision += 1;
                            json!({"status":"accepted","revision":fixture.revision})
                        }
                    }
                    _ => json!({"status":"UnsupportedLabOperation"}),
                }
            };
            reader
                .get_mut()
                .write_all(format!("{}\n", response).as_bytes())
                .await?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
    }
}

async fn pipe_call(name: &str, request: PipeRequest) -> TrialResult<Value> {
    let mut client = ClientOptions::new().open(name)?;
    client
        .write_all(format!("{}\n", serde_json::to_string(&request)?).as_bytes())
        .await?;
    let mut reader = BufReader::new(client);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(serde_json::from_str(&line)?)
}

#[derive(Clone)]
struct Bridge {
    pipe_name: String,
}

#[tool_router(server_handler)]
impl Bridge {
    #[tool(description = "Read the isolated S05-05 Host fixture; not a product operation")]
    async fn lab_context(&self) -> Result<String, rmcp::ErrorData> {
        pipe_call(
            &self.pipe_name,
            PipeRequest {
                scope: "S05-05-isolated-fixture".into(),
                op: "context".into(),
                edit: None,
            },
        )
        .await
        .map(|v| v.to_string())
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
    #[tool(
        description = "Edit the isolated Host fixture using base_revision; not a product operation"
    )]
    async fn lab_set_duration(
        &self,
        Parameters(edit): Parameters<Edit>,
    ) -> Result<String, rmcp::ErrorData> {
        pipe_call(
            &self.pipe_name,
            PipeRequest {
                scope: "S05-05-isolated-fixture".into(),
                op: "edit".into(),
                edit: Some(edit),
            },
        )
        .await
        .map(|v| v.to_string())
        .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }
}

fn response_json(result: &rmcp::model::CallToolResult) -> TrialResult<Value> {
    let text = result
        .content
        .iter()
        .find_map(|c| c.as_text())
        .ok_or_else(|| io::Error::other("Missing actual MCP response text"))?;
    Ok(serde_json::from_str(&text.text)?)
}

fn replace_existing(old: &Path, new: &Path) -> io::Result<()> {
    let old_name = wide(
        old.to_str()
            .ok_or_else(|| io::Error::other("Non Unicode trial path"))?,
    );
    let new_name = wide(
        new.to_str()
            .ok_or_else(|| io::Error::other("Non Unicode trial path"))?,
    );
    if unsafe {
        ReplaceFileW(
            old_name.as_ptr(),
            new_name.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            ptr::null(),
        )
    } == 0
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

async fn run_trial() -> TrialResult<()> {
    let output = std::env::current_dir()?.join("output");
    std::fs::create_dir_all(&output)?;
    let initial = Fixture {
        revision: 0,
        duration_ms: 5000,
        frozen_run: FixtureRun {
            source_revision: 0,
            duration_ms: 5000,
        },
        note: None,
        phases: vec![
            FixturePhase::Manual {
                duration_ms: 5000,
                resource: "W1".into(),
            },
            FixturePhase::Automatic { duration_ms: 30000 },
        ],
    };
    assert_eq!(
        serde_json::from_str::<Fixture>(&serde_json::to_string(&initial)?)?,
        initial
    );
    let schema = serde_json::to_value(schemars::schema_for!(Fixture))?;
    std::fs::write(
        output.join("fixture.schema.json"),
        serde_json::to_vec_pretty(&schema)?,
    )?;
    let type_config = ts_rs::Config::default();
    std::fs::write(
        output.join("fixture.types.txt"),
        format!(
            "{}\n{}\n{}\n",
            FixturePhase::decl(&type_config),
            FixtureRun::decl(&type_config),
            Fixture::decl(&type_config)
        ),
    )?;
    let pipe_name = format!(r"\\.\pipe\SuperFactory-S05-05-lab-{}", std::process::id());
    let state = Arc::new(Mutex::new(initial.clone()));
    let (ready_tx, ready_rx) = oneshot::channel();
    let host = tokio::spawn(serve_host(pipe_name.clone(), state.clone(), ready_tx));
    ready_rx.await?;
    let exe = std::env::current_exe()?;
    let transport = TokioChildProcess::new(tokio::process::Command::new(exe).configure(|cmd| {
        cmd.arg("bridge").arg(&pipe_name).creation_flags(0x08000000);
    }))?;
    let client = ().serve(transport).await?;
    let discovered = client.list_all_tools().await?;
    let mut names: Vec<_> = discovered.iter().map(|t| t.name.to_string()).collect();
    names.sort();
    assert_eq!(names, vec!["lab_context", "lab_set_duration"]);
    let before = response_json(
        &client
            .call_tool(CallToolRequestParams::new("lab_context"))
            .await?,
    )?;
    assert_eq!(before["fixture"]["revision"], 0);
    let command = CallToolRequestParams::new("lab_set_duration").with_arguments(
        json!({"base_revision":0,"duration_ms":6000})
            .as_object()
            .unwrap()
            .clone(),
    );
    let (mcp_response, direct_response) = tokio::join!(
        client.call_tool(command),
        pipe_call(
            &pipe_name,
            PipeRequest {
                scope: "S05-05-isolated-fixture".into(),
                op: "edit".into(),
                edit: Some(Edit {
                    base_revision: 0,
                    duration_ms: 7000
                })
            }
        )
    );
    let mcp = response_json(&mcp_response?)?;
    let direct = direct_response?;
    let statuses = [mcp["status"].as_str(), direct["status"].as_str()];
    assert_eq!(
        statuses.iter().filter(|s| **s == Some("accepted")).count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|s| **s == Some("RevisionConflict"))
            .count(),
        1
    );
    let denied = pipe_call(
        &pipe_name,
        PipeRequest {
            scope: "another-project".into(),
            op: "context".into(),
            edit: None,
        },
    )
    .await?;
    assert_eq!(denied["status"], "ScopeDenied");
    let after = response_json(
        &client
            .call_tool(CallToolRequestParams::new("lab_context"))
            .await?,
    )?;
    let committed: Fixture = serde_json::from_value(after["fixture"].clone())?;
    assert_eq!(committed.revision, 1);
    assert_eq!(committed.frozen_run, initial.frozen_run);
    // Genuine repeated geometry queries outside the Host edit lock; no artificial sleep.
    let job = Arc::new(LabJob::new());
    let stop = job.clone();
    let (worker_ready_tx, worker_ready_rx) = oneshot::channel();
    let worker_source_revision = committed.revision;
    let worker = tokio::task::spawn_blocking(move || {
        use parry3d_f64::{
            math::{Pose, Vector},
            query::distance,
            shape::Cuboid,
        };
        let cube = Cuboid::new(Vector::splat(1.0));
        let origin = Pose::IDENTITY;
        let mut ready = Some(worker_ready_tx);
        let mut processed = 0u32;
        for n in 0..100_000_000u32 {
            if stop.cancel.load(Ordering::Relaxed) {
                stop.finish("cancelled");
                return ("cancelled", processed, worker_source_revision);
            }
            let other =
                Pose::from_translation(Vector::new(3.0 + (n % 1000) as f64 / 1000.0, 0.0, 0.0));
            let measured = distance(&origin, &cube, &other, &cube)
                .expect("Supported lab Cuboid pair")
                .distance;
            assert!(measured >= 0.999999999);
            processed += 1;
            if processed == 1024 {
                ready
                    .take()
                    .unwrap()
                    .send(())
                    .expect("Live trial coordinator");
            }
        }
        stop.finish("completed");
        ("completed", processed, worker_source_revision)
    });
    worker_ready_rx.await?;
    let read_start = Instant::now();
    let while_computing = response_json(
        &client
            .call_tool(CallToolRequestParams::new("lab_context"))
            .await?,
    )?;
    let context_roundtrip_ms = read_start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(while_computing["fixture"]["revision"], 1);
    let edit_during_worker = client
        .call_tool(
            CallToolRequestParams::new("lab_set_duration").with_arguments(
                json!({"base_revision":1,"duration_ms":8000})
                    .as_object()
                    .unwrap()
                    .clone(),
            ),
        )
        .await?;
    assert_eq!(response_json(&edit_during_worker)?["status"], "accepted");
    assert_eq!(job.request_cancel(), "cancel_requested");
    let worker_done = worker.await?;
    assert_eq!(worker_done.0, "cancelled");
    assert!(worker_done.1 >= 1024 && worker_done.1 < 100_000_000);
    assert_eq!(worker_done.2, 1);
    assert_eq!(state.lock().await.revision, 2);
    let tiny_job = Arc::new(LabJob::new());
    let tiny_worker = tiny_job.clone();
    let completed_work = tokio::task::spawn_blocking(move || {
        use parry3d_f64::{
            math::{Pose, Vector},
            query::distance,
            shape::Cuboid,
        };
        let cube = Cuboid::new(Vector::splat(1.0));
        let measured = distance(
            &Pose::IDENTITY,
            &cube,
            &Pose::from_translation(Vector::new(3.0, 0.0, 0.0)),
            &cube,
        )
        .unwrap()
        .distance;
        assert_eq!(measured, 1.0);
        tiny_worker.finish("completed");
        ("completed", measured)
    })
    .await?;
    let late_cancel_response = tiny_job.request_cancel();
    assert_eq!(late_cancel_response, "already_completed");
    assert_eq!(*tiny_job.terminal.lock().unwrap(), Some("completed"));
    let old_file = output.join("current-fixture.json");
    let new_file = output.join("pending-fixture.json");
    std::fs::write(&old_file, serde_json::to_vec_pretty(&initial)?)?;
    std::fs::write(&new_file, serde_json::to_vec_pretty(&committed)?)?;
    let exclusive = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&old_file)?;
    let failed = replace_existing(&old_file, &new_file);
    assert!(failed.is_err());
    drop(exclusive);
    let preserved: Fixture = serde_json::from_slice(&std::fs::read(&old_file)?)?;
    assert_eq!(preserved, initial);
    replace_existing(&old_file, &new_file)?;
    let reopened: Fixture = serde_json::from_slice(&std::fs::read(&old_file)?)?;
    assert_eq!(reopened, committed);
    let result = json!({"kind":"S05-05-isolated-native-actual-trial-not-product", "status":"passed_actual_native_subset",
        "official_sdk":"rmcp3.5.1", "mcp_initialized":true,"actual_discovered_lab_tools":names,
        "same_host_two_actor_results":[mcp,direct],"scope_denial":denied,"frozen_run_unchanged":true,
        "pipe_acl":"current_logon_sid_and_system_no_everyone_or_anonymous", "remote_clients_rejected":true,
        "actual_locked_replace_error_code":failed.err().and_then(|e|e.raw_os_error()),
        "old_file_preserved_after_failure":true,"replacement_reopened_equals_committed":true,
        "actual_type_json_roundtrip":true,"actual_TS_declaration_and_schema_written":true,
        "actual_background_work":{"work":"real parry Cuboid distance queries, no sleep", "processed_queries":worker_done.1,
            "worker_source_revision":worker_done.2,"active_draft_revision_after_edit":2,
            "actual_terminal":worker_done.0,"late_cancel_completed_terminal":completed_work.0,
            "actual_late_cancel_response":late_cancel_response,
            "context_RPC_roundtrip_ms_diagnostic_not_NFR":context_roundtrip_ms,
            "saved_fixture_revision":reopened.revision,"still_unsaved_new_draft_revision":2},
        "actual_product_runtime":null,"actual_frontend_started":false,"product_operations_available":0,
        "not_proved":["Tauri invoke and real desktop renderer","full product types and actual eighteen operations",
            "actual Codex Agent client","paper-packaging algorithm","four-format Chinese reporting","NFR benchmark"]});
    std::fs::write(
        output.join("native-trial-result.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!("{}", serde_json::to_string(&result)?);
    client.cancel().await?;
    host.abort();
    Ok(())
}

#[tokio::main]
async fn main() -> TrialResult<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("bridge") {
        let service = Bridge {
            pipe_name: args
                .get(2)
                .ok_or_else(|| io::Error::other("Missing lab pipe"))?
                .clone(),
        }
        .serve(stdio())
        .await?;
        service.waiting().await?;
    } else if args.get(1).map(String::as_str) == Some("geometry-report") {
        geometry_report::run()?;
    } else {
        run_trial().await?;
    }
    Ok(())
}
