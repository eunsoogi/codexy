use serde_json::{Map, Value};

use super::super::fields;
use super::Spawn;

const CAPTURE_PROVENANCE_SCHEMA: &str =
    "codexy.review-control-native-history-capture-provenance.v1";

pub(super) fn legacy_role(
    selected: &Spawn,
    reviewer_thread: &str,
) -> Result<Option<String>, String> {
    let Some(receiver_agents) = &selected.receiver_agents else {
        return Ok(None);
    };
    let matching = receiver_agents
        .iter()
        .filter(|agent| agent.thread == reviewer_thread)
        .collect::<Vec<_>>();
    if matching.len() != 1 {
        return Err("reviewer invocation must preserve exactly one receiver agent".into());
    }
    Ok(matching[0].role.clone())
}

pub(super) fn capture_provenance(
    capture: Option<&Value>,
    owner_thread: &str,
    reviewer_thread: &str,
    selected: &Spawn,
) -> Result<Option<String>, String> {
    let Some(capture) = capture else {
        return Ok(None);
    };
    let capture = fields::object(capture, "reviewer capture")?;
    let Some(provenance_value) = capture.get("provenance") else {
        return Ok(None);
    };
    let provenance = fields::object(provenance_value, "capture provenance")?;
    if fields::text(provenance, &["schema"], "capture provenance schema")?.as_deref()
        != Some(CAPTURE_PROVENANCE_SCHEMA)
    {
        return Err("capture provenance schema is unsupported".into());
    }
    let session = fields::object(
        provenance
            .get("session")
            .ok_or("capture provenance must contain session")?,
        "capture provenance session",
    )?;
    let nested = nested_session_source(session)?;
    let mut sources = vec![session.clone()];
    if let Some(nested) = nested {
        sources.push(nested);
    }
    let session_id = fields::required(
        fields::merged_text(
            &sources,
            &["id", "session_id", "sessionId", "thread_id", "threadId"],
            "capture reviewer session id",
        )?,
        "capture reviewer session id",
    )?;
    let parent_thread = fields::required(
        fields::merged_text(
            &sources,
            &["parent_thread_id", "parentThreadId"],
            "capture reviewer parent thread",
        )?,
        "capture reviewer parent thread",
    )?;
    let role = fields::required(
        fields::merged_text(
            &sources,
            &["agent_role", "agentRole", "role"],
            "capture reviewer agent role",
        )?,
        "capture reviewer agent role",
    )?;
    if session_id != reviewer_thread {
        return Err("capture reviewer session does not match reviewer thread".into());
    }
    if parent_thread != owner_thread {
        return Err("capture reviewer parent does not match owner thread".into());
    }

    let spawn = fields::object(
        provenance
            .get("spawn")
            .ok_or("capture provenance must contain spawn")?,
        "capture provenance spawn",
    )?;
    let spawn_id = fields::required(
        fields::text(spawn, &["id", "call_id", "callId"], "capture spawn id")?,
        "capture spawn id",
    )?;
    let spawn_sender = fields::required(
        fields::text(
            spawn,
            &["senderThreadId", "sender_thread_id", "sender"],
            "capture spawn sender",
        )?,
        "capture spawn sender",
    )?;
    let spawn_receivers = fields::text_array(
        spawn,
        &["receiverThreadIds", "receiver_thread_ids"],
        "capture spawn receivers",
    )?
    .ok_or("capture spawn receivers must be present")?;
    let spawn_status = fields::required(
        fields::text(spawn, &["status"], "capture spawn status")?,
        "capture spawn status",
    )?;
    if spawn_status != "completed" {
        return Err("capture spawn must be completed".into());
    }
    if spawn_receivers != [reviewer_thread.to_owned()] {
        return Err("capture spawn receiver does not match reviewer thread".into());
    }
    if spawn_sender != owner_thread {
        return Err("capture spawn sender does not match owner thread".into());
    }
    if selected.id.as_deref() != Some(spawn_id.as_str()) {
        return Err("capture spawn id does not match selected invocation".into());
    }
    if selected.sender.as_deref() != Some(spawn_sender.as_str()) {
        return Err("capture spawn sender does not match selected invocation".into());
    }
    Ok(Some(role))
}

fn nested_session_source(
    session: &Map<String, Value>,
) -> Result<Option<Map<String, Value>>, String> {
    let Some(source) = session.get("source") else {
        return Ok(None);
    };
    let source = fields::object(source, "capture provenance session source")?;
    let Some(subagent) = source.get("subagent") else {
        return Ok(None);
    };
    let subagent = fields::object(subagent, "capture provenance subagent")?;
    let Some(thread_spawn) = subagent.get("thread_spawn") else {
        return Ok(None);
    };
    Ok(Some(
        fields::object(thread_spawn, "capture provenance thread spawn")?.clone(),
    ))
}
