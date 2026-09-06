use serde_json::{Value, json};

use super::super::source::{Event, Finding, Invocation};

pub(super) fn invocation_value(invocation: &Invocation) -> Value {
    json!({
        "id": invocation.id,
        "sender": invocation.sender,
        "receiver": invocation.receiver,
        "prompt": invocation.prompt,
        "model": invocation.model,
        "reasoning_effort": invocation.reasoning_effort,
        "raw": invocation.raw,
        "source": invocation.source
    })
}

pub(super) fn event_value(event: &Event) -> Result<Value, String> {
    let findings = event.findings.iter().map(finding_value).collect::<Vec<_>>();
    let source_path = event
        .source
        .get("path")
        .and_then(Value::as_str)
        .ok_or("native-history event source path is missing")?;
    Ok(json!({
        "id": event.message_id,
        "message_id": event.message_id,
        "turn_id": event.turn_id,
        "kind": event.kind,
        "reviewed_head": event.reviewed_head,
        "terminal_result": event.terminal_result,
        "findings": findings,
        "raw_message": event.raw,
        "raw_text": event.text,
        "completed_at": event.completed_at,
        "source_sequence": event.source_sequence,
        "source": event.source,
        "field_sources": {
            "kind": format!("{source_path}.review.kind"),
            "reviewed_head": format!("{source_path}.review.reviewed_head"),
            "terminal_result": format!("{source_path}.review.terminal_result"),
            "findings": format!("{source_path}.review.findings"),
            "spans": event.field_sources
        },
        "reviewer": {
            "model": event.model,
            "reasoning_effort": event.reasoning_effort,
            "source": event.model_source
        }
    }))
}

pub(super) fn history_value(event: &Event) -> Value {
    let findings = event.findings.iter().map(finding_value).collect::<Vec<_>>();
    json!({
        "id": event.message_id,
        "kind": event.kind,
        "reviewer": {
            "model": event.model,
            "reasoning_effort": event.reasoning_effort
        },
        "reviewed_head": event.reviewed_head,
        "terminal_result": event.terminal_result,
        "unresolved_findings": findings
    })
}

fn finding_value(finding: &Finding) -> Value {
    let span = finding.span.clone();
    let mut value = json!({
        "id": finding.id,
        "path": finding.path,
        "text": finding.text,
        "metadata": {
            "derived": finding.id_derived,
            "unclassified": finding.unclassified,
            "source_span": span.clone()
        },
        "source_span": span,
        "raw": finding.raw
    });
    let Some(map) = value.as_object_mut() else {
        return value;
    };
    if let Some(severity) = &finding.severity {
        map.insert("severity".into(), severity.clone());
    }
    if let Some(disposition) = &finding.disposition {
        map.insert("disposition".into(), disposition.clone());
    }
    if let Some(kind) = &finding.semantic_kind {
        map.insert("semantic_kind".into(), Value::String(kind.clone()));
    }
    if let Some(source) = &finding.source {
        map.insert("source".into(), Value::String(source.clone()));
    }
    if finding.unclassified {
        map.insert(
            "semantic_status".into(),
            Value::String("unclassified".into()),
        );
    }
    value
}
