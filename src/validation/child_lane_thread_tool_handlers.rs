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
    evidence
        .lines()
        .any(|line| line.contains("no handler registered for tool:") && has_thread_tool_name(line))
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
    })
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
