//! Explicit test-build directory picker; still issues production SelectedFileRef grants.
use super::*;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    root: PathBuf,
    directories: BTreeMap<String, PathBuf>,
}
fn selected(config: Selection, purpose: &FilePurpose) -> Result<PathBuf, String> {
    let root = config.root.canonicalize().map_err(|e| e.to_string())?;
    let key = serde_json::to_value(purpose).map_err(|e| e.to_string())?;
    let path = config.directories.get(key.as_str().unwrap())
        .ok_or_else(|| "Test selection has no directory for this purpose".to_string())?
        .canonicalize().map_err(|e| e.to_string())?;
    if path == root || !path.starts_with(&root) || !path.is_dir() {
        return Err("Test directory must be an existing child of the explicit test root".into());
    }
    Ok(path)
}
pub(super) fn configured(purpose: &FilePurpose) -> Result<Option<PathBuf>, ApiError> {
    let Some(file) = std::env::var_os("SUPERFACTORY_TEST_DIRECTORY_SELECTION") else { return Ok(None) };
    let selection = std::fs::read(file).map_err(|e| e.to_string())
        .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|e| e.to_string()))
        .and_then(|config| selected(config, purpose));
    selection.map(Some).map_err(|message| host_error(ErrorCode::InvalidInput, &message))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_test_root_and_purpose_are_enforced() {
        let root=std::env::temp_dir().join(format!("sf-test-picker-{}",std::process::id()));
        std::fs::create_dir_all(root.join("saved")).unwrap();
        let config=|| Selection{root:root.clone(),directories:BTreeMap::from([("save_project".into(),root.join("saved"))])};
        assert_eq!(selected(config(),&FilePurpose::SaveProject).unwrap(),root.join("saved").canonicalize().unwrap());
        assert!(selected(config(),&FilePurpose::ImportPackage).is_err());
        let mut escape=config();escape.directories.insert("save_project".into(),std::env::temp_dir());
        assert!(selected(escape,&FilePurpose::SaveProject).is_err());
        std::fs::remove_dir(root.join("saved")).unwrap();std::fs::remove_dir(root).unwrap();
    }
}
