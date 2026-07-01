use std::process::Command;

mod support;

use support::copy_dir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_manifest_prompt_without_orchestration_route() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    copy_fixture(&plugin_root)?;
    let manifest_path = plugin_root.join(".codex-plugin/plugin.json");
    let text = std::fs::read_to_string(&manifest_path)?;
    let mut manifest: serde_json::Value = serde_json::from_str(&text)?;
    assert!(
        manifest["interface"]["defaultPrompt"]
            .as_array()
            .ok_or("defaultPrompt")?
            .iter()
            .all(|line| line.as_str().is_some_and(|line| line.contains("MUST")))
    );
    manifest["interface"]["defaultPrompt"] = serde_json::json!(["Use Codexy as orchestrator."]);
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("interface.defaultPrompt must route through"));
    Ok(())
}

#[test]
fn validator_cli_rejects_top_level_prompt_without_orchestration_route() -> TestResult {
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
fn validator_cli_rejects_missing_top_level_prompt_metadata() -> TestResult {
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
fn validator_cli_allows_skill_prompt_without_orchestration_route() -> TestResult {
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
fn codex_orchestration_spawn_agent_examples_use_message_argument() -> TestResult {
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
fn repo_instructions_own_dogfood_policy_with_orchestration_details() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let agents = std::fs::read_to_string(root.join("AGENTS.md"))?;
    let skill =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;

    assert!(agents.contains("Dogfooding Guardrails"));
    assert!(agents.contains("failures to follow governing `AGENTS.md`"));
    assert!(agents.contains("actual Codex callable tool surface or `tool_search`"));
    assert!(agents.contains("`codex mcp list` shows Codexy `codegraph` or `lsp` enabled"));
    assert!(agents.contains("preflight branch refs"));
    assert!(agents.contains("non-existent new branch as an existing branch selector"));
    assert!(agents.contains("MUST keep exactly one"));
    assert!(agents.contains("MUST NOT stop at an open PR"));
    assert!(
        agents.contains("Child-owned lanes receive implementation and review-feedback patches")
    );

    assert!(skill.contains("Root `AGENTS.md` owns repo-wide dogfooding policy"));
    assert!(skill.contains("Parent Stop Preflight"));
    assert!(skill.contains("references/thread-and-worktree-routing.md"));
    assert!(skill.contains("Subagents are not child-owned implementation owners"));
    assert!(skill.contains("--check-child-lane-ownership --evidence-file"));
    assert!(!skill.contains("## Registered MCP Exposure Defects"));

    let thread_ref = std::fs::read_to_string(root.join(
        "plugins/codexy/skills/codex-orchestration/references/thread-and-worktree-routing.md",
    ))?;
    for expected in [
        "Thread Tool Discovery Procedure",
        "Codex App Worktree Creation Preflight",
        "thread/start",
        "turn/start",
        "tool_search` mismatch is an exposure/discovery defect",
        "exact missing-handler error",
        "no fallback route was\n   available",
        "MUST NOT use `codex exec`, `codex fork`, or `codex app-server`",
        "startingState.type=\"branch\"",
        "git check-ref-format --branch",
    ] {
        assert!(thread_ref.contains(expected));
    }
    Ok(())
}

#[test]
fn git_workflow_requires_child_lane_ownership_evidence_check() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/git-workflow/SKILL.md"))?;

    assert!(skill.contains("--check-child-lane-ownership --evidence-file"));
    assert!(skill.contains("parent MUST NOT patch the child-owned"));
    assert!(skill.contains("explicit maintainer reassignment"));
    Ok(())
}

#[test]
fn codexy_workflows_require_task_classification_first() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let classification =
        std::fs::read_to_string(root.join("plugins/codexy/skills/task-classification/SKILL.md"))?;
    let orchestration =
        std::fs::read_to_string(root.join("plugins/codexy/skills/codex-orchestration/SKILL.md"))?;
    let git_workflow =
        std::fs::read_to_string(root.join("plugins/codexy/skills/git-workflow/SKILL.md"))?;
    let qa_prompt =
        std::fs::read_to_string(root.join("plugins/codexy/skills/qa/agents/openai.yaml"))?;
    let release_prompt = std::fs::read_to_string(
        root.join("plugins/codexy/skills/release-engineering/agents/openai.yaml"),
    )?;

    assert!(classification.contains("name: task-classification"));
    assert!(classification.contains("MUST run this skill first for any Codexy work"));
    assert!(classification.contains("Classification Output"));
    assert!(classification.contains("Lane type:"));
    assert!(classification.contains("Owner decision:"));
    assert!(classification.contains("Required skills:"));
    assert!(classification.contains("Required tools/evidence:"));
    assert!(classification.contains("lane-relevant required evidence"));
    assert!(classification.contains("unavailable-tool fallbacks"));
    assert!(classification.contains("First allowed action:"));

    for lane_type in [
        "orchestration/lane setup",
        "implementation",
        "review response",
        "GitHub/merge",
        "validation/QA",
        "documentation/skill authoring",
        "plugin/release",
    ] {
        assert!(classification.contains(lane_type));
    }

    assert!(classification.contains(
        "Missing classification evidence blocks branch/worktree setup, delegation,\n  validation/QA"
    ));
    assert!(
        classification
            .contains("Classification MUST happen before acting on or using the owner decision")
    );
    assert!(orchestration.contains("$task-classification"));
    assert!(orchestration.contains(
        "Missing classification before\nsetup, validation, release, or other workflow actions"
    ));
    assert!(git_workflow.contains("$task-classification"));
    assert!(git_workflow.contains("classification evidence"));
    assert!(qa_prompt.contains("$task-classification"));
    assert!(release_prompt.contains("$task-classification"));
    Ok(())
}

#[test]
fn validator_cli_rejects_tab_indented_prompt_yaml() -> TestResult {
    assert_prompt_indent_rejected("  display_name:", "\tdisplay_name:")
}

#[test]
fn validator_cli_rejects_mixed_space_tab_prompt_yaml() -> TestResult {
    assert_prompt_indent_rejected("  display_name:", " \tdisplay_name:")
}

fn assert_prompt_indent_rejected(needle: &str, replacement: &str) -> TestResult {
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
