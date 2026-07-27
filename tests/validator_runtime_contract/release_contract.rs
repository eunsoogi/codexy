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

#[test]
fn validator_accepts_only_safe_candidate_release_tags() -> Result<(), Box<dyn std::error::Error>> {
    let valid = tempfile::tempdir()?;
    let valid_root = copy_plugin_to(valid.path())?;
    declare_bundled_platforms(&valid_root)?;
    write_candidate_release(&valid_root, "runtime-candidate-1.3.0")?;
    let output = validate(&valid_root)?;
    assert!(
        output.status.success(),
        "valid candidate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    for tag in ["runtime-candidate-", "runtime-candidate-bad/tag", "v1.3.0"] {
        let temp = tempfile::tempdir()?;
        let plugin_root = copy_plugin_to(temp.path())?;
        declare_bundled_platforms(&plugin_root)?;
        write_candidate_release(&plugin_root, tag)?;
        assert!(!validate(&plugin_root)?.status.success(), "unsafe tag accepted: {tag}");
    }
    Ok(())
}

fn write_candidate_release(plugin_root: &std::path::Path, tag: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = plugin_root.join("runtime-release.json");
    let mut release: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    release["state"] = serde_json::json!("candidate-proven");
    release["artifact"]["tag"] = serde_json::json!(tag);
    release["artifact"]["url"] = serde_json::json!(format!("https://github.com/eunsoogi/codexy/releases/download/{tag}/codexy-marketplace-plugin.tar.gz"));
    for platform in ["darwin-arm64", "linux-x86_64"] {
        for server in ["lsp", "codegraph"] {
            release["platforms"][platform][server]["path"] = serde_json::json!(format!("runtime/codexy-mcp-{server}-{platform}.bin"));
        }
    }
    std::fs::write(&path, serde_json::to_string_pretty(&release)?)?;
    let candidate = serde_json::json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": release["source"].clone(),
        "artifact": {"tag": tag},
        "compatibility": release["compatibility"].clone(),
        "platforms": release["platforms"].clone(),
    });
    std::fs::write(plugin_root.join("runtime-candidate.json"), serde_json::to_string(&candidate)?)?;
    Ok(())
}

fn validate(plugin_root: &std::path::Path) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--plugin-root")
        .arg(plugin_root)
        .arg("--check")
        .output()
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
