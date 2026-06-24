use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_waiting_evidence_that_contradicts_same_thread_action() -> TestResult {
    for action in [
        "fixed",
        "addressed",
        "implemented",
        "resolved",
        "applied",
        "handled",
        "updated",
        "responded",
        "fixes",
        "resolves",
    ] {
        let handoff = format!(
            "Review response: {action} PRRT_kwDOWaiting. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        );
        let output = validate_handoff_with_pr_state(&handoff)?;

        assert!(
            !output.status.success(),
            "validator should reject waiting evidence when the same thread is also claimed {action}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOWaiting"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_waiting_evidence_after_sentence_final_same_thread_action() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: PRRT_kwDOWaiting fixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
    )?;

    assert!(
        !output.status.success(),
        "validator should reject waiting evidence when the same thread is claimed fixed in a normal sentence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOWaiting"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_negative_pr_readiness_label_before_waiting_evidence() -> TestResult {
    for label in [
        "PR readiness: not ready",
        "PR ready: no",
        "PR ready: false",
        "PR ready: not requested",
    ] {
        let output = validate_handoff_with_pr_state(&format!(
            "{label}. Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        ))?;

        assert!(
            output.status.success(),
            "validator should treat negative PR-readiness label `{label}` as waiting evidence\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_affirmative_no_blockers_readiness_label() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR ready: no blockers. Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
    )?;

    assert!(
        !output.status.success(),
        "validator should reject affirmative no-blockers readiness while a thread is not fixed or accepted\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("PRRT_kwDOWaiting"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_ignores_action_words_inside_waiting_file_paths() -> TestResult {
    for path in ["src/fixed/review.rs", "src/updated/foo.rs", "fixed.rs"] {
        let output = validate_handoff_with_pr_state(&format!(
            "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting at {path} remains unresolved because it is not fixed or accepted yet; this lane is not complete.\n",
        ))?;

        assert!(
            output.status.success(),
            "validator should ignore review action words inside path token `{path}`\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_perfect_tense_negated_waiting_actions() -> TestResult {
    for rationale in [
        "hasn't been fixed and hasn't been accepted",
        "hasn't been addressed and hasn't been accepted",
    ] {
        let output = validate_handoff_with_pr_state(&format!(
            "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it {rationale} yet; this lane is not complete.\n",
        ))?;

        assert!(
            output.status.success(),
            "validator should treat `{rationale}` as negated waiting evidence, not a same-thread action claim\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_colon_labeled_verification_waiting_until_merge() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet. Verification completed: focused tests passed. This lane is not complete until merge.\n",
    )?;

    assert!(
        output.status.success(),
        "validator should treat colon-labeled verification evidence as waiting, not completion\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_allows_yet_been_perfect_tense_waiting_actions() -> TestResult {
    for rationale in [
        "hasn't yet been fixed and has not yet been accepted",
        "hasn't yet been addressed and has not yet been accepted",
    ] {
        let output = validate_handoff_with_pr_state(&format!(
            "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it {rationale}; this lane is not complete.\n",
        ))?;

        assert!(
            output.status.success(),
            "validator should treat `{rationale}` as complete waiting evidence, not a same-thread action claim\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_allows_long_form_perfect_tense_waiting_actions_without_yet() -> TestResult {
    for rationale in [
        "has not been fixed and has not been accepted",
        "has not been addressed and has not been accepted",
    ] {
        let output = validate_handoff_with_pr_state(&format!(
            "Review response: fixed PRRT_kwDOFixed. Thread PRRT_kwDOWaiting remains unresolved because it {rationale}; this lane is not complete.\n",
        ))?;

        assert!(
            output.status.success(),
            "validator should treat `{rationale}` as complete waiting evidence, not a same-thread action claim\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, mixed_review_thread_pr_state())?;
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

fn mixed_review_thread_pr_state() -> &'static str {
    r#"{
        "number": 174,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED",
        "reviewThreads": {"pageInfo":{"hasNextPage":false},
            "nodes": [
                {
                    "id": "PRRT_kwDOFixed",
                    "isResolved": true,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r1"}]}
                },
                {
                    "id": "PRRT_kwDOWaiting",
                    "isResolved": false,
                    "isOutdated": false,
                    "path": "src/validation/review_thread_resolution.rs",
                    "comments": {"nodes": [{"url": "https://github.com/eunsoogi/codexy/pull/174#discussion_r2"}]}
                }
            ]
        }
    }"#
}
