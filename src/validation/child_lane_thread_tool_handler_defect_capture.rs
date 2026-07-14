use super::child_lane_thread_tool_handler_issue_tracking::has_tracking_issue;
use super::child_lane_thread_tool_handler_issue_value::has_placeholder_or_pending_value;
use super::child_lane_thread_tool_handler_no_route::has_false_no_route_answer;
use super::child_lane_thread_tool_handler_route_value::has_substantive_route_value;

mod candidate_scopes;
mod capture_markers;
mod fallback_routes;
mod lane_scope_filters;
mod lane_scope_tokens;
use super::child_lane_thread_tool_handler_lane_mentions::has_unnegated_different_lane_phrase;
use super::child_lane_thread_tool_handler_scope::has_different_lane_mention;
use candidate_scopes::*;
pub(super) use capture_markers::has_negated_fallback_route_field;
use capture_markers::*;
use fallback_routes::*;

pub(super) fn has_handler_marker_and_tool_name_in_defect_capture(
    evidence: &str,
    tool: &str,
) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        is_defect_capture_line(line)
            && !has_different_lane_mention(line)
            && !has_unnegated_different_lane_phrase(line)
            && !has_negated_fallback_route_field(line)
            && (has_handler_marker_and_tool_name_in_defect_clause(line, tool)
                && has_handler_handoff_fields(&defect_candidate_scope(&lines, index))
                || opens_defect_list(line)
                    && lines[index + 1..]
                        .iter()
                        .take_while(|following| is_list_item(following))
                        .enumerate()
                        .any(|following| {
                            let (offset, following) = following;
                            !has_negated_fallback_route_field(following)
                                && has_handler_marker(following)
                                && has_tool_name(following, tool)
                                && has_handler_handoff_fields(&list_item_candidate_scope(
                                    &lines,
                                    index,
                                    &lines[index + 1..],
                                    offset,
                                ))
                        }))
    })
}
pub(super) fn has_handler_marker_in_defect_capture(evidence: &str) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        is_defect_capture_line(line)
            && !has_different_lane_mention(line)
            && !has_unnegated_different_lane_phrase(line)
            && !has_negated_fallback_route_field(line)
            && (has_handler_marker_in_defect_clause(line)
                && has_handler_handoff_fields(&defect_candidate_scope(&lines, index))
                || opens_defect_list(line)
                    && lines[index + 1..]
                        .iter()
                        .take_while(|following| is_list_item(following))
                        .enumerate()
                        .any(|following| {
                            let (offset, following) = following;
                            !has_negated_fallback_route_field(following)
                                && has_handler_marker(following)
                                && has_handler_handoff_fields(&list_item_candidate_scope(
                                    &lines,
                                    index,
                                    &lines[index + 1..],
                                    offset,
                                ))
                        }))
    })
}
