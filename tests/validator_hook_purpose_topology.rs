#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;
use std::process::Command;

#[allow(unused)]
use crate::support;

#[test]
fn validator_cli_accepts_installed_readiness_hook_topology()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;

    let hooks_path = plugin_root.join("hooks/hooks.json");
    let hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    let readiness_command = hooks_config["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"]
        .as_str()
        .ok_or("UserPromptSubmit hook command must be a string")?;
    assert!(
        readiness_command.contains("${PLUGIN_ROOT}/hooks/codexy-readiness-context.sh"),
        "readiness hook command must resolve through PLUGIN_ROOT for installed packages"
    );
    let readiness_script_path = plugin_root.join("hooks/codexy-readiness-context.sh");
    assert!(
        readiness_script_path.is_file(),
        "readiness hook command target must exist inside the installed plugin"
    );
    #[cfg(unix)]
    assert!(
        readiness_script_path.metadata()?.permissions().mode() & 0o111 != 0,
        "readiness hook command target must be executable inside the installed plugin"
    );

    let readiness_output = Command::new(&readiness_script_path)
        .arg("UserPromptSubmit")
        .output()?;
    assert!(
        readiness_output.status.success(),
        "readiness hook script should emit context successfully\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&readiness_output.stdout),
        String::from_utf8_lossy(&readiness_output.stderr)
    );
    let readiness_json: serde_json::Value = serde_json::from_slice(&readiness_output.stdout)?;
    assert_eq!(
        readiness_json["hookSpecificOutput"]["hookEventName"],
        "UserPromptSubmit"
    );
    let readiness_context = readiness_json["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("readiness hook output should include additional context")?;
    assert!(readiness_context.contains("codexy-issue-title-check.sh --issue-title"));
    assert!(readiness_context.contains("--check-issue-title"));
    assert!(readiness_context.contains("PR title and merge subject enforcement (#206)"));
    assert!(readiness_context.contains("PR label readiness enforcement (#210)"));
    assert!(readiness_context.contains("--check-completion-handoff"));
    assert!(readiness_context.contains("repositoryLabels"));
    assert!(readiness_context.contains("target base"));
    assert!(readiness_context.contains("hook entrypoints"));
    assert!(readiness_context.contains("available fallback"));
    assert!(readiness_context.contains("separate dogfood defect"));

    let output = validate_hooks(&plugin_root)?;
    assert!(
        output.status.success(),
        "validator should accept installed split-purpose hook topology\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_purpose_specific_hooks_alongside_routing_context()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;

    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["SessionStart"][0]["hooks"]
        .as_array_mut()
        .ok_or("SessionStart hooks must be an array")?
        .push(serde_json::json!({
            "type": "command",
            "command": "\"${PLUGIN_ROOT}/hooks/codexy-readiness-context.sh\" SessionStart",
            "timeout": 3,
            "statusMessage": "Loading Codexy readiness context"
        }));
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        output.status.success(),
        "validator should accept purpose-specific hooks alongside the routing context\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_readiness_hook_topology() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;

    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]
        .as_object_mut()
        .ok_or("hooks must be an object")?
        .remove("UserPromptSubmit");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject collapsed routing-only hook topology"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("UserPromptSubmit hook command must run hooks/codexy-readiness-context.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_readiness_hook_collapsed_to_routing_script()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;

    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"] =
        serde_json::json!("\"${PLUGIN_ROOT}/hooks/codexy-routing-context.sh\" SessionStart");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject readiness hooks collapsed to the routing script"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("UserPromptSubmit hook command must run hooks/codexy-readiness-context.sh"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unsafe_readiness_hook_command() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_plugin(&plugin_root)?;

    let hooks_path = plugin_root.join("hooks/hooks.json");
    let mut hooks_config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&hooks_path)?)?;
    hooks_config["hooks"]["UserPromptSubmit"][0]["hooks"][0]["command"] =
        serde_json::json!("./hooks/codexy-readiness-context.sh UserPromptSubmit");
    std::fs::write(&hooks_path, serde_json::to_string_pretty(&hooks_config)?)?;

    let output = validate_hooks(&plugin_root)?;
    assert!(
        !output.status.success(),
        "validator should reject readiness hook commands outside PLUGIN_ROOT"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("must start with a packaged ${PLUGIN_ROOT} entrypoint"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
