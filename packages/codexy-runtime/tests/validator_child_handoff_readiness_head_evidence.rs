
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn validator_rejects_pr_ready_handoff_without_pr_head() -> TestResult {
    for head_field in ["", r###","headRefOid":"""###, r###","headRefOid":"  ""###] {
        assert_rejects_child_handoff(
            &pr_state_with(&format!(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example"{head_field},"localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}"###
            )),
            "PR headRefOid",
        )?;
    }
    Ok(())
}

#[test]
fn validator_rejects_pr_ready_handoff_with_blank_captured_heads() -> TestResult {
    for (fields, needle) in [
        (
            r###""localHeadOid":"","remoteHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609""###,
            "local HEAD evidence is missing",
        ),
        (
            r###""localHeadOid":"068dbb247b7755035223c91ee39f26830f3c1609","remoteHeadOid":"  ""###,
            "remote-tracking HEAD evidence is missing",
        ),
    ] {
        assert_rejects_child_handoff(
            &pr_state_with(&format!(
                r###""mergeStateStatus":"CLEAN","headRefName":"codexy/example","headRefOid":"068dbb247b7755035223c91ee39f26830f3c1609",{fields},"worktreeStatus":"## codexy/example...origin/codexy/example","reviewThreads":{{"pageInfo":{{"hasNextPage":false}},"nodes":[]}}"###
            )),
            needle,
        )?;
    }
    Ok(())
}

fn assert_rejects_child_handoff(pr_state: &str, needle: &str) -> TestResult {
    let output = validate_handoff_with_pr_state("Child handoff: PR-ready.\n", pr_state)?;
    assert!(
        !output.status.success(),
        "validator should reject false child handoff"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(needle), "unexpected stderr: {stderr}");
    Ok(())
}

fn pr_state_with(fields: &str) -> String {
    format!(
        r#"{{"number":204,"state":"OPEN","isDraft":false,"reviewDecision":"APPROVED","latestReviews":[{{"body":"Didn't find any major issues.\n\nReviewed commit: `068dbb247b7755035223c91ee39f26830f3c1609`","author":{{"login":"automated-review"}},"submittedAt":"2026-07-03T00:00:00Z"}}],{fields}}}"#
    )
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
    crate::support::validator_completion_handoff_files(&handoff_path, &pr_state_path)
}
