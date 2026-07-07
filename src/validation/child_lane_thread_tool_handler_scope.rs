pub(super) fn scope_start_until_blank(evidence: &str, line_start: usize) -> (usize, Option<usize>) {
    let mut previous_start = line_start;
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end].rfind('\n').map_or(0, |i| i + 1);
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
        let candidate_start = evidence[..previous_end].rfind('\n').map_or(0, |i| i + 1);
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
    let mut saw_capture = is_capture_related(&evidence[capture_start..cursor]);
    let mut saw_handler_capture = is_handler_capture_line(&evidence[capture_start..cursor]);
    let current_lane = current_lane_before(evidence, handler_start);
    while cursor < evidence.len() {
        let line_start = cursor + 1;
        let line_end = line_end(evidence, line_start);
        let line = &evidence[line_start..line_end];
        let line_is_unrelated_metadata = is_unrelated_metadata_line(line);
        let line_is_same_lane_header_metadata =
            is_same_lane_header_metadata_line(evidence, line_start, line_end, handler_start);
        let line_is_handler_capture =
            is_handler_capture_line(line) && !line_is_same_lane_header_metadata;
        let line_targets_other_lane = metadata_targets_other_lane(line, current_lane.as_deref());
        let line_extends_capture = is_capture_related(line)
            && !line_targets_other_lane
            && (!line_is_unrelated_metadata || line_is_handler_capture);
        if line.trim().is_empty()
            || saw_handler_capture && line_is_same_lane_header_metadata
            || saw_capture
                && !line_extends_capture
                && (line_is_unrelated_metadata || line_targets_other_lane)
                && !line_is_same_lane_header_metadata
        {
            return line_start;
        }
        saw_capture |= line_extends_capture;
        saw_handler_capture |= line_is_handler_capture;
        cursor = line_end;
    }
    evidence.len()
}

fn line_end(s: &str, i: usize) -> usize {
    s[i..].find('\n').map_or(s.len(), |n| i + n)
}
fn is_capture_related(line: &str) -> bool {
    "dogfooding defect|tool-exposure defect|dogfooding/tool-exposure defect|handler|missing-handler|no handler registered|fallback route|fallback-route|fallback path|fallback-path"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn is_unrelated_metadata_line(line: &str) -> bool {
    line.trim_start()
        .split_once(':')
        .is_some_and(|(key, _)| !is_capture_related(&key.to_ascii_lowercase()))
}
fn is_same_lane_header_metadata_line(
    evidence: &str,
    line_start: usize,
    line_end: usize,
    handler_start: usize,
) -> bool {
    let line = &evidence[line_start..line_end];
    let Some((key, _)) = line.trim_start().split_once(':') else {
        return false;
    };
    let key = metadata_key(key);
    let current_lane = current_lane_before(evidence, handler_start);
    if metadata_targets_other_lane(line, current_lane.as_deref()) {
        return false;
    }
    let block_has_same_lane_marker =
        same_lane_header_block_has_same_lane_marker(evidence, line_end, current_lane.as_deref());
    let generic_same_lane_metadata = matches!(
        key.as_str(),
        "issue" | "pr" | "tracking issue" | "branch" | "head" | "worktree path"
    ) && (current_lane.is_none()
        || block_has_same_lane_marker
        || !matches!(key.as_str(), "issue" | "branch"));
    matches!(
        key.as_str(),
        "fallback route" | "fallback path" | "owner" | "lane owner" | "child owner"
    ) && !line.to_ascii_lowercase().contains("not used")
        || generic_same_lane_metadata
        || key.starts_with("lane ")
            && current_lane
                .as_ref()
                .is_some_and(|current_lane| current_lane == &key)
            && next_nonempty_line(evidence, line_end)
                .is_some_and(|next| is_same_lane_header_field(next))
            && block_has_same_lane_marker
}

fn metadata_targets_other_lane(line: &str, current_lane: Option<&str>) -> bool {
    let normalized = line.to_ascii_lowercase();
    let mentioned_other_lane = normalized.match_indices("lane ").any(|(start, _)| {
        let mentioned_lane = mentioned_lane_key(&normalized[start..]);
        !mentioned_lane.is_empty()
            && current_lane.is_none_or(|current_lane| mentioned_lane != current_lane)
            && !normalized[..start].trim_end().ends_with("same")
    });
    if mentioned_other_lane {
        return true;
    }
    if normalized.contains("same lane") {
        return false;
    }
    if normalized.contains("another lane")
        || normalized.contains("different lane")
        || normalized.contains("other lane")
        || normalized.contains("later lane")
    {
        return true;
    }
    false
}
fn mentioned_lane_key(mention: &str) -> &str {
    let mention = mention.trim_start();
    let Some(rest) = mention.strip_prefix("lane ") else {
        return "";
    };
    let n = rest
        .find(|ch: char| !ch.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    (n != 0)
        .then(|| &mention[.."lane ".len() + n])
        .unwrap_or_default()
}
fn current_lane_before(evidence: &str, handler_start: usize) -> Option<String> {
    let line_start = evidence[..handler_start].rfind('\n').map_or(0, |i| i + 1);
    current_lane_header_before(evidence, line_start).or_else(|| {
        let line = evidence[line_start..line_end(evidence, handler_start)].to_ascii_lowercase();
        let key = mentioned_lane_key(&line);
        is_lane_header_key(key).then(|| key.to_string())
    })
}
fn current_lane_header_before(evidence: &str, mut cursor: usize) -> Option<String> {
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = evidence[..previous_end].rfind('\n').map_or(0, |i| i + 1);
        let line = evidence[previous_start..previous_end].trim_start();
        if line.trim().is_empty() {
            return None;
        }
        if let Some((key, _)) = line.split_once(':') {
            let key = metadata_key(key);
            if is_lane_header_key(&key) {
                return Some(key);
            }
        }
        cursor = previous_start;
    }
    None
}
fn is_lane_header_key(key: &str) -> bool {
    let second_word = key.split_whitespace().nth(1).unwrap_or("");
    key.starts_with("lane ")
        && mentioned_lane_key(key) == key
        && !["owner", "ownership", "metadata"].contains(&second_word)
}

fn next_nonempty_line(evidence: &str, mut cursor: usize) -> Option<&str> {
    while cursor < evidence.len() {
        let start = cursor + usize::from(evidence.as_bytes()[cursor] == b'\n');
        let end = line_end(evidence, start);
        let line = &evidence[start..end];
        if !line.trim().is_empty() {
            return Some(line);
        }
        cursor = end;
    }
    None
}

fn is_same_lane_header_field(line: &str) -> bool {
    line.trim_start().split_once(':').is_some_and(|(key, _)| {
        ["fallback route", "fallback path", "tracking issue"].contains(&metadata_key(key).as_str())
    })
}

fn same_lane_header_block_has_same_lane_marker(
    evidence: &str,
    mut cursor: usize,
    current_lane: Option<&str>,
) -> bool {
    while cursor < evidence.len() {
        let start = cursor + usize::from(evidence.as_bytes()[cursor] == b'\n');
        let end = line_end(evidence, start);
        let line = &evidence[start..end];
        if line.to_ascii_lowercase().contains("same lane")
            || current_lane.is_some_and(|current_lane| {
                let normalized = line.to_ascii_lowercase();
                normalized
                    .match_indices("lane ")
                    .any(|(start, _)| mentioned_lane_key(&normalized[start..]) == current_lane)
            })
        {
            return true;
        }
        if line.trim().is_empty() || is_capture_related(line) {
            return false;
        }
        cursor = end;
    }
    false
}

fn metadata_key(key: &str) -> String {
    key.trim_matches([' ', '-', '*']).to_ascii_lowercase()
}

fn is_affirmative_capture_line(line: &str) -> bool {
    "captured|classified|recorded|reported|routed|tracked"
        .split('|')
        .any(|marker| line.contains(marker))
}

fn is_handler_capture_line(line: &str) -> bool {
    is_affirmative_capture_line(line)
        && "handler|missing-handler|no handler registered"
            .split('|')
            .any(|marker| line.contains(marker))
}
