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

pub(crate) fn copy_wrapper_surface(
    source_root: &std::path::Path,
    target_root: &std::path::Path,
) -> std::io::Result<()> {
    copy_dir(source_root.join("mcp"), &target_root.join("mcp"))?;
    copy_dir(
        source_root.join(".codex-plugin"),
        &target_root.join(".codex-plugin"),
    )
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

#[cfg(not(target_os = "macos"))]
fn clone_seed_file(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    super::profile_metrics::record("fixture_copy_file");
    std::fs::copy(source, target).map(|_| ())
}
