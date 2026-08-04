#![allow(clippy::redundant_pub_crate)]
#![allow(dead_code, unused_imports)]

#[path = "release_archive.rs"]
pub(crate) mod release_archive;
mod fixture_command {
    pub(crate) use super::FixtureCommand;
}
mod fixture_command_windows {
    pub(crate) use codexy_test_support::fixture_command_windows::*;
}
mod fixture_path {
    pub(crate) use codexy_test_support::fixture_path::*;
}
mod fixture_text {
    pub(crate) use codexy_test_support::fixture_text::*;
}
mod profile_interval_metrics {
    pub(crate) use codexy_test_support::profile_interval_metrics::*;
}
mod profile_metrics {
    pub(crate) use codexy_test_support::profile_metrics::*;
}
pub(crate) mod windows_archive_prerequisite {
    pub(crate) use codexy_test_support::windows_archive_prerequisite::*;
}
mod wrapper_copy {
    pub(crate) use codexy_test_support::wrapper_copy::*;
}
#[derive(Debug)]
pub(crate) struct FixtureCommand(codexy_test_support::FixtureCommand);
impl FixtureCommand {
    pub(crate) fn new(program: impl AsRef<std::ffi::OsStr>) -> Self {
        Self(codexy_test_support::FixtureCommand::new(
            program,
            std::ffi::OsStr::new(env!("CARGO_BIN_EXE_codexy-validate")),
        ))
    }
}
impl std::ops::Deref for FixtureCommand {
    type Target = codexy_test_support::FixtureCommand;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for FixtureCommand {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<std::process::Command> for FixtureCommand {
    fn from(command: std::process::Command) -> Self {
        Self(command.into())
    }
}
pub(crate) use codexy_test_support::{
    fixture_script_launcher, normalize_fixture_text, windows_fixture_companion,
    windows_static_python_fixture,
};

#[cfg(windows)]
use std::process::Command;

pub(crate) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(windows)]
    {
        let shell = fixture_command_windows::discover_windows_interpreter("sh")
            .map_err(std::io::Error::other)?;
        let path =
            fixture_path::fixture_path_text(path.as_os_str()).map_err(std::io::Error::other)?;
        let status = Command::new(shell)
            .args(["-c", "chmod +x -- \"$1\"", "fixture-mode", &path])
            .status()?;
        if !status.success() {
            return Err(std::io::Error::other("Windows fixture chmod failed"));
        }
    }
    #[cfg(all(not(unix), not(windows)))]
    let _ = path;
    Ok(())
}

pub(crate) fn materialize_admission_runtime_suite(
    plugin_root: &std::path::Path,
) -> std::io::Result<()> {
    let repository = fixture_repository(plugin_root)?;
    let suite = repository.join("tests/suites/all.rs");
    std::fs::create_dir_all(suite.parent().expect("suite parent"))?;
    std::fs::write(suite, "// admission runtime suite fixture\n")
}

fn fixture_repository(plugin_root: &std::path::Path) -> std::io::Result<&std::path::Path> {
    let parent = plugin_root.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "fixture plugin root needs a parent",
        )
    })?;
    if parent.file_name().is_some_and(|name| name == "plugins") {
        parent.parent().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "fixture plugin directory needs a repository parent",
            )
        })
    } else {
        Ok(parent)
    }
}
