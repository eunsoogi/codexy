use serde_json::{Value, json};

pub(crate) fn source_only(snapshot: &Value) -> Value {
    let mut value = snapshot.clone();
    value["capture"]["method"] = json!("connector");
    value["capture"]["source"] = json!({
        "tool": "mcp__codex_apps__github_get_pr_info",
        "arguments": {
            "repository_full_name": snapshot["repository"],
            "pr_number": snapshot["number"]
        },
        "result": {
            "number": snapshot["number"], "url": snapshot["url"],
            "base": snapshot["baseRefName"], "base_sha": snapshot["baseRefOid"],
            "head_sha": snapshot["headRefOid"], "title": "Synthetic capture"
        }
    });
    for key in ["repository", "number", "url", "baseRefName", "baseRefOid", "headRefOid"] {
        value.as_object_mut().expect("snapshot").remove(key);
    }
    value
}
