use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";

#[test]
fn validator_cli_rejects_latest_request_when_later_output_has_no_commit() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 has no duplicate lane after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; request at most one current-head Codex review.\n\
         Next action: request Codex review on current head.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "headRefCommittedDate":"2026-07-05T10:39:00Z",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-05T10:40:00Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4884799999"
            }],
            "reviews":[{
                "body":"Didn't find any major issues.",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-05T10:41:00Z"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should not let commitless Codex output clear a pending current-head request\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
fn validator_cli_allows_wait_then_non_codex_follow_up_request() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: leave PR open until Codex review arrives.\n\
         Next action: wait for @codex review. QA notes: request another maintainer check only if release evidence changes.\n"
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
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should scope follow-up request detection to Codex review clauses\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_wait_then_same_sentence_non_codex_request() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review after current GitHub state re-check; Next action: request another CI run if checks expire.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: leave PR open until Codex review arrives.\n"
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
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should not inherit Codex review context for same-sentence non-Codex requests\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_wait_for_codex_review_to_post_output() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work; waiting for current-head Codex review after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: leave PR open until Codex review arrives.\n\
         Next action: wait for @codex review to post output.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        pending_request_state("e678bf95174498eba72bfe52978e90a99ce4dcac"),
    )?;
    assert!(
        output.status.success(),
        "validator should allow wait-only wording about Codex posting output\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_no_current_head_codex_review_request_status() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work after current GitHub state re-check.\n\
         Review request status: no current-head Codex review request remains; current-head output exists.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; current-head Codex review output exists.\n\
         Next action: poll current Codex review threads.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "comments":[],
            "reviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-06T08:17:38Z",
                "commit":{"oid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should treat no current-head Codex review request as absence evidence\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn pending_request_state(head: &str) -> String {
    format!(
        r#"{{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"{head}",
            "comments":[{{
                "body":"@codex review",
                "author":{{"login":"eunsoogi"}},
                "createdAt":"2026-07-06T01:18:12Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4888339653"
            }}],
            "reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}
        }}"#
    )
}

fn validate_handoff_with_pr_state(
    handoff: &str,
    pr_state: impl AsRef<str>,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state.as_ref())?;
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
