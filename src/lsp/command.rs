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
    let mut names = vec![executable.to_owned()];
    if Path::new(executable).extension().is_some() {
        return names;
    }
    if !cfg!(windows) {
        return names;
    }
    let Some(pathext) = std::env::var_os("PATHEXT") else {
        names.push(format!("{executable}.exe"));
        return names;
    };
    for extension in pathext.to_string_lossy().split(';') {
        let extension = extension.trim();
        if extension.is_empty() {
            continue;
        }
        let suffix = if extension.starts_with('.') {
            extension.to_owned()
        } else {
            format!(".{extension}")
        };
        push_name(&mut names, executable, &suffix);
        let lowercase_suffix = suffix.to_ascii_lowercase();
        if lowercase_suffix != suffix {
            push_name(&mut names, executable, &lowercase_suffix);
        }
    }
    names
}

fn push_name(names: &mut Vec<String>, executable: &str, suffix: &str) {
    let candidate = format!("{executable}{suffix}");
    if !names.iter().any(|name| name == &candidate) {
        names.push(candidate);
    }
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
