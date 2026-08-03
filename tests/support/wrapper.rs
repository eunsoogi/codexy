#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use super::fixture_command::FixtureCommand;
use super::package_fixture::create_runtime_package;
use super::wrapper_copy::copy_wrapper_surface;
use super::wrapper_process::{
    WrapperChild, wait_for_wrapper_output as wait_for_wrapper_output_inner,
};

const WRAPPER_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) struct WrapperFixture<'a> {
    pub(crate) home: &'a std::path::Path,
    pub(crate) plugin_root: std::path::PathBuf,
    pub(crate) cargo_bin: std::path::PathBuf,
    pub(crate) cargo_log: std::path::PathBuf,
}

impl<'a> WrapperFixture<'a> {
    pub(crate) fn new(home: &'a std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
        let plugin_root = home.join("codexy");
        let source_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy");
        copy_wrapper_surface(&source_root, &plugin_root)?;
        let cargo_bin = home.join("fake-bin");
        std::fs::create_dir_all(&cargo_bin)?;
        let cargo_log = home.join("cargo.log");
        let cargo_path = cargo_bin.join("cargo");
        std::fs::write(
            &cargo_path,
            format!(
                "#!/bin/sh\n\\
                 set -eu\n\\
                 echo \"$@\" >> '{}'\n\\
                 if [ \"${{FAKE_CARGO_FAIL:-0}}\" = 1 ]; then\n\\
                   echo fake cargo failure >&2\n\\
                   exit 42\n\\
                 fi\n\\
                 root=\"\"\n\\
                 bin=\"\"\n\\
                 while [ \"$#\" -gt 0 ]; do\n\\
                   case \"$1\" in\n\\
                     --root) root=\"$2\"; shift 2 ;;\n\\
                     --bin) bin=\"$2\"; shift 2 ;;\n\\
                     *) shift ;;\n\\
                   esac\n\\
                 done\n\\
                 mkdir -p \"$root/bin\"\n\\
                 printf '#!/bin/sh\\necho fake-installed %s %s \"$@\"\\n' \"${{FAKE_RUNTIME_VERSION:-current}}\" \"$bin\" > \"$root/bin/$bin\"\n\\
                 chmod 755 \"$root/bin/$bin\"\n",
                cargo_log.display()
            ),
        )?;
        make_executable(&cargo_path)?;
        Ok(Self {
            home,
            plugin_root,
            cargo_bin,
            cargo_log,
        })
    }

    pub(crate) fn replace_wrapper(&self, server: &str, script: &str) -> std::io::Result<()> {
        let wrapper = self.plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        std::fs::write(&wrapper, script)?;
        make_executable(&wrapper)
    }

    pub(crate) fn select_platform(
        &self,
        platform: super::wrapper_platform::FixturePlatform,
    ) -> std::io::Result<()> {
        super::wrapper_platform::install_fixture_platform(&self.plugin_root, platform)
    }
}

pub(crate) fn run_wrapper(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    runtime_ref: &str,
    fake_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    run_wrapper_with_optional_failure(fixture, server, cache, runtime_ref, fake_version, false)
}

pub(crate) fn run_wrapper_with_optional_failure(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    runtime_ref: &str,
    fake_version: &str,
    fail_cargo: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut command =
        FixtureCommand::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
    command
        .env("HOME", fixture.home)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        )
        .env("CODEXY_RUNTIME_CACHE_DIR", cache)
        .env("CODEXY_RUNTIME_GIT_REF", runtime_ref)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .env("FAKE_RUNTIME_VERSION", fake_version)
        .env("FAKE_CARGO_FAIL", if fail_cargo { "1" } else { "0" })
        .arg("--help");
    let output = run_wrapper_command(&mut command)?;
    assert!(
        output.status.success(),
        "wrapper should run the bootstrapped runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

pub(crate) fn run_wrapper_command(
    command: &mut Command,
) -> Result<Output, Box<dyn std::error::Error>> {
    run_wrapper_command_with_timeout(command, WRAPPER_TIMEOUT)
}

pub(crate) fn wait_for_default_wrapper_output(
    child: WrapperChild,
    description: String,
) -> Result<Output, Box<dyn std::error::Error>> {
    wait_for_wrapper_output(child, description, WRAPPER_TIMEOUT)
}

pub(crate) fn wait_for_wrapper_output(
    child: WrapperChild,
    description: String,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let interval =
        super::profile_interval_metrics::generic_interval("wrapper.child-wait.other", "other");
    let result = wait_for_wrapper_output_inner(child, description, timeout);
    drop(interval);
    result
}

pub(crate) fn run_wrapper_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let interval =
        super::profile_interval_metrics::wrapper_interval("output", command.get_program());
    let description = format!("{command:?}");
    let child = spawn_wrapper_command(command.stdout(Stdio::piped()).stderr(Stdio::piped()))?;
    let result = wait_for_wrapper_output(child, description, timeout);
    drop(interval);
    result
}

/// Spawns a wrapper as a process-group leader so the timeout helper can reap its descendants.
pub(crate) fn spawn_wrapper_command(command: &mut Command) -> std::io::Result<WrapperChild> {
    let interval =
        super::profile_interval_metrics::wrapper_interval("spawn", command.get_program());
    #[cfg(unix)]
    command.process_group(0);
    let result = super::wrapper_process::spawn_wrapper_child(command);
    drop(interval);
    result
}

pub(crate) trait WrapperCommandExt {
    fn output_with_timeout(&mut self) -> Result<Output, Box<dyn std::error::Error>>;
}

impl WrapperCommandExt for Command {
    fn output_with_timeout(&mut self) -> Result<Output, Box<dyn std::error::Error>> {
        run_wrapper_command(self)
    }
}

pub(crate) fn assert_wrapper_uses_package_runtime_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let package = create_runtime_package(temp.path(), "darwin-arm64", server, "override")?;
    let mut command =
        FixtureCommand::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
    command
        .arg("--help")
        .env("HOME", fixture.home)
        .env(
            "PATH",
            format!("{}:/usr/bin:/bin", fixture.cargo_bin.display()),
        )
        .env(
            "CODEXY_RUNTIME_CACHE_DIR",
            temp.path().join("runtime-cache"),
        )
        .env("CODEXY_RUNTIME_PACKAGE_PATH", package)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64");
    let output = run_wrapper_command(&mut command)?;

    assert!(
        output.status.success(),
        "wrapper should exec the packaged runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&format!(
            "fake-packaged override codexy-mcp-{server} --help"
        )),
        "packaged runtime marker missing\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !fixture.cargo_log.exists(),
        "Cargo should not be invoked when the packaged runtime is available"
    );
    Ok(())
}

pub(crate) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(windows)]
    {
        let shell = super::fixture_command_windows::discover_windows_interpreter("sh")
            .map_err(std::io::Error::other)?;
        let path = super::fixture_path::fixture_path_text(path.as_os_str())
            .map_err(std::io::Error::other)?;
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
