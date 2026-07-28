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
        windows_shell_path_to_native(value, native_cwd)
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
            if let Some(replacement) = modeled_path_token(&command[begin..end], &convert)? {
                command.replace_range(begin..end, &replacement);
                start = begin + replacement.len();
            } else {
                start = end + 5;
            }
        }
    }
    for prefix in ["ln -s ", "ln -sfn ", "cp "] {
        let mut start = 0;
        while let Some(found) = command[start..].find(prefix) {
            let found = start + found;
            if !shell_command_boundary(&command, found) {
                start = found + prefix.len();
                continue;
            }
            let begin = found + prefix.len();
            let tail = &command[begin..];
            let end = begin
                + tail
                    .find(" && ")
                    .or_else(|| tail.find(" || "))
                    .or_else(|| tail.find(';'))
                    .unwrap_or(tail.len());
            let Some((path_begin, path_end)) =
                modeled_path_operand_bounds(&command[begin..end], prefix == "cp ")
            else {
                start = end;
                continue;
            };
            let path_begin = begin + path_begin;
            let path_end = begin + path_end;
            if let Some(replacement) = modeled_path_token(&command[path_begin..path_end], &convert)?
            {
                command.replace_range(path_begin..path_end, &replacement);
                start = path_begin + replacement.len();
            } else {
                start = end;
            }
        }
    }
    Ok(command)
}

fn shell_command_boundary(command: &str, offset: usize) -> bool {
    command[..offset]
        .chars()
        .next_back()
        .is_none_or(|character| {
            character.is_whitespace() || matches!(character, ';' | '&' | '|' | '(')
        })
}

fn modeled_path_operand_bounds(segment: &str, copy_source: bool) -> Option<(usize, usize)> {
    if !copy_source {
        let path_end = segment.rfind(char::is_whitespace)?;
        return Some((0, path_end));
    }

    let mut offset = 0;
    for token in segment.split_whitespace() {
        let start = offset + segment[offset..].find(token)?;
        offset = start + token.len();
        if token == "--" || token.starts_with('-') {
            continue;
        }
        return Some((start, offset));
    }
    None
}

fn modeled_path_token(
    value: &str,
    convert: &impl Fn(&str) -> Result<String, String>,
) -> Result<Option<String>, String> {
    let native = if value.starts_with('/') {
        convert(value)?
    } else if windows_to_posix_fixture_path(value).is_ok() {
        value.to_owned()
    } else if value.starts_with(r"\\") {
        return windows_to_posix_fixture_path(value).map(|_| None);
    } else {
        return Ok(None);
    };
    Ok(Some(format!("'{}'", native.replace('\'', "'\"'\"'"))))
}

#[cfg(windows)]
fn windows_shell_path_to_native(value: &str, native_cwd: &str) -> Result<String, String> {
    super::fixture_hook_path_windows::native_shell_fixture_path(value, native_cwd)
}

#[cfg(not(windows))]
fn windows_shell_path_to_native(value: &str, _native_cwd: &str) -> Result<String, String> {
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
        Ok("sudo -D 'C:\\work\\foreign' git status && ln -s 'C:\\Git\\usr\\bin\\printf' left && printf C:unrelated".into()),
    );
}

#[test]
fn modeled_path_projection_converts_copy_sources_without_rewriting_destinations() {
    let command = "cp -fP /usr/bin/printf left && printf /usr/bin/printf";
    assert_eq!(
        project_modeled_paths(command, |path| match path {
            "/usr/bin/printf" => Ok(r"C:\Git\usr\bin\printf.exe".into()),
            other => Err(other.into()),
        }),
        Ok("cp -fP 'C:\\Git\\usr\\bin\\printf.exe' left && printf /usr/bin/printf".into()),
    );
}

#[test]
fn modeled_path_projection_does_not_treat_scoped_commands_as_copy_operations() {
    let command = "scp /usr/bin/printf remote:";
    assert_eq!(
        project_modeled_paths(command, |_| Err("copy source must stay unprojected".into())),
        Ok(command.into()),
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
            "sudo -D 'C:\\work\\foreign' git status && ln -s 'C:\\Git\\usr\\bin\\printf' left && printf C:unrelated".into(),
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

#[test]
fn modeled_path_tokens_quote_raw_windows_values_without_touching_non_paths() {
    assert_eq!(
        modeled_path_token(r"C:\work\fixture path", &|_| unreachable!()),
        Ok(Some(r"'C:\work\fixture path'".into())),
    );
    assert_eq!(
        modeled_path_token(r"C:\work\O'Brien", &|_| unreachable!()),
        Ok(Some("'C:\\work\\O'\"'\"'Brien'".into())),
    );
    assert_eq!(
        modeled_path_token("C:relative", &|_| unreachable!()),
        Ok(None)
    );
    assert!(modeled_path_token(r"\\server\share", &|_| unreachable!()).is_err());
}
