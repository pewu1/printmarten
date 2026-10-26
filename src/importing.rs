use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "tif", "tiff", "bmp", "heic", "heif", "hif",
];

fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            IMAGE_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

/// Stable recursive discovery, skipping directory symlinks and duplicate files.
/// Call on the import worker so large folders never block the UI thread.
pub fn discover(paths: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<String>) {
    let mut pending: Vec<_> = paths.into_iter().rev().collect();
    let mut seen = HashSet::new();
    let mut files = vec![];
    let mut errors = vec![];
    while let Some(path) = pending.pop() {
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if metadata.is_symlink() && !path.is_file() {
            continue;
        }
        let canonical = match path.canonicalize() {
            Ok(canonical) => canonical,
            Err(error) => {
                errors.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if !seen.insert(canonical) {
            continue;
        }
        if metadata.is_dir() {
            match std::fs::read_dir(&path) {
                Ok(entries) => {
                    let mut children = vec![];
                    for entry in entries {
                        match entry {
                            Ok(entry) => children.push(entry.path()),
                            Err(error) => errors.push(format!("{}: {error}", path.display())),
                        }
                    }
                    children.sort();
                    pending.extend(children.into_iter().rev());
                }
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        } else if (metadata.is_file() || metadata.is_symlink()) && supported(&path) {
            files.push(path);
        }
    }
    if files.is_empty() && errors.is_empty() {
        errors.push("No supported photos found in the selected files or folders.".into());
    }
    (files, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_are_recursive_sorted_filtered_and_deduplicated() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::fs::create_dir(root.join("nested")).unwrap();
        for file in [
            "b.JPG",
            "a.png",
            "notes.txt",
            "nested/c.HEIC",
            "nested/d.hif",
        ] {
            std::fs::write(root.join(file), []).unwrap();
        }
        let (files, errors) = discover(vec![root.into(), root.join("nested"), root.join("a.png")]);
        assert!(errors.is_empty());
        assert_eq!(
            files,
            ["a.png", "b.JPG", "nested/c.HEIC", "nested/d.hif"].map(|file| root.join(file))
        );
    }

    #[test]
    fn empty_and_missing_folders_are_reported_without_losing_valid_files() {
        let directory = tempfile::tempdir().unwrap();
        assert!(discover(vec![directory.path().into()]).0.is_empty());
        assert!(!discover(vec![directory.path().into()]).1.is_empty());
        let photo = directory.path().join("valid.png");
        std::fs::write(&photo, []).unwrap();
        let (files, errors) = discover(vec![directory.path().join("missing"), photo.clone()]);
        assert_eq!(files, vec![photo]);
        assert_eq!(errors.len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn directory_symlink_cycles_are_not_followed() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("photo.jpg"), []).unwrap();
        std::os::unix::fs::symlink(directory.path(), directory.path().join("loop")).unwrap();
        let (files, errors) = discover(vec![directory.path().into()]);
        assert_eq!(files.len(), 1);
        assert!(errors.is_empty());
    }
}
