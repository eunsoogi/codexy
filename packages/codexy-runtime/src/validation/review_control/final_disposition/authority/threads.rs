use serde_json::{Map, Value, json};

use super::super::super::pre_pr::{object, text};
use crate::validation::review_thread_evidence;

pub(super) fn project(value: Option<&Value>) -> Result<Value, String> {
    let threads = object(value, "final disposition authority reviewThreads")?;
    check_page(threads, "final disposition authority reviewThreads")?;
    let nodes = threads
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or("final disposition authority reviewThreads must list nodes")?;
    let projected = nodes
        .iter()
        .map(|node| {
            let node = object(Some(node), "final disposition authority review thread")?;
            let comments = object(
                node.get("comments"),
                "final disposition authority review thread comments",
            )?;
            check_page(
                comments,
                "final disposition authority review thread comments",
            )?;
            let comment_nodes = comments
                .get("nodes")
                .and_then(Value::as_array)
                .ok_or("final disposition authority review comments must list nodes")?;
            let urls = comment_nodes
                .iter()
                .map(|comment| {
                    let comment = object(Some(comment), "final disposition review comment")?;
                    Ok(json!({"url": text(comment, "url", "final disposition review comment")?}))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(json!({
                "id": text(node, "id", "final disposition authority review thread")?,
                "isResolved": node.get("isResolved").and_then(Value::as_bool).ok_or("final disposition review thread must contain isResolved")?,
                "isOutdated": node.get("isOutdated").and_then(Value::as_bool).ok_or("final disposition review thread must contain isOutdated")?,
                "path": text(node, "path", "final disposition authority review thread")?,
                "comments": {"nodes": urls}
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let result = json!({"pageInfo": {"hasNextPage": false}, "nodes": projected});
    if let Some(error) = review_thread_evidence::check(&result) {
        return Err(error);
    }
    Ok(result)
}

pub(super) fn check_page(object: &Map<String, Value>, label: &str) -> Result<(), String> {
    if object
        .get("pageInfo")
        .and_then(Value::as_object)
        .and_then(|page| page.get("hasNextPage"))
        != Some(&Value::Bool(false))
    {
        return Err(format!("{label} lookup is incomplete"));
    }
    Ok(())
}

pub(super) fn sorted_ids(ids: &std::collections::HashSet<String>) -> Vec<String> {
    let mut ids = ids.iter().cloned().collect::<Vec<_>>();
    ids.sort();
    ids
}
