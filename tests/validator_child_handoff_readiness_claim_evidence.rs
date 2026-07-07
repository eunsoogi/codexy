use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_clean_yes_handoff_with_dirty_status() -> TestResult {
    for handoff in [
        "Child handoff: Clean: yes. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        "Child handoff: branch is clean.\n",
        "Child handoff: worktree is clean.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example\n M src/validation/child_handoff_readiness_claims.rs","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "current status is dirty",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_synced_pushed_handoff_with_pr_blockers() -> TestResult {
    for (fields, needle) in [
        (
            r###""mergeStateStatus":"DIRTY","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            "mergeStateStatus is DIRTY",
        ),
        (
            r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{"id":"PRRT_open","isResolved":false,"isOutdated":false,"path":"src/validation/child_handoff_readiness.rs","comments":{"nodes":[{"url":"https://github.com/eunsoogi/codexy/pull/226#discussion_r1"}]}}]}"###,
            "unresolved review thread",
        ),
    ] {
        assert_rejects_child_handoff(
            "Child handoff: branch clean, synced, and pushed at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
            &pr_state_with(fields),
            needle,
        )?;
    }
    Ok(())
}

#[test]
fn validator_treats_no_blockers_as_readiness_claim() -> TestResult {
    for handoff in [
        "Child handoff: PR ready: no blockers.\n",
        "Child handoff: parent can open PR next.\n",
        "PR ready\n- no blockers\n",
        "Child handoff: parent-handoff-ready: yes.\n",
        "parent-handoff-ready: yes.\n",
        "Parent handoff ready: yes.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example\n M src/validation/child_handoff_readiness_text.rs","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "current status is dirty",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_bare_pr_ready_handoff_without_status_evidence() -> TestResult {
    for handoff in [
        "PR-ready.\n",
        "merge-ready.\n",
        "- PR-ready.\n",
        "* merge-ready.\n",
        "Ready to merge.\n",
        "Ready for merge.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "git status evidence is missing",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_affirmative_ready_labels_without_child_marker() -> TestResult {
    for handoff in [
        "PR-ready: yes.\n",
        "Merge-ready: yes.\n",
        "PR-ready: yes, no blockers.\n",
        "Merge-ready: yes; parent owns merge.\n",
        "Pull request ready: yes.\n",
        "Ready to merge: yes.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example\n M src/validation/child_handoff_readiness.rs","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "current status is dirty",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_synced_yes_with_pushed_no() -> TestResult {
    for handoff in [
        "Child handoff: Synced: yes at 068dbb247b7755035223c91ee39f26830f3c1609. Pushed: no.\n",
        "Child handoff: Synced: yes at 068dbb247b7755035223c91ee39f26830f3c1609. Pushed branch: no.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "pushed proof is negative or non-claim",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_with_negative_parent_handoff_label() -> TestResult {
    for handoff in [
        "Child handoff: PR-ready: yes. Parent-handoff-ready: no.\n",
        "Child handoff: PR-ready: yes. Parent handoff ready: no.\n",
    ] {
        assert_rejects_child_handoff(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
            "PR-ready proof is negative or non-claim",
        )?;
    }
    Ok(())
}

#[test]
fn validator_allows_pushed_branch_no_blocker_handoff() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: Pushed branch: no. Waiting on push before parent handoff.\n",
        &pr_state_with(
            r###""mergeStateStatus":"DIRTY","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example [ahead 1]","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
        ),
    )?;
    assert!(output.status.success(), "blocker handoff should pass");
    Ok(())
}

#[test]
fn validator_allows_not_yet_readiness_blocker_labels() -> TestResult {
    for handoff in [
        "Child handoff: Pushed: not yet. Waiting on push before parent handoff.\n",
        "Child handoff: Synced: not yet. Waiting on branch sync before parent handoff.\n",
        "Child handoff: PR-ready: not yet. Waiting on review before parent handoff.\n",
        "Child handoff: Pushed: not currently. Waiting on push before parent handoff.\n",
        "Child handoff: Synced: not currently. Waiting on branch sync before parent handoff.\n",
        "Child handoff: PR-ready: not currently. Waiting on review before parent handoff.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"DIRTY","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example [ahead 1]","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(
            output.status.success(),
            "blocker label should not claim readiness for {handoff}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_readiness_blocker_headings() -> TestResult {
    for handoff in [
        "Child handoff:\nPR readiness blockers:\n- Required proof is absent.\n",
        "Child handoff:\nMerge readiness pending:\n- Required merge proof is absent.\n",
        "Child handoff:\nReady for parent handoff blockers:\n- Current-head proof is absent.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            &pr_state_with(
                r###""mergeStateStatus":"DIRTY","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example [ahead 1]","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}"###,
            ),
        )?;
        assert!(
            output.status.success(),
            "blocker heading should not claim readiness for {handoff}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn assert_rejects_child_handoff(handoff: &str, pr_state: &str, needle: &str) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, pr_state)?;
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
