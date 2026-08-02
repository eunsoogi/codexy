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
struct FixtureMaterialization {
    files: u64,
    bytes: u64,
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
fn materialize_seed(
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
fn make_seed_readonly(root: &Path) -> std::io::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::{FixtureMaterialization, make_seed_readonly, materialize_seed};
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    #[test]
    fn clearing_readonly_cannot_mutate_the_seed_or_a_sibling_overlay()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let seed = temp.path().join("seed");
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        let relative = Path::new("agents/codexy-sentinel.toml");
        let original = b"name = \"codexy-sentinel\"\n";
        std::fs::create_dir_all(seed.join("agents"))?;
        std::fs::write(seed.join(relative), original)?;
        make_seed_readonly(&seed)?;
        materialize_seed(&seed, &first, Path::new(""), &[], None)?;
        materialize_seed(&seed, &second, Path::new(""), &[], None)?;

        let first_path = first.join(relative);
        let mut permissions = std::fs::metadata(&first_path)?.permissions();
        #[cfg(unix)]
        permissions.set_mode(permissions.mode() | 0o200);
        #[cfg(windows)]
        permissions.set_readonly(false);
        std::fs::set_permissions(&first_path, permissions)?;
        std::fs::write(first_path, b"name = \"mutated\"\n")?;

        assert_eq!(std::fs::read(seed.join(relative))?, original);
        assert_eq!(std::fs::read(second.join(relative))?, original);
        Ok(())
    }

    #[test]
    fn materialization_profile_counts_each_private_file_and_byte()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let seed = temp.path().join("seed");
        let target = temp.path().join("target");
        std::fs::create_dir_all(seed.join("nested"))?;
        std::fs::write(seed.join("one.txt"), b"one")?;
        std::fs::write(seed.join("nested/two.txt"), b"twenty")?;
        let mut profile = FixtureMaterialization::default();

        materialize_seed(&seed, &target, Path::new(""), &[], Some(&mut profile))?;

        assert_eq!((profile.files, profile.bytes), (2, 9));
        assert_eq!(std::fs::read(target.join("one.txt"))?, b"one");
        assert_eq!(std::fs::read(target.join("nested/two.txt"))?, b"twenty");
        Ok(())
    }
}
