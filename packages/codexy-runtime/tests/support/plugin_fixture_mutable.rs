use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static FILES: OnceLock<Mutex<BTreeMap<PathBuf, Vec<PathBuf>>>> = OnceLock::new();

pub(super) fn files(root: &Path) -> Option<Vec<PathBuf>> {
    FILES
        .get_or_init(|| Mutex::new(BTreeMap::new()))
        .lock()
        .ok()
        .and_then(|fixtures| fixtures.get(root).cloned())
}

pub(super) fn record(root: &Path, mutable_files: &[&Path]) {
    let mut declared = mutable_files
        .iter()
        .map(|path| normalized(path))
        .collect::<Vec<_>>();
    declared.sort();
    declared.dedup();
    if let Ok(mut fixtures) = FILES.get_or_init(|| Mutex::new(BTreeMap::new())).lock() {
        fixtures.insert(root.to_path_buf(), declared);
    }
}

pub(crate) fn normalized(path: &Path) -> PathBuf {
    path.components()
        .map(|component| component.as_os_str())
        .collect()
}
