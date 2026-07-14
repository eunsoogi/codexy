use std::collections::VecDeque;
use std::path::{Path, PathBuf};

pub(super) fn cargo_manifest_paths(root: &Path) -> Vec<PathBuf> {
    let root_manifest = root.join("Cargo.toml");
    if root_manifest.is_file() {
        return vec![root_manifest];
    }
    let mut manifests = Vec::new();
    let mut pending = VecDeque::from([root.to_owned()]);
    while let Some(directory) = pending.pop_front() {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        let mut manifest = None;
        let mut child_directories = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if path.file_name().is_some_and(|name| name == "Cargo.toml") && file_type.is_file() {
                manifest = Some(path);
            } else if file_type.is_dir()
                && !path
                    .file_name()
                    .is_some_and(|name| name == ".git" || name == "target")
            {
                child_directories.push(path);
            }
        }
        if let Some(manifest) = manifest {
            manifests.push(manifest);
        } else {
            pending.extend(child_directories);
        }
    }
    manifests
}
