use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";

#[test]
fn validator_cli_rejects_review_request_plan_without_fresh_pr_comments_and_reviews() -> TestResult {
    let output = validate_handoff_with_pr_state(
        &valid_review_request_handoff(),
        r#"{
            "number":255,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert!(
        !output.status.success(),
        "validator should reject review request plans without captured PR comments/reviews\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("duplicate current-head Codex review request evidence missing"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_cli_rejects_review_request_plan_with_null_pr_comments_and_reviews() -> TestResult {
    let output = validate_handoff_with_pr_state(
        &valid_review_request_handoff(),
        r#"{
            "number":255,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":null,
            "reviews":null,
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_missing_evidence(output);
    Ok(())
}

#[test]
fn validator_cli_rejects_review_request_plan_with_object_pr_comments_and_reviews() -> TestResult {
    let output = validate_handoff_with_pr_state(
        &valid_review_request_handoff(),
        r#"{
            "number":255,
            "state":"OPEN",
            "isDraft":false,
            "mergeStateStatus":"CLEAN",
            "headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893",
            "comments":{"nodes":[]},
            "latestReviews":{"nodes":[]},
            "reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}
        }"#,
    )?;
    assert_missing_evidence(output);
    Ok(())
}

fn valid_review_request_handoff() -> String {
    format!(
        "Post-compaction continuation readiness:\n\
         Codexy orchestration contract: active @Codexy workflow routes through $codex-orchestration.\n\
         Duplicate/no-active-work state: PR #255 has no-active-work and no active Codex review request or output after current GitHub state re-check.\n\
         Parent/child ownership boundary: parent orchestrator monitors only; child-owned lanes receive edits.\n\
         {GIT_PREFLIGHT}\n\
         Stop condition: no merge; request at most one current-head Codex review.\n\
         Next action: request Codex review on current head.\n"
    )
}

fn assert_missing_evidence(output: std::process::Output) {
    assert!(
        !output.status.success(),
        "validator should reject review request plans without captured PR comments/reviews\nstdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("duplicate current-head Codex review request evidence missing"),
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
