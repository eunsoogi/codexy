use std::path::{Component, Path, PathBuf};

use toml::Value;

const TARGET_ROOTS: [&str; 4] = ["src/bin", "tests", "examples", "benches"];

pub(super) fn paths(root: &Path, path: &Path, module: &str) -> [PathBuf; 2] {
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
                    || is_directory_binary_crate_root(root, parent)
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
        .and_then(normalize_manifest_target_path)
        .is_some_and(|target| target == path)
}

fn normalize_manifest_target_path(target: &str) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in Path::new(target).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(component) => normalized.push(component),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!normalized.as_os_str().is_empty()).then_some(normalized)
}

fn is_library_or_binary_crate_root(root: &Path, parent: &Path) -> bool {
    parent == Path::new("")
        || parent == Path::new("src")
        || parent
            .ancestors()
            .find(|candidate| is_package_root(root, candidate))
            .is_some_and(|package_root| parent == package_root.join("src"))
}

fn is_directory_binary_crate_root(root: &Path, parent: &Path) -> bool {
    parent
        .ancestors()
        .find(|candidate| is_package_root(root, candidate))
        .is_some_and(|package_root| {
            parent
                .parent()
                .is_some_and(|bin_root| bin_root == package_root.join("src/bin"))
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
