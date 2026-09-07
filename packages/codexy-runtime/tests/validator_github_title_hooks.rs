use serde_json::{Value, json};
use std::io::Write as _;
use std::process::Stdio;
use std::{env, fs};

use crate::support::FixtureCommand as Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn title_hook_preserves_only_the_three_title_contracts() -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        assert_title(event, "issue", json!({
            "hook_event_name": event,
            "tool_name": "mcp__codex_apps__github_create_issue",
            "tool_input": {"title": "Valid issue", "body": "free form"},
        }), false)?;
        assert_title(event, "issue", json!({
            "hook_event_name": event,
            "tool_name": "mcp__codex_apps__github_create_issue",
            "tool_input": {"title": "fix: not an issue title", "body": "free form"},
        }), true)?;
        assert_title(event, "pr", json!({
            "hook_event_name": event,
            "tool_name": "github.update_pull_request",
            "tool_input": {"body": "any free-form body"},
        }), false)?;
        assert_title(event, "pr", json!({
            "hook_event_name": event,
            "tool_name": "github.update_pull_request",
            "tool_input": {"title": "plain title", "body": "any free-form body"},
        }), true)?;
        assert_title(event, "merge", json!({
            "hook_event_name": event,
            "tool_name": "mcp__codex_apps__github_merge_pull_request",
            "tool_input": {"pr_number": 42, "merge_method": "squash", "commit_title": "fix(hooks): free body (#42)", "commit_message": "anything"},
        }), false)?;
        assert_title(event, "merge", json!({
            "hook_event_name": event,
            "tool_name": "mcp__codex_apps__github_merge_pull_request",
            "tool_input": {"pr_number": 42, "merge_method": "squash", "commit_title": "Merge pull request (#42)", "commit_message": "anything"},
        }), true)?;
        assert_title(event, "shell", json!({
            "hook_event_name": event,
            "tool_name": "Bash",
            "tool_input": {"command": "gh pr create --title 'fix(hooks): free body' --body 'any text'"},
        }), false)?;
        assert_title(event, "shell", json!({
            "hook_event_name": event,
            "tool_name": "Bash",
            "tool_input": {"command": "gh pr create --title 'plain title' --body 'any text'"},
        }), true)?;
        assert_title(event, "shell", json!({
            "hook_event_name": event,
            "tool_name": "Bash",
            "tool_input": {"command": "gh api --method POST graphql -f query='mutation { createIssue(input: {title: \"plain title\"}) { issue { id } } }'"},
        }), true)?;
        assert_title(event, "shell", json!({
            "hook_event_name": event,
            "tool_name": "Bash",
            "tool_input": {"command": "gh api --method POST graphql -f query='mutation { createIssue(input: {title: \"Valid issue\"}) { issue { id } } }'"},
        }), false)?;
        for (command, denied) in [
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
            assert_title(event, "shell", json!({
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": {"command": command},
            }), denied)?;
        }
        for (command, denied) in [
            ("gh api repos/eunsoogi/codexy/issues -f title='plain title'", true),
            ("gh api repos/eunsoogi/codexy/issues -f title='Valid issue'", false),
            ("gh api repos/eunsoogi/codexy/issues -f body='free form'", true),
            ("gh api repos/eunsoogi/codexy/issues -F title='plain title'", true),
            ("gh api --method GET repos/eunsoogi/codexy/issues -f title='plain title'", false),
        ] {
            assert_title(event, "shell", json!({
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": {"command": command},
            }), denied)?;
        }
        assert_title(event, "nested", json!({
            "hook_event_name": event,
            "tool_name": "functions.exec",
            "tool_input": {"code": "await tools.mcp__codex_apps__github_create_pull_request({title: 'fix(hooks): nested title', body: 'any text'});"},
        }), false)?;
        assert_title(event, "nested", json!({
            "hook_event_name": event,
            "tool_name": "functions.exec",
            "tool_input": {"code": "await tools.mcp__codex_apps__github_create_pull_request({title: 'plain title', body: 'any text'});"},
        }), true)?;
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
                assert_title(event, "nested", json!({
                    "hook_event_name": event,
                    "tool_name": "functions.exec",
                    "tool_input": {"code": code.clone()},
                }), expected)?;
            }
        }
    }
    Ok(())
}

fn assert_title(event: &str, kind: &str, payload: Value, denied: bool) -> TestResult {
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
        assert!(output.status.success());
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
