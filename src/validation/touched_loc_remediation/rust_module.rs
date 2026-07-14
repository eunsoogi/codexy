use std::path::{Path, PathBuf};

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
    (match path.file_name().and_then(|name| name.to_str()) {
        Some("mod.rs") => true,
        Some("lib.rs" | "main.rs") => is_library_or_binary_crate_root(root, parent),
        Some("build.rs") => parent == Path::new("") || is_package_root(root, parent),
        _ => false,
    }) || TARGET_ROOTS
        .iter()
        .any(|directory| parent == Path::new(directory))
        || is_package_target_root(root, parent)
}

fn is_library_or_binary_crate_root(root: &Path, parent: &Path) -> bool {
    parent == Path::new("")
        || parent == Path::new("src")
        || parent
            .ancestors()
            .find(|candidate| is_package_root(root, candidate))
            .is_some_and(|package_root| parent == package_root.join("src"))
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
