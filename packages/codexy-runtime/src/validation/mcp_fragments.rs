use serde_json::Value;

use super::mcp::DISALLOWED_FRAGMENTS;

pub(super) fn disallowed_value_fragments(value: &Value) -> Vec<&'static str> {
    let mut matches = Vec::new();
    collect_fragments(value, &mut matches);
    matches.sort_unstable();
    matches.dedup();
    matches
}

fn collect_fragments(value: &Value, matches: &mut Vec<&'static str>) {
    match value {
        Value::String(text) => {
            let lowered = text.to_ascii_lowercase();
            matches.extend(
                DISALLOWED_FRAGMENTS
                    .iter()
                    .copied()
                    .filter(|fragment| lowered.contains(fragment)),
            );
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_fragments(item, matches)),
        Value::Object(items) => items
            .values()
            .for_each(|item| collect_fragments(item, matches)),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}
