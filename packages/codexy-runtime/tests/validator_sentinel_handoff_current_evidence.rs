use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn validator_requires_sentinel_evidence_for_completion_claims() -> TestResult {
    for handoff in [
        "Maintainer requested leave-open; implementation complete for the requested parser fix.\n",
        "Maintainer requested no-merge; all requested fixes completed for this PR lane.\n",
    ] {
        assert_rejects_without_appended_readiness(
            handoff,
            "Sentinel readiness evidence must be present",
        )?;
    }
    Ok(())
}

#[test]
fn validator_ties_fallback_approved_sentinel_statuses_to_current_head() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: BLOCK on current head. Maintainer explicitly approved fallback for this Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel timed out after bounded wait. Maintainer explicitly approved fallback for this timed-out Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on old head abc1234. Maintainer explicitly approved fallback for this Sentinel run. Current PR head is 32b03a210b3defb2d29dd352283ea2488e60d893. Pushed: yes.\n",
    ] {
        assert_rejects(handoff, "current PR head SHA")?;
    }
    Ok(())
}

#[test]
fn validator_accepts_current_head_fallback_approval() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: BLOCK on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel timed out after bounded wait on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this timed-out Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel pending after bounded wait on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this unobservable Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel pending verdict on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this unobservable Sentinel run. Pushed: yes.\n",
        "PR ready for parent handoff. Sentinel waiting for verdict on current head 32b03a210b3defb2d29dd352283ea2488e60d893. Maintainer explicitly approved fallback for this unobservable Sentinel run. Pushed: yes.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            output.status.success(),
            "validator should accept current-head fallback approval\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_parent_merge_readiness_with_blocking_sentinel() -> TestResult {
    for handoff in [
        "Child handoff: branch clean and pushed at 32b03a210b3defb2d29dd352283ea2488e60d893. Parent can merge. Sentinel: BLOCK on current head 32b03a210b3defb2d29dd352283ea2488e60d893.\n",
        "Child handoff: branch clean and pushed at 32b03a210b3defb2d29dd352283ea2488e60d893. Parent handoff ready. Sentinel: BLOCK on current head 32b03a210b3defb2d29dd352283ea2488e60d893.\n",
        "Child handoff: branch clean and pushed at 32b03a210b3defb2d29dd352283ea2488e60d893. Parent-handoff-ready. Sentinel: UNOBSERVABLE after bounded wait on current head 32b03a210b3defb2d29dd352283ea2488e60d893.\n",
    ] {
        assert_rejects_without_appended_readiness(handoff, "Sentinel")?;
    }
    Ok(())
}

#[test]
fn validator_rejects_future_waiting_sentinel_pass_after_current_block() -> TestResult {
    for handoff in [
        "PR ready for parent handoff. Sentinel: BLOCK on current head; waiting on Sentinel: PASS after rerun on current head 32b03a210b3defb2d29dd352283ea2488e60d893.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head; pending Sentinel: PASS after rerun on current head 32b03a210b3defb2d29dd352283ea2488e60d893.\n",
        "PR ready for parent handoff. Sentinel: BLOCK on current head; waiting on the reviewer gate returned PASS after rerun on current head 32b03a210b3defb2d29dd352283ea2488e60d893.\n",
    ] {
        assert_rejects(handoff, "Sentinel")?;
    }
    Ok(())
}

fn assert_rejects(handoff: &str, needle: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert_reject_output(handoff, needle, &output)
}

fn assert_rejects_without_appended_readiness(handoff: &str, needle: &str) -> TestResult {
    let output = validate_handoff(handoff)?;
    assert_reject_output(handoff, needle, &output)
}

fn assert_reject_output(handoff: &str, needle: &str, output: &std::process::Output) -> TestResult {
    assert!(
        !output.status.success(),
        "validator should reject handoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(needle), "unexpected stderr: {stderr}");
    Ok(())
}

fn validate_file(handoff_path: &Path, pr_state_path: &Path) -> TestResult<std::process::Output> {
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}

fn validate_open_pr_handoff(handoff: &str) -> TestResult<std::process::Output> {
    validate_handoff(&format!(
        "{handoff}\nBranch clean. Pushed at 32b03a210b3defb2d29dd352283ea2488e60d893. Remote/PR head match: yes 32b03a210b3defb2d29dd352283ea2488e60d893.\n"
    ))
}

fn validate_handoff(handoff: &str) -> TestResult<std::process::Output> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(
        &pr_state_path,
        r###"{"number":221,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/221-sentinel-bounded-wait-status","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","localHeadOid":"32b03a210b3defb2d29dd352283ea2488e60d893","remoteHeadOid":"32b03a210b3defb2d29dd352283ea2488e60d893","worktreeStatus":"## codexy/221-sentinel-bounded-wait-status...origin/codexy/221-sentinel-bounded-wait-status","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b`","author":{"login":"automated-review"},"submittedAt":"2026-07-03T00:00:00Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"###,
    )?;
    validate_file(&handoff_path, &pr_state_path)
}
