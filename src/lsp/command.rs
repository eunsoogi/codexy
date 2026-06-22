use std::path::Path;

use anyhow::Result;

use crate::lsp::pathing::resolve_root;

pub(crate) fn resolve_command(command: &[String], root: Option<&str>) -> Result<Vec<String>> {
    let Some(first) = command.first() else {
        return Ok(Vec::new());
    };
    if first.contains(std::path::MAIN_SEPARATOR) && !Path::new(first).is_absolute() {
        if let Some(root) = root {
            let mut output = vec![resolve_root(root)?.join(first).display().to_string()];
            output.extend(command.iter().skip(1).cloned());
            return Ok(output);
        }
    }
    Ok(command.to_vec())
}

pub(crate) fn resolve_executable(command: &[String]) -> (bool, Option<String>, Option<String>) {
    let Some(executable) = command.first() else {
        return (false, None, Some("server command is missing".to_owned()));
    };
    if executable.contains(std::path::MAIN_SEPARATOR) {
        let path = Path::new(executable);
        if is_executable(path) {
            return (true, Some(executable.clone()), None);
        }
        let reason = if path.exists() {
            format!("executable is not executable: {executable}")
        } else {
            format!("executable not found: {executable}")
        };
        return (false, None, Some(reason));
    }
    for entry in std::env::var_os("PATH")
        .as_deref()
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
    {
        let candidate = entry.join(executable);
        if is_executable(&candidate) {
            return (true, Some(candidate.display().to_string()), None);
        }
    }
    (
        false,
        None,
        Some(format!("executable not found on PATH: {executable}")),
    )
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    path.metadata()
        .is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}
