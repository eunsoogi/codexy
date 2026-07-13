mod support;

use support::{TestResult, copy_plugin_fixture, stderr, validator};

#[test]
fn validator_rejects_missing_parent_goal_transition_reporting_contract() -> TestResult {
    let (_temp, plugin_root) = copy_plugin_fixture()?;
    let reference =
        plugin_root.join("skills/codex-orchestration/references/goal-transition-reporting.md");
    let text = std::fs::read_to_string(&reference)?;
    assert!(text.contains("Before `create_goal`"));
    assert!(text.contains("After every goal tool call, including `get_goal`"));
    assert!(text.contains("actual Codex task/thread messaging surface"));
    assert!(text.contains("MUST NOT execute until parent delivery is confirmed"));
    assert!(text.contains("stable transition key"));
    assert!(text.contains("canonical reserved worktree"));

    std::fs::write(
        &reference,
        text.replace(
            "MUST NOT execute until parent delivery is confirmed",
            "may execute before delivery is confirmed",
        ),
    )?;

    let output = validator(&plugin_root, "--check")?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("must not execute until parent delivery is confirmed"));
    Ok(())
}
