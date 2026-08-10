use crate::support;

use support::{TestResult, stderr, validator_instruction_policy_file};

#[test]
fn validator_rejects_missing_parent_goal_transition_reporting_contract() -> TestResult {
    let fixture = support::instruction_policy_fixture(std::path::Path::new(
        "skills/orchestration/references/goal-transition-reporting.md",
    ))?;
    let reference = fixture.path();
    let text = std::fs::read_to_string(&reference)?;
    assert!(text.contains("Before `create_goal`"));
    assert!(text.contains("After every goal tool call, including `get_goal`"));
    assert!(text.contains("actual Codex task/thread messaging surface"));
    assert!(text.contains("MUST NOT execute until parent delivery is confirmed"));
    assert!(text.contains("stable transition key"));
    assert!(text.contains("canonical reserved worktree"));
    assert!(text.contains("Before a child stops, archives, or releases lane ownership"));
    assert!(text.contains(
        "Before stop, archive, ownership release, `update_goal(complete)`, or `update_goal(blocked)`"
    ));
    assert!(text.contains("terminal handoff receipt exactly once"));
    assert!(text.contains("Delivery MUST be confirmed before the stop/archive/release"));
    assert!(text.contains("MUST preserve the lane instead of transitioning"));

    std::fs::write(
        &reference,
        text.replace(
            "MUST NOT execute until parent delivery is confirmed",
            "may execute before delivery is confirmed",
        ),
    )?;

    let output = validator_instruction_policy_file(reference)?;
    assert!(!output.status.success());
    assert!(stderr(&output).contains("must not execute until parent delivery is confirmed"));
    Ok(())
}
