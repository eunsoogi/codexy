use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_readiness_without_pr_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"}]}]}"#,
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
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"}]}]}"#,
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
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[{"name":"type/fix"},{"name":"status/review"}],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
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
fn validator_accepts_readiness_with_codexy_lane_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[{"name":"bug"},{"name":"review"}],"closingIssuesReferences":[{"number":180,"labels":[{"name":"workflow"},{"name":"urgent"}]}]}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept readiness evidence with repository-specific labels\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_graphql_label_nodes() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":{"nodes":[{"name":"bug"},{"name":"review"}]},"closingIssuesReferences":{"nodes":[{"number":180,"labels":{"nodes":[{"name":"workflow"},{"name":"urgent"}]}}]}}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept GraphQL nodes label evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_codexy_readiness_with_label_consideration_evidence() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed. Labels considered: repository has no matching lane label.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should accept explicit label consideration evidence for Codexy lanes\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_stale_applied_label_claim_without_state_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed. Labels applied: type/fix, status/review.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject stale textual label application claims without captured GitHub label state\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("issue #180 labels"));
    Ok(())
}

#[test]
fn validator_accepts_user_repo_without_codexy_label_taxonomy() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":7,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"feature/customer-report","repository":"example/user-app","labels":[],"closingIssuesReferences":[{"number":3,"labels":[]}]}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not enforce Codexy label requirements on arbitrary user repositories\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_accepts_user_repo_with_codexy_in_name() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification.\n",
        r#"{"number":7,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/local-helper","repository":"example/codexy-helper","labels":[],"closingIssuesReferences":[{"number":3,"labels":[]}]}"#,
    )?;

    assert!(
        output.status.success(),
        "validator should not infer Codexy repository policy from a repo or branch name fragment\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_missing_labels_despite_broad_label_words() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed. GitHub labels: missing.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
    )?;

    assert!(
        !output.status.success(),
        "validator should reject missing labels when the handoff only says labels are missing\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
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
