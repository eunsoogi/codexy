use super::child_lane_thread_tool_handler_capture::has_absent_defect_capture;
use super::child_lane_thread_tool_handler_defect_capture::{
    has_handler_marker_and_tool_name_in_defect_capture, has_handler_marker_in_defect_capture,
};
use super::child_lane_thread_tool_handler_scope::{
    capture_end_before_unrelated_evidence, previous_nonempty_block_start, scope_start_until_blank,
};

pub(super) fn has_uncaptured_defect(evidence: &str) -> bool {
    if !has_discovered_or_expected_thread_tool(evidence) {
        return false;
    }

    evidence
        .match_indices(HANDLER_MISSING_MARKER)
        .any(|(start, _)| {
            let (line, line_start) = line_containing(evidence, start);
            let line_offset = start - line_start;
            let capture_scope = handler_missing_capture_scope(evidence, start);
            if let Some(tool) = handler_missing_tool(line, line_offset) {
                return !has_negated_handler_missing_claim(line, line_offset)
                    && !has_actionable_handler_defect_report(capture_scope, tool);
            }

            let placeholder_scope = handler_missing_placeholder_scope(evidence, line_start);
            handler_missing_placeholder(line, line_offset)
                && has_thread_tool_name(placeholder_scope)
                && !has_negated_handler_missing_claim(line, line_offset)
                && !has_actionable_handler_placeholder_report(capture_scope)
        })
}

const HANDLER_MISSING_MARKER: &str = "no handler registered for tool:";
const CAPTURE_MARKERS: &str = "captured|classified|recorded|reported|routed|tracked";
const THREAD_TOOL_DISCOVERY_MARKERS: &str = "available|callable|discovered|expected|exposed|found|listed|registered|tool_search|tool search|visible";
const THREAD_TOOL_NAMES: &str = "create_thread|fork_thread|list_projects|list_threads|read_thread|send_message_to_thread|set_thread_title";

fn has_discovered_or_expected_thread_tool(evidence: &str) -> bool {
    let mut in_discovery_list = false;
    evidence.lines().any(|line| {
        let trimmed = line.trim_start();
        let normalized = line.to_ascii_lowercase();
        let has_marker = THREAD_TOOL_DISCOVERY_MARKERS
            .split('|')
            .any(|marker| normalized.contains(marker));
        let discovered = has_thread_tool_name(line)
            && (has_marker || in_discovery_list && is_list_item(trimmed));
        in_discovery_list = if has_marker && !has_thread_tool_name(line) {
            true
        } else {
            in_discovery_list && is_list_item(trimmed)
        };
        discovered
    })
}

fn has_actionable_handler_defect_report(evidence: &str, tool: &str) -> bool {
    has_defect_label(evidence)
        && has_handler_marker_and_tool_name_in_defect_capture(evidence, tool)
        && has_affirmative_defect_capture(evidence)
        && !has_absent_defect_capture(evidence)
}

fn has_actionable_handler_placeholder_report(evidence: &str) -> bool {
    has_handler_marker_in_defect_capture(evidence) && !has_absent_defect_capture(evidence)
}

fn has_defect_label(line: &str) -> bool {
    line.contains("dogfooding defect")
        || line.contains("tool-exposure defect")
        || line.contains("dogfooding/tool-exposure defect")
}

fn has_negated_handler_missing_claim(line: &str, start: usize) -> bool {
    let prefix_start = line[..start]
        .rfind([';', '.'])
        .map_or(0, |offset| offset + 1);
    let prefix = &line[prefix_start..start];
    [
        "did not fail with",
        "didn't fail with",
        "does not fail with",
        "do not fail with",
        "not fail with",
        "without failing with",
        "did not produce",
        "does not produce",
        "no invocation produced",
        "no thread tool invocation produced",
    ]
    .into_iter()
    .any(|marker| prefix.contains(marker))
}

fn handler_missing_tool(line: &str, start: usize) -> Option<&'static str> {
    let tool = handler_tool_fragment(line, start)
        .strip_prefix("codex_app.")
        .unwrap_or_else(|| handler_tool_fragment(line, start))
        .trim_end_matches('.');

    thread_tool_names()
        .into_iter()
        .find(|thread_tool| *thread_tool == tool)
}

fn line_containing(text: &str, offset: usize) -> (&str, usize) {
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index);
    (&text[line_start..line_end], line_start)
}

fn handler_missing_capture_scope(evidence: &str, start: usize) -> &str {
    let (_, line_start) = line_containing(evidence, start);
    let capture_start = multiline_capture_start(evidence, line_start);
    let next_start = evidence[start + HANDLER_MISSING_MARKER.len()..]
        .match_indices(HANDLER_MISSING_MARKER)
        .map(|(offset, _)| start + HANDLER_MISSING_MARKER.len() + offset)
        .find(|next| {
            evidence[start..*next].contains('\n')
                && !same_handler_list_group(evidence, line_start, *next)
        })
        .unwrap_or_else(|| capture_end_before_unrelated_evidence(evidence, capture_start, start));
    &evidence[capture_start..next_start]
}

fn same_handler_list_group(evidence: &str, line_start: usize, next: usize) -> bool {
    let (next_line, next_line_start) = line_containing(evidence, next);
    if !evidence[line_start..next_line_start]
        .lines()
        .all(|line| line.trim().is_empty() || is_handler_missing_list_item(line))
    {
        return false;
    }
    is_handler_missing_list_item(&evidence[line_start..line_end(evidence, line_start)])
        && is_handler_missing_list_item(next_line)
}

fn line_end(text: &str, line_start: usize) -> usize {
    text[line_start..]
        .find('\n')
        .map_or(text.len(), |index| line_start + index)
}

fn handler_missing_placeholder_scope(evidence: &str, line_start: usize) -> &str {
    let current_line_end = line_end(evidence, line_start);
    let (mut previous_start, blank_start) = scope_start_until_blank(evidence, line_start);
    let discovery_start = blank_start
        .and_then(|blank_start| previous_nonempty_block_start(evidence, blank_start))
        .filter(|start| has_discovered_or_expected_thread_tool(&evidence[*start..line_start]));
    if !has_thread_tool_name(&evidence[previous_start..current_line_end]) {
        if let Some(discovery_start) = discovery_start {
            previous_start = discovery_start;
        }
    }
    &evidence[previous_start..current_line_end]
}

fn multiline_capture_start(evidence: &str, line_start: usize) -> usize {
    let current_line_end = line_end(evidence, line_start);
    let current_trimmed = evidence[line_start..current_line_end].trim_start();
    if !is_list_item(current_trimmed) {
        if has_defect_label(current_trimmed) {
            return line_start;
        }
        let previous_end = line_start.saturating_sub(1);
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous_line = &evidence[previous_start..previous_end];
        return if has_defect_label(previous_line) {
            previous_start
        } else {
            line_start
        };
    }

    let mut capture_start = line_start;
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let previous_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        let previous_line = &evidence[previous_start..previous_end];
        let trimmed = previous_line.trim_start();
        if is_list_item(trimmed) || has_defect_label(previous_line) {
            capture_start = previous_start;
            cursor = previous_start;
        } else {
            break;
        }
    }
    capture_start
}

fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ") || trimmed.starts_with("* ")
}

fn is_handler_missing_list_item(line: &str) -> bool {
    is_list_item(line) && line.contains(HANDLER_MISSING_MARKER)
}

fn handler_tool_fragment(line: &str, start: usize) -> &str {
    line[start + HANDLER_MISSING_MARKER.len()..]
        .trim_start_matches([' ', '`', '\'', '"'])
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'))
        .next()
        .unwrap_or_default()
}

fn handler_missing_placeholder(line: &str, start: usize) -> bool {
    let fragment =
        line[start + HANDLER_MISSING_MARKER.len()..].trim_start_matches([' ', '`', '\'', '"']);
    fragment.starts_with("...")
        || fragment.starts_with('…')
        || handler_tool_fragment(line, start).is_empty()
}

fn has_affirmative_defect_capture(line: &str) -> bool {
    CAPTURE_MARKERS.split('|').any(|marker| {
        line.match_indices(marker)
            .any(|(start, _)| !is_fallback_negation_marker(line, start, marker))
    })
}

fn is_fallback_negation_marker(line: &str, start: usize, marker: &str) -> bool {
    line[..start].ends_with("was not ")
        && "as an ordinary unavailable-tool fallback|as a normal fallback|as an unavailable-tool fallback"
            .split('|')
            .any(|suffix| line[start + marker.len()..].contains(suffix))
}

fn has_thread_tool_name(line: &str) -> bool {
    thread_tool_names()
        .into_iter()
        .any(|tool| has_tool_name(line, tool))
}

fn has_tool_name(line: &str, tool: &str) -> bool {
    line.contains(tool) || line.contains(&format!("codex_app.{tool}"))
}

fn thread_tool_names() -> impl Iterator<Item = &'static str> {
    THREAD_TOOL_NAMES.split('|')
}
