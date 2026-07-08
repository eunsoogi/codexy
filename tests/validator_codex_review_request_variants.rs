use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_duplicate_after_acknowledged_request_variants() -> TestResult {
    for body in [
        "request review from @codex",
        "request a review from @codex",
        "request @codex to review",
    ] {
        let pr_state = current_head_eyes_request_pr_state(body);
        let output = validate_handoff_with_pr_state(
            "Next action: request review from @codex on the current head.\n",
            &pr_state,
        )?;
        assert!(
            !output.status.success(),
            "validator should reject duplicate request after acknowledged variant {body:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("current-head request/output"));
    }
    Ok(())
}

fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    let temp = tempfile::tempdir()?;
    let handoff_path = temp.path().join("handoff.md");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&handoff_path, handoff)?;
    std::fs::write(&pr_state_path, pr_state)?;
    validate_completion_handoff(&handoff_path, &pr_state_path)
}

fn validate_completion_handoff(handoff_path: &Path, pr_state_path: &Path) -> OutputResult {
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

fn current_head_eyes_request_pr_state(body: &str) -> String {
    format!(
        r#"{{
        "number": 133,
        "state": "OPEN",
        "isDraft": false,
        "headRefOid": "32b03a210b3defb2d29dd352283ea2488e60d893",
        "headRefCommittedDate": "2026-06-22T12:44:00Z",
        "mergeStateStatus": "CLEAN",
        "reviewThreads": {{"pageInfo":{{"hasNextPage":false}},"nodes":[]}},
        "comments": [{{
            "body": "{body}",
            "createdAt": "2026-06-22T12:50:03Z",
            "reactionGroups": [{{"content":"EYES","users":{{"totalCount":1}}}}]
        }}],
        "reviews": []
    }}"#
    )
}
