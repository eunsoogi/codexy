use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_unobservable_sentinel_as_pr_readiness() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: UNOBSERVABLE after bounded waits. Pushed: yes.\n",
        "PR ready: no blockers. Sentinel: UNOBSERVABLE after bounded waits.\n",
        "PR readiness: yes. Sentinel: UNOBSERVABLE after bounded waits.\n",
        "Merge readiness: yes. Sentinel: UNOBSERVABLE after bounded waits.\n",
        "PR ready for parent handoff. Sentinel verdict: UNOBSERVABLE after bounded wait. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel pending after bounded wait. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel is delayed after bounded wait. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel timed out after bounded wait. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel produced no verdict after bounded wait. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel did not return PASS or BLOCK after bounded wait. Pushed: yes.\n",
        "Parent can open PR next. Packaged Sentinel Lagrange has not returned after the bounded wait. Remote/PR head match: yes.\n",
        "Ready for merge gates. Sentinel status: stuck waiting for verdict; no PASS or BLOCK surfaced.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject readiness claims backed by an unobservable Sentinel\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Sentinel"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_blocked_sentinel_as_pr_readiness() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: BLOCK, Carver found same-scope issue. Pushed: yes.\n",
        "PR ready: no blockers. Sentinel: BLOCK, Carver found same-scope issue.\n",
        "PR readiness: yes. Sentinel: BLOCK, Carver found same-scope issue.\n",
        "Merge readiness: yes. Sentinel: BLOCK, Carver found same-scope issue.\n",
        "PR ready for parent handoff. Sentinel verdict: BLOCK. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel result: BLOCK. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel gate returned BLOCK. Pushed: yes.\n",
        "Parent can open PR next. Packaged Codexy Sentinel returned BLOCK but focused tests passed.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject readiness claims backed by a blocking Sentinel verdict\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Sentinel"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_sentinel_readiness_without_explicit_status() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Packaged Codexy Sentinel Lagrange reviewed exact head and current diff. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel evidence: reviewed exact head, no blockers listed. Pushed: yes.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject Sentinel readiness without explicit PASS, BLOCK, or UNOBSERVABLE status\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Sentinel"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_current_sentinel_pass_after_superseded_block() -> TestResult {
    accept_open_pr_handoff(
        "PR ready for parent handoff. Initial Sentinel: BLOCK on earlier head; addressed with parser fixes. Rerun Sentinel: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "validator should accept current Sentinel PASS after superseded historical BLOCK evidence",
    )
}

#[test]
fn validator_rejects_unobservable_sentinel_as_push_readiness() -> TestResult {
    for handoff in [
        "Push-ready. Sentinel timed out after bounded wait. Pushed: no. PR ready: no.\n",
        "Ready to push. Sentinel pending after bounded wait. Pushed: no. PR ready: no.\n",
        "Push readiness: yes. Sentinel produced no verdict after bounded wait. Pushed: no. PR ready: no.\n",
        "Ready for push. Sentinel did not return PASS or BLOCK after bounded wait. Pushed: no. PR ready: no.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject push readiness claims backed by an unobservable Sentinel\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Sentinel"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_accepts_explicit_sentinel_pass_for_pr_readiness() -> TestResult {
    accept_open_pr_handoff(
        "PR ready for parent handoff. Sentinel: PASS, Euclid reviewed exact head and current diff. Pushed: yes. Parent will handle review and merge gates; this lane is not complete until merge.\n",
        "validator should accept explicit Sentinel PASS readiness evidence",
    )
}

#[test]
fn validator_accepts_unobservable_sentinel_when_handoff_stops_before_readiness() -> TestResult {
    for handoff in [
        "Sentinel: UNOBSERVABLE after bounded waits. Pushed: no. PR ready: no. Parent decision required: yes. This lane is not ready for handoff.\n",
        "Sentinel: UNOBSERVABLE after bounded waits. Pushed: no.\nPR ready: no\nParent decision required: yes.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should accept a bounded stuck Sentinel status when it does not claim readiness",
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
        r#"{"number":221,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-07-03T00:00:00Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )
}
