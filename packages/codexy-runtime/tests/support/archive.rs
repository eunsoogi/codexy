#![allow(clippy::redundant_pub_crate)]
#![allow(dead_code, unused_imports)]

#[path = "fixture_command.rs"]
mod fixture_command;
#[path = "fixture_command_windows.rs"]
mod fixture_command_windows;
#[path = "fixture_path.rs"]
mod fixture_path;
#[path = "fixture_text.rs"]
mod fixture_text;
#[path = "profile_interval_metrics.rs"]
mod profile_interval_metrics;
#[path = "profile_metrics.rs"]
mod profile_metrics;
#[path = "release_archive.rs"]
pub(crate) mod release_archive;
#[path = "windows_archive_prerequisite.rs"]
pub(crate) mod windows_archive_prerequisite;
#[path = "wrapper_copy.rs"]
mod wrapper_copy;

pub(crate) use fixture_command::{
    FixtureCommand, fixture_script_launcher, windows_fixture_companion,
    windows_static_python_fixture,
};
pub(crate) use fixture_text::normalize_fixture_text;

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
    let suite = repository.join("packages/codexy-runtime/tests/suites/all.rs");
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
