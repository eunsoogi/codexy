use std::process::Command;

#[allow(unused)]
use crate::support;

#[test]
fn validator_accepts_empty_packaged_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .arg("--check-hooks")
        .output()?;
    assert!(output.status.success(), "{}", text(&output));
    Ok(())
}

#[test]
fn validator_rejects_missing_hooks_configuration() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::remove_file(root.join("hooks/hooks.json"))?;
    let output = validate(&root)?;
    assert!(!output.status.success());
    assert!(text(&output).contains("hooks/hooks.json"));
    Ok(())
}

#[test]
fn validator_rejects_unsafe_generic_commands() -> Result<(), Box<dyn std::error::Error>> {
    for command in [
        "./hooks/codexy-issue-title-check.sh --issue-title Valid",
        "\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\"; touch /tmp/pwned",
    ] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        set_command(&root, command)?;
        assert!(
            !validate(&root)?.status.success(),
            "validator accepted {command}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_non_boolean_generic_hook_async() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let path = root.join("hooks/hooks.json");
    let mut hooks = read(&path)?;
    hooks["hooks"]["PostToolUse"][0]["hooks"][0]["async"] = serde_json::json!("false");
    std::fs::write(path, serde_json::to_string_pretty(&hooks)?)?;
    let output = validate(&root)?;
    assert!(!output.status.success());
    assert!(text(&output).contains("hook async must be a boolean"));
    Ok(())
}

fn copy(base: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let root = base.join("codexy");
    support::copy_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &root,
    )?;
    set_command(
        &root,
        "\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\" --issue-title Valid",
    )?;
    Ok(root)
}
fn set_command(root: &std::path::Path, command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join("hooks/hooks.json");
    let mut hooks = read(&path)?;
    hooks["hooks"]["PostToolUse"] =
        serde_json::json!([{"hooks":[{"type":"command","command":command,"timeout":3}]}]);
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
fn text(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
