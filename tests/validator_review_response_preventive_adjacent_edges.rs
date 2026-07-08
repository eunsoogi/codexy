use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_not_applicable_preventive_adjacent_claim() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review isn't applicable.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review was not performed.\n\n- Focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
        "PR readiness: not applicable. Review response: fixed the Codex review comment and verified current head.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject not-applicable claims without real waiting evidence\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
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
fn validator_allows_preventive_adjacent_markdown_heading_with_blank_before_bullets() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head.\n## Preventive adjacent review evidence\n\n- Focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused regression coverage exercises adjacent parser variants in the helper family; unrelated risk note says historical fixture coverage was missing before this fix.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(&output, "validator should allow positive coverage");
    }
    Ok(())
}

#[test]
fn validator_rejects_exact_comment_only_handoff_with_no_waiting_heading() -> TestResult {
    for handoff in [
        "Waiting: none. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting:\n- none\nReview response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: 0. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: zero. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: no. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: no waiting. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: none remaining. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: none active. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: resolved. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: cleared. Review response: fixed the exact Codex review comment and verified current head.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should not treat no-waiting headings as incomplete evidence\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
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
fn validator_rejects_exact_comment_only_handoff_with_negated_blocker_phrase() -> TestResult {
    for handoff in [
        "Review response: fixed the exact Codex review comment and verified current head; not blocked on anything.\n",
        "Not blocked on anything. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Review response: fixed the exact Codex review comment. Blocked on: none.\n",
        "Blockers: none remain. Review response: fixed the exact Codex review comment.\n",
        "Blockers: none now. Review response: fixed the exact Codex review comment.\n",
        "Blockers: resolved. Review response: fixed the exact Codex review comment.\n",
        "Blockers: cleared. Review response: fixed the exact Codex review comment.\n",
        "Not blocked: tests are green. Review response: fixed the exact Codex review comment.\n",
        "Unblocked: review fix applied. Review response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;

        assert!(
            !output.status.success(),
            "validator should not treat negated blocker phrases as incomplete evidence\nhandoff:\n{}\nstdout:\n{}\nstderr:\n{}",
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
fn validator_allows_real_waiting_state_without_preventive_adjacent_review() -> TestResult {
    for handoff in [
        "Waiting: upstream review-thread evidence is unavailable, so this review-response lane is not complete.\nReview response: fixed the exact Codex review comment.\n",
        "Blockers: none\nWaiting: pending Codex review.\nReview response: fixed the exact Codex review comment.\n",
        "Blocked: no active blockers\nWaiting: pending Codex review.\nReview response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should still allow true waiting state evidence",
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_colon_labeled_post_negated_preventive_coverage() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; focused regression coverage: not needed.\n",
        "Review response: fixed the Codex review comment and verified current head.\nPreventive adjacent review: adjacent parser variants inspected; no focused regression coverage was added.\nVerification: regression coverage suite passed.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression coverage covers the exact comment; adjacent parser variants were not tested.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; regression coverage not added.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; regression coverage is missing.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; nearby coverage check found regression coverage is missing for those variants.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression coverage is missing for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; focused tests are missing.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused testimony exercises adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression coveragee exercises adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; focused tests not run.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: adjacent parser variants in the helper family; focused tests not executed.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should reject post-negated preventive coverage\nhandoff:\n{}\nstderr:\n{}",
            handoff,
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("preventive adjacent review"),
            "unexpected stderr: {stderr}"
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_no_change_rationale_without_adjacent_subject() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: inspected functions foo and tests bar; invariants hold because the exact branch is unchanged.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should require adjacent evidence beyond invariant wording\nstdout:\n{}\nstderr:\n{}",
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
fn validator_rejects_negated_no_change_rationale_inspection_claims() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale: not inspected functions parse_review_threads and tests validator_review_response_preventive_adjacent; invariants hold because sibling parser variants share the same boundary checks.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        !output.status.success(),
        "validator should reject negated no-change inspection claims\nstdout:\n{}\nstderr:\n{}",
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
