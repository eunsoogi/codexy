use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn validator_cli_rejects_quoted_hook_entrypoint_shell_syntax()
-> Result<(), Box<dyn std::error::Error>> {
    for malicious_name in [
        "$(touch pwned; printf codexy-routing-context.sh)",
        "`touch pwned`codexy-routing-context.sh",
    ] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        std::fs::copy(
            plugin_root.join("hooks/codexy-routing-context.sh"),
            plugin_root.join("hooks").join(malicious_name),
        )?;
        set_session_start_hook_command(
            &plugin_root,
            &format!("\"${{PLUGIN_ROOT}}/hooks/{malicious_name}\" SessionStart"),
        )?;

        let output = validate_hooks(&plugin_root)?;
        assert!(
            !output.status.success(),
            "validator should reject quoted hook entrypoints with shell syntax: {malicious_name}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("entrypoint paths must not contain shell syntax"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_command_hooks_without_timeout() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]
        .as_object_mut()
        .ok_or("SessionStart hook must be an object")?
        .remove("timeout");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject command hooks without explicit timeouts"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hook timeout is required"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn set_session_start_hook_command(
    plugin_root: &std::path::Path,
    command: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["command"] = serde_json::json!(command);
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;
    Ok(())
}

fn validate_hooks(
    plugin_root: &std::path::Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-hooks",
        ])
        .output()?)
}

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    support::copy_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}
