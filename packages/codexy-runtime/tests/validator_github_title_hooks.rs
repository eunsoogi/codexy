// Exercise title checks at issue, PR, and merge boundaries, including nested
// tool calls and size-limit fallbacks without constraining free-form bodies.
use serde_json::{Value, json};
use std::io::Write as _;
use std::process::Stdio;
use std::{env, fs};

use crate::support::FixtureCommand as Command;

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

const MERGE_PULL_REQUEST_TOOL: &str = "mcp__codex_apps__github_merge_pull_request";
const AUTO_MERGE_TOOL: &str = "mcp__codex_apps__github_enable_auto_merge";
const VALID_SUBJECT: &str = "fix(sample): sample change (#123)";

fn assert_hook_input(event: &str, kind: &str, tool: &str, input: Value, denied: bool) -> TestResult {
    assert_title(event, kind, json!({"hook_event_name": event, "tool_name": tool, "tool_input": input}), denied)
}
fn assert_shell_case(event: &str, command: &str, denied: bool) -> TestResult {
    assert_hook_input(event, "shell", "Bash", json!({"command": command}), denied)
}
fn assert_nested_case(event: &str, code: &str, denied: bool) -> TestResult {
    assert_hook_input(event, "nested", "functions.exec", json!({"code": code}), denied)
}

#[test]
fn supported_merge_routes_control_the_squash_subject() -> TestResult {
    // This source-launcher proof does not establish host dispatch or denial consumption.
    for event in ["PermissionRequest", "PreToolUse"] {
        // Explicit squash and default-method requests need a caller-controlled final subject.
        for (tool, input, denied) in [
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123, "merge_method": "squash", "commit_title": VALID_SUBJECT}), false),
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123, "merge_method": "squash"}), true),
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123, "commit_title": VALID_SUBJECT}), false),
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123}), true),
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123, "merge_method": null, "commit_title": VALID_SUBJECT}), false),
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123, "merge_method": null}), true),
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123, "merge_method": "squash", "commit_title": "fix(sample): missing suffix"}), true),
        ] {
            assert_hook_input(event, "merge", tool, input, denied)?;
        }
        // Explicit non-squash methods intentionally stay outside squash-subject policy.
        for method in ["merge", "rebase"] {
            assert_hook_input(event, "merge", MERGE_PULL_REQUEST_TOOL,
                json!({"pr_number": 123, "merge_method": method, "commit_title": "plain title"}), false)?;
        }
        for (tool, input) in [
            (MERGE_PULL_REQUEST_TOOL, json!({"pr_number": 123, "merge_method": "unknown", "commit_title": VALID_SUBJECT})),
            (AUTO_MERGE_TOOL, json!({"pr_number": 123, "repository_full_name": "owner/repo"})),
        ] {
            assert_hook_input(event, "merge", tool, input, true)?;
        }

        // CLI rows cover option values, auto-merge cancellation, and uninspectable auto-merge.
        for (command, denied) in [
            ("gh pr merge 123 --squash --subject 'fix(sample): sample change (#123)'", false),
            ("gh pr merge 123 --subject 'fix(sample): sample change (#123)'", false),
            ("gh pr merge 123", true),
            ("gh pr merge 123 --auto --squash --subject 'fix(sample): sample change (#123)'", true),
            ("gh pr merge 123 --merge --subject 'plain title'", false),
            ("gh pr merge 123 --rebase --subject 'plain title'", false),
            ("gh pr merge 123 -s -t 'fix(sample): sample change (#123)'", false),
            ("gh pr merge 123 -t 'fix(sample): sample change (#123)'", false),
            ("gh pr merge 123 -m -t 'plain title'", false),
            ("gh pr merge 123 -r -t 'plain title'", false),
            ("gh pr merge 123 --merge=false", true),
            ("gh pr merge 123 --merge=false --subject 'fix(sample): sample change (#123)'", false),
            ("gh pr merge 123 --merge $FLAGS", true),
            ("gh pr merge 123 --auto=false --squash --subject 'fix(sample): sample change (#123)'", false),
            ("gh pr merge 123 --disable-auto", false),
            ("gh pr merge 123 --body '--subject=fix(sample): sample change (#123)'", true),
            ("gh pr merge 123 -F --merge", true),
            ("gh pr merge 123 -- --subject='fix(sample): sample change (#123)'", true),
            ("gh pr merge 123 --squash --subject \"$SUBJECT\"", true),
        ] {
            assert_shell_case(event, command, denied)?;
        }

        // Nested checks cover only these literal supported calls and unresolved arguments.
        for (code, denied) in [
            ("await tools.mcp__codex_apps__github_merge_pull_request({pr_number: 123, merge_method: 'squash', commit_title: 'fix(sample): sample change (#123)'});", false),
            ("await tools.mcp__codex_apps__github_merge_pull_request({pr_number: 123, commit_title: 'fix(sample): sample change (#123)'});", false),
            ("await tools.mcp__codex_apps__github_merge_pull_request({pr_number: 123});", true),
            ("await tools.mcp__codex_apps__github_merge_pull_request({pr_number: 123, merge_method: 'rebase'});", false),
            ("await tools.mcp__codex_apps__github_enable_auto_merge({pr_number: 123, repository_full_name: 'owner/repo'});", true),
            ("await tools.exec_command({cmd: 'gh pr merge 123 --auto'});", true),
            ("await tools.mcp__codex_apps__github_merge_pull_request({pr_number: 123, merge_method: method, commit_title: 'fix(sample): sample change (#123)'});", true),
        ] {
            assert_nested_case(event, code, denied)?;
        }
    }
    Ok(())
}

#[test]
fn title_hook_preserves_only_the_three_title_contracts() -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_hook_input(event, "issue", "mcp__codex_apps__github_create_issue", json!({"title": "Valid issue", "body": "free form"}), false)?;
        assert_hook_input(event, "issue", "mcp__codex_apps__github_create_issue", json!({"title": "fix: not an issue title", "body": "free form"}), true)?;
        assert_hook_input(event, "pr", "github.update_pull_request", json!({"body": "any free-form body"}), false)?;
        assert_hook_input(event, "pr", "github.update_pull_request", json!({"title": "plain title", "body": "any free-form body"}), true)?;
        assert_hook_input(
            event,
            "merge",
            MERGE_PULL_REQUEST_TOOL,
            json!({"pr_number": 123, "merge_method": "squash", "commit_title": "fix(hooks): free body (#123)"}),
            false,
        )?;
        assert_shell_case(event, "gh pr create --title 'fix(hooks): free body' --body 'any text'", false)?;
        assert_shell_case(event, "gh pr create --title 'plain title' --body 'any text'", true)?;
        for (command, denied) in [
            ("gh api --method POST graphql -f query='mutation { createIssue(input: {title: \"plain title\"}) { issue { id } } }'", true),
            ("gh api --method POST graphql -f query='mutation { createIssue(input: {title: \"Valid issue\"}) { issue { id } } }'", false),
            ("gh issue new --title 'fix: invalid issue' --body note", true),
            ("gh issue new --title 'Valid issue' --body note", false),
            ("gh pr new --title 'plain title' --body note", true),
            ("gh pr new --title 'fix(hooks): valid title' --body note", false),
            ("gh api --hostname ghe.example repos/o/r/issues -f title='plain title'", true),
            ("gh api --header 'Accept: application/json' repos/o/r/issues -f title='plain title'", true),
            ("gh api -H 'Accept: application/json' repos/o/r/issues -f title='Valid issue'", false),
            ("gh api --method POST graphql -f query='mutation { first: createIssue(input: {title: \"Valid issue\"}) { issue { id } } second: createIssue(input: {title: \"plain title\"}) { issue { id } } }'", true),
            ("gh api --method POST graphql -f query='mutation { first: createIssue(input: {title: \"Valid issue\"}) { issue { id } } second: createPullRequest(input: {title: \"fix(hooks): valid title\"}) { pullRequest { id } } }'", false),
            ("gh pr merge --body 17 42 --squash --subject 'fix(hooks): valid title (#42)'", false),
            ("gh pr merge https://github.com/o/r/pull/42 --squash --subject 'fix(hooks): valid title (#42)'", false),
            ("gh pr merge --body 17 42 --squash --subject 'fix(hooks): valid title (#17)'", true),
        ] {
            assert_shell_case(event, command, denied)?;
        }
        for (command, denied) in [
            ("gh api repos/eunsoogi/codexy/issues -f title='plain title'", true),
            ("gh api repos/eunsoogi/codexy/issues -f title='Valid issue'", false),
            ("gh api repos/eunsoogi/codexy/issues -f body='free form'", true),
            ("gh api repos/eunsoogi/codexy/issues -F title='plain title'", true),
            ("gh api --method GET repos/eunsoogi/codexy/issues -f title='plain title'", false),
        ] {
            assert_shell_case(event, command, denied)?;
        }
        assert_nested_case(event, "await tools.mcp__codex_apps__github_create_pull_request({title: 'fix(hooks): nested title', body: 'any text'});", false)?;
        assert_nested_case(event, "await tools.mcp__codex_apps__github_create_pull_request({title: 'plain title', body: 'any text'});", true)?;
    }
    Ok(())
}

#[test]
fn nested_title_fallback_preserves_supported_calls_at_size_boundary() -> TestResult {
    let invalid = "await tools.mcp__codex_apps__github_create_issue({title: 'plain'});";
    let valid = "await tools.mcp__codex_apps__github_create_issue({title: 'Valid issue'});";
    for size in [64 * 1024, 64 * 1024 + 1] {
        for (source, expected) in [(invalid, true), (valid, false)] {
            let code = format!("{}{}", " ".repeat(size - source.len()), source);
            for event in ["PermissionRequest", "PreToolUse"] {
                assert_nested_case(event, &code, expected)?;
            }
        }
    }
    Ok(())
}

pub(super) fn assert_title(
    event: &str,
    kind: &str,
    payload: Value,
    denied: bool,
) -> TestResult {
    let plugin = codexy_runtime::paths::repository_root().join("plugins/codexy-github");
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd.exe");
        command
            .args(["/d", "/c"])
            .arg(plugin.join("hooks/codexy-title-check.cmd"));
        command
    } else {
        Command::new(plugin.join("hooks/codexy-title-check.sh"))
    };
    let mut child = command
        .args([event, kind])
        .env("PLUGIN_ROOT", &plugin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().ok_or("title launcher stdin")?
        .write_all(&serde_json::to_vec(&payload)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(!output.stdout.is_empty(), denied, "{event} {kind}");
    if denied {
        let denial: Value = serde_json::from_slice(&output.stdout)?;
        let specific = &denial["hookSpecificOutput"];
        assert_eq!(specific["hookEventName"], event);
        assert!(specific["decision"]["behavior"] == "deny" || specific["permissionDecision"] == "deny");
        let reason = if event == "PermissionRequest" {
            specific["decision"]["message"].as_str().unwrap_or_default()
        } else {
            specific["permissionDecisionReason"].as_str().unwrap_or_default()
        };
        assert!(reason.starts_with("CODEXY_TITLE_CHECK_"), "{reason}");
    }
    Ok(())
}

#[test]
fn windows_title_runtime_fallback_is_valid_json() -> TestResult {
    if !cfg!(windows) {
        return Ok(());
    }
    let temporary = tempfile::tempdir()?;
    fs::write(temporary.path().join("py.cmd"), "@echo off\r\nexit /b 1\r\n")?;
    let plugin = codexy_runtime::paths::repository_root().join("plugins/codexy-github");
    let original_path = env::var_os("PATH").unwrap_or_default();
    let path = format!("{};{}", temporary.path().display(), original_path.to_string_lossy());
    for event in ["PermissionRequest", "PreToolUse"] {
        let payload = json!({
            "hook_event_name": event,
            "tool_name": "Bash",
            "tool_input": {"command": "gh issue create --title arbitrary"},
        });
        let mut command = Command::new("cmd.exe");
        let mut child = command
            .args(["/d", "/c"])
            .arg(plugin.join("hooks/codexy-title-check.cmd"))
            .args([event, "shell"])
            .env("PATH", &path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child.stdin.take().ok_or("title fallback stdin")?.write_all(
            &serde_json::to_vec(&payload)?,
        )?;
        let output = child.wait_with_output()?;
        assert!(
            output.status.success(),
            "title fallback exited with {:?}; stdout={:?}; stderr={:?}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let denial: Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        if event == "PermissionRequest" {
            assert_eq!(denial["hookSpecificOutput"]["decision"]["behavior"], "deny");
        } else {
            assert_eq!(denial["hookSpecificOutput"]["permissionDecision"], "deny");
        }
    }
    Ok(())
}
