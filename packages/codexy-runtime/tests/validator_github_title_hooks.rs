use serde_json::{Value, json};
use std::io::Write as _;
use std::process::Stdio;

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

fn assert_title(event: &str, kind: &str, payload: Value, denied: bool) -> TestResult {
    let plugin = codexy_runtime::paths::repository_root().join("plugins/codexy-github");
    let mut child = Command::new(plugin.join("hooks/codexy-title-check.sh"))
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
