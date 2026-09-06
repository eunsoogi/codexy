use serde_json::{Map, Value};

use super::{Invocation, PageSet, fields};

struct Spawn {
    raw: Value,
    id: Option<String>,
    status: Option<String>,
    sender: Option<String>,
    receivers: Vec<String>,
    prompt: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    receiver_agents: Vec<ReceiverAgent>,
    page: usize,
    turn: usize,
    item: usize,
}

pub(super) fn select(
    pages: &PageSet,
    reviewer_thread: &str,
) -> Result<(Invocation, Vec<Value>), String> {
    let spawns = collect_spawns(pages)?;
    let matching = spawns
        .iter()
        .filter(|spawn| spawn.receivers.len() == 1 && spawn.receivers[0] == reviewer_thread)
        .collect::<Vec<_>>();
    if spawns.iter().any(|spawn| {
        spawn
            .receivers
            .iter()
            .any(|receiver| receiver == reviewer_thread)
            && spawn.receivers.len() != 1
    }) {
        return Err("reviewer invocation must have exactly one receiver".into());
    }
    if matching.len() != 1 {
        return Err("owner must contain exactly one matching spawnAgent".into());
    }
    let selected = matching[0];
    if selected.status.as_deref() != Some("completed") {
        return Err("matching reviewer spawnAgent must be completed".into());
    }
    let receiver_agent = selected
        .receiver_agents
        .iter()
        .filter(|agent| agent.thread == reviewer_thread)
        .collect::<Vec<_>>();
    if receiver_agent.len() != 1 {
        return Err("reviewer invocation must preserve exactly one receiver agent".into());
    }
    let receiver_agent = receiver_agent[0];
    let role = fields::required(receiver_agent.role.clone(), "reviewer receiver agent role")?;
    if role != "codexy-sentinel" {
        return Err("reviewer receiver agent role is not codexy-sentinel".into());
    }
    let prompt = fields::required(selected.prompt.clone(), "spawnAgent prompt")?;
    let invocation = Invocation {
        raw: selected.raw.clone(),
        id: fields::required(selected.id.clone(), "spawnAgent id")?,
        sender: fields::required(selected.sender.clone(), "spawnAgent sender")?,
        receiver: reviewer_thread.to_owned(),
        prompt,
        model: fields::required(selected.model.clone(), "spawnAgent model")?,
        reasoning_effort: fields::required(selected.effort.clone(), "spawnAgent reasoningEffort")?,
        receiver_role: role,
        source: serde_json::json!({
            "page_index": selected.page,
            "turn_index": selected.turn,
            "item_index": selected.item
        }),
    };
    let helpers = spawns
        .iter()
        .filter(|spawn| !std::ptr::eq(*spawn, selected))
        .map(|spawn| serde_json::json!({"raw": spawn.raw, "excluded": true}))
        .collect::<Vec<_>>();
    Ok((invocation, helpers))
}

fn collect_spawns(pages: &PageSet) -> Result<Vec<Spawn>, String> {
    let mut result = Vec::new();
    for turn in &pages.turns {
        let turn_object = fields::object(&turn.value, "owner turn")?;
        let Some(items) = turn_object.get("items").and_then(Value::as_array) else {
            continue;
        };
        let mut objects = Vec::new();
        for (index, item) in items.iter().enumerate() {
            fields::collect(item, index, &mut objects);
        }
        for (item_index, item) in objects {
            let map = fields::object(&item, "owner item")?;
            if fields::text(map, &["type"], "tool item type")?.as_deref()
                != Some("collabAgentToolCall")
                || !matches!(
                    fields::text(map, &["tool"], "tool name")?.as_deref(),
                    Some("spawnAgent" | "spawn_agent")
                )
            {
                continue;
            }
            result.push(Spawn {
                raw: item.clone(),
                id: fields::text(map, &["id"], "spawn id")?,
                status: fields::text(map, &["status"], "spawn status")?,
                sender: fields::text(
                    map,
                    &["senderThreadId", "sender_thread_id", "sender"],
                    "spawn sender",
                )?,
                receivers: receivers(map)?,
                prompt: fields::text(map, &["prompt"], "spawn prompt")?,
                model: fields::text(map, &["model"], "spawn model")?,
                effort: fields::text(
                    map,
                    &["reasoningEffort", "reasoning_effort", "thinking"],
                    "spawn effort",
                )?,
                receiver_agents: receiver_agents(map)?,
                page: turn.page,
                turn: turn.index,
                item: item_index,
            });
        }
    }
    Ok(result)
}

fn receivers(map: &serde_json::Map<String, Value>) -> Result<Vec<String>, String> {
    let array = fields::text_array(
        map,
        &["receiverThreadIds", "receiver_thread_ids"],
        "spawn receivers",
    )?;
    let direct = fields::text(
        map,
        &["receiverThreadId", "receiver_thread_id", "receiver"],
        "spawn receiver",
    )?;
    if array.is_some() && direct.is_some() {
        return Err("spawn receiver aliases are contradictory".into());
    }
    Ok(array
        .or_else(|| direct.map(|receiver| vec![receiver]))
        .unwrap_or_default())
}

#[derive(Clone)]
struct ReceiverAgent {
    thread: String,
    role: Option<String>,
}

fn receiver_agents(map: &Map<String, Value>) -> Result<Vec<ReceiverAgent>, String> {
    let Some(value) = map.get("receiver_agents") else {
        return Ok(Vec::new());
    };
    let agents = value
        .as_array()
        .ok_or("spawn receiver_agents must be an array")?;
    agents
        .iter()
        .map(|value| {
            let agent = fields::object(value, "receiver agent")?;
            Ok(ReceiverAgent {
                thread: fields::required(
                    fields::text(
                        agent,
                        &["thread_id", "threadId", "receiverThreadId"],
                        "receiver agent thread",
                    )?,
                    "receiver agent thread",
                )?,
                role: fields::text(
                    agent,
                    &["agent_role", "agentRole", "role"],
                    "receiver agent role",
                )?,
            })
        })
        .collect()
}
