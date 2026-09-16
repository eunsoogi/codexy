use serde_json::json;

use super::thread_delivery_support::{
    CHILD as WORKER_ID, PARENT as ORCHESTRATOR_ID, plugin_roots, reason, route, run,
};
use crate::support::TestResult;

const EVENTS: &[&str] = &["PermissionRequest", "PreToolUse"];

#[test]
fn orchestrator_missing_fields_use_worker_remedy() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    let cases = [
        (
            "missing model",
            json!({"threadId":WORKER_ID,"thinking":"max"}),
            "MISSING_MODEL",
            "model",
        ),
        (
            "missing thinking",
            json!({"threadId":WORKER_ID,"model":"gpt-5.6-luna"}),
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
                        Some(ORCHESTRATOR_ID),
                        input.clone(),
                        Some(route(
                            "root_to_child",
                            ORCHESTRATOR_ID,
                            WORKER_ID,
                            "gpt-5.6-luna",
                            "max",
                        )),
                    )?,
                    event,
                )?;
                assert!(reason.contains(code), "{event} {label}: {reason}");
                assert!(reason.contains(field), "{event} {label}: {reason}");
                assert!(
                    reason.contains("Orchestrator-to-Worker"),
                    "{event} {label}: {reason}"
                );
                assert!(reason.contains("gpt-5.6-luna"), "{event} {label}: {reason}");
                assert!(reason.contains("max"), "{event} {label}: {reason}");
                assert!(reason.contains("MUST"), "{event} {label}: {reason}");
                assert!(
                    !reason.contains("Worker-to-Orchestrator"),
                    "{event} {label}: {reason}"
                );
                assert!(!reason.contains("gpt-6-astra"), "{event} {label}: {reason}");
            }
        }
    }
    Ok(())
}

#[test]
fn worker_missing_fields_use_orchestrator_remedy() -> TestResult {
    let temp = tempfile::tempdir()?;
    let transcript = temp.path().join("ignored.jsonl");
    std::fs::write(&transcript, b"not-json\n")?;
    let roots = plugin_roots(temp.path())?;
    for event in EVENTS {
        for root in &roots {
            for input in [
                json!({"threadId":ORCHESTRATOR_ID,"thinking":"medium"}),
                json!({"threadId":ORCHESTRATOR_ID,"model":"gpt-6-astra"}),
            ] {
                let reason = reason(
                    run(
                        root,
                        event,
                        &transcript,
                        Some(WORKER_ID),
                        input,
                        Some(route(
                            "child_to_parent",
                            WORKER_ID,
                            ORCHESTRATOR_ID,
                            "gpt-6-astra",
                            "medium",
                        )),
                    )?,
                    event,
                )?;
                assert!(reason.contains("MISSING_"), "{event}: {reason}");
                assert!(reason.contains("Worker-to-Orchestrator"), "{event}: {reason}");
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
                    Some(WORKER_ID),
                    json!({
                        "threadId":ORCHESTRATOR_ID,
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
            assert!(!reason.contains(WORKER_ID));
            assert!(!reason.contains(ORCHESTRATOR_ID));
        }
    }
    Ok(())
}
