//! Finite admission model against frozen expectations; not live Windows grants/Host.
use serde::Deserialize;
use std::{
    fs,
    io::{self, Cursor, Read},
    path::Path,
};

#[derive(Deserialize)]
struct Input {
    origin: String,
    scope: String,
    requested_project: Option<String>,
    active_project: Option<String>,
    granted_project: Option<String>,
    operation: String,
    operation_allowed: bool,
    implemented: bool,
    requested_revision: Option<u64>,
    actual_revision: u64,
    draft_id: Option<String>,
    allowed_drafts: Vec<String>,
    needs_file: bool,
    selection_id: Option<String>,
    selection_purpose: Option<String>,
    granted_file_purpose: Option<String>,
    session_id: String,
    task_owner_session: Option<String>,
    host_instance: String,
    requested_host_instance: String,
    connection_session: String,
    requested_connection_session: String,
    record_human_ack: bool,
    create_project: bool,
    task_kind: Option<String>,
    task_state: Option<String>,
    control_action: Option<String>,
    protected_parameter_path: bool,
    empty_edit_batch: bool,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    input: Input,
    expected: String,
}
#[derive(Deserialize)]
struct Cases {
    cases: Vec<Case>,
}

fn admit(i: &Input) -> &'static str {
    if i.requested_host_instance != i.host_instance
        || i.requested_connection_session != i.connection_session
        || !i.operation_allowed
    {
        return "PermissionDenied";
    }
    if i.scope == "workspace" {
        if i.requested_project.is_some() {
            return "InvalidInput";
        }
        if !matches!(
            i.operation.as_str(),
            "describe_capabilities"
                | "get_context"
                | "query_module_library"
                | "apply_commands"
                | "import_package"
                | "get_task"
                | "cancel_task"
        ) {
            return "InvalidInput";
        }
        if i.create_project && i.origin != "interactive_ui" {
            return "PermissionDenied";
        }
        if i.operation == "import_package" && i.origin != "interactive_ui" {
            return "PermissionDenied";
        }
        if matches!(i.operation.as_str(), "get_task" | "cancel_task")
            && i.task_owner_session.as_ref() != Some(&i.session_id)
        {
            return "PermissionDenied";
        }
    } else {
        if i.active_project.is_none() {
            return "NoActiveProject";
        }
        if i.requested_project != i.active_project
            || (i.origin == "external_agent" && i.requested_project != i.granted_project)
        {
            return "PermissionDenied";
        }
        if i.scope == "definition_draft"
            && !i
                .draft_id
                .as_ref()
                .is_some_and(|d| i.allowed_drafts.contains(d))
        {
            return "PermissionDenied";
        }
    }
    if i.record_human_ack && i.origin != "interactive_ui" {
        return "PermissionDenied";
    }
    if !i.implemented {
        return "UnsupportedCapability";
    }
    if i.needs_file
        && (i.selection_id.is_none()
            || i.granted_file_purpose.is_none()
            || i.selection_purpose != i.granted_file_purpose)
    {
        return "PermissionDenied";
    }
    if i.requested_revision.is_some_and(|r| r != i.actual_revision) {
        return "RevisionConflict";
    }
    if i.empty_edit_batch || i.protected_parameter_path {
        return "InvalidInput";
    }
    if let Some(action) = i.control_action.as_deref() {
        if i.task_state.as_deref() == Some("completed") {
            return "terminal_completed";
        }
        if action != "cancel" && i.task_kind.as_deref() != Some("simulation") {
            return "UnsupportedCapability";
        }
        if action == "step_next_timestamp" && i.task_state.as_deref() != Some("paused") {
            return "InvalidInput";
        }
    }
    "ok"
}

fn frame(input: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut prefix = [0u8; 4];
    input.read_exact(&mut prefix)?;
    let length = u32::from_le_bytes(prefix) as usize;
    if length == 0 || length > 1_048_576 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid frame length",
        ));
    }
    let mut bytes = vec![0; length];
    input.read_exact(&mut bytes)?;
    std::str::from_utf8(&bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(bytes)
}

pub fn run(root: &Path) {
    let cases: Cases =
        serde_json::from_slice(&fs::read(root.join("api-admission-cases.design.json")).unwrap())
            .unwrap();
    let mut outputs = Vec::new();
    for c in &cases.cases {
        let result = admit(&c.input);
        assert_eq!(result, c.expected, "wrong admission for {}", c.id);
        outputs.push(
            serde_json::json!({"id": c.id, "actual_model_outcome": result, "expected": c.expected}),
        );
    }
    assert_eq!(cases.cases.len(), 26);
    let payload = br#"{"scope":"workspace"}"#;
    let mut bytes = (payload.len() as u32).to_le_bytes().to_vec();
    bytes.extend_from_slice(payload);
    assert_eq!(frame(&mut Cursor::new(&bytes)).unwrap(), payload);
    let mut truncated = Cursor::new(&bytes[..bytes.len() - 1]);
    assert_eq!(
        frame(&mut truncated).unwrap_err().kind(),
        io::ErrorKind::UnexpectedEof
    );
    assert_eq!(
        frame(&mut Cursor::new(1_048_577u32.to_le_bytes()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        frame(&mut Cursor::new(0u32.to_le_bytes()))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let invalid_utf8 = [1, 0, 0, 0, 255];
    assert_eq!(
        frame(&mut Cursor::new(invalid_utf8)).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    fs::write(root.join("output/api-policy-proof-result.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "status": "passed_actual_native_finite_policy_model_and_byte_framing",
        "policy_cases": outputs, "byte_framing_cases": 5,
        "real_Windows_peer_or_Host_authorization": false,
        "product_Commands_implemented": false, "actual_product_runtime": null,
        "scope": "isolated admission model from trusted fixture snapshots; no product call or live MCP"
    })).unwrap()).unwrap();
}
