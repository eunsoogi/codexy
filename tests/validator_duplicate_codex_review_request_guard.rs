use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";

#[test]
fn validator_cli_rejects_duplicate_current_head_codex_review_request_plan() -> TestResult {
    let handoff = valid_review_request_handoff(
        "Duplicate/no-active-work state: PR #255 has no duplicate lane after current GitHub state re-check.",
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":255,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{
                "body":"@codex review",
                "url":"https://github.com/eunsoogi/codexy/pull/255#issuecomment-4880788420",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-04T05:17:47Z"
            },{
                "body":"@codex review",
                "url":"https://github.com/eunsoogi/codexy/pull/255#issuecomment-4880788656",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-04T05:17:54Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should reject duplicate Codex review request plans\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("duplicate current-head Codex review request"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_first_current_head_codex_review_request_plan() -> TestResult {
    let handoff = valid_review_request_handoff(
        "Duplicate/no-active-work state: PR #255 has no-active-work and no active Codex review request or output after current GitHub state re-check.",
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":255,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should allow the first Codex review request when no current-head request or output exists\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_ignores_review_request_text_in_pr_body() -> TestResult {
    let handoff = valid_review_request_handoff(
        "Duplicate/no-active-work state: PR #262 has no-active-work after current GitHub state re-check.",
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"7ebdf2512e64df8345cd0ee4876d8950c801c465",
            "body":"This PR documents the `@codex review` duplicate guard.",
            "comments":[{
                "body":"To use Codex here, create an environment for this repo.",
                "author":{"login":"chatgpt-codex-connector"},
                "createdAt":"2026-07-05T02:41:37Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4884600598"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should ignore @codex review text in the PR body and connector setup comments\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_review_request_after_stale_prior_head_output() -> TestResult {
    let handoff = valid_review_request_handoff(
        "Duplicate/no-active-work state: PR #255 has no-active-work and no current-head Codex review request or output after current GitHub state re-check.",
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":255,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "comments":[],
            "latestReviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-04T05:10:00Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should allow a fresh request when only prior-head request/output exists\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_pending_request_followed_by_stale_prior_head_output() -> TestResult {
    let handoff = valid_review_request_handoff(
        "Duplicate/no-active-work state: PR #255 has no duplicate lane after current GitHub state re-check.",
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":255,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-04T05:20:00Z"
            }],
            "latestReviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-04T05:21:00Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should reject another request when a request may still be pending despite stale output\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("duplicate current-head Codex review request"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn valid_review_request_handoff(duplicate_state: &str) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         {duplicate_state}\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; request at most one current-head Codex review.\n\
         Next action: request Codex review on current head.\n"
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
