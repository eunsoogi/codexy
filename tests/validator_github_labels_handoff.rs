use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_readiness_without_pr_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"}]}]}"#,
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    Ok(())
}

#[test]
fn validator_rejects_spaced_readiness_without_label_evidence() -> TestResult {
    for handoff in [
        "PR readiness evidence: all gates passed.\n",
        "merge readiness evidence: all gates passed.\n",
    ] {
        let output = validate_handoff_with_pr_state(
            handoff,
            r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
        )?;

        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("issue #180 labels"));
    }
    Ok(())
}

#[test]
fn validator_rejects_deferred_readiness_without_pr_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR is merge-ready after verification. Maintainer requested no merge.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[{"name":"type/fix"},{"name":"status/ready"}]}]}"#,
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    Ok(())
}

#[test]
fn validator_rejects_deferred_completion_without_label_evidence() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Work completed. Maintainer requested no merge.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
    )?;

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("issue #180 labels"));
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
        "Maintainer override: yes. PR is merge-ready after verification.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[{"name":"bug"},{"name":"review"}],"closingIssuesReferences":[{"number":180,"labels":[{"name":"workflow"},{"name":"urgent"}]}]}"#,
    )?;

    assert_accepted(&output, "repository-specific labels should accept");
    Ok(())
}

#[test]
fn validator_accepts_graphql_label_nodes() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Maintainer override: yes. PR is merge-ready after verification.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":{"nodes":[{"name":"bug"},{"name":"review"}]},"closingIssuesReferences":{"nodes":[{"number":180,"labels":{"nodes":[{"name":"workflow"},{"name":"urgent"}]}}]}}"#,
    )?;

    assert_accepted(&output, "GraphQL nodes label evidence should accept");
    Ok(())
}

#[test]
fn validator_rejects_label_consideration_without_captured_repository_taxonomy() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed. Labels considered: repository has no matching lane label.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
    )?;

    assert_rejected(&output, "missing repository taxonomy should reject");
    assert!(String::from_utf8_lossy(&output.stderr).contains("repositoryLabels"));
    Ok(())
}

#[test]
fn validator_accepts_codexy_readiness_with_empty_captured_repository_taxonomy() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed. Labels considered: repository has no matching lane label.\n",
        r#"{"number":185,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/180-require-github-labels","repository":"eunsoogi/codexy","labels":[],"repositoryLabels":[],"closingIssuesReferences":[{"number":180,"labels":[]}]}"#,
    )?;

    assert_accepted(&output, "empty captured repository taxonomy should accept");
    Ok(())
}

#[test]
fn validator_rejects_unlabeled_pr_even_with_no_applicable_label_claim_when_repo_labels_exist()
-> TestResult {
    for pr_state in [
        r#"{"number":209,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/209-example-unlabeled-pr","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/209","labels":[],"repositoryLabels":[{"name":"type/fix"},{"name":"status/review"},{"name":"area/workflow"}],"closingIssuesReferences":[{"number":207,"labels":[{"name":"type/fix"}]}]}"#,
        r#"{"number":209,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefName":"codexy/209-example-unlabeled-pr","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/209","labels":[],"repositoryLabels":null,"repository":{"labels":{"nodes":[{"name":"type/fix"},{"name":"status/review"}]}},"closingIssuesReferences":[{"number":207,"labels":[{"name":"type/fix"}]}]}"#,
    ] {
        let output = validate_handoff_with_pr_state(
            "PR-readiness evidence: all gates passed. Labels considered: no applicable labels for this lane.\n",
            pr_state,
        )?;

        assert_rejected(&output, "#208/#209-style unlabeled PR should reject");
        assert!(String::from_utf8_lossy(&output.stderr).contains("PR labels"));
    }
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
        "Maintainer override: yes. PR is merge-ready after verification.\n",
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
        "Maintainer override: yes. PR is merge-ready after verification.\n",
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

fn assert_rejected(output: &std::process::Output, message: &str) {
    assert!(
        !output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_accepted(output: &std::process::Output, message: &str) {
    assert!(
        output.status.success(),
        "{message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
