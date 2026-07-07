use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const GIT_PREFLIGHT: &str = "Git graph/log preflight: pwd, git status --short --branch, git rev-parse HEAD, git rev-parse origin/main, and git log --graph were captured before editing.";

#[test]
fn validator_cli_rejects_single_current_head_request_followed_by_stale_output() -> TestResult {
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
        "validator should keep a current-head request pending when later output is stale\nstdout: {}\nstderr: {}",
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
