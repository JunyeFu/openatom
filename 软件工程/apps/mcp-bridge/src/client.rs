#![cfg(windows)]
use crate::{
    decode::{self, Decoded},
    framing::{MAX_FRAME_BYTES, read_frame, write_frame},
    managed_io::ReceivedLease,
    pipe_os,
};
use serde_json::{Value, json};
use sf_commands::{ApiRequest, IpcWelcome};
use std::io;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};

pub struct HostClient {
    pipe: NamedPipeClient,
    pub host: Value,
    pub session: Value,
    runs: crate::run_identity::RunObservations,
    executions: crate::execution_identity::ExecutionObservations,
}
impl HostClient {
    pub async fn connect(endpoint: &str, expected_host: &str) -> io::Result<Self> {
        let pipe = ClientOptions::new().open(endpoint)?;
        let identity = pipe_os::current_identity()?;
        let peer = pipe_os::pipe_peer(&pipe, false, &identity)?;
        if desktop_host_process(expected_host) != Some(peer.identity.pid) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Pipe server is not the selected desktop Host",
            ));
        }
        let mut client = Self {
            pipe,
            host: json!(expected_host),
            session: Value::Null,
            runs: Default::default(),
            executions: Default::default(),
        };
        write_frame(
            &mut client.pipe,
            &json!({"wire_version":"1","expected_host_instance_id":expected_host}),
        )
        .await?;
        let welcome = read_frame(&mut client.pipe).await?;
        let _: IpcWelcome = serde_json::from_value(welcome.clone()).map_err(io::Error::other)?;
        if welcome["wire_version"] != "1"
            || welcome["api_version"] != "0.1"
            || welcome["host_instance_id"] != client.host
            || welcome["max_frame_bytes"] != MAX_FRAME_BYTES
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Canonical IPC welcome mismatch",
            ));
        }
        client.session = welcome["client_session_id"].clone();
        Ok(client)
    }
    pub async fn call(
        &mut self,
        request: &Value,
        cancelled: impl Fn() -> bool,
    ) -> io::Result<Decoded> {
        let request_type: ApiRequest =
            serde_json::from_value(request.clone()).map_err(io::Error::other)?;
        let operation =
            serde_json::from_value(request["operation"].clone()).map_err(io::Error::other)?;
        if !crate::catalog::agent_operation(&operation) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Device Gateway operations are private to the authenticated Gateway",
            ));
        }
        drop(request_type);
        write_frame(&mut self.pipe, &json!({"wire_version":"1","host_instance_id":self.host,"client_session_id":self.session,"request":request})).await?;
        let frame = read_frame(&mut self.pipe).await?;
        let decoded = decode::decode(
            frame,
            request,
            &self.host,
            &self.session,
            |handle, length| {
                unsafe { ReceivedLease::receive(handle) }?
                    .read_exact_original(handle, length, &cancelled)
            },
        )?;
        self.runs.observe(request, &decoded.original)?;
        self.executions.observe(request, &decoded.original)?;
        Ok(decoded)
    }
}

fn desktop_host_process(host: &str) -> Option<u32> {
    // Bind the selected instance to its actual Windows pipe server. The full
    // instance ID (including its startup nonce) is checked in the welcome below.
    host.strip_prefix("desktop-host-")?
        .split('-')
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod host_process_tests {
    use super::desktop_host_process;

    #[test]
    fn selected_instance_keeps_os_process_binding() {
        assert_eq!(desktop_host_process("desktop-host-82912-1793059393000"), Some(82912));
        assert_eq!(desktop_host_process("desktop-host-82912"), Some(82912));
        assert_ne!(desktop_host_process("desktop-host-82913-1793059393000"), Some(82912));
        assert_eq!(desktop_host_process("another-host-82912"), None);
        assert_eq!(desktop_host_process("desktop-host-invalid-1793059393000"), None);
    }
}
