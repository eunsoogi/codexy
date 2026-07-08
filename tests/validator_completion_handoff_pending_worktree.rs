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
        "pendingWorktreeId local:edge. Thread id did not surface; active owner: none.\n",
        "create_thread returned pendingWorktreeId local:edge. Bounded wait ended, safe retry is allowed.\n",
        "create_thread returned pendingWorktreeId local:edge. Bounded wait ended; safe retry is allowed. Metadata: branch codexy/291, issue #291, SHA abc123, review-thread id O8K31.\n",
        "create_thread returned pendingWorktreeId local:edge. Setup failed.\n",
        "create_thread returned pendingWorktreeId local:edge. Setup failed with no actionable error surfaced.\n",
        "create_thread returned pendingWorktreeId local:edge. Worktree setup is missing.\n",
        "create_thread returned pendingWorktreeId local:edge. Worktree setup missing, no actionable error surfaced.\n",
        "create_thread returned pendingWorktreeId local:edge. Thread setup failed because branch creation is still pending.\n",
        "pendingWorktreeId local:first surfaced thread id 019f-child with active owner. pendingWorktreeId local:second is still not visible.\n",
        "pending worktree ids local:first and local:second: first surfaced thread id 019f-child with active owner; second remains unresolved.\n",
        "pending worktree ids local:first and local:second: local:first surfaced thread id 019f-child with active owner; local:second remains unresolved.\n",
        "pending worktree ids:\n- local:first surfaced thread id 019f-child with active owner.\n- local:second remains unresolved.\n",
        "pending worktree ids:\n1. local:first surfaced thread id 019f-child with active owner.\n2. local:second remains unresolved.\n",
        "pending worktree ids:\n- local:first failed setup with fatal invalid reference.\n- local:second remains unresolved.\n",
        "pending worktree ids local:first and local:second.\nNotes: local:first surfaced thread id 019f-child with active owner; local:second remains unresolved.\n",
        "pending worktree ids local:first and local:second.\nEvidence: local:first surfaced thread id 019f-child with active owner; local:second remains unresolved.\n",
        "pending worktree ids local:first and local:second.\nReference: https://example.test/worktrees/local:first surfaced thread id 019f-child with active owner; local:second remains unresolved.\n",
        "pendingWorktreeId local:first failed setup with fatal invalid reference. pendingWorktreeId local:second has no surfaced thread yet.\n",
        "pendingWorktreeId local:first reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment is allowed. pendingWorktreeId local:second remains unresolved.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, not by branch, not by PR, not by SHA, and no review-thread id available; safe retry/reassignment is allowed.\n",
        "pendingWorktreeId: no thread surfaced after searches.\n",
        "pendingWorktreeId: none found after searches.\n",
        "pendingWorktreeId: not surfaced after bounded wait.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment: no.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe reassignment: false.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; retry/reassignment: not applicable.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry allowed: no.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment allowed: false.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; retry/reassignment allowed: not applicable.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry is not allowed.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment is allowed; unsafe to reassign would duplicate owners.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment: yes; unsafe to reassign would duplicate owners.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry = true; not safe to retry.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe reassignment - okay; unsafe reassignment would duplicate owners.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; retry/reassignment? yes; unsafe retry would duplicate owners.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; not safe to retry.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; unsafe to reassign.\n",
        "pendingWorktreeId local:edge reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; unsafe retry/reassignment would duplicate owners.\n",
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
        "pendingWorktreeId local:first surfaced thread id 019f-child with active owner. pendingWorktreeId local:second failed setup with fatal invalid reference.\n",
        "pendingWorktreeId local:first reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment is allowed. pendingWorktreeId local:second surfaced thread id 019f-child-2 with active owner.\n",
        "pending worktree ids local:first surfaced thread id 019f-child with active owner; local:second failed setup with fatal invalid reference.\n",
        "pending worktree ids local:first and local:second: first surfaced thread id 019f-child with active owner; second failed setup with fatal invalid reference.\n",
        "pending worktree ids local:3ae71d60 and local:88841d87: first surfaced thread id 019f-child with active owner; second failed setup with fatal invalid reference.\n",
        "pending worktree ids local:first and local:second: local:first surfaced thread id 019f-child with active owner; local:second failed setup with fatal invalid reference.\n",
        "pending worktree ids local:first and local:second: local:first reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment is allowed; local:second failed setup with fatal invalid reference.\n",
        "pending worktree ids:\n- local:first surfaced thread id 019f-child with active owner.\n- local:second failed setup with fatal invalid reference.\n",
        "pending worktree ids:\n1. local:first reached bounded timeout after list_threads searches by pending id, branch, PR, SHA, and review-thread id; safe retry/reassignment is allowed.\n2. local:second failed setup with fatal invalid reference.\n",
        "pending worktree ids:\n- local:first surfaced thread id 019f-child with active owner.\n2. local:second failed setup with fatal invalid reference.\n",
        "pendingWorktreeId: none\nNo pending worktree setup remains.\n",
        "pendingWorktreeId: null\nNo pending worktree setup remains.\n",
        "pendingWorktreeId: no\nNo pending worktree setup remains.\n",
        "pending worktree id = n/a\nNo pending worktree setup remains.\n",
        "pendingWorktreeId: n-a\nNo pending worktree setup remains.\n",
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
