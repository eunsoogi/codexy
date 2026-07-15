use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const HEAD: &str = "068dbb247b7755035223c91ee39f26830f3c1609";

#[test]
fn validator_requires_thread_evidence_for_ready_sentence_aliases() -> TestResult {
    for alias in [
        "PR is ready.",
        "pull request is ready.",
        "ready for handoff.",
        "parent-handoff-ready.",
        "parent handoff ready.",
        "parent can open PR next: yes.",
        "parent can merge.",
    ] {
        let handoff = format!(
            "Branch clean and pushed at {HEAD}. Remote/PR head match: yes {HEAD}. {alias}\n"
        );
        let missing = validate(&handoff, missing_review_threads())?;
        assert!(
            !missing.status.success(),
            "alias escaped readiness: {alias}"
        );
        let missing_error = String::from_utf8_lossy(&missing.stderr);
        assert!(
            missing_error.contains("reviewThreads") && missing_error.contains("missing"),
            "unexpected missing-state error for {alias:?}: {}",
            missing_error
        );

        let unresolved = validate(&handoff, unresolved_review_thread())?;
        assert!(
            !unresolved.status.success(),
            "alias escaped readiness: {alias}"
        );
        assert!(
            String::from_utf8_lossy(&unresolved.stderr).contains("PRRT_issue426"),
            "unexpected unresolved-state error for {alias:?}: {}",
            String::from_utf8_lossy(&unresolved.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_ignores_non_current_readiness_prose() -> TestResult {
    for handoff in [
        "- [ ] PR is ready.\n",
        "Historical example: PR is ready. Current lane is not ready.\n",
        "Fallback lane: PR is ready. Current lane is not ready.\n",
        "Historical example: PR is ready! Current lane is not ready.\n",
        "Historical example: PR is ready? Current lane is not ready.\n",
        "Historical example no. 1: PR is ready! Current lane is not ready.\n",
        "Historical example no. 1: PR is ready? Current lane is not ready.\n",
        "## PR readiness\n",
        "PR ready\n1. missing status evidence\n",
        "PR ready\n- missing status evidence\n",
        "PR ready\n+ missing status evidence\n",
        "PR ready\n- [ ] missing status evidence\n",
        "PR ready\n1. [ ] missing status evidence\n",
        "Example: PR is ready. Current lane is not ready.\n",
        "Example 1: PR is ready. Current lane is not ready.\n",
        "Historical example no. 1: PR is ready. Current lane is not ready.\n",
        "1) Historical example: PR is ready. Current lane is not ready.\n",
        "1) Example: PR is ready. Current lane is not ready.\n",
        "1) Fallback lane: PR is ready. Current lane is not ready.\n",
        "1) Historical example:\nPR is ready.\n\nCurrent lane is not ready.\n",
        "## Historical example\nPR is ready.\n",
        "Historical example:\nPR is ready.\n",
        "Fallback lane:\nPR is ready.\n",
        "For example, PR is ready. Current lane is not ready.\n",
    ] {
        let output = validate(handoff, missing_review_threads())?;
        assert!(
            output.status.success(),
            "non-current readiness text was treated as a claim {handoff:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_resumes_current_lane_after_stale_example_label() -> TestResult {
    for handoff in [
        "Historical example:\nPR is ready.\nCurrent lane: PR is ready.\n",
        "Historical example: PR is ready. Current lane: PR is ready.\n",
        "Historical example: PR is ready! Current lane: PR is ready.\n",
        "Historical example: PR is ready? Current lane: PR is ready.\n",
    ] {
        let output = validate(handoff, missing_review_threads())?;
        assert!(
            !output.status.success(),
            "explicit current-lane readiness was hidden by stale example scope: {handoff:?}"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("reviewThreads"));

        let unresolved = validate(handoff, unresolved_review_thread())?;
        assert!(
            !unresolved.status.success(),
            "explicit current-lane readiness bypassed unresolved thread: {handoff:?}"
        );
        assert!(String::from_utf8_lossy(&unresolved.stderr).contains("PRRT_issue426"));
    }
    Ok(())
}

fn validate(
    handoff: &str,
    pr_state: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn validate_completion_handoff(
    handoff_path: &Path,
    pr_state_path: &Path,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
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

fn missing_review_threads() -> &'static str {
    r###"{"number":426,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example"}"###
}

fn unresolved_review_thread() -> &'static str {
    r###"{
        "number": 426,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "headRefName": "codexy/example",
        "headRefOid": "068dbb247b7755035223c91ee39f26830f3c1609",
        "localHeadOid": "068dbb247b7755035223c91ee39f26830f3c1609",
        "remoteHeadOid": "068dbb247b7755035223c91ee39f26830f3c1609",
        "worktreeStatus": "## codexy/example...origin/codexy/example",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[{
            "id":"PRRT_issue426",
            "isResolved":false,
            "isOutdated":false,
            "path":"src/validation/review_thread_readiness.rs",
            "comments":{"nodes":[{"url":"https://github.com/eunsoogi/codexy/pull/426#discussion_r1"}]}
        }]}
    }"###
}
