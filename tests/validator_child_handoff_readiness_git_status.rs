use std::{path::Path, process::Command};

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
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"1111111111111111111111111111111111111111","remoteHeadOid":"1111111111111111111111111111111111111111","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
        "current local HEAD",
    )?;
    Ok(())
}

#[test]
fn validator_allows_capitalized_pushed_head_markers() -> TestResult {
    for marker in ["HEAD", "SHA", "Commit"] {
        let output = validate_handoff_with_pr_state(
            &format!(
                "Child handoff: branch clean, synced. Pushed review-feedback fixes {marker} 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n"
            ),
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(output.status.success(), "should allow {marker}");
    }
    Ok(())
}

#[test]
fn validator_allows_compact_pushed_hash_labels() -> TestResult {
    for handoff in [
        "Child handoff: branch clean, synced. Pushed: yes, 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
        "Child handoff: branch clean, synced. Pushed: 068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(
            output.status.success(),
            "should allow compact pushed hash label\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_remote_pr_head_match_hashes() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: branch clean. Remote/PR head match: yes (068dbb247b7755035223c91ee39f26830f3c1609). PR ready for parent handoff; parent will handle merge gates.\n",
        &pr_state_with(
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
    )?;
    assert!(
        output.status.success(),
        "should allow remote/pr head match hash evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_equals_style_local_head_hashes() -> TestResult {
    for handoff in [
        "Child handoff: branch clean, synced. Pushed: yes HEAD=068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
        "Child handoff: branch clean, synced. local head=068dbb247b7755035223c91ee39f26830f3c1609. PR ready for parent handoff; parent will handle merge gates.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(
            output.status.success(),
            "should allow equals-style local head evidence\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
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
        r#"{{"number":204,"state":"OPEN","isDraft":false,"reviewDecision":"APPROVED","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`","author":{{"login":"chatgpt-codex-connector"}},"submittedAt":"2026-07-03T00:00:00Z"}}],{fields}}}"#
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
