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
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions and tests; invariants hold because sibling parser variants share the same boundary checks.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions parse_review_threads and tests; invariants hold because sibling parser variants share the same boundary checks.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions and tests validator_review_response_preventive_adjacent; invariants hold because sibling parser variants share the same boundary checks.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should require both inspected code and test surfaces\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_semicolon_separated_adjacent_coverage() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused tests cover the exact comment only; regression coverage exercises adjacent parser variants in the helper family.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        output.status.success(),
        "validator should allow semicolon-separated adjacent coverage\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_no_change_rationale_with_code_and_coverage_surface() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected code surface parse_review_threads and regression coverage validator_review_response_preventive_adjacent; invariants hold because sibling parser variants share the same boundary checks.\n",
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review no-change rationale:\nFunctions: inspected parse_review_threads and sibling parser variants.\nTests: inspected validator_review_response_preventive_adjacent coverage.\nInvariants hold because sibling parser variants share the same boundary checks.\n",
        "Review response: fixed the Codex review comment and verified current head.\n## Preventive adjacent review no-change rationale\n\nFunctions: inspected parse_review_threads and sibling parser variants.\nTests: inspected validator_review_response_preventive_adjacent coverage.\nInvariants hold because sibling parser variants share the same boundary checks.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            output.status.success(),
            "validator should allow concrete code plus coverage rationale\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
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
fn validator_rejects_bullet_feedback_after_empty_waiting_or_blockers() -> TestResult {
    for handoff in [
        "Waiting:\nReview feedback:\n- fixed the exact Codex review comment and verified current head.\n",
        "Blockers:\nCodex feedback:\n- handled the exact Codex review comment and verified current head.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should not treat empty waiting/blocker headings as incomplete evidence\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"));
    }
    Ok(())
}

#[test]
fn validator_rejects_adverbial_coverage_omission() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused tests intentionally omit adjacent parser variants in the helper family.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject omitted adjacent coverage even with an adverb\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_borrowed_preventive_evidence_from_feedback_sections() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review: no focused regression coverage was added.\nCodex feedback: regression coverage exercises adjacent parser variants in the helper family.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should not borrow preventive evidence from later feedback sections\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"));
    Ok(())
}

#[test]
fn validator_rejects_no_change_rationale_with_uninspected_tests() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions parse_review_threads; tests not inspected; invariants hold because sibling parser variants share the same boundary checks.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions parse_review_threads; tests were not inspected; invariants hold because sibling parser variants share the same boundary checks.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions parse_review_threads; coverage not inspected; invariants hold because sibling parser variants share the same boundary checks.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject no-change rationales with uninspected test surface\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
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

#[test]
fn validator_rejects_stale_blocker_mentions_without_preventive_review() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. This was blocked on cargo earlier, now cleared.\n",
        "Review response: fixed the Codex review comment and verified current head. Historical blocker: blocked by CI earlier, now resolved.\n",
        "Review response: fixed the Codex review comment and verified current head. Previously blocked due to fixtures; blocker cleared now.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject stale blocker mentions without preventive review\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("preventive adjacent review"));
    }
    Ok(())
}

#[test]
fn validator_rejects_future_preventive_coverage_claims() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression coverage is planned for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused tests will run for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression tests to run later for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: plan to run regression tests for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression tests will cover adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject future preventive coverage claims\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
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
