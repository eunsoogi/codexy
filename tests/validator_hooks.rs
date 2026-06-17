#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[test]
fn validator_cli_checks_hook_contract_surface() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--check-hooks")
        .output()?;
    assert!(
        output.status.success(),
        "validator --check-hooks failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_installed_plugin_hook_entrypoints()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;

    let hooks_path = plugin_root.join("hooks/hooks.json");
    let hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    let command = hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["command"]
        .as_str()
        .ok_or("SessionStart hook command must be a string")?;
    assert!(
        command.contains("${PLUGIN_ROOT}/hooks/codexy-routing-context.sh"),
        "hook command must resolve through PLUGIN_ROOT for installed packages"
    );
    assert!(
        !command.contains("PLUGIN_DATA") && !command.contains("~/.codex"),
        "hook command must not reference writable plugin data or user state"
    );
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    assert!(
        script_path.is_file(),
        "hook command target must exist inside the installed plugin"
    );
    #[cfg(unix)]
    assert!(
        script_path.metadata()?.permissions().mode() & 0o111 != 0,
        "hook command target must be executable inside the installed plugin"
    );

    let hook_output = Command::new(&script_path).arg("SessionStart").output()?;
    assert!(
        hook_output.status.success(),
        "hook script should emit context successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&hook_output.stdout),
        String::from_utf8_lossy(&hook_output.stderr)
    );
    let hook_json: serde_json::Value = serde_json::from_slice(&hook_output.stdout)?;
    assert_eq!(
        hook_json["hookSpecificOutput"]["hookEventName"],
        "SessionStart"
    );
    assert!(
        hook_json["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .is_some_and(|context| context.contains("$codex-orchestration")),
        "hook output should surface lightweight Codexy routing context"
    );

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;
    assert!(
        output.status.success(),
        "validator should accept installed plugin-local hook entrypoints\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_plugin_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    std::fs::remove_file(plugin_root.join("hooks/hooks.json"))?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "validator should reject missing plugin hook config"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hooks/hooks.json"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_hooks_without_plugin_root_command()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["command"] =
        serde_json::json!("./hooks/codexy-routing-context.sh SessionStart");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "validator should reject hook commands that do not resolve through PLUGIN_ROOT"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("must reference a packaged ${PLUGIN_ROOT} path"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_hook_user_state_mutation() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let script_path = plugin_root.join("hooks/codexy-routing-context.sh");
    let mut script = std::fs::read_to_string(&script_path)?;
    script.push_str("\ntouch ~/.codex/codexy-hook-state\n");
    std::fs::write(&script_path, script)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "validator should reject hook scripts that mutate user Codex state"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hook script must not contain"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_non_boolean_hook_async() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;
    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"][0]["async"] = serde_json::json!("false");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-hooks",
        ])
        .output()?;
    assert!(
        !output.status.success(),
        "validator should reject non-boolean hook async values"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hook async must be a boolean"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn copy_plugin(plugin_root: &std::path::Path) -> std::io::Result<()> {
    copy_dir(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        plugin_root,
    )
}

fn copy_dir(source: &std::path::Path, target: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}
