//! Checks field-specific, model-visible corrections without assigning a role.

use serde_json::json;

use super::thread_delivery_support::{plugin_roots, reason, run_with_tool};
use crate::support::TestResult;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];
const TOOLS: &[&str] = &[
    "codex_app__send_message_to_thread",
    "mcp__codex_app__send_message_to_thread",
];

#[test]
fn missing_or_empty_fields_name_the_edit_and_preserve_assignment_context() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("unused-transcript.jsonl");
    let roots = plugin_roots(temp.path())?;
    let cases = [
        (
            "missing recipient",
            json!({"model":"gpt-6-luna","thinking":"max"}),
            "MISSING_RECIPIENT",
            "threadId",
        ),
        (
            "empty recipient",
            json!({"threadId":" ","model":"gpt-6-luna","thinking":"max"}),
            "MISSING_RECIPIENT",
            "threadId",
        ),
        (
            "missing model",
            json!({"threadId":"known-thread","thinking":"max"}),
            "MISSING_MODEL",
            "model",
        ),
        (
            "missing thinking",
            json!({"threadId":"known-thread","model":"gpt-6-luna"}),
            "MISSING_THINKING",
            "thinking",
        ),
        (
            "missing pair",
            json!({"threadId":"known-thread"}),
            "MISSING_ROUTE_FIELDS",
            "model and thinking",
        ),
        ("non-object input", json!(null), "MISSING_DELIVERY_FIELDS", "threadId"),
    ];

    for root in &roots {
        for event in EVENTS {
            for tool in TOOLS {
                for (label, tool_input, code, field) in &cases {
                    let reason = reason(
                        run_with_tool(
                            root,
                            event,
                            tool,
                            &transcript,
                            tool_input.clone(),
                            None,
                        )?,
                        event,
                    )?;
                    assert!(reason.contains(code), "{event} {label}: {reason}");
                    assert!(reason.contains(field), "{event} {label}: {reason}");
                    for allowed_value in [
                        "gpt-6.1-sol",
                        "low",
                        "gpt-6-luna",
                        "max",
                        "gpt-6.1-terra",
                        "medium",
                    ] {
                        assert!(reason.contains(allowed_value), "{event} {label}: {reason}");
                    }
                    assert!(reason.contains("known recipient"), "{event} {label}: {reason}");
                    assert!(reason.contains("assigned pair"), "{event} {label}: {reason}");
                    assert!(reason.contains("correct"), "{event} {label}: {reason}");
                    assert!(reason.contains("retry only"), "{event} {label}: {reason}");
                    if *field == "threadId" {
                        assert!(
                            reason.contains("threadId from the assignment"),
                            "{event} {label}: {reason}"
                        );
                        assert!(reason.contains("do not guess"), "{event} {label}: {reason}");
                        assert!(
                            reason.contains("correct only the missing"),
                            "{event} {label}: {reason}"
                        );
                    }
                    assert!(!reason.contains("routing metadata"), "{event} {label}: {reason}");
                    assert!(
                        !reason.contains("CODEXY_THREAD_DELIVERY_MISSING_ROUTING_METADATA"),
                        "{event} {label}: {reason}"
                    );
                }
            }
        }
    }
    Ok(())
}
