use serde_json::json;

use super::thread_delivery_support::{
    CHILD, PARENT, plugin_roots, reason, route, run,
};
use crate::support::TestResult;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

#[test]
fn root_missing_fields_use_root_to_child_remedy() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
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
        for (label, input, code, field) in &cases {
            for root in &roots {
                let reason = reason(
                    run(
                        root,
                        event,
                        &transcript,
                        Some(PARENT),
                        input.clone(),
                        Some(route("root_to_child", PARENT, CHILD, "gpt-5.6-luna", "max")),
                    )?,
                    event,
                )?;
                assert!(reason.contains(code), "{event} {label}: {reason}");
                assert!(reason.contains(field), "{event} {label}: {reason}");
                assert!(reason.contains("root-to-child"), "{event} {label}: {reason}");
                assert!(reason.contains("gpt-5.6-luna"), "{event} {label}: {reason}");
                assert!(reason.contains("max"), "{event} {label}: {reason}");
                assert!(reason.contains("MUST"), "{event} {label}: {reason}");
                assert!(!reason.contains("child-to-parent"), "{event} {label}: {reason}");
                assert!(!reason.contains("gpt-6-astra"), "{event} {label}: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn child_missing_fields_use_child_to_parent_remedy() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    for event in EVENTS {
        for root in &roots {
            for input in [
                json!({"threadId":PARENT,"thinking":"medium"}),
                json!({"threadId":PARENT,"model":"gpt-6-astra"}),
            ] {
                let reason = reason(
                    run(
                        root,
                        event,
                        &transcript,
                        Some(CHILD),
                        input,
                    Some(route("child_to_parent", CHILD, PARENT, "gpt-6-astra", "medium")),
                    )?,
                    event,
                )?;
                assert!(reason.contains("MISSING_"), "{event}: {reason}");
                assert!(reason.contains("child-to-parent"), "{event}: {reason}");
                assert!(reason.contains("gpt-6-astra"), "{event}: {reason}");
                assert!(reason.contains("medium"), "{event}: {reason}");
                assert!(reason.contains("MUST"), "{event}: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn malformed_metadata_uses_mandatory_non_retry_wording() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    for event in EVENTS {
        for root in &roots {
            let reason = reason(
                run(
                    root,
                    event,
                    &transcript,
                    Some(CHILD),
                    json!({
                        "threadId":PARENT,
                        "model":"gpt-6-astra",
                        "thinking":"medium"
                    }),
                    Some(json!({"authenticated":true})),
                )?,
                event,
            )?;
            assert!(reason.contains("MALFORMED_ROUTING_METADATA"));
            assert!(reason.contains("MUST NOT retry blindly"));
            assert!(reason.contains("MUST obtain"));
            assert!(!reason.contains(CHILD));
            assert!(!reason.contains(PARENT));
            }
        }
    Ok(())
}
