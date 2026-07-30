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
        match windows_to_posix_fixture_path(value) {
            Ok(path) => Ok(path),
            Err(_) if matches!(key, "GIT_COMMON_DIR" | "GIT_DIR") => Ok(value.to_owned()),
            Err(error) => Err(error),
        }
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
    assert_eq!(
        windows_fixture_environment_value("GIT_DIR", "host-git-dir"),
        Ok("host-git-dir".into())
    );
    assert_eq!(
        windows_fixture_environment_value("GIT_DIR", "C:\\work\\git-dir"),
        Ok("/c/work/git-dir".into())
    );
    assert_eq!(
        windows_fixture_environment_value("GIT_COMMON_DIR", "host-common"),
        Ok("host-common".into())
    );
    assert_eq!(
        windows_fixture_environment_value("GIT_COMMON_DIR", "D:/work/common"),
        Ok("/d/work/common".into())
    );
}
