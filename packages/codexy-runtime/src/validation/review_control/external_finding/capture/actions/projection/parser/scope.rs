use regex::Regex;
use serde_json::{Map, Value};

use super::{normalize_log, parse_log};
use crate::validation::review_control::pre_pr::text;

const MAX_EXCERPT_BYTES: usize = 16 * 1024;

pub(super) fn scoped_log(
    log: &str,
    step: &Map<String, Value>,
    repository: &str,
) -> Result<String, String> {
    let start = bound(text(step, "started_at", "Actions failed step")?, false)?;
    let end = bound(text(step, "completed_at", "Actions failed step")?, true)?;
    if start >= end {
        return Err("Actions failed step has an invalid time window".into());
    }
    let lines = log
        .lines()
        .filter_map(|line| {
            let (timestamp, body) = timestamped(line)?;
            (timestamp >= start.as_str() && timestamp <= end.as_str()).then_some(body)
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err("Actions job log has no lines inside the selected step".into());
    }
    let normalized = normalize_log(&lines.join("\n"));
    parse_log(&normalized, repository)?;
    let lines = normalized.lines().collect::<Vec<_>>();
    let first = lines
        .iter()
        .position(|line| line.starts_with("ERROR:"))
        .ok_or("Actions selected step has no canonical ERROR")?;
    let exception = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*(?:Error|Exception):")
        .map_err(|error| error.to_string())?;
    let last = lines
        .iter()
        .enumerate()
        .skip(first)
        .find(|(_, line)| exception.is_match(line))
        .map(|(index, _)| index)
        .ok_or("Actions selected step has no canonical exception")?;
    let excerpt = lines[first..=last].join("\n");
    if excerpt.len() > MAX_EXCERPT_BYTES {
        return Err("Actions selected-step excerpt is too large".into());
    }
    Ok(excerpt)
}

fn timestamped(line: &str) -> Option<(&str, &str)> {
    let end = line.find('Z')?;
    let timestamp = &line[..=end];
    if timestamp.len() < 20 || !timestamp.contains('T') {
        return None;
    }
    Some((timestamp, line[end + 1..].trim_start()))
}

fn bound(value: &str, upper: bool) -> Result<String, String> {
    if !value.ends_with('Z') || !value.contains('T') {
        return Err("Actions failed step timestamps must be UTC ISO-8601 values".into());
    }
    if value.contains('.') {
        return Ok(value.to_owned());
    }
    let suffix = if upper { ".9999999Z" } else { ".0000000Z" };
    Ok(format!("{}{}", value.trim_end_matches('Z'), suffix))
}
