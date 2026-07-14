use std::path::{Component, Path, PathBuf};

use toml::Value;

const TARGET_ROOTS: [&str; 4] = ["src/bin", "tests", "examples", "benches"];

pub(super) fn declared_paths(root: &Path, path: &Path, source: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut attributed_path = None;
    for line in source.lines() {
        let line = line.trim();
        if let Some(path) = path_attribute(line) {
            attributed_path = Some(path);
            continue;
        }
        let declaration = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        let Some(module) = declaration
            .strip_prefix("mod ")
            .and_then(|name| name.strip_suffix(';'))
        else {
            if !line.starts_with("#[") {
                attributed_path = None;
            }
            continue;
        };
        if let Some(attribute) = attributed_path.take() {
            if let Some(path) =
                normalize_relative_path(path.parent().unwrap_or(Path::new("")), attribute)
            {
                paths.push(path);
            }
        } else {
            paths.extend(default_paths(root, path, module));
        }
    }
    paths
}

fn default_paths(root: &Path, path: &Path, module: &str) -> [PathBuf; 2] {
    let parent = path.parent().unwrap_or(Path::new(""));
    let module_parent = if is_crate_root(root, path, parent) {
        parent.to_owned()
    } else {
        parent.join(path.file_stem().unwrap_or_default())
    };
    [
        module_parent.join(format!("{module}.rs")),
        module_parent.join(module).join("mod.rs"),
    ]
}

fn is_crate_root(root: &Path, path: &Path, parent: &Path) -> bool {
    is_manifest_target_root(root, path, parent)
        || (match path.file_name().and_then(|name| name.to_str()) {
            Some("mod.rs") => true,
            Some("lib.rs") => is_library_or_binary_crate_root(root, parent),
            Some("main.rs") => {
                is_library_or_binary_crate_root(root, parent)
                    || is_directory_target_crate_root(root, parent)
            }
            Some("build.rs") => parent == Path::new("") || is_package_root(root, parent),
            _ => false,
        })
        || TARGET_ROOTS
            .iter()
            .any(|directory| parent == Path::new(directory))
        || is_package_target_root(root, parent)
}

fn is_manifest_target_root(root: &Path, path: &Path, parent: &Path) -> bool {
    parent
        .ancestors()
        .find(|candidate| is_package_root(root, candidate))
        .is_some_and(|package_root| {
            let manifest_path = root.join(package_root).join("Cargo.toml");
            let Ok(manifest) = std::fs::read_to_string(manifest_path) else {
                return false;
            };
            let Ok(manifest) = toml::from_str::<Value>(&manifest) else {
                return false;
            };
            manifest_declares_target_path(
                &manifest,
                path.strip_prefix(package_root).unwrap_or(path),
            )
        })
}

fn manifest_declares_target_path(manifest: &Value, path: &Path) -> bool {
    target_path_matches(
        path,
        manifest
            .get("package")
            .and_then(Value::as_table)
            .and_then(|package| package.get("build")),
    ) || ["lib"].iter().any(|kind| {
        target_path_matches(
            path,
            manifest
                .get(kind)
                .and_then(Value::as_table)
                .and_then(|target| target.get("path")),
        )
    }) || ["bin", "test", "example", "bench"].iter().any(|kind| {
        manifest
            .get(kind)
            .and_then(Value::as_array)
            .is_some_and(|targets| {
                targets.iter().any(|target| {
                    target_path_matches(
                        path,
                        target.as_table().and_then(|target| target.get("path")),
                    )
                })
            })
    })
}

fn target_path_matches(path: &Path, target: Option<&Value>) -> bool {
    target
        .and_then(Value::as_str)
        .and_then(|target| normalize_relative_path(Path::new(""), target))
        .is_some_and(|target| target == path)
}

fn normalize_relative_path(base: &Path, path: &str) -> Option<PathBuf> {
    let mut normalized = base.to_owned();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(component) => normalized.push(component),
            Component::ParentDir if !normalized.pop() => return None,
            Component::ParentDir => {}
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!normalized.as_os_str().is_empty()).then_some(normalized)
}

fn path_attribute(line: &str) -> Option<&str> {
    let value = line.strip_prefix("#[path")?.strip_suffix(']')?.trim_start();
    value
        .strip_prefix('=')?
        .trim()
        .strip_prefix('"')?
        .strip_suffix('"')
}

fn is_library_or_binary_crate_root(root: &Path, parent: &Path) -> bool {
    parent == Path::new("")
        || parent == Path::new("src")
        || parent
            .ancestors()
            .find(|candidate| is_package_root(root, candidate))
            .is_some_and(|package_root| parent == package_root.join("src"))
}

fn is_directory_target_crate_root(root: &Path, parent: &Path) -> bool {
    parent
        .ancestors()
        .find(|candidate| is_package_root(root, candidate))
        .is_some_and(|package_root| {
            parent.parent().is_some_and(|target_root| {
                TARGET_ROOTS
                    .iter()
                    .any(|directory| target_root == package_root.join(directory))
            })
        })
}

fn is_package_root(root: &Path, parent: &Path) -> bool {
    root.join(parent).join("Cargo.toml").is_file()
}

fn is_package_target_root(root: &Path, parent: &Path) -> bool {
    parent
        .ancestors()
        .find(|candidate| is_package_root(root, candidate))
        .is_some_and(|package_root| {
            TARGET_ROOTS
                .iter()
                .any(|directory| parent == package_root.join(directory))
        })
}
