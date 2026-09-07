use serde_json::Value;
use std::collections::BTreeSet;
use std::io::Write as _;
use std::process::Stdio;

use crate::support::FixtureCommand as Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type LauncherResult = Result<Option<Value>, Box<dyn std::error::Error>>;

#[test]
fn github_hooks_bind_only_context_and_local_bash_safety() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy-github");
    let hooks: Value = serde_json::from_str(&std::fs::read_to_string(root.join("hooks/hooks.json"))?)?;
    let events = hooks["hooks"].as_object().ok_or("hooks object")?;
    assert_eq!(events.len(), 3);
    assert!(events.contains_key("UserPromptSubmit"));
    assert!(events.contains_key("PermissionRequest"));
    assert!(events.contains_key("PreToolUse"));

    let prompt = events["UserPromptSubmit"].as_array().ok_or("prompt hooks")?;
    assert_eq!(prompt.len(), 1);
    assert!(prompt[0].get("matcher").is_none());
    assert!(prompt[0]["hooks"][0]["command"]
        .as_str()
        .unwrap_or_default()
        .contains("codexy-github-workflow-context"));

    for event in ["PermissionRequest", "PreToolUse"] {
        let groups = events[event].as_array().ok_or("preventive hooks")?;
        assert_eq!(groups.len(), 6, "{event}");
        for (index, (matcher, kind)) in [
            (
                "^(?:mcp__codex_apps__github_(?:create|update)_issue|github\\.(?:create|update)_issue)$",
                "issue",
            ),
            (
                "^(?:mcp__codex_apps__github_(?:create|update)_pull_request|github\\.(?:create|update)_pull_request)$",
                "pr",
            ),
            (
                "^(?:mcp__codex_apps__github_(?:merge_pull_request|enable_auto_merge)|github\\.(?:merge_pull_request|enable_auto_merge))$",
                "merge",
            ),
            ("^functions\\.exec$", "nested"),
            ("^Bash$", "shell"),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(groups[index]["matcher"], matcher);
            let handler = &groups[index]["hooks"][0];
            assert_eq!(handler["type"], "command");
            assert_eq!(handler["timeout"], 5);
            assert!(handler["command"]
                .as_str()
                .unwrap_or_default()
                .contains(&format!("codexy-title-check.sh\" {event} {kind}")));
            assert!(handler["commandWindows"]
                .as_str()
                .unwrap_or_default()
                .contains(&format!("codexy-title-check.cmd\" {event} {kind}")));
        }
        let handler = &groups[5]["hooks"][0];
        assert_eq!(groups[5]["matcher"], "^Bash$");
        assert_eq!(handler["type"], "command");
        assert_eq!(handler["timeout"], 5);
        assert!(handler["command"]
            .as_str()
            .unwrap_or_default()
            .contains("codexy-destructive-command"));
        assert!(handler["commandWindows"]
            .as_str()
            .unwrap_or_default()
            .contains("codexy-destructive-command"));
    }

    let serialized = hooks.to_string();
    assert!(!serialized.contains("plugin-version-bump"));
    assert!(!serialized.contains("codexy-repository-"));
    assert!(!serialized.contains("UNAVAILABLE"));
    let expected_hook_files = [
        "codexy-destructive-command.cmd",
        "codexy-destructive-command.py",
        "codexy-destructive-command.sh",
        "codexy-github-workflow-context.cmd",
        "codexy-github-workflow-context.ps1",
        "codexy-github-workflow-context.sh",
        "codexy-hook-runtime.sh",
        "codexy-title-check.cmd",
        "codexy-title-check.py",
        "codexy-title-check.sh",
        "codexy-issue-title-check.sh",
        "codexy-merge-message-check.sh",
        "codexy-pr-label-check.sh",
        "codexy-pr-title-check.sh",
        "codexy-readiness-guard-json.sh",
        "codexy-readiness-guard-pr-labels.sh",
        "codexy-readiness-guard-values.sh",
        "codexy-readiness-guard.sh",
        "codexy-title-policy.sh",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    let actual_hook_files = std::fs::read_dir(root.join("hooks"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("codexy-"))
        .collect::<BTreeSet<_>>();
    assert_eq!(actual_hook_files, expected_hook_files);
    Ok(())
}

#[test]
fn destructive_launcher_preserves_github_mutation_and_credential_boundaries() -> TestResult {
    for event in ["PermissionRequest", "PreToolUse"] {
        let github_mutation = payload(event, "gh issue edit 912 --repo eunsoogi/codexy --title updated");
        assert!(run(event, github_mutation)?.is_none());

        let credential = payload(event, "gh auth token");
        assert_denied(&run(event, credential)?.ok_or("credential output")?, event);

        let destructive = payload(event, "rm -rf /");
        assert_denied(&run(event, destructive)?.ok_or("destructive output")?, event);
    }
    Ok(())
}

fn payload(event: &str, command: &str) -> Value {
    serde_json::json!({
        "hook_event_name": event,
        "tool_name": "Bash",
        "tool_input": {"command": command},
        "cwd": codexy_runtime::paths::repository_root().display().to_string(),
    })
}

fn run(event: &str, payload: Value) -> LauncherResult {
    let repository = codexy_runtime::paths::repository_root();
    let plugin = repository.join("plugins/codexy-github");
    let hooks = plugin.join("hooks");
    let mut child = Command::new(hooks.join("codexy-destructive-command.sh"))
        .arg(event)
        .env("PLUGIN_ROOT", plugin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("launcher stdin")?
        .write_all(&serde_json::to_vec(&payload)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    if output.stdout.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&output.stdout)?))
}

fn assert_denied(output: &Value, event: &str) {
    let specific = &output["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], event);
    if event == "PermissionRequest" {
        assert_eq!(specific["decision"]["behavior"], "deny");
        assert!(specific["decision"]["message"]
            .as_str()
            .unwrap_or_default()
            .starts_with("CODEXY_DESTRUCTIVE_COMMAND_"));
    } else {
        assert_eq!(specific["permissionDecision"], "deny");
        assert!(specific["permissionDecisionReason"]
            .as_str()
            .unwrap_or_default()
            .starts_with("CODEXY_DESTRUCTIVE_COMMAND_"));
    }
}
