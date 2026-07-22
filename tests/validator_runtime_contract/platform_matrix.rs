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
    let wrapper_path = plugin_root.join("mcp/codexy-mcp-lsp");
    let wrapper = std::fs::read_to_string(&wrapper_path)?.replace(
        "bundled_platforms=\"darwin-arm64 linux-x86_64 windows-x86_64\"",
        "bundled_platforms=\"darwin-arm64 linux-x86_64\"",
    );
    std::fs::write(&wrapper_path, wrapper)?;

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
fn validator_cli_accepts_supported_platform_with_build_matrix_coverage()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] =
        serde_json::json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
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
        output.status.success(),
        "validator should accept advertised platforms with release matrix coverage: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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

#[test]
fn validator_cli_rejects_platform_narrowing_without_native_windows()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["supportedPlatforms"] = serde_json::json!(["darwin-arm64", "linux-x86_64"]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("supportedPlatforms must include windows-x86_64"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
