use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use super::super::declaration::{declarations, inline_modules};
use super::super::normalize_relative_path;

pub(super) fn is_path_attributed_module(root: &Path, target: &Path, roots: Vec<PathBuf>) -> bool {
    let mut pending = roots
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
        let parent = module_parent(&path, children_are_siblings);
        if visit_source(root, target, &source, &parent, &mut pending) {
            return true;
        }
    }
    false
}

fn visit_source(
    root: &Path,
    target: &Path,
    source: &str,
    parent: &Path,
    pending: &mut VecDeque<(PathBuf, bool)>,
) -> bool {
    for declaration in declarations(source) {
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
        enqueue_default_children(root, parent, &declaration.module, pending);
    }
    for inline in inline_modules(source) {
        let scope = inline_scope(parent, inline.module, inline.path.as_deref());
        if visit_source(root, target, inline.body, &scope, pending) {
            return true;
        }
    }
    false
}

fn module_parent(path: &Path, children_are_siblings: bool) -> PathBuf {
    let parent = path.parent().unwrap_or(Path::new(""));
    if children_are_siblings {
        parent.to_owned()
    } else {
        parent.join(path.file_stem().unwrap_or_default())
    }
}

fn inline_scope(parent: &Path, module: &str, path: Option<&str>) -> PathBuf {
    path.and_then(|path| normalize_relative_path(parent, path))
        .unwrap_or_else(|| parent.join(module.strip_prefix("r#").unwrap_or(module)))
}

fn enqueue_default_children(
    root: &Path,
    parent: &Path,
    module: &str,
    pending: &mut VecDeque<(PathBuf, bool)>,
) {
    let module = module.strip_prefix("r#").unwrap_or(module);
    for child in [
        parent.join(format!("{module}.rs")),
        parent.join(module).join("mod.rs"),
    ] {
        if root.join(&child).is_file() {
            let siblings = child.file_name().is_some_and(|name| name == "mod.rs");
            pending.push_back((child, siblings));
        }
    }
}
