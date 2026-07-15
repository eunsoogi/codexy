use super::super::child_lane_thread_tool_handler_capture::has_absent_defect_capture;
use super::super::child_lane_thread_tool_handler_scope_labels::{
    is_handler_capture_line, line_key_value,
};
use super::lane_metadata::{
    has_different_lane_mention, is_different_lane_line, lane_label_for_current_scope,
    lane_label_for_scope, lane_mention_labels, line_mentions_different_lane,
};
use super::ownership_boundaries::{
    is_handoff_metadata_line, is_lane_scoped_defect_preface_metadata_line, is_list_item, line_end,
};

pub(crate) fn capture_end_before_unrelated_evidence(
    evidence: &str,
    capture_start: usize,
    handler_start: usize,
) -> usize {
    let mut cursor = line_end(evidence, handler_start);
    let mut scope_lane = lane_label_for_scope(evidence, capture_start, cursor)
        .or_else(|| lane_label_for_current_scope(evidence, handler_start, cursor));
    let mut saw_capture = is_capture_related(&evidence[capture_start..cursor]);
    let mut saw_handler_defect_capture =
        is_handler_defect_capture_line(&evidence[capture_start..cursor]);
    let mut pending_defect_capture = has_open_defect_capture(&evidence[capture_start..cursor]);
    let mut saw_unscoped_handoff_metadata = false;
    while cursor < evidence.len() {
        let line_start = cursor + 1;
        let line_end = line_end(evidence, line_start);
        let line = &evidence[line_start..line_end];
        let line_opens_defect_capture = has_open_defect_capture(line);
        let line_is_unscoped_lane_prefixed_defect_capture = scope_lane.is_none()
            && !saw_unscoped_handoff_metadata
            && is_handler_defect_capture_line(line);
        if is_different_lane_line(line, scope_lane.as_deref())
            && !line_is_unscoped_lane_prefixed_defect_capture
        {
            return line_start;
        }
        let line_matches_upcoming_defect_lane = scope_lane.is_none()
            && is_lane_scoped_defect_preface_metadata_line(line)
            && metadata_line_matches_upcoming_defect_lane(evidence, line_end, line);
        let line_is_unscoped_defect_without_different_lane = scope_lane.is_none()
            && line_opens_defect_capture
            && !super::super::child_lane_thread_tool_handler_lane_mentions::has_unnegated_different_lane_phrase(line)
            && !has_different_lane_mention(line);
        let line_is_pending_unscoped_capture_without_different_lane = scope_lane.is_none()
            && pending_defect_capture
            && is_handler_capture_line(line)
            && !has_absent_defect_capture(line)
            && !super::super::child_lane_thread_tool_handler_lane_mentions::has_unnegated_different_lane_phrase(line)
            && !has_different_lane_mention(line);
        let line_names_different_lane = line_mentions_different_lane(line, scope_lane.as_deref())
            && !line_is_unscoped_defect_without_different_lane
            && !line_is_pending_unscoped_capture_without_different_lane
            && !line_matches_upcoming_defect_lane;
        if line_names_different_lane {
            return line_start;
        }
        if scope_lane.is_none()
            && is_handler_defect_capture_line(line)
            && !has_different_lane_mention(line)
        {
            scope_lane = lane_mention_labels(line).into_iter().next();
        }
        let line_is_unrelated_metadata = is_unrelated_metadata_line(line)
            && (saw_handler_defect_capture
                || !is_excluded_lane_metadata_line(line)
                || line_names_different_lane);
        let line_has_handler_defect_capture = is_handler_defect_capture_line(line)
            || pending_defect_capture
                && is_handler_capture_line(line)
                && !has_absent_defect_capture(line);
        let line_extends_capture =
            if is_handoff_metadata_line(line) || line_matches_upcoming_defect_lane {
                !line_names_different_lane
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
        saw_handler_defect_capture |= line_has_handler_defect_capture;
        pending_defect_capture |= line_opens_defect_capture;
        saw_unscoped_handoff_metadata |= is_handoff_metadata_line(line)
            && lane_mention_labels(line).is_empty()
            && !line_matches_upcoming_defect_lane;
        cursor = line_end;
    }
    evidence.len()
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
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    !is_capture_related(&key.to_ascii_lowercase())
}

fn is_excluded_lane_metadata_line(line: &str) -> bool {
    let Some((key, _)) = line_key_value(line) else {
        return false;
    };
    let key = key.trim_start().to_ascii_lowercase();
    [
        "lane owner",
        "lane owners",
        "lane ownership",
        "lane metadata",
    ]
    .iter()
    .any(|prefix| key.starts_with(prefix))
}

fn metadata_line_matches_upcoming_defect_lane(
    evidence: &str,
    metadata_line_end: usize,
    line: &str,
) -> bool {
    let lanes = lane_mention_labels(line);
    let Some(metadata_lane) = lanes.first() else {
        return false;
    };
    if lanes.iter().skip(1).any(|lane| lane != metadata_lane) {
        return false;
    }
    let mut cursor = metadata_line_end;
    while cursor < evidence.len() {
        let next_start = cursor + 1;
        let next_end = line_end(evidence, next_start);
        let next_line = &evidence[next_start..next_end];
        if next_line.trim().is_empty() || is_different_lane_line(next_line, Some(metadata_lane)) {
            return false;
        }
        if is_handler_defect_capture_line(next_line) {
            let defect_lanes = lane_mention_labels(next_line);
            return defect_lanes.first() == Some(metadata_lane)
                && !defect_lanes
                    .iter()
                    .skip(1)
                    .any(|lane| lane != metadata_lane);
        }
        if has_open_defect_capture(next_line)
            && list_capture_matches_metadata_lane(evidence, next_end, metadata_lane)
        {
            return true;
        }
        if !is_lane_scoped_defect_preface_metadata_line(next_line) && !is_capture_related(next_line)
        {
            return false;
        }
        cursor = next_end;
    }
    false
}

fn list_capture_matches_metadata_lane(
    evidence: &str,
    defect_header_end: usize,
    metadata_lane: &str,
) -> bool {
    let header_start = evidence[..defect_header_end]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let header_lanes = lane_mention_labels(&evidence[header_start..defect_header_end]);
    let header_matches_metadata_lane = header_lanes.first().is_some_and(|lane| {
        lane == metadata_lane
            && !header_lanes
                .iter()
                .skip(1)
                .any(|lane| lane != metadata_lane)
    });
    let mut cursor = defect_header_end;
    while cursor < evidence.len() {
        let item_start = cursor + 1;
        let item_end = line_end(evidence, item_start);
        let item = &evidence[item_start..item_end];
        if item.trim().is_empty() || !is_list_item(item) {
            return false;
        }
        let item_lanes = lane_mention_labels(item);
        if item_lanes.iter().any(|lane| lane != metadata_lane) {
            return false;
        }
        if item_lanes.first().is_some_and(|lane| lane == metadata_lane)
            && is_handler_capture_line(item)
            && !has_absent_defect_capture(item)
        {
            return true;
        }
        if item_lanes.is_empty()
            && header_matches_metadata_lane
            && is_handler_capture_line(item)
            && !has_absent_defect_capture(item)
        {
            return true;
        }
        cursor = item_end;
    }
    false
}

fn is_handler_defect_capture_line(line: &str) -> bool {
    is_handler_capture_line(line) && has_defect_label(line) && !has_absent_defect_capture(line)
}

fn has_open_defect_capture(line: &str) -> bool {
    has_defect_label(line) && !has_absent_defect_capture(line)
}

fn has_defect_label(line: &str) -> bool {
    line.contains("dogfooding defect")
        || line.contains("tool-exposure defect")
        || line.contains("dogfooding/tool-exposure defect")
}
