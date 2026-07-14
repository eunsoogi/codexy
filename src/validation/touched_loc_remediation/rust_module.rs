use std::path::{Component, Path, PathBuf};

mod attribute;
mod declaration;
mod origin;
mod scope;

use declaration::declarations;

pub(super) const TARGET_ROOTS: [&str; 4] = ["src/bin", "tests", "examples", "benches"];

pub(super) fn declared_paths(root: &Path, path: &Path, source: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut default_parent = None;
    for declaration in declarations(source) {
        if let Some(attribute) = declaration.path {
            if let Some(module_path) =
                normalize_relative_path(path.parent().unwrap_or(Path::new("")), &attribute)
            {
                if module_path != path {
                    paths.push(module_path);
                }
            }
        } else {
            let module = declaration
                .module
                .strip_prefix("r#")
                .unwrap_or(&declaration.module);
            let module_parent = default_parent.get_or_insert_with(|| module_parent(root, path));
            paths.extend(default_paths(module_parent, module));
        }
    }
    paths
}

fn module_parent(root: &Path, path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new(""));
    if is_crate_root(root, path, parent) || origin::is_path_attributed_module(root, path) {
        parent.to_owned()
    } else {
        parent.join(path.file_stem().unwrap_or_default())
    }
}

fn default_paths(module_parent: &Path, module: &str) -> [PathBuf; 2] {
    [
        module_parent.join(format!("{module}.rs")),
        module_parent.join(module).join("mod.rs"),
    ]
}

fn is_crate_root(root: &Path, path: &Path, parent: &Path) -> bool {
    origin::is_manifest_target_root(root, path)
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

pub(super) fn normalize_relative_path(base: &Path, path: &str) -> Option<PathBuf> {
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
