use serde_json::{Map, Value, json};

use super::super::fields;

#[path = "request/grammar.rs"]
mod grammar;

pub(super) fn candidate(
    turn: &Map<String, Value>,
    base: &str,
) -> Result<Option<Map<String, Value>>, String> {
    let Some(items) = turn.get("items").and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut result = Map::new();
    let mut spans = Map::new();
    for (index, item) in items.iter().enumerate() {
        let Some(map) = item.as_object() else {
            continue;
        };
        if fields::text(map, &["type"], "request item type")?.as_deref() != Some("userMessage") {
            continue;
        }
        let Some(text) = message_text(map) else {
            continue;
        };
        let raw_first = text.lines().next().unwrap_or_default();
        let leading = raw_first.len() - raw_first.trim_start().len();
        let first = raw_first.trim();
        if let Some(kind) = explicit_kind(first) {
            merge(
                &mut result,
                "kind",
                Value::String(kind),
                "review request kind",
            )?;
            spans.insert(
                "kind".into(),
                json!({
                    "source": format!("{base}.items[{index}].content"),
                    "start": leading,
                    "end": leading + first.len(),
                    "coordinate": "utf8_bytes"
                }),
            );
        }
        let lower = text.to_ascii_lowercase();
        if let Some(review_start) = lower.find("review current exact pr") {
            let review_text = &text[review_start..];
            let review_lower = &lower[review_start..];
            if let Some(head_offset) = review_lower.find("head")
                && let Some((head, sha_offset)) =
                    find_sha(&review_text[head_offset + "head".len()..])
            {
                let start = review_start + head_offset + "head".len() + sha_offset;
                merge(
                    &mut result,
                    "reviewed_head",
                    Value::String(head.clone()),
                    "review request reviewed head",
                )?;
                spans.insert(
                    "reviewed_head".into(),
                    json!({
                        "source": format!("{base}.items[{index}].content"),
                        "start": start,
                        "end": start + 40,
                        "coordinate": "utf8_bytes"
                    }),
                );
            }
        }
    }
    if result.is_empty() {
        return Ok(None);
    }
    if !spans.is_empty() {
        result.insert("sourceSpans".into(), Value::Object(spans));
    }
    Ok(Some(result))
}

fn explicit_kind(first: &str) -> Option<String> {
    let normalized = first.to_ascii_lowercase();
    if normalized.contains("same-reviewer")
        && normalized.contains("delta")
        && !negated(&normalized, "delta")
    {
        return Some("delta".into());
    }
    if normalized.starts_with("authorized strict delta recheck")
        && normalized.contains("frozen final head")
        && normalized.contains("only")
        && !negated(&normalized, "delta")
    {
        return Some("delta".into());
    }
    if affirmative_strict_review(&normalized) && !normalized.contains("delta") {
        return Some("full".into());
    }
    let (label, value) = normalized.split_once(':')?;
    if ["kind", "review kind", "review type"].contains(&label.trim())
        && ["full", "delta", "required_current_head"].contains(&value.trim())
    {
        return Some(value.trim().into());
    }
    None
}

fn affirmative_strict_review(text: &str) -> bool {
    if grammar::starts_with_example_wrapper(text) {
        return false;
    }
    let words = text
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let strict_profile = words.windows(2).any(|pair| pair == ["strict", "profile"]);
    let Some(review_index) = words.iter().position(|word| *word == "review") else {
        return false;
    };
    let affirmative_start = words.first().is_some_and(|word| {
        matches!(
            *word,
            "strict"
                | "perform"
                | "run"
                | "conduct"
                | "complete"
                | "start"
                | "begin"
                | "execute"
                | "authorize"
                | "authorized"
                | "please"
                | "review"
                | "select"
                | "selected"
                | "use"
                | "we"
                | "i"
        )
    });
    let negated = grammar::contains_prefix_negation(text)
        || grammar::contains_post_review_negation(&words[review_index + 1..]);
    strict_profile && affirmative_start && !negated
}

fn negated(text: &str, subject: &str) -> bool {
    [
        format!("do not {subject}"),
        format!("don't {subject}"),
        format!("not a {subject}"),
        format!("without {subject}"),
    ]
    .iter()
    .any(|phrase| text.contains(phrase))
}

fn message_text(map: &Map<String, Value>) -> Option<String> {
    map.get("content")
        .and_then(Value::as_array)
        .map(|content| {
            content
                .iter()
                .filter_map(Value::as_object)
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>()
        })
        .filter(|text| !text.is_empty())
}

fn find_sha(value: &str) -> Option<(String, usize)> {
    let mut start = None;
    for (index, character) in value
        .char_indices()
        .chain(std::iter::once((value.len(), '\0')))
    {
        if character.is_ascii_hexdigit() {
            start.get_or_insert(index);
        } else if let Some(start) = start.take() {
            let part = &value[start..index];
            if fields::is_sha(part) {
                return Some((part.to_owned(), start));
            }
        }
    }
    None
}

fn merge(map: &mut Map<String, Value>, key: &str, value: Value, label: &str) -> Result<(), String> {
    if map.get(key).is_some_and(|prior| prior != &value) {
        return Err(format!("{label} is contradictory"));
    }
    map.insert(key.into(), value);
    Ok(())
}
