pub(super) type OutputResult = Result<std::process::Output, Box<dyn std::error::Error>>;

pub(super) fn accept_open_pr_handoff(
    handoff: &str,
    failure_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        output.status.success(),
        "{failure_message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

pub(super) fn reject_open_pr_completion_handoff(
    handoff: &str,
    failure_message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate_open_pr_handoff(handoff)?;
    assert!(
        !output.status.success(),
        "{failure_message}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("opening a PR is not completion"),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

pub(super) fn validate_handoff_with_pr_state(handoff: &str, pr_state: &str) -> OutputResult {
    validate_completion_handoff(handoff, pr_state)
}

fn validate_completion_handoff(handoff: &str, pr_state: &str) -> OutputResult {
    crate::support::validator_completion_handoff(handoff, pr_state)
}

fn validate_open_pr_handoff(handoff: &str) -> OutputResult {
    validate_handoff_with_pr_state(
        handoff,
        r#"{"number":128,"state":"OPEN","isDraft":false,"mergeStateStatus":"CLEAN","reviewDecision":"APPROVED","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","reviewProfile":"strict","reviewEvidence":{"schema":"codexy.review-readiness.v1","head_oid":"32b03a210b3defb2d29dd352283ea2488e60d893","profile":"strict","reviewer":{"name":"codexy-sentinel","model":"gpt-5.6-sol","reasoning_effort":"xhigh"},"state":"passed","event_id":"e-passed","blockers":[]},"reviewLedger":{"schema":"codexy.review-ledger.v1","events":[{"id":"e-full","predecessor_event_id":null,"profile":"strict","head_oid":"32b03a210b3defb2d29dd352283ea2488e60d893","state":"full","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null},{"id":"e-passed","predecessor_event_id":"e-full","profile":"strict","head_oid":"32b03a210b3defb2d29dd352283ea2488e60d893","state":"passed","full_used":1,"delta_used":0,"blockers":[],"boundaries":["validator"],"escalation":null}]}}"#,
    )
}
