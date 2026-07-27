use std::path::Path;

use super::fixture_path::windows_to_posix_fixture_path;

pub(crate) fn hook_fixture_model_input(
    command: &str,
    cwd: &Path,
) -> Result<(String, String), String> {
    let native_cwd = cwd
        .to_str()
        .ok_or_else(|| "hook fixture path is not valid UTF-8".to_owned())?;
    hook_fixture_model_input_for_platform(command, native_cwd, cfg!(windows), |value| {
        windows_shell_path_to_native(value)
    })
}

fn hook_fixture_model_input_for_platform(
    command: &str,
    native_cwd: &str,
    is_windows: bool,
    convert: impl Fn(&str) -> Result<String, String>,
) -> Result<(String, String), String> {
    if !is_windows {
        return Ok((command.to_owned(), native_cwd.to_owned()));
    }

    // The policy model is native Python, so retain its event cwd and all ordinary
    // command data in native form. Validate the cwd with the shared fixture-path
    // contract, then project only declared Git-Bash path operands for the model.
    windows_to_posix_fixture_path(native_cwd)?;
    let model = project_modeled_paths(command, convert)?;
    Ok((model, native_cwd.to_owned()))
}

fn project_modeled_paths(
    command: &str,
    convert: impl Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    let mut command = command.to_owned();
    for prefix in ["sudo -D ", "sudo --chdir="] {
        let mut start = 0;
        while let Some(found) = command[start..].find(prefix) {
            let begin = start + found + prefix.len();
            let Some(end) = command[begin..].find(" git ").map(|end| begin + end) else {
                break;
            };
            if command[begin..end].starts_with('/') {
                let replacement = convert(&command[begin..end])?;
                command.replace_range(begin..end, &replacement);
                start = begin + replacement.len();
            } else {
                start = end + 5;
            }
        }
    }
    for prefix in ["ln -s ", "ln -sfn "] {
        let mut start = 0;
        while let Some(found) = command[start..].find(prefix) {
            let begin = start + found + prefix.len();
            let tail = &command[begin..];
            let end = begin
                + tail
                    .find(" && ")
                    .or_else(|| tail.find(" || "))
                    .or_else(|| tail.find(';'))
                    .unwrap_or(tail.len());
            let Some(space) = command[begin..end].rfind(char::is_whitespace) else {
                start = end;
                continue;
            };
            let path_end = begin + space;
            if command[begin..path_end].starts_with('/') {
                let replacement = convert(&command[begin..path_end])?;
                command.replace_range(begin..path_end, &replacement);
                start = begin + replacement.len();
            } else {
                start = end;
            }
        }
    }
    Ok(command)
}

#[cfg(windows)]
fn windows_shell_path_to_native(value: &str) -> Result<String, String> {
    use std::process::Command;

    let shell = super::fixture_command_windows::discover_windows_interpreter("sh")?;
    let output = Command::new(shell)
        .args(["-c", "cygpath -w -- \"$1\"", "fixture-path", value])
        .output()
        .map_err(|error| format!("converting Git-Bash fixture path {value}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "converting Git-Bash fixture path {value}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let native = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!native.is_empty())
        .then_some(native)
        .ok_or_else(|| format!("converting Git-Bash fixture path {value}: empty output"))
}

#[cfg(not(windows))]
fn windows_shell_path_to_native(value: &str) -> Result<String, String> {
    Ok(value.to_owned())
}

#[test]
fn modeled_path_projection_touches_only_declared_operands() {
    let command =
        "sudo -D /c/work/foreign git status && ln -s /usr/bin/printf left && printf C:unrelated";
    assert_eq!(
        project_modeled_paths(command, |path| match path {
            "/c/work/foreign" => Ok(r"C:\work\foreign".into()),
            "/usr/bin/printf" => Ok(r"C:\Git\usr\bin\printf".into()),
            other => Err(other.into()),
        }),
        Ok("sudo -D C:\\work\\foreign git status && ln -s C:\\Git\\usr\\bin\\printf left && printf C:unrelated".into()),
    );
}

#[test]
fn windows_hook_model_input_preserves_native_cwd_and_only_projects_shell_operands() {
    let native_cwd = r"C:\work\owned";
    let command =
        "sudo -D /c/work/foreign git status && ln -s /usr/bin/printf left && printf C:unrelated";
    assert_eq!(
        hook_fixture_model_input_for_platform(command, native_cwd, true, |path| match path {
            "/c/work/foreign" => Ok(r"C:\work\foreign".into()),
            "/usr/bin/printf" => Ok(r"C:\Git\usr\bin\printf".into()),
            other => Err(other.into()),
        }),
        Ok((
            "sudo -D C:\\work\\foreign git status && ln -s C:\\Git\\usr\\bin\\printf left && printf C:unrelated".into(),
            native_cwd.into(),
        )),
    );
    assert!(
        hook_fixture_model_input_for_platform("git status", r"\\server\share", true, |value| {
            Ok(value.to_owned())
        })
        .is_err()
    );
}
