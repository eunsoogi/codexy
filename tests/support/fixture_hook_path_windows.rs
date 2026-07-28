#[cfg(windows)]
pub(super) fn native_shell_fixture_path(value: &str) -> Result<String, String> {
    use std::collections::BTreeMap;
    use std::process::Command;
    use std::sync::{Mutex, OnceLock};

    static PATH_CACHE: OnceLock<Mutex<BTreeMap<String, String>>> = OnceLock::new();
    let cache = PATH_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if let Some(native) = cache
        .lock()
        .map_err(|_| "Git-Bash fixture path cache lock poisoned".to_owned())?
        .get(value)
        .cloned()
    {
        return Ok(native);
    }
    let shell = super::fixture_command_windows::discover_windows_interpreter("sh")?;
    let native = native_shell_fixture_path_with(
        value,
        |interpreter| {
            super::fixture_command_windows::discover_windows_interpreter(interpreter).and_then(
                |path| {
                    path.to_str().map(str::to_owned).ok_or_else(|| {
                        format!("Windows fixture interpreter `{interpreter}` is not valid UTF-8")
                    })
                },
            )
        },
        || {
            std::env::temp_dir()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| "Windows fixture temporary root is not valid UTF-8".to_owned())
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
        .insert(value.to_owned(), native.clone());
    Ok(native)
}

fn native_shell_fixture_path_with(
    value: &str,
    discover: impl Fn(&str) -> Result<String, String>,
    temporary_root: impl Fn() -> Result<String, String>,
    convert: impl Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    match value {
        "/usr/bin/git" => discover("git"),
        "/usr/bin/printf" => discover("sh"),
        "/var/tmp" => temporary_root(),
        _ => convert(value),
    }
}

#[cfg(test)]
mod tests {
    use super::native_shell_fixture_path_with;

    #[test]
    fn native_model_uses_host_identities_for_declared_posix_fixture_paths() {
        let discover = |name: &str| -> Result<String, String> {
            match name {
                "git" => Ok(r"C:\\host\\git.exe".to_owned()),
                "sh" => Ok(r"C:\\host\\sh.exe".to_owned()),
                other => Err(format!("missing {other}")),
            }
        };
        let temporary_root = || -> Result<String, String> { Ok(r"C:\\host\\temp".to_owned()) };
        let convert =
            |path: &str| -> Result<String, String> { Ok(format!(r"C:\\converted\\{path}")) };

        assert_eq!(
            native_shell_fixture_path_with("/usr/bin/git", discover, temporary_root, convert),
            Ok(r"C:\\host\\git.exe".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with("/usr/bin/printf", discover, temporary_root, convert),
            Ok(r"C:\\host\\sh.exe".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with("/var/tmp", discover, temporary_root, convert),
            Ok(r"C:\\host\\temp".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with("/opt/custom", discover, temporary_root, convert),
            Ok(r"C:\\converted\\/opt/custom".to_owned())
        );
        assert_eq!(
            native_shell_fixture_path_with(r"\\server\\share", discover, temporary_root, |_| {
                Err("Windows fixture paths do not support UNC values".to_owned())
            }),
            Err("Windows fixture paths do not support UNC values".to_owned())
        );
    }
}
