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
fn field_denials_are_actionable_on_authoritative_and_installed_hooks() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("child.jsonl");
    std::fs::write(&transcript, child_transcript(CHILD, &[PARENT]))?;
    let roots = plugin_roots(temp.path())?;
    let cases = [
        (
            "missing model",
            json!({"threadId":PARENT,"thinking":"medium"}),
            "MISSING_MODEL",
            ["model", "gpt-5.6-sol", "retry once"],
        ),
        (
            "missing thinking",
            json!({"threadId":PARENT,"model":"gpt-5.6-sol"}),
            "MISSING_THINKING",
            ["thinking", "medium", "retry once"],
        ),
        (
            "unsupported model",
            json!({"threadId":PARENT,"model":"gpt-5.6-terra","thinking":"medium"}),
            "UNSUPPORTED_MODEL",
            ["model", "gpt-5.6-sol", "retry once"],
        ),
        (
            "unsupported thinking",
            json!({"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"high"}),
            "UNSUPPORTED_THINKING",
            ["thinking", "medium", "retry once"],
        ),
        (
            "unsupported model and thinking",
            json!({"threadId":PARENT,"model":"gpt-5.6-luna","thinking":"max"}),
            "UNSUPPORTED_MODEL",
            ["model", "gpt-5.6-sol", "medium"],
        ),
        (
            "wrong recipient",
            json!({"threadId":OTHER,"model":"gpt-5.6-sol","thinking":"medium"}),
            "WRONG_RECIPIENT",
            ["threadId", "authenticated parent", "retry once"],
        ),
        (
            "missing model and wrong recipient",
            json!({"threadId":OTHER,"thinking":"medium"}),
            "MISSING_MODEL",
            ["threadId", "authenticated parent", "gpt-5.6-sol"],
        ),
    ];
    for event in EVENTS {
        for (label, tool_input, code, terms) in &cases {
            for root in &roots {
                let reason = reason(run(root, event, &transcript, Some(CHILD), tool_input.clone())?, event)?;
                assert!(reason.contains(code), "{event} {label}: {reason}");
                for term in terms {
                    assert!(reason.to_ascii_lowercase().contains(&term.to_ascii_lowercase()), "{event} {label}: {reason}");
                }
                assert!(!reason.contains(PARENT), "{event} {label} leaked parent id: {reason}");
                assert!(!reason.contains(OTHER), "{event} {label} leaked recipient id: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn identity_and_context_denials_are_distinct_and_non_leaking() -> TestResult {
    let temp = tempfile::tempdir()?;
    let malformed = temp.path().join("malformed.jsonl");
    std::fs::write(&malformed, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    for event in EVENTS {
        for root in &roots {
            let input = json!({"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"medium"});
            let missing = reason(run(root, event, &malformed, None, input.clone())?, event)?;
            assert!(missing.contains("MISSING_IDENTITY") && missing.contains("MUST NOT retry blindly"), "{event}: {missing}");
            assert!(!missing.contains(PARENT), "{event} leaked parent id: {missing}");

            let context = reason(run(root, event, &malformed, Some(CHILD), input)?, event)?;
            assert!(context.contains("UNTRUSTED_CONTEXT") && context.contains("MUST NOT retry blindly"), "{event}: {context}");
            assert!(!context.contains(CHILD), "{event} leaked session id: {context}");
            assert!(!context.contains(PARENT), "{event} leaked parent id: {context}");
            assert!(!context.contains("not-json"), "{event} leaked transcript: {context}");
        }
    }
    Ok(())
}

#[test]
fn authenticated_child_and_root_to_child_routes_remain_admitted_for_both_events() -> TestResult {
    let temp = tempfile::tempdir()?;
    let child = temp.path().join("child.jsonl");
    std::fs::write(&child, child_transcript(CHILD, &[PARENT]))?;
    let root = temp.path().join("root.jsonl");
    std::fs::write(&root, root_transcript(PARENT))?;
    let roots = plugin_roots(temp.path())?;
    for event in EVENTS {
        for plugin in &roots {
            let child_output = run(
                plugin,
                event,
                &child,
                Some(CHILD),
                json!({"threadId":PARENT,"model":"gpt-5.6-sol","thinking":"medium"}),
            )?;
            assert!(child_output.status.success());
            assert!(child_output.stdout.is_empty(), "{event}: child route denied");
            let root_output = run(
                plugin,
                event,
                &root,
                Some(PARENT),
                json!({"threadId":CHILD,"model":"gpt-5.6-luna","thinking":"max"}),
            )?;
            assert!(root_output.status.success());
            assert!(root_output.stdout.is_empty(), "{event}: root route denied");
        }
    }
    Ok(())
}

#[test]
fn launcher_fallbacks_keep_the_same_runtime_diagnostic_on_both_platforms() -> TestResult {
    let hooks = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    for launcher in ["codexy-thread-delivery.sh", "codexy-thread-delivery.cmd"] {
        let source = std::fs::read_to_string(hooks.join(launcher))?;
        assert!(source.contains("CODEXY_THREAD_DELIVERY_RUNTIME"), "{launcher}");
        assert!(source.contains("MUST NOT execute"), "{launcher}");
        assert!(source.contains("PermissionRequest"), "{launcher}");
        assert!(source.contains("PreToolUse"), "{launcher}");
    }
    Ok(())
}

fn child_transcript(session: &str, parents: &[&str]) -> Vec<u8> {
    let content = parents
        .iter()
        .map(|parent| json!({"type":"input_text","text":delegation(parent)}))
        .collect::<Vec<_>>();
    transcript(session, content)
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
    transcript(
        session,
        vec![json!({"type":"input_text","text":"Root-owned work."})],
    )
}

fn transcript(session: &str, content: Vec<Value>) -> Vec<u8> {
    let lines = [
        json!({"type":"session_meta","payload":{"id":session,"session_id":session}}).to_string(),
        json!({"type":"turn_context","payload":{}}).to_string(),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":content}}).to_string(),
    ];
    format!("{}\n", lines.join("\n")).into_bytes()
}

fn delegation(parent: &str) -> String {
    format!("<codex_delegation><source_thread_id>{parent}</source_thread_id><input>Owned lane.</input></codex_delegation>")
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
