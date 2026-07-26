use std::process::Command;

#[allow(unused)]
use crate::support;

#[test]
fn validator_rejects_quoted_entrypoint_shell_syntax() -> Result<(), Box<dyn std::error::Error>> {
    for name in [
        "$(touch pwned; printf codexy-issue-title-check.sh)",
        "`touch pwned`check.sh",
    ] {
        let temp = tempfile::tempdir()?;
        let root = fixture(temp.path())?;
        std::fs::copy(
            root.join("hooks/codexy-issue-title-check.sh"),
            root.join("hooks").join(name),
        )?;
        set_command(
            &root,
            &format!("\"${{PLUGIN_ROOT}}/hooks/{name}\" PostToolUse"),
        )?;
        let output = validate(&root)?;
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("entrypoint paths must not contain shell syntax")
        );
    }
    Ok(())
}

#[test]
fn validator_requires_generic_hook_timeouts() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = fixture(temp.path())?;
    let path = root.join("hooks/hooks.json");
    let mut hooks = read(&path)?;
    hooks["hooks"]["PostToolUse"][0]["hooks"][0]
        .as_object_mut()
        .ok_or("handler")?
        .remove("timeout");
    std::fs::write(path, serde_json::to_string_pretty(&hooks)?)?;
    let output = validate(&root)?;
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("hook timeout must be a positive integer")
    );
    Ok(())
}

fn fixture(base: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = base.join("codexy");
    support::copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &root,
    )?;
    let path = root.join("hooks/hooks.json");
    let mut hooks = read(&path)?;
    hooks["hooks"]["PostToolUse"] = serde_json::json!([{"hooks":[{"type":"command","command":"\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\" --issue-title Valid","timeout":3}]}]);
    std::fs::write(path, serde_json::to_string_pretty(&hooks)?)?;
    Ok(root)
}

fn set_command(root: &std::path::Path, command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join("hooks/hooks.json");
    let mut hooks = read(&path)?;
    hooks["hooks"]["PostToolUse"][0]["hooks"][0]["command"] = serde_json::json!(command);
    std::fs::write(path, serde_json::to_string_pretty(&hooks)?)?;
    Ok(())
}
fn read(path: &std::path::Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}
fn validate(root: &std::path::Path) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            root.to_str().ok_or("root")?,
            "--check-hooks",
        ])
        .output()?)
}
