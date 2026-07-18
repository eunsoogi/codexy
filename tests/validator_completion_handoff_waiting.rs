
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
        "Blocked after repeated true impasse: cannot make meaningful progress without maintainer input.",
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
