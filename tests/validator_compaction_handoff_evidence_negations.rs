use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

const OPEN_PR_STATE: &str =
    r#"{"number":170,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN"}"#;
const CONTRACT: &str =
    "Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.";
const DUPLICATE_STATE: &str = "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.";
const OWNERSHIP_BOUNDARY: &str = "Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.";
const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";
const STOP_CONDITION: &str =
    "Stop condition: no merge; leave PR open until current-head Codex review is clean.";

#[test]
fn validator_cli_rejects_negated_duplicate_state_capture() -> TestResult {
    assert_missing(
        "Duplicate/no-active-work state: no duplicate/no-active-work state was captured for PR #170 after current GitHub state re-check.",
        "compacted continuation evidence missing duplicate/no-active-work state",
    )
}

#[test]
fn validator_cli_rejects_negated_ownership_boundary_capture() -> TestResult {
    assert_missing(
        "Parent/child ownership boundary: no parent/child ownership boundary was captured; child-owned lanes receive edits.",
        "compacted continuation evidence missing parent/child ownership boundary",
    )
}

#[test]
fn validator_cli_rejects_missing_current_stop_condition() -> TestResult {
    assert_missing(
        "Stop condition: no current stop condition was captured.",
        "compacted continuation evidence missing authoritative stop condition",
    )
}

fn assert_missing(field: &str, expected_stderr: &str) -> TestResult {
    let output = validate_open_pr_handoff(&handoff_with(field))?;
    assert!(
        !output.status.success(),
        "validator should reject handoff\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected_stderr),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn handoff_with(field: &str) -> String {
    let (duplicate_state, ownership_boundary, stop_condition) =
        if field.starts_with("Duplicate/no-active-work") {
            (field, OWNERSHIP_BOUNDARY, STOP_CONDITION)
        } else if field.starts_with("Parent/child ownership boundary") {
            (DUPLICATE_STATE, field, STOP_CONDITION)
        } else {
            (DUPLICATE_STATE, OWNERSHIP_BOUNDARY, field)
        };
    format!(
        "Post-compaction continuation readiness:\n\
         {CONTRACT}\n\
         {duplicate_state}\n\
         {ownership_boundary}\n\
         {GIT_PREFLIGHT}\n\
         {stop_condition}\n\
         Next action: stop.\n"
    )
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
