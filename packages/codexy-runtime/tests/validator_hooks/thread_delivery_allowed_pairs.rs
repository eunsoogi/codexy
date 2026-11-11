//! Confirms request-level pair admission for both hook events and tool aliases.

use serde_json::{Value, json};

use super::thread_delivery_support::{
    assert_admitted, assert_denied, input, plugin_roots, run_with_tool,
};
use crate::support::TestResult;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];
const TOOLS: &[&str] = &[
    "codex_app__send_message_to_thread",
    "mcp__codex_app__send_message_to_thread",
];
const PAIRS: &[(&str, &str)] = &[
    ("gpt-6.1-sol", "low"),
    ("gpt-6-luna", "max"),
    ("gpt-6.1-terra", "medium"),
];

#[test]
fn all_allowed_pairs_admit_without_custom_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    // A nonexistent transcript demonstrates that request admission does not read one.
    let transcript = temp.path().join("unused-transcript.jsonl");
    let roots = plugin_roots(temp.path())?;

    for root in &roots {
        for event in EVENTS {
            for tool in TOOLS {
                for (model, thinking) in PAIRS {
                    assert_admitted(
                        run_with_tool(
                            root,
                            event,
                            tool,
                            &transcript,
                            input("known-thread", model, thinking, "Continue the assigned lane."),
                            None,
                        )?,
                        event,
                        "allowed pair without routing metadata",
                    )?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn legacy_metadata_presence_and_shape_do_not_change_pair_admission() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("unused-transcript.jsonl");
    let roots = plugin_roots(temp.path())?;
    let legacy_values: [Option<Value>; 4] = [
        None,
        Some(json!({
            "schema":"codexy.thread-delivery.v2",
            "authenticated":true,
            "direction":"child_to_parent",
            "sender_thread_id":"different-sender",
            "target_thread_id":"different-recipient",
            "target_model":"gpt-6.1-sol",
            "target_thinking":"low"
        })),
        Some(json!("not-an-envelope")),
        Some(json!({"authenticated":true})),
    ];

    for root in &roots {
        for event in EVENTS {
            for tool in TOOLS {
                for (model, thinking) in PAIRS {
                    for legacy_metadata in legacy_values.iter().cloned() {
                        assert_admitted(
                            run_with_tool(
                                root,
                                event,
                                tool,
                                &transcript,
                                input("known-thread", model, thinking, "same request"),
                                legacy_metadata,
                            )?,
                            event,
                            "legacy metadata must not gate a valid pair",
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[test]
fn legacy_metadata_cannot_admit_a_crossed_pair() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("unused-transcript.jsonl");
    let roots = plugin_roots(temp.path())?;
    let metadata_values = [
        json!({
            "schema":"codexy.thread-delivery.v2",
            "authenticated":true,
            "direction":"child_to_parent",
            "sender_thread_id":"known-worker",
            "target_thread_id":"known-thread",
            "target_model":"gpt-6.1-sol",
            "target_thinking":"low"
        }),
        json!("not-an-envelope"),
        json!({"authenticated":true}),
    ];

    for root in &roots {
        for event in EVENTS {
            for tool in TOOLS {
                let request = input("known-thread", "gpt-6.1-sol", "max", "receipt");
                let baseline = run_with_tool(
                    root,
                    event,
                    tool,
                    &transcript,
                    request.clone(),
                    None,
                )?;
                let baseline_stdout = baseline.stdout.clone();
                assert_denied(baseline, event, "CODEXY_THREAD_DELIVERY_UNSUPPORTED_THINKING")?;

                for metadata in metadata_values.iter().cloned() {
                    let result = run_with_tool(
                        root,
                        event,
                        tool,
                        &transcript,
                        request.clone(),
                        Some(metadata),
                    )?;
                    assert_eq!(
                        result.stdout, baseline_stdout,
                        "legacy metadata changed the crossed-pair denial for {event} {tool}"
                    );
                    assert_denied(
                        result,
                        event,
                        "CODEXY_THREAD_DELIVERY_UNSUPPORTED_THINKING",
                    )?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn nonempty_recipient_shape_does_not_claim_target_authorization() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("unused-transcript.jsonl");
    let roots = plugin_roots(temp.path())?;
    for root in &roots {
        for event in EVENTS {
            assert_admitted(
                run_with_tool(
                    root,
                    event,
                    "mcp__codex_app__send_message_to_thread",
                    &transcript,
                    input("another-known-thread", "gpt-6-luna", "max", "receipt"),
                    Some(json!({"target_thread_id":"different-thread"})),
                )?,
                event,
                "nonempty recipient and allowed pair",
            )?;
        }
    }
    Ok(())
}
