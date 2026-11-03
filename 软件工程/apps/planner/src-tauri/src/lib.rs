use serde::Serialize;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use ts_rs::TS;

#[cfg(windows)]
mod verification_display;
#[cfg(windows)]
mod agent_pipe;
#[cfg(windows)]
mod native_render;
#[cfg(windows)]
mod native_render_capture;
mod project_api;
mod device_process;
mod execution_runtime;
mod execution_setup;

#[derive(Serialize, TS)]
pub struct EngineeringStatus {
    pub phase: String,
    pub business_operations_ready: bool,
}

#[tauri::command]
fn engineering_status() -> EngineeringStatus {
    EngineeringStatus {
        phase: "S07".into(),
        business_operations_ready: false,
    }
}

#[tauri::command]
fn engineering_frontend_ready(
    app: tauri::AppHandle,
    phase: String,
    react_version: String,
    three_revision: String,
) {
    if std::env::args().any(|arg| arg == "--s07-smoke") {
        let passed = phase == "S07" && react_version == "19.3.0" && three_revision == "186";
        println!(
            "{}",
            serde_json::json!({"kind":"S07_engineering_boot", "phase":phase, "react_version":react_version, "three_revision":three_revision, "business_operations_ready":false, "passed":passed})
        );
        app.exit(if passed { 0 } else { 1 });
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(project_api::ProjectApiState::new())
        .manage(project_api::native_capture_probe::NativeCaptureProbeState::default())
        .manage(native_render::NativeRenderState::default())
        .manage(project_api::public_render_smoke::PublicRenderSmokeState::default())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            engineering_status,
            project_api::project_import_cloud_connection,
            engineering_frontend_ready,
            native_render::native_render_input,
            native_render::native_render_asset,
            native_render::native_render_file,
            native_render::native_render_ready,
            native_render::native_render_failure,
            project_api::native_capture_probe::native_capture_probe_plan,
            project_api::native_capture_probe::native_capture_probe_frame,
            project_api::native_capture_probe::native_capture_probe_failure,
            project_api::native_capture_probe::native_capture_probe_event,
            project_api::project_api,
            project_api::project_set_simulated_device_maintenance,
            project_api::project_close_state,
            project_api::project_confirm_export_assets,
            project_api::project_finish_close,
            #[cfg(feature = "nfr-path-fixture")]
            project_api::nfr_path_fixture::project_select_fixture_directory,
            #[cfg(feature = "nfr-path-fixture")]
            project_api::nfr_path_fixture::project_select_fixture_file,
            project_api::project_select_directory,
            project_api::project_select_file,
            project_api::project_read_media_asset,
            project_api::project_read_full_run,
            project_api::project_read_execution_media_asset,
            project_api::project_read_stored_packaging,
            project_api::project_read_stored_manufacturing_file,
            project_api::project_list_stored_packaging,
            project_api::manufacturing_export::project_export_manufacturing_package,
            project_api::port_output::project_read_run_port_output,
            project_api::port_output::project_read_run_port_outputs,
            project_api::run_observation::project_read_run_observation,
            project_api::run_observation::project_read_run_observation_delta,
            agent_pipe::agent_connection_info,
            agent_pipe::agent_authorize_connection,
            agent_pipe::agent_update_connection_authority,
            agent_pipe::agent_grant_task_access,
            agent_pipe::agent_revoke_connection,
            agent_pipe::agent_select_directory
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            project_api::public_render_smoke::prepare(
                &app.state::<project_api::public_render_smoke::PublicRenderSmokeState>(),
                &std::env::args().collect::<Vec<_>>(),
            ).map_err(std::io::Error::other)?;
            app.state::<project_api::ProjectApiState>().set_render_backend(std::sync::Arc::new(native_render::NativeRenderBackend::new(
                app.handle().clone(), app.state::<native_render::NativeRenderState>().inner().clone(),
                vec![sf_domain::RenderViewKind::LineCanvas, sf_domain::RenderViewKind::MultiHallCanvas, sf_domain::RenderViewKind::TimelineSnapshot, sf_domain::RenderViewKind::DeviceDetail],
            ))).map_err(|e| std::io::Error::other(e.message))?;
            let library_root = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../modules")
            } else {
                app.path().resource_dir()?.join("modules")
            };
            app.state::<project_api::ProjectApiState>()
                .register_bundled_library(&library_root)
                .map_err(std::io::Error::other)?;
            let execution_resources = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../resources")
            } else {
                app.path().resource_dir()?
            };
            app.state::<project_api::ProjectApiState>()
                .configure_execution(&execution_resources, &app.path().app_data_dir()?)
                .map_err(|e| std::io::Error::other(e.message))?;
            let report_resources = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../../tools/architecture-proof/assets")
            } else {
                app.path().resource_dir()?.join("report-resources")
            };
            app.state::<project_api::ProjectApiState>()
                .register_evaluation_resources(&report_resources)
                .map_err(std::io::Error::other)?;
            let capture_smoke = project_api::native_capture_probe::prepare(
                &app.state::<project_api::ProjectApiState>(),
                &app.state::<project_api::native_capture_probe::NativeCaptureProbeState>(),
                &std::env::args().collect::<Vec<_>>(),
            )
            .map_err(std::io::Error::other)?;
            app.manage(agent_pipe::AgentPipeState::start(
                app.state::<project_api::ProjectApiState>().inner().clone(),
            )?);
            let host_smoke = std::env::args().any(|arg| arg == "--p0-01-host-smoke");
            let agent_smoke = std::env::args().any(|arg| arg == "--p3-04-mcp-smoke");
            let s07_smoke = std::env::args().any(|arg| arg == "--s07-smoke");
            let smoke = host_smoke || agent_smoke || s07_smoke || capture_smoke;
            let mut window_builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                // Leave room to calibrate the actual canvas, excluding the workbench docks.
                .inner_size(if cfg!(feature = "nfr-devtools") { 2200.0 } else { 1440.0 }, if cfg!(feature = "nfr-devtools") { 1302.0 } else { 900.0 })
                .maximized(!smoke && !cfg!(feature = "nfr-devtools"))
                .focused(!capture_smoke)
                .skip_taskbar(capture_smoke)
                .title(if s07_smoke {
                    "超级工厂 · 工程准备"
                } else {
                    "超级工厂 · 工业工作台"
                })
                .visible(!smoke);
            if capture_smoke {
                let capture_app = app.handle().clone();
                window_builder = window_builder
                    .initialization_script(project_api::native_capture_probe::INITIALIZATION_SCRIPT)
                    .on_page_load(|window,payload| {
                        let _ = project_api::native_capture_probe::record(window.app_handle(),"page-load",serde_json::json!({"event":format!("{:?}",payload.event()),"url":payload.url().as_str()}));
                    })
                    .on_web_resource_request(move |request,response| {
                        let path=request.uri().path();
                        if path == "/" || path.ends_with(".html") || path.ends_with(".js") {
                            let _ = project_api::native_capture_probe::record(&capture_app,"local-resource",serde_json::json!({"uri":request.uri().to_string(),"status":response.status().as_u16(),"content_type":response.headers().get("content-type").and_then(|value|value.to_str().ok())}));
                        }
                    });
            }
            if capture_smoke {
                project_api::native_capture_probe::record(app.handle(),"window-build.requested",serde_json::json!({}))?;
            }
            // Verification windows must be born on the selected display, even with mixed DPI.
            let verify_monitor = if std::env::args().any(|arg| arg == "--verify-on-secondary") {
                let primary = app.primary_monitor()?.ok_or_else(|| std::io::Error::other("No primary display"))?;
                let target = app.available_monitors()?.into_iter()
                    .find(|monitor| monitor.position() != primary.position())
                    .ok_or_else(|| std::io::Error::other("Enable an extended secondary display before verification"))?;
                window_builder = window_builder.visible(false).maximized(false)
                    .inner_size(320.0, 200.0);
                Some(target)
            } else {
                None
            };
            #[cfg(not(windows))]
            if verify_monitor.is_some() { return Err(std::io::Error::other("Secondary verification requires Windows").into()); }
            #[cfg(windows)]
            let creation_hook = verify_monitor.as_ref().map(|m| {
                let p = m.work_area().position;
                verification_display::CreationHook::install(verification_display::creation_origin(p.x, p.y))
            }).transpose()?;
            let main_window = window_builder.build()?;
            #[cfg(windows)]
            if creation_hook.as_ref().is_some_and(|hook| !hook.applied()) {
                return Err(std::io::Error::other("Secondary HWND creation hook was not applied").into());
            }
            #[cfg(windows)]
            drop(creation_hook);
            if let Some(target) = verify_monitor {
                #[cfg(windows)]
                {
                    let area = target.work_area().size;
                    verification_display::fit_hidden_client(main_window.hwnd()?.0, area.width, area.height)?;
                }
                let actual = main_window.current_monitor()?;
                if main_window.is_visible()? || actual.as_ref().map(|m| m.position()) != Some(target.position()) {
                    return Err(std::io::Error::other("Could not create the hidden window on the secondary display").into());
                }
                let origin = main_window.outer_position()?;
                let client = main_window.inner_size()?;
                main_window.show()?;
                println!("{}", serde_json::json!({"event":"verification_first_show",
                    "monitor":target.name(), "created_hidden":true, "origin":[origin.x,origin.y],
                    "client_size":[client.width,client.height]}));
            }
            if capture_smoke {
                project_api::native_capture_probe::window_built(&main_window).map_err(std::io::Error::other)?;
            }
            let _ = app
                .get_webview_window("main")
                .expect("created main webview");
            if host_smoke {
                let passed = project_api::smoke(&app.state::<project_api::ProjectApiState>());
                app.handle().exit(if passed { 0 } else { 1 });
            }
            if agent_smoke {
                let result = agent_pipe::smoke(
                    &app.state::<agent_pipe::AgentPipeState>(),
                    app.handle().clone(),
                );
                let passed = match result {
                    Ok(passed) => passed,
                    Err(error) => {
                        eprintln!("MCP native smoke failed: {error}");
                        false
                    }
                };
                app.handle().exit(if passed { 0 } else { 1 });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("S07 desktop host failed");
}
