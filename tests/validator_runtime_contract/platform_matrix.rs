use super::*;

#[test]
fn validator_cli_rejects_supported_platform_without_bundled_mcp_runtimes()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] =
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    add_windows_runtime_release(&plugin_root)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject advertised platforms without bundled runtimes"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("bundled platforms for lsp must match"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_platform_outside_immutable_runtime_inventory()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] =
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    add_windows_runtime_release(&plugin_root)?;
    for server in ["lsp", "codegraph"] {
        let wrapper_path = plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&wrapper_path)?.replace(
            "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
            "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        );
        std::fs::write(&wrapper_path, wrapper)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject platforms outside the immutable runtime inventory"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("immutable runtime package must retain platforms"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn add_windows_runtime_release(plugin_root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let path = plugin_root.join("runtime-release.json");
    let mut release: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    release["state"] = serde_json::json!("candidate-proven");
    release["artifact"]["tag"] = serde_json::json!("runtime-candidate-windows-proof");
    release["artifact"]["url"] = serde_json::json!("https://github.com/eunsoogi/codexy/releases/download/runtime-candidate-windows-proof/codexy-marketplace-plugin.tar.gz");
    release["platforms"]["windows-x86_64"] = serde_json::json!({
        "lsp": { "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
        "codegraph": { "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb" }
    });
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        for server in ["lsp", "codegraph"] {
            release["platforms"][platform][server]["path"] =
                serde_json::json!(format!("runtime/codexy-mcp-{server}-{platform}.bin"));
        }
    }
    std::fs::write(&path, serde_json::to_string_pretty(&release)?)?;
    let candidate = serde_json::json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": release["source"].clone(),
        "artifact": { "tag": release["artifact"]["tag"].clone() },
        "compatibility": release["compatibility"].clone(),
        "platforms": release["platforms"].clone(),
    });
    std::fs::write(
        plugin_root.join("runtime-candidate.json"),
        serde_json::to_string(&candidate)?,
    )?;
    Ok(())
}

#[test]
fn validator_cli_rejects_platform_narrowing_without_required_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] = serde_json::json!(["darwin-arm64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    for server in ["lsp", "codegraph"] {
        let wrapper_path = plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        let wrapper = std::fs::read_to_string(&wrapper_path)?.replace(
            "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
            "bundled_platforms=\"darwin-arm64\"",
        );
        std::fs::write(&wrapper_path, wrapper)?;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(
        !output.status.success(),
        "validator should reject narrowing below the baseline supported platforms"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("supportedPlatforms must include linux-x86_64"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
