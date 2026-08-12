use super::*;

#[test]
fn validator_cli_rejects_packaged_plugin_without_generated_runtime_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    select_candidate_platforms(&plugin_root)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "packaged artifact validation should reject missing generated runtime binaries"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("codexy-mcp-lsp-darwin-arm64.bin bundled MCP runtime missing"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_packaged_plugin_with_generated_runtime_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    select_candidate_platforms(&plugin_root)?;
    for runtime_name in packaged_runtime_names() {
        write_runtime_fixture(
            &plugin_root,
            runtime_name,
            &runtime_binary_fixture(runtime_name),
        )?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(
        output.status.success(),
        "packaged artifact validation should accept generated runtime binaries\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_package_without_native_windows_mcp_dispatcher()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    select_candidate_platforms(&plugin_root)?;
    for runtime_name in packaged_runtime_names() {
        write_runtime_fixture(
            &plugin_root,
            runtime_name,
            &runtime_binary_fixture(runtime_name),
        )?;
    }
    std::fs::remove_file(plugin_root.join("mcp/codexy-mcp-devtools.exe"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("native Windows MCP dispatcher missing"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_package_without_thin_legacy_windows_delegate()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    select_candidate_platforms(&plugin_root)?;
    for runtime_name in packaged_runtime_names() {
        write_runtime_fixture(
            &plugin_root,
            runtime_name,
            &runtime_binary_fixture(runtime_name),
        )?;
    }
    std::fs::remove_file(plugin_root.join("mcp/codexy-mcp-lsp.cmd"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("thin Windows MCP delegate missing"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_native_windows_legacy_delegate()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    select_candidate_platforms(&plugin_root)?;
    for runtime_name in packaged_runtime_names() {
        write_runtime_fixture(
            &plugin_root,
            runtime_name,
            &runtime_binary_fixture(runtime_name),
        )?;
    }
    std::fs::write(
        plugin_root.join("mcp/codexy-mcp-lsp.cmd"),
        runtime_binary_fixture("codexy-mcp-lsp-windows-x86_64.exe"),
    )?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-runtime-artifacts",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("must be the exact thin Windows MCP delegate for lsp"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_duplicate_server_runtime_bytes() -> Result<(), Box<dyn std::error::Error>> {
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        let temp = tempfile::tempdir()?;
        let plugin_root = copy_plugin_to(temp.path())?;
        select_candidate_platforms(&plugin_root)?;
        for runtime_name in packaged_runtime_names() {
            let fixture = if runtime_name.contains(platform) && runtime_name.contains("codegraph") {
                runtime_binary_fixture(&runtime_name.replace("codegraph", "lsp"))
            } else {
                runtime_binary_fixture(runtime_name)
            };
            write_runtime_fixture(&plugin_root, runtime_name, &fixture)?;
        }

        let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
            .args([
                "--plugin-root",
                plugin_root.to_str().ok_or("plugin root path")?,
                "--check-runtime-artifacts",
            ])
            .output()?;

        assert!(!output.status.success(), "{platform} duplicate unexpectedly passed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("must not contain duplicate native MCP runtime bytes") && stderr.contains(platform),
            "unexpected stderr for {platform}: {stderr}",
        );
    }
    Ok(())
}

fn select_candidate_platforms(plugin_root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    manifest["supportedPlatforms"] = serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    Ok(())
}
