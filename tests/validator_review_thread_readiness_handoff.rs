use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_ready_handoff_with_accepted_no_change_rationale() -> TestResult {
    for (handoff, pr_state) in [
        (
            "Review response: addressed and verified current head. Accepted no-change rationale documented for thread PRRT_kwDOExample. Codex review passed on the current head. PR is merge-ready.\n",
            unresolved_accepted_thread_ready_pr_state(),
        ),
        (
            "Review response: addressed and verified current head. Accepted no-change rationale documented for thread PRRT_kwDOExample. Maintainer override: yes. PR is merge-ready.\n",
            unresolved_accepted_thread_override_pr_state(),
        ),
        (
            "Review response: addressed and verified current head. Accepted no-change rationale documented for thread PRRT_kwDOOutdated. Maintainer override: yes. PR is merge-ready.\n",
            unresolved_accepted_outdated_thread_override_pr_state(),
        ),
    ] {
        let output = validate_handoff_with_pr_state(handoff, pr_state)?;
        assert!(
            output.status.success(),
            "validator should honor accepted no-change rationale before blocking readiness\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_override_ready_handoff_without_review_thread_evidence() -> TestResult {
    let handoff =
        "Maintainer override: yes. PR is merge-ready after review-thread cleanup verification.\n";
    for (pr_state, expected) in [
        (
            override_ready_missing_review_threads_pr_state(),
            "missing reviewThreads",
        ),
        (
            override_ready_paginated_review_threads_pr_state(),
            "pagination hasNextPage true",
        ),
    ] {
        let output = validate_handoff_with_pr_state(handoff, pr_state)?;
        assert!(
            !output.status.success(),
            "validator should require complete reviewThreads evidence before override readiness claims\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "expected {expected:?} in stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_readiness_aliases_without_review_thread_evidence() -> TestResult {
    for handoff in [
        "Maintainer override: yes. PR-readiness evidence: all gates passed.\n",
        "Maintainer override: yes. PR readiness evidence: all gates passed.\n",
        "Maintainer override: yes. merge-readiness evidence: all gates passed.\n",
        "Maintainer override: yes. merge readiness evidence: all gates passed.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            override_ready_missing_review_threads_pr_state(),
        )?;
        assert!(
            !output.status.success(),
            "validator should require reviewThreads evidence for readiness alias handoff {handoff:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("missing reviewThreads"),
            "expected missing reviewThreads in stderr for handoff {handoff:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_readiness_aliases_with_unresolved_threads() -> TestResult {
    for handoff in [
        "Maintainer override: yes. PR-readiness evidence: all gates passed.\n",
        "Maintainer override: yes. PR readiness evidence: all gates passed.\n",
        "Maintainer override: yes. merge-readiness evidence: all gates passed.\n",
        "Maintainer override: yes. merge readiness evidence: all gates passed.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, unresolved_alias_ready_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject readiness alias handoff {handoff:?} while review threads remain unresolved\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOAlias"),
            "expected unresolved thread id in stderr for handoff {handoff:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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

fn unresolved_accepted_thread_ready_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "latestReviews": [{
            "body": "Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
            "author": {"login":"chatgpt-codex-connector"},
            "submittedAt":"2026-06-22T12:50:03Z"
        }],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[{
            "id": "PRRT_kwDOExample",
            "isResolved": false,
            "isOutdated": false,
            "path": "plugins/codexy/skills/git-workflow/SKILL.md",
            "comments": {"nodes": [{
                "author": {"login":"reviewer"},
                "url": "https://github.com/eunsoogi/codexy/pull/133#discussion_r1"
            }]}
        }]}
    }"#
}

fn unresolved_accepted_thread_override_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[{
            "id": "PRRT_kwDOExample",
            "isResolved": false,
            "isOutdated": false,
            "path": "plugins/codexy/skills/git-workflow/SKILL.md",
            "comments": {"nodes": [{
                "author": {"login":"reviewer"},
                "url": "https://github.com/eunsoogi/codexy/pull/133#discussion_r1"
            }]}
        }]}
    }"#
}

fn unresolved_accepted_outdated_thread_override_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[{
            "id": "PRRT_kwDOOutdated",
            "isResolved": false,
            "isOutdated": true,
            "path": "plugins/codexy/skills/git-workflow/SKILL.md",
            "comments": {"nodes": [{
                "url": "https://github.com/eunsoogi/codexy/pull/133#discussion_r3"
            }]}
        }]}
    }"#
}

fn override_ready_missing_review_threads_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED"
    }"#
}

fn override_ready_paginated_review_threads_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":true},"nodes":[]}
    }"#
}

fn unresolved_alias_ready_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "repository": "eunsoogi/codexy",
        "labels": [{"name":"type/fix"},{"name":"status/review"}],
        "closingIssuesReferences": [{"number":266,"labels":[{"name":"type/fix"},{"name":"status/review"}]}],
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[{
            "id": "PRRT_kwDOAlias",
            "isResolved": false,
            "isOutdated": false,
            "path": "plugins/codexy/skills/git-workflow/SKILL.md",
            "comments": {"nodes": [{
                "author": {"login":"reviewer"},
                "url": "https://github.com/eunsoogi/codexy/pull/133#discussion_r2"
            }]}
        }]}
    }"#
}
