use super::{copy_github as copy, text, validate};
use crate::support::{FixtureCommand as Command, fixture_native_launcher};
use serde_json::json;
use std::io::Write as _;
use std::process::Stdio;

const LAUNCHERS: &[&str] = &["codexy-destructive-command"];

#[test]
fn validator_rejects_unpinned_policy_imports() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let policy = root.join("hooks/codexy_policy/shell_destructive.py");
    let source = std::fs::read_to_string(&policy)?;
    std::fs::write(
        &policy,
        format!("import codexy_policy.not_packaged\n{source}"),
    )?;
    let output = validate(&root)?;
    assert!(!output.status.success());
    assert!(text(&output).contains("import is unpinned"), "{}", text(&output));
    Ok(())
}

#[test]
fn materialized_plugin_executes_the_retained_safety_hook_for_both_events()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    for event in ["PermissionRequest", "PreToolUse"] {
        for launcher in LAUNCHERS {
            let input = json!({
                "hook_event_name": event,
                "tool_name": "Bash",
                "tool_input": null,
                "cwd": temp.path(),
            });
            let shell = root.join(format!("hooks/{launcher}.sh"));
            let native = fixture_native_launcher(cfg!(windows), &shell).ok_or("native launcher")?;
            let mut child = Command::new(native)
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
            assert!(output.stdout.windows(10).any(|window| window == b"UNRESOLVED"));
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn materialized_launcher_fails_closed_when_shared_runtime_is_unavailable()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::write(root.join("hooks/codexy-hook-runtime.sh"), "#!/bin/sh\nexit 1\n")?;
    for event in ["PermissionRequest", "PreToolUse"] {
        let output = Command::new(root.join("hooks/codexy-destructive-command.sh"))
            .arg(event)
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::null())
            .output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_DESTRUCTIVE_COMMAND_RUNTIME"));
    }
    Ok(())
}

#[test]
fn real_launcher_hides_interpreter_failures_behind_one_denial()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::write(
        root.join("hooks/codexy-destructive-command.py"),
        "raise RuntimeError('must not leak')\n",
    )?;
    for event in ["PermissionRequest", "PreToolUse"] {
        let launcher = fixture_native_launcher(
            cfg!(windows),
            &root.join("hooks/codexy-destructive-command.sh"),
        )
        .ok_or("native launcher")?;
        let output = Command::new(launcher)
            .arg(event)
            .env("PLUGIN_ROOT", &root)
            .stdin(Stdio::null())
            .output()?;
        assert!(output.status.success());
        assert!(output.stderr.is_empty(), "interpreter stderr leaked");
        let denial: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        assert_eq!(denial["hookSpecificOutput"]["hookEventName"], event);
        assert!(String::from_utf8(output.stdout)?.contains("CODEXY_DESTRUCTIVE_COMMAND_RUNTIME"));
    }
    Ok(())
}

#[test]
fn shared_envelope_fails_closed_at_the_core_input_boundary()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let cases = [
        vec![0xff],
        br#"{"hook_event_name":"PreToolUse","hook_event_name":"PreToolUse","tool_name":"mcp__codex_app__send_message_to_thread"}"#.to_vec(),
        br#"{"hook_event_name":"PermissionRequest","tool_name":"mcp__codex_app__send_message_to_thread","tool_input":null}"#.to_vec(),
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
        child.stdin.take().ok_or("launcher stdin")?.write_all(&payload)?;
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
