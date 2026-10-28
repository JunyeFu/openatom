use serde_json::{Value, json};
use sf_mcp_bridge::decode::decode;

fn fixture() -> (Value, Value) {
    let request = json!({"api_version":"0.1","request_id":"request","project_id":null,"scope":{"kind":"workspace"},"operation":"get_object","params":{"object_id":{"kind":"project","id":"missing"},"source":{"kind":"draft"},"node_index":null}});
    let reply = json!({"api_version":"0.1","request_id":"request","context":{"host_instance_id":"host","session_id":"session","project_id":null,"project_revision":null,"scope":null,"scope_revision":null},"outcome":{"status":"error","data":{"code":"PermissionDenied","message":"Actual original error","object_refs":[],"field_errors":[],"expected_revision":null,"actual_revision":null}}});
    (
        request,
        json!({"wire_version":"1","host_instance_id":"host","client_session_id":"session","originating_operation":"get_object","reply":reply,"managed_assets":null}),
    )
}
fn managed(frame: &mut Value) -> Vec<u8> {
    let original = serde_json::to_vec(&frame["reply"]).unwrap();
    frame["reply"]["outcome"] = json!({"status":"managed","data":{"result_id":"result","response_schema_ref":"packages/commands/generated/schemas/get_object.response.schema.json","byte_length":original.len()}});
    frame["managed_results"] = json!([{"result_id":"result","handle_value":123}]);
    original
}
#[test]
fn recovered_error_preserves_original_and_mcp_error_resource() {
    let (request, mut frame) = fixture();
    let bytes = managed(&mut frame);
    let decoded = decode(
        frame,
        &request,
        &json!("host"),
        &json!("session"),
        |handle, length| {
            assert_eq!(handle, 123);
            assert_eq!(length, bytes.len() as u64);
            Ok(bytes.clone())
        },
    )
    .unwrap();
    let result = decoded.mcp().unwrap();
    assert_eq!(result.is_error, Some(true));
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["structuredContent"]["outcome"]["status"], "managed");
    let resource = &result["content"][1]["resource"];
    assert_eq!(resource["uri"], "superfactory-result://result");
    assert_eq!(resource["mimeType"], "application/json");
    assert_eq!(resource["text"], String::from_utf8(bytes).unwrap());
}
#[test]
fn mismatch_is_rejected_before_acquiring_any_numeric_handle() {
    for mutation in [
        "host",
        "session",
        "operation",
        "schema",
        "result_id",
        "duplicate",
    ] {
        let (request, mut frame) = fixture();
        managed(&mut frame);
        match mutation {
            "host" => frame["host_instance_id"] = json!("other"),
            "session" => frame["client_session_id"] = json!("other"),
            "operation" => frame["originating_operation"] = json!("get_context"),
            "schema" => frame["reply"]["outcome"]["data"]["response_schema_ref"] = json!("other"),
            "result_id" => frame["managed_results"][0]["result_id"] = json!("other"),
            _ => frame["managed_results"]
                .as_array_mut()
                .unwrap()
                .push(json!({"result_id":"result","handle_value":123})),
        }
        assert!(
            decode(
                frame,
                &request,
                &json!("host"),
                &json!("session"),
                |_, _| panic!("must not acquire an unbound lease")
            )
            .is_err(),
            "{mutation}"
        );
    }
}
#[test]
fn nested_or_changed_original_response_is_rejected() {
    for mutation in ["nested", "context", "request", "trailing"] {
        let (request, mut frame) = fixture();
        let mut original: Value = serde_json::from_slice(&managed(&mut frame)).unwrap();
        match mutation {
            "nested" => original["outcome"] = frame["reply"]["outcome"].clone(),
            "context" => original["context"]["scope_revision"] = json!(9),
            "request" => original["request_id"] = json!("other"),
            _ => {}
        }
        let mut bytes = serde_json::to_vec(&original).unwrap();
        if mutation == "trailing" {
            bytes.extend_from_slice(b" false");
        }
        frame["reply"]["outcome"]["data"]["byte_length"] = json!(bytes.len());
        assert!(
            decode(
                frame,
                &request,
                &json!("host"),
                &json!("session"),
                |_, _| Ok(bytes.clone())
            )
            .is_err(),
            "{mutation}"
        );
    }
}
#[test]
fn inline_error_is_a_business_error_without_invented_managed_resource() {
    let (request, frame) = fixture();
    let result = decode(
        frame,
        &request,
        &json!("host"),
        &json!("session"),
        |_, _| panic!(),
    )
    .unwrap()
    .mcp()
    .unwrap();
    assert_eq!(result.is_error, Some(true));
    assert_eq!(result.content.len(), 1);
}
#[test]
fn actual_host_permission_error_may_withhold_context_inline_or_managed() {
    for is_managed in [false, true] {
        let (request, mut frame) = fixture();
        frame["reply"]["context"] = Value::Null;
        let bytes = if is_managed {
            managed(&mut frame)
        } else {
            vec![]
        };
        let decoded = decode(
            frame,
            &request,
            &json!("host"),
            &json!("session"),
            |_, _| Ok(bytes.clone()),
        )
        .unwrap();
        assert!(decoded.original["context"].is_null());
        assert_eq!(decoded.mcp().unwrap().is_error, Some(true));
    }
}

#[cfg(windows)]
#[test]
fn actual_os_lease_is_readonly_exact_length_and_closed_after_consumption() {
    use sf_mcp_bridge::{
        managed_io::{ReceivedLease, publish_readonly},
        pipe_os::{self, Peer, ProcessHandle},
    };
    use windows_sys::Win32::{
        Foundation::{GENERIC_WRITE, GetHandleInformation},
        System::Threading::{OpenProcess, PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    let identity = pipe_os::current_identity().unwrap();
    let process = unsafe {
        OpenProcess(
            PROCESS_DUP_HANDLE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            std::process::id(),
        )
    };
    assert!(!process.is_null());
    let peer = Peer {
        identity: identity.clone(),
        process: ProcessHandle(process),
    };
    let bytes = b"actual duplicated OS file";
    let handle = publish_readonly(bytes, &peer, &identity).unwrap();
    // Attempting to duplicate GENERIC_WRITE cannot amplify this read-only lease.
    let mut writable = std::ptr::null_mut();
    let escalation = unsafe {
        windows_sys::Win32::Foundation::DuplicateHandle(
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            handle as usize as _,
            windows_sys::Win32::System::Threading::GetCurrentProcess(),
            &mut writable,
            GENERIC_WRITE,
            0,
            0,
        )
    };
    assert_eq!(escalation, 0);
    let read = unsafe { ReceivedLease::receive(handle) }
        .unwrap()
        .read_exact_original(handle, bytes.len() as u64, || false)
        .unwrap();
    assert_eq!(read, bytes);
    let mut flags = 0;
    assert_eq!(
        unsafe { GetHandleInformation(handle as usize as _, &mut flags) },
        0
    );
    for (length, cancel) in [(bytes.len() as u64 + 1, false), (bytes.len() as u64, true)] {
        let handle = publish_readonly(bytes, &peer, &identity).unwrap();
        assert!(
            unsafe { ReceivedLease::receive(handle) }
                .unwrap()
                .read_exact_original(handle, length, || cancel)
                .is_err()
        );
        assert_eq!(
            unsafe { GetHandleInformation(handle as usize as _, &mut flags) },
            0
        );
    }
}
