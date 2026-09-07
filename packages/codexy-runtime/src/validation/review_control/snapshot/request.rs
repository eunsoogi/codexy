use serde_json::Value;

use super::normalize;

pub(crate) fn normalize_request_states(
    request: &Value,
) -> Result<(Option<Value>, Option<Value>), String> {
    let current = request
        .get("current_pr_state")
        .map(|state| normalize_connector(state, "current"))
        .transpose()?;
    let previous = request
        .get("previous_pr_state")
        .map(|state| normalize_connector(state, "previous"))
        .transpose()?;
    Ok((current, previous))
}

fn normalize_connector(value: &Value, label: &str) -> Result<Value, String> {
    let is_connector = value
        .get("capture")
        .and_then(Value::as_object)
        .and_then(|capture| capture.get("method"))
        .and_then(Value::as_str)
        == Some("connector");
    if is_connector {
        normalize(value, label)
    } else {
        Ok(value.clone())
    }
}
