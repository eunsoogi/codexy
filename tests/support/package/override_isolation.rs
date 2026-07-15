use super::*;

pub(crate) fn assert_wrapper_does_not_reuse_package_override_as_default_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let fixture = WrapperFixture::new(temp.path())?;
    let cache = temp.path().join("runtime-cache");
    let override_root = temp.path().join("override");
    let release_root = temp.path().join("release");
    std::fs::create_dir_all(&override_root)?;
    std::fs::create_dir_all(&release_root)?;
    let override_package =
        create_runtime_package(&override_root, "darwin-arm64", server, "override")?;
    let release_package = create_runtime_package(&release_root, "darwin-arm64", server, "default")?;

    let override_output =
        Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
            .env("HOME", fixture.home)
            .env("PATH", "/usr/bin:/bin")
            .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
            .env("CODEXY_RUNTIME_PACKAGE_PATH", &override_package)
            .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
            .arg("--help")
            .output()?;
    assert!(
        override_output.status.success(),
        "explicit package override should install and run\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&override_output.stdout),
        String::from_utf8_lossy(&override_output.stderr)
    );
    let override_stdout = String::from_utf8(override_output.stdout)?;
    assert!(
        override_stdout.contains(&format!(
            "fake-packaged override codexy-mcp-{server} --help"
        )),
        "wrapper should exec explicit override runtime, got {override_stdout:?}"
    );

    let artifact_api = create_artifact_api_response(temp.path(), &release_package)?;
    let fake_bin = create_fake_curl_bin_with_release_package(
        temp.path(),
        &artifact_api,
        Some(&release_package),
    )?;
    let default_output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;
    assert!(
        default_output.status.success(),
        "default package lookup should run after override env is removed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&default_output.stdout),
        String::from_utf8_lossy(&default_output.stderr)
    );
    let default_stdout = String::from_utf8(default_output.stdout)?;
    assert!(
        default_stdout.contains(&format!("fake-packaged default codexy-mcp-{server} --help")),
        "wrapper should exec default package runtime, got {default_stdout:?}"
    );
    assert!(
        !default_stdout.contains("fake-packaged override"),
        "explicit override cache must not shadow the default runtime, got {default_stdout:?}"
    );
    let curl_log = std::fs::read_to_string(temp.path().join("curl.log"))?;
    assert!(
        curl_log.contains("releases/latest/download/codexy-marketplace-plugin.tar.gz"),
        "default lookup should fetch the release package after override removal, got {curl_log:?}"
    );
    Ok(())
}

pub(crate) fn assert_wrapper_refreshes_package_before_stale_cache_without_cargo(
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
        "stale",
    )?;
    let package_path = create_runtime_package(temp.path(), "darwin-arm64", server, "fresh")?;

    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_PACKAGE_PATH", &package_path)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        output.status.success(),
        "package refresh should run before stale no-Cargo cache\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!("fake-packaged fresh codexy-mcp-{server} --help")),
        "wrapper should exec fresh packaged runtime before stale cache, got {stdout:?}"
    );
    Ok(())
}

pub(crate) fn assert_wrapper_keeps_ref_override_exact_without_package_override(
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
        "default-main",
    )?;

    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env(
            "CODEXY_RUNTIME_GIT_REPOSITORY",
            "https://github.com/example/codexy",
        )
        .env("CODEXY_RUNTIME_GIT_REF", "release-candidate")
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        !output.status.success(),
        "explicit ref override must not use default main package/cache\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let package_path = create_runtime_package(temp.path(), "darwin-arm64", server, "override")?;
    let output = Command::new(fixture.plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", fixture.home)
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env(
            "CODEXY_RUNTIME_GIT_REPOSITORY",
            "https://github.com/example/codexy",
        )
        .env("CODEXY_RUNTIME_GIT_REF", "release-candidate")
        .env("CODEXY_RUNTIME_PACKAGE_PATH", &package_path)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;
    assert!(
        output.status.success(),
        "explicit package override should satisfy explicit ref without Cargo\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!(
            "fake-packaged override codexy-mcp-{server} --help"
        )),
        "wrapper should exec explicit package runtime, got {stdout:?}"
    );
    Ok(())
}
