use serde_json::Value;

pub(super) fn check(
    value: Option<&Value>,
    history: &[Value],
    reviewed_head: &str,
    terminal: &str,
    findings: &[Value],
    current_head: &str,
    base_oid: Option<&str>,
    issue_number: u64,
    repository: Option<&str>,
    pull_request: Option<u64>,
    pr_state: Option<&Value>,
) -> Result<(), String> {
    if let Some(value) = value {
        super::super::final_disposition::check(
            value,
            history,
            reviewed_head,
            terminal,
            findings,
            current_head,
            base_oid,
            issue_number,
            repository,
            pull_request,
            pr_state,
        )?;
    }
    Ok(())
}
