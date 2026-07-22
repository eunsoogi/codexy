#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use super::wrapper_copy::copy_dir;
pub(crate) use super::wrapper_process::wait_for_wrapper_output;

const WRAPPER_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct WrapperFixture<'a> {
    pub(crate) home: &'a std::path::Path,
    pub(crate) plugin_root: std::path::PathBuf,
    pub(crate) cargo_bin: std::path::PathBuf,
}

impl<'a> WrapperFixture<'a> {
    pub(crate) fn new(home: &'a std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let plugin_root = home.join("codexy");
        copy_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
        let cargo_bin = home.join("fake-bin");
        std::fs::create_dir_all(&cargo_bin)?;
        Ok(Self {
            home,
            plugin_root,
            cargo_bin,
        })
    }

    pub(crate) fn replace_wrapper(&self, server: &str, script: &str) -> std::io::Result<()> {
        let wrapper = self.plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        std::fs::write(&wrapper, script)?;
        make_executable(&wrapper)
    }
}

pub(crate) fn run_wrapper_command(
    command: &mut Command,
) -> Result<Output, Box<dyn std::error::Error>> {
    run_wrapper_command_with_timeout(command, WRAPPER_TIMEOUT)
}

pub(crate) fn run_wrapper_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let description = format!("{command:?}");
    let child = spawn_wrapper_command(command.stdout(Stdio::piped()).stderr(Stdio::piped()))?;
    wait_for_wrapper_output(child, description, timeout)
}

/// Spawns a wrapper as a process-group leader so the timeout helper can reap its descendants.
fn spawn_wrapper_command(command: &mut Command) -> std::io::Result<Child> {
    #[cfg(unix)]
    command.process_group(0);
    command.spawn()
}

pub(crate) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub(crate) fn published_bootstrap_version(
    root: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    contract["bootstrapVersion"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "release bootstrapVersion".into())
}

pub(crate) fn next_bootstrap_version(version: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (major, minor, patch) = version
        .split_once('.')
        .and_then(|(major, rest)| {
            rest.split_once('.')
                .map(|(minor, patch)| (major, minor, patch))
        })
        .ok_or("bootstrapVersion semver")?;
    Ok(format!("{major}.{minor}.{}", patch.parse::<u64>()? + 1))
}
