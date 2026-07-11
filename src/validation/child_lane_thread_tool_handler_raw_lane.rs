use std::ops::Range;

use super::child_lane_thread_tool_handler_scope::{
    capture_end_before_unrelated_evidence, has_different_lane_mention,
};
use super::child_lane_thread_tool_handlers::{
    HANDLER_MISSING_MARKER, line_containing, multiline_capture_start, same_defect_list_report,
    same_handler_list_group,
};

pub(super) fn handler_missing_capture_range(evidence: &str, start: usize) -> Range<usize> {
    let (_, line_start) = line_containing(evidence, start);
    let capture_start = multiline_capture_start(evidence, line_start);
    let next_start = evidence[start + HANDLER_MISSING_MARKER.len()..]
        .match_indices(HANDLER_MISSING_MARKER)
        .map(|(offset, _)| start + HANDLER_MISSING_MARKER.len() + offset)
        .find(|next| {
            evidence[start..*next].contains('\n')
                && !same_handler_list_group(evidence, line_start, *next)
                && !same_defect_list_report(evidence, line_start, *next)
                && !line_containing(evidence, *next).0.contains("exact")
        })
        .unwrap_or_else(|| capture_end_before_unrelated_evidence(evidence, capture_start, start));
    capture_start..next_start
}

pub(super) fn has_different_lane_defect_capture(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        let normalized = line.to_ascii_lowercase();
        (normalized.contains("dogfooding defect")
            || normalized.contains("tool-exposure defect")
            || normalized.contains("dogfooding/tool-exposure defect"))
            && has_different_lane_mention(line)
    })
}
