use super::child_lane_thread_tool_handler_capture::has_absent_defect_capture;

pub(super) fn scope_start_until_blank(evidence: &str, line_start: usize) -> (usize, Option<usize>) {
    let mut previous_start = line_start;
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if evidence[candidate_start..previous_end].trim().is_empty() {
            return (previous_start, Some(candidate_start));
        }
        previous_start = candidate_start;
        cursor = candidate_start;
    }
    (previous_start, None)
}
pub(super) fn previous_nonempty_block_start(evidence: &str, block_end: usize) -> Option<usize> {
    let mut block_start = block_end;
    let mut cursor = block_end;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if evidence[candidate_start..previous_end].trim().is_empty() {
            break;
        }
        block_start = candidate_start;
        cursor = candidate_start;
    }
    (block_start != block_end).then_some(block_start)
}
pub(super) fn capture_end_before_unrelated_evidence(
    evidence: &str,
    capture_start: usize,
    handler_start: usize,
) -> usize {
    let mut cursor = line_end(evidence, handler_start);
    let scope_lane = lane_label_for_scope(evidence, capture_start, cursor);
    let mut saw_capture = is_capture_related(&evidence[capture_start..cursor]);
    while cursor < evidence.len() {
        let line_start = cursor + 1;
        let line_end = line_end(evidence, line_start);
        let line = &evidence[line_start..line_end];
        if is_different_lane_line(line, scope_lane.as_deref()) {
            return line_start;
        }
        let line_is_unrelated_metadata = is_unrelated_metadata_line(line);
        let line_extends_capture = if is_handoff_metadata_line(line) {
            true
        } else {
            is_capture_related(line)
                && (!line_is_unrelated_metadata || is_handler_capture_line(line))
        };
        if line.trim().is_empty()
            || saw_capture && !line_extends_capture && line_is_unrelated_metadata
        {
            return line_start;
        }
        saw_capture |= line_extends_capture;
        cursor = line_end;
    }
    evidence.len()
}
fn line_end(text: &str, line_start: usize) -> usize {
    text[line_start..]
        .find('\n')
        .map_or(text.len(), |index| line_start + index)
}
fn is_capture_related(line: &str) -> bool {
    [
        "dogfooding defect",
        "tool-exposure defect",
        "dogfooding/tool-exposure defect",
        "handler",
        "missing-handler",
        "no handler registered",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}
fn is_unrelated_metadata_line(line: &str) -> bool {
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    !is_capture_related(&key.to_ascii_lowercase())
}

pub(super) fn is_handoff_metadata_line(line: &str) -> bool {
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    matches!(
        key.to_ascii_lowercase().trim(),
        "fallback route"
            | "fallback route used"
            | "fallback path"
            | "tracking issue"
            | "tracked in issue"
            | "tracked by issue"
            | "separate tracking issue"
            | "separate dogfood issue"
            | "separate dogfooding issue"
            | "follow-up issue"
    )
}

pub(super) fn preceding_handoff_metadata_start(evidence: &str, line_start: usize) -> usize {
    let mut capture_start = line_start;
    let mut cursor = line_start;
    let current_lane = lane_label(&evidence[line_start..line_end(evidence, line_start)]);
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous_line = &evidence[previous_start..previous_end];
        if is_different_lane_line(previous_line, current_lane.as_deref()) {
            break;
        }
        if is_handoff_metadata_line(previous_line)
            && lane_label_for_scope(evidence, 0, previous_start)
                .is_some_and(|lane| Some(lane.as_str()) != current_lane.as_deref())
        {
            break;
        }
        if !is_handoff_metadata_line(previous_line) || has_absent_defect_capture(previous_line) {
            break;
        }
        capture_start = previous_start;
        cursor = previous_start;
    }
    capture_start
}

pub(super) fn following_handoff_metadata_has(
    evidence: &str,
    line_start: usize,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    let mut cursor = line_end(evidence, line_start);
    let current_lane = lane_label_for_scope(evidence, line_start, cursor);
    while cursor < evidence.len() {
        let next_start = cursor + 1;
        let next_end = line_end(evidence, next_start);
        let line = &evidence[next_start..next_end];
        if line.trim().is_empty() {
            return false;
        }
        if is_different_lane_line(line, current_lane.as_deref()) {
            return false;
        }
        if predicate(line) {
            return true;
        }
        if !is_handoff_metadata_line(line) {
            return false;
        }
        cursor = next_end;
    }
    false
}

pub(super) fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ") || trimmed.starts_with("* ")
}

fn line_key_value(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim_start();
    let trimmed = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .unwrap_or(trimmed);
    let (key, value) = trimmed.split_once(':')?;
    let key = strip_lane_label_prefix(key);
    if key.trim().is_empty() {
        return value.trim_start().split_once(':');
    }
    Some((key, value))
}

fn strip_lane_label_prefix(key: &str) -> &str {
    let Some(rest) = key
        .trim_start()
        .strip_prefix("Lane ")
        .or_else(|| key.trim_start().strip_prefix("lane "))
    else {
        return key;
    };
    let label_end = rest
        .find(|ch: char| ch.is_whitespace() || ch == '-' || ch == '.')
        .unwrap_or(rest.len());
    let label = rest[..label_end].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    if label.is_empty() {
        return key;
    }
    rest[label_end..].trim_start_matches(|ch: char| ch.is_whitespace() || ch == '-' || ch == '.')
}

fn lane_label_for_scope(evidence: &str, start: usize, end: usize) -> Option<String> {
    evidence[start..end].lines().filter_map(lane_label).last()
}

fn is_different_lane_line(line: &str, current_lane: Option<&str>) -> bool {
    let Some(next_lane) = lane_label(line) else {
        return false;
    };
    current_lane.is_none_or(|current_lane| next_lane != current_lane)
}

fn lane_label(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let trimmed = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
        .unwrap_or(trimmed)
        .trim_start();
    let rest = trimmed
        .strip_prefix("Lane ")
        .or_else(|| trimmed.strip_prefix("lane "))?;
    let label = rest
        .split(|ch: char| ch.is_whitespace() || ch == ':' || ch == '-' || ch == '.')
        .next()
        .unwrap_or_default()
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
    (!label.is_empty()).then(|| format!("lane {}", label.to_ascii_lowercase()))
}

fn is_affirmative_capture_line(line: &str) -> bool {
    [
        "captured",
        "classified",
        "recorded",
        "reported",
        "routed",
        "tracked",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}

fn is_handler_capture_line(line: &str) -> bool {
    is_affirmative_capture_line(line)
        && ["handler", "missing-handler", "no handler registered"]
            .into_iter()
            .any(|marker| line.contains(marker))
}
