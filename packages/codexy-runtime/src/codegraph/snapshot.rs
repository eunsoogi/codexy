use std::fs;
use std::path::Path;

use ignore::WalkBuilder;
use sha2::{Digest, Sha256};

use super::files::to_posix;

#[derive(Debug)]
pub(super) struct FileSnapshot {
    pub(super) files: Vec<String>,
    pub(super) environment_digest: [u8; 32],
}

pub(super) fn environment_digest(root: &Path) -> [u8; 32] {
    let mut paths = WalkBuilder::new(root)
        .hidden(false)
        .standard_filters(true)
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            name != ".git" && name != "node_modules"
        })
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter(|entry| is_environment_file(entry.path()))
        .filter_map(|entry| entry.into_path().canonicalize().ok())
        .collect::<Vec<_>>();
    let git_exclude = root.join(".git/info/exclude");
    if git_exclude.is_file() {
        paths.push(git_exclude);
    }
    paths.sort();

    let mut hasher = Sha256::new();
    for path in paths {
        let relative = path
            .strip_prefix(root)
            .map_or_else(|_| to_posix(&path), to_posix);
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        match fs::read(&path) {
            Ok(contents) => hasher.update(contents),
            Err(error) => hasher.update(error.to_string().as_bytes()),
        }
        hasher.update([0xff]);
    }
    hasher.finalize().into()
}

fn is_environment_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".gitignore") | Some(".ignore") | Some("go.mod")
    )
}
