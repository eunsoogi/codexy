use std::error::Error;
use std::path::Path;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;
type OutputResult = Result<std::process::Output, Box<dyn Error>>;

#[test]
fn validator_rejects_no_change_rationale_missing_code_or_test_surface() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions parse_review_threads; invariants hold because sibling parser variants share the same boundary checks.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected tests validator_review_response_preventive_adjacent; invariants hold because sibling parser variants share the same boundary checks.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should require both inspected code and test surfaces\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"));
    }
    Ok(())
}

#[test]
fn validator_allows_no_change_rationale_with_code_and_coverage_surface() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected code surface parse_review_threads and regression coverage validator_review_response_preventive_adjacent; invariants hold because sibling parser variants share the same boundary checks.\n",
        resolved_review_thread_pr_state(),
    )?;
    assert!(
        output.status.success(),
        "validator should allow concrete code plus coverage rationale\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_no_waiting_no_related_prose_without_preventive_review() -> TestResult {
    for handoff in [
        "Waiting on nothing after the exact review fix.\nReview response: fixed the Codex review comment and verified current head.\n",
        "Waiting: no child reroute was needed after the exact review fix.\nReview response: fixed the Codex review comment and verified current head.\n",
        "Waiting: no related parser code was touched after the exact review fix.\nReview response: fixed the Codex review comment and verified current head.\n",
        "Waiting: no adjacent helper changes were needed after the exact review fix.\nReview response: fixed the Codex review comment and verified current head.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject no-waiting/no-related prose without preventive review\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"));
    }
    Ok(())
}

#[test]
fn validator_rejects_failed_preventive_coverage_claims() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression tests failed for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused tests are failing for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression coverage was blocked for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: no passing regression tests cover adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject failed preventive coverage claims\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"));
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
