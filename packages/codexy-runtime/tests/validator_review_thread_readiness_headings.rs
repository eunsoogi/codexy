use std::path::Path;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_readiness_blocker_headings_as_waiting_status() -> TestResult {
    for handoff in [
        "Maintainer override: yes. PR-readiness blockers: unresolved review threads remain.\n",
        "Maintainer override: yes. merge-readiness blocker: waiting on review cleanup.\n",
        "Maintainer override: yes. PR readiness: blocked pending CI.\n",
        "Maintainer override: yes. merge-readiness: waiting on review cleanup.\n",
        "Maintainer override: yes. PR readiness status: blocked pending CI.\n",
        "Maintainer override: yes. PR readiness status: not ready.\n",
        "Maintainer override: yes. merge readiness status: not currently ready.\n",
        "Maintainer override: yes. merge readiness status: not yet complete.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, missing_review_threads_pr_state())?;
        assert!(
            output.status.success(),
            "validator should not treat blocker/status heading as readiness claim {handoff:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_affirmative_readiness_blocker_status_labels() -> TestResult {
    for handoff in [
        "Maintainer override: yes. PR-readiness blockers: none.\n",
        "Maintainer override: yes. PR readiness status: ready.\n",
    ] {
        let output = validate_handoff_with_pr_state(handoff, missing_review_threads_pr_state())?;
        assert!(
            !output.status.success(),
            "validator should treat affirmative blocker/status values as readiness claims {handoff:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("missing reviewThreads"));
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
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}

fn missing_review_threads_pr_state() -> &'static str {
    r#"{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "mergeStateStatus": "CLEAN",
        "reviewDecision": "APPROVED"
    }"#
}
