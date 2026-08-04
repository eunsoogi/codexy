use std::{collections::BTreeSet, ffi::OsStr, path::PathBuf};

pub(crate) fn executable_path(command: &str) -> Result<PathBuf, String> {
    let path = std::env::var_os("PATH").ok_or_else(|| "PATH is unavailable".to_owned())?;
    let extensions = std::env::var_os("PATHEXT").unwrap_or_default();
    executable_path_in(command, &path, &extensions)
}

pub(crate) fn executable_path_in(
    command: &str,
    path: &OsStr,
    extensions: &OsStr,
) -> Result<PathBuf, String> {
    let suffixes = executable_suffixes(command, extensions)?;
    for directory in std::env::split_paths(path) {
        for suffix in &suffixes {
            let candidate = directory.join(format!("{command}{suffix}"));
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err(format!("required command missing: {command}"))
}

fn executable_suffixes(command: &str, extensions: &OsStr) -> Result<Vec<String>, String> {
    if std::path::Path::new(command).extension().is_some() {
        return Ok(vec![String::new()]);
    }
    let mut seen = BTreeSet::new();
    let mut suffixes = vec![String::new()];
    for extension in extensions
        .to_string_lossy()
        .split(';')
        .filter(|value| !value.is_empty())
    {
        let normalized = extension.to_ascii_lowercase();
        if !normalized.starts_with('.') || !seen.insert(normalized) {
            return Err(format!("ambiguous PATHEXT entry: {extension}"));
        }
        suffixes.push(extension.to_owned());
    }
    Ok(suffixes)
}
