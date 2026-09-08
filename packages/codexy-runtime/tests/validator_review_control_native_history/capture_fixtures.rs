use serde_json::{Value, json};

pub(crate) fn read_thread_request() -> Value {
    let mut request = super::request();
    for (role, cursor) in [("owner", "owner-cursor"), ("reviewer", "reviewer-cursor")] {
        let pages = request[role]["pages"]
            .as_array_mut()
            .expect("fixture pages");
        for page in pages.iter_mut() {
            page["page"]
                .as_object_mut()
                .expect("fixture page metadata")
                .remove("cursor");
        }
        request[role]["capture"] = json!({
            "method": "read_thread",
            "requests": [
                {"cursor": null},
                {"cursor": cursor}
            ]
        });
    }
    request
}

pub(crate) fn host_provenance_request() -> Value {
    let mut request = read_thread_request();
    request["owner"]["pages"][1]["turns"][0]["items"][0]
        .as_object_mut()
        .expect("reviewer spawn")
        .remove("receiver_agents");
    request["reviewer"]["capture"]["provenance"] = json!({
        "schema": "codexy.review-control-native-history-capture-provenance.v1",
        "session": {
            "id": "reviewer-thread",
            "session_id": "reviewer-thread",
            "parent_thread_id": "owner-thread",
            "agent_nickname": "Noether",
            "agent_role": "codexy-sentinel",
            "source": {
                "subagent": {
                    "thread_spawn": {
                        "parent_thread_id": "owner-thread",
                        "agent_nickname": "Noether",
                        "agent_role": "codexy-sentinel"
                    }
                }
            }
        },
        "spawn": {
            "id": "spawn-reviewer",
            "senderThreadId": "owner-thread",
            "receiverThreadIds": ["reviewer-thread"],
            "status": "completed"
        }
    });
    request
}
