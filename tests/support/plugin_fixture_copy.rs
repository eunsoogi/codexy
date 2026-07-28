use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::sync::{Mutex, OnceLock};

#[cfg(windows)]
struct PrivateSeed {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

#[cfg(windows)]
static PRIVATE_SEED: OnceLock<Mutex<Option<PrivateSeed>>> = OnceLock::new();

pub(super) fn materialize(
    source: PathBuf,
    target: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    #[cfg(not(windows))]
    {
        let _ = mutable_files;
        return super::copy_dir(source, target);
    }
    #[cfg(windows)]
    materialize_windows(&source, target, Path::new(""), mutable_files)
}

#[cfg(windows)]
fn materialize_windows(
    source: &Path,
    target: &Path,
    relative: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    let seed = private_seed(source)?;
    materialize_seed(&seed, target, relative, mutable_files)
}

#[cfg(windows)]
fn private_seed(source: &Path) -> std::io::Result<PathBuf> {
    let seeds = PRIVATE_SEED.get_or_init(|| Mutex::new(None));
    let mut seed = seeds
        .lock()
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "fixture seed lock"))?;
    if seed.is_none() {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("codexy");
        super::copy_dir(source, &root)?;
        *seed = Some(PrivateSeed { _temp: temp, root });
    }
    Ok(seed.as_ref().expect("private fixture seed").root.clone())
}

#[cfg(windows)]
fn materialize_seed(
    source: &Path,
    target: &Path,
    relative: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let entry_relative = relative.join(entry.file_name());
        if source_path.is_dir() {
            materialize_seed(&source_path, &target_path, &entry_relative, mutable_files)?;
        } else if mutable_files.iter().any(|path| *path == entry_relative) {
            std::fs::copy(&source_path, &target_path)?;
            let mut permissions = std::fs::metadata(&target_path)?.permissions();
            permissions.set_readonly(false);
            std::fs::set_permissions(&target_path, permissions)?;
        } else {
            std::fs::hard_link(&source_path, &target_path)?;
            let mut permissions = std::fs::metadata(&target_path)?.permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(&target_path, permissions)?;
            super::profile_metrics::record("fixture_private_seed_link");
        }
    }
    Ok(())
}
