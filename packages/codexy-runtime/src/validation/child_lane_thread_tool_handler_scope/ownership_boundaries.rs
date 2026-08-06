use super::super::child_lane_thread_tool_handler_capture::has_absent_defect_capture;
use super::super::child_lane_thread_tool_handler_scope_labels::{
    line_key_value, strip_list_prefix,
};
use super::lane_metadata::{
    handoff_metadata_lane, is_different_lane_line, lane_label, lane_label_for_current_scope,
};

pub(crate) fn scope_start_until_blank(evidence: &str, line_start: usize) -> (usize, Option<usize>) {
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

pub(crate) fn previous_nonempty_block_start(evidence: &str, block_end: usize) -> Option<usize> {
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

pub(crate) fn line_end(text: &str, line_start: usize) -> usize {
    text[line_start..]
        .find('\n')
        .map_or(text.len(), |index| line_start + index)
}

pub(crate) fn is_handoff_metadata_line(line: &str) -> bool {
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    matches!(
        key.to_ascii_lowercase().trim(),
        "fallback route"
            | "fallback-route"
            | "fallback route used"
            | "fallback path"
            | "fallback-path"
            | "no fallback route"
            | "no fallback-route"
            | "no fallback path"
            | "no fallback-path"
            | "tracking issue"
            | "tracked in issue"
            | "tracked by issue"
            | "separate tracking issue"
            | "separate dogfood issue"
            | "separate dogfooding issue"
            | "follow-up issue"
    )
}

pub(crate) fn is_lane_scoped_defect_preface_metadata_line(line: &str) -> bool {
    if is_handoff_metadata_line(line) {
        return true;
    }
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    matches!(
        key.to_ascii_lowercase().trim(),
        "pending worktree"
            | "pending worktree id"
            | "pending worktree ids"
            | "child thread"
            | "child thread id"
            | "child thread ids"
            | "thread"
            | "thread id"
            | "thread ids"
    )
}

pub(crate) fn preceding_handoff_metadata_start(evidence: &str, line_start: usize) -> usize {
    let mut capture_start = line_start;
    let mut cursor = line_start;
    let evidence_line = &evidence[line_start..line_end(evidence, line_start)];
    let current_lane = lane_label(evidence_line)
        .or_else(|| lane_label_for_current_scope(evidence, line_start, line_start));
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
            && handoff_metadata_lane(evidence, previous_start, previous_line)
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
    if capture_start != line_start && previous_line_is_defect_label(evidence, capture_start) {
        return line_start;
    }
    capture_start
}

fn previous_line_is_defect_label(evidence: &str, line_start: usize) -> bool {
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous_line = &evidence[previous_start..previous_end];
        if is_exact_handler_error_metadata_line(previous_line) {
            cursor = previous_start;
            continue;
        }
        return !has_absent_defect_capture(previous_line)
            && [
                "dogfooding defect",
                "tool-exposure defect",
                "dogfooding/tool-exposure defect",
            ]
            .into_iter()
            .any(|marker| previous_line.contains(marker));
    }
    false
}

fn is_exact_handler_error_metadata_line(line: &str) -> bool {
    let Some((key, value)) = line_key_value(line) else {
        return false;
    };
    key.trim()
        .eq_ignore_ascii_case("exact missing-handler error")
        && value
            .to_ascii_lowercase()
            .contains("no handler registered for tool:")
}

pub(crate) fn following_handoff_metadata_has(
    evidence: &str,
    line_start: usize,
    predicate: impl Fn(&str) -> bool,
) -> bool {
    let mut cursor = line_end(evidence, line_start);
    let current_lane = lane_label_for_current_scope(evidence, line_start, cursor);
    while cursor < evidence.len() {
        let next_start = cursor + 1;
        let next_end = line_end(evidence, next_start);
        let line = &evidence[next_start..next_end];
        if line.trim().is_empty() || is_different_lane_line(line, current_lane.as_deref()) {
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

pub(crate) fn is_list_item(line: &str) -> bool {
    strip_list_prefix(line) != line.trim_start()
}
