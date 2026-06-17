use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn validator_cli_rejects_empty_nickname_entries() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/codexy-pathfinder.toml");
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
    let planner_path = plugin_root.join("agents/codexy-pathfinder.toml");
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
    let planner_path = plugin_root.join("agents/codexy-pathfinder.toml");
    let planner = std::fs::read_to_string(&planner_path)?;
    let planner = planner.replace("developer_instructions = \"\"\"\n", "removed = \"\"\"\n");
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("developer_instructions must be a non-empty string"));
    Ok(())
}

#[test]
fn validator_cli_allows_supported_custom_agent_config_layers()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/codexy-pathfinder.toml");
    let mut planner = std::fs::read_to_string(&planner_path)?;
    planner.push_str(
        "\nmcp_servers = [\"grep_app\"]\n\n[[skills.config]]\npath = \"/tmp/codexy-qa/SKILL.md\"\nenabled = false\n",
    );
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_rejects_table_shaped_skills_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/codexy-pathfinder.toml");
    let mut planner = std::fs::read_to_string(&planner_path)?;
    planner.push_str("\n[skills.config]\n\"codexy:qa\" = { enabled = true }\n");
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("skills.config must be an array"));
    Ok(())
}

#[test]
fn validator_cli_rejects_unsupported_skills_config_fields() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/codexy-pathfinder.toml");
    let mut planner = std::fs::read_to_string(&planner_path)?;
    planner.push_str("\n[[skills.config]]\nname = \"codexy:qa\"\n");
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("skills.config.name is not part"));
    Ok(())
}

#[test]
fn validator_cli_rejects_unsupported_skills_config_layers() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let planner_path = plugin_root.join("agents/codexy-pathfinder.toml");
    let mut planner = std::fs::read_to_string(&planner_path)?;
    planner.push_str("\n[skills.unsupported]\nfoo = true\n");
    std::fs::write(&planner_path, planner)?;

    let output = validator(&plugin_root)?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("skills.unsupported is not part"));
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
