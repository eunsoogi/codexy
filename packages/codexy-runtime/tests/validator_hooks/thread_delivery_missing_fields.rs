use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::support::TestResult;

const CHILD: &str = "01a03690-a037-7141-af1c-1d7cdb093087";
const PARENT: &str = "01a02ebc-804e-7702-a0ca-503c50741db8";
const OTHER: &str = "01a031f2-da35-7960-9686-502b7e373676";
const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

#[test]
fn root_missing_fields_use_root_to_child_remedy() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("root.jsonl");
    std::fs::write(&transcript, root_transcript(PARENT))?;
    let roots = plugin_roots(temp.path())?;
    let cases = [
        (
            "missing model",
            json!({"threadId":CHILD,"thinking":"max"}),
            "MISSING_MODEL",
            "model",
        ),
        (
            "missing thinking",
            json!({"threadId":CHILD,"model":"gpt-5.6-luna"}),
            "MISSING_THINKING",
            "thinking",
        ),
    ];
    for event in EVENTS {
        for (label, tool_input, code, field) in &cases {
            for root in &roots {
                let reason = reason(run(root, event, &transcript, Some(PARENT), tool_input.clone())?, event)?;
                assert!(reason.contains(code), "{event} {label}: {reason}");
                assert!(reason.contains(field), "{event} {label}: {reason}");
                assert!(reason.contains("root-to-child"), "{event} {label}: {reason}");
                assert!(reason.contains("MUST"), "{event} {label}: {reason}");
                assert!(!reason.contains("child-to-parent"), "{event} {label}: {reason}");
                assert!(!reason.contains("authenticated parent"), "{event} {label}: {reason}");
                assert!(!reason.contains("gpt-5.6-sol"), "{event} {label}: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn agent_facing_diagnostics_use_mandatory_modal_wording() -> TestResult {
    let temp = tempfile::tempdir()?;
    let child = temp.path().join("child.jsonl");
    std::fs::write(&child, child_transcript(CHILD, PARENT))?;
    let malformed = temp.path().join("malformed.jsonl");
    std::fs::write(&malformed, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    let cases = [
        (
            "missing model",
            &child,
            Some(CHILD),
            json!({"threadId":PARENT,"thinking":"medium"}),
        ),
        (
            "missing thinking",
            &child,
            Some(CHILD),
            json!({"threadId":PARENT,"model":"gpt-5.6-sol"}),
        ),
        (
            "unsupported model",
            &child,
            Some(CHILD),
            json!({"threadId":PARENT,"model":"gpt-5.6-terra","thinking":"medium"}),
        ),
        (
            "unsupported thinking",
            &child,
            Some(CHILD),
            json!({"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"high"}),
        ),
        (
            "wrong recipient",
            &child,
            Some(CHILD),
            json!({"threadId":OTHER,"model":"gpt-5.6-sol","thinking":"medium"}),
        ),
        (
            "missing identity",
            &malformed,
            None,
            json!({"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"medium"}),
        ),
        (
            "untrusted context",
            &malformed,
            Some(CHILD),
            json!({"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"medium"}),
        ),
    ];
    for event in EVENTS {
        for (label, transcript, session, tool_input) in &cases {
            for root in &roots {
                let reason = reason(run(root, event, transcript, *session, tool_input.clone())?, event)?;
                assert!(reason.contains("MUST"), "{event} {label}: {reason}");
                assert!(!reason.contains("do not"), "{event} {label}: {reason}");
                let required = match *label {
                    "missing model" | "missing thinking" => {
                        &["MUST correct", "MUST retry once"][..]
                    }
                    "unsupported model" => {
                        &["MUST use", "MUST correct", "MUST retry once"][..]
                    }
                    "unsupported thinking" => {
                        &["MUST use", "MUST correct", "MUST retry once"][..]
                    }
                    "wrong recipient" => &[
                        "MUST set",
                        "MUST use",
                        "MUST correct",
                        "MUST retry once",
                        "MUST NOT guess",
                    ][..],
                    "missing identity" => &["MUST NOT retry blindly", "MUST retry only"][..],
                    "untrusted context" => &["MUST NOT retry blindly", "MUST stop"][..],
                    _ => unreachable!("unexpected diagnostic case"),
                };
                for term in required {
                    assert!(reason.contains(term), "{event} {label}: {reason}");
                }
                if *session == Some(CHILD) && *label != "untrusted context" {
                    for term in ["child-to-parent", "threadId", "gpt-5.6-sol", "medium"] {
                        assert!(reason.contains(term), "{event} {label}: {reason}");
                    }
                }
            }
        }
    }
    Ok(())
}

fn plugin_roots(temp: &Path) -> TestResult<[PathBuf; 2]> {
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

fn root_transcript(session: &str) -> Vec<u8> {
    let mut bytes = transcript(session, "Root-owned work.");
    bytes.extend_from_slice(
        format!(
            "{}\n",
            json!({"type":"response_item","payload":{"type":"function_call_output","name":"create_thread","namespace":"codex_app","output":json!({"threadId":CHILD,"parentThreadId":session}).to_string()}})
        )
        .as_bytes(),
    );
    bytes
}

fn child_transcript(session: &str, parent: &str) -> Vec<u8> {
    transcript(
        session,
        &format!(
            "<codex_delegation><source_thread_id>{parent}</source_thread_id><input>Owned lane.</input></codex_delegation>"
        ),
    )
}

fn transcript(session: &str, text: &str) -> Vec<u8> {
    let lines = [
        json!({"type":"session_meta","payload":{"id":session,"session_id":session}}).to_string(),
        json!({"type":"turn_context","payload":{}}).to_string(),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":text}]}}).to_string(),
    ];
    format!("{}\n", lines.join("\n")).into_bytes()
}

fn run(
    root: &Path,
    event: &str,
    transcript: &Path,
    session: Option<&str>,
    tool_input: Value,
) -> TestResult<std::process::Output> {
    let mut payload = json!({
        "hook_event_name": event,
        "tool_name": "codex_app__send_message_to_thread",
        "tool_input": tool_input,
        "transcript_path": transcript,
    });
    if let Some(session) = session {
        payload["session_id"] = json!(session);
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

fn reason(output: std::process::Output, event: &str) -> TestResult<String> {
    assert!(output.status.success(), "hook failed: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "hook wrote stderr: {}", String::from_utf8_lossy(&output.stderr));
    let denial: Value = serde_json::from_slice(&output.stdout)?;
    let specific = &denial["hookSpecificOutput"];
    assert_eq!(specific["hookEventName"], event);
    let reason = if event == "PermissionRequest" {
        specific["decision"]["message"].as_str().ok_or("permission message")?
    } else {
        specific["permissionDecisionReason"].as_str().ok_or("permission reason")?
    };
    Ok(reason.to_owned())
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
