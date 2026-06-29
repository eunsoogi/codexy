use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_allows_currently_negated_completion_claims() -> TestResult {
    for handoff in [
        "Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane is not currently complete.\n",
        "Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; this lane isn't currently complete.\n",
        "Thread PRRT_kwDOWaiting remains unresolved because it is not fixed or accepted yet; we aren't currently complete.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            output.status.success(),
            "validator should allow currently-negated completion wording\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
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

fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(
        &pr_state_path,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{"id":"PRRT_kwDOWaiting","isResolved":false,"isOutdated":false,"path":"src/validation/review_thread_resolution.rs","comments":{"nodes":[{"url":"https://github.com/eunsoogi/codexy/pull/174#discussion_r2"}]}}]}}"#,
    )?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}
