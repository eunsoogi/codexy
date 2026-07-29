use std::process::Command;

const PR_STATE: &str = r#"{"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#;
const CONTRACT_AUTHORIZATION: &str = r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractId":"codexy-main-squash","contractVersion":1,"recordIssuer":"maintainer-recorded","target":"current-pull-request","negated":false,"revoked":false}"#;

#[test]
fn validator_accepts_the_checked_repository_workflow_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let output = validate(CONTRACT_AUTHORIZATION)?;
    assert!(output.status.success(), "{}", stderr(&output));
    Ok(())
}

#[test]
fn validator_accepts_exact_explicit_user_or_maintainer_intent() -> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        r#"{"kind":"explicit-user-intent","actor":"user","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","recordIssuer":"maintainer-recorded","sourceReference":"user-intent://thread/authoritative-message","negated":false,"revoked":false}"#,
        r#"{"kind":"explicit-maintainer-intent","actor":"maintainer","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","recordIssuer":"maintainer-recorded","sourceReference":"maintainer-intent://review/authoritative-message","negated":false,"revoked":false}"#,
    ] { assert!(validate(authorization)?.status.success(), "{authorization}"); }
    Ok(())
}

#[test]
fn validator_rejects_non_authoritative_or_wrongly_scoped_intent()
-> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        r#"{"kind":"generic-finish","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#,
        r#"{"kind":"parent-agent-prose","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#,
        &CONTRACT_AUTHORIZATION.replacen("\"prNumber\":128", "\"prNumber\":127", 1),
        &CONTRACT_AUTHORIZATION.replacen("\"baseRefName\":\"main\"", "\"baseRefName\":\"release\"", 1),
        &CONTRACT_AUTHORIZATION.replacen("\"headRefOid\":\"32b03a210b3defb2d29dd352283ea2488e60d893\"", "\"headRefOid\":\"stale\"", 1),
    ] {
        let output = validate(authorization)?;
        assert!(!output.status.success(), "{authorization}");
        assert!(stderr(&output).contains("merge authorization"), "{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_rejects_forged_or_revoked_authorization() -> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        r#"{"kind":"explicit-user-intent","actor":"user","sourceReference":"user://parent-agent-forged"}"#,
        r#"{"kind":"explicit-maintainer-intent","actor":"maintainer","sourceReference":"maintainer://parent-agent-forged"}"#,
        r#"{"kind":"explicit-user-intent","actor":"user","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","recordIssuer":"agent-self-authored","sourceReference":"user-intent://parent-agent","negated":false,"revoked":false}"#,
        &CONTRACT_AUTHORIZATION.replacen("\"negated\":false", "\"negated\":true", 1),
        &CONTRACT_AUTHORIZATION.replacen("\"revoked\":false", "\"revoked\":true", 1),
        &CONTRACT_AUTHORIZATION.replacen("\"negated\":false", "\"negated\":\"true\"", 1),
        &CONTRACT_AUTHORIZATION.replacen("\"recordIssuer\":\"maintainer-recorded\"", "\"recordIssuer\":\"agent-self-authored\"", 1),
        &CONTRACT_AUTHORIZATION.replacen("\"contractId\":\"codexy-main-squash\"", "\"contractId\":\"invented-contract\"", 1),
        &CONTRACT_AUTHORIZATION.replacen("\"target\":\"current-pull-request\"", "\"target\":\"all-pull-requests\"", 1),
        &CONTRACT_AUTHORIZATION.replacen(",\"target\":\"current-pull-request\"", "", 1),
    ] {
        let output = validate(authorization)?;
        assert!(!output.status.success(), "{authorization}");
        assert!(stderr(&output).contains("merge authorization"), "{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_rejects_ambiguous_or_untyped_current_targets() -> Result<(), Box<dyn std::error::Error>> {
    for authorization in [
        CONTRACT_AUTHORIZATION.replace("128", "null"),
        CONTRACT_AUTHORIZATION.replace("\"main\"", "null"),
        CONTRACT_AUTHORIZATION.replace("\"32b03a210b3defb2d29dd352283ea2488e60d893\"", "null"),
        CONTRACT_AUTHORIZATION.replacen("\"negated\":false", "\"negated\":true,\"negated\":false", 1),
        CONTRACT_AUTHORIZATION.replacen("\"prNumber\":128", "\"pr\\u004eumber\":127,\"prNumber\":128", 1),
        CONTRACT_AUTHORIZATION.replace("128", "0"),
    ] {
        let output = validate(&authorization)?;
        assert!(!output.status.success(), "{authorization}\n{}", stderr(&output));
    }
    Ok(())
}

#[test]
fn validator_rejects_ambiguous_pr_state_without_rejecting_metadata_values() -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_with_state(CONTRACT_AUTHORIZATION, r#"{"number":127,"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893"}"#)?;
    assert!(!output.status.success(), "{}", stderr(&output));
    let metadata = r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractId":"codexy-main-squash","contractVersion":1,"recordIssuer":"maintainer-recorded","target":"current-pull-request","negated":false,"revoked":false,"note":"kind"}"#;
    assert!(validate(metadata)?.status.success());
    Ok(())
}

#[test]
fn validator_rejects_combined_validation_modes() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let authorization_path = temp.path().join("merge-authorization.json");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&authorization_path, CONTRACT_AUTHORIZATION)?;
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

fn validate(authorization: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    validate_with_state(authorization, PR_STATE)
}

fn validate_with_state(authorization: &str, state: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let authorization_path = temp.path().join("merge-authorization.json");
    let pr_state_path = temp.path().join("pr-state.json");
    std::fs::write(&authorization_path, authorization)?;
    std::fs::write(&pr_state_path, state)?;
    Ok(Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args([
            "--check-merge-authorization",
            "--merge-authorization-file",
            authorization_path.to_str().ok_or("authorization path")?,
            "--merge-authorization-pr-state-file",
            pr_state_path.to_str().ok_or("PR state path")?,
        ])
        .output()?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
