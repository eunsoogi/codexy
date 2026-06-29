use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_blocked_pending_codex_review_handoff() -> TestResult {
    for handoff in [
        "Blocked: current-head @codex review request has an eyes reaction and is still pending.\n",
        "Currently blocked: pending @codex review is still processing.\n",
        "Blocked: current-head @codex review comment has an eyes reaction and is still pending.\n",
        "Blocked: pending @codex review request has no actionable feedback yet.\n",
        "Blocked: pending Codex review feedback.\n",
        "Blocked: pending @codex review feedback.\n",
        "Blocked: pending @codex review, awaiting feedback.\n",
        "Blocked: @codex review is waiting for feedback.\n",
        "Blocked: @codex review is awaiting feedback.\n",
        "Blocked: awaiting Codex review feedback.\n",
        "Blocked: Codex review feedback pending.\n",
        "Blocked: @codex review feedback pending.\n",
        "Blocked: @codex review feedback has not returned yet.\n",
        "Blocked: Codex review feedback has not yet returned.\n",
        "Blocked: Codex review feedback is pending.\n",
        "Blocked: Codex review is pending feedback from the connector.\n",
        "Blocked: pending @codex review, security review passed.\n",
        "Blocked: waiting for Codex review feedback from the connector.\n",
        "Blocked: missing Codex review response is pending.\n",
        "Blocked: @codex review has not returned yet.\n",
        "Blocked state: pending Codex review is still processing.\n",
        "Blocked.\nWaiting: pending Codex review is still processing.\n",
        "Goal blocked because child-thread work is still pending.\n",
        "Goal blocked.\nPending child thread response.\n",
        "Blocked: child thread is still working on the required change.\n",
        "Blocked: child thread verification is still pending.\n",
        "Blocked: child thread has not returned yet.\n",
        "Blocked: child thread is still pending feedback.\n",
        "Blocked: child lane is still pending.\n",
        "Blocked: missing child thread response is pending.\n",
        "Goal blocked until Codex connector review returns.\n",
        "Goal blocked until child thread returns.\n",
        "Blocker: queued worktree setup has not completed yet.\n",
        "Blocked: worktree setup is queued.\n",
        "Blocked: thread setup is queued.\n",
        "Blocked on asynchronous tool completion.\n",
        "Blocked on asynchronous GitHub tool completion.\n",
        "Blocked on asynchronous Codex tool completion.\n",
        "Blocked: async GitHub merge tool is still pending.\n",
        "Blocked: asynchronous Codex review tool has not returned yet.\n",
        "Blocked: asynchronous tool has not returned yet.\n",
        "Blocked: async tool has not returned yet.\n",
        "Blocked until the asynchronous tool returns.\n",
        "Blocked: waiting for the tool result to return.\n",
        "Blocked: background operation has not yet returned.\n",
        "Blocked: async operation result has not yet returned.\n",
        "Previous test failure was fixed. Blocked: async GitHub tool hasn't returned yet.\n",
        "Earlier failure is resolved. Blocked: asynchronous Codex tool hasn't returned yet.\n",
        "Blocked: @codex review has not returned yet, previous async tool returned a permission error earlier.\n",
        "Previous blocker resolved: required status checks failed and were fixed. Blocked: pending @codex review.\n",
        "Previous blocker resolved: async GitHub tool returned a permission error and was fixed. Blocked: @codex review has not returned yet.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject non-blocking wait classified as blocked\nstdout:\n{}\nstderr:\n{}",
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
        "Previous blocker resolved: required status checks failed and were fixed, blocked: pending @codex review.\n",
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("waiting state"));
    let output = validate_handoff_with_pr_state(
        "Blocked: pending @codex review, required status checks are failing after the formatter issue was fixed.\n",
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
        "Blocked: required security review is still pending.\n",
        "Blocked: required security review is still pending.\nWaiting: pending Codex review is still processing.\n",
        "Blocked: required security review is still pending; waiting for pending Codex review is still processing.\n",
        "Blocked: required checks are failing. Waiting: pending Codex review is still processing.\n",
        "Blocked: required status checks are failing. Waiting: pending Codex review is still processing.\n",
        "Blocked: status checks are failing. No security review required. Waiting: pending Codex review is still processing.\n",
        "Blocked: pending @codex review, required status checks are failing.\n",
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
        "Blocked: Codex review feedback from the connector is pending resolution.\n",
        "Blocked: current-head Codex review feedback is pending resolution.\n",
        "Blocked: Codex review feedback pending resolution.\n",
        "Blocked: current-head @codex review request has pending actionable feedback.\n",
        "Blocked until Codex review feedback is pending resolution and the connector returns.\n",
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
        "Not currently blocked: pending @codex review is still processing.\n",
        "Non-blocker: pending Codex review is still processing.\n",
        "Blockers: None.\nNot a blocker: pending Codex review is still processing.\n",
        "Blockers: None.\nWaiting: pending Codex review is still processing.\n",
        "Blocked: no. Waiting: pending Codex review is still processing.\n",
        "No known blockers. Waiting: pending Codex review is still processing.\n",
        "Blockers - none. Waiting: pending Codex review is still processing.\n",
        "Blocked? no. Waiting: pending Codex review is still processing.\n",
        "Goal blocked: no. Waiting: pending Codex review is still processing.\n",
        "Blocked state: none. Waiting: pending Codex review is still processing.\n",
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
        "Blocked: async GitHub tool completion returned a permission error.\n",
        "Blocked: async GitHub tool completion returned. Permission error prevents merge.\n",
        "Blocked: async GitHub tool completion returned, permission error prevents merge.\n",
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
        "{failure_message}\nstdout:\n{}\nstderr:\n{}",
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
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#,
    )
}
