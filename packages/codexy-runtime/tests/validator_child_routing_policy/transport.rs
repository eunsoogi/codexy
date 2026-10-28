//! Drives resolved Worker pairs through source and installed-copy hook launchers.
//! These subprocesses prove local admission, not an app task or effective host settings.

use super::*;
use crate::support::FixtureCommand as Command;
use std::{io::Write as _, path::Path, process::Stdio};

#[test]
fn issue_1442_resolved_pairs_survive_creation_and_authenticated_delivery() -> TestResult {
    let primary = primary_checkout()?;
    let temp = tempfile::tempdir()?;
    let source = root();
    let installed = temp.path().join("installed");
    crate::support::copy_dir(source.clone(), &installed)?;
    let plugin_roots = [source, installed];
    let mut assignments = vec![
        request("general", LUNA),
        request("simple", LUNA),
        bounded_request(),
    ];
    for facts in hard_work_cases()? {
        let mut hard = request("general", TERRA);
        hard["hard_work"] = facts;
        assignments.push(hard);
    }
    for assignment in assignments {
        let chosen = resolve(&assignment)?;
        let expected = if assignment.get("hard_work").is_some() {
            TERRA
        } else {
            LUNA
        };
        assert_eq!(chosen, selected("generic", "create_thread", expected));
        for plugin_root in &plugin_roots {
            for event in ["PreToolUse", "PermissionRequest"] {
                let creation = json!({
                    "hook_event_name":event, "tool_name":"mcp__codex_app__create_thread", "cwd":primary,
                    "tool_input":{
                        "model":chosen["model"], "thinking":chosen["thinking"],
                        "target":{"type":"project", "projectId":"routing-proof",
                            "environment":{"type":"worktree"}}
                    }
                });
                assert!(!denied(
                    plugin_root,
                    "codexy-child-thread-creation.sh",
                    event,
                    &creation
                )?);
                let mut crossed = creation.clone();
                crossed["tool_input"]["thinking"] = json!("high");
                assert!(denied(
                    plugin_root,
                    "codexy-child-thread-creation.sh",
                    event,
                    &crossed
                )?);

                let mut followup = request("general", expected);
                followup["schema"] = json!("codexy.child-routing-request.v2");
                followup["codex_thread_operation"] = json!("send_message_to_thread");
                followup["codex_thread_direction"] = json!("parent_to_generic");
                followup["recipient_assignment"] =
                    json!({"model":chosen["model"],"thinking":chosen["thinking"]});
                let delivered = resolve(&followup)?;
                assert_eq!(
                    delivered,
                    selected("parent_to_generic", "send_message_to_thread", expected)
                );
                let envelope = delivery_envelope(event, &delivered, &primary);
                assert!(!denied(plugin_root, "codexy-thread-delivery.sh", event, &envelope)?);
                let mut crossed = envelope.clone();
                crossed["tool_input"]["model"] =
                    json!(if expected == LUNA { TERRA.0 } else { LUNA.0 });
                assert!(denied(plugin_root, "codexy-thread-delivery.sh", event, &crossed)?);
                let mut wrong_recipient = envelope;
                wrong_recipient["tool_input"]["threadId"] = json!("another-worker");
                assert!(denied(
                    plugin_root,
                    "codexy-thread-delivery.sh",
                    event,
                    &wrong_recipient
                )?);
            }
        }
    }
    Ok(())
}

fn delivery_envelope(event: &str, chosen: &Value, cwd: &Path) -> Value {
    // Host-authenticated metadata is fixture input; caller prompt text never supplies identity.
    json!({
        "hook_event_name":event, "tool_name":"mcp__codex_app__send_message_to_thread",
        "cwd":cwd, "session_id":"orchestrator-proof",
        "tool_input":{"threadId":"worker-proof", "model":chosen["model"],
            "thinking":chosen["thinking"], "prompt":"Continue the assigned Worker scope."},
        "codexy_thread_delivery":{
            "schema":"codexy.thread-delivery.v2", "authenticated":true,
            "direction":"root_to_child", "sender_thread_id":"orchestrator-proof",
            "target_thread_id":"worker-proof", "target_model":chosen["model"],
            "target_thinking":chosen["thinking"]
        }
    })
}

fn denied(plugin_root: &Path, launcher: &str, event: &str, input: &Value) -> TestResult<bool> {
    let mut child = Command::new(plugin_root.join("hooks").join(launcher))
        .arg(event)
        .env("PLUGIN_ROOT", plugin_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("hook stdin")?
        .write_all(&serde_json::to_vec(input)?)?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "hook failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "hook stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if output.stdout.is_empty() {
        return Ok(false);
    }
    let result: Value = serde_json::from_slice(&output.stdout)?;
    Ok(if event == "PermissionRequest" {
        result["hookSpecificOutput"]["decision"]["behavior"] == "deny"
    } else {
        result["hookSpecificOutput"]["permissionDecision"] == "deny"
    })
}

fn primary_checkout() -> TestResult<std::path::PathBuf> {
    let output = std::process::Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;
    assert!(output.status.success());
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .ok_or("primary checkout")?
        .into())
}
