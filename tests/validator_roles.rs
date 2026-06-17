use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn validator_cli_rejects_empty_nickname_entries() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/planner.toml");
    let mut planner = std::fs::read_to_string(&planner_path)?;
    planner.push_str("\nnickname_candidates = [\"\", \"Plan\"]\n");
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("nickname_candidates must be a list of non-empty strings"));
    Ok(())
}

#[test]
fn validator_cli_rejects_non_custom_agent_fields() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/planner.toml");
    let mut planner = std::fs::read_to_string(&planner_path)?;
    planner.push_str("\ndisplay_name = \"Planner\"\n");
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("display_name is not part of the supported Codex custom-agent")
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_agent_missing_developer_instructions()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/planner.toml");
    let planner = std::fs::read_to_string(&planner_path)?;
    let planner = planner.replace("developer_instructions = \"\"\"\n", "removed = \"\"\"\n");
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("developer_instructions must be a non-empty string"));
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
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--plugin-root",
            plugin_root.to_str().ok_or("plugin root path")?,
            "--check-roles",
        ])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
