use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Hash, PartialEq, Eq)]
struct InterpreterCacheKey {
    interpreter: String,
    path: OsString,
    pathext: OsString,
}

static INTERPRETER_CACHE: OnceLock<Mutex<HashMap<InterpreterCacheKey, PathBuf>>> = OnceLock::new();

pub(crate) fn fixture_script_launcher(
    is_windows: bool,
    contents: &[u8],
) -> Result<Option<&'static str>, String> {
    if !is_windows {
        return Ok(None);
    }
    fixture_script_interpreter(contents)
}

pub(crate) fn windows_fixture_companion(program: &Path) -> Option<PathBuf> {
    (program
        .extension()
        .is_some_and(|extension| extension == "sh"))
    .then(|| program.with_extension("cmd"))
    .filter(|companion| companion.is_file())
}

/// A copied concern fixture has a Windows command companion whose sole runtime invocation
/// is an adjacent Python dispatcher. Test support may run that dispatcher directly when `py`
/// is unavailable, while preserving the command's `--event` argument contract.
pub(crate) fn windows_static_python_fixture(program: &Path) -> Option<PathBuf> {
    let companion = windows_fixture_companion(program)?;
    let python = program.with_extension("py");
    let stem = program.file_stem()?.to_string_lossy();
    let expected = format!("py -3 -I -B \"%~dp0{stem}.py\" --event \"%event%\"");
    std::fs::read_to_string(companion)
        .ok()?
        .contains(&expected)
        .then_some(python)
        .filter(|python| python.is_file())
}

pub(super) fn fixture_script_interpreter(contents: &[u8]) -> Result<Option<&'static str>, String> {
    let first_line = contents
        .splitn(2, |byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    let first_line = first_line.strip_suffix(b"\r").unwrap_or(first_line);
    if !first_line.starts_with(b"#!") {
        return Ok(None);
    }
    let first_line = std::str::from_utf8(first_line)
        .map_err(|_| "malformed fixture script shebang".to_owned())?;
    match first_line {
        "#!/bin/sh" => Ok(Some("sh")),
        "#!/usr/bin/env bash" => Ok(Some("bash")),
        "#!/usr/bin/env python3" => Ok(Some("python")),
        "#!" => Err("malformed fixture script shebang".to_owned()),
        _ => Err(format!("unsupported fixture script shebang: {first_line}")),
    }
}

#[cfg(windows)]
pub(super) fn discover_windows_interpreter(interpreter: &str) -> Result<PathBuf, String> {
    let path = std::env::var_os("PATH").ok_or_else(|| {
        format!("Windows fixture interpreter `{interpreter}` cannot discover PATH")
    })?;
    let extensions = std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
    let cache_key = InterpreterCacheKey {
        interpreter: interpreter.to_owned(),
        path: path.clone(),
        pathext: extensions.clone(),
    };
    if let Ok(mut cache) = INTERPRETER_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        if let Some(candidate) = cache
            .get(&cache_key)
            .filter(|candidate| candidate.is_file())
        {
            return Ok(candidate.clone());
        }
        cache.remove(&cache_key);
    }
    let extensions = extensions.to_string_lossy();
    let candidates = std::iter::once(interpreter.to_owned()).chain(
        extensions
            .split(';')
            .filter(|extension| !extension.is_empty())
            .map(|extension| format!("{interpreter}{extension}")),
    );
    for directory in std::env::split_paths(&path) {
        for candidate in candidates.clone() {
            let candidate = directory.join(candidate);
            if candidate.is_file() {
                if let Ok(mut cache) = INTERPRETER_CACHE
                    .get_or_init(|| Mutex::new(HashMap::new()))
                    .lock()
                {
                    cache.insert(cache_key, candidate.clone());
                }
                return Ok(candidate);
            }
        }
    }
    Err(format!(
        "Windows fixture interpreter `{interpreter}` was not found on the host PATH"
    ))
}
