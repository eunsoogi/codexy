use std::process::Command;

use super::{WrapperFixture, make_executable};

pub(crate) fn assert_wrapper_reuses_default_package_git_fallback(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let failed_package_bin = create_failed_package_bin(temp.path())?;

    let first = run_failed_package_with_cargo(
        &fixture,
        server,
        &cache,
        &failed_package_bin,
        "git-fallback",
    )?;
    let second = run_failed_package_with_cargo(
        &fixture,
        server,
        &cache,
        &failed_package_bin,
        "unwanted-reinstall",
    )?;
    assert!(
        first.contains("git-fallback"),
        "first launch must run Cargo fallback"
    );
    assert!(
        second.contains("git-fallback") && !second.contains("unwanted-reinstall"),
        "second launch must reuse safely identified Git fallback, got {second:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&fixture.cargo_log)?
            .matches(&format!("--bin codexy-mcp-{server}"))
            .count(),
        1,
        "default-package Git fallback must install only once"
    );
    Ok(())
}

fn run_failed_package_with_cargo(
    fixture: &WrapperFixture,
    server: &str,
    cache: &std::path::Path,
    failed_package_bin: &std::path::Path,
    version: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .arg("--help")
        .env("HOME", fixture.home)
        .env(
            "PATH",
            format!(
                "{}:{}:/usr/bin:/bin",
                failed_package_bin.display(),
                fixture.cargo_bin.display()
            ),
        )
        .env("CODEXY_RUNTIME_CACHE_DIR", cache)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .env("FAKE_RUNTIME_VERSION", version)
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "wrapper failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn create_failed_package_bin(
    root: &std::path::Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let bin = root.join("failed-package-bin");
    std::fs::create_dir_all(&bin)?;
    let curl = bin.join("curl");
    std::fs::write(&curl, "#!/bin/sh\nexit 22\n")?;
    make_executable(&curl)?;
    Ok(bin)
}
