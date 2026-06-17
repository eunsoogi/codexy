use std::process::Command;

mod support;

use support::copy_dir;

#[test]
fn validator_cli_rejects_manifest_prompt_without_orchestration_route()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let mut manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    manifest["interface"]["defaultPrompt"] = serde_json::json!(["Use Codexy as orchestrator."]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let output = validator(&plugin_root, "--check")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("interface.defaultPrompt must route through"));
    Ok(())
}

#[test]
fn validator_cli_rejects_top_level_prompt_without_orchestration_route()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let prompt = std::fs::read_to_string(&prompt_path)?;
    std::fs::write(
        &prompt_path,
        prompt.replace("$codex-orchestration", "Codexy orchestration"),
    )?;

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("interface.default_prompt must route through"));
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_top_level_prompt_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    std::fs::remove_file(plugin_root.join("agents/openai.yaml"))?;

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("agents/openai.yaml is required"));
    Ok(())
}

#[test]
fn validator_cli_allows_skill_prompt_without_orchestration_route()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let prompt_path = plugin_root.join("skills/git-workflow/agents/openai.yaml");
    assert!(!std::fs::read_to_string(prompt_path)?.contains("$codex-orchestration"));

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(output.status.success(), "stderr:\n{}", stderr(&output));
    Ok(())
}

#[test]
fn codex_orchestration_spawn_agent_examples_use_message_argument()
-> Result<(), Box<dyn std::error::Error>> {
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;
    assert!(!skill.contains("prompt="));
    assert!(skill.contains("spawn_agent(agent_type=\"codexy-sentinel\", message="));
    assert!(skill.contains("spawn_agent(agent_type=\"codexy-pathfinder\", message="));
    assert!(skill.contains("spawn_agent(agent_type=\"codexy-cartographer\", message="));
    Ok(())
}

#[test]
fn repo_instructions_own_dogfood_policy_with_orchestration_details()
-> Result<(), Box<dyn std::error::Error>> {
    let agents = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("AGENTS.md"),
    )?;
    let skill = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins/codexy/skills/codex-orchestration/SKILL.md"),
    )?;

    assert!(agents.contains("Dogfooding Guardrails"));
    assert!(agents.contains("failures to follow governing `AGENTS.md`"));
    assert!(agents.contains("actual Codex callable tool surface or `tool_search`"));
    assert!(agents.contains("`codex mcp list` shows Codexy `codegraph` or `lsp` enabled"));
    assert!(agents.contains("preflight branch refs"));
    assert!(agents.contains("non-existent new branch as an existing branch selector"));
    assert!(agents.contains("keep exactly one active"));
    assert!(agents.contains("must not stop at an open PR"));
    assert!(
        agents.contains("Child-owned lanes receive implementation and review-feedback patches")
    );

    assert!(skill.contains("Root `AGENTS.md` owns repo-wide dogfooding policy"));
    assert!(skill.contains("Parent Stop Preflight"));
    assert!(skill.contains("Codex App Worktree Creation Preflight"));
    assert!(skill.contains("startingState.type=\"branch\""));
    assert!(skill.contains("git check-ref-format --branch"));
    assert!(!skill.contains("## Registered MCP Exposure Defects"));
    Ok(())
}

#[test]
fn validator_cli_rejects_tab_indented_prompt_yaml() -> Result<(), Box<dyn std::error::Error>> {
    assert_prompt_indent_rejected("  display_name:", "\tdisplay_name:")
}

#[test]
fn validator_cli_rejects_mixed_space_tab_prompt_yaml() -> Result<(), Box<dyn std::error::Error>> {
    assert_prompt_indent_rejected("  display_name:", " \tdisplay_name:")
}

fn assert_prompt_indent_rejected(
    needle: &str,
    replacement: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let prompt_path = plugin_root.join("agents/openai.yaml");
    let prompt = std::fs::read_to_string(&prompt_path)?;
    std::fs::write(&prompt_path, prompt.replace(needle, replacement))?;

    let output = validator(&plugin_root, "--check-roles")?;

    assert!(!output.status.success());
    assert!(stderr(&output).contains("must not contain tab indentation"));
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
