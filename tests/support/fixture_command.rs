use std::ffi::{OsStr, OsString};
use std::process::Command;

use super::fixture_command_windows::fixture_script_interpreter;
#[cfg(windows)]
use super::fixture_command_windows::{discover_windows_interpreter, windows_static_python_command};
pub(super) use super::fixture_command_windows::{
    fixture_script_launcher, windows_fixture_companion, windows_static_python_fixture,
};
use super::{
    fixture_path::{fixture_path_environment_value, fixture_path_text},
    fixture_text::materialized_script_source,
};

/// A test-only command factory that preserves direct native execution and launches POSIX
/// fixture scripts through `sh` on Windows.
#[derive(Debug)]
pub(crate) struct FixtureCommand(Command);

impl FixtureCommand {
    pub(crate) fn new(program: impl AsRef<std::ffi::OsStr>) -> Self {
        let program = program.as_ref();
        #[cfg(windows)]
        if let Some(command) = windows_static_python_command(std::path::Path::new(program))
            .unwrap_or_else(|error| panic!("{error}"))
        {
            return Self(command);
        }
        #[cfg(windows)]
        if let Some(companion) = windows_fixture_companion(std::path::Path::new(program)) {
            return Self(Command::new(companion));
        }
        if let Some(source) = materialized_script_source(std::path::Path::new(program)) {
            let command = materialized_script_command(program, &source)
                .unwrap_or_else(|error| panic!("{error}"));
            return Self(command);
        }
        #[cfg(windows)]
        {
            if let Ok(contents) = std::fs::read(std::path::Path::new(program)) {
                match fixture_script_launcher(true, &contents) {
                    Ok(Some(interpreter)) => {
                        let uses_posix_path = matches!(interpreter, "sh" | "bash");
                        let interpreter = discover_windows_interpreter(interpreter)
                            .unwrap_or_else(|error| panic!("{error}"));
                        let mut command = Command::new(interpreter);
                        let program: OsString = if uses_posix_path {
                            fixture_path_text(program)
                                .unwrap_or_else(|error| panic!("{error}"))
                                .into()
                        } else {
                            program.to_owned()
                        };
                        command.arg(program);
                        return Self(command);
                    }
                    Ok(None) => {}
                    Err(error) => panic!("{error}"),
                }
            }
        }
        Self(Command::new(program))
    }

    pub(crate) fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let key = key.as_ref();
        let value = fixture_path_environment_value(key, value.as_ref())
            .unwrap_or_else(|error| panic!("{error}"));
        self.0.env(key, value);
        self
    }

    pub(crate) fn envs<K, V, I>(&mut self, variables: I) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
        I: IntoIterator<Item = (K, V)>,
    {
        for (key, value) in variables {
            self.env(key, value);
        }
        self
    }

    pub(crate) fn arg_path(&mut self, path: impl AsRef<OsStr>) -> &mut Self {
        let path = fixture_path_text(path).unwrap_or_else(|error| panic!("{error}"));
        self.0.arg(path);
        self
    }

    pub(crate) fn env_path<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let value = fixture_path_text(value).unwrap_or_else(|error| panic!("{error}"));
        self.0.env(key, value);
        self
    }

    pub(crate) fn env_path_list<K, I, V>(&mut self, key: K, values: I) -> &mut Self
    where
        K: AsRef<OsStr>,
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        let value = values
            .into_iter()
            .map(|value| fixture_path_text(value).unwrap_or_else(|error| panic!("{error}")))
            .collect::<Vec<_>>()
            .join(":");
        self.0.env(key, value);
        self
    }

    pub(crate) fn path_arg(&mut self, path: impl AsRef<OsStr>) -> &mut Self {
        self.arg_path(path)
    }
}

fn materialized_script_command(
    program: &OsStr,
    source: &std::path::Path,
) -> Result<Command, String> {
    let contents = std::fs::read(std::path::Path::new(program))
        .map_err(|error| format!("reading materialized fixture script: {error}"))?;
    let interpreter = match fixture_script_interpreter(&contents) {
        Ok(Some(interpreter)) => interpreter,
        Ok(None) => return Ok(Command::new(program)),
        #[cfg(not(windows))]
        Err(_) => return Ok(Command::new(program)),
        #[cfg(windows)]
        Err(error) => return Err(error),
    };
    let script = String::from_utf8(contents)
        .map_err(|_| "materialized fixture script must be valid UTF-8".to_owned())?;
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
    let mut command = Command::new(interpreter);
    command.arg("-c").arg(script).arg(source);
    Ok(command)
}

impl std::ops::Deref for FixtureCommand {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for FixtureCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Command> for FixtureCommand {
    fn from(command: Command) -> Self {
        Self(command)
    }
}

#[cfg(test)]
#[path = "fixture_command_controls.rs"]
mod controls;
