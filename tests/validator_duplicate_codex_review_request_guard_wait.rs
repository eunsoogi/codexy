use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";

#[test]
fn validator_cli_allows_wait_only_codex_review_output_handoff() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review output after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: leave PR open until Codex review output arrives.\n\
         Next action: wait for @codex review output.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"3c1236912337d472e544c914d9f5e77798fdf97d",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-06T01:18:12Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4888339653"
            }],
            "reviews":[],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should allow wait-only @codex review output handoffs\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_wait_only_codex_review_output_from_existing_request() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review output after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: leave PR open until Codex review output arrives.\n\
         Next action: wait for @codex review output from the existing request.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"7363f7b29a5323f82d0a03d0046d9a62ecc70976",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-06T01:18:12Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4888339653"
            }],
            "reviews":[],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should allow wait-only output handoffs that mention an existing request\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_bare_wait_only_codex_review_handoff() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: leave PR open until Codex review arrives.\n\
         Next action: wait for @codex review.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"e678bf95174498eba72bfe52978e90a99ce4dcac",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-06T01:18:12Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4888339653"
            }],
            "reviews":[],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should allow bare wait-only @codex review handoffs\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_wait_only_codex_review_then_request_again() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: leave PR open until Codex review arrives.\n\
         Next action: wait for @codex review, then request again if needed.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"e678bf95174498eba72bfe52978e90a99ce4dcac",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-06T01:18:12Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4888339653",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "reviews":[],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should reject wait-only wording followed by request-again intent\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("duplicate current-head Codex review request blocked"),
        "stderr should explain duplicate review request guard\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_negated_stop_then_wait_only_request_again() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: do not request again unless maintainer explicitly directs.\n\
         Next action: wait for @codex review, then request again if needed.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"e678bf95174498eba72bfe52978e90a99ce4dcac",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-06T01:18:12Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4888339653",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "reviews":[],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should not let a negated stop condition mask later request-again intent\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("duplicate current-head Codex review request blocked"),
        "stderr should explain duplicate review request guard\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn validate_handoff_with_pr_state(
    handoff: &str,
    pr_state: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
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
