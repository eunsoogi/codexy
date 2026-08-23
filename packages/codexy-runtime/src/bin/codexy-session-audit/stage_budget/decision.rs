use super::schema::{
    Decision, Events, Measure, Measures, NextAction, OversizedResult, Owner, ProofState, Safety,
    Stage, StageBudgetReceipt,
};
use super::{Accounting, MeasureFallback};
use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StageBudgetResult {
    pub(crate) valid: bool,
    pub(crate) schema: String,
    pub(crate) metadata_only: bool,
    pub(crate) stage: Stage,
    pub(crate) stage_sequence: u64,
    pub(crate) previous_receipt_identity: Option<String>,
    pub(crate) receipt_identity: String,
    pub(crate) owner: Owner,
    pub(crate) identity: super::schema::Identity,
    pub(crate) units: super::schema::Units,
    pub(crate) safety: Safety,
    pub(crate) proof: ProofState,
    pub(crate) decision: Decision,
    pub(crate) next_action: NextAction,
    pub(crate) budget: Value,
    pub(crate) accounting: Accounting,
    pub(crate) measure_availability: Value,
    pub(crate) oversized_result: Option<OversizedResult>,
}

pub(crate) fn evaluate(receipt: StageBudgetReceipt) -> Result<StageBudgetResult> {
    super::validate_receipt(&receipt)?;
    let duplicates = duplicate_event_count(&receipt.events)?;
    let local_replays = duplicates
        .checked_add(receipt.events.unchanged_waits)
        .and_then(|v| v.checked_add(receipt.events.full_state_replays))
        .and_then(|v| v.checked_add(receipt.events.oversized_preview_reads))
        .context("stage replay accounting overflowed")?;
    let prior = receipt
        .continuity
        .previous
        .as_ref()
        .map_or(0, |v| v.cumulative_replay_events);
    let replay_events = prior
        .checked_add(local_replays)
        .context("cumulative stage replay accounting overflowed")?;
    super::schema::validate_continuity(&receipt, replay_events)?;
    let decision = expected_decision(&receipt, duplicates, replay_events);
    let next_action = next_action(&receipt, &decision);
    if receipt.decision != decision {
        bail!("declared decision does not match bounded stage usage");
    }
    if receipt.next_action != next_action {
        bail!("declared next action does not match bounded stage usage");
    }
    let measure_fallbacks = measure_fallbacks(&receipt.measures)?;
    Ok(StageBudgetResult {
        valid: true,
        schema: receipt.schema,
        metadata_only: receipt.metadata_only,
        stage: receipt.stage,
        stage_sequence: receipt.stage_sequence,
        previous_receipt_identity: receipt.previous_receipt_identity,
        receipt_identity: receipt.receipt_identity,
        owner: receipt.owner,
        identity: receipt.identity,
        units: receipt.units,
        safety: receipt.safety,
        proof: receipt.proof,
        decision,
        next_action,
        budget: json!({
            "contextBytes": receipt.usage.context_bytes,
            "contextLimitBytes": receipt.limits.context_bytes,
            "toolOutputBytes": receipt.usage.tool_output_bytes,
            "toolOutputLimitBytes": receipt.limits.tool_output_bytes,
            "turns": receipt.usage.turns, "turnLimit": receipt.limits.turns,
            "toolCalls": receipt.usage.tool_calls, "toolCallLimit": receipt.limits.tool_calls,
            "replayEvents": replay_events, "replayLimit": receipt.limits.replay_events
        }),
        accounting: Accounting {
            duplicate_event_count: duplicates,
            unchanged_waits: receipt.events.unchanged_waits,
            full_state_replays: receipt.events.full_state_replays,
            oversized_preview_reads: receipt.events.oversized_preview_reads,
            replay_events,
            measure_fallbacks,
        },
        measure_availability: json!({
            "inputTokens": measure_state(&receipt.measures.input_tokens),
            "wallTimeMs": measure_state(&receipt.measures.wall_time_ms),
            "observedCostUsd": measure_state(&receipt.measures.observed_cost_usd),
            "toolInputBytes": measure_state(&receipt.measures.tool_input_bytes),
            "toolOutputBytes": measure_state(&receipt.measures.tool_output_bytes),
            "cacheInputTokens": measure_state(&receipt.measures.cache_input_tokens)
        }),
        oversized_result: receipt.oversized_result,
    })
}

fn duplicate_event_count(events: &Events) -> Result<u64> {
    let mut seen = HashSet::with_capacity(events.identities.len());
    events.identities.iter().try_fold(0_u64, |count, id| {
        if seen.insert(id) {
            Ok(count)
        } else {
            count
                .checked_add(1)
                .context("duplicate event accounting overflowed")
        }
    })
}

fn expected_decision(r: &StageBudgetReceipt, duplicates: u64, replay: u64) -> Decision {
    if reviewer_gate_is_terminal(r) {
        return "stop_and_handoff".to_string();
    }
    let exhausted = [
        r.usage.context_bytes >= r.limits.context_bytes,
        r.usage.tool_output_bytes >= r.limits.tool_output_bytes,
        r.usage.turns >= r.limits.turns,
        r.usage.tool_calls >= r.limits.tool_calls,
        replay >= r.limits.replay_events,
    ]
    .into_iter()
    .any(|v| v);
    let oversized = r
        .oversized_result
        .as_ref()
        .is_some_and(|v| v.state != "unavailable");
    if exhausted || oversized {
        return "stop_and_handoff".to_string();
    }
    let replay_seen = duplicates > 0
        || r.events.unchanged_waits > 0
        || r.events.full_state_replays > 0
        || r.events.oversized_preview_reads > 0;
    if replay_seen
        || near(r.usage.context_bytes, r.limits.context_bytes)
        || near(r.usage.tool_output_bytes, r.limits.tool_output_bytes)
        || near(r.usage.turns, r.limits.turns)
        || near(r.usage.tool_calls, r.limits.tool_calls)
        || near(replay, r.limits.replay_events)
    {
        "compact".to_string()
    } else {
        "continue".to_string()
    }
}

fn reviewer_gate_is_terminal(r: &StageBudgetReceipt) -> bool {
    if !["selected-review", "wait"].contains(&r.stage.as_str()) {
        return false;
    }
    ["pass", "block", "unobservable"].contains(&r.safety.selected_reviewer_state.as_str())
        || ["pass", "blocked", "not-applicable"].contains(&r.safety.external_gate.as_str())
        || (r.safety.selected_reviewer_state == "not-applicable"
            && r.safety.external_gate == "none")
}

fn near(value: u64, limit: u64) -> bool {
    u128::from(value) * 100 >= u128::from(limit) * 80
}

fn next_action(r: &StageBudgetReceipt, decision: &str) -> NextAction {
    if decision == "continue"
        && (r.stage == "wait"
            || (r.stage == "selected-review"
                && ["pending", "running"].contains(&r.safety.selected_reviewer_state.as_str())))
    {
        return "wait-for-event".to_string();
    }
    match decision {
        "continue" => "continue-stage",
        "compact" => "compact-context",
        "stop_and_handoff" => "handoff-parent",
        _ => "invalid",
    }
    .to_string()
}

fn measure_fallbacks(m: &Measures) -> Result<Vec<MeasureFallback>> {
    [
        fallback(
            "input-tokens",
            "context-bytes",
            &m.input_tokens.availability,
            m.input_tokens.reason.as_deref(),
        ),
        fallback(
            "wall-time-ms",
            "turns",
            &m.wall_time_ms.availability,
            m.wall_time_ms.reason.as_deref(),
        ),
        fallback(
            "observed-cost-usd",
            "tool-output-bytes",
            &m.observed_cost_usd.availability,
            m.observed_cost_usd.reason.as_deref(),
        ),
        fallback(
            "tool-input-bytes",
            "tool-calls",
            &m.tool_input_bytes.availability,
            m.tool_input_bytes.reason.as_deref(),
        ),
        fallback(
            "tool-output-bytes",
            "tool-calls-and-turns",
            &m.tool_output_bytes.availability,
            m.tool_output_bytes.reason.as_deref(),
        ),
        fallback(
            "cache-input-tokens",
            "unavailable-not-zero",
            &m.cache_input_tokens.availability,
            m.cache_input_tokens.reason.as_deref(),
        ),
    ]
    .into_iter()
    .collect::<Result<Vec<_>>>()
    .map(|items| items.into_iter().flatten().collect())
}

fn fallback(
    measure: &str,
    name: &str,
    availability: &str,
    reason: Option<&str>,
) -> Result<Option<MeasureFallback>> {
    if availability != "unavailable" {
        return Ok(None);
    }
    Ok(Some(MeasureFallback {
        measure: measure.to_string(),
        reason: reason
            .context("validated unavailable measure reason")?
            .to_string(),
        fallback: name.to_string(),
    }))
}

fn measure_state<T: Serialize>(m: &Measure<T>) -> Value {
    json!({"state":m.availability,"value":&m.value,"reason":&m.reason})
}
