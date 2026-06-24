pub(super) fn has_tool_name_in_defect_capture(evidence: &str, tool: &str) -> bool {
    let lines = evidence.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        is_defect_capture_line(line)
            && (has_tool_name_in_defect_clause(line, tool)
                || opens_defect_list(line)
                    && lines[index + 1..]
                        .iter()
                        .take_while(|following| is_list_item(following))
                        .any(|following| has_tool_name(following, tool)))
    })
}

fn is_defect_capture_line(line: &str) -> bool {
    line.contains("dogfooding defect")
        || line.contains("tool-exposure defect")
        || line.contains("dogfooding/tool-exposure defect")
}

fn has_tool_name_in_defect_clause(line: &str, tool: &str) -> bool {
    defect_capture_clause(line).is_some_and(|clause| has_tool_name(clause, tool))
}

fn opens_defect_list(line: &str) -> bool {
    defect_capture_clause(line).is_some_and(|clause| clause.trim_end().ends_with(':'))
}

fn defect_capture_clause(line: &str) -> Option<&str> {
    let clause = &line[line.find("defect")?..];
    let end = [". ", "; "]
        .into_iter()
        .filter_map(|boundary| clause.find(boundary))
        .min()
        .unwrap_or(clause.len());
    Some(&clause[..end])
}

fn is_list_item(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("- ") || trimmed.starts_with("* ")
}

fn has_tool_name(line: &str, tool: &str) -> bool {
    line.contains(tool) || line.contains(&format!("codex_app.{tool}"))
}
