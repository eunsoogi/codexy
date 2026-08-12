use super::{copy_github as copy, read, text, validate};
use crate::support::{FixtureCommand as Command, fixture_native_launcher};
use std::io::Write as _;
use std::process::Stdio;

const LAUNCHERS: &[&str] = &[
    "codexy-repository-issue",
    "codexy-repository-pull-request",
    "codexy-repository-merge",
    "codexy-repository-github-command",
    "codexy-destructive-command",
];

#[path = "admission_artifact/runtime_failures.rs"]
mod runtime_failures;

#[test]
fn validator_rejects_static_cross_concern_policy_imports() -> Result<(), Box<dyn std::error::Error>> {
    for injection in [
        "import codexy_policy.shell_github_policy\n",
        "marker = 1; import codexy_policy.shell_github_policy\n",
        "from codexy_policy \\\n         import shell_github_policy\n",
    ] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let policy = root.join("hooks/codexy_policy/shell_destructive.py");
        let source = std::fs::read_to_string(&policy)?;
        std::fs::write(&policy, format!("{injection}{source}"))?;
        let output = validate(&root)?;
        assert!(!output.status.success());
        assert!(
            text(&output).contains("import closure crosses concern boundary"),
            "{injection}: {}",
            text(&output)
        );
    }
    Ok(())
}

#[test]
fn validator_rejects_dynamic_cross_concern_policy_imports() -> Result<(), Box<dyn std::error::Error>> {
    for injection in [
        "import importlib as il\nil.import_module('codexy_policy.shell_github_policy')\n",
        "from importlib import (import_module as load,)\nload('codexy_policy.shell_github_policy')\n",
    ] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let policy = root.join("hooks/codexy_policy/shell_destructive.py");
        let source = std::fs::read_to_string(&policy)?;
        std::fs::write(&policy, format!("{injection}{source}"))?;
        let output = validate(&root)?;
        assert!(!output.status.success());
        assert!(
            text(&output).contains("rejects dynamic imports"),
            "{injection}: {}",
            text(&output)
        );
    }
    Ok(())
}

#[test]
fn installed_plugin_activates_the_native_github_hooks() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let hooks = read(&root.join("hooks/hooks.json"))?;
    assert_eq!(hooks["hooks"]["UserPromptSubmit"].as_array().ok_or("prompt hooks")?.len(), 1);
    let groups = hooks["hooks"]["PreToolUse"].as_array().ok_or("admission hooks")?;
    assert_eq!(groups.len(), 2);
    for group in groups {
        let handler = &group["hooks"][0];
        assert_eq!(handler["type"], "command");
        assert_eq!(handler["timeout"], 5);
        assert!(handler["command"].as_str().unwrap_or_default().contains("${PLUGIN_ROOT}/hooks/codexy-github-admission.sh"));
        assert!(handler["commandWindows"].as_str().unwrap_or_default().contains("${PLUGIN_ROOT}/hooks/codexy-github-admission-"));
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
        let output = Command::new(root.join("hooks/codexy-repository-issue.sh"))
            .arg(event)
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::null())
            .output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_REPOSITORY_ISSUE_RUNTIME"));
    }
    Ok(())
}

#[test]
fn real_launchers_hide_interpreter_failures_behind_one_denial()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::write(
        root.join("hooks/codexy-repository-issue.py"),
        "raise RuntimeError('must not leak')\n",
    )?;
    for event in ["PermissionRequest", "PreToolUse"] {
        let launcher = fixture_native_launcher(
            cfg!(windows),
            &root.join("hooks/codexy-repository-issue.sh"),
        )
        .ok_or("native repository issue launcher")?;
        let output = Command::new(launcher)
            .arg(event)
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::null())
            .output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty(), "interpreter stderr leaked");
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_REPOSITORY_ISSUE_RUNTIME"));
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
