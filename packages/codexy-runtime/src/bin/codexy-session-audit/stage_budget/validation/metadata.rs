use super::super::schema::{Events, Limits, Measure, Measures, OversizedResult, Usage};
use anyhow::{Context as _, Result, bail};

pub(super) fn closed(value: &str, allowed: &[&str], label: &str) -> Result<()> {
    if !allowed.contains(&value) {
        bail!("{label} is outside the closed contract");
    }
    Ok(())
}

pub(super) fn validate_events(events: &Events) -> Result<()> {
    if events.identities.is_empty() || events.identities.len() > 256 {
        bail!("event identities must contain between 1 and 256 bounded identifiers");
    }
    for identity in &events.identities {
        validate_token(identity, "event identity")?;
    }
    Ok(())
}

pub(super) fn validate_measures(
    measures: &Measures,
    output: u64,
    result: Option<&OversizedResult>,
) -> Result<()> {
    validate_measure(&measures.input_tokens, "input tokens")?;
    validate_measure(&measures.wall_time_ms, "wall time")?;
    validate_measure(&measures.observed_cost_usd, "observed cost")?;
    validate_measure(&measures.tool_input_bytes, "tool input bytes")?;
    validate_measure(&measures.tool_output_bytes, "tool output bytes")?;
    validate_measure(&measures.cache_input_tokens, "cache input tokens")?;
    if measures.tool_output_bytes.availability == "unavailable" {
        if !result.is_some_and(|value| value.state == "unavailable") || output != 0 {
            bail!(
                "unavailable tool output bytes require zero usage and unavailable result metadata"
            );
        }
    } else if measures.tool_output_bytes.value != Some(output) {
        bail!("tool output bytes must match the emitted-byte usage measure");
    } else if result.is_some_and(|value| value.state == "unavailable") {
        bail!("unavailable result metadata requires unavailable tool output bytes");
    }
    if measures
        .observed_cost_usd
        .value
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        bail!("observed cost must be a finite non-negative measure");
    }
    Ok(())
}

fn validate_measure<T>(measure: &Measure<T>, label: &str) -> Result<()> {
    match (
        measure.availability.as_str(),
        measure.value.is_some(),
        measure.reason.as_deref(),
    ) {
        ("available", false, _) | ("available", true, Some(_)) => {
            bail!("{label} available state requires a value and no unavailable reason")
        }
        ("available", true, None) => Ok(()),
        ("unavailable", true, _) => bail!("{label} unavailable state must remain null"),
        ("unavailable", false, reason) => validate_token(
            reason.context("unavailable measures require an explicit reason")?,
            "unavailable measure reason",
        ),
        _ => bail!("measure availability is outside the closed contract"),
    }
}

pub(super) fn validate_oversized_result(
    result: Option<&OversizedResult>,
    events: &Events,
    limits: &Limits,
    usage: &Usage,
) -> Result<()> {
    if let Some(value) = result {
        validate_token(&value.identity, "oversized result identity")?;
        closed(
            &value.kind,
            &["tool-output", "preview", "history"],
            "oversized kind",
        )?;
        closed(
            &value.state,
            &["oversized", "replay-blocked", "unavailable"],
            "oversized state",
        )?;
        if value.state == "unavailable" {
            if value.kind != "tool-output" || value.bytes != 0 {
                bail!("unavailable oversized results must be zero-byte tool output metadata");
            }
        } else {
            let limit = if value.kind == "tool-output" {
                limits.tool_output_bytes
            } else {
                limits.context_bytes
            };
            let accounted = if value.kind == "tool-output" {
                usage.tool_output_bytes
            } else {
                usage.context_bytes
            };
            if value.bytes <= limit || accounted < value.bytes {
                bail!("oversized result must exceed and be included in its configured budget");
            }
        }
        if value.body_replayed {
            bail!("oversized result body replay is not permitted");
        }
    }
    if events.oversized_preview_reads > 0
        && (result.is_none() || result.is_some_and(|value| value.kind == "tool-output"))
    {
        bail!("oversized preview reads require preview or history metadata");
    }
    Ok(())
}

pub(super) fn validate_token(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 256
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/' | b':' | b'@')
        })
    {
        bail!("{label} must be bounded metadata");
    }
    Ok(())
}

pub(super) fn validate_sha(value: &str, label: &str) -> Result<()> {
    if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} must be a 40-character hexadecimal identity");
    }
    Ok(())
}

pub(super) fn validate_digest(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("receipt identity must be a 64-character SHA-256 identity");
    }
    Ok(())
}
