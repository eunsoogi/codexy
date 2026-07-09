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
        "Waiting:\nVerification:\ncargo test passed.\nReview response: fixed the exact Codex review comment.\n",
        "Blockers:\nTests:\ncargo test passed.\nReview response: fixed the exact Codex review comment.\n",
        "Waiting:\nSentinel:\nno blockers.\nReview response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_rejects_requirement_templates_as_preventive_evidence() -> TestResult {
    for handoff in [
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review evidence: include focused preventive regression coverage for any adjacent gap found.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: child handoff must include regression coverage for adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: required focused regression tests cover adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review checklist: regression coverage covers adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: focused regression tests should cover adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: regression coverage needs to cover adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: focused tests should run for adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review checklist: checked regression coverage covers adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review template checked: regression coverage covers adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review requirement: checked regression coverage covers adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_rejects_modal_preventive_coverage_claims() -> TestResult {
    for handoff in [
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: focused regression tests can cover adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: focused regression tests could cover adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: regression coverage may cover adjacent parser variants in the helper family.\n",
        "Review response: fixed the exact Codex review comment and verified current head. Preventive adjacent review: focused tests might exercise adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_allows_executed_required_preventive_coverage() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: required regression tests passed for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: required focused tests ran for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: required regression coverage was executed for adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: regression coverage exercises not applicable readiness labels across adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(&output, "validator should allow executed required coverage");
    }
    Ok(())
}

#[test]
fn validator_allows_later_preventive_evidence_sections() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review evidence below.\n\n## Preventive adjacent review\nFocused regression coverage exercises adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review evidence below.\n\nPreventive adjacent review:\nTests: focused regression coverage exercises adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review summary follows later.\n\nPreventive adjacent review evidence:\n- Focused regression coverage exercises adjacent parser variants in the helper family.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_success(
            &output,
            "validator should scan later preventive evidence sections",
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_contradictory_later_preventive_sections() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review was not performed.\n\n## Preventive adjacent review\nFocused regression coverage exercises adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review isn't applicable.\n\n## Preventive adjacent review\nFocused regression coverage exercises adjacent parser variants in the helper family.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review: focused regression coverage exercises adjacent parser variants in the helper family.\n\n## Preventive adjacent review\nPreventive adjacent review was not performed.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review no-change rationale:\nFunctions: parser_variant_guard\nTests: validator_review_response_preventive_adjacent_edges\nInvariants hold because sibling parser variants share the same boundary checks.\n\n## Preventive adjacent review\nNot applicable.\n",
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review evidence:\n- Focused regression coverage exercises adjacent parser variants in the helper family.\n\nPreventive adjacent review isn't applicable.\n",
        "Review response: fixed the Codex review comment and verified current head.\n\n## Preventive adjacent review\nFocused regression coverage exercises adjacent parser variants in the helper family.\n\nNot applicable.\n",
        "Review response: fixed the Codex review comment and verified current head.\n\n## Preventive adjacent review no-change rationale\nFunctions: parser_variant_guard\nTests: validator_review_response_preventive_adjacent_edges\nInvariants hold because sibling parser variants share the same boundary checks.\n\nWas not performed.\n",
        "Review response: fixed the Codex review comment and verified current head.\n\nPreventive adjacent review evidence:\n- Focused regression coverage exercises adjacent parser variants in the helper family.\n\nIsn't applicable.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_rejects_historical_waiting_labels_without_preventive_review() -> TestResult {
    for handoff in [
        "Waiting: pending Codex review earlier, now resolved. Review response: fixed the exact Codex review comment.\n",
        "Waiting: pending child reroute previously, now cleared. Review response: fixed the exact Codex review comment.\n",
        "Blocked: pending CI earlier, now resolved. Review response: fixed the exact Codex review comment.\n",
        "Blocker: pending CI previously, now cleared. Review response: fixed the exact Codex review comment.\n",
        "Waiting: pending Codex review earlier, resolved. Review response: fixed the exact Codex review comment.\n",
        "Blocker: pending CI previously, cleared. Review response: fixed the exact Codex review comment.\n",
        "Waiting: pending Codex review previously resolved; not currently pending. Review response: fixed the exact Codex review comment.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, resolved_review_thread_pr_state())?;
        assert_rejects_preventive_adjacent(&output, handoff);
    }
    Ok(())
}

#[test]
fn validator_allows_current_pending_waiting_after_stale_pending_context() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Waiting: pending Codex review previously resolved; current pending maintainer confirmation. Review response: fixed the exact Codex review comment.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert!(
        output.status.success(),
        "validator should preserve current pending waiting labels\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
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
