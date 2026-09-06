use std::collections::HashSet;

use serde_json::Value;

use super::super::{Event, Finding, fields};

pub(super) fn parse(value: &Value, message_id: &str, span: &str) -> Result<Finding, String> {
    let map = fields::object(value, "finding")?;
    let id = fields::text(map, &["id", "findingId", "finding_id"], "finding id")?;
    let path = fields::text(map, &["path", "file"], "finding path")?;
    let text = fields::required(
        fields::text(map, &["text", "message", "description"], "finding text")?,
        "finding text",
    )?;
    let semantic_kind = fields::text(
        map,
        &["semanticKind", "semantic_kind", "kind"],
        "finding semantic kind",
    )?;
    let source = fields::text(
        map,
        &["source", "findingSource", "finding_source"],
        "finding source",
    )?;
    let unclassified = path.is_none() && (semantic_kind.is_none() || source.is_none());
    let id_derived = id.is_none();
    let id = id.unwrap_or_else(|| fields::derived_id(message_id, span, value));
    let source_span = map
        .get("sourceSpan")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"path": span}));
    Ok(Finding {
        raw: value.clone(),
        id,
        id_derived,
        path,
        text,
        severity: fields::value(map, &["severity"], "finding severity")?,
        disposition: fields::value(map, &["disposition"], "finding disposition")?,
        semantic_kind,
        source,
        unclassified,
        span: serde_json::json!({
            "path": span,
            "derived": id_derived,
            "raw": source_span
        }),
    })
}

pub(super) fn validate_order(events: &mut [Event]) -> Result<(), String> {
    if events.iter().all(|event| event.completed_at.is_some()) {
        events.sort_by_key(|event| event.completed_at);
        if events
            .windows(2)
            .any(|pair| pair[0].completed_at == pair[1].completed_at)
        {
            return Err("review events have ambiguous completion order".into());
        }
    }
    let mut ids = HashSet::new();
    let mut heads = HashSet::new();
    let mut kinds = HashSet::new();
    let mut finding_ids = HashSet::new();
    let mut last_rank = 0;
    for event in events {
        if !ids.insert(event.message_id.clone()) || !heads.insert(event.reviewed_head.clone()) {
            return Err("review source contains duplicate event identity".into());
        }
        if !kinds.insert(event.kind.clone()) {
            return Err("review source contains duplicate review kind".into());
        }
        for finding in &event.findings {
            if !finding_ids.insert(finding.id.clone()) {
                return Err("review source contains duplicate finding id".into());
            }
        }
        let rank = ["full", "delta", "required_current_head"]
            .iter()
            .position(|value| *value == event.kind)
            .ok_or("review source contains unsupported kind")?;
        if rank < last_rank {
            return Err("review source events are reordered".into());
        }
        last_rank = rank;
    }
    Ok(())
}
