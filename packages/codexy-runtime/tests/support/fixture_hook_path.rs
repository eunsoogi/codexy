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

pub(crate) fn hook_fixture_model_input_for_platform(
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

pub(crate) fn project_modeled_paths(
    command: &str,
    convert: impl Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    let mut command = command.to_owned();
    for prefix in ["cd ", "pushd "] {
        let mut start = 0;
        while let Some(found) = command[start..].find(prefix) {
            let found = start + found;
            if !shell_command_boundary(&command, found) {
                start = found + prefix.len();
                continue;
            }
            let begin = found + prefix.len();
            let end = begin + shell_command_terminator(&command[begin..]);
            if let Some(replacement) = modeled_path_token(&command[begin..end], &convert)? {
                command.replace_range(begin..end, &replacement);
                start = begin + replacement.len();
            } else {
                start = end;
            }
        }
    }
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
            let end = begin + shell_command_terminator(&command[begin..]);
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
    let mut command_start = true;
    let mut quote = None;
    let bytes = command.as_bytes();
    let mut index = 0;
    while index < offset {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'\'' | b'\"' => {
                quote = Some(byte);
                command_start = false;
            }
            b'\\' => {
                command_start = false;
                index += 1;
            }
            b';' | b'\n' | b'(' => command_start = true,
            b'&' | b'|' if bytes.get(index + 1) == Some(&byte) => {
                command_start = true;
                index += 1;
            }
            value if value.is_ascii_whitespace() => {}
            _ => command_start = false,
        }
        index += 1;
    }
    quote.is_none() && command_start
}

fn shell_command_terminator(segment: &str) -> usize {
    let mut quote = None;
    let bytes = segment.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(active_quote) = quote {
            if byte == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'\'' | b'\"' => quote = Some(byte),
            b'\\' => index += 1,
            b';' | b'\n' | b'&' | b'|' => return segment[..index].trim_end().len(),
            _ => {}
        }
        index += 1;
    }
    segment.trim_end().len()
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

pub(crate) fn modeled_path_token(
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
