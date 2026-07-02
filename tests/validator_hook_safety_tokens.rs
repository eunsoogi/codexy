use std::process::Command;

#[allow(unused)]
mod support;

#[test]
fn validator_cli_rejects_node_command_with_tab_separator() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["PostToolUse"] = serde_json::json!([{ "hooks": [{
        "type": "command",
        "command": "\"${PLUGIN_ROOT}/hooks/codexy-routing-context.sh\" node\ttool.js",
        "timeout": 5
    }]}]);
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject node token"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not reference \"node\""),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_nodejs_command_alias() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["PostToolUse"] = serde_json::json!([{ "hooks": [{
        "type": "command",
        "command": "\"${PLUGIN_ROOT}/hooks/codexy-routing-context.sh\" nodejs helper.js",
        "timeout": 5
    }]}]);
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject nodejs command alias"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not reference \"nodejs\""),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_node_script_with_tab_separator() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    std::fs::write(
        plugin_root.join("hooks/codexy-routing-context.sh"),
        "#!/bin/sh\nnode\ttool.js\n",
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject node token"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not run \"node\""),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_nodejs_script_alias() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    std::fs::write(
        plugin_root.join("hooks/codexy-routing-context.sh"),
        "#!/bin/sh\nnodejs helper.js\n",
    )?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject nodejs script alias"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("must not run \"nodejs\""),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_node_script_behind_shell_wrappers()
-> Result<(), Box<dyn std::error::Error>> {
    for line in ["exec node helper.js", "env node helper.js"] {
        let temp = tempfile::tempdir()?;
        let plugin_root = temp.path().join("codexy");
        copy_plugin(&plugin_root)?;
        std::fs::write(
            plugin_root.join("hooks/codexy-routing-context.sh"),
            format!("#!/bin/sh\n{line}\n"),
        )?;

        let output = validate_hooks(&plugin_root)?;
        assert!(
            !output.status.success(),
            "validator should reject wrapped node token in {line}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("must not run \"node\""),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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
