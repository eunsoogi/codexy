use serde_json::{Value, json};

use super::fixtures;

#[test]
fn native_spawn_agent_shape_preserves_source_facts() -> Result<(), String> {
    let request = request();
    let receipt = super::native_history::normalize_native_history(&request)?;
    assert_eq!(
        receipt["source"]["owner"]["pages"],
        request["owner"]["pages"]
    );
    assert_eq!(receipt["owner"]["invocation"]["raw"]["tool"], "spawn_agent");
    assert_eq!(
        receipt["owner"]["invocation"]["raw"]["sender_thread_id"],
        "owner-thread"
    );
    assert_eq!(
        receipt["owner"]["invocation"]["raw"]["receiver_thread_ids"],
        json!(["reviewer-thread"])
    );
    assert_eq!(receipt["owner"]["invocation"]["model"], "gpt-5.6-sol");
    assert_eq!(receipt["owner"]["invocation"]["reasoning_effort"], "xhigh");
    assert_eq!(receipt["events"][0]["message_id"], "message-full");
    assert_eq!(receipt["events"][1]["message_id"], "message-delta");
    for event in receipt["events"].as_array().ok_or("events")? {
        assert_eq!(event["reviewer"]["model"], "gpt-5.6-sol");
        assert_eq!(event["reviewer"]["reasoning_effort"], "xhigh");
        assert_eq!(event["reviewer"]["source"], "owner_spawn");
    }
    assert!(
        receipt["events"][0]["raw_text"]
            .as_str()
            .ok_or("full raw text")?
            .contains("gpt-6-astra/xhigh")
    );
    assert_eq!(receipt["history_projection"]["terminal_review_count"], 2);
    assert_eq!(
        receipt["owner"]["helpers"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(receipt["owner"]["helpers"][0]["raw"]["tool"], "spawn_agent");
    Ok(())
}

pub(crate) fn request() -> Value {
    let mut request = fixtures::request();
    native_spawn(
        &mut request["owner"]["pages"][0]["turns"][0]["items"][0],
        "helper-thread",
        "gpt-5.6-luna",
        "minimal",
        "helper",
        "codexy-cartographer",
    );
    native_spawn(
        &mut request["owner"]["pages"][1]["turns"][0]["items"][0],
        "reviewer-thread",
        "gpt-5.6-sol",
        "xhigh",
        "Confucius",
        "codexy-sentinel",
    );
    let review = request["reviewer"]["pages"][1]["turns"][0]["items"][0]["review"]
        .as_object_mut()
        .expect("full review metadata");
    review.remove("model");
    review.remove("reasoningEffort");
    let text = request["reviewer"]["pages"][1]["turns"][0]["items"][0]["text"]
        .as_str()
        .expect("full review text")
        .to_owned();
    request["reviewer"]["pages"][1]["turns"][0]["items"][0]["text"] =
        json!(format!("{text}\nThe prose mentions gpt-6-astra/xhigh."));
    request
}

fn native_spawn(
    value: &mut Value,
    receiver: &str,
    model: &str,
    effort: &str,
    nickname: &str,
    role: &str,
) {
    let map = value.as_object_mut().expect("spawn object");
    let sender = map.remove("senderThreadId").expect("sender");
    let receivers = map.remove("receiverThreadIds").expect("receivers");
    map.insert("tool".into(), json!("spawn_agent"));
    map.insert("sender_thread_id".into(), sender);
    map.insert("receiver_thread_ids".into(), receivers);
    map.insert(
        "receiver_agents".into(),
        json!([{
            "thread_id": receiver,
            "agent_nickname": nickname,
            "agent_role": role
        }]),
    );
    map.insert("model".into(), json!(model));
    map.remove("reasoningEffort");
    map.insert("reasoning_effort".into(), json!(effort));
}
