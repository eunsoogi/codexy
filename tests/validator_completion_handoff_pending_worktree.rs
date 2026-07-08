use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_cli_rejects_unresolved_pending_worktree_ids() -> TestResult {
    for handoff in [
        "create_thread returned pendingWorktreeId local:3ae71d60-490a-41e3-a9b3-b76c36f47186.\nNo thread surfaced in list_threads searches.\n",
        "Pending worktree id local:88841d87-c470-48b5-9df6-b168eb049339 is still not visible after branch and PR searches.\n",
        "create_thread returned pendingWorktreeId local:edge. Thread surfaced.\n",
        "create_thread returned pendingWorktreeId local:edge. Thread id 019f-child surfaced; owner thread unknown.\n",
        "create_thread returned pendingWorktreeId local:edge. Bounded wait ended, safe retry is allowed.\n",
        "create_thread returned pendingWorktreeId local:edge. Bounded wait ended; safe retry is allowed. Metadata: branch codexy/291, issue #291, SHA abc123, review-thread id O8K31.\n",
        "create_thread returned pendingWorktreeId local:edge. Setup failed.\n",
        "create_thread returned pendingWorktreeId local:edge. Setup failed with no actionable error surfaced.\n",
        "create_thread returned pendingWorktreeId local:edge. Worktree setup is missing.\n",
        "create_thread returned pendingWorktreeId local:edge. Worktree setup missing, no actionable error surfaced.\n",
        "create_thread returned pendingWorktreeId local:edge. Thread setup failed because branch creation is still pending.\n",
    ] {
        let output = validate_open_pr_handoff(handoff)?;
        assert!(
            !output.status.success(),
            "validator should reject pending worktree ids without surfaced, failed, or bounded-timeout state\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("pending worktree"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

#[test]
fn validator_cli_allows_resolved_pending_worktree_ids() -> TestResult {
    for handoff in [
        "create_thread returned pendingWorktreeId local:3ae71d60-490a-41e3-a9b3-b76c36f47186. Surfaced thread id 019f-child was observed and active lane accounting state is active.\n",
        "Pending worktree id local:88841d87-c470-48b5-9df6-b168eb049339 reached bounded timeout state after list_threads searches by pending id, branch, PR, SHA, and review-thread id; active lane accounting state is not-surfaced-after-bounded-wait and safe retry/reassignment is allowed.\n",
        "Pending worktree id local:88841d87-c470-48b5-9df6-b168eb049339 failed setup with fatal invalid reference; active lane accounting state is failed and retry requires corrected base ref.\n",
    ] {
        accept_open_pr_handoff(
            handoff,
            "validator should allow pending worktree ids with explicit surfaced, failed, or bounded-timeout accounting",
        )?;
    }
    Ok(())
}

fn accept_open_pr_handoff(handoff: &str, failure_message: &str) -> TestResult {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        output.status.success(),
        "{failure_message}\nhandoff:\n{handoff}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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

fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    validate_handoff_with_pr_state(
        handoff,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","latestReviews":[{"body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-06-22T12:50:03Z"}],"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"#,
    )
}
