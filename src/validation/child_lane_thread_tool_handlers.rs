pub(super) fn has_uncaptured_defect(evidence: &str) -> bool {
    has_discovered_or_expected_thread_tool(evidence)
        && has_thread_tool_handler_missing_evidence(evidence)
        && !has_actionable_handler_defect_report(evidence)
}

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

fn has_thread_tool_handler_missing_evidence(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        line.match_indices("no handler registered for tool:")
            .any(|(start, _)| {
                let claim = handler_missing_claim(line, start);
                has_thread_tool_name(claim) && !has_negated_handler_missing_claim(line, start)
            })
    })
}

fn has_actionable_handler_defect_report(evidence: &str) -> bool {
    evidence.lines().any(|line| {
        (line.contains("dogfooding defect") || line.contains("tool-exposure defect"))
            && [
                "no handler registered",
                "handler registered",
                "handler-missing",
                "missing handler",
            ]
            .into_iter()
            .any(|marker| line.contains(marker))
            && has_affirmative_defect_capture(line)
            && !has_absent_defect_capture(line)
    })
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

fn handler_missing_claim(line: &str, start: usize) -> &str {
    let prefix_start = line[..start].rfind(';').map_or(0, |offset| offset + 1);
    let suffix_end = line[start..]
        .find(';')
        .map_or(line.len(), |offset| start + offset);
    &line[prefix_start..suffix_end]
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
        .any(|tool| line.contains(tool) || line.contains(&format!("codex_app.{tool}")))
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
