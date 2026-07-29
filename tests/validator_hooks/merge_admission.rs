use crate::support::{FixtureCommand as Command, make_executable};
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

#[cfg(unix)]
#[test]
fn canonical_wrapper_reaches_gh_through_the_installed_admission_hook() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = super::admission_runtime::repository(
        workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git",
    )?;
    let message = owned.join("message.txt");
    let authorization = owned.join("authorization.json");
    let state_file = owned.join("state.json");
    let body = owned.join("body.txt");
    std::fs::write(&message, "fix(workflow): require intent (#128)\n\nFixes #503\n")?;
    std::fs::write(&authorization, contract())?;
    std::fs::write(&state_file, state())?;
    std::fs::write(&body, "Fixes #503\n")?;
    let fake_bin = workspace.path().join("bin");
    std::fs::create_dir(&fake_bin)?;
    let fake_gh = fake_bin.join("gh");
    std::fs::write(&fake_gh, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$CODEXY_GH_RECORD\"\n")?;
    make_executable(&fake_gh)?;
    let record = workspace.path().join("gh-record.txt");
    let wrapper = root.join("hooks/codexy-authorized-squash-merge.sh");
    let command = format!(
        "{} --expected-pr 128 --expected-issue 503 --merge-message-file {} --merge-authorization-file {} --merge-authorization-pr-state-file {} --repo eunsoogi/codexy --match-head-commit 32b03a210b3defb2d29dd352283ea2488e60d893 --subject 'fix(workflow): require intent (#128)' --body-file {}",
        wrapper.display(), message.display(), authorization.display(), state_file.display(), body.display()
    );
    super::admission_runtime::assert_case(&root, &owned, &command, false, &[])?;
    super::admission_runtime::assert_case(
        &root, &owned,
        "gh pr merge 128 --repo eunsoogi/codexy --squash --match-head-commit 32b03a210b3defb2d29dd352283ea2488e60d893 --subject 'fix(workflow): require intent (#128)' --body-file body.txt",
        true, &[],
    )?;
    let path = format!("{}:{}", fake_bin.display(), std::env::var("PATH")?);
    let output = Command::new(&wrapper)
        .current_dir(&owned)
        .env("PATH", path)
        .env("CODEXY_GH_RECORD", &record)
        .args(["--expected-pr", "128", "--expected-issue", "503", "--merge-message-file"])
        .arg(&message)
        .args(["--merge-authorization-file"])
        .arg(&authorization)
        .args(["--merge-authorization-pr-state-file"])
        .arg(&state_file)
        .args(["--repo", "eunsoogi/codexy", "--match-head-commit", "32b03a210b3defb2d29dd352283ea2488e60d893", "--subject", "fix(workflow): require intent (#128)", "--body-file"])
        .arg(&body)
        .output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        std::fs::read_to_string(record)?.lines().collect::<Vec<_>>(),
        ["pr", "merge", "128", "--repo", "eunsoogi/codexy", "--squash", "--delete-branch", "--match-head-commit", "32b03a210b3defb2d29dd352283ea2488e60d893", "--subject", "fix(workflow): require intent (#128)", "--body-file", body.to_str().ok_or("body path")?]
    );
    Ok(())
}

fn admission(root: &std::path::Path, message: &std::path::Path, authorization: &std::path::Path, state: &std::path::Path) -> std::io::Result<std::process::Output> {
    Command::new(root.join("hooks/codexy-merge-admission-check.sh"))
        .args(["--expected-pr", "128", "--expected-issue", "503", "--merge-message-file"])
        .arg(message).args(["--merge-authorization-file"]).arg(authorization)
        .args(["--merge-authorization-pr-state-file"]).arg(state).output()
}

fn contract() -> &'static str { r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractId":"codexy-main-squash","contractVersion":1,"recordIssuer":"maintainer-recorded","target":"current-pull-request","negated":false,"revoked":false}"# }
fn state() -> &'static str { r#"{"number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[]}"# }
