#![allow(clippy::redundant_pub_crate)]

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

pub(super) struct WrapperFixture<'a> {
    pub(super) home: &'a std::path::Path,
    pub(super) plugin_root: std::path::PathBuf,
    pub(super) cargo_bin: std::path::PathBuf,
    pub(super) cargo_log: std::path::PathBuf,
}

impl<'a> WrapperFixture<'a> {
    pub(super) fn new(home: &'a std::path::Path) -> Result<Self, Box<dyn std::error::Error>> {
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
                "#!/bin/sh\n\
                 set -eu\n\
                 echo \"$@\" >> '{}'\n\
                 if [ \"${{FAKE_CARGO_FAIL:-0}}\" = 1 ]; then\n\
                   echo fake cargo failure >&2\n\
                   exit 42\n\
                 fi\n\
                 root=\"\"\n\
                 bin=\"\"\n\
                 while [ \"$#\" -gt 0 ]; do\n\
                   case \"$1\" in\n\
                     --root) root=\"$2\"; shift 2 ;;\n\
                     --bin) bin=\"$2\"; shift 2 ;;\n\
                     *) shift ;;\n\
                   esac\n\
                 done\n\
                 mkdir -p \"$root/bin\"\n\
                 printf '#!/bin/sh\\necho fake-installed %s %s \"$@\"\\n' \"${{FAKE_RUNTIME_VERSION:-current}}\" \"$bin\" > \"$root/bin/$bin\"\n\
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

    pub(super) fn runtime_package(
        &self,
        server: &str,
        fake_version: &str,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let package_root = self.home.join("package-root");
        let runtime_dir = package_root.join("plugins/codexy/runtime");
        std::fs::create_dir_all(&runtime_dir)?;
        let runtime_name = format!("codexy-mcp-{server}-darwin-arm64.bin");
        let runtime_path = runtime_dir.join(&runtime_name);
        std::fs::write(
            &runtime_path,
            format!(
                "#!/bin/sh\n\
                 echo fake-packaged {fake_version} codexy-mcp-{server} \"$@\"\n"
            ),
        )?;
        make_executable(&runtime_path)?;
        let package = self.home.join("codexy-marketplace-plugin.tar.gz");
        let status = Command::new("tar")
            .arg("-czf")
            .arg(&package)
            .arg("-C")
            .arg(&package_root)
            .arg("plugins/codexy")
            .status()?;
        assert!(status.success(), "tar should create runtime package");
        Ok(package)
    }
}

pub(super) fn run_wrapper(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    runtime_ref: &str,
    fake_version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    run_wrapper_with_optional_failure(fixture, server, cache, runtime_ref, fake_version, false)
}

pub(super) fn run_wrapper_with_package(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    runtime_ref: &str,
    fake_version: &str,
    package: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = wrapper_command(fixture, server, cache, runtime_ref, fake_version, false)
        .env(
            "CODEXY_RUNTIME_PACKAGE_URL",
            format!("file://{}", package.display()),
        )
        .arg("--help")
        .output()?;
    assert!(
        output.status.success(),
        "wrapper should run the bootstrapped runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

pub(super) fn run_wrapper_with_optional_failure(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    runtime_ref: &str,
    fake_version: &str,
    fail_cargo: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = wrapper_command(
        fixture,
        server,
        cache,
        runtime_ref,
        fake_version,
        fail_cargo,
    )
    .arg("--help")
    .output()?;
    assert!(
        output.status.success(),
        "wrapper should run the bootstrapped runtime\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?)
}

fn wrapper_command(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    runtime_ref: &str,
    fake_version: &str,
    fail_cargo: bool,
) -> Command {
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
        .env("FAKE_CARGO_FAIL", if fail_cargo { "1" } else { "0" });
    command
}

pub(super) fn runtime_cache_contains_executable(
    cache: &std::path::Path,
) -> Result<bool, Box<dyn std::error::Error>> {
    if !cache.exists() {
        return Ok(false);
    }
    for entry in std::fs::read_dir(cache)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let permissions = entry.metadata()?.permissions();
            #[cfg(unix)]
            {
                if permissions.mode() & 0o111 != 0 {
                    return Ok(true);
                }
            }
        }
        if entry.file_type()?.is_dir() && runtime_cache_contains_executable(&entry.path())? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn copy_dir(source: impl AsRef<std::path::Path>, target: &std::path::Path) -> std::io::Result<()> {
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

fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
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
