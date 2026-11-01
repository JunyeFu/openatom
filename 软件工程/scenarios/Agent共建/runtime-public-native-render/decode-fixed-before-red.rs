//! Canonical Host replies and Host-issued read-only leases.
use crate::contract::{response_schema_ref, validate_response};
use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::{Value, json};
use sf_commands::{IpcResponseFrame, OperationName};
use std::io;

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub struct Decoded {
    pub outer: Value,
    pub original: Value,
    pub raw: Option<String>,
    pub images: Vec<ContentBlock>,
}

impl Decoded {
    pub fn mcp(self) -> io::Result<CallToolResult> {
        let mut result = if self.original["outcome"]["status"] == "error" {
            CallToolResult::structured_error(self.outer.clone())
        } else {
            CallToolResult::structured(self.outer.clone())
        };
        if let Some(text) = self.raw {
            let id = self.outer["outcome"]["data"]["result_id"]
                .as_str()
                .ok_or_else(|| invalid("Managed result ID is missing"))?;
            result.content.push(serde_json::from_value(json!({"type":"resource","resource":{"uri":format!("superfactory-result://{id}"),"mimeType":"application/json","text":text}})).map_err(io::Error::other)?);
        }
        result.content.extend(self.images);
        Ok(result)
    }
}

pub fn decode(
    frame: Value,
    request: &Value,
    host: &Value,
    session: &Value,
    mut read: impl FnMut(u64, u64) -> io::Result<Vec<u8>>,
) -> io::Result<Decoded> {
    // Decode the strict envelope before treating any numeric field as a lease.
    let _: IpcResponseFrame<Value> =
        serde_json::from_value(frame.clone()).map_err(io::Error::other)?;
    if frame["wire_version"] != "1"
        || &frame["host_instance_id"] != host
        || &frame["client_session_id"] != session
        || frame["originating_operation"] != request["operation"]
    {
        return Err(invalid("IPC reply identity or operation mismatch"));
    }
    let operation: OperationName =
        serde_json::from_value(request["operation"].clone()).map_err(io::Error::other)?;
    let outer = frame["reply"].clone();
    validate_response(&operation, &outer)?;
    if outer["api_version"] != request["api_version"]
        || outer["request_id"] != request["request_id"]
        || (!outer["context"].is_null()
            && (outer["context"]["host_instance_id"] != *host
                || outer["context"]["session_id"] != *session))
    {
        return Err(invalid("Response correlation or Host context mismatch"));
    }
    let results = frame["managed_results"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let assets = frame["managed_assets"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let mut handles = std::collections::BTreeSet::new();
    for attachment in results.iter().chain(assets) {
        let handle = attachment["handle_value"]
            .as_u64()
            .ok_or_else(|| invalid("Lease handle is not an integer"))?;
        if handle == 0 || !handles.insert(handle) {
            return Err(invalid("Duplicate or null managed lease"));
        }
    }
    let (original, raw) = if outer["outcome"]["status"] == "managed" {
        let metadata = &outer["outcome"]["data"];
        if results.len() != 1
            || results[0]["result_id"] != metadata["result_id"]
            || metadata["response_schema_ref"] != response_schema_ref(&operation)
        {
            return Err(invalid(
                "Managed result metadata/schema/attachment mismatch",
            ));
        }
        let length = metadata["byte_length"]
            .as_u64()
            .ok_or_else(|| invalid("Managed byte length is missing"))?;
        let text = String::from_utf8(read(results[0]["handle_value"].as_u64().unwrap(), length)?)
            .map_err(io::Error::other)?;
        let original: Value = serde_json::from_str(&text).map_err(io::Error::other)?;
        validate_response(&operation, &original)?;
        if original["outcome"]["status"] == "managed"
            || original["api_version"] != outer["api_version"]
            || original["request_id"] != outer["request_id"]
            || original["context"] != outer["context"]
        {
            return Err(invalid(
                "Recovered response is nested or has different correlation/context",
            ));
        }
        (original, Some(text))
    } else {
        if !results.is_empty() {
            return Err(invalid("Inline reply must not carry managed JSON leases"));
        }
        (outer.clone(), None)
    };
    // The shared Host deliberately withholds context on PermissionDenied and
    // unresolved scopes. Preserve that error; never invent a visible context.
    if original["context"].is_null() && original["outcome"]["status"] != "error" {
        return Err(invalid(
            "A successful response must identify its Host context",
        ));
    }
    if matches!(operation, OperationName::GetRun) && original["outcome"]["status"] == "ok" {
        let view = &original["outcome"]["data"];
        let id = if view["kind"] == "full" {
            &view["data"]["id"]
        } else {
            &view["data"]["run_id"]
        };
        if id != &request["params"]["run_id"] {
            return Err(invalid("Recovered RunId differs from the requested run"));
        }
        if view["kind"] == "full" {
            let run = &view["data"];
            if run["input_ref"]["project_id"] != outer["context"]["project_id"] {
                return Err(invalid("Run input project differs from response context"));
            }
        }
    }
    let mut images = Vec::new();
    if assets.len() > 1 {
        return Err(invalid("A render task exposes one scene asset"));
    }
    for attachment in assets {
        let task = &original["outcome"]["data"];
        let output = &task["result"]["data"];
        if !matches!(operation, OperationName::GetTask)
            || original["outcome"]["status"] != "ok"
            || task["task"]["id"] != request["params"]["task_id"]
            || task["task"]["id"] != attachment["task_id"]
            || task["originating_operation"] != "render_view"
            || task["task"]["kind"] != "rendering"
            || task["task"]["state"] != "completed"
            || task["task"]["scope"] != json!({"kind":"edit","data":attachment["scope"]})
            || task["task"]["input_revision"] != attachment["scope_revision"]
            || task["result"]["kind"] != "render"
            || output["scene"]["file"]["id"] != attachment["asset_id"]
            || output["requested_scope"] != attachment["scope"]
            || output["requested_scope_revision"] != attachment["scope_revision"]
            || output["scene"]["scene_revision"] != attachment["scope_revision"]
        {
            return Err(invalid(
                "Image lease is not the requested completed render output",
            ));
        }
        let mime = attachment["mime_type"]
            .as_str()
            .ok_or_else(|| invalid("Image MIME is missing"))?;
        let bytes = read(
            attachment["handle_value"].as_u64().unwrap(),
            attachment["byte_length"]
                .as_u64()
                .ok_or_else(|| invalid("Image length is missing"))?,
        )?;
        let valid = match mime {
            "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            "image/jpeg" => {
                bytes.starts_with(&[0xff, 0xd8, 0xff]) && bytes.ends_with(&[0xff, 0xd9])
            }
            _ => false,
        };
        if !valid {
            return Err(invalid("Image lease does not match its PNG/JPEG MIME"));
        }
        images.push(
            serde_json::from_value(json!({"type":"image","mimeType":mime,"data":base64(&bytes)}))
                .map_err(io::Error::other)?,
        );
    }
    Ok(Decoded {
        outer,
        original,
        raw,
        images,
    })
}

fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        output.push(TABLE[((n >> 18) & 63) as usize] as char);
        output.push(TABLE[((n >> 12) & 63) as usize] as char);
        output.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    output
}

#[cfg(test)]
mod render_tests {
    use super::*;
    fn fixture() -> (Value, Value, Vec<u8>) {
        let request = json!({"api_version":"0.1","request_id":"render-read","project_id":"P","scope":{"kind":"project"},"operation":"get_task","params":{"task_id":"T"}});
        let view = json!({"kind":"line_canvas","canvas":"line_canvas","hall_id":null,"object_ref":null,"project_revision":9,"run_id":null,"view_time_ms":null,"lod":"detailed","camera":{"coordinate_frame":"right_handed_z_up","unit":"meter","eye":{"x":23,"y":17,"z":12},"target":{"x":15,"y":8,"z":1},"zoom":1}});
        let bytes = include_bytes!(
            "../../../scenarios/Agent共建/runtime-public-render-host/control-pixel.png"
        )
        .to_vec();
        let scene = json!({"evidence_origin":"actual_software","id":"scene","file":{"id":"image","path":"rendered/T.png","kind":"png"},"view":view,"scene_revision":9});
        let reply = json!({"api_version":"0.1","request_id":"render-read","context":{"host_instance_id":"host","session_id":"session","project_id":"P","project_revision":9,"scope":{"kind":"project","data":"P"},"scope_revision":9},"outcome":{"status":"ok","data":{"task":{"id":"T","scope":{"kind":"edit","data":{"kind":"project","data":"P"}},"kind":"rendering","state":"completed","input_revision":9,"progress":null,"result_ref":{"kind":"asset","path":"rendered/T.png"},"failure":null},"originating_operation":"render_view","allowed_controls":[],"result":{"kind":"render","data":{"scene":scene,"requested_scope":{"kind":"project","data":"P"},"requested_scope_revision":9,"visible_objects":[],"issues":[]}},"failure":null}}});
        let frame = json!({"wire_version":"1","host_instance_id":"host","client_session_id":"session","originating_operation":"get_task","reply":reply,"managed_assets":[{"task_id":"T","asset_id":"image","scope":{"kind":"project","data":"P"},"scope_revision":9,"mime_type":"image/png","byte_length":bytes.len(),"handle_value":31}]});
        (request, frame, bytes)
    }
    #[test]
    fn completed_scene_revision_must_match_task_and_lease_before_image_read() {
        let (request, mut frame, _) = fixture();
        frame["reply"]["outcome"]["data"]["result"]["data"]["scene"]["scene_revision"] = json!(8);
        assert!(
            decode(
                frame,
                &request,
                &json!("host"),
                &json!("session"),
                |_, _| panic!("A wrong scene revision must not acquire its image handle")
            )
            .is_err()
        );
    }
    #[test]
    fn png_task_and_managed_original_json_are_independent_same_reply_leases() {
        for managed in [false, true] {
            let (request, mut frame, png) = fixture();
            // Reading an admitted R9 scene after editing the same Project to
            // R11 does not change its Task/scene revision or image identity.
            frame["reply"]["context"]["project_revision"] = json!(11);
            frame["reply"]["context"]["scope_revision"] = json!(11);
            let original = serde_json::to_vec(&frame["reply"]).unwrap();
            if managed {
                frame["reply"]["outcome"] = json!({"status":"managed","data":{"result_id":"full-task","response_schema_ref":"packages/commands/generated/schemas/get_task.response.schema.json","byte_length":original.len()}});
                frame["managed_results"] = json!([{"result_id":"full-task","handle_value":32}]);
            }
            let mut reads = vec![];
            let decoded = decode(
                frame,
                &request,
                &json!("host"),
                &json!("session"),
                |handle, length| {
                    reads.push(handle);
                    let bytes = match handle {
                        31 => &png,
                        32 => &original,
                        _ => panic!("foreign handle"),
                    };
                    assert_eq!(length, bytes.len() as u64);
                    Ok(bytes.clone())
                },
            )
            .unwrap();
            assert_eq!(decoded.images.len(), 1);
            assert_eq!(decoded.raw.is_some(), managed);
            let mcp = serde_json::to_value(decoded.mcp().unwrap()).unwrap();
            assert_eq!(
                mcp["content"].as_array().unwrap().last().unwrap()["type"],
                "image"
            );
            assert_eq!(reads, if managed { vec![32, 31] } else { vec![31] });
        }
    }
    #[test]
    fn mismatched_task_asset_scope_state_operation_and_duplicate_handles_are_rejected() {
        for (path, value) in [
            ("/managed_assets/0/task_id", json!("other")),
            ("/managed_assets/0/asset_id", json!("other")),
            (
                "/managed_assets/0/scope",
                json!({"kind":"definition_draft","data":{"id":"D","revision":9}}),
            ),
            ("/managed_assets/0/scope_revision", json!(8)),
            ("/managed_assets/0/handle_value", json!(0)),
            ("/reply/outcome/data/task/state", json!("running")),
            (
                "/reply/outcome/data/originating_operation",
                json!("run_scenario"),
            ),
        ] {
            let (request, mut frame, _) = fixture();
            *frame.pointer_mut(path).unwrap() = value;
            assert!(
                decode(
                    frame,
                    &request,
                    &json!("host"),
                    &json!("session"),
                    |_, _| panic!("{path} must reject before acquisition")
                )
                .is_err(),
                "{path}"
            );
        }
        let (request, mut frame, _) = fixture();
        let duplicate = frame["managed_assets"][0].clone();
        frame["managed_assets"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(
            decode(
                frame,
                &request,
                &json!("host"),
                &json!("session"),
                |_, _| panic!()
            )
            .is_err()
        );
    }
    #[test]
    fn wrong_mime_bytes_and_announced_length_never_emit_an_image() {
        for case in ["mime", "bytes", "length"] {
            let (request, mut frame, mut bytes) = fixture();
            if case == "mime" {
                frame["managed_assets"][0]["mime_type"] = json!("image/jpeg");
            }
            if case == "bytes" {
                bytes = b"not an actual PNG".to_vec();
            }
            if case == "length" {
                frame["managed_assets"][0]["byte_length"] = json!(bytes.len() + 1);
            }
            assert!(
                decode(
                    frame,
                    &request,
                    &json!("host"),
                    &json!("session"),
                    |_, length| {
                        if length != bytes.len() as u64 {
                            return Err(invalid("Actual lease length differs"));
                        }
                        Ok(bytes.clone())
                    }
                )
                .is_err(),
                "{case}"
            );
        }
    }
    #[cfg(windows)]
    #[test]
    fn both_real_readonly_os_handles_feed_one_managed_render_response() {
        use crate::{
            managed_io::{ReceivedLease, publish_readonly},
            pipe_os::{self, Peer, ProcessHandle},
        };
        use windows_sys::Win32::System::Threading::{
            OpenProcess, PROCESS_DUP_HANDLE, PROCESS_QUERY_LIMITED_INFORMATION,
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
        let (request, mut frame, png) = fixture();
        frame["reply"]["outcome"]["data"]["result"]["data"]["issues"] = json!([{
            "code":"controlled_large_transport_fixture","object_ref":null,"field_path":null,
            "message":"x".repeat(crate::framing::MAX_FRAME_BYTES),"blocks":[]
        }]);
        let original = serde_json::to_vec(&frame["reply"]).unwrap();
        assert!(original.len() > crate::framing::MAX_FRAME_BYTES);
        let image = publish_readonly(&png, &peer, &identity).unwrap();
        let result = publish_readonly(&original, &peer, &identity).unwrap();
        frame["managed_assets"][0]["handle_value"] = json!(image);
        frame["reply"]["outcome"] = json!({"status":"managed","data":{"result_id":"actual-handles","response_schema_ref":"packages/commands/generated/schemas/get_task.response.schema.json","byte_length":original.len()}});
        frame["managed_results"] = json!([{"result_id":"actual-handles","handle_value":result}]);
        assert!(serde_json::to_vec(&frame).unwrap().len() < crate::framing::MAX_FRAME_BYTES);
        let decoded = decode(
            frame,
            &request,
            &json!("host"),
            &json!("session"),
            |handle, length| {
                unsafe { ReceivedLease::receive(handle) }?.read_exact_original(
                    handle,
                    length,
                    || false,
                )
            },
        )
        .unwrap();
        assert_eq!(decoded.images.len(), 1);
        assert!(decoded.raw.is_some());
    }
}
