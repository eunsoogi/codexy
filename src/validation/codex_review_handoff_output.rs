use serde_json::Value;

pub(super) fn is_review_output_text(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    let output = "didn't find any major issues|no major issues|no actionable issues|no suggestions|no issues|suggestion|review complete|review completed|completed review|finished review|looks good|actionable issue|approved|+1";
    !text.trim().eq("@codex review")
        && !text.contains("create an environment for this repo")
        && !is_review_progress_text(&text)
        && output.split('|').any(|phrase| text.contains(phrase))
}

pub(super) fn iter_json_objects(value: &Value) -> Box<dyn Iterator<Item = &Value> + '_> {
    match value {
        Value::Object(map) => Box::new(
            std::iter::once(value).chain(map.values().flat_map(|value| iter_json_objects(value))),
        ),
        Value::Array(items) => Box::new(items.iter().flat_map(|value| iter_json_objects(value))),
        _ => Box::new(std::iter::empty()),
    }
}

pub(super) fn oid_matches(head: &str, oid: &str) -> bool {
    head.starts_with(oid) || oid.starts_with(head)
}

fn is_review_progress_text(text: &str) -> bool {
    let future = "will post|will provide|will add|i'll post|i will post|when complete|once complete|after review completes|review is still running|review is in progress|review started";
    let result = "suggestion|finding|issue|comment";
    future.split('|').any(|phrase| text.contains(phrase))
        && result.split('|').any(|phrase| text.contains(phrase))
}
