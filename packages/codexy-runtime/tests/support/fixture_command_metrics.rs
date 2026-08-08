use std::ffi::OsStr;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::Instant;

use super::{FixtureCommand, receipt};

const OUTPUT_KEY: &str = "fixture-command.output.unattributed";
const SPAWN_KEY: &str = "fixture-command.spawn.unattributed";
const STATUS_KEY: &str = "fixture-command.status.unattributed";
const OUTPUT_INTERVAL: &str = "fixture-command.output";
const SPAWN_INTERVAL: &str = "fixture-command.spawn";
const STATUS_INTERVAL: &str = "fixture-command.status";

impl FixtureCommand {
    pub(crate) fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.command.arg(arg);
        self
    }

    pub(crate) fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    pub(crate) fn current_dir(&mut self, directory: impl AsRef<Path>) -> &mut Self {
        self.command.current_dir(directory);
        self
    }

    pub(crate) fn stdin(&mut self, config: Stdio) -> &mut Self {
        self.command.stdin(config);
        self
    }

    pub(crate) fn stdout(&mut self, config: Stdio) -> &mut Self {
        self.command.stdout(config);
        self
    }

    pub(crate) fn stderr(&mut self, config: Stdio) -> &mut Self {
        self.command.stderr(config);
        self
    }

    pub(crate) fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.command.env_remove(key);
        self
    }

    pub(crate) fn env_clear(&mut self) -> &mut Self {
        self.command.env_clear();
        self
    }

    #[track_caller]
    pub(crate) fn output(&mut self) -> std::io::Result<Output> {
        let interval = super::super::profile_interval_metrics::command_interval_at(
            OUTPUT_INTERVAL,
            self.command_family,
            std::panic::Location::caller(),
        );
        let started = Instant::now();
        let result = receipt::output(&mut self.command, self.receipt.as_ref());
        super::super::profile_metrics::record_command_wait(
            OUTPUT_KEY,
            self.command_family,
            started.elapsed(),
        );
        drop(interval);
        result
    }

    pub(crate) fn status(&mut self) -> std::io::Result<ExitStatus> {
        self.measure(STATUS_KEY, STATUS_INTERVAL, Command::status)
    }

    pub(crate) fn spawn(&mut self) -> std::io::Result<Child> {
        self.measure(SPAWN_KEY, SPAWN_INTERVAL, Command::spawn)
    }

    fn measure<T>(
        &mut self,
        key: &str,
        interval_key: &'static str,
        invoke: impl FnOnce(&mut Command) -> std::io::Result<T>,
    ) -> std::io::Result<T> {
        let interval = super::super::profile_interval_metrics::command_interval(
            interval_key,
            self.command_family,
        );
        let started = Instant::now();
        let result = invoke(&mut self.command);
        super::super::profile_metrics::record_command_wait(
            key,
            self.command_family,
            started.elapsed(),
        );
        drop(interval);
        result
    }
}
