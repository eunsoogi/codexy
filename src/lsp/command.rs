use std::ffi::OsStr;
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
    let executable_names = executable_names(executable);
    for entry in std::env::var_os("PATH")
        .as_deref()
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
    {
        for name in &executable_names {
            let candidate = entry.join(name);
            if is_executable(&candidate) {
                return (true, Some(candidate.display().to_string()), None);
            }
        }
    }
    (
        false,
        None,
        Some(format!("executable not found on PATH: {executable}")),
    )
}

fn executable_names(executable: &str) -> Vec<String> {
    executable_names_for_platform(
        executable,
        cfg!(windows),
        std::env::var_os("PATHEXT").as_deref(),
    )
}

fn executable_names_for_platform(
    executable: &str,
    is_windows: bool,
    _pathext: Option<&OsStr>,
) -> Vec<String> {
    let mut names = vec![executable.to_owned()];
    if Path::new(executable).extension().is_some() {
        return names;
    }
    if is_windows {
        names.push(format!("{executable}.exe"));
    }
    names
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

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::executable_names_for_platform;

    #[test]
    fn windows_names_ignore_unlaunchable_pathext_shims() {
        let names = executable_names_for_platform(
            "rust-analyzer",
            true,
            Some(OsStr::new(".CMD;.BAT;.EXE")),
        );

        assert_eq!(names, vec!["rust-analyzer", "rust-analyzer.exe"]);
    }
}
