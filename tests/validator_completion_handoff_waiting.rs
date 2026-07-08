use std::{path::Path, process::Command};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;
#[test]
fn validator_cli_rejects_blocked_pending_codex_review_handoff() -> TestResult {
    for handoff in [
        "Blocked on current-head @codex review request.\n",
        "Blocked: current-head @codex review request.\n",
        "Blocked: pending @codex review, required checks are failing? no.\n",
        "Blocked: no @codex review feedback has returned yet.\n",
        "Blocked: pending Codex review feedback.\n",
        "Blocked: pending @codex review feedback.\n",
        "Blocked: pending @codex review after previous blocker resolved.\n",
        "Blocked: pending @codex review, awaiting feedback.\n",
        "Blocked: waiting on feedback from @codex review.\n",
        "Blocked: waiting on feedback from Codex review.\n",
        "Blocked: pending feedback from @codex review.\n",
        "Blocked: awaiting feedback from Codex review.\n",
        "Blocked: pending @codex review, required status checks are failing = false.\n",
        "Blocked: pending @codex review, status checks failed - none.\n",
        "Blocked: @codex review feedback has not arrived yet.\n",
        "Blocked: Codex connector review feedback is pending.\n",
        "Blocked because Codex review has no feedback yet.\n",
        "Blocked: pending @codex review, security review passed.\n",
        "Blocked: waiting on Codex review feedback from the connector.\n",
        "Blocked by @codex review request.\n",
        "Blocked due to @codex review request.\n",
        "Blocked: @codex review has eyes only.\n",
        "Blocked because Codex review has eyes only.\n",
        "Blocked.\nWaiting: pending @codex review.\nRequired checks are failing: no.\n",
        "Goal blocked because child-thread work is still pending.\n",
        "Goal blocked after previous blocker resolved: pending @codex review.\n",
        "Blocked: child thread is still working on the required change.\n",
        "Blocked: child thread verification is still pending.\n",
        "Blocked: child thread has not returned yet.\n",
        "Work is blocked after previous blocker resolved: pending @codex review.\n",
        "Previously blocked on @codex review; now blocked on pending @codex review.\n",
        "Blocked: child-owned review-response work is still pending.\n",
        "Blocked: missing child thread response is pending.\n",
        "Previous blocker resolved: now blocked on pending @codex review.\n",
        "Blocked: @codex review has not returned yet after previous permission error was fixed.\n",
        "Blocked: @codex review is pending after previous Codex review usage limits were reached and fixed.\n",
        "Blocked: @codex review has not returned yet after previous Codex review request failed and was fixed.\n",
        "Blocked: @codex review is pending after previous code-review usage limits were reached and fixed.\n",
        "Blocker: queued worktree setup has not completed yet.\n",
        "Blocked: worktree setup is queued.\n",
        "Blocked: thread setup is queued.\n",
        "Blocked on asynchronous tool completion.\n",
        "Blocked on asynchronous GitHub tool completion.\n",
        "Blocked on asynchronous Codex tool completion.\n",
        "Blocked: async GitHub merge tool is still pending.\n",
        "Blocked: asynchronous Codex review tool has not returned yet.\n",
        "Blocked: async tool has not returned yet.\n",
        "Blocked until the asynchronous tool returns.\n",
        "Blocked: waiting for the tool result to return.\n",
        "Previous test failure was fixed. Blocked: async GitHub tool hasn't returned yet.\n",
        "Blocked: async GitHub tool is still pending after previous permission error was fixed.\n",
        "Blocked: @codex review has not returned yet after previous async tool returned a permission error earlier.\n",
        "Previous blocker resolved: required status checks failed and were fixed. Blocked: pending @codex review.\n",
        "Previous blocker resolved: async GitHub tool returned a permission error and was fixed. Blocked: @codex review has not returned yet.\n",
        "Blocked: pending @codex review after required checks failed and were fixed.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject non-blocking wait classified as blocked\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("waiting state"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
#[test]
fn validator_cli_handles_resolved_comma_blocker_boundaries() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Previous blocker resolved: required status checks failed and were cleared, blocked: pending @codex review.\n",
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("waiting state"));
    let output = validate_handoff_with_pr_state(
        "Blocked: pending @codex review, required status checks are failing after the formatter issue was fixed.\n",
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )?;
    assert!(output.status.success());
    let output = validate_handoff_with_pr_state(
        "Blocked: pending @codex review, required status checks failed after the formatter issue was fixed.\n",
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )?;
    assert!(output.status.success());
    Ok(())
}
#[test]
fn validator_cli_allows_true_impasse_blocked_handoff() -> TestResult {
    accept_open_pr_handoff(
        "Blocked after repeated true impasse: cannot make meaningful progress without maintainer input.\n",
        "validator should preserve true impasse blocked handoffs",
    )?;
    Ok(())
}
#[test]
fn validator_cli_allows_unrelated_pending_review_blocker() -> TestResult {
    for handoff in [
        "Blocked: security review is waiting for approval. Codex review context noted.\n",
        "Blocked: pending @codex review; waiting for feedback from the maintainer to choose the release path.\n",
        "Blocked: security review is pending with no security review output yet. Waiting: pending Codex review.\n",
        "Blocked: waiting on feedback from the maintainer to choose the release path. Codex review context noted.\n",
        "Blocked: security review feedback is pending. Codex review context noted.\n",
        "Blocked: pending review feedback from the maintainer; waiting for feedback to arrive. Codex review context noted.\n",
        "Blocked: pending @codex review, required status checks are failing.\n",
        "Blocked: pending @codex review; required status checks are failing.\n",
        "Blocked: pending @codex review, required status checks are failing after previous blocker was resolved.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should not treat real blockers plus separate waiting evidence as false blocked claims",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_allows_actionable_codex_review_blocker() -> TestResult {
    for handoff in [
        "Blocked: Codex review requested changes remain unresolved.\n",
        "Blocked: Codex review feedback is pending resolution.\n",
        "Blocked: Codex review usage limit was reached.\n",
        "Blocked: Codex review usage limits were reached.\n",
        "Blocked: @codex review usage limits were reached.\n",
        "Blocked: Codex connector review request failed.\n",
        "Blocked on Codex review: the connector asked to create an environment for this repo.\n",
        "Blocked: Codex review failed because previous Codex review failure was fixed but code-review usage limits were reached.\n",
        "Blocked: Codex review failed because code-review usage limits were reached; pending @codex review retry.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should preserve actionable Codex review blockers",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_allows_missing_child_evidence_blocker() -> TestResult {
    for handoff in [
        "Blocked: child thread omitted required goal tool evidence.\n",
        "Blocked until child thread returns required goal tool evidence.\n",
        "Blocked: child thread is still pending required goal tool evidence.\n",
        "Blocked: child-owned review-response work is missing required verification evidence.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should preserve missing child-lane evidence blockers",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_allows_negated_blocker_waiting_state() -> TestResult {
    for handoff in [
        "Not a blocker: pending Codex review is still processing.\n",
        "Previously blocked on @codex review; now Codex review passed, Sentinel: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893, and PR is merge-ready.\n",
        "Blockers: none\nWaiting: pending Codex review.\n",
        "Previous blocker resolved. Waiting: pending Codex review.\n",
        "Previous blocker was resolved. Waiting: pending @codex review.\n",
        "Blocked: no\nWaiting: pending Codex review.\n",
        "Blocked: no active blockers\nWaiting: pending Codex review.\n",
        "No current blockers. Waiting: pending Codex review is still processing.\n",
        "Was blocked on @codex review; now Codex review passed, Sentinel: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893, and PR is merge-ready.\n",
        "Blocked? no. Waiting: pending Codex review is still processing.\n",
        "Blocked = false. Waiting: pending @codex review.\n",
        "Blocked: no; Waiting: pending Codex review is still processing.\n",
        "Blockers: none remaining\nWaiting: pending Codex review.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should allow waiting evidence that is not classified as blocked",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_allows_failed_setup_blockers() -> TestResult {
    for handoff in [
        "Blocked: worktree setup failed because the requested base branch does not exist.\n",
        "Blocked: thread setup failed with fatal invalid reference.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should preserve real setup failure blockers",
        )?;
    }
    Ok(())
}
#[test]
fn validator_cli_allows_returned_async_tool_failures() -> TestResult {
    for handoff in [
        "Blocked: async GitHub merge tool returned a permission error.\n",
        "Blocked: async GitHub merge tool returned. Permission error prevents merge.\n",
        "Blocked: async GitHub tool completion failed with a permission error.\n",
        "Blocked: async GitHub tool completion returned. Permission error prevents merge.\n",
        "Blocked: @codex review has not returned yet after async GitHub merge tool returned a permission error.\n",
        "Blocked: asynchronous Codex tool returned an authentication failure.\n",
        "Blocked: tool result returned with a fatal API error.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should preserve async/tool failures after the tool has returned",
        )?;
    }
    Ok(())
}
fn validate_completion_handoff(handoff_path: &Path, pr_state_path: &Path) -> OutputResult {
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-completion-handoff",
            "--handoff-file",
            handoff_path.to_str().ok_or("handoff path")?,
            "--pr-state-file",
            pr_state_path.to_str().ok_or("pr state path")?,
        ])
        .output()?)
}
fn accept_open_pr_handoff(handoff: &str, failure_message: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        output.status.success(),
        "{failure_message}\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}
fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    validate_handoff_with_pr_state(
        handoff,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )
}
