use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Copy)]
pub(crate) struct FixtureScriptBinding {
    pub(crate) invocation: &'static str,
    pub(crate) child: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) enum FixtureArgumentDomain {
    Posix,
    /// A native GitHub CLI mock with a concrete adapter launcher. The adapter
    /// carries API identifiers out-of-band and translates only declared file
    /// operands before invoking the native payload.
    GitHubApi {
        adapter_launcher_environment: &'static str,
    },
}

const GITHUB_ARGV_ADAPTER: &str = r##"#!/usr/bin/env python3
import os
import pathlib
import subprocess
import sys

def fail(message):
    print(f'fixture gh argv transport: {message}', file=sys.stderr)
    sys.exit(2)

def read_transport(path):
    lines = pathlib.Path(path).read_text(encoding='utf-8').splitlines()
    if not lines or not lines[0]:
        fail('missing logical repository')
    return lines[0], lines[1:]

def native_path(value):
    if os.name != 'nt':
        return value
    try:
        return subprocess.check_output(
            ['cygpath', '-w', '--', value], text=True, stderr=subprocess.PIPE
        ).rstrip('\r\n')
    except (OSError, subprocess.CalledProcessError) as error:
        fail(f'filesystem conversion: {error}')

def native_arguments(args):
    file_indices = set()
    if args[:2] == ['release', 'download']:
        file_indices.update(index + 1 for index, value in enumerate(args) if value == '--dir')
    elif args[:2] == ['release', 'upload'] and len(args) > 3:
        file_indices.add(3)
    elif args[:2] == ['attestation', 'verify'] and len(args) > 2:
        file_indices.add(2)
    return [native_path(value) if index in file_indices else value for index, value in enumerate(args)]

def payload_is_posix(path):
    return pathlib.Path(path).read_bytes().startswith(b'#!/bin/sh')

payload, launcher, transport = sys.argv[1:]
repository, arguments = read_transport(transport)
os.environ['GITHUB_REPOSITORY'] = repository
os.environ['CODEXY_FIXTURE_GH_TRANSPORT'] = '1'
if not payload_is_posix(payload):
    arguments = native_arguments(arguments)
os.execv(launcher, [launcher, payload, *arguments])
"##;

pub(crate) fn fixture_github_argv_adapter_path(path: &Path) -> PathBuf {
    path.parent()
        .expect("fixture script parent")
        .join(".codexy-fixture-github-argv.py")
}

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

/// Sources a POSIX fixture script with an explicit interpreter path for every
/// bare command binding. This keeps shell and Python mocks deterministic under
/// Git Bash, where PATH lookup can lose to a native `.exe` or omit `python3`.
pub(crate) fn bind_posix_fixture_shell_launchers(
    path: &Path,
    bindings: &[(&str, &str, &str, FixtureArgumentDomain)],
) -> io::Result<()> {
    let source = fs::read_to_string(path)?;
    let Some((shebang, body)) = source.split_once('\n') else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fixture script has no body",
        ));
    };
    if shebang != "#!/bin/sh" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fixture script is not POSIX shell",
        ));
    }
    let mut bound = format!("{shebang}\n");
    bound.push_str(
        "fixture_finish() { fixture_status=$?; trap - 0; if test \"$fixture_status\" -ne 0; then printf 'fixture shell failure: %s exited %s\\n' \"$0\" \"$fixture_status\" >&2; fi; exit \"$fixture_status\"; }\ntrap fixture_finish 0\n",
    );
    for (command, payload_environment, launcher_environment, domain) in bindings {
        validate_identifier(command)?;
        validate_identifier(payload_environment)?;
        validate_identifier(launcher_environment)?;
        match domain {
            FixtureArgumentDomain::Posix => bound.push_str(&format!(
                "{command}() {{ \"${launcher_environment}\" \"${payload_environment}\" \"$@\"; }}\n"
            )),
            FixtureArgumentDomain::GitHubApi {
                adapter_launcher_environment,
            } => {
                validate_identifier(adapter_launcher_environment)?;
                let adapter = fixture_github_argv_adapter_path(path);
                fs::write(&adapter, GITHUB_ARGV_ADAPTER)?;
                crate::support::make_executable(&adapter)?;
                let adapter =
                    crate::support::fixture_path_text(&adapter).map_err(io::Error::other)?;
                bound.push_str(&format!(
                    "{command}() {{ fixture_argv_transport=\"{adapter}.args\"; {{ printf '%s\\n' \"${{GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}}\"; printf '%s\\n' \"$@\"; }} > \"$fixture_argv_transport\"; \"${adapter_launcher_environment}\" \"{adapter}\" \"${payload_environment}\" \"${launcher_environment}\" \"$fixture_argv_transport\"; return \"$?\"; }}\n"
                ));
            }
        }
    }
    bound.push_str(body);
    fs::write(path, bound)?;
    crate::support::make_executable(path)
}

/// Launches the known copied child scripts through the parent's concrete shell.
/// These declarations are test-fixture contracts, not a parser for production
/// shell: every child invocation is named explicitly by its owning fixture.
pub(crate) fn bind_posix_fixture_script_launchers(
    path: &Path,
    launcher_environment: &str,
    fixture_root_environment: &str,
    bindings: &[FixtureScriptBinding],
) -> io::Result<()> {
    validate_identifier(launcher_environment)?;
    validate_identifier(fixture_root_environment)?;
    let mut bound = fs::read_to_string(path)?;
    for binding in bindings {
        validate_fixture_script_path(binding.child)?;
        if !binding.invocation.starts_with(binding.child)
            || bound.matches(binding.invocation).count() != 1
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "fixture child invocation must occur once: {}",
                    binding.invocation
                ),
            ));
        }
        let arguments = binding
            .invocation
            .strip_prefix(binding.child)
            .unwrap_or_default();
        let replacement = format!(
            "\"${launcher_environment}\" \"${{{fixture_root_environment}}}/{}\"{arguments}",
            binding.child
        );
        bound = bound.replacen(binding.invocation, &replacement, 1);
    }
    fs::write(path, bound)
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
    fs::write(path, source)?;
    crate::support::make_executable(path)
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

fn validate_fixture_script_path(value: &str) -> io::Result<()> {
    let valid = value.starts_with("scripts/")
        && value.chars().all(|character| {
            character == '/'
                || character == '-'
                || character == '_'
                || character.is_ascii_alphanumeric()
        });
    valid
        .then_some(())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "fixture script path"))
}
