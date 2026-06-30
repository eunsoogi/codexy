use std::path::Path;

use crate::paths::display_relative;
use crate::validation::instruction_policy_match;
pub(super) fn check_text(path: &Path, text: &str, errors: &mut Vec<String>, strict_clauses: bool) {
    let mut in_fence = false;
    let mut check_fence = false;
    let mut previous_prohibition_list = false;
    let mut previous_dangling_modal = false;
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            let lang = trimmed
                .trim_start_matches('`')
                .trim_start_matches('~')
                .trim();
            check_fence = !in_fence && matches!(lang, "" | "text");
            in_fence = !in_fence;
            previous_prohibition_list = false;
            previous_dangling_modal = false;
            continue;
        }
        let fenced_template_line = in_fence
            && check_fence
            && (trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed
                    .split_once(". ")
                    .is_some_and(|(prefix, _)| prefix.chars().all(|ch| ch.is_ascii_digit())));
        if in_fence && !check_fence
            || in_fence && !fenced_template_line
            || trimmed.is_empty()
            || trimmed.starts_with('>') && !trimmed.contains("MUST")
        {
            previous_prohibition_list = false;
            previous_dangling_modal = false;
            continue;
        }
        let normalized = instruction_line(trimmed);
        let custom_agent_toml = path.extension().and_then(|ext| ext.to_str()) == Some("toml");
        let passive_mandatory = custom_agent_toml || trimmed.starts_with("- ");
        let line_segments = checkable_line_segments(normalized);
        if line_segments.iter().any(|segment| {
            instruction_policy_match::has_prohibition_without_must_not(segment)
                || previous_prohibition_list
                    && instruction_policy_match::starts_with_inverted_prohibition(segment)
        }) {
            errors.push(format!(
                "{}:{} prohibitions must use MUST NOT",
                display_relative(path),
                index + 1
            ));
            previous_prohibition_list = false;
            previous_dangling_modal = false;
            continue;
        }
        if previous_dangling_modal
            && line_segments
                .iter()
                .any(|segment| instruction_policy_match::starts_with_modal(segment))
        {
            errors.push(format!(
                "{}:{} mandatory instructions must use MUST without duplicated modal wrapping",
                display_relative(path),
                index + 1
            ));
            previous_dangling_modal = false;
            continue;
        }
        if previous_dangling_modal {
            previous_prohibition_list = false;
            previous_dangling_modal =
                instruction_policy_match::ends_with_dangling_modal(normalized);
            continue;
        }
        previous_prohibition_list = normalized.contains("MUST NOT") && normalized.ends_with(',');
        previous_dangling_modal = instruction_policy_match::ends_with_dangling_modal(normalized);
        let root_agents = path.file_name().and_then(|name| name.to_str()) == Some("AGENTS.md");
        if line_segments.iter().any(|segment| {
            instruction_policy_match::has_bare_mandatory_without_must(
                segment,
                strict_clauses,
                root_agents,
                custom_agent_toml,
                passive_mandatory,
            )
        }) {
            errors.push(format!(
                "{}:{} mandatory instructions must use MUST",
                display_relative(path),
                index + 1
            ));
        }
    }
}

fn checkable_line_segments(line: &str) -> Vec<&str> {
    if !line.starts_with('|') {
        return vec![line];
    }
    line.trim_matches('|')
        .split('|')
        .map(str::trim)
        .filter(|cell| !cell.is_empty() && !cell.chars().all(|ch| matches!(ch, '-' | ':' | ' ')))
        .collect()
}

fn instruction_line(line: &str) -> &str {
    let line = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .unwrap_or(line);
    line.split_once(". ")
        .filter(|(prefix, _)| prefix.chars().all(|ch| ch.is_ascii_digit()))
        .map_or(line, |(_, rest)| rest)
}
