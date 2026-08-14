use crate::support::{FixtureCommand as Command, make_executable};
use serde_json::json;

use super::admission_runtime::{TestResult, assert_tool_case, plugin_root};

#[path = "merge_admission/admission_cases.rs"]
mod admission_cases;
#[path = "merge_admission/wrapper_cases.rs"]
mod wrapper_cases;

fn admission(
    root: &std::path::Path,
    message: &std::path::Path,
    authorization: &std::path::Path,
    state: &std::path::Path,
) -> std::io::Result<std::process::Output> {
    let mut command = Command::new(root.join("hooks/codexy-merge-admission-check.sh"));
    command.args([
        "--expected-pr",
        "128",
        "--expected-issue",
        "503",
        "--merge-message-file",
    ]);
    command.arg_path(message);
    command.args(["--merge-authorization-file"]);
    command.arg_path(authorization);
    command.args(["--merge-authorization-pr-state-file"]);
    command.arg_path(state);
    command.output()
}

#[cfg(unix)]
fn wrapper_output(
    root: &std::path::Path,
    capture: &str,
    fail_api: bool,
) -> TestResult<(std::process::Output, bool)> {
    let (output, merged, _) = wrapper_with_payload(
        root,
        capture,
        fail_api,
        "fix(workflow): require intent (#128)\n\nFixes #503\n",
        "fix(workflow): require intent (#128)",
        "Fixes #503\n",
        false,
    )?;
    Ok((output, merged))
}

#[cfg(unix)]
fn wrapper_with_payload(
    root: &std::path::Path,
    capture: &str,
    fail_api: bool,
    message_text: &str,
    subject: &str,
    body_text: &str,
    mutate_body: bool,
) -> TestResult<(std::process::Output, bool, String)> {
    let workspace = tempfile::tempdir()?;
    let owned = super::admission_runtime::repository(
        workspace.path(),
        "owned",
        "git@github.com:eunsoogi/codexy.git",
    )?;
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
    std::fs::write(
        &fake_gh,
        r#"#!/bin/sh
if [ "$1" = api ]; then [ "${CODEXY_GH_FAIL:-}" != 1 ] && cat "$CODEXY_GH_STATE"; exit; fi
[ -z "${CODEXY_MUTATION_TARGET:-}" ] || printf 'mutated after admission\n' > "$CODEXY_MUTATION_TARGET"
while [ "$#" -gt 0 ]; do
  [ "$1" != --body-file ] || { cat "$2" > "$CODEXY_GH_RECORD"; exit; }
  shift
done
exit 1
"#,
    )?;
    make_executable(&fake_gh)?;
    let mut command = Command::new(root.join("hooks/codexy-authorized-squash-merge.sh"));
    command
        .current_dir(&owned)
        .env(
            "PATH",
            format!("{}:{}", fake_bin.display(), std::env::var("PATH")?),
        )
        .env("CODEXY_GH_STATE", &capture_file)
        .env("CODEXY_GH_RECORD", &record)
        .env("CODEXY_GH_FAIL", if fail_api { "1" } else { "0" })
        .args([
            "--expected-pr",
            "128",
            "--expected-issue",
            "503",
            "--merge-message-file",
        ])
        .arg(&message)
        .args([
            "--repo",
            "eunsoogi/codexy",
            "--match-head-commit",
            "32b03a210b3defb2d29dd352283ea2488e60d893",
            "--subject",
            subject,
            "--body-file",
        ])
        .arg(&body);
    if mutate_body {
        command.env("CODEXY_MUTATION_TARGET", &body);
    }
    let output = command.output()?;
    let merged = record.exists();
    Ok((
        output,
        merged,
        if merged {
            std::fs::read_to_string(record)?
        } else {
            String::new()
        },
    ))
}

fn contract() -> &'static str {
    r#"{"kind":"repository-workflow-contract","intent":"merge","mergeClass":"squash","prNumber":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","contractCommentId":"IC_contract","contractCommentUrl":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-129","target":"current-pull-request","negated":false,"revoked":false}"#
}
fn state() -> &'static str {
    r#"{"repository":"eunsoogi/codexy","number":128,"baseRefName":"main","headRefOid":"32b03a210b3defb2d29dd352283ea2488e60d893","title":"fix(workflow): require intent","body":"Fixes #503\n","comments":[{"id":"IC_contract","url":"https://github.com/eunsoogi/codexy/pull/128#issuecomment-129","body":"AUTHORIZE REPOSITORY SQUASH CONTRACT: PR #128 BASE main HEAD 32b03a210b3defb2d29dd352283ea2488e60d893","author":{"login":"maintainer"},"authorAssociation":"MEMBER"}]}"#
}

fn github_plugin_root() -> std::path::PathBuf {
    codexy_runtime::paths::repository_root().join("plugins/codexy-github")
}
