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
    while cursor < evidence.len() {
        let line_start = cursor + 1;
        let line_end = line_end(evidence, line_start);
        let line = &evidence[line_start..line_end];
        let line_is_unrelated_metadata = is_unrelated_metadata_line(line);
        let line_extends_capture = is_capture_related(line)
            && (!line_is_unrelated_metadata || is_handler_capture_line(line));
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
        "fallback route",
        "fallback-route",
        "fallback path",
        "fallback-path",
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
