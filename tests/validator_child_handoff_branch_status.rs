use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_pushed_handoff_without_matching_branch_status_evidence() -> TestResult {
    let handoff =
        "Child handoff: branch clean. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n";
    for status_field in [
        "",
        r#","worktreeStatus":null"#,
        r#","worktreeStatus":"""#,
        r#","worktreeStatus":"nothing to commit""#,
    ] {
        assert_rejects_child_handoff(
            handoff,
            pr_state_with(&format!(
                r#""mergeStateStatus":"CLEAN","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609"{status_field},"reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}"#
            )),
            "branch status evidence",
        )?;
    }
    for status in [
        "## main...origin/main",
        "## codexy/example",
        "## codexy/example...",
        "## HEAD (no branch)",
    ] {
        assert_rejects_child_handoff(
            handoff,
            pr_state_with(&format!(
                r#""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"{status}","reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}"#
            )),
            "branch status evidence",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pushed_handoff_when_branch_status_is_unsynced() -> TestResult {
    let handoff =
        "Child handoff: branch clean. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n";
    for (status, needle) in [
        ("[ahead 1]", "ahead"),
        ("[behind 1]", "behind"),
        ("[ahead 1, behind 2]", "ahead 1, behind 2"),
        ("[gone]", "gone"),
    ] {
        assert_rejects_child_handoff(
            handoff,
            pr_state_with(&format!(
                "\"mergeStateStatus\":\"CLEAN\",\"headRefName\":\"codexy/example\",\"headRefOid\":\"068dbb247b7755035223c91ee39f26830f3c1609\",\"worktreeStatus\":\"## codexy/example...origin/codexy/example {status}\",\"reviewThreads\":{{\"pageInfo\":{{\"hasNextPage\":false}},\"nodes\":[]}}",
            )),
            needle,
        )?;
    }
    for fields in [
        r###""worktreeStatus":"## codexy/example...origin/codexy/example","localStatus":"## codexy/example...origin/codexy/example [behind 1]""###,
        r###""worktreeStatus":"## codexy/example...origin/codexy/example\n## codexy/example...origin/codexy/example [behind 1]""###,
    ] {
        assert_rejects_child_handoff(
            handoff,
            pr_state_with(&format!(
                r#""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609",{fields},"reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}"#
            )),
            "behind",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pushed_handoff_when_branch_is_behind() -> TestResult {
    assert_rejects_child_handoff(
        "Child handoff: branch clean. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        pr_state_with(
            "\"mergeStateStatus\":\"CLEAN\",\"headRefName\":\"codexy/example\",\"headRefOid\":\"068dbb247b7755035223c91ee39f26830f3c1609\",\"worktreeStatus\":\"## codexy/example...origin/codexy/example [behind 1]\",\"reviewThreads\":{\"pageInfo\":{\"hasNextPage\":false},\"nodes\":[]}",
        ),
        "behind",
    )
}

#[test]
fn validator_allows_pushed_handoff_when_branch_name_contains_diverged() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Child handoff: branch clean. Pushed: yes at 068dbb247b7755035223c91ee39f26830f3c1609.\n",
        &pr_state_with(
            "\"mergeStateStatus\":\"CLEAN\",\"headRefName\":\"feature-diverged-case\",\"headRefOid\":\"068dbb247b7755035223c91ee39f26830f3c1609\",\"worktreeStatus\":\"## feature-diverged-case...origin/feature-diverged-case\",\"reviewThreads\":{\"pageInfo\":{\"hasNextPage\":false},\"nodes\":[]}",
        ),
    )?;
    assert!(
        output.status.success(),
        "branch name should not imply divergence"
    );
    Ok(())
}

fn assert_rejects_child_handoff(handoff: &str, pr_state: String, needle: &str) -> TestResult {
    let output = validate_handoff_with_pr_state(handoff, &pr_state)?;
    assert!(
        !output.status.success(),
        "validator should reject false child handoff\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(needle), "unexpected stderr: {stderr}");
    Ok(())
}

fn pr_state_with(fields: &str) -> String {
    format!(
        r#"{{
            "number":204,
            "state":"OPEN",
            "isDraft":false,
            "reviewDecision":"APPROVED",
            "latestReviews":[{{
                "body":"Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`",
                "author":{{"login":"chatgpt-codex-connector"}},
                "submittedAt":"2026-07-03T00:00:00Z"
            }}],
            {fields}
        }}"#
    )
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
