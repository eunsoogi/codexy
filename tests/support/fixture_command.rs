use std::process::Command;

/// A test-only command factory that preserves direct native execution and launches POSIX
/// fixture scripts through `sh` on Windows.
#[derive(Debug)]
pub(crate) struct FixtureCommand(Command);

impl FixtureCommand {
    pub(crate) fn new(program: impl AsRef<std::ffi::OsStr>) -> Self {
        let program = program.as_ref();
        #[cfg(windows)]
        {
            if let Ok(contents) = std::fs::read(std::path::Path::new(program)) {
                if fixture_script_launcher(true, &contents).is_some() {
                    let mut command = Command::new("sh");
                    command.arg(program);
                    return Self(command);
                }
            }
        }
        Self(Command::new(program))
    }
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

fn fixture_script_launcher(is_windows: bool, contents: &[u8]) -> Option<&'static str> {
    (is_windows && contents.starts_with(b"#!/bin/sh")).then_some("sh")
}

#[test]
fn shell_fixture_launcher_uses_sh_only_for_windows_shell_scripts() {
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh\necho fixture\n"),
        Some("sh")
    );
    assert_eq!(
        fixture_script_launcher(true, b"#!/bin/sh\r\necho fixture\r\n"),
        Some("sh")
    );
    assert_eq!(fixture_script_launcher(true, b"MZ\x90\0"), None);
    assert_eq!(
        fixture_script_launcher(false, b"#!/bin/sh\necho fixture\n"),
        None
    );
}
