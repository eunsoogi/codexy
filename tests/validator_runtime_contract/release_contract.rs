use super::*;
use std::io::Write as _;

#[test]
fn validator_rejects_missing_runtime_release_contract() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    declare_bundled_platforms(&plugin_root)?;
    std::fs::remove_file(plugin_root.join("runtime-release.json"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("runtime-release.json"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_runtime_release_unknown_fields() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = copy_plugin_to(temp.path())?;
    declare_bundled_platforms(&plugin_root)?;
    let path = plugin_root.join("runtime-release.json");
    let mut contract: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    contract["untrusted"] = serde_json::json!(true);
    std::fs::write(&path, serde_json::to_string_pretty(&contract)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("untrusted"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn declare_bundled_platforms(plugin_root: &std::path::Path) -> std::io::Result<()> {
    for server in ["lsp", "codegraph"] {
        let path = plugin_root.join(format!("mcp/codexy-mcp-{server}"));
        std::fs::OpenOptions::new()
            .append(true)
            .open(path)?
            .write_all(b"\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\n")?;
    }
    Ok(())
}
