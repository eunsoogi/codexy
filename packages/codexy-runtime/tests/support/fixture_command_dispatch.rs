use std::{
    ffi::{OsStr, OsString},
    process::Command,
};

#[cfg(windows)]
use super::super::fixture_command_windows::discover_windows_interpreter;
use super::super::{
    fixture_command_windows::fixture_script_interpreter, fixture_path::fixture_path_text,
};

impl super::FixtureCommand {
    pub(crate) fn assert_fixture_script_launcher_mappings() {
        for (contents, interpreter) in [
            (b"#!/bin/sh\necho fixture\n".as_slice(), Some("sh")),
            (b"#!/bin/sh\r\necho fixture\r\n".as_slice(), Some("sh")),
            (
                b"#!/usr/bin/env bash\necho fixture\n".as_slice(),
                Some("bash"),
            ),
            (
                b"#!/usr/bin/env python3\nprint('fixture')\n".as_slice(),
                Some("python"),
            ),
        ] {
            assert_eq!(
                super::fixture_script_launcher(true, contents),
                Ok(interpreter)
            );
        }
        assert_eq!(super::fixture_script_launcher(true, b"MZ\x90\0"), Ok(None));
        assert_eq!(
            super::fixture_script_launcher(false, b"#!/bin/sh\necho fixture\n"),
            Ok(None)
        );
    }

    pub(crate) fn assert_fixture_script_launcher_rejections() {
        for (contents, expected) in [
            (
                b"#!/usr/bin/env ruby\nputs 'fixture'\n".as_slice(),
                "unsupported fixture script shebang: #!/usr/bin/env ruby",
            ),
            (
                b"#!/bin/sh-invalid\necho fixture\n".as_slice(),
                "unsupported fixture script shebang: #!/bin/sh-invalid",
            ),
            (
                b"#!\nfixture\n".as_slice(),
                "malformed fixture script shebang",
            ),
        ] {
            assert_eq!(
                super::fixture_script_launcher(true, contents),
                Err(expected.to_owned())
            );
        }
    }
}

pub(super) fn materialized_script_command(
    program: &OsStr,
    source: &std::path::Path,
) -> Result<(Command, bool), String> {
    let contents = std::fs::read(std::path::Path::new(program))
        .map_err(|error| format!("reading materialized fixture script: {error}"))?;
    let interpreter = match fixture_script_interpreter(&contents) {
        Ok(Some(interpreter)) => interpreter,
        Ok(None) => return Ok((Command::new(program), false)),
        #[cfg(not(windows))]
        Err(_) => return Ok((Command::new(program), false)),
        #[cfg(windows)]
        Err(error) => return Err(error),
    };
    let uses_posix_path = matches!(interpreter, "sh" | "bash");
    #[cfg(windows)]
    let interpreter = discover_windows_interpreter(interpreter)?;
    #[cfg(not(windows))]
    let interpreter = if interpreter == "python" {
        "python3"
    } else {
        interpreter
    };
    let source: OsString = if uses_posix_path {
        fixture_path_text(source)?.into()
    } else {
        source.as_os_str().to_owned()
    };
    let materialized: OsString = if uses_posix_path {
        fixture_path_text(program)?.into()
    } else {
        program.to_owned()
    };
    let mut command = Command::new(interpreter);
    command
        .arg("-c")
        .arg("materialized=$1\nshift\n. \"$materialized\"")
        .arg(source)
        .arg(materialized);
    Ok((command, uses_posix_path))
}

#[cfg(unix)]
pub(super) fn generated_fixture_script_command(program: &OsStr) -> Option<Command> {
    use std::os::unix::fs::PermissionsExt as _;

    let path = std::path::Path::new(program);
    if !path.is_absolute() {
        return None;
    }
    let candidate = path.canonicalize().ok()?;
    let temp_root = std::env::temp_dir().canonicalize().ok()?;
    if !candidate.starts_with(temp_root) || !candidate.is_file() {
        return None;
    }
    if std::fs::metadata(&candidate).ok()?.permissions().mode() & 0o111 == 0 {
        return None;
    }
    let contents = std::fs::read(candidate).ok()?;
    let interpreter = fixture_script_interpreter(&contents).ok()??;
    let interpreter = match interpreter {
        "sh" => "/bin/sh",
        "python" => "python3",
        _ => interpreter,
    };
    let mut command = Command::new(interpreter);
    command.arg(program);
    Some(command)
}
