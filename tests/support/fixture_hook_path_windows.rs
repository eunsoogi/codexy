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

fn native_shell_fixture_path_with(
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

fn fixture_path_cache_key(value: &str, fixture_root: &str) -> (String, Option<String>) {
    (
        value.to_owned(),
        (value == "/var/tmp").then(|| fixture_root.to_owned()),
    )
}

#[cfg(test)]
mod tests {
    use super::{fixture_path_cache_key, native_shell_fixture_path_with};

    #[test]
    fn native_model_uses_host_identities_for_declared_posix_fixture_paths() {
        let discover = |name: &str| -> Result<String, String> {
            match name {
                "git" => Ok(r"C:\\host\\git.exe".to_owned()),
                "sh" => Ok(r"C:\\host\\sh.exe".to_owned()),
                other => Err(format!("missing {other}")),
            }
        };
        let convert =
            |path: &str| -> Result<String, String> { Ok(format!(r"C:\\converted\\{path}")) };

        assert_eq!(
            native_shell_fixture_path_with("/usr/bin/git", r"C:\\host\\fixture", discover, convert),
            Ok(r"C:\\host\\git.exe".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with(
                "/usr/bin/printf",
                r"C:\\host\\fixture",
                discover,
                convert
            ),
            Ok(r"C:\\host\\sh.exe".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with("/var/tmp", r"C:\\host\\fixture", discover, convert),
            Ok(r"C:\\host\\fixture".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with("/opt/custom", r"C:\\host\\fixture", discover, convert),
            Ok(r"C:\\converted\\/opt/custom".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with(
                r"\\server\\share",
                r"C:\\host\\fixture",
                discover,
                |_| { Err("Windows fixture paths do not support UNC values".to_owned()) }
            ),
            Err("Windows fixture paths do not support UNC values".to_owned())
        );
    }

    #[test]
    fn fixture_path_cache_key_keeps_fixture_root_context() {
        assert_ne!(
            fixture_path_cache_key("/var/tmp", r"C:\\host\\fixture-a"),
            fixture_path_cache_key("/var/tmp", r"C:\\host\\fixture-b")
        );
        assert_eq!(
            fixture_path_cache_key("/usr/bin/git", r"C:\\host\\fixture-a"),
            fixture_path_cache_key("/usr/bin/git", r"C:\\host\\fixture-b")
        );
    }
}
