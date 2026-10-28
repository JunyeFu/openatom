//! Pure path policy, shared by the test-only Tauri fixture and finite FS tests.
use std::io::{Error, ErrorKind, Result};
use std::path::{Component, Path, PathBuf};

pub fn checked_path(root: &Path, selected: &Path, directory: bool) -> Result<PathBuf> {
    if !selected.is_absolute()
        || selected
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "absolute path without parent traversal required",
        ));
    }
    let root = root.canonicalize()?;
    let resolved = if selected.exists() {
        selected.canonicalize()?
    } else if !directory {
        let parent = selected
            .parent()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing parent"))?
            .canonicalize()?;
        parent.join(
            selected
                .file_name()
                .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing filename"))?,
        )
    } else {
        return Err(Error::new(
            ErrorKind::NotFound,
            "create the owned fixture directory before selecting it",
        ));
    };
    if !resolved.starts_with(&root) {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "selection escapes this batch fixture root",
        ));
    }
    if directory && !resolved.is_dir() || !directory && resolved.is_dir() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "selection kind differs from requested purpose",
        ));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    fn fixture() -> (PathBuf, PathBuf) {
        let base =
            PathBuf::from(std::env::var("SF_POLICY_TEST_ROOT").expect("owned test root required"))
                .join(format!("case-{}", NEXT.fetch_add(1, Ordering::SeqCst)));
        let root = base.join("allowed");
        let other = base.join("outside");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        (root, other)
    }
    #[test]
    fn existing_directory_allowed() {
        let (r, _) = fixture();
        assert_eq!(
            checked_path(&r, &r, true).unwrap(),
            r.canonicalize().unwrap()
        );
    }
    #[test]
    fn existing_and_new_file_allowed() {
        let (r, _) = fixture();
        let f = r.join("a.sfproj");
        std::fs::write(&f, b"unit").unwrap();
        assert!(checked_path(&r, &f, false).is_ok());
        assert!(checked_path(&r, &r.join("new.zip"), false).is_ok());
    }
    #[test]
    fn outside_and_prefix_sibling_denied() {
        let (r, o) = fixture();
        assert_eq!(
            checked_path(&r, &o, true).unwrap_err().kind(),
            ErrorKind::PermissionDenied
        );
        let sibling = r.with_file_name("allowed-other");
        std::fs::create_dir_all(&sibling).unwrap();
        assert!(checked_path(&r, &sibling, true).is_err());
    }
    #[test]
    fn traversal_and_relative_denied() {
        let (r, _) = fixture();
        assert!(checked_path(&r, &r.join("../outside"), true).is_err());
        assert!(checked_path(&r, Path::new("relative"), true).is_err());
    }
    #[test]
    fn missing_directory_and_wrong_kind_denied() {
        let (r, _) = fixture();
        assert!(checked_path(&r, &r.join("missing"), true).is_err());
        assert!(checked_path(&r, &r, false).is_err());
        let f = r.join("file");
        std::fs::write(&f, b"unit").unwrap();
        assert!(checked_path(&r, &f, true).is_err());
    }
    #[test]
    fn missing_parent_not_created_by_selector() {
        let (r, _) = fixture();
        assert!(checked_path(&r, &r.join("absent/new.zip"), false).is_err());
        assert!(!r.join("absent").exists());
    }
}
