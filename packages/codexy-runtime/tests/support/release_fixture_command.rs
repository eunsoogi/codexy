use std::{ffi::OsStr, io, path::Path, process::Output};

use super::FixtureCommand;

/// Launches a declared release-script fixture with explicit scalar and path inputs.
///
/// Copied release scripts execute under a POSIX shell on Windows. Filesystem values
/// must therefore cross the fixture boundary through `path`, while release metadata
/// and booleans remain scalars.
pub(crate) struct ReleaseFixtureCommand {
    command: FixtureCommand,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ReleaseFixtureOutcome {
    Success,
    Failure,
}

impl ReleaseFixtureCommand {
    pub(crate) fn new(script: impl AsRef<OsStr>) -> Self {
        Self {
            command: FixtureCommand::new(script),
        }
    }

    pub(crate) fn current_dir(&mut self, directory: impl AsRef<Path>) -> &mut Self {
        self.command.current_dir(directory);
        self
    }

    pub(crate) fn arg(&mut self, value: impl AsRef<OsStr>) -> &mut Self {
        self.command.arg(value);
        self
    }

    pub(crate) fn args<I, S>(&mut self, values: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(values);
        self
    }

    pub(crate) fn arg_path(&mut self, value: impl AsRef<OsStr>) -> &mut Self {
        self.command.arg_path(value);
        self
    }

    pub(crate) fn scalar(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.command.env(key, value);
        self
    }

    pub(crate) fn path(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.command.env_path(key, value);
        self
    }

    /// Preserves a path for a native payload launched by the POSIX fixture shell.
    ///
    /// The shell still receives projected paths for its own files. This channel is
    /// only for an environment value that the shell forwards to a native payload.
    pub(crate) fn payload_path(
        &mut self,
        key: impl AsRef<OsStr>,
        value: impl AsRef<OsStr>,
    ) -> &mut Self {
        self.command.env(key, value);
        self
    }

    pub(crate) fn output(&mut self) -> io::Result<Output> {
        self.command.output()
    }

    pub(crate) fn assert_outcome<'a>(
        operation: &str,
        expected: ReleaseFixtureOutcome,
        output: &'a Output,
    ) -> &'a Output {
        let expected_success = matches!(expected, ReleaseFixtureOutcome::Success);
        assert!(
            output.status.success() == expected_success,
            "{operation} expected {expected:?}, exited {:?}; stdout: {}; stderr: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        output
    }
}
