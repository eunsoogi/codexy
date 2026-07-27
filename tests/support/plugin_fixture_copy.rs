use std::path::{Path, PathBuf};

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
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        let entry_relative = relative.join(entry.file_name());
        if source_path.is_dir() {
            materialize_windows(&source_path, &target_path, &entry_relative, mutable_files)?;
        } else if mutable_files.iter().any(|path| *path == entry_relative) {
            super::profile_metrics::record("fixture_mutable_copy");
            std::fs::copy(&source_path, &target_path)?;
        } else if std::fs::hard_link(&source_path, &target_path).is_err() {
            super::profile_metrics::record("fixture_copy_fallback");
            std::fs::copy(&source_path, &target_path)?;
        } else {
            super::profile_metrics::record("fixture_hard_link");
        }
    }
    Ok(())
}
