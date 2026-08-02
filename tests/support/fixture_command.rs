use std::ffi::{OsStr, OsString};
use std::process::Command;
#[path = "archive_inspection_receipt.rs"]
mod archive_inspection_receipt;
#[path = "fixture_command_metrics.rs"]
mod metrics;
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
use archive_inspection_receipt as receipt;
/// A test-only factory for native commands and POSIX fixture scripts on Windows.
#[derive(Debug)]
pub(crate) struct FixtureCommand {
    command: Command,
    uses_posix_paths: bool,
    receipt: Option<receipt::ArchiveInspectorReceipt>,
}
impl FixtureCommand {
    pub(crate) fn new(program: impl AsRef<std::ffi::OsStr>) -> Self {
        let program = program.as_ref();
        #[cfg(windows)]
        if let Some(command) = windows_static_python_command(std::path::Path::new(program))
            .unwrap_or_else(|error| panic!("{error}"))
        {
            return Self::from_command(command, false, program);
        }
        #[cfg(windows)]
        if let Some(companion) = windows_fixture_companion(std::path::Path::new(program)) {
            return Self::from_command(Command::new(companion), false, program);
        }
        if let Some(source) = materialized_script_source(std::path::Path::new(program)) {
            let (command, uses_posix_paths) = materialized_script_command(program, &source)
                .unwrap_or_else(|error| panic!("{error}"));
            return Self::from_command(command, uses_posix_paths, program);
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
                        command.arg(&program);
                        return Self::from_command(command, uses_posix_path, &program);
                    }
                    Ok(None) => {}
                    Err(error) => panic!("{error}"),
                }
            }
        }
        Self::from_command(Command::new(program), false, program)
    }

    fn from_command(mut command: Command, uses_posix_paths: bool, program: &OsStr) -> Self {
        let receipt = receipt::configure_command(&mut command, program, |directory| {
            if uses_posix_paths {
                fixture_path_text(directory)
                    .unwrap_or_else(|error| panic!("{error}"))
                    .into()
            } else {
                directory.as_os_str().to_owned()
            }
        });
        let test_mode =
            receipt.is_some() || matches!(std::env::var("CODEXY_TEST_MODE").as_deref(), Ok("1"));
        let mut fixture = Self {
            command,
            uses_posix_paths,
            receipt,
        };
        if test_mode {
            fixture.env_path(
                "CODEXY_TEST_VALIDATE_PLUGIN_CONFIG_BINARY",
                env!("CARGO_BIN_EXE_codexy-validate"),
            );
        }
        fixture
    }

    pub(crate) fn env<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let key = key.as_ref();
        let value = if self.uses_posix_paths {
            fixture_path_environment_value(key, value.as_ref())
                .unwrap_or_else(|error| panic!("{error}"))
        } else {
            value.as_ref().to_owned()
        };
        self.command.env(key, value);
        self
    }

    pub(crate) fn envs<K, V, I>(&mut self, variables: I) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
        I: IntoIterator<Item = (K, V)>,
    {
        variables
            .into_iter()
            .fold(self, |fixture, (key, value)| fixture.env(key, value))
    }

    pub(crate) fn arg_path(&mut self, path: impl AsRef<OsStr>) -> &mut Self {
        let path = self.path_value(path.as_ref());
        self.command.arg(path);
        self
    }

    pub(crate) fn env_path<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let value = self.path_value(value.as_ref());
        self.command.env(key, value);
        self
    }

    pub(crate) fn env_path_list<K, I, V>(&mut self, key: K, values: I) -> &mut Self
    where
        K: AsRef<OsStr>,
        I: IntoIterator<Item = V>,
        V: AsRef<OsStr>,
    {
        let values = values
            .into_iter()
            .map(|value| self.path_value(value.as_ref()))
            .collect::<Vec<_>>();
        let value = if self.uses_posix_paths {
            values
                .iter()
                .map(|value| value.to_string_lossy())
                .collect::<Vec<_>>()
                .join(":")
                .into()
        } else {
            std::env::join_paths(values)
                .unwrap_or_else(|error| panic!("joining fixture paths: {error}"))
        };
        self.command.env(key, value);
        self
    }

    fn path_value(&self, value: &OsStr) -> OsString {
        if self.uses_posix_paths {
            fixture_path_text(value)
                .unwrap_or_else(|error| panic!("{error}"))
                .into()
        } else {
            value.to_owned()
        }
    }
}

fn materialized_script_command(
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

impl std::ops::Deref for FixtureCommand {
    type Target = Command;

    fn deref(&self) -> &Self::Target {
        &self.command
    }
}

impl std::ops::DerefMut for FixtureCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}

impl From<Command> for FixtureCommand {
    fn from(command: Command) -> Self {
        Self {
            command,
            uses_posix_paths: false,
            receipt: None,
        }
    }
}

#[cfg(test)]
#[path = "fixture_command_controls.rs"]
mod controls;
