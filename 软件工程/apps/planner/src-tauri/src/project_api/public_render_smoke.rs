//! Explicit controlled public-MCP experiment, started by the actual main IPC.
//! It never runs in normal product mode and never constructs another Host.
use super::*;
use std::{
    io::{self, Read},
    os::windows::process::CommandExt,
    process::{Command, Stdio},
};
use tauri::Manager;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeCase {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scope: Option<ApiScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    draft_from_preparation: Option<usize>,
    params: sf_commands::RenderViewParams,
}
#[derive(Serialize)]
struct ResolvedCase {
    name: String,
    scope: ApiScope,
    params: sf_commands::RenderViewParams,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    #[serde(default)]
    preparations: Vec<ApiRequest>,
    cases: Vec<SmokeCase>,
    #[serde(default)]
    allow_view_control: bool,
}
#[derive(Clone)]
struct Config {
    input: PathBuf,
    output: PathBuf,
    bridge: PathBuf,
    plan: Plan,
}
struct DriverChild(std::process::Child);
impl Drop for DriverChild {
    fn drop(&mut self) {
        if matches!(self.0.try_wait(), Ok(None)) {
            // This exact process belongs to this explicit experiment. Closing
            // its pipes also lets its bridge observe EOF on a failed admin step.
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
impl Config {
    fn parse(args: &[String]) -> Result<Option<Self>, String> {
        if !args.iter().any(|a| a == "--p3-04-public-render-smoke") {
            return Ok(None);
        }
        if args.iter().any(|a| {
            matches!(
                a.as_str(),
                "--p3-04-capture-smoke"
                    | "--p3-04-mcp-smoke"
                    | "--s07-smoke"
                    | "--p0-01-host-smoke"
            )
        }) {
            return Err("Public render smoke cannot be mixed with another experiment".into());
        }
        let path = |flag: &str| -> Result<PathBuf, String> {
            let positions: Vec<_> = args
                .iter()
                .enumerate()
                .filter(|(_, a)| a.as_str() == flag)
                .map(|(i, _)| i)
                .collect();
            if positions.len() != 1 {
                return Err(format!("Expected exactly one {flag}"));
            }
            let value = args
                .get(positions[0] + 1)
                .filter(|s| !s.starts_with("--"))
                .ok_or_else(|| format!("Missing {flag} value"))?;
            let value = PathBuf::from(value);
            if !value.is_absolute() {
                return Err(format!("{flag} must be an explicit absolute path"));
            }
            Ok(value)
        };
        let input = path("--render-input")?;
        let output = path("--render-output")?;
        let bridge = path("--render-bridge")?;
        let views = path("--render-views")?;
        let plan: Plan = serde_json::from_slice(&std::fs::read(views).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        if plan.cases.is_empty()
            || plan.cases.iter().any(|c| {
                matches!(c.scope, Some(ApiScope::Workspace))
                    || c.scope.is_some() == c.draft_from_preparation.is_some()
            })
        {
            return Err("Each case needs either an explicit Project/Draft scope or a real preparation Draft selection".into());
        }
        if !input.is_dir() || !bridge.is_file() {
            return Err(
                "Public render input directory and actual bridge executable must exist".into(),
            );
        }
        Ok(Some(Self {
            input,
            output,
            bridge,
            plan,
        }))
    }
}
#[derive(Default)]
pub struct PublicRenderSmokeState(Mutex<Option<Config>>);
pub fn prepare(state: &PublicRenderSmokeState, args: &[String]) -> Result<(), String> {
    *state
        .0
        .lock()
        .map_err(|_| "Public render smoke state poisoned")? = Config::parse(args)?;
    Ok(())
}
pub fn trigger(app: &tauri::AppHandle) -> Result<(), ApiError> {
    let config = app
        .state::<PublicRenderSmokeState>()
        .0
        .lock()
        .map_err(|_| {
            host_error(
                ErrorCode::ComputationFailed,
                "Public render smoke state poisoned",
            )
        })?
        .take();
    let Some(config) = config else {
        return Ok(());
    };
    let app = app.clone();
    std::thread::Builder::new().name("public-render-MCP-proof".into()).spawn(move|| {
        let result=run(&app,&config);
        let passed=result.as_ref().is_ok_and(|report| report["passed"]==true);
        match result { Ok(report)=>println!("{report}"),Err(e)=> {
            let report=json!({"kind":"P3-04_public_native_render_MCP","passed":false,"error":e.to_string(),"controlled_authorization":true,"human_or_Codex_acceptance":false});
            if config.output.is_dir() { let _=write_json(&config.output.join("failure.json"),&report); }
            eprintln!("{report}");
        }}
        app.exit(if passed {0} else {1});
    }).map_err(|e|host_error(ErrorCode::ComputationFailed,&e.to_string()))?;
    Ok(())
}
fn write_json(path: &Path, value: &Value) -> io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec_pretty(value).map_err(io::Error::other)?)
}
fn data(reply: Value) -> io::Result<Value> {
    if reply["outcome"]["status"] != "ok" {
        return Err(io::Error::other(format!(
            "Real UI Host request failed: {reply}"
        )));
    }
    Ok(reply["outcome"]["data"].clone())
}
fn snapshot(state: &ProjectApiState, project: &Value) -> io::Result<Value> {
    let project_state = data(business_object(
        state,
        project,
        "project",
        project
            .as_str()
            .ok_or_else(|| io::Error::other("Project ID missing"))?,
    ))?;
    let context = data(state.dispatch_ui(
        "main",
        smoke_request(
            "render-proof-context",
            project.clone(),
            "get_context",
            json!({"selection":[]}),
        ),
    ))?;
    let mut runs = Vec::new();
    for summary in context["run_summaries"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
    {
        runs.push(data(state.dispatch_ui(
            "main",
            smoke_request(
                "render-proof-full-run",
                project.clone(),
                "get_run",
                json!({"run_id":summary["run_id"],"query":{"kind":"full_result"}}),
            ),
        ))?);
    }
    Ok(json!({"project":project_state,"dirty":context["project"]["dirty"],"runs":runs}))
}
fn resolve_cases(cases: &[SmokeCase], commits: &[Value]) -> io::Result<Vec<ResolvedCase>> {
    cases
        .iter()
        .map(|case| {
            let mut params = case.params.clone();
            let scope = match (&case.scope, case.draft_from_preparation) {
                (Some(scope), None) => scope.clone(),
                (None, Some(index)) => {
                    let commit = commits
                        .get(index)
                        .ok_or_else(|| io::Error::other("Selected preparation is missing"))?;
                    let scope: sf_domain::EditScope =
                        serde_json::from_value(commit["scope"].clone())
                            .map_err(io::Error::other)?;
                    let sf_domain::EditScope::DefinitionDraft(identity) = scope else {
                        return Err(io::Error::other(
                            "Selected preparation is not an actual DefinitionDraft EditCommit",
                        ));
                    };
                    let revision: Revision = serde_json::from_value(commit["new_revision"].clone())
                        .map_err(io::Error::other)?;
                    if json!(identity.revision) != json!(revision) {
                        return Err(io::Error::other(
                            "Actual Draft EditCommit identity/revision mismatch",
                        ));
                    }
                    params.revision = revision;
                    let target = sf_domain::ObjectRef {
                        kind: sf_domain::ObjectKind::DefinitionDraft,
                        id: ObjectId::try_from(identity.id.as_str().to_owned())
                            .map_err(io::Error::other)?,
                    };
                    params.object_id = Some(target.clone());
                    params.view.object_ref = Some(target);
                    ApiScope::DefinitionDraft(identity.id)
                }
                _ => {
                    return Err(io::Error::other(
                        "Exactly one explicit Scope or preparation Draft selection is required",
                    ));
                }
            };
            Ok(ResolvedCase {
                name: case.name.clone(),
                scope,
                params,
            })
        })
        .collect()
}
fn run(app: &tauri::AppHandle, config: &Config) -> io::Result<Value> {
    std::fs::create_dir(&config.output)?;
    let state = app.state::<ProjectApiState>();
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| io::Error::other("Actual main window missing"))?;
    let selected = state
        .register_directory(
            FilePurpose::ImportPackage,
            Some(config.input.canonicalize()?),
        )
        .map_err(|e| io::Error::other(e.message))?;
    let admitted=data(state.dispatch_ui("main",smoke_request("render-proof-import",Value::Null,"import_package",json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}))))?;
    let imported = data(state.wait_for_task(Value::Null, admitted["id"].clone()))?;
    if imported["task"]["state"] != "completed" {
        return Err(io::Error::other(format!(
            "Actual source import did not complete: {imported}"
        )));
    }
    data(state.dispatch_ui("main",smoke_request("render-proof-open",Value::Null,"apply_commands",json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":admitted["id"],"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],"selected_assets":[],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"accept_missing_as_draft":true,"departure":"stay"}}}]}))))?;
    let context = data(state.dispatch_ui(
        "main",
        smoke_request(
            "render-proof-project",
            Value::Null,
            "get_context",
            json!({"selection":[]}),
        ),
    ))?;
    let project = context["project"]["id"].clone();
    let mut preparations = Vec::new();
    for request in &config.plan.preparations {
        let request = serde_json::to_value(request).map_err(io::Error::other)?;
        if request["operation"] != "apply_commands"
            || request["project_id"] != project
            || request["scope"]["kind"] == "workspace"
        {
            return Err(io::Error::other(
                "Preparations must be explicit real ApplyCommands in this loaded Project/Draft",
            ));
        }
        preparations.push(data(state.dispatch_ui("main", request))?);
    }
    let cases = resolve_cases(&config.plan.cases, &preparations)?;
    let before = snapshot(&state, &project)?;
    let main_before = json!({"visible":main.is_visible().map_err(io::Error::other)?,"focused":main.is_focused().map_err(io::Error::other)?,"title":main.title().map_err(io::Error::other)?});
    write_json(&config.output.join("source-before.json"), &before)?;
    write_json(
        &config.output.join("plan.json"),
        &json!({"host_instance_id":state.host_instance_id,"project_id":project,"input":config.input,"cases":cases,"requested_cases":config.plan.cases,"preparations":preparations,"controlled_authorization":true,"human_or_Codex_acceptance":false}),
    )?;
    let python = std::env::var_os("SUPERFACTORY_TEST_PYTHON").ok_or_else(|| {
        io::Error::other("SUPERFACTORY_TEST_PYTHON must select the existing installed test runtime")
    })?;
    let driver = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../scenarios/Agent共建/runtime-public-native-render/process_render_client.py");
    let pipe = app.state::<crate::agent_pipe::AgentPipeState>();
    let endpoint = pipe.info()?["endpoint"]
        .as_str()
        .ok_or_else(|| io::Error::other("Actual Host endpoint missing"))?
        .to_owned();
    let mut child = DriverChild(
        Command::new(python)
            .args(["-B", "-X", "utf8"])
            .arg(driver)
            .arg(&config.bridge)
            .arg(endpoint)
            .arg(state.host_instance_id.as_str())
            .arg(project.as_str().unwrap())
            .arg(&config.output)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(0x08000000)
            .spawn()?,
    );
    let mut stdout = child.0.stdout.take().unwrap();
    let mut stderr = child.0.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut text = String::new();
        stdout.read_to_string(&mut text).map(|_| text)
    });
    let err = std::thread::spawn(move || {
        let mut text = String::new();
        stderr.read_to_string(&mut text).map(|_| text)
    });
    let mut authorities = Vec::new();
    let mut authorized = false;
    let status = loop {
        let info = pipe.info()?;
        let peer_file = config.output.join("bridge-peer.json");
        let expected_peer = if !authorized && peer_file.is_file() {
            Some(
                serde_json::from_slice::<Value>(&std::fs::read(peer_file)?)
                    .map_err(io::Error::other)?,
            )
        } else {
            None
        };
        for peer in info["connections"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            let Some(expected_pid) = expected_peer
                .as_ref()
                .and_then(|expected| expected["pid"].as_u64())
            else {
                continue;
            };
            if Some(expected_pid) != peer["pid"].as_u64() {
                continue;
            }
            if peer["authorized"] == false && peer["revoked"] == false {
                let session = peer["session_id"]
                    .as_str()
                    .ok_or_else(|| io::Error::other("Actual peer session missing"))?
                    .to_owned();
                let drafts: Vec<_> = cases
                    .iter()
                    .filter_map(|case| {
                        if let ApiScope::DefinitionDraft(id) = &case.scope {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let choice = json!({"project_id":project,"operations":["describe_capabilities","render_view","get_task","cancel_task"],"definition_drafts":drafts,"allow_view_control":config.plan.allow_view_control,"edit":[]});
                crate::agent_pipe::agent_authorize_connection(
                    main.clone(),
                    app.state::<crate::agent_pipe::AgentPipeState>(),
                    session.clone(),
                    serde_json::from_value(choice.clone()).map_err(io::Error::other)?,
                )
                .map_err(io::Error::other)?;
                authorities.push(json!({"peer":peer,"choice":choice,"actual_main_command":true}));
                authorized = true;
            }
        }
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = out
        .join()
        .map_err(|_| io::Error::other("MCP stdout reader panicked"))??;
    let stderr = err
        .join()
        .map_err(|_| io::Error::other("MCP stderr reader panicked"))??;
    std::fs::write(config.output.join("driver.stdout.txt"), &stdout)?;
    std::fs::write(config.output.join("driver.stderr.txt"), &stderr)?;
    let after = snapshot(&state, &project)?;
    write_json(&config.output.join("source-after.json"), &after)?;
    let main_after = json!({"visible":main.is_visible().map_err(io::Error::other)?,"focused":main.is_focused().map_err(io::Error::other)?,"title":main.title().map_err(io::Error::other)?});
    let driver: Value =
        serde_json::from_slice(&std::fs::read(config.output.join("driver-result.json"))?)
            .map_err(io::Error::other)?;
    let unchanged = before == after;
    let report = json!({"kind":"P3-04_public_native_render_MCP","passed":status.success()&&driver["passed"]==true&&unchanged,"driver_exit":status.code(),"driver":driver,"source_unchanged":unchanged,"main_before":main_before,"main_after":main_after,"authorities":authorities,"controlled_authorization":true,"human_or_Codex_acceptance":false,"output":config.output});
    write_json(&config.output.join("report.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_render_runner_is_disabled_normally_and_cannot_reuse_private_probe_flags() {
        assert!(Config::parse(&["superfactory".into()]).unwrap().is_none());
        assert!(
            Config::parse(&[
                "superfactory".into(),
                "--p3-04-public-render-smoke".into(),
                "--p3-04-capture-smoke".into()
            ])
            .is_err()
        );
        assert!(
            Config::parse(&[
                "superfactory".into(),
                "--p3-04-public-render-smoke".into(),
                "--render-input".into(),
                "relative".into()
            ])
            .is_err()
        );
    }

    #[test]
    fn case_binding_uses_the_actual_new_draft_commit_without_guessing_host_or_changing_project() {
        let state = ProjectApiState::new();
        let created = data(state.dispatch_ui("main", smoke_request("smoke-binding-create", Value::Null, "apply_commands", json!({"base_revision":null,"commands":[{"kind":"create_project","data":{"title":"real Draft binding","time_reference":{"label":"relative","origin_iso8601":null,"timezone":"Asia/Shanghai"},"departure":"stay"}}]})))).unwrap();
        let project = created["project_id"].clone();
        let before = business_object(&state, &project, "project", project.as_str().unwrap());
        let package: Value = serde_json::from_str(include_str!(
            "../../../../../modules/devices/template-support/operations/module.json"
        ))
        .unwrap();
        let commit = data(state.dispatch_ui("main", smoke_request("smoke-binding-draft",project.clone(),"apply_commands",json!({"base_revision":1,"commands":[{"kind":"create_definition_draft","data":package["data"]["definition"]}]})))).unwrap();
        let params = json!({"revision":1,"view":{"kind":"module_preview","canvas":"line_canvas","hall_id":null,"object_ref":null,"project_revision":1,"run_id":null,"view_time_ms":null,"lod":"detailed","camera":before["outcome"]["data"]["data"]["views"]["line_camera"]},"object_id":null,"preview":null,"delivery":"capture_only"});
        let case: SmokeCase = serde_json::from_value(
            json!({"name":"actual newly created Draft","draft_from_preparation":0,"params":params}),
        )
        .unwrap();
        let resolved = resolve_cases(&[case.clone()], &[commit.clone()]).unwrap();
        assert_eq!(
            json!(resolved[0].scope),
            json!({"kind":"definition_draft","data":commit["scope"]["data"]["id"]})
        );
        assert_eq!(json!(resolved[0].params.revision), commit["new_revision"]);
        assert_eq!(
            json!(resolved[0].params.object_id),
            json!({"kind":"definition_draft","id":commit["scope"]["data"]["id"]})
        );
        assert!(resolve_cases(&[case.clone()], &[]).is_err());
        assert!(resolve_cases(&[case], &[created]).is_err());
        assert_eq!(
            before,
            business_object(&state, &project, "project", project.as_str().unwrap())
        );
    }
}
