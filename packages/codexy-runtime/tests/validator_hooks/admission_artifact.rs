use super::{copy, read};
use crate::support::FixtureCommand as Command;
use std::io::Write as _;
use std::process::Stdio;

const LAUNCHERS: &[&str] = &[
    "codexy-thread-delivery",
    "codexy-repository-issue",
    "codexy-repository-pull-request",
    "codexy-repository-merge",
    "codexy-repository-github-command",
    "codexy-destructive-command",
];

#[test]
fn packaged_concern_hooks_are_reachable() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let hooks = read(&root.join("hooks/hooks.json"))?;
    for event in ["PermissionRequest", "PreToolUse"] {
        let groups = hooks["hooks"][event].as_array().ok_or("groups")?;
        assert_eq!(groups.len(), LAUNCHERS.len());
        for (group, launcher) in groups.iter().zip(LAUNCHERS) {
            let handler = &group["hooks"][0];
            assert_eq!(handler["type"], "command", "{event} {launcher}");
            assert_eq!(handler["timeout"], 5, "{event} {launcher}");
            assert_eq!(
                handler["command"],
                format!("\"${{PLUGIN_ROOT}}/hooks/{launcher}.sh\" {event}")
            );
            assert_eq!(
                handler["commandWindows"],
                format!("\"${{PLUGIN_ROOT}}/hooks/{launcher}.cmd\" {event}")
            );
        }
    }
    Ok(())
}

#[test]
fn materialized_plugin_executes_every_concern_hook_for_both_events()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let install_base = temp.path().join("installed plugin with spaces");
    let root = copy(&install_base)?;
    let tools = [
        "codex_app__send_message_to_thread",
        "mcp__codex_apps__github_create_issue",
        "mcp__codex_apps__github_create_pull_request",
        "mcp__codex_apps__github_merge_pull_request",
        "Bash",
        "Bash",
    ];
    for event in ["PermissionRequest", "PreToolUse"] {
        for (launcher, tool) in LAUNCHERS.iter().zip(tools) {
            let input = serde_json::json!({
                "hook_event_name": event,
                "tool_name": tool,
                "tool_input": null,
                "cwd": temp.path(),
            });
            let mut child = Command::new(root.join(format!("hooks/{launcher}.sh")))
                .arg(event)
                .env("PLUGIN_ROOT", &root)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            child
                .stdin
                .take()
                .ok_or("launcher stdin")?
                .write_all(&serde_json::to_vec(&input)?)?;
            let output = child.wait_with_output()?;
            assert!(output.status.success(), "{event} {launcher}");
            assert!(output.stderr.is_empty(), "{event} {launcher}");
            let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
            assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        }
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn materialized_launchers_fail_closed_when_shared_runtime_is_unavailable()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::write(root.join("hooks/codexy-hook-runtime.sh"), "#!/bin/sh\nexit 1\n")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        let output = Command::new(root.join("hooks/codexy-thread-delivery.sh"))
            .arg(event)
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::null())
            .output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_THREAD_DELIVERY_RUNTIME"));
    }
    Ok(())
}

#[test]
fn real_launchers_hide_interpreter_failures_behind_one_denial()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::write(
        root.join("hooks/codexy-thread-delivery.py"),
        "raise RuntimeError('must not leak')\n",
    )?;
    for event in ["PermissionRequest", "PreToolUse"] {
        let output = Command::new(root.join("hooks/codexy-thread-delivery.sh"))
            .arg(event)
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::null())
            .output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty(), "interpreter stderr leaked");
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_THREAD_DELIVERY_RUNTIME"));
    }
    Ok(())
}

#[test]
fn shared_envelope_fails_closed_at_every_input_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let cases = [
        vec![0xff],
        br#"{"hook_event_name":"PreToolUse","hook_event_name":"PreToolUse","tool_name":"codex_app__send_message_to_thread"}"#.to_vec(),
        br#"{"hook_event_name":"PermissionRequest","tool_name":"codex_app__send_message_to_thread","tool_input":null}"#.to_vec(),
        vec![b' '; 1024 * 1024 + 1],
    ];
    for payload in cases {
        let mut child = Command::new(root.join("hooks/codexy-thread-delivery.sh"))
            .arg("PreToolUse")
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        child.stdin.take().ok_or("stdin")?.write_all(&payload)?;
        let output = child.wait_with_output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let reason = denial["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .ok_or("reason")?;
        assert!(reason.starts_with("CODEXY_THREAD_DELIVERY_ENVELOPE"));
    }
    Ok(())
}
