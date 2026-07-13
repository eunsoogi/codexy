mod support;

use support::{TestResult, copy_plugin_fixture, stderr, validator};

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
