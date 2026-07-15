use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;
const OPEN_PR_STATE: &str =
    r#"{"number":170,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN"}"#;
const DUPLICATE_STATE: &str = "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.";
const OWNERSHIP_BOUNDARY: &str = "Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.";
const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";
const STOP_CONDITION: &str =
    "Stop condition: no merge; leave PR open until parent final acceptance.";

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
fn validator_cli_rejects_placeholder_codexy_contract() -> TestResult {
    for contract in [
        "Codexy orchestration contract: no active @Codexy workflow was captured.",
        "Codexy workflow: codexy workflow.",
        "Codexy orchestration contract: orchestration workflow.",
        "Codexy orchestration contract: @Codexy should be restored before continuing.",
    ] {
        let output = validate_open_pr_handoff(&valid_handoff_with(
            contract,
            DUPLICATE_STATE,
            OWNERSHIP_BOUNDARY,
            GIT_PREFLIGHT,
            STOP_CONDITION,
        ))?;
        assert_invalid(
            &output,
            "compacted continuation evidence missing Codexy orchestration contract",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_planned_duplicate_state_check() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.",
        "Duplicate/no-active-work state: current PR should be checked for duplicate/no-active-work.",
        OWNERSHIP_BOUNDARY,
        GIT_PREFLIGHT,
        STOP_CONDITION,
    ))?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing duplicate/no-active-work state",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_placeholder_ownership_boundary() -> TestResult {
    for ownership_boundary in [
        "Parent/child ownership boundary: not captured.",
        "Ownership boundary: ownership boundary.",
        "Parent/child ownership boundary: child-owned lanes receive edits; boundary should be preserved before continuing.",
    ] {
        let output = validate_open_pr_handoff(&valid_handoff_with(
            "Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.",
            DUPLICATE_STATE,
            ownership_boundary,
            GIT_PREFLIGHT,
            STOP_CONDITION,
        ))?;
        assert_invalid(
            &output,
            "compacted continuation evidence missing parent/child ownership boundary",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_negated_stop_condition_placeholders() -> TestResult {
    for stop_condition in [
        "Stop condition: not requested.",
        "Stop condition: not checked.",
        "Stop condition: missing.",
        "Stop condition: was not captured during compaction.",
        "Stop condition: current stop condition was not captured during compaction.",
        "Stop condition: will be checked before editing.",
        "Stop condition: to be captured after resume.",
        "Stop condition: next action should stop after push/evidence.",
    ] {
        let output = validate_open_pr_handoff(&valid_handoff_with(
            "Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.",
            DUPLICATE_STATE,
            OWNERSHIP_BOUNDARY,
            GIT_PREFLIGHT,
            stop_condition,
        ))?;
        assert_invalid(
            &output,
            "compacted continuation evidence missing authoritative stop condition",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_not_checked_git_preflight() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.",
        DUPLICATE_STATE,
        OWNERSHIP_BOUNDARY,
        "Git graph/log preflight: not checked; pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph.",
        STOP_CONDITION,
    ))?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}
#[test]
fn validator_cli_rejects_heading_section_text_after_partial_git_preflight() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until parent final acceptance.\n\
         ## Git graph/log preflight captured before editing\n\
         - pwd\n\
         ## Stop condition\n\
         No merge; later text mentions git status --short --branch.\n\
         ## Next action\n\
         Do not backfill git rev-parse HEAD, git rev-parse origin/main, or git log --graph from this section.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}
#[test]
fn validator_cli_rejects_list_section_text_after_partial_git_preflight() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         - Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         - Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         - Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         - Git graph/log preflight captured before editing:\n\
           - pwd\n\
         - [x] Stop condition: no merge; later text mentions git status --short --branch.\n\
         - Next action: do not backfill git rev-parse HEAD, git rev-parse origin/main, or git log --graph from this section.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_markdown_list_evidence_fields() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         - Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         - Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         - Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         - Git graph/log preflight captured before editing:\n\
           - pwd\n\
           - git status --short --branch\n\
           - git rev-parse HEAD\n\
           - git rev-parse origin/main\n\
           - git log --graph --oneline --decorate --all --max-count=50\n\
         - [x] Stop condition: no merge; leave PR open until parent final acceptance.\n\
         Next action: stop.\n",
    )?;
    assert_valid(&output);
    Ok(())
}

#[test]
fn validator_cli_accepts_markdown_heading_evidence_fields() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         ### Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         ### Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         ### Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         ### Git graph/log preflight captured before editing:\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=50\n\
         ### Stop condition: no merge; leave PR open until parent final acceptance.\n\
         Next action: stop.\n",
    )?;
    assert_valid(&output);
    Ok(())
}

fn valid_handoff_with(
    contract: &str,
    duplicate_state: &str,
    ownership_boundary: &str,
    git_preflight: &str,
    stop_condition: &str,
) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         {contract}\n\
         {duplicate_state}\n\
         {ownership_boundary}\n\
         {git_preflight}\n\
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
