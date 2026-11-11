//! Builds ordinary request fixtures and runs source or installed hook launchers.

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::support::TestResult;

pub(super) const LEGACY_METADATA_FIELD: &str = "codexy_thread_delivery";

pub(super) fn input(recipient: &str, model: &str, thinking: &str, prompt: &str) -> Value {
    json!({
        "threadId":recipient,
        "model":model,
        "thinking":thinking,
        "prompt":prompt
    })
}

pub(super) fn plugin_roots(temp: &Path) -> TestResult<[PathBuf; 2]> {
    let installed = temp.join("installed");
    crate::support::copy_dir(
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        &installed,
    )?;
    Ok([
        codexy_runtime::paths::repository_root().join("plugins/codexy"),
        installed,
    ])
}

pub(super) fn run_with_tool(
    root: &Path,
    event: &str,
    tool_name: &str,
    transcript: &Path,
    tool_input: Value,
    legacy_metadata: Option<Value>,
) -> TestResult<std::process::Output> {
    // Legacy metadata is included only to prove it cannot change request admission.
    let mut payload = json!({
        "hook_event_name":event,
        "tool_name":tool_name,
        "tool_input":tool_input,
        "transcript_path":transcript
    });
    if let Some(metadata) = legacy_metadata {
        payload[LEGACY_METADATA_FIELD] = metadata;
    }
    let payload = serde_json::to_vec(&payload)?;
    let mut command = launcher(root);
    let mut child = command
        .arg(event)
        .env("PLUGIN_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().ok_or("hook stdin")?.write_all(&payload)?;
    Ok(child.wait_with_output()?)
}

pub(super) fn reason(output: std::process::Output, event: &str) -> TestResult<String> {
    assert!(output.status.success(), "hook failed");
    assert!(output.stderr.is_empty(), "hook wrote stderr");
    let denial: Value = serde_json::from_slice(&output.stdout)?;
    let specific = &denial["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], event);
    assert!(
        specific.get("additionalContext").is_none(),
        "{event}: denial included additional context"
    );
    let reason = if event == "PermissionRequest" {
        specific["decision"]["message"].as_str().ok_or("permission message")?
    } else {
        specific["permissionDecisionReason"].as_str().ok_or("permission reason")?
    };
    Ok(reason.to_owned())
}

pub(super) fn assert_admitted(
    output: std::process::Output,
    event: &str,
    label: &str,
) -> TestResult {
    assert!(output.status.success(), "{event} {label}: hook failed");
    assert!(
        output.stdout.is_empty(),
        "{event} {label}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stderr.is_empty(), "{event} {label}: hook wrote stderr");
    Ok(())
}

pub(super) fn assert_denied(
    output: std::process::Output,
    event: &str,
    expected: &str,
) -> TestResult {
    let reason = reason(output, event)?;
    assert!(reason.contains(expected), "{event}: {reason}");
    Ok(())
}

#[cfg(windows)]
fn launcher(root: &Path) -> Command {
    let mut command = Command::new("cmd");
    command
        .arg("/d")
        .arg("/c")
        .arg(root.join("hooks/codexy-thread-delivery.cmd"));
    command
}

#[cfg(not(windows))]
fn launcher(root: &Path) -> Command {
    Command::new(root.join("hooks/codexy-thread-delivery.sh"))
}
