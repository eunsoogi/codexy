use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_exact_comment_only_review_response_handoff() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the exact Codex review comment and verified current head.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject exact-comment-only review-response handoff without preventive adjacent review evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_exact_comment_only_handoff_with_negated_unresolved_threads() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the exact Codex review comment and verified current head. No review thread remains unresolved.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should not treat negated unresolved-thread wording as incomplete evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_exact_comment_only_handoff_with_no_blockers_heading() -> TestResult {
    for handoff in [
        "Blockers: none. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blocker: none. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blocked: none. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blockers: 0. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blockers: zero. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blockers:\n- none\nReview response: fixed the exact Codex review comment and verified current head.\n",
        "Blockers: no blockers. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blocked? no. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blocked - none. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blocker? no. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Blockers - none. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Readiness: not complete.\nReview response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused regression coverage exercises adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should not treat no-blocker headings as incomplete evidence\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"),
            "unexpected stderr for handoff:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
#[test]
fn validator_allows_real_blocker_state_without_preventive_adjacent_review() -> TestResult {
    for handoff in [
        "Blocker: upstream review-thread evidence is unavailable, so this review-response lane is not complete.\nReview response: fixed the exact Codex review comment.\n",
        "Blocked: no current review yet. Review response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should allow true blocker state evidence",
        );
    }
    Ok(())
}
#[test]
fn validator_allows_preventive_adjacent_regression_coverage() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused regression tests exercise adjacent parser variants in the touched helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression tests were added for adjacent parser variants in the touched helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression coverage was added for adjacent parser variants in the touched helper family.\n",
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review:\nTests: focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review\nTests: focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
        "Review response: fixed the Codex review comment and verified current head.\n## Preventive adjacent review\n\nTests: focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review\nFunctions: inspected adjacent parser variants in the touched helper family.\nTests: focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should allow preventive adjacent coverage",
        );
    }
    Ok(())
}

#[test]
fn validator_allows_preventive_adjacent_section_bullets() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review:\n- Focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert_success(
        &output,
        "validator should scan preventive adjacent section bullets",
    );
    Ok(())
}

#[test]
fn validator_rejects_negated_preventive_adjacent_regression_coverage() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: no focused regression coverage for adjacent parser variants in the touched helper family.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject negated preventive adjacent coverage claims\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_post_negated_preventive_adjacent_coverage() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; focused regression coverage is not needed.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused regression coverage for adjacent parser variants is missing.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject post-negated coverage\nhandoff:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}
#[test]
fn validator_allows_preventive_adjacent_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions parse_review_threads and tests validator_review_response_preventive_adjacent; invariants hold because none of the sibling parser variants bypass this boundary.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert_success(
        &output,
        "validator should allow concrete preventive no-change rationale",
    );
    Ok(())
}

#[test]
fn validator_rejects_not_applicable_preventive_adjacent_no_change_rationale() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions parse_review_threads and tests validator_review_response_preventive_adjacent; invariants hold because not applicable.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject not-applicable preventive no-change rationale\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"),
        "unexpected stderr: {}",
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

fn assert_success(output: &std::process::Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn resolved_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},"nodes":[]}
    }"#
}
