#![allow(clippy::redundant_pub_crate)]

mod package;
mod package_fixture;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

pub(super) use package::{
    assert_wrapper_discovers_default_artifact_without_cargo,
    assert_wrapper_installs_packaged_runtime_without_cargo,
    assert_wrapper_keeps_ref_override_exact_without_package_override,
    assert_wrapper_prefers_durable_default_package_without_cargo,
    assert_wrapper_refreshes_package_before_stale_cache_without_cargo,
    assert_wrapper_requires_token_for_default_artifact_without_cargo,
};

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

pub(super) fn run_wrapper_with_optional_failure(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    runtime_ref: &str,
    fake_version: &str,
    fail_cargo: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
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

pub(super) fn copy_dir(
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

pub(super) fn make_executable(path: &std::path::Path) -> std::io::Result<()> {
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
