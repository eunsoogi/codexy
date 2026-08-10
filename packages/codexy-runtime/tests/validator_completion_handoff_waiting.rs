
type TestResult = Result<(), Box<dyn std::error::Error>>;

const OPEN_PR_STATE: &str =
    r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN"}"#;

#[test]
fn validator_rejects_non_blocking_waits_described_as_blocked() -> TestResult {
    for handoff in [
        "Goal blocked because child-thread work is still pending.",
        "Blocked: child thread verification is still pending.",
        "Blocker: queued worktree setup has not completed yet.",
        "Blocked on asynchronous tool completion.",
        "Blocked: async tool has not returned yet.",
        "Blocked while waiting for parent authorization.",
        "Blocked while waiting for dependency integration.",
        "Blocked while waiting for a resource slot.",
        "Blocked while waiting for a Sentinel result.",
        "Blocked while waiting for CI completion.",
        "Blocked while waiting for connector review.",
        "Blocked after repeated true impasse: cannot make meaningful progress without maintainer input.",
        "Blocked after repeated true impasse because an external state change is required.",
    ] {
        let output = validate(handoff)?;
        assert!(
            !output.status.success(),
            "handoff unexpectedly passed: {handoff}"
        );
        assert!(stderr(&output).contains("waiting state"));
    }
    Ok(())
}

#[test]
fn validator_preserves_real_blockers() -> TestResult {
    for handoff in [
        "Blocked: review feedback requested changes remain unresolved.",
        "Blocked: required status checks are failing.",
        "Blocked: child thread omitted required goal tool evidence.",
        "Blocked: worktree setup failed with an invalid reference.",
        "Blocked: async tool failed authentication.",
    ] {
        let output = validate(handoff)?;
        assert!(
            output.status.success(),
            "real blocker was rejected: {handoff}\n{}",
            stderr(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_preserves_true_impasse() -> TestResult {
    let output = validate(
        "Blocked by an unanswered user decision that materially changes the result; no safe default and no in-scope action exist.",
    )?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

fn validate(handoff: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, OPEN_PR_STATE)?;
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
