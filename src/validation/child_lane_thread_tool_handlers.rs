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
const THREAD_TOOL_DISCOVERY_MARKERS: &str = "available|callable|discovered|expected|exposed|found|listed|registered|tool_search|tool search|visible";
const THREAD_TOOL_NAMES: &str = "create_thread|fork_thread|list_projects|list_threads|read_thread|send_message_to_thread|set_thread_title";

fn has_discovered_or_expected_thread_tool(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        let normalized = line.to_ascii_lowercase();
        has_thread_tool_name(line)
            && THREAD_TOOL_DISCOVERY_MARKERS
                .split('|')
                .any(|marker| normalized.contains(marker))
    })
}

fn has_actionable_handler_defect_report(evidence: &str, tool: &str) -> bool {
    has_defect_label(evidence)
        && [
            "no handler registered",
            "handler registered",
            "handler-missing",
            "missing-handler",
            "missing handler",
        ]
        .into_iter()
        .any(|marker| evidence.contains(marker))
        && has_tool_name(evidence, tool)
        && has_affirmative_defect_capture(evidence)
        && !has_absent_defect_capture(evidence)
}

fn has_actionable_handler_placeholder_report(evidence: &str) -> bool {
    has_defect_label(evidence)
        && evidence.contains(HANDLER_MISSING_MARKER)
        && has_affirmative_defect_capture(evidence)
        && !has_absent_defect_capture(evidence)
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
    let (_, line_start) = line_containing(evidence, start);
    let capture_start = multiline_capture_start(evidence, line_start);
    let next_start = evidence[start + HANDLER_MISSING_MARKER.len()..]
        .find(HANDLER_MISSING_MARKER)
        .map_or(evidence.len(), |offset| {
            start + HANDLER_MISSING_MARKER.len() + offset
        });
    &evidence[capture_start..next_start]
}

fn handler_missing_placeholder_scope(evidence: &str, line_start: usize) -> &str {
    let mut previous_start = line_start;
    let mut cursor = line_start;
    while cursor > 0 {
        let previous_end = cursor - 1;
        let candidate_start = evidence[..previous_end]
            .rfind('\n')
            .map_or(0, |index| index + 1);
        if !evidence[candidate_start..previous_end].trim().is_empty() {
            previous_start = candidate_start;
            break;
        }
        cursor = candidate_start;
    }
    let current_line_end = evidence[line_start..]
        .find('\n')
        .map_or(evidence.len(), |index| line_start + index);
    &evidence[previous_start..current_line_end]
}

fn multiline_capture_start(evidence: &str, line_start: usize) -> usize {
    let current_line_end = evidence[line_start..]
        .find('\n')
        .map_or(evidence.len(), |index| line_start + index);
    let current_trimmed = evidence[line_start..current_line_end].trim_start();
    if !current_trimmed.starts_with("- ") && !current_trimmed.starts_with("* ") {
        return line_start;
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
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") || has_defect_label(previous_line)
        {
            capture_start = previous_start;
            cursor = previous_start;
        } else {
            break;
        }
    }
    capture_start
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
        "not routed",
        "not tracked",
        "without capturing",
        "without recording",
        "without reporting",
    ]
    .into_iter()
    .any(|marker| line.contains(marker))
        || [
            "defect not reported",
            "handler defect not reported",
            "handler-missing defect not reported",
            "missing-handler defect not reported",
            "not reported as a dogfooding defect",
            "not reported as a tool-exposure defect",
            "not reported as dogfooding defect",
            "not reported as tool-exposure defect",
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

fn thread_tool_names() -> impl Iterator<Item = &'static str> {
    THREAD_TOOL_NAMES.split('|')
}
