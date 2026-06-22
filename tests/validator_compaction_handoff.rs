use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Output = std::process::Output;
type OutputResult = Result<Output, Box<dyn std::error::Error>>;
const DUPLICATE_STATE: &str = "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.";
const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";
const OPEN_PR_STATE: &str = r#"{"number":170,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED"}"#;

fn assert_valid(output: &Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_invalid(output: &Output, message: &str, expected_stderr: &str) {
    assert!(
        !output.status.success(),
        "{message}\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected_stderr),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn validator_cli_rejects_compacted_continuation_without_codexy_contract() -> TestResult {
    let output = validate_open_pr_handoff(
        "Compacted continuation summary:\n\
         Ready to continue after compaction.\n\
         Next action: edit the README branch and request review.\n",
    )?;
    assert_invalid(
        &output,
        "validator should reject continuation readiness that omits Codexy contract evidence",
        "compacted continuation evidence missing Codexy orchestration contract",
    );
    Ok(())
}

#[test]
fn validator_cli_accepts_compacted_continuation_with_required_codexy_evidence() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Duplicate/no-active-work state: PR #170 is a duplicate PR and has no active work after current GitHub state re-check.",
        "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.",
        "Stop condition: stop without implementation edits until duplicate/no-active-work evidence is rebuilt.",
    ))?;
    assert_valid(
        &output,
        "validator should accept continuation readiness with required Codexy evidence",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_compacted_continuation_without_git_graph_preflight() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Next action: edit files.\n",
    )?;
    assert_invalid(
        &output,
        "validator should reject continuation readiness without git graph/log preflight",
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_compacted_continuation_without_stop_condition() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.\n\
         Next action: edit files.\n",
    )?;
    assert_invalid(
        &output,
        "validator should reject continuation readiness without authoritative stop condition",
        "compacted continuation evidence missing authoritative stop condition",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_empty_or_negated_stop_condition() -> TestResult {
    for handoff in [
        valid_handoff_with(DUPLICATE_STATE, GIT_PREFLIGHT, "Stop condition:"),
        valid_handoff_with(
            DUPLICATE_STATE,
            GIT_PREFLIGHT,
            "Authoritative stop condition was not captured.",
        ),
        valid_handoff_with(DUPLICATE_STATE, GIT_PREFLIGHT, "Stop condition: none."),
    ] {
        let output = validate_open_pr_handoff(&handoff)?;
        assert_invalid(
            &output,
            "validator should reject empty or negated stop condition evidence",
            "compacted continuation evidence missing authoritative stop condition",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_substantive_no_stop_conditions() -> TestResult {
    for stop_condition in [
        "Stop condition: no merge; leave PR open until current-head Codex review is clean.",
        "Stop condition: no implementation edits until the duplicate lane is closed.",
    ] {
        let output = validate_open_pr_handoff(&valid_handoff_with(
            "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.",
            "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.",
            stop_condition,
        ))?;
        assert_valid(
            &output,
            "validator should accept substantive no-prefixed stop conditions",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_accepts_multiline_git_preflight_evidence() -> TestResult {
    let output = validate_open_pr_handoff(&valid_handoff_with(
        "Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.",
        "Git graph/log preflight captured before editing:\n\
         - pwd\n\
         - git status --short --branch\n\
         - git rev-parse HEAD\n\
         - git rev-parse origin/main\n\
         - git log --graph --oneline --decorate --all --max-count=50\n\
         * abc1234 fix(validation): reject missing review thread evidence",
        "Stop condition: no merge; leave PR open until current-head Codex review is clean.",
    ))?;
    assert_valid(
        &output,
        "validator should accept multiline git preflight evidence",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_uncaptured_duplicate_state() -> TestResult {
    for handoff in [
        valid_handoff_with(
            "Duplicate/no-active-work state: not captured.",
            GIT_PREFLIGHT,
            "Stop condition: no merge; leave PR open until current-head Codex review is clean.",
        ),
        valid_handoff_with(
            "Continuation state: current GitHub state was checked.",
            GIT_PREFLIGHT,
            "Stop condition: stop until duplicate/no-active-work evidence is rebuilt.",
        ),
    ] {
        let output = validate_open_pr_handoff(&handoff)?;
        assert_invalid(
            &output,
            "validator should reject missing or uncaptured duplicate/no-active-work state",
            "compacted continuation evidence missing duplicate/no-active-work state",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_rejects_compacted_continuation_with_git_preflight_shorthand() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: stop without implementation edits until duplicate/no-active-work evidence is rebuilt.\n\
         Git graph/log preflight: git log --graph and head/base were checked before editing.\n\
         Next action: stop.\n",
    )?;
    assert_invalid(
        &output,
        "validator should reject shorthand git preflight evidence",
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_negated_git_preflight_evidence() -> TestResult {
    let output = validate_open_pr_handoff(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #170 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         Stop condition: stop without implementation edits until duplicate/no-active-work evidence is rebuilt.\n\
         Git graph/log preflight: did not run pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, or git log --graph before editing.\n\
         Next action: stop.\n",
    )?;
    assert_invalid(
        &output,
        "validator should reject negated git preflight evidence",
        "compacted continuation evidence missing git graph/log preflight",
    );
    Ok(())
}

fn valid_handoff_with(duplicate_state: &str, git_preflight: &str, stop_condition: &str) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         {duplicate_state}\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {git_preflight}\n\
         {stop_condition}\n\
         Next action: stop.\n"
    )
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

fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    validate_handoff_with_pr_state(handoff, OPEN_PR_STATE)
}
