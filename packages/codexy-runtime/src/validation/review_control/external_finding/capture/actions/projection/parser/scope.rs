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
    log_group_index: usize,
) -> Result<String, String> {
    if ambiguous_step_boundary {
        return project_log(select_step_log_group(log, log_group_index)?, repository);
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
    project_log(&lines.join("\n"), repository)
}

fn project_log(log: &str, repository: &str) -> Result<String, String> {
    let normalized = normalize_log(log);
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

fn select_step_log_group(log: &str, expected: usize) -> Result<&str, String> {
    if expected == 0 {
        return Err("Actions selected step log group is invalid".into());
    }
    let mut group = 0;
    let mut depth = 0usize;
    let mut start = None;
    let mut end = log.len();
    let mut offset = 0;
    for line in log.split_inclusive('\n') {
        if line.contains("##[group]") {
            let top_level = depth == 0;
            let is_step = line.contains("##[group]Run ");
            let is_cleanup =
                line.contains("##[group]Post Run ") || line.contains("##[group]Complete job");
            if top_level && is_cleanup && start.is_some() {
                end = offset;
                break;
            }
            if top_level && is_step {
                group += 1;
                if start.is_some() {
                    end = offset;
                    break;
                }
                if group == expected {
                    start = Some(offset + line.len());
                }
            }
            depth += 1;
        } else if line.contains("##[endgroup]") {
            depth = depth.saturating_sub(1);
        } else if start.is_some() && (line.contains(" Post job cleanup.")) {
            end = offset;
            break;
        }
        offset += line.len();
    }
    let start = start.ok_or("Actions job log does not expose the selected step group")?;
    let selected = &log[start..end];
    if selected.trim().is_empty() {
        return Err("Actions selected step log group is empty".into());
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::scoped_log;

    #[test]
    fn uses_the_authenticated_log_group_for_same_second_steps() {
        let step = json!({
            "started_at": "2026-01-01T00:01:00Z",
            "completed_at": "2026-01-01T00:02:00Z"
        });
        let log = concat!(
            "##[group]Run previous step\n",
            "##[group]Run nested previous command\n",
            "2026-01-01T00:01:00.100Z ERROR: unrelated.unittest (unrelated)\n",
            "2026-01-01T00:01:00.200Z Traceback (most recent call last):\n",
            "2026-01-01T00:01:00.300Z   File \"D:\\a\\codexy\\codexy\\packages\\getcodexy\\tests\\unrelated.py\", line 1\n",
            "2026-01-01T00:01:00.400Z NotImplementedError: unrelated\n",
            "##[endgroup]\n",
            "##[endgroup]\n",
            "##[group]Run selected step\n",
            "2026-01-01T00:02:00.100Z ERROR: packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests.test_process_result_captures_bounded_diagnostics (packages.getcodexy.tests.test_component_capability_probe.CapabilityProcessTests)\n",
            "2026-01-01T00:02:00.200Z Traceback (most recent call last):\n",
            "2026-01-01T00:02:00.300Z   File \"D:\\a\\codexy\\codexy\\packages\\getcodexy\\tests\\test_component_capability_probe.py\", line 57\n",
            "2026-01-01T00:02:00.400Z NotImplementedError: outside\n",
            "##[endgroup]\n",
            "##[group]Post Run actions/checkout@v7\n",
        );
        let excerpt = scoped_log(
            log,
            step.as_object().expect("step"),
            "eunsoogi/codexy",
            true,
            2,
        )
        .expect("selected step log");
        assert!(excerpt.contains("NotImplementedError"));
        assert!(!excerpt.contains("unrelated"));
    }
}

fn timestamped(line: &str) -> Option<(Timestamp, &str)> {
    let end = line.find('Z')?;
    Some((timestamp(&line[..=end]).ok()?, line[end + 1..].trim_start()))
}

fn timestamp(value: &str) -> Result<Timestamp, String> {
    let value = value
        .strip_suffix('Z')
        .ok_or("Actions step timestamps must be UTC ISO-8601 values")?;
    let (second, fraction) = value.split_once('.').unwrap_or((value, ""));
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
