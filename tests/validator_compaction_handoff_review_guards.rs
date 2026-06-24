use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

const OPEN_PR_STATE: &str =
    r#"{"number":170,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN"}"#;
const CONTRACT: &str =
    "Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.";
const DUPLICATE_STATE: &str = "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.";
const OWNERSHIP_BOUNDARY: &str = "Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.";
const GIT_PREFLIGHT: &str = "Git graph/log preflight captured before editing:\n\
     - pwd\n\
     - git status --short --branch\n\
     - git rev-parse HEAD\n\
     - git rev-parse origin/main\n\
     - git log --graph --oneline --decorate --all --max-count=50";
const STOP_CONDITION: &str =
    "Stop condition: no merge; leave PR open until current-head Codex review is clean.";

fn assert_valid(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "validator should accept handoff\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_invalid(output: &std::process::Output, expected_stderr: &str) {
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
}

#[test]
fn validator_cli_accepts_git_preflight_when_later_prose_negates_other_checks() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        DUPLICATE_STATE,
        &format!(
            "{GIT_PREFLIGHT}\n\
             I did not run full cargo test because the review-response lane only needed focused validation."
        ),
    ))?;
    assert_valid(&output);
    Ok(())
}

#[test]
fn validator_cli_rejects_tautological_duplicate_state() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Duplicate/no-active-work state: no-active-work.",
        GIT_PREFLIGHT,
    ))?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing duplicate/no-active-work state",
    );
    Ok(())
}

fn valid_handoff_with(duplicate_state: &str, git_preflight: &str) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         {CONTRACT}\n\
         {duplicate_state}\n\
         {OWNERSHIP_BOUNDARY}\n\
         {git_preflight}\n\
         {STOP_CONDITION}\n\
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
