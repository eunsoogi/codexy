use serde_json::{Value, json};

use super::delegation;

pub(super) fn child_transcript(
    session: &str,
    parents: &[&str],
    content: Vec<Value>,
) -> Vec<u8> {
    let mut lines = vec![
        json!({"type":"session_meta","payload":{"id":session,"session_id":session,"thread_source":"agent_created_thread"}}).to_string(),
        json!({"type":"turn_context","payload":{}}).to_string(),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":content}}).to_string(),
    ];
    for (index, parent) in parents.iter().enumerate() {
        let output = delegation(parent, "Implement the owned lane.");
        lines.push(json!({
            "type":"response_item",
            "payload":{
                "type":"function_call_output",
                "id":format!("fco-fixture-{index}"),
                "call_id":format!("call-fixture-{index}"),
                "name":"create_thread",
                "namespace":"codex_app",
                "output":output
            }
        }).to_string());
        lines.push(json!({
            "type":"event_msg",
            "payload":{
                "type":"item_completed",
                "thread_id":session,
                "item":{
                    "type":"FunctionCallOutput",
                    "id":format!("fco-fixture-{index}"),
                    "name":"create_thread",
                    "namespace":"codex_app",
                    "status":"completed",
                    "output":delegation(parent, "Implement the owned lane.")
                }
            }
        }).to_string());
    }
    format!("{}\n", lines.join("\n")).into_bytes()
}
