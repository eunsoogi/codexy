use std::{path::Path, process::Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;
type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

#[test]
fn validator_accepts_stacked_pr_linked_issue_labels_without_closing_references() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "Maintainer override: yes. PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"linkedIssueReferences":{"nodes":[{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{"nodes":[{"name":"type/fix"},{"name":"status/in-progress"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}]}}]},"reviewThreads":{"pageInfo":{"hasNextPage":false},"nodes":[]}}"###,
    )?;

    assert_accepted(
        &output,
        "stacked PRs should accept separately captured linked issue labels",
    );
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_linked_issue_without_issue_labels() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"linkedIssueReferences":{"nodes":[{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{"nodes":[]}}]}}"###,
    )?;

    assert_rejected(&output, "stacked PR issue labels should still be required");
    assert!(String::from_utf8_lossy(&output.stderr).contains("issue #253 labels"));
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_linked_issue_without_issue_url() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"linkedIssueReferences":{"nodes":[{"number":253,"labels":{"nodes":[{"name":"type/fix"}]}}]}}"###,
    )?;

    assert_rejected(
        &output,
        "stacked PR issue identity should require authoritative URL evidence",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("linkedIssueReferences"),
        "stderr should name the missing stacked issue evidence\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_linked_issue_from_wrong_repository() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"linkedIssueReferences":{"nodes":[{"number":253,"url":"https://github.com/not-eunsoogi/not-codexy/issues/253","labels":{"nodes":[{"name":"type/fix"}]}}]}}"###,
    )?;

    assert_rejected(
        &output,
        "stacked PR issue identity should require the PR repository issue URL",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("linkedIssueReferences"),
        "stderr should name the missing same-repository stacked issue evidence\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_referenced_issue_alias_without_linked_issue_references()
-> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"referencedIssues":{"nodes":[{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{"nodes":[{"name":"type/fix"}]}}]}}"###,
    )?;

    assert_rejected(
        &output,
        "stacked PRs should accept only the documented linkedIssueReferences field",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("linkedIssueReferences"),
        "stderr should name the missing documented field\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_closing_keyword_with_trailing_punctuation() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253.\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"linkedIssueReferences":{"nodes":[{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{"nodes":[{"name":"type/fix"}]}}]}}"###,
    )?;

    assert_rejected(
        &output,
        "stacked PR fallback should require an exact final closing keyword line",
    );
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_closing_keyword_with_trailing_text() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253 and closes #254\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"linkedIssueReferences":{"nodes":[{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{"nodes":[{"name":"type/fix"}]}}]}}"###,
    )?;

    assert_rejected(
        &output,
        "stacked PR fallback should reject multiple or decorated closing refs",
    );
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_closing_keyword_without_single_space() -> TestResult {
    for body in [
        "Fixes#253",
        "Fixes  #253",
        "fixes #253",
        " Fixes #253",
        "Fixes #253 ",
    ] {
        let pr_state = stacked_pr_state_with_body(body);
        let output = validate_handoff_with_pr_state(
            "PR-readiness evidence: all gates passed.\n",
            &pr_state,
        )?;

        assert_rejected(
            &output,
            "stacked PR fallback should match the documented exact closing keyword grammar",
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_stacked_pr_closing_references_without_linked_issue_references() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nStacked PR body.\n\nFixes #253\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":{"nodes":[{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{"nodes":[{"name":"type/fix"}]}}]},"linkedIssueReferences":{"nodes":[]}}"###,
    )?;

    assert_rejected(
        &output,
        "stacked PRs should require the documented linkedIssueReferences evidence",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("linkedIssueReferences"),
        "stderr should name the missing stacked linked issue evidence\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn validator_rejects_default_branch_pr_without_closing_issue_references() -> TestResult {
    let output = validate_handoff_with_pr_state(
        "PR-readiness evidence: all gates passed.\n",
        r###"{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"main","defaultBranchRef":{"name":"main"},"body":"## Summary\n\nDefault branch PR body.\n\nFixes #253\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{"name":"type/fix"},{"name":"status/review"},{"name":"priority/high"},{"name":"area/workflow"},{"name":"area/qa"}],"closingIssuesReferences":[],"linkedIssueReferences":{"nodes":[{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{"nodes":[{"name":"type/fix"}]}}]}}"###,
    )?;

    assert_rejected(
        &output,
        "default branch PRs should still require closingIssuesReferences",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("closingIssuesReferences"),
        "stderr should name the missing default-branch evidence\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn stacked_pr_state_with_body(body: &str) -> String {
    format!(
        r###"{{"number":255,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","baseRefName":"codexy/215-reject-false-clean-child-handoffs","defaultBranchRef":{{"name":"main"}},"body":"## Summary\n\nStacked PR body.\n\n{body}\n","headRefName":"codexy/253-comparable-pushed-head-evidence","repository":"eunsoogi/codexy","url":"https://github.com/eunsoogi/codexy/pull/255","labels":[{{"name":"type/fix"}},{{"name":"status/review"}},{{"name":"priority/high"}},{{"name":"area/workflow"}},{{"name":"area/qa"}}],"closingIssuesReferences":[],"linkedIssueReferences":{{"nodes":[{{"number":253,"url":"https://github.com/eunsoogi/codexy/issues/253","labels":{{"nodes":[{{"name":"type/fix"}}]}}}}]}}}}"###
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
