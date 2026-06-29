use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_allows_colon_verification_waiting_without_github_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Verification completed: focused tests passed. This lane is not complete until merge.\n",
        codexy_pr_state_without_labels(),
    )?;

    assert!(
        output.status.success(),
        "validator should not treat colon verification waiting evidence as GitHub-label completion\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_still_rejects_completion_without_github_labels() -> TestResult {
    let output =
        validate_handoff_with_pr_state("Work completed.\n", codexy_pr_state_without_labels())?;

    assert!(
        !output.status.success(),
        "validator should still require label evidence for completion claims\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    Ok(())
}

fn codexy_pr_state_without_labels() -> &'static str {
    r#"{"number":177,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/174-review-thread-resolution-gate","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/177","labels":[],"closingIssuesReferences":[{"number":174,"labels":[]}]}"#
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
