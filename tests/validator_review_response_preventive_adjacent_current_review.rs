use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_embedded_blocker_phrases_without_preventive_review() -> TestResult {
    for handoff in [
        "Review response: fixed the exact Codex review comment. Unblocked on CI after rerun.\n",
        "Review response: fixed the exact Codex review comment. Unblocked by review fixtures after rerun.\n",
        "Review response: fixed the exact Codex review comment. Preblocked on CI wording was removed.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_allows_current_free_text_blocker_without_preventive_review() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the exact Codex review comment. Blocked on current maintainer confirmation before handoff.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert_success(
        &output,
        "validator should preserve true free-text blocker phrases",
    );
    Ok(())
}

#[test]
fn validator_allows_preventive_coverage_before_follow_up_labels() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review:\nTests: focused regression coverage exercises adjacent parser variants in the touched helper family.\nFollow-ups: not applicable to this parser.\n",
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review:\nTests: focused regression coverage exercises adjacent parser variants in the touched helper family.\nFollow-up: not applicable to this parser.\n",
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review:\nTests: focused regression coverage exercises adjacent parser variants in the touched helper family.\nStatus: not applicable to this parser.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should not treat unrelated labels as preventive contradictions",
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_uncovered_adjacent_coverage_checks() -> TestResult {
    for handoff in [
        "Review response: fixed the exact Codex review comment. Preventive adjacent review: regression coverage checks show adjacent parser variants in the helper family are uncovered.\n",
        "Review response: fixed the exact Codex review comment. Preventive adjacent review: focused tests checked adjacent parser variants in the helper family; adjacent variants remain uncovered.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_allows_positive_adjacent_coverage_checks() -> TestResult {
    for handoff in [
        "Review response: fixed the exact Codex review comment. Preventive adjacent review: regression coverage checks passed for adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment. Preventive adjacent review: focused tests checked adjacent parser variants in the helper family and passed.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should allow positive adjacent coverage checks",
        );
    }
    Ok(())
}

#[test]
fn validator_allows_current_blocker_after_historical_label_context() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Blockers: previous CI failure resolved; current blocker is maintainer approval. Review response: fixed the exact Codex review comment.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert_success(
        &output,
        "validator should preserve current blocker labels after historical context",
    );
    Ok(())
}

#[test]
fn validator_rejects_prepositional_no_blocker_labels() -> TestResult {
    for handoff in [
        "Blocked on: none. Review response: fixed the exact Codex review comment.\n",
        "Blocked by: none. Review response: fixed the exact Codex review comment.\n",
        "Blocked due to: no open blockers. Review response: fixed the exact Codex review comment.\n",
        "Waiting on: none. Review response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_rejects_empty_status_headings_before_review_response() -> TestResult {
    for handoff in [
        "Waiting:\nStatus: current head verified.\nReview response: fixed the exact Codex review comment.\n",
        "Blockers:\nFollow-ups: none.\nReview response: fixed the exact Codex review comment.\n",
        "Waiting:\nFollow-up: no remaining action.\nReview response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
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

fn assert_rejects_preventive_adjacent(output: &std::process::Output, handoff: &str) {
    assert!(
        !output.status.success(),
        "validator should reject missing preventive adjacent evidence\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
        handoff,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"));
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
