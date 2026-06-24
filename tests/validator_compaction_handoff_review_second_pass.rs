use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Output = std::process::Output;
type OutputResult = Result<Output, Box<dyn std::error::Error>>;

const OPEN_PR_STATE: &str =
    r#"{"number":170,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN"}"#;

fn assert_valid(output: &Output) {
    assert!(
        output.status.success(),
        "validator should accept handoff\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_invalid(output: &Output, expected_stderr: &str) {
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
fn validator_cli_accepts_git_status_branch_output_in_preflight_block() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Git graph/log preflight captured before editing:\n\
         $ pwd\n\
         /repo/codexy\n\
         $ git status --short --branch\n\
         ## codexy/171-preserve-codexy-compaction...origin/codexy/171-preserve-codexy-compaction\n\
          M src/foo:bar.rs\n\
         $ git rev-parse HEAD\n\
         141283b684a5bf7db85ecd49d197ce81ffe28e95\n\
         $ git rev-parse origin/main\n\
         06a57800817c259a22d6a507650d22cf04bdded0\n\
         $ git log --graph --oneline --decorate --all --max-count=5\n\
         * 141283b fix(validation): bound git preflight evidence blocks\n",
    )?;
    assert_valid(&output);
    Ok(())
}

#[test]
fn validator_cli_rejects_no_stop_condition_placeholder() -> TestResult {
    for stop_condition in [
        "Stop condition: no stop condition was captured.",
        "Stop condition: No stop condition was captured.",
        "Stop condition: no authoritative stop condition.",
    ] {
        let output = validate_open_pr_handoff(&format!(
            "Post-compaction continuation readiness:\n\
             Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
             Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
             Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
             Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.\n\
             {stop_condition}\n\
             Next action: stop.\n"
        ))?;
        assert_invalid(
            &output,
            "compacted continuation evidence missing authoritative stop condition",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_markdown_heading_after_git_status_command() -> TestResult {
    for heading in ["## Verification", "## Results"] {
        let output = validate_open_pr_handoff(&format!(
            "Post-compaction continuation readiness:\n\
             Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
             Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
             Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
             Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
             Git graph/log preflight captured before editing:\n\
             $ pwd\n\
             /repo/codexy\n\
             $ git status --short --branch\n\
             {heading}\n\
             \n\
             Later verification text mentions git rev-parse HEAD, git rev-parse origin/main,\n\
             and git log --graph as follow-up checks.\n"
        ))?;
        assert_invalid(
            &output,
            "compacted continuation evidence missing git graph/log preflight",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_one_word_markdown_heading_after_git_status_command() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Git graph/log preflight captured before editing:\n\
         $ pwd\n\
         /repo/codexy\n\
         $ git status --short --branch\n\
         ## Verification\n\
         $ git rev-parse HEAD\n\
         141283b684a5bf7db85ecd49d197ce81ffe28e95\n\
         $ git rev-parse origin/main\n\
         06a57800817c259a22d6a507650d22cf04bdded0\n\
         $ git log --graph --oneline --decorate --all --max-count=5\n\
         * 141283b fix(validation): bound git preflight evidence blocks\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_git_preflight_block_negated_after_all_commands() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Git graph/log preflight captured before editing:\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=5\n\
         These commands were not run.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_git_preflight_block_when_not_all_commands_were_run() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Git graph/log preflight captured before editing:\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=5\n\
         Not all commands were run/captured: pwd, git status --short --branch, git rev-parse HEAD,\n\
         git rev-parse origin/main, and git log --graph.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
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
