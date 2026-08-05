use std::process::Command;

use serde_json::Value;

const PR_STATE: &str = r#"{"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[]}"#;
const LOCAL_CONTRACT: &str = r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractId":"codexy-main-squash","contractVersion":1,"recordIssuer":"maintainer-recorded","target":"current-pull-request","negated":false,"revoked":false}"#;
const COMMENT_STATE: &str = r#"{"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"id":"IC_128","url":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-128","body":"AUTHORIZE SQUASH MERGE: PR #128 BASE main HEAD 32b03a210b3defb2d29dd352283ea2488e60d893","author":{"login":"maintainer"},"authorAssociation":"OWNER"}]}"#;
const COMMENT_INTENT: &str = r#"{"kind":"explicit-maintainer-intent","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","commentId":"IC_128","commentUrl":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-128","negated":false,"revoked":false}"#;
const CONTRACT_STATE: &str = r#"{"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"id":"IC_contract","url":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-129","body":"AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #128 BASE main HEAD 32b03a210b3defb2d29dd352283ea2488e60d893","author":{"login":"maintainer"},"authorAssociation":"MEMBER"}]}"#;
const EXTERNAL_CONTRACT: &str = r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractCommentId":"IC_contract","contractCommentUrl":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-129","target":"current-pull-request","negated":false,"revoked":false}"#;

#[test]
fn validator_accepts_the_external_repository_workflow_contract()
-> Result<(), Box<dyn std::error::Error>> {
    assert_valid(EXTERNAL_CONTRACT, CONTRACT_STATE);
    Ok(())
}

#[test]
fn validator_rejects_a_repository_local_contract_claim() -> Result<(), Box<dyn std::error::Error>> {
    assert_invalid(LOCAL_CONTRACT, PR_STATE);
    Ok(())
}

#[test]
fn validator_accepts_exact_explicit_user_or_maintainer_intent() -> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        COMMENT_INTENT,
        &COMMENT_INTENT.replacen("explicit-maintainer-intent", "explicit-user-intent", 1),
    ] { assert_valid(authorization, COMMENT_STATE); }
    Ok(())
}

#[test]
fn validator_rejects_non_authoritative_or_wrongly_scoped_intent()
-> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        r#"{"kind":"generic-finish","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#,
        r#"{"kind":"parent-agent-prose","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#,
        &LOCAL_CONTRACT.replacen("\"prNumber\":128", "\"prNumber\":127", 1),
        &LOCAL_CONTRACT.replacen("\"baseRefName\":\"main\"", "\"baseRefName\":\"release\"", 1),
        &LOCAL_CONTRACT.replacen("\"headRefOid\":\"32b03a210b3defb2d29dd352283ea2488e60d893\"", "\"headRefOid\":\"stale\"", 1),
    ] {
        let errors = assert_invalid(authorization, PR_STATE);
        assert!(errors.iter().any(|error| error.contains("merge authorization")), "{errors:?}");
    }
    Ok(())
}

#[test]
fn validator_rejects_forged_or_revoked_authorization() -> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        r#"{"kind":"explicit-user-intent","actor":"user","sourceReference":"user://parent-agent-forged"}"#,
        r#"{"kind":"explicit-maintainer-intent","actor":"maintainer","sourceReference":"maintainer://parent-agent-forged"}"#,
        r#"{"kind":"explicit-user-intent","actor":"user","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","recordIssuer":"agent-self-authored","sourceReference":"user-intent://parent-agent","negated":false,"revoked":false}"#,
        COMMENT_INTENT,
        &COMMENT_INTENT.replacen("IC_128", "IC_forged", 1),
        &COMMENT_INTENT.replacen("commentUrl", "sourceReference", 1),
        &LOCAL_CONTRACT.replacen("\"negated\":false", "\"negated\":true", 1),
        &LOCAL_CONTRACT.replacen("\"revoked\":false", "\"revoked\":true", 1),
        &LOCAL_CONTRACT.replacen("\"negated\":false", "\"negated\":\"true\"", 1),
        &LOCAL_CONTRACT.replacen("\"recordIssuer\":\"maintainer-recorded\"", "\"recordIssuer\":\"agent-self-authored\"", 1),
        &LOCAL_CONTRACT.replacen("\"contractId\":\"codexy-main-squash\"", "\"contractId\":\"invented-contract\"", 1),
        &LOCAL_CONTRACT.replacen("\"target\":\"current-pull-request\"", "\"target\":\"all-pull-requests\"", 1),
        &LOCAL_CONTRACT.replacen(",\"target\":\"current-pull-request\"", "", 1),
    ] {
        let errors = assert_invalid(authorization, PR_STATE);
        assert!(errors.iter().any(|error| error.contains("merge authorization")), "{errors:?}");
    }
    Ok(())
}

#[test]
fn validator_rejects_non_authoritative_comment_evidence() -> Result<(), Box<dyn std::error::Error>> {
    for state in [
        COMMENT_STATE.replacen("\"OWNER\"", "\"CONTRIBUTOR\"", 1),
        COMMENT_STATE.replacen("AUTHORIZE SQUASH MERGE", "parent says finish", 1),
        COMMENT_STATE.replacen("BASE main", "BASE release", 1),
        COMMENT_STATE.replacen("HEAD 32b03a210b3defb2d29dd352283ea2488e60d893", "HEAD stale", 1),
        COMMENT_STATE.replacen("\"id\":\"IC_128\"", "\"id\":\"IC_128\",\"id\":\"IC_other\"", 1),
    ] {
        assert_invalid(COMMENT_INTENT, &state);
    }
    Ok(())
}

#[test]
fn validator_rejects_stale_or_replayed_external_contract_evidence() -> Result<(), Box<dyn std::error::Error>> {
    let mut replay: Value = serde_json::from_str(CONTRACT_STATE)?;
    let comments = replay
        .get_mut("comments")
        .and_then(Value::as_array_mut)
        .ok_or("contract comments")?;
    comments.push(comments.first().cloned().ok_or("contract comment")?);
    let replay = serde_json::to_string(&replay)?;
    for state in [
        CONTRACT_STATE.replacen("BASE main", "BASE release", 1),
        replay,
        CONTRACT_STATE.replacen("AUTHORIZE REPOSITORY SQUASH CONTRACT", "AUTHORIZE SQUASH MERGE", 1),
    ] {
        assert_invalid(EXTERNAL_CONTRACT, &state);
    }
    Ok(())
}

#[test]
fn validator_rejects_ambiguous_or_untyped_current_targets() -> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        LOCAL_CONTRACT.replace("128", "null"),
        LOCAL_CONTRACT.replace("\"main\"", "null"),
        LOCAL_CONTRACT.replace("\"32b03a210b3defb2d29dd352283ea2488e60d893\"", "null"),
        LOCAL_CONTRACT.replacen("\"negated\":false", "\"negated\":true,\"negated\":false", 1),
        LOCAL_CONTRACT.replacen("\"prNumber\":128", "\"pr\\u004eumber\":127,\"prNumber\":128", 1),
        LOCAL_CONTRACT.replace("128", "0"),
    ] {
        assert_invalid(&authorization, PR_STATE);
    }
    Ok(())
}

#[test]
fn validator_rejects_ambiguous_pr_state_without_rejecting_metadata_values() -> Result<(), Box<dyn std::error::Error>> {
    assert_invalid(LOCAL_CONTRACT, r#"{"number":127,"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#);
    let metadata = EXTERNAL_CONTRACT.replacen("\"revoked\":false", "\"revoked\":false,\"note\":\"kind\"", 1);
    assert_valid(&metadata, CONTRACT_STATE);
    let nullable = COMMENT_STATE.replacen(
        "\"comments\":[",
        "\"comments\":[{\"id\":\"IC_deleted\",\"url\":\"https://github.com/eunsoogi/codexy/pull/128#issuecomment-127\",\"body\":\"ordinary\",\"author\":null,\"authorAssociation\":\"NONE\"},",
        1,
    );
    assert_valid(COMMENT_INTENT, &nullable);
    assert_invalid(COMMENT_INTENT, &nullable.replacen("\"id\":\"IC_128\"", "\"id\":\"IC_128\",\"id\":\"IC_duplicate\"", 1));
    Ok(())
}

#[test]
fn validator_rejects_combined_validation_modes() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let authorization_path = temp.path().join("merge-authorization.json");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&authorization_path, LOCAL_CONTRACT)?;
    std::fs::write(&pr_state_path, PR_STATE)?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-merge-authorization",
            "--merge-authorization-file",
            authorization_path.to_str().ok_or("authorization path")?,
            "--merge-authorization-pr-state-file",
            pr_state_path.to_str().ok_or("PR state path")?,
            "--check-issue-intake",
        ])
        .output()?;
    assert!(!output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("exactly one validation mode"), "{}", stderr(&output));
    Ok(())
}

#[test]
fn authorization_does_not_make_a_separate_failed_gate_pass() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let message = temp.path().join("merge-message.txt");
    std::fs::write(&message, "not a valid merge message")?;
    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-merge-message",
            "--expected-pr",
            "128",
            "--merge-message-file",
            message.to_str().ok_or("message path")?,
        ])
        .output()?;
    assert!(!output.status.success(), "{}", stderr(&output));
    Ok(())
}

fn assert_valid(authorization: &str, state: &str) {
    let errors = codexy_runtime::validation::merge_authorization_diagnostics(authorization, state);
    assert!(errors.is_empty(), "{authorization}: {errors:?}");
}

fn assert_invalid(authorization: &str, state: &str) -> Vec<String> {
    let errors = codexy_runtime::validation::merge_authorization_diagnostics(authorization, state);
    assert!(!errors.is_empty(), "{authorization}");
    errors
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
