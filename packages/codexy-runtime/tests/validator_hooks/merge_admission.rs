use crate::support::{FixtureCommand as Command, make_executable};
use serde_json::json;

use super::admission_runtime::{TestResult, assert_tool_case, plugin_root};

#[test]
fn merge_admission_hook_admits_valid_message_and_authorization() -> TestResult {
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
fn canonical_wrapper_rejects_caller_authorization_state_paths() -> TestResult {
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
    assert!(!output.status.success(), "caller-owned authorization state reached gh: {}", String::from_utf8_lossy(&output.stderr));
    assert!(!record.exists(), "caller-owned authorization state reached gh");
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_fetches_authorization_from_github_before_merging() -> TestResult {
    let root = plugin_root();
    let workspace = tempfile::tempdir()?;
    let owned = super::admission_runtime::repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let message = owned.join("message.txt");
    let body = owned.join("body.txt");
    std::fs::write(&message, "fix(workflow): require intent (#128)\n\nFixes #503\n")?;
    std::fs::write(&body, "Fixes #503\n")?;
    let fake_bin = workspace.path().join("bin");
    std::fs::create_dir(&fake_bin)?;
    let fake_gh = fake_bin.join("gh");
    std::fs::write(&fake_gh, "#!/bin/sh\nif [ \"$1\" = api ]; then cat \"$CODEXY_GH_STATE\"; else printf '%s\\n' \"$@\" > \"$CODEXY_GH_RECORD\"; fi\n")?;
    make_executable(&fake_gh)?;
    let state_file = workspace.path().join("github-state.json");
    let record = workspace.path().join("gh-record.txt");
    std::fs::write(&state_file, state().replace("AUTHORIZE REPOSITORY SQUASH CONTRACT", "AUTHORIZE SQUASH MERGE"))?;
    let output = Command::new(root.join("hooks/codexy-authorized-squash-merge.sh"))
        .current_dir(&owned)
        .env("PATH", format!("{}:{}", fake_bin.display(), std::env::var("PATH")?))
        .env("CODEXY_GH_STATE", &state_file).env("CODEXY_GH_RECORD", &record)
        .args(["--expected-pr", "128", "--expected-issue", "503", "--merge-message-file"])
        .arg(&message)
        .args(["--repo", "eunsoogi/codexy", "--match-head-commit", "32b03a210b3defb2d29dd352283ea2488e60d893", "--subject", "fix(workflow): require intent (#128)", "--body-file"])
        .arg(&body).output()?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        std::fs::read_to_string(record)?.lines().take(2).collect::<Vec<_>>(),
        ["pr", "merge"]
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_binds_validated_message_to_merge_payload() -> TestResult {
    let message = "fix(workflow): require intent (#128)\n\nFixes #503\n";
    let subject = "fix(workflow): require intent (#128)";
    let (output, merged, _) = wrapper_with_payload(&plugin_root(), state(), false, message, subject, "Fixes #503\n", false)?;
    assert!(output.status.success(), "exact payload rejected: {}", String::from_utf8_lossy(&output.stderr));
    assert!(merged, "exact payload did not reach merge");
    for (actual_subject, actual_body) in [
        ("fix: malformed subject", "Fixes #503\n"),
        (subject, "This body does not close #503\n"),
    ] {
        let (output, merged, _) = wrapper_with_payload(&plugin_root(), state(), false, message, actual_subject, actual_body, false)?;
        assert!(!output.status.success(), "decoy message admitted: {}", String::from_utf8_lossy(&output.stderr));
        assert!(!merged, "decoy message reached merge");
    }
    let invalid = "fix: malformed subject\n\nFixes #503\n";
    let (output, merged, _) = wrapper_with_payload(&plugin_root(), state(), false, invalid, "fix: malformed subject", "Fixes #503\n", false)?;
    assert!(!output.status.success(), "malformed exact payload admitted");
    assert!(!merged, "malformed exact payload reached merge");
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_gh_uses_immutable_body_snapshot() -> TestResult {
    let (output, merged, body) = wrapper_with_payload(
        &plugin_root(), state(), false, "fix(workflow): require intent (#128)\n\nFixes #503\n",
        "fix(workflow): require intent (#128)", "Fixes #503\n", true,
    )?;
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(merged, "immutable body did not reach merge");
    assert_eq!(body, "Fixes #503\n", "post-admission mutation changed gh body");
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_wrapper_rejects_bad_github_authorization_captures() -> TestResult {
    let duplicate = state().replacen(
        "]}",
        r#",{"id":"IC_replay","url":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-130","body":"AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #128 BASE main HEAD 32b03a210b3defb2d29dd352283ea2488e60d893","author":{"login":"maintainer"},"authorAssociation":"MEMBER"}]}"#,
        1,
    );
    for capture in [
        state().replacen("eunsoogi/codexy", "openai/codex", 1),
        state().replacen("\"number\":128", "\"number\":127", 1),
        state().replacen("32b03a210b3defb2d29dd352283ea2488e60d893", "stale-head", 1),
        state().replacen("AUTHORIZE REPOSITORY", "DO NOT AUTHORIZE REPOSITORY", 1),
        duplicate,
    ] {
        let (output, merged) = wrapper_output(&plugin_root(), &capture, false)?;
        assert!(!output.status.success(), "bad capture admitted: {}", String::from_utf8_lossy(&output.stderr));
        assert!(!merged, "bad capture reached merge");
    }
    let (output, merged) = wrapper_output(&plugin_root(), state(), true)?;
    assert!(!output.status.success(), "GitHub API failure admitted");
    assert!(!merged, "GitHub API failure reached merge");
    Ok(())
}

fn admission(root: &std::path::Path, message: &std::path::Path, authorization: &std::path::Path, state: &std::path::Path) -> std::io::Result<std::process::Output> {
    let mut command = Command::new(root.join("hooks/codexy-merge-admission-check.sh"));
    command.args(["--expected-pr", "128", "--expected-issue", "503", "--merge-message-file"]);
    command.arg_path(message);
    command.args(["--merge-authorization-file"]);
    command.arg_path(authorization);
    command.args(["--merge-authorization-pr-state-file"]);
    command.arg_path(state);
    command.output()
}

#[cfg(unix)]
fn wrapper_output(root: &std::path::Path, capture: &str, fail_api: bool) -> TestResult<(std::process::Output, bool)> {
    let (output, merged, _) = wrapper_with_payload(root, capture, fail_api, "fix(workflow): require intent (#128)\n\nFixes #503\n", "fix(workflow): require intent (#128)", "Fixes #503\n", false)?;
    Ok((output, merged))
}

#[cfg(unix)]
fn wrapper_with_payload(root: &std::path::Path, capture: &str, fail_api: bool, message_text: &str, subject: &str, body_text: &str, mutate_body: bool) -> TestResult<(std::process::Output, bool, String)> {
    let workspace = tempfile::tempdir()?;
    let owned = super::admission_runtime::repository(workspace.path(), "owned", "git@github.com:eunsoogi/codexy.git")?;
    let message = owned.join("message.txt");
    let body = owned.join("body.txt");
    let capture_file = workspace.path().join("capture.json");
    let record = workspace.path().join("merge.txt");
    let fake_bin = workspace.path().join("bin");
    std::fs::write(&message, message_text)?;
    std::fs::write(&body, body_text)?;
    std::fs::write(&capture_file, capture)?;
    std::fs::create_dir(&fake_bin)?;
    let fake_gh = fake_bin.join("gh");
    std::fs::write(&fake_gh, r#"#!/bin/sh
if [ "$1" = api ]; then [ "${CODEXY_GH_FAIL:-}" != 1 ] && cat "$CODEXY_GH_STATE"; exit; fi
[ -z "${CODEXY_MUTATION_TARGET:-}" ] || printf 'mutated after admission\n' > "$CODEXY_MUTATION_TARGET"
while [ "$#" -gt 0 ]; do
  [ "$1" != --body-file ] || { cat "$2" > "$CODEXY_GH_RECORD"; exit; }
  shift
done
exit 1
"#)?;
    make_executable(&fake_gh)?;
    let mut command = Command::new(root.join("hooks/codexy-authorized-squash-merge.sh"));
    command
        .current_dir(&owned)
        .env("PATH", format!("{}:{}", fake_bin.display(), std::env::var("PATH")?))
        .env("CODEXY_GH_STATE", &capture_file).env("CODEXY_GH_RECORD", &record)
        .env("CODEXY_GH_FAIL", if fail_api { "1" } else { "0" })
        .args(["--expected-pr", "128", "--expected-issue", "503", "--merge-message-file"])
        .arg(&message)
        .args(["--repo", "eunsoogi/codexy", "--match-head-commit", "32b03a210b3defb2d29dd352283ea2488e60d893", "--subject", subject, "--body-file"])
        .arg(&body);
    if mutate_body { command.env("CODEXY_MUTATION_TARGET", &body); }
    let output = command.output()?;
    let merged = record.exists();
    Ok((output, merged, if merged { std::fs::read_to_string(record)? } else { String::new() }))
}

fn contract() -> &'static str { r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractCommentId":"IC_contract","contractCommentUrl":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-129","target":"current-pull-request","negated":false,"revoked":false}"# }
fn state() -> &'static str { r#"{"repository":"eunsoogi/codexy","number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","comments":[{"id":"IC_contract","url":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-129","body":"AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #128 BASE main HEAD 32b03a210b3defb2d29dd352283ea2488e60d893","author":{"login":"maintainer"},"authorAssociation":"MEMBER"}]}"# }
