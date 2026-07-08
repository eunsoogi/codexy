use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_zero_count_adjacent_coverage_claims() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression tests cover zero adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused tests cover 0 adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_rejects_exact_comment_only_handoff_with_no_pending_blockers() -> TestResult {
    for handoff in [
        "Blockers: no pending blockers. Review response: fixed the exact Codex review comment.\n",
        "Waiting: no pending waiting. Review response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_rejects_bulleted_review_feedback_after_empty_status_heading() -> TestResult {
    for handoff in [
        "Waiting:\n- Review feedback: fixed the exact Codex review comment.\n",
        "Blockers:\n- Codex feedback: handled the exact Codex review comment.\n",
        "Waiting:\n- Review response: fixed the exact Codex review comment.\n",
        "Waiting:\n+ Review feedback: fixed the exact Codex review comment.\n",
        "Blockers:\n+ Codex feedback: handled the exact Codex review comment.\n",
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
