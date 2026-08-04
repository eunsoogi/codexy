#[cfg(windows)]
pub(super) fn native_shell_fixture_path(value: &str, native_cwd: &str) -> Result<String, String> {
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::process::Command;
    use std::sync::{Mutex, OnceLock};

    let fixture_root = Path::new(native_cwd)
        .parent()
        .and_then(|path| path.to_str())
        .ok_or_else(|| "Windows fixture cwd has no valid parent directory".to_owned())?;
    let cache_key = fixture_path_cache_key(value, fixture_root);
    static PATH_CACHE: OnceLock<Mutex<BTreeMap<(String, Option<String>), String>>> =
        OnceLock::new();
    let cache = PATH_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Some(native) = cache
        .lock()
        .map_err(|_| "Git-Bash fixture path cache lock poisoned".to_owned())?
        .get(&cache_key)
        .cloned()
    {
        return Ok(native);
    }
    let shell = super::fixture_command_windows::discover_windows_interpreter("sh")?;
    let native = native_shell_fixture_path_with(
        value,
        fixture_root,
        |interpreter| {
            super::fixture_command_windows::discover_windows_interpreter(interpreter).and_then(
                |path| {
                    path.to_str().map(str::to_owned).ok_or_else(|| {
                        format!("Windows fixture interpreter `{interpreter}` is not valid UTF-8")
                    })
                },
            )
        },
        |path| {
            let output = Command::new(&shell)
                .args(["-c", "cygpath -w -- \"$1\"", "fixture-path", path])
                .output()
                .map_err(|error| format!("converting Git-Bash fixture path {path}: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "converting Git-Bash fixture path {path}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
            let native = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            (!native.is_empty())
                .then_some(native)
                .ok_or_else(|| format!("converting Git-Bash fixture path {path}: empty output"))
        },
    )?;
    cache
        .lock()
        .map_err(|_| "Git-Bash fixture path cache lock poisoned".to_owned())?
        .insert(cache_key, native.clone());
    Ok(native)
}

pub(crate) fn native_shell_fixture_path_with(
    value: &str,
    fixture_root: &str,
    discover: impl Fn(&str) -> Result<String, String>,
    convert: impl Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    match value {
        "/usr/bin/git" => discover("git"),
        "/usr/bin/printf" => discover("sh"),
        "/var/tmp" => Ok(fixture_root.to_owned()),
        _ => convert(value),
    }
}

pub(crate) fn fixture_path_cache_key(value: &str, fixture_root: &str) -> (String, Option<String>) {
    (
        value.to_owned(),
        (value == "/var/tmp").then(|| fixture_root.to_owned()),
    )
}
