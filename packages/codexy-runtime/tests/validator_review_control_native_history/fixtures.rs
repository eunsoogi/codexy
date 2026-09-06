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

pub(crate) fn recovery_snapshot() -> Value {
    let mut current = current_snapshot(50, CURRENT_HEAD);
    current["reviewProfile"] = json!("strict");
    current["capture"]["owningIssue"] = json!({
        "repository": "octo/example",
        "number": 17,
        "url": "https://github.com/octo/example/issues/17",
        "association": "owner-assignment"
    });
    current
}

pub(crate) fn snapshot_with_heads(created: u64, base: &str, head: &str) -> Value {
    let mut current = recovery_snapshot();
    current["createdAtEpoch"] = json!(created);
    current["headRefOid"] = json!(head);
    current["baseRefOid"] = json!(base);
    current
}

pub(crate) fn next_control(
    recovered: &Value,
    delta_head: &str,
    current_head: &str,
) -> crate::support::TestResult<Value> {
    let mut control = recovered["reviewControl"].clone();
    let history = control["terminal_review_history"]
        .as_array()
        .cloned()
        .ok_or("recovered history")?;
    let mut history = history;
    history.push(json!({
        "id": "required-current",
        "kind": "required_current_head",
        "reviewer": control["reviewer"].clone(),
        "reviewed_head": current_head,
        "terminal_result": "PASS",
        "unresolved_findings": []
    }));
    control["reviewed_head"] = json!(current_head);
    control["terminal_result"] = json!("PASS");
    control["unresolved_findings"] = json!([]);
    control["terminal_review_count"] = json!(3);
    control["terminal_review_history"] = Value::Array(history);
    control["post_cap_re_review"] = json!({
        "reason": "mandatory_base_integration",
        "prior_reviewed_head": delta_head,
        "qualifying_change": {
            "from_head": delta_head,
            "to_head": current_head,
            "evidence_commit": current_head,
            "finding_ids": []
        }
    });
    control
        .as_object_mut()
        .ok_or("review control")?
        .remove("native_history_recovery");
    Ok(control)
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
        "reasoningEffort": "low",
        "receiver_agents": [{
            "thread_id": "reviewer-thread",
            "agent_role": "codexy-sentinel"
        }]
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
        "reasoningEffort": "minimal",
        "receiver_agents": [{
            "thread_id": "helper-thread",
            "agent_role": "codexy-cartographer"
        }]
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
            "owner-cursor",
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
            "reviewer-cursor",
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
            "cursor": if has_more { Value::Null } else if cursor.is_empty() { Value::Null } else { Value::String(cursor.into()) },
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
