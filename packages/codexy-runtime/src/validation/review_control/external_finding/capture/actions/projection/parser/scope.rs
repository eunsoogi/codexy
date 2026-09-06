use regex::Regex;
use serde_json::{Map, Value};

use super::{normalize_log, parse_log};
use crate::validation::review_control::pre_pr::text;

const MAX_EXCERPT_BYTES: usize = 16 * 1024;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct Timestamp {
    second: String,
    nanos: u32,
}

pub(super) fn scoped_log(
    log: &str,
    step: &Map<String, Value>,
    repository: &str,
    ambiguous_step_boundary: bool,
) -> Result<String, String> {
    if ambiguous_step_boundary {
        return Err(
            "Actions selected step shares a timestamp boundary with an adjacent step".into(),
        );
    }
    let start = timestamp(text(step, "started_at", "Actions failed step")?)?;
    let end = timestamp(text(step, "completed_at", "Actions failed step")?)?;
    if start >= end {
        return Err("Actions failed step has an invalid time window".into());
    }
    let lines = log
        .lines()
        .filter_map(|line| {
            let (timestamp, body) = timestamped(line)?;
            (timestamp >= start && timestamp <= end).then_some(body)
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

fn timestamped(line: &str) -> Option<(Timestamp, &str)> {
    let end = line.find('Z')?;
    Some((timestamp(&line[..=end]).ok()?, line[end + 1..].trim_start()))
}

fn timestamp(value: &str) -> Result<Timestamp, String> {
    let value = value
        .strip_suffix('Z')
        .ok_or("Actions step timestamps must be UTC ISO-8601 values")?;
    let (second, fraction) = value.split_once('.').map_or((value, ""), |parts| parts);
    let bytes = second.as_bytes();
    if second.len() != 19
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || !second
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7 | 10 | 13 | 16) || byte.is_ascii_digit())
    {
        return Err("Actions failed step timestamps must be UTC ISO-8601 values".into());
    }
    if fraction.bytes().any(|byte| !byte.is_ascii_digit()) || fraction.len() > 9 {
        return Err("Actions failed step timestamp precision is invalid".into());
    }
    let nanos = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u32>()
            .map_err(|_| "Actions failed step timestamp precision is invalid".to_owned())?
            * 10u32.pow(9 - fraction.len() as u32)
    };
    Ok(Timestamp {
        second: second.to_owned(),
        nanos,
    })
}
