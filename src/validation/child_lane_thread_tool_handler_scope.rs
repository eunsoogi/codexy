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
    let mut saw_capture = is_capture_related(&evidence[capture_start..cursor]);
    let mut saw_handler_capture = is_handler_capture_line(&evidence[capture_start..cursor]);
    while cursor < evidence.len() {
        let line_start = cursor + 1;
        let line_end = line_end(evidence, line_start);
        let line = &evidence[line_start..line_end];
        let line_is_unrelated_metadata = is_unrelated_metadata_line(line);
        let line_is_same_lane_header_metadata =
            is_same_lane_header_metadata_line(evidence, line_start, line_end, handler_start);
        let line_is_handler_capture = is_handler_capture_line(line);
        let line_extends_capture =
            is_capture_related(line) && (!line_is_unrelated_metadata || line_is_handler_capture);
        if line.trim().is_empty()
            || saw_handler_capture && line_is_same_lane_header_metadata
            || saw_capture
                && !line_extends_capture
                && line_is_unrelated_metadata
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
    let Some((key, _)) = line.trim_start().split_once(':') else {
        return false;
    };
    !is_capture_related(&key.to_ascii_lowercase())
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
    matches!(
        key.as_str(),
        "issue"
            | "pr"
            | "tracking issue"
            | "branch"
            | "head"
            | "worktree path"
            | "fallback route"
            | "owner"
            | "lane owner"
            | "child owner"
    ) || key.starts_with("lane ")
        && current_lane_header_before(evidence, handler_start)
            .is_some_and(|current_lane| current_lane == key)
        && next_nonempty_line(evidence, line_end)
            .is_some_and(|next| is_same_lane_header_field(next))
        && same_lane_header_block_has_same_lane_marker(evidence, line_end)
}

fn current_lane_header_before(evidence: &str, mut cursor: usize) -> Option<String> {
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let line = evidence[previous_start..previous_end].trim_start();
        if line.trim().is_empty() {
            return None;
        }
        if let Some((key, _)) = line.split_once(':') {
            let key = metadata_key(key);
            if key.starts_with("lane ") {
                return Some(key);
            }
        }
        cursor = previous_start;
    }
    None
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
    let Some((key, _)) = line.trim_start().split_once(':') else {
        return false;
    };
    matches!(
        metadata_key(key).as_str(),
        "fallback route" | "tracking issue"
    )
}

fn same_lane_header_block_has_same_lane_marker(evidence: &str, mut cursor: usize) -> bool {
    while cursor < evidence.len() {
        let start = cursor + usize::from(evidence.as_bytes()[cursor] == b'\n');
        let end = line_end(evidence, start);
        let line = &evidence[start..end];
        if line.trim().is_empty() || is_capture_related(line) {
            return false;
        }
        if line.to_ascii_lowercase().contains("same lane") {
            return true;
        }
        cursor = end;
    }
    false
}

fn metadata_key(key: &str) -> String {
    key.trim_start_matches(['-', '*'])
        .trim_start()
        .trim()
        .to_ascii_lowercase()
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
