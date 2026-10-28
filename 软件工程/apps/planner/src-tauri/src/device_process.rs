//! Native-only trusted child ownership. Not a Renderer/Agent command surface.
//! No Engine state, cloud retirement, PID reopening, forced kill, or automatic restart.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

pub const CONTRACT_VERSION: &str = "sf.execution/2.0-draft11";
const PYTHON_FLAGS: [&str; 4] = ["-I", "-B", "-X", "utf8"];
static SPAWN_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static RETAINED: OnceLock<Mutex<Vec<OwnedDevice>>> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceError {
    InvalidNativeConfiguration,
    InvalidBinding,
    MissingTrustedResource,
    InvalidTrustedProfile,
    RuntimeProbeFailed,
    JournalAlreadyExists,
    PrivateDirectoryFailed,
    SpawnFailed,
    BootstrapWriteFailed,
    ControlWriteFailed,
    ControlPending,
    ChannelUncertain,
    InvalidChildMessage,
    ChildReportedError,
    ChildOutputFailed,
    CreationIdentityFailed,
    WaitFailed,
    NotIdle,
    UnsupportedControl,
    Timeout,
    UnexpectedExit,
}
impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for DeviceError {}

/// Construct only from Native-owned fixed configuration, never an IPC request field.
pub struct NativeCloudOrigin(String);
impl NativeCloudOrigin {
    pub fn from_native_configuration(value: String) -> Result<Self, DeviceError> {
        let authority = value
            .strip_prefix("https://")
            .ok_or(DeviceError::InvalidNativeConfiguration)?;
        let authority = authority.strip_suffix('/').unwrap_or(authority);
        if authority.is_empty()
            || authority.starts_with(':')
            || !authority
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
        {
            return Err(DeviceError::InvalidNativeConfiguration);
        }
        Ok(Self(format!("https://{authority}")))
    }
}
impl fmt::Debug for NativeCloudOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NativeCloudOrigin([redacted])")
    }
}

/// No Serialize, Display or revealed Debug. The caller obtains this from Native secret storage.
pub struct DeviceToken(String);
impl DeviceToken {
    pub fn from_native_secret(value: String) -> Result<Self, DeviceError> {
        if value.is_empty() || value.bytes().any(|b| matches!(b, 0 | b'\r' | b'\n')) {
            return Err(DeviceError::InvalidNativeConfiguration);
        }
        Ok(Self(value))
    }
}
impl fmt::Debug for DeviceToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DeviceToken([redacted])")
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ExecutionIdentity {
    pub session_id: String,
    pub execution_epoch_id: String,
    pub project_id: String,
    pub host_context_session_id: String,
}
#[derive(Clone, Debug)]
pub struct DeviceBinding {
    pub host_context_session_id: String,
    pub device_id: String,
    pub device_boot_id: String,
    pub dataset_id: String,
    pub identity: Option<ExecutionIdentity>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceProfile {
    ProcessorA,
    ProcessorB,
    FactoryCommands,
    TelemetryOnly,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RuntimeProbe {
    pub python_version: String,
    pub sqlite_version: String,
    pub openssl_version: String,
    pub isolated: bool,
    pub no_site: bool,
    pub thread_ran: bool,
    pub sqlite_roundtrip: bool,
    pub monotonic_advanced: bool,
}

/// Paths come from Tauri's resource_dir/private app_data_dir in the Native setup layer.
/// No per-launch interpreter, script, profile-file, credential-file or URL path is accepted.
pub struct NativeDeviceRuntime {
    resource_root: PathBuf,
    python: PathBuf,
    script: PathBuf,
    private_root: PathBuf,
    origin: NativeCloudOrigin,
    profiles: BTreeMap<String, u64>,
}
impl NativeDeviceRuntime {
    pub fn from_native_paths(
        resource_dir: &Path,
        private_app_data_dir: &Path,
        origin: NativeCloudOrigin,
    ) -> Result<Self, DeviceError> {
        if !resource_dir.is_absolute() || !private_app_data_dir.is_absolute() {
            return Err(DeviceError::InvalidNativeConfiguration);
        }
        let installation = resource_dir
            .canonicalize()
            .map_err(|_| DeviceError::MissingTrustedResource)?;
        let root = installation
            .join("v2-devices")
            .canonicalize()
            .map_err(|_| DeviceError::MissingTrustedResource)?;
        if !root.starts_with(&installation) {
            return Err(DeviceError::MissingTrustedResource);
        }
        let python = fixed_file(&root, "python/python.exe")?;
        let script = fixed_file(&root, "scripts/virtual_device.py")?;
        for name in [
            "python/python314.zip",
            "python/python314.dll",
            "scripts/journal.py",
            "scripts/protocol.py",
            "licenses/PYTHON-LICENSE.txt",
        ] {
            fixed_file(&root, name)?;
        }
        let pth = fixed_file(&root, "python/python314._pth")?;
        if fs::read_to_string(pth)
            .map_err(|_| DeviceError::MissingTrustedResource)?
            .lines()
            .collect::<Vec<_>>()
            != ["python314.zip", ".", "../scripts"]
        {
            return Err(DeviceError::InvalidTrustedProfile);
        }
        let profile_path = fixed_file(&root, "scripts/devices.demo.json")?;
        let data: Value = serde_json::from_slice(
            &fs::read(profile_path).map_err(|_| DeviceError::MissingTrustedResource)?,
        )
        .map_err(|_| DeviceError::InvalidTrustedProfile)?;
        if data["contract_version"] != CONTRACT_VERSION || data["source_mode"] != "software_in_loop"
        {
            return Err(DeviceError::InvalidTrustedProfile);
        }
        let mut profiles = BTreeMap::new();
        for item in data["execution_profiles"]
            .as_array()
            .ok_or(DeviceError::InvalidTrustedProfile)?
        {
            let id = item["profile_id"]
                .as_str()
                .ok_or(DeviceError::InvalidTrustedProfile)?;
            let ms = item["processing_time_ms"]
                .as_u64()
                .filter(|v| *v <= 9_007_199_254_740_991)
                .ok_or(DeviceError::InvalidTrustedProfile)?;
            if profiles.insert(id.to_owned(), ms).is_some() {
                return Err(DeviceError::InvalidTrustedProfile);
            }
        }
        if !profiles.contains_key("processor-a") || !profiles.contains_key("processor-b") {
            return Err(DeviceError::InvalidTrustedProfile);
        }
        fs::create_dir_all(private_app_data_dir)
            .map_err(|_| DeviceError::PrivateDirectoryFailed)?;
        let private_app_data = private_app_data_dir
            .canonicalize()
            .map_err(|_| DeviceError::PrivateDirectoryFailed)?;
        let private_root = private_app_data.join("v2-device-journals");
        fs::create_dir_all(&private_root).map_err(|_| DeviceError::PrivateDirectoryFailed)?;
        let private_root = private_root
            .canonicalize()
            .map_err(|_| DeviceError::PrivateDirectoryFailed)?;
        if !private_root.starts_with(&private_app_data) {
            return Err(DeviceError::PrivateDirectoryFailed);
        }
        Ok(Self {
            resource_root: root,
            python,
            script,
            private_root,
            origin,
            profiles,
        })
    }

    pub fn verify_imports(&self) -> Result<RuntimeProbe, DeviceError> {
        const PROBE: &str = r#"import json,ssl,sqlite3,urllib.request,threading,time,platform,sys,virtual_device,journal,protocol
c=sqlite3.connect(':memory:'); c.execute('create table probe(value)'); c.execute('insert into probe values(7)'); ok=c.execute('select value from probe').fetchone()[0]==7; c.close()
ssl.create_default_context(); urllib.request.Request('https://example.invalid')
ran=[]; t=threading.Thread(target=lambda:ran.append(True)); t.start(); t.join()
a=time.monotonic_ns(); time.sleep(.002)
print(json.dumps(dict(python_version=platform.python_version(),sqlite_version=sqlite3.sqlite_version,openssl_version=ssl.OPENSSL_VERSION,isolated=bool(sys.flags.isolated),no_site=bool(sys.flags.no_site),thread_ran=bool(ran),sqlite_roundtrip=ok,monotonic_advanced=time.monotonic_ns()>a)))"#;
        let output = self
            .command()
            .arg("-c")
            .arg(PROBE)
            .output()
            .map_err(|_| DeviceError::RuntimeProbeFailed)?;
        if !output.status.success() {
            return Err(DeviceError::RuntimeProbeFailed);
        }
        let result: RuntimeProbe =
            serde_json::from_slice(&output.stdout).map_err(|_| DeviceError::RuntimeProbeFailed)?;
        if result.python_version != "3.14.9"
            || !result.isolated
            || !result.no_site
            || !result.thread_ran
            || !result.sqlite_roundtrip
            || !result.monotonic_advanced
        {
            return Err(DeviceError::RuntimeProbeFailed);
        }
        Ok(result)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.python);
        command
            .args(PYTHON_FLAGS)
            .current_dir(&self.resource_root)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // Do not inherit Native/cloud credentials or Python configuration. Only OS DLL/root discovery.
        for name in ["SystemRoot", "WINDIR"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW; Child handle remains owned.
        }
        command
    }

    pub fn spawn(
        &self,
        binding: DeviceBinding,
        profile: DeviceProfile,
        token: &DeviceToken,
        accept_commands: bool,
    ) -> Result<ManagedDevice, DeviceError> {
        validate_binding(&binding, profile, accept_commands)?;
        let session = binding
            .identity
            .as_ref()
            .map(|i| i.session_id.as_str())
            .unwrap_or("telemetry-only");
        let parent = self
            .private_root
            .join(component(&binding.host_context_session_id))
            .join(component(session))
            .join(component(&binding.device_id));
        fs::create_dir_all(&parent).map_err(|_| DeviceError::PrivateDirectoryFailed)?;
        if !parent
            .canonicalize()
            .map_err(|_| DeviceError::PrivateDirectoryFailed)?
            .starts_with(&self.private_root)
        {
            return Err(DeviceError::PrivateDirectoryFailed);
        }
        let boot_dir = parent.join(component(&binding.device_boot_id));
        match fs::create_dir(&boot_dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(DeviceError::JournalAlreadyExists);
            }
            Err(_) => return Err(DeviceError::PrivateDirectoryFailed),
        }
        let journal = boot_dir.join("device.sqlite");
        let mut child = self
            .command()
            .arg(&self.script)
            .spawn()
            .map_err(|_| DeviceError::SpawnFailed)?;
        let creation = CreationIdentity {
            process_id: child.id(),
            windows_creation_time_100ns: windows_creation_time(&child),
            spawn_sequence: SPAWN_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        };
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("piped child stdout");
        let stderr = child.stderr.take().expect("piped child stderr");
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                let read = (&mut reader).take(65_537).read_until(b'\n', &mut line);
                match read {
                    Ok(0) => break,
                    Ok(_) if line.len() <= 65_536 => {
                        let _ = sender.send(ChildOutput::Line(line));
                    }
                    _ => {
                        let _ = sender.send(ChildOutput::Failed);
                        break;
                    }
                }
            }
            let _ = sender.send(ChildOutput::Closed);
        });
        let stderr_seen = Arc::new(AtomicBool::new(false));
        let seen = Arc::clone(&stderr_seen);
        thread::spawn(move || {
            let mut stream = stderr;
            let mut buffer = [0u8; 4096];
            while let Ok(count) = stream.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                seen.store(true, Ordering::Relaxed); // Drain, never retain or log raw stderr.
            }
        });
        let mut owned = OwnedDevice {
            child,
            stdin,
            receiver,
            stderr_seen,
            creation,
            binding,
            journal_path: journal,
            profile,
            latest_status: None,
            pending: Some(Pending::Initial),
            shutdown_acknowledged: false,
            shutdown_blocked: false,
            stdout_closed: false,
            closing_started: false,
            exit_status: None,
            fault: None,
        };
        #[cfg(windows)]
        if owned.creation.windows_creation_time_100ns.is_none() {
            owned.fault = Some(DeviceError::CreationIdentityFailed);
        }
        let duration = match profile {
            DeviceProfile::ProcessorA => self.profiles["processor-a"],
            DeviceProfile::ProcessorB => self.profiles["processor-b"],
            DeviceProfile::FactoryCommands => 0,
            DeviceProfile::TelemetryOnly => 0,
        };
        #[derive(Serialize)]
        struct Bootstrap<'a> {
            contract_version: &'static str,
            mode: &'static str,
            base_url: &'a str,
            token: &'a str,
            journal_path: &'a Path,
            device_id: &'a str,
            device_boot_id: &'a str,
            dataset_id: &'a str,
            identity: &'a Option<ExecutionIdentity>,
            processing_time_ms: u64,
            #[serde(skip_serializing_if = "Option::is_none")]
            processing_time_source: Option<&'static str>,
            #[serde(skip_serializing_if="Option::is_none")]
            processing_speed: Option<u64>,
            accept_commands: bool,
        }
        let bootstrap = Bootstrap {
            contract_version: CONTRACT_VERSION,
            mode: if profile == DeviceProfile::TelemetryOnly {
                "telemetry_only"
            } else {
                "execution"
            },
            base_url: &self.origin.0,
            token: &token.0,
            journal_path: &owned.journal_path,
            device_id: &owned.binding.device_id,
            device_boot_id: &owned.binding.device_boot_id,
            dataset_id: &owned.binding.dataset_id,
            identity: &owned.binding.identity,
            processing_time_ms: duration,
            processing_time_source: (profile == DeviceProfile::FactoryCommands).then_some("command"),
            processing_speed: (profile == DeviceProfile::FactoryCommands).then_some(30),
            accept_commands,
        };
        let mut bytes = serde_json::to_vec(&bootstrap).expect("fixed bootstrap serializes");
        bytes.push(b'\n');
        if owned.fault.is_some() {
            owned.stdin.take(); // No bootstrap/action if creation identity could not be established.
        } else if owned.write(&bytes).is_err() {
            owned.fault = Some(DeviceError::BootstrapWriteFailed);
        }
        bytes.fill(0);
        // After spawn, every failure is retained in the returned owner; never lose a live Child in Err.
        Ok(ManagedDevice { owned: Some(owned) })
    }
}

fn fixed_file(root: &Path, relative: &str) -> Result<PathBuf, DeviceError> {
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|_| DeviceError::MissingTrustedResource)?;
    if !path.starts_with(root) || !path.is_file() {
        return Err(DeviceError::MissingTrustedResource);
    }
    Ok(path)
}
fn component(value: &str) -> String {
    // Reversible encoding of an identity, not a hash or a user-selected filesystem path.
    value
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn validate_binding(
    b: &DeviceBinding,
    profile: DeviceProfile,
    accepting: bool,
) -> Result<(), DeviceError> {
    let valid = |s: &str| !s.is_empty() && !s.contains('\0');
    if [
        &b.host_context_session_id,
        &b.device_id,
        &b.device_boot_id,
        &b.dataset_id,
    ]
    .iter()
    .any(|s| !valid(s))
    {
        return Err(DeviceError::InvalidBinding);
    }
    match (&b.identity, profile) {
        (None, DeviceProfile::TelemetryOnly) if !accepting => Ok(()),
        (Some(i), p)
            if p != DeviceProfile::TelemetryOnly
                && i.host_context_session_id == b.host_context_session_id
                && [&i.session_id, &i.execution_epoch_id, &i.project_id]
                    .iter()
                    .all(|s| valid(s)) =>
        {
            Ok(())
        }
        _ => Err(DeviceError::InvalidBinding),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreationIdentity {
    pub process_id: u32,
    pub windows_creation_time_100ns: Option<u64>,
    pub spawn_sequence: u64,
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LocalDeviceState {
    Idle,
    Accepted,
    Started,
    Failed,
}
#[derive(Clone, Debug)]
pub struct DeviceStatus {
    pub registered: bool,
    pub state: LocalDeviceState,
    pub accept_commands: bool,
    pub maintenance: bool,
    pub shutdown_ready: bool,
    pub pending_poll: bool,
    pub pending_events: u64,
    pub sampled_slots: u64,
    pub missed_slots: u64,
    pub transport_unknown: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChildLifecycle {
    Running,
    ChannelUncertain,
    ExitObservedAwaitingOutput,
    ExitedClean,
    ExitedUnreconciled,
}
#[derive(Clone, Debug)]
pub struct ProcessFacts {
    pub creation: CreationIdentity,
    pub device_id: String,
    pub device_boot_id: String,
    pub journal_path: PathBuf,
    pub lifecycle: ChildLifecycle,
    pub status: Option<DeviceStatus>,
    pub shutdown_acknowledged: bool,
    pub shutdown_blocked: bool,
    pub exit_code: Option<i32>,
    pub stderr_seen: bool,
    pub fault: Option<DeviceError>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireStatus {
    event: String,
    device_id: String,
    device_boot_id: String,
    registered: bool,
    state: LocalDeviceState,
    accept_commands: bool,
    maintenance: bool,
    shutdown_ready: bool,
    pending_poll: bool,
    pending_events: u64,
    sampled_slots: u64,
    missed_slots: u64,
    last_error: Option<String>,
}
impl OwnedDevice {
    fn status_message(&self, value: Value) -> Result<DeviceStatus, DeviceError> {
        let s: WireStatus =
            serde_json::from_value(value).map_err(|_| DeviceError::InvalidChildMessage)?;
        if s.event != "status"
            || s.device_id != self.binding.device_id
            || s.device_boot_id != self.binding.device_boot_id
            || (s.last_error.is_some() && s.last_error.as_deref() != Some("transport_unknown"))
            || (s.shutdown_ready
                && (!s.registered
                    || s.state != LocalDeviceState::Idle
                    || s.pending_poll
                    || s.pending_events != 0))
        {
            return Err(DeviceError::InvalidChildMessage);
        }
        Ok(DeviceStatus {
            registered: s.registered,
            state: s.state,
            accept_commands: s.accept_commands,
            maintenance: s.maintenance,
            shutdown_ready: s.shutdown_ready,
            pending_poll: s.pending_poll,
            pending_events: s.pending_events,
            sampled_slots: s.sampled_slots,
            missed_slots: s.missed_slots,
            transport_unknown: s.last_error.is_some(),
        })
    }
    fn write(&mut self, bytes: &[u8]) -> Result<(), DeviceError> {
        let stdin = self.stdin.as_mut().ok_or(DeviceError::ControlWriteFailed)?;
        stdin
            .write_all(bytes)
            .and_then(|_| stdin.flush())
            .map_err(|_| DeviceError::ControlWriteFailed)
    }
    fn on_line(&mut self, bytes: &[u8]) -> Result<(), DeviceError> {
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| DeviceError::InvalidChildMessage)?;
        match value["event"].as_str() {
            Some("status") => {
                let status = self.status_message(value)?;
                if let Some(Pending::SetAccept(expected)) = self.pending {
                    if status.accept_commands != expected {
                        return Err(DeviceError::InvalidChildMessage);
                    }
                }
                if let Some(Pending::SetMaintenance(expected)) = self.pending {
                    if status.maintenance != expected {
                        return Err(DeviceError::InvalidChildMessage);
                    }
                }
                if matches!(self.pending, Some(Pending::Shutdown)) {
                    return Err(DeviceError::InvalidChildMessage);
                }
                self.latest_status = Some(status);
                self.pending = None;
            }
            Some("shutdown_blocked") if matches!(self.pending, Some(Pending::Shutdown)) => {
                if value.as_object().map(|m| m.len()) != Some(2) {
                    return Err(DeviceError::InvalidChildMessage);
                }
                self.latest_status = Some(self.status_message(value["status"].clone())?);
                self.shutdown_blocked = true;
                self.pending = None;
            }
            Some("shutdown")
                if matches!(self.pending, Some(Pending::Shutdown))
                    && value
                        == json!({"event":"shutdown","state":"idle","outbox_settled":true}) =>
            {
                self.shutdown_acknowledged = true;
                self.pending = None;
            }
            Some("error") => return Err(DeviceError::ChildReportedError),
            _ => return Err(DeviceError::InvalidChildMessage),
        }
        Ok(())
    }
    fn facts(&self) -> ProcessFacts {
        let lifecycle = match self.exit_status {
            Some(status) if self.stdout_closed => {
                if status.success() && self.shutdown_acknowledged && self.fault.is_none() {
                    ChildLifecycle::ExitedClean
                } else {
                    ChildLifecycle::ExitedUnreconciled
                }
            }
            Some(_) => ChildLifecycle::ExitObservedAwaitingOutput,
            None if self.fault.is_some() => ChildLifecycle::ChannelUncertain,
            None => ChildLifecycle::Running,
        };
        ProcessFacts {
            creation: self.creation,
            device_id: self.binding.device_id.clone(),
            device_boot_id: self.binding.device_boot_id.clone(),
            journal_path: self.journal_path.clone(),
            lifecycle,
            status: self.latest_status.clone(),
            shutdown_acknowledged: self.shutdown_acknowledged,
            shutdown_blocked: self.shutdown_blocked,
            exit_code: self.exit_status.and_then(|s| s.code()),
            stderr_seen: self.stderr_seen.load(Ordering::Relaxed),
            fault: self.fault,
        }
    }
}
enum ChildOutput {
    Line(Vec<u8>),
    Failed,
    Closed,
}
#[derive(Clone, Copy)]
enum Pending {
    Initial,
    Status,
    SetAccept(bool),
    SetMaintenance(bool),
    Shutdown,
}
struct OwnedDevice {
    child: Child,
    stdin: Option<ChildStdin>,
    receiver: mpsc::Receiver<ChildOutput>,
    stderr_seen: Arc<AtomicBool>,
    creation: CreationIdentity,
    binding: DeviceBinding,
    journal_path: PathBuf,
    profile: DeviceProfile,
    latest_status: Option<DeviceStatus>,
    pending: Option<Pending>,
    shutdown_acknowledged: bool,
    shutdown_blocked: bool,
    stdout_closed: bool,
    closing_started: bool,
    exit_status: Option<ExitStatus>,
    fault: Option<DeviceError>,
}

#[must_use = "Keep the Child owner until normal exit or reclaim it from retained_children"]
pub struct ManagedDevice {
    owned: Option<OwnedDevice>,
}
impl fmt::Debug for ManagedDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ManagedDevice")
            .field("facts", &self.owned.as_ref().map(OwnedDevice::facts))
            .finish()
    }
}
impl ManagedDevice {
    pub fn poll(&mut self) -> ProcessFacts {
        let owned = self.owned.as_mut().expect("owned device");
        loop {
            match owned.receiver.try_recv() {
                Ok(ChildOutput::Line(bytes)) => {
                    if let Err(error) = owned.on_line(&bytes) {
                        owned.fault.get_or_insert(error);
                    }
                }
                Ok(ChildOutput::Failed) => {
                    owned.fault.get_or_insert(DeviceError::ChildOutputFailed);
                }
                Ok(ChildOutput::Closed) | Err(mpsc::TryRecvError::Disconnected) => {
                    owned.stdout_closed = true;
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
            }
        }
        if owned.exit_status.is_none() {
            match owned.child.try_wait() {
                Ok(status) => owned.exit_status = status,
                Err(_) => {
                    owned.fault.get_or_insert(DeviceError::WaitFailed);
                }
            }
        }
        owned.facts()
    }
    fn control(&mut self, value: Value, pending: Pending) -> Result<(), DeviceError> {
        let facts = self.poll();
        let owned = self.owned.as_mut().expect("owned device");
        if facts.fault.is_some() {
            return Err(DeviceError::ChannelUncertain);
        }
        if facts.lifecycle != ChildLifecycle::Running {
            return Err(DeviceError::UnexpectedExit);
        }
        if owned.shutdown_acknowledged
            || (owned.closing_started && matches!(pending, Pending::SetAccept(_) | Pending::SetMaintenance(_)))
        {
            return Err(DeviceError::UnsupportedControl);
        }
        if owned.pending.is_some() {
            return Err(DeviceError::ControlPending);
        }
        let mut bytes = serde_json::to_vec(&value).expect("fixed private control serializes");
        bytes.push(b'\n');
        owned.pending = Some(pending);
        if matches!(pending, Pending::Shutdown) {
            owned.closing_started = true;
        }
        if let Err(error) = owned.write(&bytes) {
            owned.fault = Some(error);
            return Err(error);
        }
        Ok(())
    }
    pub fn request_status(&mut self) -> Result<(), DeviceError> {
        self.control(json!({"action":"status"}), Pending::Status)
    }
    pub fn set_accept_commands(&mut self, value: bool) -> Result<(), DeviceError> {
        if self.owned.as_ref().expect("owned device").profile == DeviceProfile::TelemetryOnly {
            return Err(DeviceError::UnsupportedControl);
        }
        self.control(
            json!({"action":"set_accept_commands","accept_commands":value}),
            Pending::SetAccept(value),
        )
    }
    pub fn set_maintenance(&mut self, value: bool) -> Result<(), DeviceError> {
        if self.owned.as_ref().expect("owned device").profile == DeviceProfile::TelemetryOnly {
            return Err(DeviceError::UnsupportedControl);
        }
        self.control(json!({"action":"set_maintenance","maintenance":value}), Pending::SetMaintenance(value))
    }
    pub fn request_shutdown(&mut self) -> Result<(), DeviceError> {
        let facts = self.poll();
        if facts.lifecycle == ChildLifecycle::ExitedClean {
            return Ok(());
        }
        if facts.shutdown_acknowledged {
            return Ok(());
        }
        let telemetry_only = self.owned.as_ref().expect("owned device").profile
            == DeviceProfile::TelemetryOnly;
        // Telemetry closes new sampling first, then retains its original outbox
        // until shutdown is acknowledged. Execution keeps its idle/settled gate.
        if !facts.status.as_ref().is_some_and(|s| {
            s.state == LocalDeviceState::Idle
                && s.registered
                && !s.pending_poll
                && (telemetry_only || (s.shutdown_ready && s.pending_events == 0))
        }) {
            return Err(DeviceError::NotIdle);
        }
        self.control(json!({"action":"shutdown"}), Pending::Shutdown)
    }
    pub fn wait_for_control(&mut self, timeout: Duration) -> Result<ProcessFacts, DeviceError> {
        let start = Instant::now();
        loop {
            let facts = self.poll();
            if let Some(error) = facts.fault {
                return Err(error);
            }
            if matches!(facts.lifecycle, ChildLifecycle::ExitedUnreconciled) {
                return Err(DeviceError::UnexpectedExit);
            }
            if self.owned.as_ref().expect("owned device").pending.is_none() {
                return Ok(facts);
            }
            if start.elapsed() >= timeout {
                return Err(DeviceError::Timeout);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
    pub fn wait_for_exit(&mut self, timeout: Duration) -> Result<ProcessFacts, DeviceError> {
        let start = Instant::now();
        loop {
            let facts = self.poll();
            match facts.lifecycle {
                ChildLifecycle::ExitedClean => return Ok(facts),
                ChildLifecycle::ExitedUnreconciled => return Err(DeviceError::UnexpectedExit),
                _ => {}
            }
            if start.elapsed() >= timeout {
                return Err(DeviceError::Timeout);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
    /// Release an observed exited OS handle, retaining the journal and explicit exit classification.
    /// This is not cloud retirement and cannot release a running/unknown process.
    pub fn into_exited_facts(mut self) -> Result<ProcessFacts, DeviceError> {
        let facts = self.poll();
        if !matches!(
            facts.lifecycle,
            ChildLifecycle::ExitedClean | ChildLifecycle::ExitedUnreconciled
        ) {
            return Err(DeviceError::NotIdle); // Drop retains the live owner, including stdin.
        }
        self.owned.take();
        Ok(facts)
    }
}
impl Drop for ManagedDevice {
    fn drop(&mut self) {
        if let Some(owned) = self.owned.take() {
            if owned.facts().lifecycle != ChildLifecycle::ExitedClean {
                // Preserve real Child/stdin/receiver, not just its PID. No EOF or kill on Drop.
                RETAINED
                    .get_or_init(|| Mutex::new(Vec::new()))
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(owned);
            }
        }
    }
}
pub fn retained_children() -> Vec<ProcessFacts> {
    RETAINED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .map(OwnedDevice::facts)
        .collect()
}
pub fn reclaim_retained(creation: CreationIdentity) -> Option<ManagedDevice> {
    let mut children = RETAINED
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let index = children
        .iter()
        .position(|child| child.creation == creation)?;
    Some(ManagedDevice {
        owned: Some(children.remove(index)),
    })
}

#[cfg(windows)]
fn windows_creation_time(child: &Child) -> Option<u64> {
    use std::os::windows::io::AsRawHandle;
    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetProcessTimes(
            handle: *mut std::ffi::c_void,
            created: *mut FileTime,
            exited: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
    }
    let (mut created, mut exited, mut kernel, mut user) = (
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
        FileTime::default(),
    );
    // The handle belongs to our actual Child; never opened from a PID or process name.
    let ok = unsafe {
        GetProcessTimes(
            child.as_raw_handle(),
            &mut created,
            &mut exited,
            &mut kernel,
            &mut user,
        )
    };
    (ok != 0).then_some(((created.high as u64) << 32) | created.low as u64)
}
#[cfg(not(windows))]
fn windows_creation_time(_: &Child) -> Option<u64> {
    None
}
