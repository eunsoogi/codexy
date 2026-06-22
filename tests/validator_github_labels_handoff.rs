use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_readiness_without_pr_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":180,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","labels":[],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"},{"name":"priority/high"},{"name":"area/workflow"}]}]}"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject readiness evidence without PR labels\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    Ok(())
}

#[test]
fn validator_rejects_deferred_readiness_without_pr_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification. Maintainer requested no merge.\n",
        r#"{"number":180,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","labels":[],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"},{"name":"priority/high"},{"name":"area/workflow"}]}]}"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject readiness evidence without PR labels even when merge is deferred\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    Ok(())
}

#[test]
fn validator_rejects_readiness_without_issue_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":180,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"}],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"},{"name":"priority/high"}]}]}"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject readiness evidence without closing issue labels\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("issue #180 labels"));
    Ok(())
}

#[test]
fn validator_accepts_readiness_with_taxonomy_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":180,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"}],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"},{"name":"priority/high"},{"name":"area/workflow"}]}]}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept readiness evidence with required taxonomy labels\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_graphql_label_nodes() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":180,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","labels":{"nodes":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"}]},"closingIssuesReferences":{"nodes":[{"number":180,"labels":{"nodes":[{"name":"type/fix"},{"name":"status/ready"},{"name":"priority/high"},{"name":"area/workflow"}]}}]}}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept GraphQL nodes label evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
