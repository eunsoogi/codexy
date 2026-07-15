#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use super::package_fixture::create_runtime_package;

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
        copy_dir(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
            &plugin_root,
        )?;
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
    let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
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
    child: Child,
    description: String,
) -> Result<Output, Box<dyn std::error::Error>> {
    wait_for_wrapper_output(child, description, WRAPPER_TIMEOUT)
}

pub(crate) fn run_wrapper_command_with_timeout(
    command: &mut Command,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let description = format!("{command:?}");
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    wait_for_wrapper_output(child, description, timeout)
}

pub(crate) trait WrapperCommandExt {
    fn output_with_timeout(&mut self) -> Result<Output, Box<dyn std::error::Error>>;
}

impl WrapperCommandExt for Command {
    fn output_with_timeout(&mut self) -> Result<Output, Box<dyn std::error::Error>> {
        run_wrapper_command(self)
    }
}

pub(crate) fn wait_for_wrapper_output(
    mut child: Child,
    description: String,
    timeout: Duration,
) -> Result<Output, Box<dyn std::error::Error>> {
    let started = Instant::now();

    loop {
        if child.try_wait()?.is_some() {
            return Ok(child.wait_with_output()?);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            let message = format!(
                "wrapper subprocess timed out after {}s: {description}\nstdout:\n{}\nstderr:\n{}",
                timeout.as_secs(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, message).into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub(crate) fn assert_wrapper_uses_package_runtime_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let package = create_runtime_package(temp.path(), "darwin-arm64", server, "override")?;
    let mut command = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")));
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

pub(crate) fn copy_dir(
    source: impl AsRef<std::path::Path>,
    target: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
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
