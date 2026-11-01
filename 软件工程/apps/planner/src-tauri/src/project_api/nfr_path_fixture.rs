//! Compiled and registered only with nfr-path-fixture; never mints a selection.
use super::{ProjectApiState, directory_dialog_title, file_dialog_title, host_error};
use sf_commands::{ApiError, ErrorCode, FilePurpose, SelectedFileRef};
use std::path::PathBuf;

fn selected(path: String, directory: bool) -> Result<PathBuf, ApiError> {
    if !std::env::args().any(|arg| arg == "--nfr-path-fixture") {
        return Err(host_error(
            ErrorCode::PermissionDenied,
            "NFR path fixture was not explicitly enabled",
        ));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../.work/noncu-redispatch-20261025/infrastructure/fixtures");
    super::nfr_path_policy::checked_path(&root, &PathBuf::from(path), directory)
        .map_err(|error| host_error(ErrorCode::PermissionDenied, &error.to_string()))
}

#[tauri::command]
pub async fn project_select_fixture_directory(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    purpose: FilePurpose,
    selected_path: String,
) -> Result<SelectedFileRef, ApiError> {
    directory_dialog_title(window.label(), &purpose)?;
    state.register_directory(purpose, Some(selected(selected_path, true)?))
}

#[tauri::command]
pub async fn project_select_fixture_file(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, ProjectApiState>,
    purpose: FilePurpose,
    selected_path: String,
) -> Result<SelectedFileRef, ApiError> {
    file_dialog_title(window.label(), &purpose)?;
    state.register_file(purpose, Some(selected(selected_path, false)?))
}
