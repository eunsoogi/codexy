use std::{path::Path, process::Command};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
#[test]
fn validator_rejects_unobservable_sentinel_as_pr_readiness() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: UNOBSERVABLE after bounded waits. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel verdict: UNOBSERVABLE after bounded wait. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel timed out after bounded wait. Pushed: yes.\n",
        "Parent can open PR next. Packaged Sentinel Lagrange has not returned after the bounded wait. Remote/PR head match: yes.\n",
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
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer explicitly approved fallback for the previous unobservable Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer explicitly approved fallback for the previous Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer explicitly approved fallback for the previous reviewer gate run. Pushed: yes.\n",
        "PR is ready. Sentinel: BLOCK on current head.\n",
        "Completed. Sentinel: BLOCK on current head.\n",
        "PR is ready. Sentinel: BLOCK on current head. Previous Sentinel: UNOBSERVABLE after bounded wait. Maintainer explicitly approved fallback for this previous unobservable Sentinel run.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head; rerun Sentinel: PASS before push.\n",
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
fn validator_rejects_invalid_sentinel_readiness_evidence() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel doesn't pass on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel evidence: reviewed exact head, no blockers listed. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: PASS on old SHA abc1234, current PR head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: PASS, but not on current PR head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: PASS, but not for current head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: PASS, but not on the current PR head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: PASS, but not for the current head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject missing, stale, or statusless Sentinel readiness evidence\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
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
        "PR ready for parent handoff. Initial Sentinel: BLOCK on earlier head; addressed with parser fixes. Rerun Sentinel: PASS after rerun on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "validator should accept current Sentinel PASS after superseded historical BLOCK evidence",
    )
}
#[test]
fn validator_accepts_reviewer_named_sentinel_pass() -> TestResult {
    accept_open_pr_handoff(
        "PR ready for parent handoff. Packaged Codexy Sentinel Turing: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
        "validator should accept reviewer-named Sentinel PASS readiness evidence",
    )
}
#[test]
fn validator_ignores_unrelated_pending_review_after_sentinel_pass() -> TestResult {
    accept_open_pr_handoff(
        "Push-ready. Sentinel: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Codex review has not returned, so PR ready: no.\n",
        "validator should not treat unrelated pending Codex review text as Sentinel UNOBSERVABLE",
    )
}
#[test]
fn validator_rejects_unobservable_sentinel_as_push_readiness() -> TestResult {
    for handoff in [
        "Push-ready. Sentinel timed out after bounded wait. Pushed at 32b03a210b3defb2d29dd352283ea2488e60d893.\n",
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
        "PR ready for parent handoff. Sentinel: PASS, Euclid reviewed exact head 32b03a210b3defb2d29dd352283ea2488e60d893 as planned. Pushed: yes. Parent will handle review and merge gates; this lane is not complete until merge.\n",
        "validator should accept explicit Sentinel PASS readiness evidence",
    )?;
    accept_open_pr_handoff(
        "Previous reviewer feedback addressed and Sentinel PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. PR ready for parent handoff. Pushed: yes.\n",
        "validator should not treat non-Sentinel previous context as stale PASS evidence",
    )
}
#[test]
fn validator_rejects_current_block_before_hypothetical_future_pass() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: BLOCK on current head; waiting for Sentinel: PASS after rerun.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Previous Sentinel: PASS on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject current Sentinel BLOCK despite a future or historical PASS\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
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
fn validator_accepts_unobservable_sentinel_when_handoff_stops_before_readiness() -> TestResult {
    for handoff in [
        "Sentinel: UNOBSERVABLE after bounded waits. PR ready: no. Parent decision required: yes. This lane is not ready for handoff.\n",
        "Sentinel: UNOBSERVABLE after bounded waits. We aren't ready for handoff.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should accept a bounded stuck Sentinel status when it does not claim readiness",
        )?;
    }
    Ok(())
}
#[test]
fn validator_accepts_approved_fallback_for_timed_out_sentinel_readiness() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel timed out after bounded wait on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this unobservable Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel timed out after bounded wait on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this timed-out Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for current reviewer gate run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for the current Sentinel run. Pushed: yes.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should honor an explicit maintainer-approved Sentinel fallback",
        )?;
    }
    Ok(())
}
#[test]
fn validator_rejects_unapproved_sentinel_fallback_requirement_as_readiness() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: UNOBSERVABLE after bounded waits. Maintainer-approved fallback required before readiness; no maintainer response yet. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer explicitly approved fallback for CI. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer explicitly approved fallback for CI because the Sentinel run was blocked. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer explicitly approved fallback for this Sentinel run? no. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer approval: fallback approved for this Sentinel run? no. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer approval: fallback approved for this Sentinel run? Pushed: yes.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject fallback requirement text without actual approval\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
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
fn validate_file(handoff_path: &Path, pr_state_path: &Path) -> TestResult<std::process::Output> {
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
fn validate_with_state(handoff: &str, pr_state: &str) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_file(&handoff_path, &pr_state_path)
}
fn validate_open_pr_handoff(handoff: &str) -> TestResult<std::process::Output> {
    let not_ready = handoff.contains("not ready") || handoff.contains("aren't ready");
    let handoff = if not_ready {
        handoff.to_string()
    } else {
        format!(
            "{handoff}\nBranch clean. Pushed at 32b03a210b3defb2d29dd352283ea2488e60d893. Remote/PR head match: yes 32b03a210b3defb2d29dd352283ea2488e60d893.\n"
        )
    };
    validate_with_state(
        &handoff,
        r###"{"number":221,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/221-sentinel-bounded-wait-status","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","localHeadOid":"32b03a210b3defb2d29dd352283ea2488e60d893","remoteHeadOid":"32b03a210b3defb2d29dd352283ea2488e60d893","worktreeStatus":"## codexy/221-sentinel-bounded-wait-status...origin/codexy/221-sentinel-bounded-wait-status","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-07-03T00:00:00Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"###,
    )
}
