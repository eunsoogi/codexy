pub(super) fn has_uncaptured_defect(evidence: &str) -> bool {
    if !has_discovered_or_expected_thread_tool(evidence) {
        return false;
    }

    evidence
        .match_indices(HANDLER_MISSING_MARKER)
        .any(|(start, _)| {
            let (line, line_start) = line_containing(evidence, start);
            let line_offset = start - line_start;
            handler_missing_tool(line, line_offset).is_some_and(|tool| {
                !has_negated_handler_missing_claim(line, line_offset)
                    && !has_actionable_handler_defect_report(
                        handler_missing_capture_scope(evidence, start),
                        tool,
                    )
            })
        })
}

const HANDLER_MISSING_MARKER: &str = "no handler registered for tool:";

fn has_discovered_or_expected_thread_tool(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        has_thread_tool_name(line)
            && [
                "available",
                "callable",
                "discovered",
                "expected",
                "exposed",
                "found",
                "listed",
                "registered",
                "tool_search",
                "tool search",
            ]
            .into_iter()
            .any(|marker| line.contains(marker))
    })
}

fn has_actionable_handler_defect_report(evidence: &str, tool: &str) -> bool {
    evidence.lines().any(|line| {
        has_defect_label(line)
            && [
                "no handler registered",
                "handler registered",
                "handler-missing",
                "missing-handler",
                "missing handler",
            ]
            .into_iter()
            .any(|marker| line.contains(marker))
            && has_tool_name(line, tool)
            && has_affirmative_defect_capture(line)
            && !has_absent_defect_capture(line)
    })
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
        .unwrap_or_else(|| handler_tool_fragment(line, start));

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
    let next_start = evidence[start + HANDLER_MISSING_MARKER.len()..]
        .find(HANDLER_MISSING_MARKER)
        .map_or(evidence.len(), |offset| {
            start + HANDLER_MISSING_MARKER.len() + offset
        });
    &evidence[start..next_start]
}

fn handler_tool_fragment(line: &str, start: usize) -> &str {
    line[start + HANDLER_MISSING_MARKER.len()..]
        .trim_start_matches([' ', '`', '\'', '"'])
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'))
        .next()
        .unwrap_or_default()
}

fn has_affirmative_defect_capture(line: &str) -> bool {
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

fn has_absent_defect_capture(line: &str) -> bool {
    [
        "defect: none",
        "defect none",
        "no dogfooding defect",
        "no tool-exposure defect",
        "not a dogfooding defect",
        "not a tool-exposure defect",
        "not captured",
        "not classified",
        "not recorded",
        "not reported",
        "not routed",
        "not tracked",
        "without capturing",
        "without recording",
        "without reporting",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
}

fn has_thread_tool_name(line: &str) -> bool {
    thread_tool_names()
        .into_iter()
        .any(|tool| has_tool_name(line, tool))
}

fn has_tool_name(line: &str, tool: &str) -> bool {
    line.contains(tool) || line.contains(&format!("codex_app.{tool}"))
}

fn thread_tool_names() -> [&'static str; 6] {
    [
        "create_thread",
        "fork_thread",
        "list_threads",
        "read_thread",
        "send_message_to_thread",
        "set_thread_title",
    ]
}
