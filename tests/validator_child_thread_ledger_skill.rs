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
fn validator_cli_rejects_mutating_or_polling_sentinel_observation() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
    let skill = std::fs::read_to_string(&skill_path)?;
    std::fs::write(
        &skill_path,
        skill
            .replace(
                "Status observation of a running packaged Sentinel MUST be read-only.",
                "Status observation of a running packaged Sentinel may mutate it.",
            )
            .replace(
                "Parent policy MUST use event-driven terminal deltas and MUST NOT poll a running Sentinel.",
                "Parent policy may poll a running Sentinel.",
            ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("status observation of a running packaged sentinel must be read-only"));
    assert!(stderr.contains("event-driven terminal deltas"));
    Ok(())
}

#[test]
fn validator_cli_rejects_weakened_or_historical_sentinel_observation_clauses() -> TestResult {
    for (target, replacement, expected) in [
        (
            "Status observation of a running packaged Sentinel MUST be read-only.",
            "Status observation of a running packaged Sentinel MUST be read-only, but it may interrupt a live Sentinel.",
            "status observation of a running packaged sentinel must be read-only",
        ),
        (
            "Status observation of a running packaged Sentinel MUST be read-only.",
            "Status observation of a running packaged Sentinel MUST NOT be read-only.",
            "status observation of a running packaged sentinel must be read-only",
        ),
        (
            "Status observation of a running packaged Sentinel MUST be read-only.",
            "Stale example: Status observation of a running packaged Sentinel MUST be read-only.",
            "status observation of a running packaged sentinel must be read-only",
        ),
        (
            "Status observation of a running packaged Sentinel MUST be read-only.",
            "The active policy is described elsewhere.\n\n## Historical example\n\nStatus observation of a running packaged Sentinel MUST be read-only.",
            "status observation of a running packaged sentinel must be read-only",
        ),
        (
            "delayed output alone MUST NOT cause `UNOBSERVABLE`.",
            "delayed output alone MUST NOT cause `UNOBSERVABLE`, but a status request may declare it unavailable.",
            "delayed output alone must not cause `unobservable`",
        ),
    ] {
        let (_temp, plugin_root) = copy_plugin_fixture()?;
        let skill_path = plugin_root.join("skills/codex-orchestration/SKILL.md");
        let skill = std::fs::read_to_string(&skill_path)?;
        std::fs::write(&skill_path, skill.replace(target, replacement))?;

        let output = validator(&plugin_root, "--check")?;
        assert!(!output.status.success(), "accepted {replacement:?}");
        assert!(stderr(&output).contains(expected));
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
