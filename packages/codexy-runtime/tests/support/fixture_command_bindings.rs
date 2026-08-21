use std::{io, path::Path, process::Command};

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
    write_posix_fixture_shell_runner_with_scrub(path, target_environment, bindings, &[], &[])
}

pub(crate) fn write_posix_fixture_shell_runner_with_scrub(
    path: &Path,
    target_environment: &str,
    bindings: &[(&str, &str)],
    scrubbed_environment: &[&str],
    rebound_environment: &[(&str, &str)],
) -> io::Result<()> {
    let mut source = String::from("#!/bin/sh\nset -eu\n");
    for name in scrubbed_environment {
        validate_identifier(name)?;
        source.push_str(&format!("unset {name}\n"));
    }
    for (name, value_environment) in rebound_environment {
        validate_identifier(name)?;
        validate_identifier(value_environment)?;
        source.push_str(&format!("{name}=\"${value_environment}\"\nexport {name}\n"));
    }
    for (command, payload_environment) in bindings {
        validate_identifier(command)?;
        validate_identifier(payload_environment)?;
        source.push_str(&format!(
            "{command}() {{ sh \"${payload_environment}\" \"$@\"; }}\n"
        ));
    }
    validate_identifier(target_environment)?;
    source.push_str(&format!(". \"${target_environment}\" \"$@\"\n"));
    crate::support::write_executable_fixture(path, source)
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
    let mut characters = value.chars();
    let starts_identifier = matches!(characters.next(), Some('_' | 'a'..='z' | 'A'..='Z'));
    let continues_identifier =
        characters.all(|character| character == '_' || character.is_ascii_alphanumeric());
    let parser_accepts = starts_identifier
        && continues_identifier
        && Command::new("sh")
            .args(["-n", "-c", &format!("{value}() {{ :; }}")])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
    parser_accepts
        .then_some(())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "fixture shell identifier"))
}
