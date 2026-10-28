//! Native-only connection file and lifecycle. No Renderer or Agent configuration.
use crate::device_process::{DeviceProfile, DeviceToken, NativeCloudOrigin, NativeDeviceRuntime};
use crate::execution_runtime::{
    self, NativeExecutionActor, NativeExecutionConfig, NativeExecutionDevice,
};
use serde::Deserialize;
use sf_commands::execution_cloud::ExecutionCloudConnection;
use sf_commands::{
    ApiError, CloudReadConnection, ErrorCode, ExecutionPreparationRuntime, ProjectHost,
};
use sf_domain::cloud_execution::CandidateBinding;
use sf_domain::{CloudProjectId, ObjectId, ProjectId, ScenarioId};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

fn error(code: ErrorCode, message: &str) -> ApiError {
    ApiError {
        code,
        message: message.into(),
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}

// Only deserialized from app_data_dir/v2-cloud/connection-multistage.json, never an IPC body.
// Secret fields deliberately have neither Debug nor Serialize.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrivateConnection {
    connection_ref: ObjectId,
    project_id: ProjectId,
    cloud_project_id: CloudProjectId,
    endpoint: String,
    reader_token: String,
    gateway_token: String,
    candidate_bindings: Vec<CandidateBinding>,
    devices: Vec<PrivateDevice>,
    scenario_id: Option<ScenarioId>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PrivateDevice {
    device_id: ObjectId,
    profile: PrivateProfile,
    token: String,
}
#[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum PrivateProfile {
    #[serde(rename = "processor-a")]
    ProcessorA,
    #[serde(rename = "processor-b")]
    ProcessorB,
    #[serde(rename = "factory-commands")]
    FactoryCommands,
}
impl PrivateConnection {
    fn validate_bindings(&self) -> Result<(), ApiError> {
        let candidates: BTreeSet<_> = self
            .candidate_bindings
            .iter()
            .map(|b| &b.device_id)
            .collect();
        let devices: BTreeSet<_> = self.devices.iter().map(|b| &b.device_id).collect();
        let profiles: BTreeSet<_> = self.devices.iter().map(|b| &b.profile).collect();
        let factory = profiles.len() == 1 && profiles.contains(&PrivateProfile::FactoryCommands);
        if factory && self.scenario_id.is_none() {
            return Err(error(ErrorCode::InvalidInput, "整厂连接需要指定要绑定的生产方案。"));
        }
        let binding_ids: BTreeSet<_> = self.candidate_bindings.iter().map(|b| &b.binding_id).collect();
        if self.candidate_bindings.len() < 2
            || devices.len() != self.devices.len()
            || binding_ids.len() != self.candidate_bindings.len()
            || candidates != devices
            || (!factory && (profiles.len() != 2 || profiles.contains(&PrivateProfile::FactoryCommands)
                || candidates.len() != self.candidate_bindings.len()))
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "设备绑定必须逐台对应，设备和绑定编号不可重复；整厂设备统一使用按指令执行的配置。",
            ));
        }
        for device in &self.devices {
            if device.profile == PrivateProfile::FactoryCommands { continue; }
            let candidate = self
                .candidate_bindings
                .iter()
                .find(|b| b.device_id == device.device_id)
                .expect("same exact device set");
            let nominal = match device.profile {
                PrivateProfile::ProcessorA => 10000,
                PrivateProfile::ProcessorB => 15000,
                PrivateProfile::FactoryCommands => unreachable!(),
            };
            if candidate.nominal_work_ms.value() != nominal {
                return Err(error(
                    ErrorCode::InvalidInput,
                    "设备处理时间与所选配置不一致，请检查设备绑定中的参考加工时间。",
                ));
            }
        }
        Ok(())
    }
}

// Import stays inside Native: renderer receives only a count, never file bytes or tokens.
pub(super) fn validate_connection_import(bytes: &[u8], project: &ProjectId) -> Result<usize, ApiError> {
    let profile: PrivateConnection = serde_json::from_slice(bytes)
        .map_err(|_| error(ErrorCode::InvalidInput, "连接文件格式不正确；原配置未更改。"))?;
    profile.validate_bindings()?;
    if &profile.project_id != project {
        return Err(error(ErrorCode::InvalidInput, "连接文件属于其他工程；请选用当前工程的连接文件。"));
    }
    let count = profile.devices.len();
    NativeCloudOrigin::from_native_configuration(profile.endpoint.clone())
        .map_err(|_| error(ErrorCode::InvalidInput, "连接地址必须是有效的 HTTPS 服务地址。"))?;
    CloudReadConnection::https(profile.connection_ref.clone(), profile.project_id.clone(),
        profile.cloud_project_id.clone(), profile.endpoint.clone(), profile.reader_token, profile.candidate_bindings)?;
    ExecutionCloudConnection::https(profile.connection_ref, profile.project_id, profile.cloud_project_id,
        profile.endpoint, profile.gateway_token)?;
    for device in profile.devices {
        DeviceToken::from_native_secret(device.token)
            .map_err(|_| error(ErrorCode::InvalidInput, "设备凭据格式不正确；原配置未更改。"))?;
    }
    Ok(count)
}
pub(super) fn store_connection_import(bytes: &[u8], private: &Path) -> Result<(), ApiError> {
    std::fs::create_dir_all(private).map_err(|_| error(ErrorCode::IoFailure, "无法创建本机连接目录。"))?;
    let target = private.join("connection-multistage.json");
    if target.exists() {
        // Keep the previous credentials exclusively in the same Native-private directory.
        std::fs::copy(&target, private.join("connection-before-import.json"))
            .map_err(|_| error(ErrorCode::IoFailure, "无法保留原连接配置；导入已停止。"))?;
    }
    std::fs::write(private.join("connection-import.pending"), bytes)
        .map_err(|_| error(ErrorCode::IoFailure, "无法写入连接文件；原配置未更改。"))?;
    // Windows replacement uses the already preserved private copy for rollback.
    if let Err(_) = std::fs::copy(private.join("connection-import.pending"), &target) {
        if private.join("connection-before-import.json").exists() {
            std::fs::copy(private.join("connection-before-import.json"), &target)
                .map_err(|_| error(ErrorCode::IoFailure, "连接写入失败；原文件仍保存在本机 connection-before-import.json。"))?;
        }
        return Err(error(ErrorCode::IoFailure, "连接写入失败，已保留原配置。"));
    }
    std::fs::remove_file(private.join("connection-import.pending"))
        .map_err(|_| error(ErrorCode::IoFailure, "连接已保存，但临时文件清理失败；请检查本机连接目录。"))?;
    Ok(())
}

pub(super) fn validate_connection_model_bindings(bytes: &[u8], host: &ProjectHost, ui: &ObjectId) -> Result<(), ApiError> {
    let profile: PrivateConnection = serde_json::from_slice(bytes)
        .map_err(|_| error(ErrorCode::InvalidInput, "连接文件格式不正确；原配置未更改。"))?;
    if let Some(scenario) = profile.scenario_id {
        host.validate_native_factory_bindings(ui, &scenario, &profile.candidate_bindings)?;
    }
    Ok(())
}

pub(super) struct NativeExecutionInstallation {
    project_id: ProjectId,
    reader: CloudReadConnection,
    runtime: ExecutionPreparationRuntime,
    actor: NativeExecutionActor,
}
impl NativeExecutionInstallation {
    pub(super) fn load(
        host: Arc<Mutex<ProjectHost>>,
        resource_root: &Path,
        app_data_dir: &Path,
    ) -> Result<Option<Self>, ApiError> {
        let private = app_data_dir.join("v2-cloud");
        let bytes = match std::fs::read(private.join("connection-multistage.json")) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err(error(
                    ErrorCode::IoFailure,
                    "Cannot read the Native private cloud connection",
                ));
            }
        };
        let profile: PrivateConnection = serde_json::from_slice(&bytes).map_err(|_| {
            error(
                ErrorCode::InvalidInput,
                "Private cloud connection does not match the Native configuration contract",
            )
        })?;
        profile.validate_bindings()?;
        let reader = CloudReadConnection::https(
            profile.connection_ref.clone(),
            profile.project_id.clone(),
            profile.cloud_project_id.clone(),
            profile.endpoint.clone(),
            profile.reader_token,
            profile.candidate_bindings,
        )?;
        let cloud = Arc::new(ExecutionCloudConnection::https(
            profile.connection_ref.clone(),
            profile.project_id.clone(),
            profile.cloud_project_id,
            profile.endpoint.clone(),
            profile.gateway_token,
        )?);
        let origin =
            NativeCloudOrigin::from_native_configuration(profile.endpoint).map_err(|_| {
                error(
                    ErrorCode::InvalidInput,
                    "Native device connection requires a fixed HTTPS origin",
                )
            })?;
        let devices = Arc::new(
            NativeDeviceRuntime::from_native_paths(resource_root, &private, origin).map_err(
                |_| {
                    error(
                        ErrorCode::InvalidInput,
                        "Native device resources or private runtime directory are invalid",
                    )
                },
            )?,
        );
        let mut bindings = BTreeMap::new();
        for device in profile.devices {
            bindings.insert(
                device.device_id,
                NativeExecutionDevice {
                    profile: match device.profile {
                        PrivateProfile::ProcessorA => DeviceProfile::ProcessorA,
                        PrivateProfile::ProcessorB => DeviceProfile::ProcessorB,
                        PrivateProfile::FactoryCommands => DeviceProfile::FactoryCommands,
                    },
                    token: DeviceToken::from_native_secret(device.token).map_err(|_| {
                        error(
                            ErrorCode::InvalidInput,
                            "Native device credential is invalid",
                        )
                    })?,
                },
            );
        }
        let (runtime, actor) = execution_runtime::start_execution_runtime(
            host,
            NativeExecutionConfig {
                cloud,
                devices,
                connection_ref: profile.connection_ref,
                bindings,
                archive_root: private.join("executions"),
            },
        )?;
        Ok(Some(Self {
            project_id: profile.project_id,
            reader,
            runtime,
            actor,
        }))
    }

    // Reader/actor construction and all I/O occurred in setup, outside the Host lock.
    // Reinstall the same queue after an authorized project departure resets the registry.
    pub(super) fn install_for_active_project(
        &self,
        host: &mut ProjectHost,
        ui: &ObjectId,
    ) -> Result<(), ApiError> {
        if host.desktop_close_state().project_id.as_ref() == Some(&self.project_id)
            && !host.execution_preparation_available()
        {
            if self.actor.is_finished() {
                return Err(error(
                    ErrorCode::ComputationFailed,
                    "Native execution worker has exited",
                ));
            }
            host.install_cloud_read_connection(ui, self.reader.clone())?;
            host.install_execution_preparation_runtime(
                ui,
                self.project_id.clone(),
                self.runtime.clone(),
            )?;
        }
        Ok(())
    }

    pub(super) fn require_settled_for_close(&self) -> Result<(), ApiError> {
        if self.actor.status().has_obligations {
            return Err(error(
                ErrorCode::InvalidInput,
                "Native execution still owns preparation, production or cleanup obligations",
            ));
        }
        Ok(())
    }

    pub(super) fn request_device_maintenance(
        &self,
        identity: &sf_domain::cloud_execution::ExecutionIdentity,
        device: &ObjectId,
        maintenance: bool,
    ) -> Result<(), ApiError> {
        self.actor.request_device_maintenance(identity, device, maintenance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use sf_commands::FilePurpose;
    use sf_domain::TaskHandle;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    fn id(s: &str) -> ObjectId {
        s.to_owned().try_into().unwrap()
    }
    fn root() -> std::path::PathBuf {
        // Both the product crate and its independent harness locate by source file.
        let source = Path::new(file!());
        let source = if source.is_absolute() {
            source.to_path_buf()
        } else {
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/execution_setup.rs")
        };
        source
            .canonicalize()
            .unwrap()
            .parent()
            .unwrap()
            .join("../../../..")
            .canonicalize()
            .unwrap()
    }
    fn directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "sf-native-connection-proof-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("v2-cloud")).unwrap();
        path
    }
    fn profile() -> Value {
        let candidates: Value = serde_json::from_slice(
            &std::fs::read(root().join("resources/v2-demo/candidate-bindings.json")).unwrap(),
        )
        .unwrap();
        json!({"connection_ref":"native-config-proof", "project_id":"Task8-two-device-six-units", "cloud_project_id":"native-cloud-proof", "endpoint":"https://example.invalid/", "reader_token":"finite-reader", "gateway_token":"finite-gateway", "candidate_bindings":candidates,
            "devices":[{"device_id":"L1-D01","profile":"processor-a","token":"finite-device-a"},{"device_id":"L1-D02","profile":"processor-b","token":"finite-device-b"}]})
    }
    fn host() -> (Arc<Mutex<ProjectHost>>, ObjectId) {
        let ui = id("native-config-ui");
        (
            Arc::new(Mutex::new(ProjectHost::new(
                id("native-config-host"),
                ui.clone(),
            ))),
            ui,
        )
    }
    fn load(
        value: &Value,
        host: Arc<Mutex<ProjectHost>>,
    ) -> Result<Option<NativeExecutionInstallation>, ApiError> {
        let path = directory();
        std::fs::write(
            path.join("v2-cloud/connection-multistage.json"),
            serde_json::to_vec(value).unwrap(),
        )
        .unwrap();
        NativeExecutionInstallation::load(host, &root().join("resources"), &path)
    }
    fn open_demo(host: &mut ProjectHost, ui: &ObjectId) {
        let selected = host
            .select_directory(
                ui,
                FilePurpose::ImportPackage,
                root().join("resources/v2-demo/project"),
            )
            .unwrap();
        let call = |host: &mut ProjectHost, operation: &str, params: Value| {
            host.dispatch(ui, serde_json::from_value(json!({"api_version":"0.1","request_id":format!("config-{}",NEXT.fetch_add(1,Ordering::Relaxed)),"project_id":null,"scope":{"kind":"workspace"},"operation":operation,"params":params})).unwrap())
        };
        let imported = call(
            host,
            "import_package",
            json!({"selected_file_ref":selected,"import_policy":"open_project","base_revision":null}),
        );
        assert_eq!(imported["outcome"]["status"], "ok", "{imported}");
        let task: TaskHandle = serde_json::from_value(imported["outcome"]["data"].clone()).unwrap();
        let mut work = host.drain_io_work();
        host.finish_io(work.pop().unwrap().run());
        let adopted = call(
            host,
            "apply_commands",
            json!({"base_revision":null,"commands":[{"kind":"adopt_imported_draft","data":{"task_id":task.id,"choices":{"chosen_policy":"open_project","objects_to_create":[],"module_versions":[],"reused_bindings":{"item_types":{},"resources":{},"targets":{},"ports":{}},"selected_assets":[],"accept_missing_as_draft":false,"departure":"discard_after_tasks_terminal"}}}]}),
        );
        assert_eq!(adopted["outcome"]["status"], "ok", "{adopted}");
    }
    #[test]
    fn native_import_checks_project_binding_and_preserves_previous_private_file() {
        let p=profile();let bytes=serde_json::to_vec(&p).unwrap();
        let project=serde_json::from_value(p["project_id"].clone()).unwrap();
        assert_eq!(validate_connection_import(&bytes,&project).unwrap(),2);
        assert!(validate_connection_import(&bytes,&"other-project".to_owned().try_into().unwrap()).is_err());
        let mut invalid=p.clone();invalid["devices"][1]["device_id"]=json!("L1-D01");
        assert!(validate_connection_import(&serde_json::to_vec(&invalid).unwrap(),&project).is_err());
        let path=directory().join("v2-cloud");
        std::fs::write(path.join("connection-multistage.json"),b"previous-private-configuration").unwrap();
        store_connection_import(&bytes,&path).unwrap();
        assert_eq!(std::fs::read(path.join("connection-multistage.json")).unwrap(),bytes);
        assert_eq!(std::fs::read(path.join("connection-before-import.json")).unwrap(),b"previous-private-configuration");
        assert!(!path.join("connection-import.pending").exists());
    }
    #[test]
    fn factory_profiles_accept_multiple_stages_per_device_but_reject_missing_scenario_and_duplicates() {
        let mut p = profile();
        p["scenario_id"] = json!("factory-scenario");
        for device in p["devices"].as_array_mut().unwrap() { device["profile"] = json!("factory-commands"); }
        let mut phase = p["candidate_bindings"][0].clone();
        phase["binding_id"] = json!("second-stage");
        phase["nominal_work_ms"] = json!(1700);
        p["candidate_bindings"].as_array_mut().unwrap().push(phase);
        let project = serde_json::from_value(p["project_id"].clone()).unwrap();
        assert_eq!(validate_connection_import(&serde_json::to_vec(&p).unwrap(), &project).unwrap(), 2);
        let mut missing = p.clone(); missing.as_object_mut().unwrap().remove("scenario_id");
        assert!(validate_connection_import(&serde_json::to_vec(&missing).unwrap(), &project).is_err());
        let duplicate = p["devices"][0].clone(); p["devices"].as_array_mut().unwrap().push(duplicate);
        assert!(validate_connection_import(&serde_json::to_vec(&p).unwrap(), &project).is_err());
    }
    #[test]
    fn missing_private_file_leaves_the_original_app_unconfigured() {
        let (host, _) = host();
        assert!(
            NativeExecutionInstallation::load(host, Path::new("missing-resources"), &directory())
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn unknown_private_fields_are_rejected_without_echoing_credentials() {
        let (host, _) = host();
        let mut p = profile();
        p["untrusted_secret_field"] = json!("do-not-echo-value");
        let e = match load(&p, host) {
            Err(e) => e,
            Ok(_) => panic!("unknown configuration field accepted"),
        };
        assert!(matches!(e.code, ErrorCode::InvalidInput));
        assert!(!e.message.contains("do-not-echo"));
        assert!(!e.message.contains("untrusted_secret_field"));
    }
    #[test]
    fn exact_candidate_set_profiles_and_nominal_durations_are_required() {
        for change in 0..3 {
            let mut p = profile();
            match change {
                0 => p["devices"][1]["device_id"] = json!("L1-D01"),
                1 => p["devices"][1]["profile"] = json!("processor-a"),
                _ => p["candidate_bindings"][1]["nominal_work_ms"] = json!(10000),
            }
            let parsed: PrivateConnection = serde_json::from_value(p).unwrap();
            assert!(matches!(
                parsed.validate_bindings().unwrap_err().code,
                ErrorCode::InvalidInput
            ));
        }
    }
    #[test]
    fn four_exact_devices_can_reuse_the_two_trusted_profiles() {
        let mut p = profile();
        for (index, device) in [(0, "L1-D03"), (1, "L1-D04")] {
            let mut candidate = p["candidate_bindings"][index].clone();
            candidate["device_id"] = json!(device);
            candidate["binding_id"] = json!(format!("binding-{device}"));
            p["candidate_bindings"].as_array_mut().unwrap().push(candidate);
            let mut configured = p["devices"][index].clone();
            configured["device_id"] = json!(device);
            p["devices"].as_array_mut().unwrap().push(configured);
        }
        let parsed: PrivateConnection = serde_json::from_value(p).unwrap();
        parsed.validate_bindings().unwrap();
    }
    #[test]
    fn previous_connection_file_is_left_untouched_by_multistage_installation() {
        let (host, _) = host();
        let path = directory();
        let original = path.join("v2-cloud/connection.json");
        let bytes = serde_json::to_vec(&profile()).unwrap();
        std::fs::write(&original, &bytes).unwrap();
        assert!(NativeExecutionInstallation::load(host, Path::new("missing-resources"), &path).unwrap().is_none());
        assert_eq!(std::fs::read(original).unwrap(), bytes);
    }
    #[test]
    fn private_plaintext_origin_is_not_admitted_to_native_runtime() {
        let (host, _) = host();
        let mut p = profile();
        p["endpoint"] = json!("http://127.0.0.1:9/");
        assert!(matches!(
            load(&p, host),
            Err(ApiError {
                code: ErrorCode::InvalidInput,
                ..
            })
        ));
    }
    #[test]
    fn real_saved_project_installs_and_reinstalls_the_same_native_queue_without_io() {
        let (host, ui) = host();
        let installation = load(&profile(), host.clone()).unwrap().unwrap();
        let mut guard = host.lock().unwrap();
        installation
            .install_for_active_project(&mut guard, &ui)
            .unwrap();
        assert!(!guard.execution_preparation_available());
        open_demo(&mut guard, &ui);
        installation
            .install_for_active_project(&mut guard, &ui)
            .unwrap();
        assert!(guard.execution_preparation_available());
        installation.require_settled_for_close().unwrap();
        open_demo(&mut guard, &ui);
        assert!(!guard.execution_preparation_available());
        installation
            .install_for_active_project(&mut guard, &ui)
            .unwrap();
        assert!(guard.execution_preparation_available());
        assert!(!installation.actor.is_finished());
        assert!(installation.actor.status().children.is_empty());
        assert!(!installation.actor.status().has_obligations);
    }
}
