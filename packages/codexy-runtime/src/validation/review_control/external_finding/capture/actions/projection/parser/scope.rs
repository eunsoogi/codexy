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
    let start = timestamp(text(step, "started_at", "Actions failed step")?)?;
    let end = timestamp(text(step, "completed_at", "Actions failed step")?)?;
    if start >= end {
        return Err("Actions failed step has an invalid time window".into());
    }
    let (log, end, inclusive_end) = if ambiguous_step_boundary {
        let (selected, group_end) = select_step_log_group(log, &start, &end)?;
        (selected, group_end, false)
    } else {
        (log, end, true)
    };
    let lines = log
        .lines()
        .filter_map(|line| {
            let (timestamp, body) = timestamped(line)?;
            let selected = if inclusive_end {
                timestamp >= start && timestamp <= end
            } else {
                timestamp >= start && timestamp < end
            };
            selected.then_some(body)
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Err("Actions job log has no lines inside the selected step".into());
    }
    project_log(&lines.join("\n"), repository)
}

fn select_step_log_group<'a>(
    log: &'a str,
    start: &Timestamp,
    end: &Timestamp,
) -> Result<(&'a str, Timestamp), String> {
    let mut groups = Vec::new();
    let mut current = None;
    let mut depth = 0usize;
    let mut offset = 0;
    for line in log.split_inclusive('\n') {
        if line.contains("##[group]") {
            let top_level = depth == 0;
            let is_step = line.contains("##[group]Run ");
            let is_cleanup =
                line.contains("##[group]Post Run ") || line.contains("##[group]Complete job");
            if top_level && (is_step || is_cleanup) {
                let boundary = timestamped(line)
                    .map(|(timestamp, _)| timestamp)
                    .ok_or("Actions log group has no timestamped boundary")?;
                if let Some((timestamp, group_start)) = current.take() {
                    groups.push((timestamp, group_start, Some((offset, boundary.clone()))));
                }
                if is_cleanup {
                    break;
                }
                current = Some((boundary, offset + line.len()));
            }
            depth += 1;
        } else if line.contains("##[endgroup]") {
            depth = depth.saturating_sub(1);
        } else if current.is_some() && line.contains(" Post job cleanup.") {
            let boundary = timestamped(line)
                .map(|(timestamp, _)| timestamp)
                .ok_or("Actions cleanup boundary has no timestamp")?;
            if let Some((timestamp, group_start)) = current.take() {
                groups.push((timestamp, group_start, Some((offset, boundary))));
            }
            break;
        }
        offset += line.len();
    }
    if let Some((timestamp, group_start)) = current {
        groups.push((timestamp, group_start, None));
    }
    let candidates = groups
        .iter()
        .filter(|(timestamp, _, _)| timestamp.second == start.second && timestamp >= start)
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return Err("Actions log does not expose one selected step group".into());
    }
    let (_, group_start, closure) = candidates[0];
    let Some((group_end, boundary)) = closure else {
        return Err("Actions selected step group has no authenticated closure".into());
    };
    if boundary.second != end.second || boundary < end {
        return Err("Actions selected step group closure is inconsistent".into());
    }
    Ok((&log[*group_start..*group_end], boundary.clone()))
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::scoped_log;

    fn ambiguous(log: &str) -> Result<String, String> {
        let step = json!({
            "started_at": "2026-01-01T00:01:00Z",
            "completed_at": "2026-01-01T00:02:00Z"
        });
        scoped_log(
            log,
            step.as_object().expect("step"),
            "eunsoogi/codexy",
            true,
        )
    }

    #[test]
    fn uses_the_authenticated_step_window_for_ambiguous_steps() {
        let log = concat!(
            "2026-01-01T00:00:59.900Z ##[group]Run previous step\n",
            "##[endgroup]\n",
            "2026-01-01T00:01:00.000Z ##[group]Run selected step\n",
            "2026-01-01T00:01:30.100Z selected command output\n",
            "2026-01-01T00:01:30.200Z ERROR: selected.unittest (selected)\n",
            "2026-01-01T00:01:30.300Z Traceback (most recent call last):\n",
            "2026-01-01T00:01:30.400Z   File \"D:\\a\\codexy\\codexy\\packages\\getcodexy\\tests\\selected.py\", line 1\n",
            "2026-01-01T00:01:30.500Z NotImplementedError: selected\n",
            "2026-01-01T00:02:00.050Z Post job cleanup.\n",
            "2026-01-01T00:02:00.100Z ERROR: outside.unittest (outside)\n",
        );
        let excerpt = ambiguous(log).expect("selected step log");
        assert!(excerpt.contains("selected"));
        assert!(!excerpt.contains("outside"));
    }

    #[test]
    fn rejects_a_boundary_failure_outside_the_authenticated_group() {
        let log = concat!(
            "2026-01-01T00:01:00.000Z ##[group]Run selected step\n",
            "2026-01-01T00:01:30.100Z selected command output\n",
            "2026-01-01T00:02:00.050Z Post job cleanup.\n",
            "2026-01-01T00:02:00.100Z ERROR: selected.unittest (selected)\n",
            "2026-01-01T00:02:00.200Z Traceback (most recent call last):\n",
            "2026-01-01T00:02:00.300Z   File \"D:\\a\\codexy\\codexy\\packages\\getcodexy\\tests\\selected.py\", line 1\n",
            "2026-01-01T00:02:00.400Z NotImplementedError: selected\n",
        );
        let result = ambiguous(log);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_duplicate_step_groups_with_the_same_start_second() {
        let log = concat!(
            "2026-01-01T00:01:00.100Z ##[group]Run earlier step\n",
            "2026-01-01T00:01:10Z earlier output\n",
            "##[endgroup]\n",
            "2026-01-01T00:01:00.200Z ##[group]Run selected step\n",
            "2026-01-01T00:01:30Z ERROR: selected.unittest (selected)\n",
            "2026-01-01T00:01:31Z Traceback (most recent call last):\n",
            "2026-01-01T00:01:32Z   File \"D:\\a\\codexy\\codexy\\packages\\getcodexy\\tests\\selected.py\", line 1\n",
            "2026-01-01T00:01:33Z NotImplementedError: selected\n",
            "##[endgroup]\n",
            "2026-01-01T00:02:00.050Z Post job cleanup.\n",
        );
        assert!(ambiguous(log).is_err());
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
