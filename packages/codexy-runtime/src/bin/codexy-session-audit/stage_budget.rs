use anyhow::{Context as _, Result, bail};
use serde::Serialize;
use serde_json::Value;
use std::{fs, io::Read as _, path::Path};
#[path = "stage_budget/decision.rs"]
mod decision;
#[path = "stage_budget/schema.rs"]
mod schema;
#[path = "stage_budget/validation.rs"]
mod validation;
pub(crate) use decision::StageBudgetResult;
pub(crate) use validation::validate_receipt;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Accounting {
    duplicate_event_count: u64,
    unchanged_waits: u64,
    full_state_replays: u64,
    oversized_preview_reads: u64,
    replay_events: u64,
    measure_fallbacks: Vec<MeasureFallback>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MeasureFallback {
    measure: String,
    reason: String,
    fallback: String,
}
pub(crate) fn validate_file(path: &Path) -> Result<StageBudgetResult> {
    let file = fs::File::open(path)
        .with_context(|| format!("opening stage budget receipt {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take((super::MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading stage budget receipt {}", path.display()))?;
    if bytes.len() > super::MAX_INPUT_BYTES {
        bail!(
            "stage budget receipt exceeds {} bytes",
            super::MAX_INPUT_BYTES
        );
    }
    let receipt: schema::StageBudgetReceipt = serde_json::from_slice(&bytes)
        .context("stage budget receipt must use the closed metadata schema")?;
    decision::evaluate(receipt)
}
pub(crate) enum InputFormat {
    Codex,
    Generic,
}
pub(crate) fn detect_input_format(input: &str) -> Result<InputFormat> {
    let mut codex = false;
    let mut generic = false;
    for (index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .with_context(|| format!("invalid JSON on metadata line {}", index + 1))?;
        let object = value
            .as_object()
            .with_context(|| format!("metadata line {} must be a JSON object", index + 1))?;
        codex |= object.get("type").and_then(Value::as_str) == Some("session_meta");
        generic |= object.get("event").and_then(Value::as_str) == Some("turn.completed");
    }
    if codex && generic {
        bail!("mixed generic and Codex session metadata formats are not allowed");
    }
    Ok(if codex {
        InputFormat::Codex
    } else {
        InputFormat::Generic
    })
}
