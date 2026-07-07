use serde_json::Value;

pub(super) fn check(pr_state: &Value) -> Option<String> {
    let thread = pr_state
        .get("reviewThreads")
        .and_then(|threads| threads.get("nodes"))
        .and_then(Value::as_array)?
        .iter()
        .find(|thread| thread.get("isResolved").and_then(Value::as_bool) == Some(false))?;
    Some(format!(
        "unresolved review thread remains before PR-ready or merge-ready claims: {}; resolve fixed or accepted threads after current-head verification",
        thread_label(thread)
    ))
}

fn thread_label(thread: &Value) -> String {
    let id = thread
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("unknown thread");
    let path = thread
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("unknown path");
    let url = thread
        .get("comments")
        .and_then(|comments| comments.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|comments| comments.iter())
        .find_map(|comment| comment.get("url").and_then(Value::as_str))
        .unwrap_or("no comment URL");
    format!("{id} at {path} ({url})")
}
