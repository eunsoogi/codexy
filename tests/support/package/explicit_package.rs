use super::*;

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
