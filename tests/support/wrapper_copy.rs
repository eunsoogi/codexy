pub(crate) fn copy_dir(
    source: impl AsRef<std::path::Path>,
    target: &std::path::Path,
) -> std::io::Result<()> {
    super::profile_metrics::record("fixture_copy_dir");
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            if is_generated_fixture_directory(&source_path) {
                continue;
            }
            copy_dir(&source_path, &target_path)?;
        } else {
            clone_seed_file(&source_path, &target_path)?;
        }
    }
    Ok(())
}

pub(super) fn is_generated_fixture_directory(path: &std::path::Path) -> bool {
    path.file_name().is_some_and(|name| name == "__pycache__")
}

#[cfg(target_os = "macos")]
fn clone_seed_file(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    super::profile_metrics::record("fixture_copy_file");
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let source_c = CString::new(source.as_os_str().as_bytes())?;
    let target_c = CString::new(target.as_os_str().as_bytes())?;
    // SAFETY: both pointers are NUL-terminated paths valid for this call.
    if unsafe { libc::clonefile(source_c.as_ptr(), target_c.as_ptr(), 0) } == 0 {
        return Ok(());
    }
    std::fs::copy(source, target).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::copy_dir;

    #[test]
    fn fixture_copy_omits_generated_python_bytecode() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::create_dir_all(source.join("codexy_policy/__pycache__"))?;
        std::fs::write(
            source.join("codexy_policy/filesystem_state.py"),
            "state = 'source'\n",
        )?;
        std::fs::write(
            source.join("codexy_policy/__pycache__/filesystem_state.pyc"),
            b"bytecode",
        )?;

        copy_dir(&source, &target)?;

        assert!(target.join("codexy_policy/filesystem_state.py").is_file());
        assert!(!target.join("codexy_policy/__pycache__").exists());
        Ok(())
    }
}

#[cfg(not(target_os = "macos"))]
fn clone_seed_file(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    super::profile_metrics::record("fixture_copy_file");
    std::fs::copy(source, target).map(|_| ())
}
