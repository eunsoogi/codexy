#[path = "reviewer/finding.rs"]
mod finding;
#[path = "reviewer/metadata.rs"]
mod metadata;

use serde_json::Value;

use super::{Event, Finding, Invocation, PageSet, fields};

pub(super) fn events(pages: &PageSet, invocation: &Invocation) -> Result<Vec<Event>, String> {
    let mut events = Vec::new();
    for turn in &pages.turns {
        let turn_map = fields::object(&turn.value, "reviewer turn")?;
        let Some(items) = turn_map.get("items").and_then(Value::as_array) else {
            continue;
        };
        let mut objects = Vec::new();
        for (index, item) in items.iter().enumerate() {
            fields::collect(item, index, &mut objects);
        }
        let finals = objects
            .into_iter()
            .filter(|(_, item)| is_final_message(item))
            .collect::<Vec<_>>();
        if finals.len() > 1 {
            return Err("reviewer turn contains multiple final messages".into());
        }
        let Some((item_index, message)) = finals.into_iter().next() else {
            continue;
        };
        let message_map = fields::object(&message, "final message")?;
        let text_value = fields::required(
            fields::text(message_map, &["text", "content"], "final message text")?,
            "final message text",
        )?;
        let base = format!(
            "pages[{}].turns[{}].items[{}]",
            turn.page, turn.index, item_index
        );
        let turn_base = format!("pages[{}].turns[{}]", turn.page, turn.index);
        let candidates = metadata::candidates(turn_map, message_map, &text_value, &turn_base)?;
        if candidates.is_empty() {
            continue;
        }
        let kind = fields::required(
            fields::merged_text(
                &candidates,
                &["kind", "reviewKind", "review_kind"],
                "review kind",
            )?,
            "review kind",
        )?;
        if !["full", "delta", "required_current_head"].contains(&kind.as_str()) {
            return Err("review kind is unsupported".into());
        }
        let reviewed_head = fields::required(
            fields::merged_text(
                &candidates,
                &["reviewedHead", "reviewed_head"],
                "reviewed head",
            )?,
            "reviewed head",
        )?;
        if !fields::is_sha(&reviewed_head) {
            return Err("reviewed head must be a 40-character SHA".into());
        }
        let terminal_result = fields::required(
            fields::merged_text(
                &candidates,
                &["terminalResult", "terminal_result", "verdict"],
                "terminal result",
            )?,
            "terminal result",
        )?;
        if !["PASS", "BLOCK", "UNOBSERVABLE"].contains(&terminal_result.as_str()) {
            return Err("terminal result is unsupported".into());
        }
        let finding_value = fields::merged_value(&candidates, &["findings"], "findings")?
            .ok_or("review result must contain findings")?;
        let finding_values = finding_value
            .as_array()
            .ok_or("review findings must be an array")?;
        let message_id = fields::required(
            fields::text(message_map, &["id"], "message id")?,
            "message id",
        )?;
        let completed_at = fields::merged_u64(
            &[turn_map.clone(), message_map.clone()],
            &["completedAt", "completed_at"],
            "completedAt",
        )?;
        let (model, effort, model_source) = metadata::reviewer_facts(&candidates, invocation)?;
        let findings = finding_values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                finding::parse(value, &message_id, &format!("{base}.findings[{index}]"))
            })
            .collect::<Result<Vec<Finding>, _>>()?;
        events.push(Event {
            raw: message,
            message_id,
            turn_id: turn.id.clone(),
            kind,
            reviewed_head,
            terminal_result,
            findings,
            text: text_value,
            completed_at,
            source_sequence: events.len(),
            source: serde_json::json!({
                "path": base,
                "page_index": turn.page,
                "turn_index": turn.index,
                "item_index": item_index
            }),
            field_sources: metadata::field_sources(&candidates, &base),
            model,
            reasoning_effort: effort,
            model_source,
        });
    }
    Ok(events)
}

pub(super) fn validate_order(events: &mut [Event]) -> Result<(), String> {
    finding::validate_order(events)
}

fn is_final_message(value: &Value) -> bool {
    let Ok(map) = fields::object(value, "reviewer item") else {
        return false;
    };
    fields::text(map, &["type"], "message type")
        .ok()
        .flatten()
        .is_some_and(|kind| kind.eq_ignore_ascii_case("agentMessage"))
        && fields::text(map, &["phase"], "message phase")
            .ok()
            .flatten()
            .as_deref()
            == Some("final_answer")
}
