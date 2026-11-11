//! Checks actionable pair errors and retains the outer event/tool boundary.

use serde_json::json;

use super::thread_delivery_support::{
    assert_denied, input, plugin_roots, reason, run_with_tool, run_with_unregistered_event,
};
use crate::support::TestResult;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];
const TOOLS: &[&str] = &[
    "codex_app__send_message_to_thread",
    "mcp__codex_app__send_message_to_thread",
];

#[test]
fn unsupported_models_and_crossed_pairs_have_actionable_corrections() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("unused-transcript.jsonl");
    let roots = plugin_roots(temp.path())?;
    let cases = [
        (
            "unsupported model",
            "gpt-5.6-terra",
            "medium",
            "UNSUPPORTED_MODEL",
            "model",
        ),
        (
            "unsupported effort",
            "gpt-6-luna",
            "ultra",
            "UNSUPPORTED_THINKING",
            "thinking",
        ),
        (
            "Sol crossed with max",
            "gpt-6.1-sol",
            "max",
            "UNSUPPORTED_THINKING",
            "thinking",
        ),
        (
            "Sol crossed with medium",
            "gpt-6.1-sol",
            "medium",
            "UNSUPPORTED_THINKING",
            "thinking",
        ),
        (
            "Luna crossed with low",
            "gpt-6-luna",
            "low",
            "UNSUPPORTED_THINKING",
            "thinking",
        ),
        (
            "Luna crossed with medium",
            "gpt-6-luna",
            "medium",
            "UNSUPPORTED_THINKING",
            "thinking",
        ),
        (
            "Terra crossed with low",
            "gpt-6.1-terra",
            "low",
            "UNSUPPORTED_THINKING",
            "thinking",
        ),
        (
            "Terra crossed with max",
            "gpt-6.1-terra",
            "max",
            "UNSUPPORTED_THINKING",
            "thinking",
        ),
    ];

    for root in &roots {
        for event in EVENTS {
            for tool in TOOLS {
                for (label, model, thinking, code, field) in &cases {
                    let reason = reason(
                        run_with_tool(
                            root,
                            event,
                            tool,
                            &transcript,
                            input("known-thread", model, thinking, "receipt"),
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
                    assert!(!reason.contains("guess a thread ID"), "{event} {label}: {reason}");
                }
            }
        }
    }
    Ok(())
}

#[test]
fn unexpected_tool_remains_an_envelope_denial() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("unused-transcript.jsonl");
    let roots = plugin_roots(temp.path())?;
    for root in &roots {
        for event in EVENTS {
            assert_denied(
                run_with_tool(
                    root,
                    event,
                    "codex_app__send_message",
                    &transcript,
                    input("known-thread", "gpt-6.1-sol", "low", "receipt"),
                    Some(json!("malformed legacy metadata")),
                )?,
                event,
                "CODEXY_THREAD_DELIVERY_ENVELOPE",
            )?;
        }
    }
    Ok(())
}

#[test]
fn unexpected_event_is_rejected_by_the_launcher() -> TestResult {
    let temp = tempfile::tempdir()?;
    let roots = plugin_roots(temp.path())?;
    for root in &roots {
        let output = run_with_unregistered_event(root, "UnknownEvent")?;
        assert!(!output.status.success(), "unexpected event was accepted");
        assert!(output.stdout.is_empty(), "unexpected event emitted a decision");
    }
    Ok(())
}

#[test]
fn launcher_fallbacks_keep_the_same_runtime_diagnostic_on_both_platforms() -> TestResult {
    let hooks = codexy_runtime::paths::repository_root().join("plugins/codexy/hooks");
    for launcher in ["codexy-thread-delivery.sh", "codexy-thread-delivery.cmd"] {
        let source = std::fs::read_to_string(hooks.join(launcher))?;
        assert!(
            source.contains("CODEXY_THREAD_DELIVERY_RUNTIME"),
            "{launcher}"
        );
        assert!(source.contains("MUST NOT execute"), "{launcher}");
        assert!(source.contains("PermissionRequest"), "{launcher}");
        assert!(source.contains("PreToolUse"), "{launcher}");
    }
    Ok(())
}
