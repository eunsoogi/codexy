// Native follow-ups cannot be semantically inspected reliably before delivery.
use super::{run_payload, TestResult, EVENTS};
use serde_json::json;

#[test]
fn native_followups_are_denied_even_for_read_only_or_opaque_messages() -> TestResult {
    for event in EVENTS {
        for prefix in ["", "agents__", "multi_agent_v1__", "agents", "collaboration", "multi_agent_v1"] {
            for name in ["send_message", "followup_task", "send_input"] {
                let tool = format!("{prefix}{name}");
                for message in ["Review the diff and report findings.", "Own the issue and implement it.", "gAAAAopaque"] {
                    let input = json!({"hook_event_name":event,"tool_name":tool,
                        "tool_input":{"target":"helper","message":message}});
                    let reason = run_payload(&input, event)?.ok_or("follow-up must be denied")?;
                    assert!(reason.contains("CODEXY_SUBAGENT_OWNERSHIP_FOLLOWUP_DISABLED"), "{tool}: {reason}");
                    assert!(reason.contains("child chat"), "{reason}");
                }
                // V1 can use structured items and needs no message field.
                let input = json!({"hook_event_name":event,"tool_name":tool,
                    "tool_input":{"target":"helper","items":[{"type":"text","text":"Review"}]}});
                let reason = run_payload(&input, event)?.ok_or("follow-up must be denied")?;
                assert!(reason.contains("CODEXY_SUBAGENT_OWNERSHIP_FOLLOWUP_DISABLED"), "{tool}: {reason}");
            }
        }
    }
    Ok(())
}
