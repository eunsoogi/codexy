use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";

#[test]
fn validator_cli_allows_status_evidence_about_absent_review_request() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 is duplicate/no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: do not merge; current-head Codex review output exists.\n\
         Next action: poll current Codex review threads.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"1a8a18330c904a1f5621c9110431277ad8366ebc",
            "comments":[],
            "reviews":[{
                "body":"Codex Review\n\nHere is an actionable issue.\n\nReviewed commit: `1a8a18330c904a1f5621c9110431277ad8366ebc`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-05T05:36:35Z",
                "commit":{"oid":"1a8a18330c904a1f5621c9110431277ad8366ebc"}
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should not treat status evidence as a planned review request\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_ignores_nested_review_text_request_mentions() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 has no-active-work after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; request at most one current-head Codex review.\n\
         Next action: request Codex review on current head.\n"
    );
    let output = validate_handoff_with_pr_state(
        &handoff,
        r##"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"9ca76685a9f4a1f041ee6ef2e846876897ee3009",
            "headRefCommittedDate":"2026-07-05T08:00:00Z",
            "comments":[],
            "reviews":[{
                "body":"please request @codex review after this lands",
                "author":{"login":"human-reviewer"},
                "submittedAt":"2026-07-05T08:10:19Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#pullrequestreview-123"
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[{
                "id":"PRRT_nested",
                "isResolved":true,
                "isOutdated":false,
                "path":"src/validation/codex_review_handoff_events.rs",
                "comments":{"nodes":[{
                    "body":"please request @codex review",
                    "author":{"login":"human-reviewer"},
                    "url":"https://github.com/eunsoogi/codexy/pull/262#discussion_r123"
                }]}
            }]}
        }"##,
    )?;
    assert!(
        output.status.success(),
        "validator should ignore non-issue-comment @codex review mentions\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_allows_request_after_old_request_was_fulfilled() -> TestResult {
    let handoff = format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #262 has no-active-work after current GitHub state re-check.\n\
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
            "headRefOid":"a5af7920ff3e61d4496bfcf0d9e5c7acea96243f",
            "headRefCommittedDate":"2026-07-05T04:00:00Z",
            "comments":[{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-05T03:47:46Z",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4884742478"
            }],
            "reviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `0572edeeb262c70ccf5dbdb0e89b4136e27fd5e4`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-05T03:50:40Z",
                "commit":{"oid":"0572edeeb262c70ccf5dbdb0e89b4136e27fd5e4"}
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "validator should allow one fresh request after an old request has later Codex output\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_latest_request_when_first_later_output_is_stale() -> TestResult {
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
                "createdAt":"2026-07-05T10:30:00Z",
                "commit":{"oid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4884799988",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            },{
                "body":"@codex review",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-05T10:40:00Z",
                "commit":{"oid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4884799999",
                "reactionGroups":[{"content":"EYES","users":{"totalCount":1}}]
            }],
            "reviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-05T10:41:00Z",
                "commit":{"oid":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should keep the latest request pending when first later output is for an older request\nstdout: {}\nstderr: {}",
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
fn validator_cli_allows_later_current_head_output_after_latest_request() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Codex review passed on the current head. PR is merge-ready.\n",
        r#"{
            "number":262,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED",
            "headRefOid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "comments":[
                {"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-07-05T10:30:00Z","url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4884799988"},
                {"body":"@codex review","author":{"login":"eunsoogi"},"createdAt":"2026-07-05T10:40:00Z","url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4884799999"}],
            "reviews":[
                {"body":"Didn't find any major issues.\n\nReviewed commit: `bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-07-05T10:35:00Z","commit":{"oid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}},
                {"body":"Didn't find any major issues.\n\nReviewed commit: `bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb`","author":{"login":"chatgpt-codex-connector"},"submittedAt":"2026-07-05T10:41:00Z","commit":{"oid":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        output.status.success(),
        "later current-head output should satisfy the latest review request\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
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
