use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use toml::Value as TomlValue;

pub(super) fn cargo_manifest_paths(root: &Path) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    let mut pending = VecDeque::from([(root.to_owned(), false)]);
    while let Some((directory, inside_workspace)) = pending.pop_front() {
        let is_root = directory == root;
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
        let declares_workspace = manifest
            .as_ref()
            .is_some_and(|path| manifest_declares_workspace(path));
        if let Some(manifest) = manifest.as_ref() {
            if !inside_workspace || declares_workspace {
                manifests.push(manifest.to_owned());
            }
        }
        if is_root || manifest.is_none() {
            let child_inside_workspace = inside_workspace || declares_workspace;
            pending.extend(
                child_directories
                    .into_iter()
                    .map(|path| (path, child_inside_workspace)),
            );
        }
    }
    manifests
}

fn manifest_declares_workspace(path: &Path) -> bool {
    let Ok(source) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(manifest) = toml::from_str::<TomlValue>(&source) else {
        return false;
    };
    manifest.get("workspace").is_some_and(TomlValue::is_table)
}
