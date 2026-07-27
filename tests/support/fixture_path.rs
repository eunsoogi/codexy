use std::ffi::{OsStr, OsString};

const POSIX_PATH_ENVIRONMENTS: &[&str] = &[
    "CODEXY_RUNTIME_CACHE",
    "CODEXY_RUNTIME_CACHE_DIR",
    "CODEXY_RUNTIME_ARTIFACTS",
    "CODEXY_RUNTIME_DIR",
    "CODEXY_RUNTIME_PACKAGE_PATH",
    "FAKE_RELEASE_DIR",
    "FAKE_UPLOAD_LOG",
    "GIT_COMMON_DIR",
    "GIT_DIR",
    "PLUGIN_ROOT",
];

pub(crate) fn fixture_path_text(value: impl AsRef<OsStr>) -> Result<String, String> {
    let value = value.as_ref();
    #[cfg(windows)]
    {
        return windows_fixture_path_os(value).map(|value| value.to_string_lossy().into_owned());
    }
    #[cfg(not(windows))]
    {
        Ok(value.to_string_lossy().into_owned())
    }
}

pub(crate) fn hook_fixture_shell_input(
    command: &str,
    cwd: &std::path::Path,
) -> Result<(String, String), String> {
    #[cfg(windows)]
    let native_cwd = cwd
        .to_str()
        .ok_or_else(|| "hook fixture path is not valid UTF-8".to_owned())?;
    let cwd = fixture_path_text(cwd)?;
    #[cfg(windows)]
    let command = normalize_declared_hook_shell_paths(command, &[(native_cwd, cwd.as_str())]);
    #[cfg(not(windows))]
    let command = command.to_owned();
    Ok((command, cwd))
}

fn normalize_declared_hook_shell_paths(command: &str, paths: &[(&str, &str)]) -> String {
    paths
        .iter()
        .fold(command.to_owned(), |command, (native, posix)| {
            command.replace(native, posix)
        })
}

pub(crate) fn fixture_path_environment_value(
    key: &OsStr,
    value: &OsStr,
) -> Result<OsString, String> {
    #[cfg(windows)]
    {
        return windows_fixture_environment_value(
            key.to_string_lossy().as_ref(),
            value.to_string_lossy().as_ref(),
        )
        .map(Into::into);
    }
    #[cfg(not(windows))]
    let _ = key;
    Ok(value.to_owned())
}

pub(crate) fn windows_fixture_environment_value(key: &str, value: &str) -> Result<String, String> {
    if POSIX_PATH_ENVIRONMENTS.contains(&key) {
        windows_to_posix_fixture_path(value)
    } else {
        Ok(value.to_owned())
    }
}

pub(crate) fn windows_to_posix_fixture_path(value: &str) -> Result<String, String> {
    let value = value.strip_prefix(r"\\?\").unwrap_or(value);
    if value.starts_with(r"\\") {
        return Err(format!(
            "Windows fixture paths do not support UNC values: {value}"
        ));
    }
    if value.starts_with('/') {
        return Ok(value.to_owned());
    }
    let bytes = value.as_bytes();
    let Some((&drive, tail)) = bytes.split_first() else {
        return Err("Windows fixture path must be absolute: ".to_owned());
    };
    if !drive.is_ascii_alphabetic() || tail.first() != Some(&b':') {
        return Err(format!("Windows fixture path must be absolute: {value}"));
    }
    let tail = &value[2..];
    if !tail.starts_with(['\\', '/']) {
        return Err(format!("Windows fixture path must be absolute: {value}"));
    }
    Ok(format!(
        "/{}/{}",
        (drive as char).to_ascii_lowercase(),
        tail[1..].replace('\\', "/")
    ))
}

#[cfg(windows)]
fn windows_fixture_path_os(value: &OsStr) -> Result<OsString, String> {
    let value = value
        .to_str()
        .ok_or_else(|| "Windows fixture path is not valid UTF-8".to_owned())?;
    windows_to_posix_fixture_path(value).map(Into::into)
}

#[test]
fn windows_fixture_paths_use_the_msys_absolute_path_contract() {
    assert_eq!(
        windows_to_posix_fixture_path("C:\\work\\fixture path"),
        Ok("/c/work/fixture path".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path("D:/runtime/cache"),
        Ok("/d/runtime/cache".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path(r"\\?\D:\runtime\cache"),
        Ok("/d/runtime/cache".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path("C:relative"),
        Err("Windows fixture path must be absolute: C:relative".into())
    );
    assert_eq!(
        windows_to_posix_fixture_path("\\\\server\\share"),
        Err("Windows fixture paths do not support UNC values: \\\\server\\share".into())
    );
    assert_eq!(
        windows_fixture_environment_value("CODEXY_RUNTIME_DIR", "C:\\runtime\\with spaces"),
        Ok("/c/runtime/with spaces".into())
    );
    assert_eq!(
        windows_fixture_environment_value("CODEXY_RUNTIME_PLATFORM", "windows-x86_64"),
        Ok("windows-x86_64".into())
    );
}

#[test]
fn declared_hook_shell_paths_normalize_only_declared_values() {
    let native = r"C:\work\fixture path";
    let command = format!("sudo -D {native} git status --short && printf '%s' C:unrelated");
    assert_eq!(
        normalize_declared_hook_shell_paths(&command, &[(native, "/c/work/fixture path")]),
        "sudo -D /c/work/fixture path git status --short && printf '%s' C:unrelated"
    );
}
