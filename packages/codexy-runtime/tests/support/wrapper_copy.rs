pub(crate) fn copy_dir(
    source: impl AsRef<std::path::Path>,
    target: &std::path::Path,
) -> std::io::Result<()> {
    let source = source.as_ref();
    super::profile_metrics::record("fixture_copy_dir");
    std::fs::create_dir_all(target)
        .map_err(|error| copy_error("create_dir_all", source, target, None, error))?;
    let entries = std::fs::read_dir(source)
        .map_err(|error| copy_error("read_dir", source, target, None, error))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| copy_error("read_dir_entry", source, target, None, error))?;
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

fn copy_error(
    operation: &str,
    source_root: &std::path::Path,
    target_root: &std::path::Path,
    entry: Option<&std::path::Path>,
    error: std::io::Error,
) -> std::io::Error {
    let entry = entry
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<directory>".to_owned());
    std::io::Error::new(
        error.kind(),
        format!(
            "fixture copy {operation}: source={} target={} entry={entry}: {error}",
            source_root.display(),
            target_root.display(),
        ),
    )
}

fn copy_file_error(
    source: &std::path::Path,
    target: &std::path::Path,
    error: std::io::Error,
) -> std::io::Error {
    copy_error("copy_file", source, target, Some(source), error)
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
    std::fs::copy(source, target)
        .map(|_| ())
        .map_err(|error| copy_file_error(source, target, error))
}

#[cfg(not(target_os = "macos"))]
fn clone_seed_file(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    super::profile_metrics::record("fixture_copy_file");
    std::fs::copy(source, target)
        .map(|_| ())
        .map_err(|error| copy_file_error(source, target, error))
}
