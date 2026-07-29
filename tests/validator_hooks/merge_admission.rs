use crate::support::FixtureCommand as Command;
use serde_json::json;

use super::admission_runtime::{TestResult, assert_tool_case, plugin_root};

#[test]
fn merge_admission_hook_runs_message_and_authorization_checks() -> TestResult {
    let root = plugin_root();
    let temp = tempfile::tempdir()?;
    let message = temp.path().join("message.txt");
    let authorization = temp.path().join("authorization.json");
    let state_file = temp.path().join("state.json");
    std::fs::write(&message, "fix(workflow): require intent (#128)\n\nFixes #503\n")?;
    std::fs::write(&authorization, contract())?;
    std::fs::write(&state_file, state())?;
    let output = admission(&root, &message, &authorization, &state_file)?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    std::fs::write(&authorization, "{}")?;
    let output = admission(&root, &message, &authorization, &state_file)?;
    assert!(!output.status.success(), "authorization bypassed: {}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

#[test]
fn connector_merge_without_authoritative_state_is_denied() -> TestResult {
    assert_tool_case(&plugin_root(), "mcp__codex_apps__github_merge_pull_request", json!({
        "repository_full_name":"eunsoogi/codexy", "pr_number":128, "merge_method":"squash",
        "expected_head_sha":"32b03a210b3defb2d29dd352283ea2488e60d893",
        "commit_title":"fix(workflow): require intent (#128)", "commit_message":"Fixes #503"
    }), true)
}

fn admission(root: &std::path::Path, message: &std::path::Path, authorization: &std::path::Path, state: &std::path::Path) -> std::io::Result<std::process::Output> {
    Command::new(root.join("hooks/codexy-merge-admission-check.sh"))
        .args(["--expected-pr", "128", "--expected-issue", "503", "--merge-message-file"])
        .arg(message).args(["--merge-authorization-file"]).arg(authorization)
        .args(["--merge-authorization-pr-state-file"]).arg(state).output()
}

fn contract() -> &'static str { r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractId":"codexy-main-squash","contractVersion":1,"recordIssuer":"maintainer-recorded","target":"current-pull-request","negated":false,"revoked":false}"# }
fn state() -> &'static str { r#"{"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[]}"# }
