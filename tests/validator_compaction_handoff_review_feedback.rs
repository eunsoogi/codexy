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
fn validator_cli_rejects_after_compaction_continuation_without_evidence() -> TestResult {
    for handoff in "Resuming after compaction; I will edit the PR now.|Continuing after compaction; I will edit the PR now.|Conversation compaction: ready to continue.|Compaction handoff: next action is to edit the PR.".split('|') {
        let output = validate_open_pr_handoff(handoff)?;
        assert_invalid(
            &output,
            "compacted continuation evidence missing Codexy orchestration contract",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_unchecked_checklist_evidence() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         - [ ] Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         - [ ] Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         - [ ] Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         - [ ] Git graph/log preflight captured before editing:\n\
           - pwd\n\
           - git status --short --branch\n\
           - git rev-parse HEAD\n\
           - git rev-parse origin/main\n\
           - git log --graph --oneline --decorate --all --max-count=50\n\
         - [ ] Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Next action: stop.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing Codexy orchestration contract",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_not_preserved_evidence_values() -> TestResult {
    for (contract, duplicate_state, ownership_boundary, stop_condition, expected) in [
        (
            "Codexy orchestration contract: not preserved @Codexy workflow.",
            DUPLICATE_STATE,
            OWNERSHIP_BOUNDARY,
            STOP_CONDITION,
            "compacted continuation evidence missing Codexy orchestration contract",
        ),
        (
            CONTRACT,
            "Duplicate/no-active-work state: not preserved duplicate/no-active-work.",
            OWNERSHIP_BOUNDARY,
            STOP_CONDITION,
            "compacted continuation evidence missing duplicate/no-active-work state",
        ),
        (
            CONTRACT,
            DUPLICATE_STATE,
            "Parent/child ownership boundary: not preserved child-owned boundary.",
            STOP_CONDITION,
            "compacted continuation evidence missing parent/child ownership boundary",
        ),
        (
            CONTRACT,
            DUPLICATE_STATE,
            OWNERSHIP_BOUNDARY,
            "Stop condition: not preserved no merge.",
            "compacted continuation evidence missing authoritative stop condition",
        ),
    ] {
        let output = validate_open_pr_handoff(&valid_handoff_with(
            contract,
            duplicate_state,
            ownership_boundary,
            GIT_PREFLIGHT,
            stop_condition,
        ))?;
        assert_invalid(&output, expected);
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_git_preflight_block_with_prose_negation() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        CONTRACT,
        DUPLICATE_STATE,
        OWNERSHIP_BOUNDARY,
        "Git graph/log preflight captured before editing:\n\
         Commands were not run:\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=50",
        STOP_CONDITION,
    ))?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_arbitrary_heading_text_after_partial_git_preflight() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         ## Git graph/log preflight captured before editing\n\
         - pwd\n\
         ## Verification\n\
         Later verification text mentions git status --short --branch, git rev-parse HEAD,\n\
         git rev-parse origin/main, and git log --graph, but it is not preflight evidence.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unrelated_list_section_after_partial_git_preflight() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         - Git graph/log preflight captured before editing:\n\
           - pwd\n\
         - Phase 2 verification\n\
           Later prose mentions git status --short --branch, git rev-parse HEAD,\n\
           git rev-parse origin/main, and git log --graph were checked later.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unbulleted_section_label_after_partial_git_preflight() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Git graph/log preflight captured before editing:\n\
         - pwd\n\
         Phase 2 post-review verification: git status --short --branch, git rev-parse HEAD,\n\
         git rev-parse origin/main, and git log --graph were checked later.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_unchecked_git_preflight_checklist_item() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         - [x] Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         - [x] Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         - [x] Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         - [ ] Git graph/log preflight captured before editing:\n\
           - pwd\n\
           - git status --short --branch\n\
           - git rev-parse HEAD\n\
           - git rev-parse origin/main\n\
           - git log --graph --oneline --decorate --all --max-count=50\n\
         - [x] Stop condition: no merge; leave PR open until current-head Codex review is clean.\n\
         Next action: stop.\n",
    )?;
    assert_invalid(
        &output,
        "compacted continuation evidence missing git graph/log preflight",
    );
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
