use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_not_applicable_preventive_adjacent_claim() -> TestResult {
    for handoff in [
        "Review response: fixed the Codex review comment and verified current head. Preventive adjacent review isn't applicable.\n",
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
    let output = validate_handoff_with_pr_state(
        "Review response: fixed the Codex review comment and verified current head.\n## Preventive adjacent review\n\n- Focused regression coverage exercises adjacent parser variants in the touched helper family.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert_success(
        &output,
        "validator should scan preventive adjacent heading bodies after blank lines",
    );
    Ok(())
}

#[test]
fn validator_rejects_exact_comment_only_handoff_with_no_waiting_heading() -> TestResult {
    for handoff in [
        "Waiting: none. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: no. Review response: fixed the exact Codex review comment and verified current head.\n",
        "Waiting: no waiting. Review response: fixed the exact Codex review comment and verified current head.\n",
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
fn validator_allows_real_waiting_state_without_preventive_adjacent_review() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Waiting: upstream review-thread evidence is unavailable, so this review-response lane is not complete.\nReview response: fixed the exact Codex review comment.\n",
        resolved_review_thread_pr_state(),
    )?;

    assert_success(
        &output,
        "validator should still allow true waiting state evidence",
    );
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
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOResolved",
                    "isResolved": true,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {
                        "nodes": [
                            {
                                "url": "https://github.com/eunsoogi/codexy/pull/130#discussion_r3",
                                "body": "Please fix this exact parser branch."
                            }
                        ]
                    }
                }
            ]
        }
    }"#
}
