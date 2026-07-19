use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_clean_or_pr_ready_handoff_without_local_status() -> TestResult {
    for (handoff, fields, needle) in [
        (
            "Child handoff: branch clean, synced, and pushed at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
            r#""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
            "local git status evidence is missing",
        ),
        (
            "Child handoff: PR-ready.\n",
            r#""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
            "local git status evidence is missing",
        ),
        (
            "Child handoff: branch clean.\n",
            r#""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
            "local git status evidence is missing",
        ),
        (
            "Child handoff: PR-ready.\n",
            r#""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"  ","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
            "local git status evidence is missing",
        ),
        (
            "Child handoff: branch clean.\n",
            r#""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"","gitStatusShort":"M src/validation/child_handoff_readiness.rs","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"#,
            "current status is dirty",
        ),
    ] {
        assert_rejects_child_handoff(handoff, pr_state_with(fields), needle)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pushed_handoff_without_comparable_local_head() -> TestResult {
    for handoff in [
        "Child handoff: branch clean. Pushed: yes. PR head 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        "Child handoff: branch clean. Pushed: yes, PR head 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        "Child handoff: branch clean. Pushed: yes, pull request head 068dbb247b7755035223c91ee39f26830f3c1609.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "any comparable handoff head",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_when_branch_status_is_ahead() -> TestResult {
    for handoff in [
        "Child handoff: PR-ready.\n",
        "Child handoff: parent can merge.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example [ahead 1]","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "current branch status is not pushed",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_when_local_head_is_stale() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean. Remote/PR head match: yes (068dbb247b7755035223c91ee39f26830f3c1609). PR ready for parent handoff; parent will handle merge gates.\n",
        pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"0","remoteHeadOid":"068dbb","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
        "current local HEAD",
    )?;
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_from_non_pr_branch() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: PR-ready.\n",
        pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## main...origin/main","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
        "current branch status does not match PR branch",
    )?;
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_without_branch_header() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: PR-ready.\n",
        pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"nothing to commit","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
        "current branch status evidence is missing",
    )?;
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_without_head_ref_name() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: PR-ready.\n",
        pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## main...origin/main","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
        "current branch status does not match PR branch",
    )?;
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_without_captured_heads() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: PR-ready.\n",
        pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
        "local HEAD evidence is missing",
    )?;
    Ok(())
}

#[test]
fn validator_allows_capitalized_pushed_head_markers() -> TestResult {
    for marker in [
        "review-feedback fixes HEAD",
        "review-feedback fixes SHA",
        "review-feedback fixes Commit",
        "feedback fixes HEAD",
    ] {
        let handoff = format!(
            "Child handoff: branch clean, synced. Pushed {marker} 068dbb247b7755035223c91ee39f26830f3c1609. Packaged Codexy Sentinel Turing: PASS on current head 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n"
        );
        let output = validate_handoff_with_pr_state(
            &handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(output.status.success(), "should allow handoff: {handoff}");
    }
    Ok(())
}

#[test]
fn validator_allows_compact_pushed_hash_labels() -> TestResult {
    for handoff in [
        "Child handoff: branch clean, synced. Pushed: yes, 068dbb247b7755035223c91ee39f26830f3c1609. Packaged Codexy Sentinel Turing: PASS on current head 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
        "Child handoff: branch clean, synced. Pushed: 068dbb247b7755035223c91ee39f26830f3c1609. Packaged Codexy Sentinel Turing: PASS on current head 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(output.status.success());
    }
    Ok(())
}

#[test]
fn validator_allows_remote_pr_head_match_hashes() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: branch clean. Remote/PR head match: yes (068dbb247b7755035223c91ee39f26830f3c1609). Packaged Codexy Sentinel Turing: PASS on current head 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
        &pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
    )?;
    assert!(output.status.success());
    Ok(())
}

#[test]
fn validator_allows_equals_style_local_head_hashes() -> TestResult {
    for handoff in [
        "Child handoff: branch clean, synced. Pushed: yes HEAD=068dbb247b7755035223c91ee39f26830f3c1609. Packaged Codexy Sentinel Turing: PASS on current head 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
        "Child handoff: branch clean, synced. local head=068dbb247b7755035223c91ee39f26830f3c1609. Packaged Codexy Sentinel Turing: PASS on current head 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(output.status.success());
    }
    Ok(())
}

fn assert_rejects_child_handoff(handoff: &str, pr_state: String, needle: &str) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, &pr_state)?;
    assert!(
        !output.status.success(),
        "validator should reject false child handoff"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(needle), "unexpected stderr: {stderr}");
    Ok(())
}

fn pr_state_with(fields: &str) -> String {
    format!(
        r#"{{"number":204,"state":"OPEN","isDraft":false,"reviewDecision":"APPROVED","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`","author":{{"login":"automated-review"}},"submittedAt":"2026-07-03T00:00:00Z"}}],{fields}}}"#
    )
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn validate_completion_handoff(handoff_path: &Path, pr_state_path: &Path) -> OutputResult {
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}
