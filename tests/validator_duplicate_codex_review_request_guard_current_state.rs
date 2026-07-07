use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";

#[test]
fn validator_cli_requires_head_ref_oid_before_codex_review_request_plan() -> TestResult {
    let output = validate_handoff_with_pr_state(
        &valid_review_request_handoff(
            "Duplicate/no-active-work state: PR #262 has no-active-work after current GitHub state re-check.",
            "Next action: request Codex review on current head.",
        ),
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "comments":[],
            "reviews":[{
                "body":"Didn't find any major issues.\n\nReviewed commit: `32b03a210b3defb2d29dd352283ea2488e60d893`",
                "author":{"login":"chatgpt-codex-connector"},
                "submittedAt":"2026-07-07T05:17:47Z",
                "commit":{"oid":"32b03a210b3defb2d29dd352283ea2488e60d893"}
            }],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should reject request plans when PR state lacks headRefOid",
        "duplicate current-head Codex review request evidence incomplete: include headRefOid",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_duplicate_request_before_eyes_reaction() -> TestResult {
    let output = validate_handoff_with_pr_state(
        &valid_review_request_handoff(
            "Duplicate/no-active-work state: PR #262 has no duplicate lane after current GitHub state re-check.",
            "Next action: request Codex review on current head.",
        ),
        r#"{
            "number":262,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":[{
                "body":"@codex review",
                "url":"https://github.com/eunsoogi/codexy/pull/262#issuecomment-4880788420",
                "author":{"login":"eunsoogi"},
                "createdAt":"2026-07-07T05:17:47Z",
                "reactionGroups":[]
            }],
            "reviews":[],
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_rejected_with_stderr(
        &output,
        "validator should treat top-level @codex review comments as pending before eyes",
        "duplicate current-head Codex review request blocked",
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_canonical_at_codex_request_variants_without_fresh_state() -> TestResult {
    for action in [
        "Next action: request review from @codex.",
        "Next action: request @codex to review.",
    ] {
        let output = validate_handoff_with_pr_state(
            &valid_review_request_handoff(
                "Duplicate/no-active-work state: PR #262 has no-active-work after current GitHub state re-check.",
                action,
            ),
            r#"{
                "number":262,
                "state":"OPEN",
                "isDraft":false,
                "mergeStateStatus":"CLEAN",
                "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
                "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
            }"#,
        )?;
        assert_rejected_with_stderr(
            &output,
            &format!("validator should require fresh comments/reviews for {action}"),
            "duplicate current-head Codex review request evidence missing",
        );
    }
    Ok(())
}

#[test]
fn validator_cli_allows_without_posting_or_requesting_codex_review_status() -> TestResult {
    for action in [
        "Next action: hand off status without posting @codex review.",
        "Next action: continue without requesting @codex review.",
    ] {
        let output = validate_handoff_with_pr_state(
            &valid_non_request_handoff(
                "Duplicate/no-active-work state: PR #262 has no-active-work after current GitHub state re-check.",
                action,
            ),
            r#"{
                "number":262,
                "state":"OPEN",
                "isDraft":false,
                "mergeStateStatus":"CLEAN",
                "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
                "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
            }"#,
        )?;
        assert!(
            output.status.success(),
            "validator should allow negated Codex review action: {action}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn valid_non_request_handoff(duplicate_state: &str, action: &str) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         {duplicate_state}\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; do not post @codex review.\n\
         {action}\n"
    )
}

fn valid_review_request_handoff(duplicate_state: &str, action: &str) -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         {duplicate_state}\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; request at most one current-head Codex review.\n\
         {action}\n"
    )
}

fn assert_rejected_with_stderr(output: &std::process::Output, message: &str, expected: &str) {
    assert!(!output.status.success(), "{message}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
