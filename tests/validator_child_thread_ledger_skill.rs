use crate::support;

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
fn validator_cli_rejects_missing_material_child_event_consumption_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    for required in [
        "Material child event",
        "actionable review feedback",
        "replacement-owner availability",
        "validate the stable event identity",
        "consume it in the same turn",
        "perform the authorized parent-owned next action",
        "record a concrete execution blocker",
        "acknowledgement-only output MUST NOT satisfy consumption",
        "Duplicate stable event identities MUST remain deduplicated with no parent action",
        "unchanged continuation observations MUST NOT create assistant turns",
    ] {
        assert!(skill.contains(required), "missing {required:?}");
    }
    std::fs::write(
        &skill_path,
        skill.replace(
            "When a Material child event arrives—terminal child state, actionable review feedback, or replacement-owner availability—the parent MUST validate the stable event identity and consume it in the same turn. To consume the event, the parent MUST perform the authorized parent-owned next action, such as route actionable review feedback, start a replacement owner, or resolve a verified gate, or MUST record a concrete execution blocker. An acknowledgement-only output MUST NOT satisfy consumption. Duplicate stable event identities MUST remain deduplicated with no parent action, and unchanged continuation observations MUST NOT create assistant turns.",
            "Child events may be summarized for a later turn.",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    for required in [
        "material child event",
        "replacement-owner availability",
        "consume it in the same turn",
        "acknowledgement-only output must not satisfy consumption",
        "duplicate stable event identities must remain deduplicated with no parent action",
    ] {
        assert!(
            stderr.contains(required),
            "missing {required:?} in {stderr}"
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_each_unconsumed_material_child_event() -> TestResult {
    for (required, replacement) in [
        (
            "route actionable review feedback",
            "summarize actionable review feedback",
        ),
        ("start a replacement owner", "mention a replacement owner"),
        (
            "acknowledgement-only output MUST NOT satisfy consumption",
            "acknowledgement-only output is sufficient",
        ),
        (
            "Duplicate stable event identities MUST remain deduplicated with no parent action",
            "Duplicate stable event identities may be handled again",
        ),
    ] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        assert!(skill.contains(required), "missing {required:?}");
        std::fs::write(&skill_path, skill.replace(required, replacement))?;

        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "accepted {required:?}");
        assert!(
            stderr(&output).contains(&required.to_ascii_lowercase()),
            "missing {required:?} in {}",
            stderr(&output)
        );
    }
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
