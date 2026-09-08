use serde_json::{Value, json};

use super::{DELTA_EVENT, DELTA_HEAD, FULL_EVENT, FULL_HEAD, REMAINING_FINDING};

pub(super) fn owner_pages() -> Vec<Value> {
    vec![
        page(
            "01a07932-770f-7c31-8ee8-e4521337f869",
            "owner-cursor",
            true,
            vec![json!({
                "id": "owner-helper",
                "type": "collabAgentToolCall",
                "tool": "spawnAgent",
                "status": "completed",
                "senderThreadId": "01a07932-770f-7c31-8ee8-e4521337f869",
                "receiverThreadIds": ["helper-thread"],
                "prompt": "Inspect an unrelated detail.",
                "model": "gpt-5.6-luna",
                "reasoningEffort": "minimal",
                "receiver_agents": [{"thread_id": "helper-thread", "agent_role": "codexy-cartographer"}]
            })],
        ),
        page(
            "01a07932-770f-7c31-8ee8-e4521337f869",
            "owner-cursor",
            false,
            vec![json!({
                "id": "strict-reviewer-dispatch-919",
                "type": "collabAgentToolCall",
                "tool": "spawnAgent",
                "status": "completed",
                "senderThreadId": "01a07932-770f-7c31-8ee8-e4521337f869",
                "receiverThreadIds": ["01a07c00-9b13-7123-957c-97ac40aca4b5"],
                "prompt": "Review the current #919 change using the selected strict profile.",
                "model": "gpt-5.6-sol",
                "reasoningEffort": "xhigh",
                "receiver_agents": [{"thread_id": "01a07c00-9b13-7123-957c-97ac40aca4b5", "agent_role": "codexy-sentinel"}]
            })],
        ),
    ]
}

pub(super) fn reviewer_pages() -> Vec<Value> {
    vec![
        page_with_turn(
            "01a07c00-9b13-7123-957c-97ac40aca4b5",
            "reviewer-cursor",
            true,
            "reviewer-turn-delta",
            200,
            json!({
                "id": DELTA_EVENT,
                "type": "agentMessage",
                "phase": "final_answer",
                "text": "Sanitized delta review: BLOCK.",
                "review": {
                    "kind": "delta",
                    "reviewedHead": DELTA_HEAD,
                    "terminalResult": "BLOCK",
                    "model": "gpt-5.6-sol",
                    "reasoningEffort": "xhigh",
                    "findings": [
                        finding("919-opaque-status-scalar", "plugins/codexy/skills/project-brief/SKILL.md", "The status representation introduces a delimiter grammar.", "contract"),
                        finding(REMAINING_FINDING, "plugins/codexy/skills/project-brief/SKILL.md", "Current-head proof lacks one canonical replay binding.", "proof")
                    ]
                }
            }),
        ),
        page_with_turn(
            "01a07c00-9b13-7123-957c-97ac40aca4b5",
            "reviewer-cursor",
            false,
            "reviewer-turn-full",
            100,
            json!({
                "id": FULL_EVENT,
                "type": "agentMessage",
                "phase": "final_answer",
                "text": "Sanitized full review: BLOCK.",
                "review": {
                    "kind": "full",
                    "reviewedHead": FULL_HEAD,
                    "terminalResult": "BLOCK",
                    "model": "gpt-5.6-sol",
                    "reasoningEffort": "xhigh",
                    "findings": [
                        finding("919-human-machine-facts", "plugins/codexy/skills/project-brief/SKILL.md", "Human and machine status facts diverge.", "contract"),
                        finding("919-provenance-record", "plugins/codexy/skills/project-brief/contract.md", "The provenance record is inconsistent.", "provenance")
                    ]
                }
            }),
        ),
    ]
}

fn finding(id: &str, path: &str, text: &str, semantic_kind: &str) -> Value {
    json!({
        "id": id,
        "path": path,
        "text": text,
        "severity": "high",
        "disposition": "unresolved",
        "semanticKind": semantic_kind,
        "source": "reviewer-observation"
    })
}

fn page(thread: &str, cursor: &str, has_more: bool, items: Vec<Value>) -> Value {
    let turn_id = items
        .first()
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("owner-page");
    json!({
        "thread": {"id": thread},
        "page": {
            "order": "newest_first",
            "limit": 10,
            "cursor": if has_more { Value::Null } else { Value::String(cursor.into()) },
            "nextCursor": if has_more { Value::String(cursor.into()) } else { Value::Null },
            "hasMore": has_more
        },
        "turns": [{"id": format!("owner-turn-{turn_id}"), "status": "completed", "completedAt": 10, "items": items}]
    })
}

fn page_with_turn(
    thread: &str,
    cursor: &str,
    has_more: bool,
    turn_id: &str,
    completed_at: u64,
    item: Value,
) -> Value {
    json!({
        "thread": {"id": thread},
        "page": {
            "order": "newest_first",
            "limit": 10,
            "cursor": if has_more { Value::Null } else { Value::String(cursor.into()) },
            "nextCursor": if has_more { Value::String(cursor.into()) } else { Value::Null },
            "hasMore": has_more
        },
        "turns": [{"id": turn_id, "status": "completed", "completedAt": completed_at, "items": [item]}]
    })
}
