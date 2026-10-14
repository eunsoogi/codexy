use serde_json::Value;

use super::{cancel_unique_binding, text};

// Validate the completed sender envelope before using its recipient as a binding key.
pub(super) fn cancel_request_binding_for_posttool_message(payload: &Value) -> anyhow::Result<bool> {
    if payload.get("hook_event_name").and_then(Value::as_str) != Some("PostToolUse")
        || payload.get("tool_name").and_then(Value::as_str)
            != Some("mcp__codex_app__send_message_to_thread")
    {
        return Ok(false);
    }
    let Some(input) = payload.get("tool_input").and_then(Value::as_object) else {
        return Ok(false);
    };
    let Ok(input_recipient) = text(input.get("threadId"), "threadId", 256) else {
        return Ok(false);
    };
    let Some(response) = payload.get("tool_response").and_then(Value::as_object) else {
        return Ok(false);
    };
    if response.get("isError").and_then(Value::as_bool) != Some(false) {
        return Ok(false);
    }
    let Some(content) = response.get("content").and_then(Value::as_array) else {
        return Ok(false);
    };
    if content.len() != 1 || content[0].get("type").and_then(Value::as_str) != Some("text") {
        return Ok(false);
    }
    let Ok(encoded_result) = text(content[0].get("text"), "tool response text", 16_384) else {
        return Ok(false);
    };
    let Ok(result) = serde_json::from_str::<Value>(&encoded_result) else {
        return Ok(false);
    };
    let Ok(output_recipient) = text(result.get("threadId"), "response threadId", 256) else {
        return Ok(false);
    };
    if input_recipient != output_recipient {
        return Ok(false);
    }

    // session_id belongs to the sender; match across the shared Watcher state root.
    let root = crate::watcher::io::state_root()?;
    cancel_unique_binding(&root, |binding| binding.main_session_id == input_recipient)
}
