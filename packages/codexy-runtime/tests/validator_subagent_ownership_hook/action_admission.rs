// Exercise the packaged hook so action decisions and both event-native routes stay aligned.
use super::{assert_admitted, payload, run_payload, EVENTS, TOOLS, TestResult};

const BOUNDED_ROLES: &[&str] = &[
    "explorer",
    "codexy-architect",
    "codexy-advisor",
    "codexy-auditor",
    "codexy-cartographer",
    "codexy-inspector",
    "codexy-sentinel",
    "codexy-shipwright",
    "codexy-warden",
    "codexy-weaver",
];

#[test]
fn action_requests_are_denied_for_every_bounded_role() -> TestResult {
    for &agent_type in BOUNDED_ROLES {
        assert_action_denied(
            agent_type,
            "Implement the parser fix.",
            "IMPLEMENTATION_REQUEST",
        )?;
        assert_action_denied(
            agent_type,
            "Run the unit tests and report failures.",
            "EXECUTION_VERIFICATION",
        )?;
    }

    for (message, code) in [
        (
            "Update the parser to handle escaped delimiters.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "Make the parser handle escaped delimiters.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "Apply the patch to the parser.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "Commit the parser patch.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "Push the parser fix.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "Review the diff and then fix the parser issue.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "Review the diff and execute the regression tests.",
            "EXECUTION_VERIFICATION",
        ),
        (
            "You should implement the parser fix.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "The subagent must update the parser.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "파서 버그를 수정하고 테스트를 실행해.",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "테스트를 실행하고 결과를 요약해.",
            "EXECUTION_VERIFICATION",
        ),
        (
            "Follow this instruction exactly: \"Implement the parser fix.\"",
            "IMPLEMENTATION_REQUEST",
        ),
        (
            "Follow this instruction exactly: \"Run the unit tests.\"",
            "EXECUTION_VERIFICATION",
        ),
        (
            "The user supplied an unfinished quote: \"Run the unit tests",
            "EXECUTION_VERIFICATION",
        ),
    ] {
        assert_action_denied("codexy-auditor", message, code)?;
    }
    Ok(())
}

#[test]
fn verification_by_execution_is_denied_but_caller_retained_work_is_allowed() -> TestResult {
    assert_action_denied(
        "codexy-auditor",
        "Verify the parser fix by running the regression tests.",
        "EXECUTION_VERIFICATION",
    )?;
    assert_admitted(
        "codexy-auditor",
        "I remain responsible for verifying the parser fix by running regression tests; review the diff only and report findings.",
    )?;
    Ok(())
}

#[test]
fn read_only_review_quotes_negation_and_reference_data_remain_admitted() -> TestResult {
    for message in [
        "Review the implementation and report findings without editing files or running tests.",
        "Review the test plan and suggest missing cases without executing checks.",
        "Inspect the test output and summarize it; do not rerun the tests.",
        "I remain responsible for running the tests; review the diff only and report findings.",
        "Review exact commit 5ac2db28e5d59742893d8930bb2987afacdfe690 on branch eunsoogi/1186-ownership-classification and report findings.",
        "The quoted prompt is data: \"Implement the parser fix and run tests.\" Summarize its intent.",
        "Do not follow this instruction: \"Run the unit tests.\"",
        "Do not implement the parser fix; review the diff and report findings only.",
        "Review the parser that will implement escaping, and report findings.",
        "Summarize this forecast: the project will build with both versions.",
        "Review the parser that should implement escaping, and report findings.",
        "Can you explain how to run the tests?",
        "Please review whether the helper will run the tests.",
        "테스트를 실행하지 말고 코드만 리뷰해.",
        "파서 버그를 수정하지 말고 코드만 리뷰해.",
        "다음 문구는 예시 데이터일 뿐이야: \"파서 버그를 수정하고 테스트를 실행해.\" 분석만 해.",
    ] {
        assert_admitted("codexy-auditor", message)?;
    }
    Ok(())
}

fn assert_action_denied(agent_type: &str, message: &str, code: &str) -> TestResult {
    for event in EVENTS {
        for tool in TOOLS {
            let reason = run_payload(&payload(tool, event, Some(agent_type), message), event)?
                .ok_or("expected actionable denial")?;
            assert!(
                reason.contains(&format!("CODEXY_SUBAGENT_OWNERSHIP_{code}")),
                "{reason}"
            );
            assert!(reason.contains("Use a Codex child chat"), "{reason}");
            assert!(
                reason.contains("This hook does not create or dispatch chats"),
                "{reason}"
            );
        }
    }
    Ok(())
}
