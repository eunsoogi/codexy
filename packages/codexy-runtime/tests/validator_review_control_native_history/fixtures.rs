use serde_json::{Value, json};

pub(crate) const FULL_HEAD: &str = "1111111111111111111111111111111111111111";
pub(crate) const DELTA_HEAD: &str = "2222222222222222222222222222222222222222";
pub(crate) const CURRENT_HEAD: &str = "3333333333333333333333333333333333333333";
pub(crate) const BASE_HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub(crate) fn request() -> Value {
    json!({
        "schema": "codexy.review-control-native-history-request.v1",
        "target": {
            "repository": "octo/example",
            "owningIssue": 17,
            "pullRequest": 23,
            "phase": "post_pr"
        },
        "owner": {"pages": owner_pages()},
        "reviewer": {"pages": reviewer_pages()}
    })
}

pub(crate) fn current_snapshot(created: u64, head: &str) -> Value {
    json!({
        "repository": "octo/example",
        "number": 23,
        "owningIssue": 17,
        "baseRefName": "main",
        "baseRefOid": BASE_HEAD,
        "headRefOid": head,
        "url": "https://github.com/octo/example/pull/23",
        "createdAtEpoch": created,
        "capture": {
            "provider": "github",
            "method": "graphql",
            "authenticated": true
        }
    })
}

fn owner_pages() -> Vec<Value> {
    let selected = json!({
        "type": "collabAgentToolCall",
        "id": "spawn-reviewer",
        "tool": "spawnAgent",
        "status": "completed",
        "senderThreadId": "owner-thread",
        "receiverThreadIds": ["reviewer-thread"],
        "prompt": "Review the current change using the selected profile.",
        "model": "model.initial",
        "reasoningEffort": "low"
    });
    let helper = json!({
        "type": "collabAgentToolCall",
        "id": "spawn-helper",
        "tool": "spawnAgent",
        "status": "completed",
        "senderThreadId": "owner-thread",
        "receiverThreadIds": ["helper-thread"],
        "prompt": "Inspect one unrelated detail.",
        "model": "model.helper",
        "reasoningEffort": "minimal"
    });
    vec![
        page(
            "owner-thread",
            "owner-cursor",
            true,
            vec![turn("owner-turn-new", vec![helper])],
        ),
        page(
            "owner-thread",
            "",
            false,
            vec![turn("owner-turn-old", vec![selected])],
        ),
    ]
}

fn reviewer_pages() -> Vec<Value> {
    let delta = json!({
        "type": "AgentMessage",
        "id": "message-delta",
        "phase": "final_answer",
        "text": "검토 결과: delta",
        "review": {
            "kind": "delta",
            "reviewedHead": DELTA_HEAD,
            "terminalResult": "BLOCK",
            "findings": [
                {
                    "id": "delta-code",
                    "path": "src/lib.rs",
                    "text": "코드 수정이 필요합니다.",
                    "severity": "high",
                    "disposition": "unresolved"
                },
                {
                    "text": "신선한 호스트 증거가 없습니다.",
                    "severity": "medium",
                    "semanticKind": "proof",
                    "source": "reviewer-observation"
                }
            ]
        }
    });
    let full = json!({
        "type": "agentMessage",
        "id": "message-full",
        "phase": "final_answer",
        "text": "Full review with an explicit continuation model.",
        "review": {
            "kind": "full",
            "reviewedHead": FULL_HEAD,
            "terminalResult": "PASS",
            "model": "model.continuation",
            "reasoningEffort": "medium",
            "findings": [
                {
                    "id": "full-docs",
                    "path": "README.md",
                    "text": "문서 확인이 필요합니다.",
                    "severity": "low",
                    "disposition": "observed"
                }
            ]
        }
    });
    vec![
        page(
            "reviewer-thread",
            "reviewer-cursor",
            true,
            vec![turn_at("reviewer-turn-new", 200, vec![delta])],
        ),
        page(
            "reviewer-thread",
            "",
            false,
            vec![turn_at("reviewer-turn-old", 100, vec![full])],
        ),
    ]
}

fn page(thread: &str, cursor: &str, has_more: bool, turns: Vec<Value>) -> Value {
    json!({
        "thread": {"id": thread},
        "page": {
            "order": "newest_first",
            "limit": 10,
            "cursor": if cursor.is_empty() { Value::Null } else { Value::String(cursor.into()) },
            "nextCursor": if has_more { Value::String(cursor.into()) } else { Value::Null },
            "hasMore": has_more
        },
        "turns": turns
    })
}

fn turn(id: &str, items: Vec<Value>) -> Value {
    turn_at(id, 10, items)
}

fn turn_at(id: &str, completed_at: u64, items: Vec<Value>) -> Value {
    json!({
        "id": id,
        "status": "completed",
        "error": null,
        "completedAt": completed_at,
        "items": items
    })
}
