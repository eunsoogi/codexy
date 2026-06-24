use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Output = std::process::Output;
type OutputResult = Result<Output, Box<dyn std::error::Error>>;

const STALE_DUPLICATE_STATE: &str = "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.";
const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";
const PR_STATE: &str = r#"{"number":172,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","closingIssuesReferences":[{"number":171}]}"#;
const PR_STATE_WITH_GRAPHQL_ISSUES: &str = r#"{"number":172,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","closingIssuesReferences":{"nodes":[{"number":171}]}}"#;

#[test]
fn validator_cli_rejects_duplicate_state_for_stale_pr_number() -> TestResult {
    let output = validate_handoff_with_default_pr_state(&format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         {STALE_DUPLICATE_STATE}\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Next action: stop.\n"
    ))?;
    assert!(
        !output.status.success(),
        "validator should reject handoff\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("compacted continuation evidence missing duplicate/no-active-work state"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_duplicate_state_for_current_linked_issue_array() -> TestResult {
    let output = validate_handoff_with_pr_state(
        &valid_handoff_with_duplicate_state(
            "Duplicate/no-active-work state: issue #171 is duplicate/no-active-work after current GitHub state re-check.",
        ),
        PR_STATE,
    )?;
    assert!(
        output.status.success(),
        "validator should accept handoff\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_duplicate_state_for_current_linked_issue_nodes() -> TestResult {
    let output = validate_handoff_with_pr_state(
        &valid_handoff_with_duplicate_state(
            "Duplicate/no-active-work state: issue #171 is duplicate/no-active-work after current GitHub state re-check.",
        ),
        PR_STATE_WITH_GRAPHQL_ISSUES,
    )?;
    assert!(
        output.status.success(),
        "validator should accept handoff\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn valid_handoff_with_duplicate_state(duplicate_state: &str) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         {duplicate_state}\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Next action: stop.\n"
    )
}

fn validate_handoff_with_default_pr_state(handoff: &str) -> OutputResult {
    validate_handoff_with_pr_state(handoff, PR_STATE)
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
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
