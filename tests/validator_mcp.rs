#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn validator_cli_rejects_mixed_type_string_arrays() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let mcp_path = plugin_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path)?)?;
    mcp_config["lsp"]["args"] = serde_json::json!(["run", 7, "--quiet"]);
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_config)?)?;

    let output = validator(&plugin_root, "--check-mcp")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("args must be an array of strings"));
    Ok(())
}

#[test]
fn validator_cli_rejects_mcp_entrypoints_outside_plugin_root()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    std::fs::write(temp.path().join("outside.txt"), "outside\n")?;
    let mcp_path = plugin_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path)?)?;
    mcp_config["lsp"]["command"] = serde_json::json!("sh");
    mcp_config["lsp"]["args"] = serde_json::json!(["./../outside.txt"]);
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_config)?)?;

    let output = validator(&plugin_root, "--check-mcp")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("must stay inside the plugin root"));
    Ok(())
}

#[test]
fn validator_cli_rejects_script_runtime_mcp_entrypoints() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let script_name = ["server", &["j", "s"].join("")].join(".");
    std::fs::write(
        plugin_root.join("mcp").join(&script_name),
        "removed runtime\n",
    )?;
    let mcp_path = plugin_root.join(".mcp.json");
    let mut mcp_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&mcp_path)?)?;
    mcp_config["lsp"]["command"] = serde_json::json!(["no", "de"].join(""));
    mcp_config["lsp"]["args"] = serde_json::json!([format!("./mcp/{script_name}"), "--stdio"]);
    std::fs::write(&mcp_path, serde_json::to_string_pretty(&mcp_config)?)?;

    let output = validator(&plugin_root, "--check-mcp")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("must not use"));
    Ok(())
}

#[test]
fn validator_cli_accepts_installed_plugin_mcp_entrypoints() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let mcp_path = plugin_root.join(".mcp.json");
    let mcp_config: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(mcp_path)?)?;
    for server_name in ["lsp", "codegraph"] {
        let command = mcp_config[server_name]["command"]
            .as_str()
            .ok_or("MCP command must be a string")?;
        assert!(command.starts_with("./"));
        assert!(plugin_root.join(command).is_file());
        #[cfg(unix)]
        assert!(plugin_root.join(command).metadata()?.permissions().mode() & 0o111 != 0);
        assert!(
            !mcp_config[server_name]["args"]
                .as_array()
                .ok_or("MCP args must be an array")?
                .iter()
                .any(|arg| arg.as_str().is_some_and(|item| item.contains("../")))
        );
    }

    let output = validator(&plugin_root, "--check-mcp")?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

fn copy_fixture(plugin_root: &std::path::Path) -> std::io::Result<()> {
    copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn validator(
    plugin_root: &std::path::Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            mode,
        ])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
