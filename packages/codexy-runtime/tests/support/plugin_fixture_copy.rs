use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

#[cfg(windows)]
struct PrivateSeed {
    _temp: tempfile::TempDir,
    root: PathBuf,
}

#[cfg(windows)]
static PRIVATE_SEED: OnceLock<Mutex<Option<PrivateSeed>>> = OnceLock::new();

#[derive(Default)]
pub(crate) struct FixtureMaterialization {
    pub(crate) files: u64,
    pub(crate) bytes: u64,
}

pub(super) fn materialize(
    source: PathBuf,
    target: &Path,
    mutable_files: &[&Path],
    identity: &str,
) -> std::io::Result<()> {
    #[cfg(not(windows))]
    {
        let _ = mutable_files;
        let started = Instant::now();
        super::copy_dir(&source, target)?;
        if super::profile_metrics::enabled() {
            let profile = materialization_profile(target)?;
            super::profile_metrics::record_fixture_materialization(
                identity,
                profile.files,
                profile.bytes,
                started.elapsed().as_secs_f64(),
            );
        }
        return Ok(());
    }
    #[cfg(windows)]
    materialize_windows(&source, target, Path::new(""), mutable_files, identity)
}

#[cfg(windows)]
fn materialize_windows(
    source: &Path,
    target: &Path,
    relative: &Path,
    mutable_files: &[&Path],
    identity: &str,
) -> std::io::Result<()> {
    let seed = private_seed(source)?;
    if super::profile_metrics::enabled() {
        let mut profile = FixtureMaterialization::default();
        let started = Instant::now();
        materialize_seed(&seed, target, relative, mutable_files, Some(&mut profile))?;
        super::profile_metrics::record_fixture_materialization(
            identity,
            profile.files,
            profile.bytes,
            started.elapsed().as_secs_f64(),
        );
    } else {
        materialize_seed(&seed, target, relative, mutable_files, None)?;
    }
    Ok(())
}

fn materialization_profile(root: &Path) -> std::io::Result<FixtureMaterialization> {
    let mut profile = FixtureMaterialization::default();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let nested = materialization_profile(&path)?;
            profile.files += nested.files;
            profile.bytes += nested.bytes;
        } else {
            profile.files += 1;
            profile.bytes += std::fs::metadata(path)?.len();
        }
    }
    Ok(profile)
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
        make_seed_readonly(&root)?;
        *seed = Some(PrivateSeed { _temp: temp, root });
    }
    Ok(seed.as_ref().expect("private fixture seed").root.clone())
}

#[cfg(any(test, windows))]
pub(crate) fn materialize_seed(
    source: &Path,
    target: &Path,
    relative: &Path,
    mutable_files: &[&Path],
    mut profile: Option<&mut FixtureMaterialization>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let entry_relative = relative.join(entry.file_name());
        if source_path.is_dir() {
            materialize_seed(
                &source_path,
                &target_path,
                &entry_relative,
                mutable_files,
                profile.as_deref_mut(),
            )?;
        } else {
            if mutable_files.iter().any(|path| *path == entry_relative) {
                std::fs::copy(&source_path, &target_path)?;
                let mut permissions = std::fs::metadata(&target_path)?.permissions();
                permissions.set_readonly(false);
                std::fs::set_permissions(&target_path, permissions)?;
            } else {
                std::fs::copy(&source_path, &target_path)?;
                super::profile_metrics::record("fixture_private_seed_copy");
            }
            if let Some(profile) = profile.as_deref_mut() {
                profile.files += 1;
                profile.bytes += std::fs::metadata(source_path)?.len();
            }
        }
    }
    Ok(())
}

#[cfg(any(test, windows))]
pub(crate) fn make_seed_readonly(root: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            make_seed_readonly(&path)?;
        } else {
            let mut permissions = std::fs::metadata(&path)?.permissions();
            permissions.set_readonly(true);
            std::fs::set_permissions(path, permissions)?;
        }
    }
    Ok(())
}
