use std::process::Command;

use super::WrapperFixture;
use super::package_fixture::{
    create_artifact_api_response, create_fake_curl_bin, create_runtime_package,
    create_source_layout_plugin, install_cached_runtime,
};

pub(crate) fn assert_wrapper_installs_packaged_runtime_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let cache = temp.path().join("runtime-cache");
    let package_path = create_runtime_package(temp.path(), "darwin-arm64", server, "fresh")?;
    let plugin_root = create_source_layout_plugin(temp.path())?;

    let output = Command::new(plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", temp.path())
        .env("PATH", "/usr/bin:/bin")
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_PACKAGE_PATH", &package_path)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        output.status.success(),
        "fresh no-Cargo package fallback should run\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!("fake-packaged fresh codexy-mcp-{server} --help")),
        "wrapper should exec packaged runtime, got {stdout:?}"
    );
    Ok(())
}

pub(crate) fn assert_wrapper_discovers_default_artifact_without_cargo(
    server: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let cache = temp.path().join("runtime-cache");
    let package_path = create_runtime_package(temp.path(), "darwin-arm64", server, "artifact")?;
    let artifact_api = create_artifact_api_response(temp.path(), &package_path)?;
    let fake_bin = create_fake_curl_bin(temp.path(), &artifact_api)?;
    let plugin_root = create_source_layout_plugin(temp.path())?;

    let output = Command::new(plugin_root.join(format!("mcp/codexy-mcp-{server}")))
        .env("HOME", temp.path())
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .env("CODEXY_RUNTIME_CACHE_DIR", &cache)
        .env("CODEXY_RUNTIME_PLATFORM", "darwin-arm64")
        .arg("--help")
        .output()?;

    assert!(
        output.status.success(),
        "fresh no-Cargo default artifact fallback should run\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(&format!(
            "fake-packaged artifact codexy-mcp-{server} --help"
        )),
        "wrapper should exec default artifact runtime, got {stdout:?}"
    );
    let curl_log = std::fs::read_to_string(temp.path().join("curl.log"))?;
    assert!(
        curl_log.contains("per_page=100"),
        "default artifact lookup should request enough artifacts to skip PR outputs, got {curl_log:?}"
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
