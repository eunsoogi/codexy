use std::{fs, io, path::Path};

/// Sources a POSIX fixture script after binding bare command names to mock payloads.
///
/// Git Bash can reject an extensionless PATH script despite a valid shebang and
/// chmod state. A shell function keeps the production script's bare invocation
/// while directly interpreting the projected fixture payload through `sh`.
pub(crate) fn write_posix_fixture_shell_runner(
    path: &Path,
    target_environment: &str,
    bindings: &[(&str, &str)],
) -> io::Result<()> {
    let mut source = String::from("#!/bin/sh\nset -eu\n");
    for (command, payload_environment) in bindings {
        validate_identifier(command)?;
        validate_identifier(payload_environment)?;
        source.push_str(&format!(
            "{command}() {{ sh \"${payload_environment}\" \"$@\"; }}\n"
        ));
    }
    validate_identifier(target_environment)?;
    source.push_str(&format!(". \"${target_environment}\" \"$@\"\n"));
    fs::write(path, source)?;
    super::make_executable(path)
}

pub(crate) fn write_single_posix_fixture_shell_runner(
    path: &Path,
    target_environment: &str,
    command: &str,
    payload_environment: &str,
) -> io::Result<()> {
    write_posix_fixture_shell_runner(path, target_environment, &[(command, payload_environment)])
}

fn validate_identifier(value: &str) -> io::Result<()> {
    value
        .chars()
        .enumerate()
        .all(|(_, character)| character == '_' || character.is_ascii_alphanumeric())
        .then_some(())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "fixture shell identifier"))
}
