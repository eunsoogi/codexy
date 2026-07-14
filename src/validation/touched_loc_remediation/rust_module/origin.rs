use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

use super::declaration::declarations;
use super::{TARGET_ROOTS, normalize_relative_path};

pub(super) fn is_manifest_target_root(root: &Path, path: &Path) -> bool {
    cargo_target_paths(root).iter().any(|target| target == path)
        || ancestor_manifest_declares_target(root, path)
}

pub(super) fn is_path_attributed_module(root: &Path, target: &Path) -> bool {
    let mut pending = crate_roots(root, target)
        .into_iter()
        .map(|path| (path, true))
        .collect::<VecDeque<_>>();
    let mut visited = HashSet::new();
    while let Some((path, children_are_siblings)) = pending.pop_front() {
        if !visited.insert(path.clone()) {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(root.join(&path)) else {
            continue;
        };
        let parent = path.parent().unwrap_or(Path::new(""));
        for declaration in declarations(&source) {
            if let Some(attribute) = declaration.path {
                let Some(child) = normalize_relative_path(parent, &attribute) else {
                    continue;
                };
                if child == target {
                    return true;
                }
                if root.join(&child).is_file() {
                    pending.push_back((child, true));
                }
                continue;
            }
            let module = declaration
                .module
                .strip_prefix("r#")
                .unwrap_or(&declaration.module);
            let module_parent = if children_are_siblings {
                parent.to_owned()
            } else {
                parent.join(path.file_stem().unwrap_or_default())
            };
            for child in [
                module_parent.join(format!("{module}.rs")),
                module_parent.join(module).join("mod.rs"),
            ] {
                if root.join(&child).is_file() {
                    let child_is_mod = child.file_name().is_some_and(|name| name == "mod.rs");
                    pending.push_back((child, child_is_mod));
                }
            }
        }
    }
    false
}

fn crate_roots(root: &Path, target: &Path) -> Vec<PathBuf> {
    let mut roots = cargo_target_paths(root);
    if roots.is_empty() {
        roots.extend(default_package_roots(root, Path::new("")));
    }
    if let Some(package_root) = target
        .ancestors()
        .find(|candidate| is_package_root(root, candidate))
    {
        roots.extend(default_package_roots(root, package_root));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn default_package_roots(root: &Path, package_root: &Path) -> Vec<PathBuf> {
    let mut roots = ["lib.rs", "main.rs", "build.rs", "src/lib.rs", "src/main.rs"]
        .iter()
        .map(|path| package_root.join(path))
        .filter(|path| root.join(path).is_file())
        .collect::<Vec<_>>();
    for target_root in TARGET_ROOTS.map(|path| package_root.join(path)) {
        let Ok(entries) = std::fs::read_dir(root.join(&target_root)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "rs") {
                if let Ok(path) = path.strip_prefix(root) {
                    roots.push(path.to_owned());
                }
            } else if path.is_dir() {
                let main = path.join("main.rs");
                if main.is_file() {
                    if let Ok(path) = main.strip_prefix(root) {
                        roots.push(path.to_owned());
                    }
                }
            }
        }
    }
    roots
}

fn cargo_target_paths(root: &Path) -> Vec<PathBuf> {
    let Ok(output) = Command::new("cargo")
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let Ok(metadata) = serde_json::from_slice::<JsonValue>(&output.stdout) else {
        return Vec::new();
    };
    metadata
        .get("packages")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter(|package| {
            package
                .get("manifest_path")
                .and_then(JsonValue::as_str)
                .is_some_and(|manifest| repository_relative_path(root, manifest).is_some())
        })
        .flat_map(|package| {
            package
                .get("targets")
                .and_then(JsonValue::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|target| target.get("src_path").and_then(JsonValue::as_str))
        .filter_map(|target| repository_relative_path(root, target))
        .collect()
}

fn repository_relative_path(root: &Path, target: &str) -> Option<PathBuf> {
    let target = Path::new(target);
    let relative = target.strip_prefix(root).ok().or_else(|| {
        let canonical_root = root.canonicalize().ok()?;
        target.strip_prefix(canonical_root).ok()
    })?;
    normalize_relative_path(Path::new(""), relative.to_str()?)
}

fn ancestor_manifest_declares_target(root: &Path, path: &Path) -> bool {
    path.ancestors()
        .find(|candidate| is_package_root(root, candidate))
        .is_some_and(|package_root| {
            let Ok(source) = std::fs::read_to_string(root.join(package_root).join("Cargo.toml"))
            else {
                return false;
            };
            let Ok(manifest) = toml::from_str::<TomlValue>(&source) else {
                return false;
            };
            declared_target_paths(&manifest, package_root)
                .iter()
                .any(|target| target == path)
        })
}

fn declared_target_paths(manifest: &TomlValue, package_root: &Path) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    if let Some(target) = manifest
        .get("package")
        .and_then(TomlValue::as_table)
        .and_then(|package| package.get("build"))
    {
        targets.push(target);
    }
    if let Some(target) = manifest
        .get("lib")
        .and_then(TomlValue::as_table)
        .and_then(|target| target.get("path"))
    {
        targets.push(target);
    }
    for kind in ["bin", "test", "example", "bench"] {
        if let Some(entries) = manifest.get(kind).and_then(TomlValue::as_array) {
            targets.extend(
                entries
                    .iter()
                    .filter_map(|target| target.as_table().and_then(|target| target.get("path"))),
            );
        }
    }
    targets
        .into_iter()
        .filter_map(TomlValue::as_str)
        .filter_map(|target| normalize_relative_path(package_root, target))
        .collect()
}

fn is_package_root(root: &Path, package_root: &Path) -> bool {
    root.join(package_root).join("Cargo.toml").is_file()
}
