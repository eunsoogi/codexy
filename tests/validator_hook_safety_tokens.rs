#[allow(unused)]
use crate::support;
use std::process::Command;

#[test]
fn validator_rejects_node_tokens_in_generic_hook_commands_and_scripts()
-> Result<(), Box<dyn std::error::Error>> {
    for mutation in ["command", "script"] {
        let temp = tempfile::tempdir()?;
        let root = fixture(temp.path())?;
        if mutation == "command" {
            set_command(
                &root,
                "\"${PLUGIN_ROOT}/hooks/codexy-issue-title-check.sh\" node\ttool.js",
            )?;
        } else {
            std::fs::write(
                root.join("hooks/codexy-issue-title-check.sh"),
                "#!/bin/sh\nnodejs helper.js\n",
            )?;
        }
        let output = validate(&root)?;
        assert!(!output.status.success(), "validator accepted {mutation}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("node"));
    }
    Ok(())
}
fn fixture(base: &std::path::Path) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
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
    let mut hooks: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    hooks["hooks"]["PostToolUse"] =
        serde_json::json!([{"hooks":[{"type":"command","command":command,"timeout":3}]}]);
    std::fs::write(path, serde_json::to_string_pretty(&hooks)?)?;
    Ok(())
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
