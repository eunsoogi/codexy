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
    _relative: &Path,
    mutable_files: &[&Path],
) -> std::io::Result<()> {
    let seed = private_seed(source)?;
    let _ = mutable_files;
    super::copy_dir(seed, target)
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
