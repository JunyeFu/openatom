//! Private read of the actual same-Run output port counters.
use super::*;
use sf_commands::RunPortOutput;
use sf_domain::{PortRef, RunId, SimTimeMs};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunPortOutputReadRequest {
    project_id: ProjectId,
    scope: ApiScope,
    scope_revision: Revision,
    run_id: RunId,
    at_ms: SimTimeMs,
    port: PortRef,
}

impl ProjectApiState {
    fn read_run_port_outputs_ui(
        &self,
        window_label: &str,
        requests: &[RunPortOutputReadRequest],
    ) -> Vec<Result<RunPortOutput, ApiError>> {
        requests
            .iter()
            .map(|request| self.read_run_port_output_ui(window_label, request))
            .collect()
    }

    fn read_run_port_output_ui(
        &self,
        window_label: &str,
        request: &RunPortOutputReadRequest,
    ) -> Result<RunPortOutput, ApiError> {
        if window_label != "main" {
            return Err(host_error(
                ErrorCode::PermissionDenied,
                "Only the main desktop window may read actual Run port output",
            ));
        }
        self.host
            .lock()
            .map_err(|_| {
                host_error(
                    ErrorCode::ComputationFailed,
                    "Managed Project Host state is poisoned",
                )
            })?
            .read_run_port_output(
                &self.ui_session_id,
                &request.project_id,
                &request.scope,
                &request.scope_revision,
                &request.run_id,
                &request.at_ms,
                &request.port,
            )
    }
}

#[tauri::command]
pub fn project_read_run_port_output(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: RunPortOutputReadRequest,
) -> Result<RunPortOutput, ApiError> {
    state.read_run_port_output_ui(window.label(), &request)
}

#[tauri::command(async)]
pub fn project_read_run_port_outputs(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    requests: Vec<RunPortOutputReadRequest>,
) -> Vec<Result<RunPortOutput, ApiError>> {
    state.read_run_port_outputs_ui(window.label(), &requests)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> Value {
        json!({"project_id":"P1","scope":{"kind":"project"},"scope_revision":1,
            "run_id":"R1","at_ms":75000,"port":{"module_ref":{"id":"device","version":"1.0.0"},"instance_id":"D1","port_id":"output"}})
    }

    #[test]
    fn run_port_output_boundary_has_no_caller_path_or_quantity_override() {
        let original = request();
        serde_json::from_value::<RunPortOutputReadRequest>(original.clone()).unwrap();
        for key in [
            "caller",
            "session_id",
            "path",
            "completed_output",
            "accepted_output",
        ] {
            let mut value = original.clone();
            value[key] = json!("not permitted");
            assert!(serde_json::from_value::<RunPortOutputReadRequest>(value).is_err());
        }
    }

    #[test]
    fn run_port_output_boundary_requires_main_before_source_lookup() {
        let state = ProjectApiState::new();
        let parsed = serde_json::from_value(request()).unwrap();
        let failure = state.read_run_port_output_ui("agent", &parsed).unwrap_err();
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    }

    #[test]
    fn batch_preserves_each_original_admission_and_error() {
        let state = ProjectApiState::new();
        let mut other = request();
        other["project_id"] = json!("P2");
        other["scope_revision"] = json!(2);
        let requests: Vec<RunPortOutputReadRequest> =
            serde_json::from_value(json!([request(), other])).unwrap();
        for label in ["agent", "main"] {
            let batch = state.read_run_port_outputs_ui(label, &requests);
            assert_eq!(batch.len(), requests.len());
            for (request, actual) in requests.iter().zip(batch) {
                let expected = state.read_run_port_output_ui(label, request);
                assert!(actual.is_err());
                assert_eq!(
                    serde_json::to_value(actual).unwrap(),
                    serde_json::to_value(expected).unwrap()
                );
            }
        }
    }
}
