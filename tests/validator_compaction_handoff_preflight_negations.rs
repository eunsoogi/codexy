use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

const OPEN_PR_STATE: &str =
    r#"{"number":170,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN"}"#;
const CONTRACT: &str =
    "Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.";
const DUPLICATE_STATE: &str = "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.";
const OWNERSHIP_BOUNDARY: &str = "Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.";
const STOP_CONDITION: &str =
    "Stop condition: no merge; leave PR open until current-head Codex review is clean.";

#[test]
fn validator_cli_rejects_git_preflight_block_with_generic_run_capture_negation() -> TestResult {
    for git_preflight in [
        "Git graph/log preflight captured before editing:\n\
         Preflight commands were not actually run or captured:\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=50",
        "Git graph/log preflight captured before editing:\n\
         No preflight command execution/capture occurred:\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=50",
        "Git graph/log preflight captured before editing:\n\
         Commands were not run.\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=50",
        "Git graph/log preflight captured before editing:\n\
         Commands were not captured.\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=50",
    ] {
        let output = validate_open_pr_handoff(&valid_handoff_with(git_preflight))?;
        assert!(
            !output.status.success(),
            "validator should reject handoff\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("compacted continuation evidence missing git graph/log preflight"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn valid_handoff_with(git_preflight: &str) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         {CONTRACT}\n\
         {DUPLICATE_STATE}\n\
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
