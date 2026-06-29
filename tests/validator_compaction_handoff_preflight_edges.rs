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
fn validator_cli_accepts_planned_words_inside_git_log_subjects() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Git graph/log preflight captured before editing:\n\
         $ pwd\n\
         /repo/codexy\n\
         $ git status --short --branch\n\
         ## work\n\
         $ git rev-parse HEAD\n\
         141283b684a5bf7db85ecd49d197ce81ffe28e95\n\
         $ git rev-parse origin/main\n\
         06a57800817c259a22d6a507650d22cf04bdded0\n\
         $ git log --graph --oneline --decorate --all --max-count=5\n\
         * deadbee docs: commands to be run after resume\n",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept handoff\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_wrong_rev_parse_targets() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Git graph/log preflight captured before editing:\n\
         $ pwd\n\
         /repo/codexy\n\
         $ git status --short --branch\n\
         ## work\n\
         $ git rev-parse HEAD~1\n\
         141283b684a5bf7db85ecd49d197ce81ffe28e95\n\
         $ git rev-parse origin/main~1\n\
         06a57800817c259a22d6a507650d22cf04bdded0\n\
         $ git log --graph --oneline --decorate --all --max-count=5\n\
         * 141283b fix(validation): bound git preflight evidence blocks\n",
    ))?;
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
    Ok(())
}

#[test]
fn validator_cli_accepts_negation_words_inside_git_log_subjects() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Git graph/log preflight captured before editing:\n\
         $ pwd\n\
         /repo/codexy\n\
         $ git status --short --branch\n\
         ## work\n\
         $ git rev-parse HEAD\n\
         141283b684a5bf7db85ecd49d197ce81ffe28e95\n\
         $ git rev-parse origin/main\n\
         06a57800817c259a22d6a507650d22cf04bdded0\n\
         $ git log --graph --oneline --decorate --all --max-count=5\n\
         * deadbee docs: these commands were not run before resume\n",
    ))?;
    assert!(
        output.status.success(),
        "validator should accept handoff\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_rev_parse_pathspec_targets() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Git graph/log preflight captured before editing:\n\
         $ pwd\n\
         /repo/codexy\n\
         $ git status --short --branch\n\
         ## work\n\
         $ git rev-parse HEAD:README.md\n\
         141283b684a5bf7db85ecd49d197ce81ffe28e95\n\
         $ git rev-parse origin/main:README.md\n\
         06a57800817c259a22d6a507650d22cf04bdded0\n\
         $ git log --graph --oneline --decorate --all --max-count=5\n\
         * 141283b fix(validation): bound git preflight evidence blocks\n",
    ))?;
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
    Ok(())
}

#[test]
fn validator_cli_rejects_commands_mentioned_only_in_git_log_subjects() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Git graph/log preflight captured before editing:\n\
         $ pwd\n\
         /repo/codexy\n\
         $ git log --graph --oneline --decorate --all --max-count=5\n\
         * deadbee docs: capture git status --short --branch and git rev-parse HEAD against git rev-parse origin/main\n",
    ))?;
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
