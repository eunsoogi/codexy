use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

const OPEN_PR_STATE: &str =
    r#"{"number":172,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN"}"#;

#[test]
fn validator_cli_allows_compaction_topic_status_handoff_next_action() -> TestResult {
    for handoff in [
        "Status update for PR #172: implemented the compaction validator.\n\
         Next action: wait for review per maintainer instruction.\n",
        "Status update for PR #172: implemented the compaction summary readiness trigger.\n\
         Next action: wait for review per maintainer instruction.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            output.status.success(),
            "validator should accept handoff\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_compaction_summary_next_action_without_evidence() -> TestResult {
    for handoff in [
        "Compaction summary:\n\
         Next action: edit the PR branch.\n",
        "Compaction summary:\n\
         - Goal: preserve Codexy compaction handoffs.\n\
         - Next action: edit the PR branch.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject handoff\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("compacted continuation evidence missing Codexy orchestration contract"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, OPEN_PR_STATE)?;
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
