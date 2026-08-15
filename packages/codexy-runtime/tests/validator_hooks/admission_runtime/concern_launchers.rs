use std::io::Write as _;
use std::path::Path;
use std::process::Stdio;

use crate::support::FixtureCommand as Command;
use serde_json::Value;

use super::TestResult;

pub(super) fn assert_input(
    root: &Path,
    input: Value,
    denied: bool,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult {
    let description = input.to_string();
    let event = input["hook_event_name"].as_str().ok_or("event")?;
    let tool = input["tool_name"].as_str().ok_or("tool")?;
    let launchers = launchers(tool)?;
    let mut denials = 0;
    let mut diagnostics = Vec::new();
    for launcher in &launchers {
        let mut child = Command::new(root.join("hooks").join(format!("{launcher}.sh")));
        child.arg(event);
        child.env_clear();
        if let Some(path) = std::env::var_os("PATH") {
            child.env("PATH", path);
        }
        child.env_path("PLUGIN_ROOT", root);
        child.envs(environment.iter().copied());
        child
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = child.spawn()?;
        child
            .stdin
            .take()
            .ok_or("stdin")?
            .write_all(&serde_json::to_vec(&input)?)?;
        let output = child.wait_with_output()?;
        diagnostics.push(String::from_utf8_lossy(&output.stderr).into_owned());
        assert!(
            output.status.success(),
            "launcher failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if output.stdout.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("invalid denial for {description}: {error}"))?;
        let decision = if event == "PermissionRequest" {
            &value["hookSpecificOutput"]["decision"]["behavior"]
        } else {
            &value["hookSpecificOutput"]["permissionDecision"]
        };
        assert_eq!(decision, "deny", "{description}");
        denials += 1;
    }
    assert_eq!(
        denials > 0,
        denied,
        "{description}; launchers={launchers:?}; stderr={diagnostics:?}",
    );
    Ok(())
}

fn launchers(tool: &str) -> TestResult<Vec<&'static str>> {
    match tool {
        "Bash" => Ok(vec![
            "codexy-repository-github-command",
            "codexy-destructive-command",
        ]),
        "codex_app__send_message_to_thread" => Ok(vec!["codexy-thread-delivery"]),
        "mcp__codex_apps__github_create_issue" | "mcp__codex_apps__github_update_issue" => {
            Ok(vec!["codexy-repository-issue"])
        }
        "mcp__codex_apps__github_create_pull_request"
        | "mcp__codex_apps__github_update_pull_request" => {
            Ok(vec!["codexy-repository-pull-request"])
        }
        "mcp__codex_apps__github_merge_pull_request"
        | "mcp__codex_apps__github_enable_auto_merge" => {
            Ok(vec!["codexy-repository-merge"])
        }
        _ => Err(format!("no concern launcher for {tool}").into()),
    }
}

#[test]
fn bash_concerns_have_independent_positive_and_negative_owners() -> TestResult {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy-github");
    let workspace = tempfile::tempdir()?;
    let owned = workspace.path().join("owned");
    std::fs::create_dir_all(owned.join(".git"))?;
    std::fs::write(
        owned.join(".git/config"),
        "[remote \"origin\"]\n\turl = git@github.com:eunsoogi/codexy.git\n",
    )?;
    let policy = owned.join(".codex/repository-github-policy.json");
    std::fs::create_dir_all(policy.parent().ok_or("policy parent")?)?;
    std::fs::write(
        policy,
        "{\"schema\":\"codexy.repository-github-policy/v1\",\"repository\":\"eunsoogi/codexy\"}",
    )?;
    for (launcher, command, denied) in [
        ("codexy-repository-github-command", "gh issue create --title invalid", true),
        ("codexy-destructive-command", "gh issue create --title invalid", false),
        ("codexy-repository-github-command", "git push --force origin topic", false),
        ("codexy-destructive-command", "git push --force origin topic", true),
    ] {
        let input = serde_json::json!({
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": command},
            "cwd": owned,
        });
        let output = run_launcher(&root, launcher, "PreToolUse", &input, &[])?;
        assert_eq!(!output.is_empty(), denied, "{launcher}: {command}");
    }
    Ok(())
}

pub(super) fn run_launcher(
    root: &Path,
    launcher: &str,
    event: &str,
    input: &Value,
    environment: &[(&str, &std::ffi::OsStr)],
) -> TestResult<Vec<u8>> {
    let mut child = Command::new(root.join("hooks").join(format!("{launcher}.sh")));
    child.arg(event).env_path("PLUGIN_ROOT", root);
    child.envs(environment.iter().copied());
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = child.spawn()?;
    child
        .stdin
        .take()
        .ok_or("stdin")?
        .write_all(&serde_json::to_vec(input)?)?;
    let output = child.wait_with_output()?;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    Ok(output.stdout)
}
