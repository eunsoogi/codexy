use std::path::{Path, PathBuf};
use std::process::Command;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
fn validator_cli_rejects_missing_live_worktree_reservation_preflight() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    std::fs::write(
        &routing_path,
        routing
            .replace(
                "Live Worktree Reservation Preflight",
                "Worktree setup notes",
            )
            .replace("reservation map", "untracked list")
            .replace(
                "MUST NOT create or fork the new thread",
                "May create the new thread",
            ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("live worktree reservation preflight"));
    assert!(stderr.contains("reservation map"));
    assert!(stderr.contains("must not create or fork the new thread"));
    Ok(())
}

#[test]
fn validator_cli_rejects_exception_that_weakens_reservation_preflight() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    assert!(routing.contains("MUST NOT create or fork the new thread, retry the same path"));
    std::fs::write(
        &routing_path,
        routing.replace(
            "MUST NOT create or fork the new thread, retry the same path",
            "MUST NOT create or fork the new thread unless the allocator appears healthy, then retry the same path",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("must not create or fork the new thread"));
    Ok(())
}

#[test]
fn validator_cli_rejects_comma_prefixed_exception_that_weakens_reservation_preflight() -> TestResult
{
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    std::fs::write(
        &routing_path,
        routing.replace(
            "MUST NOT create or fork the new thread, retry the same path",
            "MUST NOT create or fork the new thread, unless the allocator appears healthy, then retry the same path",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    Ok(())
}

#[test]
fn validator_cli_rejects_semicolon_prefixed_exception_that_weakens_reservation_preflight()
-> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    std::fs::write(
        &routing_path,
        routing.replace(
            "MUST NOT create or fork the new thread, retry the same path",
            "MUST NOT create or fork the new thread; unless the allocator appears healthy, then retry the same path",
        ),
    )?;
    assert!(!validator(&plugin_root, "--check")?.status.success());
    Ok(())
}

#[test]
fn validator_cli_rejects_historical_example_that_retains_reservation_phrase() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    assert!(routing.contains("MUST NOT create or fork the new thread, retry the same path"));
    std::fs::write(
        &routing_path,
        routing.replace(
            "The parent MUST NOT create or fork the new thread, retry the same path",
            "Historical example only: the parent MUST NOT create or fork the new thread, retry the same path",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    let stderr = stderr(&output);
    assert!(stderr.contains("must not create or fork the new thread"));
    Ok(())
}

#[test]
fn validator_cli_rejects_numbered_historical_example_that_retains_reservation_phrase() -> TestResult
{
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    std::fs::write(
        &routing_path,
        routing.replace(
            "The parent MUST NOT create or fork the new thread, retry the same path",
            "Historical example no. 1: the parent MUST NOT create or fork the new thread, retry the same path",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    Ok(())
}

#[test]
fn validator_cli_accepts_current_contract_after_historical_examples_heading() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    std::fs::write(
        &routing_path,
        routing.replace(
            "The parent MUST NOT create or fork the new thread, retry the same path",
            "## Historical examples\n\n## Current contract\n\nThe parent MUST NOT create or fork the new thread, retry the same path",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_cli_accepts_contract_after_unrelated_not_required_sentence() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let routing_path =
        plugin_root.join("skills/codex-orchestration/references/thread-and-worktree-routing.md");
    let routing = std::fs::read_to_string(&routing_path)?;
    std::fs::write(
        &routing_path,
        routing.replace(
            "The parent MUST NOT create or fork the new thread, retry the same path",
            "A retry explanation is not required. The parent MUST NOT create or fork the new thread, retry the same path",
        ),
    )?;
    let output = validator(&plugin_root, "--check")?;
    assert!(output.status.success(), "{}", stderr(&output));
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

fn copy_plugin_fixture() -> TestResult<(tempfile::TempDir, PathBuf)> {
    let temp = tempfile::tempdir()?;
    let plugin_root = temp.path().join("codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok((temp, plugin_root))
}

fn validator(
    plugin_root: &Path,
    mode: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let root = plugin_root.to_str().ok_or("plugin root path")?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", root, mode])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
