use super::*;

pub(crate) fn assert_wrapper_reuses_cache_before_default_package_refresh_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    install_cached_runtime(
        &cache,
        "https://github.com/eunsoogi/codexy",
        "main",
        "darwin-arm64",
        server,
        "cached",
    )?;
    let release_package = create_runtime_package(temp.path(), "darwin-arm64", server, "fresh")?;
    let artifact_api = create_artifact_api_response(temp.path(), &release_package)?;
    let fake_bin = create_fake_curl_bin_with_release_package(
        temp.path(),
        &artifact_api,
        Some(&release_package),
    )?;

    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        output.status.success(),
        "cached runtime should run before default package refresh\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!("fake-installed cached codexy-mcp-{server} --help")),
        "wrapper should exec cached runtime before implicit package refresh, got {stdout:?}"
    );
    assert!(
        !temp.path().join("curl.log").exists(),
        "implicit default package lookup should not run when cached runtime exists"
    );
    Ok(())
}

pub(crate) fn assert_wrapper_ignores_legacy_cache_before_default_package_refresh_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    install_legacy_cached_runtime(
        &cache,
        "https://github.com/eunsoogi/codexy",
        "main",
        "darwin-arm64",
        server,
        "legacy",
    )?;
    let release_package = create_runtime_package(temp.path(), "darwin-arm64", server, "fresh")?;
    let artifact_api = create_artifact_api_response(temp.path(), &release_package)?;
    let fake_bin = create_fake_curl_bin_with_release_package(
        temp.path(),
        &artifact_api,
        Some(&release_package),
    )?;

    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        output.status.success(),
        "default package refresh should replace legacy cache\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!("fake-packaged fresh codexy-mcp-{server} --help")),
        "wrapper should ignore legacy cache and exec refreshed runtime, got {stdout:?}"
    );
    let curl_log = std::fs::read_to_string(temp.path().join("curl.log"))?;
    assert!(
        curl_log.contains("releases/latest/download/codexy-marketplace-plugin.tar.gz"),
        "legacy cache should not bypass default package lookup, got {curl_log:?}"
    );
    Ok(())
}
