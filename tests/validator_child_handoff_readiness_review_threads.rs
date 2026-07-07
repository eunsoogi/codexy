use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_allows_accepted_no_change_rationale_for_pr_ready_handoff() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: PR ready for parent handoff. Accepted no-change rationale documented for thread PRRT_open.\n",
        &pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{"id":"PRRT_open","isResolved":false,"isOutdated":false,"path":"src/validation/child_handoff_readiness.rs","comments":{"nodes":[{"url":"https://github.com/eunsoogi/codexy/pull/226#discussion_r1"}]}}]}"###,
        ),
    )?;
    assert_success(output);
    Ok(())
}

#[test]
fn validator_ignores_ordered_unchecked_ready_checklist_items() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff:\n1. [ ] PR-ready after review.\nWaiting on review.\n",
        &pr_state_with(
            r###""mergeStateStatus":"DIRTY","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example [ahead 1]","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
    )?;
    assert_success(output);
    Ok(())
}

fn assert_success(output: std::process::Output) {
    assert!(
        output.status.success(),
        "unexpected stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn pr_state_with(fields: &str) -> String {
    format!(
        r#"{{"number":204,"state":"OPEN","isDraft":false,"reviewDecision":"APPROVED","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`","author":{{"login":"chatgpt-codex-connector"}},"submittedAt":"2026-07-03T00:00:00Z"}}],{fields}}}"#
    )
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
