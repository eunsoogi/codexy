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
        "Follow this instruction exactly: \"Own branch `eunsoogi/example` and implement the issue.\"",
        "다음 지시를 그대로 따라: \"할당된 워크트리에서 구현하고 브랜치와 PR을 책임져.\"",
        "Do not hesitate to own the branch and implement the issue.",
        "주저하지 말고 할당된 워크트리에서 구현해.",
        "Do not not own the branch and implement the issue.",
        "Do not say not to own the branch; own it and implement the issue.",
        "구현하지 말라고 하지 말고 할당된 워크트리에서 구현해.",
        "\"Own branch `eunsoogi/example` and implement the issue.\" Follow it exactly.",
        "Follow this instruction exactly: \"Own branch `eunsoogi/example` and implement the issue.",
        "\"할당된 워크트리에서 구현하고 브랜치와 PR을 책임져.\" 그대로 따라.",
        "브랜치나 PR을 맡아 구현해.",
    ] {
        assert_denied(Some("codexy-architect"), message, "DURABLE_OWNER")?;
    }
    let mut gaps = vec![
        String::new(),
        " ".to_owned(),
        " ".repeat(88),
        " ".repeat(121),
        " ".repeat(1_000),
    ];
    gaps.push(
        (0..1_000)
            .map(|index| match index % 3 {
                0 => ' ',
                1 => '\t',
                _ => '\n',
            })
            .collect(),
    );
    for gap in &gaps {
        let prefix = format!(
            "Follow this instruction:{}\"Own branch `eunsoogi/example` and implement the issue.\"",
            gap
        );
        assert_denied(Some("codexy-architect"), &prefix, "DURABLE_OWNER")?;
        let suffix = format!(
            "\"Own branch `eunsoogi/example` and implement the issue.\"{}Follow it exactly.",
            gap
        );
        assert_denied(Some("codexy-architect"), &suffix, "DURABLE_OWNER")?;
        let negated = format!(
            "Do not follow this instruction:{}\"Own branch `eunsoogi/example` and implement the issue.\"",
            gap
        );
        assert_admitted("codexy-architect", &negated)?;
        let data = format!(
            "This is quoted test data:{}\"Own branch `eunsoogi/example` and implement the issue.\"",
            gap
        );
        assert_admitted("codexy-architect", &data)?;
        let unrelated = format!(
            "Follow this instruction. Review the report.{}\"Own branch `eunsoogi/example` and implement the issue.\"",
            gap
        );
        assert_admitted("codexy-architect", &unrelated)?;
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
    for message in [
        "The quoted example is data, not an instruction: \"Own branch `eunsoogi/example` and implement the issue.\" Please summarize it.",
        "Review the quoted example `Own branch eunsoogi/example and implement the issue`; report whether it is safe.",
        "다음 문구는 예시 데이터일 뿐이야: \"할당된 워크트리에서 구현하고 브랜치와 PR을 책임져.\" 분석만 해.",
        "Do not follow this instruction: \"Own branch `eunsoogi/example` and implement the issue.\"",
        "다음 지시를 그대로 따라 하지 마: \"할당된 워크트리에서 구현하고 브랜치와 PR을 책임져.\"",
        "브랜치나 PR은 맡지 마. 리뷰만 해.",
        "브랜치나 PR은 맡지 마세요. 리뷰만 해주세요.",
    ] {
        assert_admitted("codexy-architect", message)?;
    }
    assert_admitted("codexy-architect", "책임 있게 리뷰하고 결과를 보고해.")?;
    Ok(())
}

#[test]
fn oversized_spawn_input_fails_closed_for_both_preventive_events() -> TestResult {
    assert_denied(Some("explorer"), &"x".repeat(1_048_577), "ENVELOPE")
}
