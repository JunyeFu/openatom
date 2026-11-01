//! Main-window-only declaration for this actual immutable export candidate.
use super::*;
use sf_commands::{ExportPackageParams, ManufacturingFileDeclaration};
use sf_domain::TaskHandle;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManufacturingPackageExportRequest {
    project_id: ProjectId,
    scope: ApiScope,
    scope_revision: Revision,
    params: ExportPackageParams,
    declarations: Vec<ManufacturingFileDeclaration>,
    confirm_redistribution: bool,
}

impl ProjectApiState {
    fn export_manufacturing_ui(
        &self,
        window_label: &str,
        request: ManufacturingPackageExportRequest,
    ) -> Result<TaskHandle, ApiError> {
        if window_label != "main" {
            return Err(host_error(
                ErrorCode::PermissionDenied,
                "Only the main desktop window may declare this manufacturing export",
            ));
        }
        let mut host = self.host.lock().map_err(|_| {
            host_error(
                ErrorCode::ComputationFailed,
                "Managed Project Host state is poisoned",
            )
        })?;
        let task = host.export_manufacturing_package(
            &self.ui_session_id,
            &request.project_id,
            &request.scope,
            &request.scope_revision,
            request.params,
            request.declarations,
            request.confirm_redistribution,
        )?;
        let mut queue_failed = false;
        // Preserve the original admission order and lock-free I/O execution.
        for work in host.drain_io_work() {
            if let Err(failed) = self.io_sender.send(work) {
                host.finish_io(failed.0.fail(host_error(
                    ErrorCode::ComputationFailed,
                    "Project I/O worker is no longer available",
                )));
                queue_failed = true;
            }
        }
        drop(host);
        if queue_failed {
            signal_io_completion(&self.io_completed);
        }
        Ok(task)
    }
}

#[tauri::command]
pub fn project_export_manufacturing_package(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    request: ManufacturingPackageExportRequest,
) -> Result<TaskHandle, ApiError> {
    state.export_manufacturing_ui(window.label(), request)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Value {
        json!({
            "project_id":"P1", "scope":{"kind":"project"}, "scope_revision":1,
            "params":{"revision":1,"selected_destination_ref":{"selection_id":"chosen","purpose":"export_package"},
                "package_scope":{"purpose":"private_project","scenarios":[],"run_ids":[],"artifact_ids":["A1"],"module_refs":[],"include_redistributable_assets":true}},
            "declarations":[{"artifact_id":"A1","file":{"path":"manufacturing/cut.svg","kind":"asset"},"source":"Explicit test statement","license":"Explicit test license"}],
            "confirm_redistribution":true
        })
    }

    #[test]
    fn manufacturing_export_boundary_rejects_caller_path_and_bytes_fields() {
        let original = request();
        serde_json::from_value::<ManufacturingPackageExportRequest>(original.clone()).unwrap();
        for key in ["caller", "session_id", "path", "bytes", "asset_id"] {
            let mut value = original.clone();
            value[key] = json!("cannot supply authority or file content");
            assert!(serde_json::from_value::<ManufacturingPackageExportRequest>(value).is_err());
        }
        for key in ["asset_id", "bytes", "caller"] {
            let mut value = original.clone();
            value["declarations"][0][key] = json!("not an actual manufacturing declaration");
            assert!(serde_json::from_value::<ManufacturingPackageExportRequest>(value).is_err());
        }
    }

    #[test]
    fn manufacturing_export_boundary_requires_main_before_project_or_file_lookup() {
        let state = ProjectApiState::new();
        let parsed = serde_json::from_value(request()).unwrap();
        let failure = state.export_manufacturing_ui("agent", parsed).unwrap_err();
        assert!(matches!(failure.code, ErrorCode::PermissionDenied));
    }
}
