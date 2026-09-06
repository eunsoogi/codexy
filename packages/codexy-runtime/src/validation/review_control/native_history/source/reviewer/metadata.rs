use serde_json::{Map, Value};

use super::super::{Invocation, fields};

#[path = "markdown.rs"]
mod markdown;
#[path = "request.rs"]
mod request;

pub(super) fn candidates(
    turn: &Map<String, Value>,
    message: &Map<String, Value>,
    raw: &str,
    base: &str,
) -> Result<Vec<Map<String, Value>>, String> {
    let mut result = Vec::new();
    for map in [turn, message] {
        for key in [
            "review",
            "reviewResult",
            "review_result",
            "structuredReview",
        ] {
            if let Some(value) = map.get(key) {
                result.push(fields::object(value, "review metadata")?.clone());
            }
        }
        if [
            "kind",
            "reviewKind",
            "reviewedHead",
            "reviewed_head",
            "terminalResult",
            "terminal_result",
            "findings",
        ]
        .iter()
        .any(|key| map.contains_key(*key))
        {
            result.push(map.clone());
        }
    }
    if let Some(candidate) = request::candidate(turn, base)? {
        result.push(candidate);
    }
    if let Some(candidate) = markdown::candidate(raw)? {
        result.push(candidate);
    }
    let trimmed = raw.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        let parsed = serde_json::from_str::<Value>(trimmed)
            .map_err(|error| format!("final message JSON metadata is invalid: {error}"))?;
        if let Some(map) = parsed.as_object() {
            result.push(map.clone());
        }
    } else if let Some(map) = labeled(raw)? {
        result.push(map);
    }
    Ok(result)
}

pub(super) fn field_sources(candidates: &[Map<String, Value>], base: &str) -> Value {
    let mut spans = Map::new();
    for candidate in candidates {
        let Some(source_spans) = candidate.get("sourceSpans").and_then(Value::as_object) else {
            continue;
        };
        for (field, span) in source_spans {
            spans.entry(field.clone()).or_insert_with(|| span.clone());
        }
    }
    if spans.is_empty() {
        Value::Object(Map::new())
    } else {
        let mut result = Map::new();
        result.insert("path".into(), Value::String(base.to_owned()));
        result.insert("spans".into(), Value::Object(spans));
        Value::Object(result)
    }
}

pub(super) fn reviewer_facts(
    candidates: &[Map<String, Value>],
    invocation: &Invocation,
) -> Result<(String, String, String), String> {
    let mut sources = candidates.to_vec();
    for candidate in candidates {
        for key in ["reviewer", "reviewerInvocation"] {
            if let Some(value) = candidate.get(key) {
                sources.push(fields::object(value, "reviewer facts")?.clone());
            }
        }
    }
    let model = fields::merged_text(&sources, &["model"], "review model")?;
    let effort = fields::merged_text(
        &sources,
        &["reasoningEffort", "reasoning_effort", "thinking"],
        "review effort",
    )?;
    match (model, effort) {
        (Some(model), Some(effort)) => Ok((model, effort, "reviewer_event".into())),
        (None, None) => Ok((
            invocation.model.clone(),
            invocation.reasoning_effort.clone(),
            "owner_spawn".into(),
        )),
        _ => Err("reviewer event must provide both model and reasoning effort".into()),
    }
}

fn labeled(raw: &str) -> Result<Option<Map<String, Value>>, String> {
    let mut map = Map::new();
    for (_, _, line) in markdown::operative_lines(raw) {
        let Some((label, value)) = line.split_once(':') else {
            continue;
        };
        let label = label
            .trim()
            .trim_start_matches('-')
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "_");
        let value = value.trim();
        match label.as_str() {
            "kind" | "review_kind" | "review_type" => {
                merge(&mut map, "kind", Value::String(clean(value)), "review kind")?;
            }
            "reviewed_head" | "exact_reviewed_pr_head" => {
                let sha = value
                    .split_whitespace()
                    .map(clean)
                    .find(|part| fields::is_sha(part))
                    .ok_or("reviewed head label lacks a SHA")?;
                merge(
                    &mut map,
                    "reviewed_head",
                    Value::String(sha),
                    "reviewed head",
                )?;
            }
            "terminal_result" | "terminal_verdict" => {
                let result = value
                    .split_whitespace()
                    .next()
                    .map(clean)
                    .filter(|result| !result.is_empty())
                    .ok_or("terminal result label is empty")?;
                merge(
                    &mut map,
                    "terminal_result",
                    Value::String(result),
                    "terminal result",
                )?;
            }
            "findings" | "findings_json" => {
                let findings = serde_json::from_str(value)
                    .map_err(|error| format!("findings JSON is invalid: {error}"))?;
                merge(&mut map, "findings", findings, "review findings")?;
            }
            _ => {}
        }
    }
    Ok((!map.is_empty()).then_some(map))
}

fn merge(map: &mut Map<String, Value>, key: &str, value: Value, label: &str) -> Result<(), String> {
    if map.get(key).is_some_and(|prior| prior != &value) {
        return Err(format!("{label} is contradictory"));
    }
    map.insert(key.into(), value);
    Ok(())
}

fn clean(value: &str) -> String {
    value
        .trim_matches(|character: char| {
            character == '\u{60}' || character == '*' || ",.;()[]".contains(character)
        })
        .to_owned()
}
