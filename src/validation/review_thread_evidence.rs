use serde_json::Value;

pub(super) fn check(threads: &Value) -> Option<String> {
    let Some(has_next_page) = threads
        .get("pageInfo")
        .and_then(|page| page.get("hasNextPage"))
        .and_then(Value::as_bool)
    else {
        return Some(
            "incomplete reviewThreads.nodes PR state evidence: missing pagination pageInfo.hasNextPage"
                .into(),
        );
    };
    if has_next_page {
        return Some(
            "incomplete reviewThreads.nodes PR state evidence: pagination hasNextPage true".into(),
        );
    }
    check_nodes(threads.get("nodes").and_then(Value::as_array)?)
}
fn check_nodes(nodes: &[Value]) -> Option<String> {
    nodes.iter().enumerate().find_map(|(index, thread)| {
        let missing = [
            ("id", !has_string(thread, "id")),
            ("isResolved", !has_bool(thread, "isResolved")),
            ("isOutdated", !has_bool(thread, "isOutdated")),
            ("path", !has_string(thread, "path")),
            (
                "comments.nodes.url",
                !comment_urls(thread).any(|url| !url.is_empty()),
            ),
        ]
        .into_iter()
        .find(|(_, missing)| *missing)
        .map(|(field, _)| field)?;
        Some(format!(
            "incomplete reviewThreads.nodes PR state evidence at index {index}: missing {missing}"
        ))
    })
}

fn has_string(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_str).is_some()
}

fn has_bool(value: &Value, field: &str) -> bool {
    value.get(field).and_then(Value::as_bool).is_some()
}

fn comment_urls(thread: &Value) -> impl Iterator<Item = &str> {
    thread
        .get("comments")
        .and_then(|comments| comments.get("nodes"))
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|nodes| nodes.iter())
        .filter_map(|comment| comment.get("url").and_then(Value::as_str))
}
