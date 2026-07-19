pub(crate) fn copy_dir(
    source: impl AsRef<std::path::Path>,
    target: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            clone_seed_file(&source_path, &target_path)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn clone_seed_file(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    if source.ends_with("assets/codexy-agent-hero.png")
        && std::fs::hard_link(source, target).is_ok()
    {
        return Ok(());
    }
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
    if source.ends_with("assets/codexy-agent-hero.png")
        && std::fs::hard_link(source, target).is_ok()
    {
        return Ok(());
    }
    std::fs::copy(source, target).map(|_| ())
}
