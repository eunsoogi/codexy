use std::path::{Path, PathBuf};

pub(super) fn pathdiff(from_dir: &Path, target: &Path) -> String {
    let from = components(from_dir);
    let to = components(target);
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = vec!["..".to_owned(); from.len().saturating_sub(common)];
    parts.extend(to.into_iter().skip(common));
    if parts.is_empty() {
        ".".to_owned()
    } else {
        parts.join("/")
    }
}

pub(super) fn path_join_posix(root: &Path, file: &str) -> PathBuf {
    file.split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

pub(super) fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub(super) fn normalize_posix(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|previous| *previous != "..") {
                    parts.pop();
                } else {
                    parts.push(part);
                }
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

fn components(path: &Path) -> Vec<String> {
    path.components()
        .map(|item| item.as_os_str().to_string_lossy().to_string())
        .collect()
}
