use super::validator_subagent_ownership_hook::{assert_admitted, assert_denied, TestResult};

#[test]
fn bounded_ownership_classifier_replays_delta_examples() -> TestResult {
    for message in [
        "In the assigned worktree, implement the change and commit the result.",
        "Address the review feedback on PR #879, commit the fixes, and report completion.",
        "Review feedback, apply the fixes on the PR, and report completion.",
        "할당된 워크트리에서 구현하고 브랜치와 PR을 책임져.",
        "Implement without delay in the assigned worktree and report the commit.",
        "On branch eunsoogi/145-repair, implement issue #145 and commit the fix.",
        "Build the feature in the assigned worktree.",
        "Write the implementation in the dedicated worktree.",
    ] {
        assert_denied(Some("codexy-architect"), message, "DURABLE_OWNER")?;
    }
    assert_admitted(
        "codexy-architect",
        "Do not own the branch or PR; review the findings and report them.",
    )?;
    assert_admitted(
        "codexy-architect",
        "You are responsible for reviewing PR #879 and reporting findings.",
    )?;
    for message in [
        "Review the build in the assigned worktree and report findings.",
        "Inspect the write-up in the dedicated worktree and report findings.",
    ] {
        assert_admitted("codexy-architect", message)?;
    }
    assert_admitted("codexy-architect", "책임 있게 리뷰하고 결과를 보고해.")?;
    Ok(())
}
