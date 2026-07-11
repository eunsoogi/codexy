mod support;

use support::{TestResult, copy_plugin_fixture, stderr, validator};

#[test]
fn validator_cli_rejects_missing_child_thread_ledger_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    assert!(skill.contains("Active child Codex app threads MUST be capped"));
    assert!(skill.contains("blocker, latest evidence, and next action"));
    std::fs::write(
        &skill_path,
        skill
            .replace(
                "Active child Codex app threads MUST be capped",
                "Active child Codex app threads have a bounded concurrency limit",
            )
            .replace(
                "blocker, latest evidence, and next action",
                "blocker, and next action",
            ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("active child codex app threads must be capped at 5"));
    assert!(stderr.contains("latest evidence"));
    Ok(())
}

#[test]
fn validator_cli_rejects_specialist_subagent_cap_exception() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    assert!(skill.contains("Packaged specialist subagents MUST NOT be counted"));
    std::fs::write(
        &skill_path,
        skill.replace(
            "Packaged specialist subagents MUST NOT be counted as active\nchild Codex app threads.",
            "Packaged specialist subagents MUST NOT be counted unless existing code explicitly treats them as Codex app child threads.",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("packaged specialist subagents must not be counted unless"));
    Ok(())
}

#[test]
fn validator_cli_rejects_missing_dreaming_worktree_reservation_fields() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let dreaming_path = plugin_root.join("skills/dreaming/SKILL.md");
    let dreaming = std::fs::read_to_string(&dreaming_path)?;
    std::fs::write(
        &dreaming_path,
        dreaming
            .replace("canonical\nworktree CWD", "worktree location")
            .replace("MUST NOT recycle the worktree", "may recycle the worktree"),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("canonical worktree cwd"));
    assert!(stderr.contains("must not recycle the worktree"));
    Ok(())
}
