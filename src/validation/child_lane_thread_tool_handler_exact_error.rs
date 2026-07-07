const HANDLER_MISSING_MARKER: &str = "no handler registered for tool:";
const THREAD_TOOL_NAMES: &str = "create_thread|fork_thread|list_projects|list_threads|read_thread|send_message_to_thread|set_thread_title";
const NEGATED_HANDLER_MISSING_MARKERS: &str = "did not fail with|didn't fail with|does not fail with|do not fail with|not fail with|without failing with|did not produce|does not produce|no invocation produced|no thread tool invocation produced";

pub(super) fn placeholder_tools_have_exact_errors(
    placeholder_scope: &str,
    capture_scope: &str,
) -> bool {
    THREAD_TOOL_NAMES
        .split('|')
        .filter(|tool| has_tool_name(placeholder_scope, tool))
        .all(|tool| has_exact_thread_tool_handler_error(capture_scope, tool))
}

fn has_exact_thread_tool_handler_error(evidence: &str, tool: &str) -> bool {
    evidence
        .match_indices(HANDLER_MISSING_MARKER)
        .any(|(start, _)| {
            let (line, line_start) = line_containing(evidence, start);
            let line_offset = start - line_start;
            handler_missing_tool(line, line_offset).is_some_and(|exact_tool| exact_tool == tool)
                && !has_negated_handler_missing_claim(line, line_offset)
        })
}

fn handler_missing_tool(line: &str, start: usize) -> Option<&'static str> {
    let tool = handler_tool_fragment(line, start)
        .strip_prefix("codex_app.")
        .unwrap_or_else(|| handler_tool_fragment(line, start))
        .trim_end_matches('.');

    THREAD_TOOL_NAMES
        .split('|')
        .find(|thread_tool| *thread_tool == tool)
}

fn line_containing(text: &str, offset: usize) -> (&str, usize) {
    let line_start = text[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index);
    (&text[line_start..line_end], line_start)
}

fn handler_tool_fragment(line: &str, start: usize) -> &str {
    line[start + HANDLER_MISSING_MARKER.len()..]
        .trim_start_matches([' ', '`', '\'', '"'])
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'))
        .next()
        .unwrap_or_default()
}

fn has_tool_name(line: &str, tool: &str) -> bool {
    line.contains(tool) || line.contains(&format!("codex_app.{tool}"))
}

fn has_negated_handler_missing_claim(line: &str, start: usize) -> bool {
    let prefix = &line[..start];
    let prefix_start = prefix.rfind([';', '.', ',']).map_or(0, |offset| offset + 1);
    let prefix_start = prefix_start.max(prefix.rfind(" but ").map_or(0, |offset| offset + 5));
    let prefix = &prefix[prefix_start..];
    NEGATED_HANDLER_MISSING_MARKERS.split('|').any(|marker| {
        prefix.match_indices(marker).any(|(offset, _)| {
            prefix[offset + marker.len()..]
                .trim_matches([' ', '`', '\'', '"'])
                .is_empty()
        })
    })
}
