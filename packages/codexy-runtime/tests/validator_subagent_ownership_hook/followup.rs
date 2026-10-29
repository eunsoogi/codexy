// Exercise both callable follow-up schemas through the packaged launcher.
use super::{run_payload, TestResult, EVENTS};
use serde_json::json;

#[test]
fn followups_apply_classifier_without_target_role_claims() -> TestResult {
    for event in EVENTS {
        for prefix in ["", "agents__", "multi_agent_v1__"] {
            for name in ["send_message", "followup_task"] {
                let tool = format!("{prefix}{name}");
                for (message, denied) in [
                    ("Own branch `issue` and implement the issue.", true),
                    ("Implement the task in the reserved worktree.", true),
                    ("Review the diff and report findings.", false),
                    ("Do not own the branch.", false),
                    ("The quoted text is: `Own branch issue`", false),
                    ("Please report your progress.", false),
                ] {
                    let input = json!({"hook_event_name":event,"tool_name":tool,
                        "tool_input":{"target":"helper","message":message,"agent_type":"explorer"}});
                    let reason = run_payload(&input, event)?;
                    assert_eq!(reason.is_some(), denied, "{tool}: {message}: {reason:?}");
                    if let Some(reason) = reason {
                        assert!(reason.contains("CODEXY_SUBAGENT_OWNERSHIP_DURABLE_OWNER"));
                    }
                }
                for input in [json!({}), json!({"target":"", "message":"hello"}),
                    json!({"target":"helper", "message":null})] {
                    let payload = json!({"hook_event_name":event,"tool_name":tool,"tool_input":input});
                    let reason = run_payload(&payload, event)?.ok_or("missing denial")?;
                    assert!(reason.contains("CODEXY_SUBAGENT_OWNERSHIP_ENVELOPE"));
                }
            }
        }
    }
    Ok(())
}
