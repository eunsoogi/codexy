use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

mod manifest;
mod traversal;

use super::{TARGET_ROOTS, normalize_relative_path};
use manifest::cargo_manifest_paths;

pub(super) fn is_manifest_target_root(root: &Path, path: &Path) -> bool {
    cargo_target_paths(root)
        .paths
        .iter()
        .any(|target| target == path)
        || ancestor_manifest_declares_target(root, path)
}

pub(super) fn is_path_attributed_module(root: &Path, target: &Path) -> bool {
    traversal::is_path_attributed_module(root, target, crate_roots(root, target))
}

pub(super) fn allows_default_target_roots(root: &Path, path: &Path) -> bool {
    let Some(package_root) = path
        .ancestors()
        .find(|candidate| is_package_root(root, candidate))
    else {
        return true;
    };
    !cargo_target_paths(root)
        .package_roots
        .iter()
        .any(|known_root| known_root == package_root)
}

fn crate_roots(root: &Path, target: &Path) -> Vec<PathBuf> {
    let cargo_targets = cargo_target_paths(root);
    let mut roots = cargo_targets.paths;
    if roots.is_empty() {
        roots.extend(default_package_roots(root, Path::new("")));
    }
    if let Some(package_root) = target
        .ancestors()
        .find(|candidate| is_package_root(root, candidate))
        .filter(|package_root| {
            !cargo_targets
                .package_roots
                .iter()
                .any(|known_root| known_root == package_root)
        })
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

struct CargoTargets {
    paths: Vec<PathBuf>,
    package_roots: Vec<PathBuf>,
}

fn cargo_target_paths(root: &Path) -> CargoTargets {
    let mut targets = cargo_manifest_paths(root)
        .into_iter()
        .map(|manifest| cargo_target_paths_for_manifest(root, &manifest))
        .fold(
            CargoTargets {
                paths: Vec::new(),
                package_roots: Vec::new(),
            },
            |mut targets, discovered| {
                targets.paths.extend(discovered.paths);
                targets.package_roots.extend(discovered.package_roots);
                targets
            },
        );
    targets.paths.sort();
    targets.paths.dedup();
    targets.package_roots.sort();
    targets.package_roots.dedup();
    targets
}

fn cargo_target_paths_for_manifest(root: &Path, manifest: &Path) -> CargoTargets {
    let Ok(output) = Command::new("cargo")
        .args(["metadata", "--manifest-path"])
        .arg(manifest)
        .args(["--offline", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
    else {
        return CargoTargets {
            paths: Vec::new(),
            package_roots: Vec::new(),
        };
    };
    if !output.status.success() {
        return CargoTargets {
            paths: Vec::new(),
            package_roots: Vec::new(),
        };
    }
    let Ok(metadata) = serde_json::from_slice::<JsonValue>(&output.stdout) else {
        return CargoTargets {
            paths: Vec::new(),
            package_roots: Vec::new(),
        };
    };
    let mut discovered = CargoTargets {
        paths: Vec::new(),
        package_roots: Vec::new(),
    };
    for package in metadata
        .get("packages")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
    {
        let Some(manifest) = package
            .get("manifest_path")
            .and_then(JsonValue::as_str)
            .and_then(|manifest| repository_relative_path(root, manifest))
        else {
            continue;
        };
        discovered
            .package_roots
            .push(manifest.parent().unwrap_or(Path::new("")).to_owned());
        discovered.paths.extend(
            package
                .get("targets")
                .and_then(JsonValue::as_array)
                .into_iter()
                .flatten()
                .filter_map(|target| target.get("src_path").and_then(JsonValue::as_str))
                .filter_map(|target| repository_relative_path(root, target)),
        );
    }
    discovered
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
