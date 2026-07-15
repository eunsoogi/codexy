use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_allows_ordered_blocker_after_ready_heading() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR ready\n1. missing status evidence\nWaiting on review.\n",
        &dirty_pr_state(),
    )?;
    assert!(
        output.status.success(),
        "ordered blocker item should keep heading non-affirmative\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_ordered_checked_ready_label_without_child_marker() -> TestResult {
    for handoff in [
        "1. PR-ready: yes.\n",
        "1) merge-ready: yes.\n",
        "1. [x] PR-ready: yes.\n",
    ] {
        assert_rejects_dirty_ordered_ready_label(handoff)?;
    }
    Ok(())
}

fn assert_rejects_dirty_ordered_ready_label(handoff: &str) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, &dirty_pr_state())?;
    assert!(
        !output.status.success(),
        "ready label should require readiness evidence"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("current status is dirty"),
        "unexpected stderr: {stderr}"
    );
    Ok(())
}

fn dirty_pr_state() -> String {
    r###"{"number":204,"state":"OPEN","isDraft":false,"reviewDecision":"APPROVED","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`","author":{"login":"automated-review"},"submittedAt":"2026-07-03T00:00:00Z"}],"mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example\n M src/validation/child_handoff_readiness_claims.rs","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"###.to_string()
}

fn validate_handoff_with_pr_state(
    handoff: &str,
    pr_state: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
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
