use std::process::Command;

use super::release_cache::{
    assert_server_info, create_fake_curl_bin, create_runtime_package, initialize_wrapper,
    wrapper_command,
};
use super::{WrapperFixture, make_executable, release_version};

pub(crate) fn assert_wrapper_rejects_stale_default_release_then_accepts_matching_release(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let expected_release = release_version::current_plugin_release(&fixture.plugin_root)?;
    let stale_release = "0.0.0";
    let stale_root = temp.path().join("stale-release");
    let stale_package = create_runtime_package(&stale_root, server, stale_release)?;
    let stale_bin = create_fake_curl_bin(&stale_root, &stale_package)?;

    let stale_output = wrapper_command(&fixture, server, &cache, &stale_bin)
        .arg("--help")
        .output()?;
    let stale_stderr = String::from_utf8_lossy(&stale_output.stderr);
    assert!(
        !stale_output.status.success(),
        "stale releases/latest package must not run for {server}\nstdout:\n{}\nstderr:\n{stale_stderr}",
        String::from_utf8_lossy(&stale_output.stdout),
    );
    assert!(
        stale_stderr.contains(&format!(
            "runtime package release mismatch: expected {expected_release}, observed {stale_release}"
        )),
        "stale package diagnostic must identify expected and observed releases without unrelated output: {stale_stderr}"
    );
    assert!(
        !cache.exists() || std::fs::read_dir(&cache)?.next().is_none(),
        "stale package must leave no cache directory under the active key"
    );

    let matching_root = temp.path().join("matching-release");
    let matching_package = create_runtime_package(&matching_root, server, &expected_release)?;
    let matching_bin = create_fake_curl_bin(&matching_root, &matching_package)?;
    assert_server_info(
        initialize_wrapper(&fixture, server, &cache, &matching_bin)?,
        server,
        &expected_release,
    );
    Ok(())
}

pub(crate) fn assert_wrapper_allows_explicit_package_release_mismatch(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let package = create_runtime_package(temp.path(), server, "0.0.0")?;

    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .arg("--help")
        .env("HOME", fixture.home)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_PACKAGE_PATH", &package)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .output()?;
    assert!(
        output.status.success(),
        "explicit package override is an isolated opt-in and may use a mismatched release\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains(&format!("fake-packaged 0.0.0 codexy-mcp-{server} --help")),
        "explicit package override should run its requested runtime"
    );
    Ok(())
}

pub(crate) fn assert_wrapper_recovers_from_poisoned_v2_cache_with_matching_release(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let runtime = format!("codexy-mcp-{server}");
    let poisoned = cache
        .join(current_default_cache_key(&fixture, &runtime)?)
        .join("bin")
        .join(&runtime);
    std::fs::create_dir_all(poisoned.parent().ok_or("poisoned runtime has no parent")?)?;
    std::fs::write(&poisoned, "#!/bin/sh\necho poisoned-existing-cache\n")?;
    make_executable(&poisoned)?;

    let expected_release = release_version::current_plugin_release(&fixture.plugin_root)?;
    let release_root = temp.path().join("matching-release");
    let release = create_runtime_package(&release_root, server, &expected_release)?;
    let fake_bin = create_fake_curl_bin(&release_root, &release)?;
    let help = wrapper_command(&fixture, server, &cache, &fake_bin)
        .arg("--help")
        .output()?;
    assert!(
        help.status.success()
            && String::from_utf8_lossy(&help.stdout).contains(&format!(
                "fake-packaged {expected_release} {runtime} --help"
            )),
        "poisoned v2 cache must be replaced before execution\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&help.stdout),
        String::from_utf8_lossy(&help.stderr),
    );
    assert!(
        !std::fs::read_to_string(&poisoned)?.contains("poisoned-existing-cache"),
        "poisoned runtime must be removed before the matching runtime is cached"
    );
    assert_server_info(
        initialize_wrapper(&fixture, server, &cache, &fake_bin)?,
        server,
        &expected_release,
    );
    Ok(())
}

pub(crate) fn assert_wrapper_recovers_from_mismatched_cache_marker(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let runtime = format!("codexy-mcp-{server}");
    let cache_root = cache.join(current_default_cache_key(&fixture, &runtime)?);
    let poisoned = cache_root.join("bin").join(&runtime);
    std::fs::create_dir_all(poisoned.parent().ok_or("poisoned runtime has no parent")?)?;
    std::fs::write(&poisoned, "#!/bin/sh\necho poisoned-marker-cache\n")?;
    make_executable(&poisoned)?;
    std::fs::write(
        cache_root.join("plugin.json"),
        r#"{"name":"codexy","version":"0.0.0"}"#,
    )?;

    let expected_release = release_version::current_plugin_release(&fixture.plugin_root)?;
    let release_root = temp.path().join("matching-release");
    let release = create_runtime_package(&release_root, server, &expected_release)?;
    let fake_bin = create_fake_curl_bin(&release_root, &release)?;
    assert_server_info(
        initialize_wrapper(&fixture, server, &cache, &fake_bin)?,
        server,
        &expected_release,
    );
    Ok(())
}

fn current_default_cache_key(
    fixture: &WrapperFixture,
    runtime: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("python3")
        .arg(fixture.plugin_root.join("mcp/codexy-runtime-cache-key.py"))
        .arg(fixture.plugin_root.join(".codex-plugin/plugin.json"))
        .args([
            "0",
            "https://github.com/eunsoogi/codexy",
            "main",
            "darwin-arm64",
            "stdio-newline-v1",
            "package-default",
            runtime,
        ])
        .output()?;
    if !output.status.success() {
        return Err("runtime cache key helper failed".into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
