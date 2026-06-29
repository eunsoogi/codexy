use super::git_preflight_commands::{
    REQUIRED_PREFLIGHT_COMMANDS, has_all_commands, has_executed_evidence, pre_log_output_lines,
};
use super::git_preflight_lines::{
    is_git_log_graph_output_line, is_git_status_output_after_command,
};

pub(super) fn has_git_graph_log_preflight(text: &str) -> bool {
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    lines.iter().enumerate().any(|(index, line)| {
        is_git_preflight_line(line) && !is_unchecked_checklist_item(line) && {
            let block = git_preflight_evidence_block(&lines, index);
            has_all_commands(&block)
                && has_executed_evidence(&block)
                && !has_negated_evidence(&block)
        }
    })
}

fn git_preflight_evidence_block(lines: &[&str], start: usize) -> String {
    let mut block = String::new();
    let mut saw_git_log_command = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        if index > start
            && starts_handoff_section(line)
            && !is_git_status_output_after_command(lines, index)
            && !(saw_git_log_command && is_git_log_graph_output_line(line))
        {
            break;
        }
        if is_unchecked_checklist_item(line) {
            continue;
        }
        if line.to_ascii_lowercase().contains("git log --graph") {
            saw_git_log_command = true;
        }
        block.push_str(line);
        block.push('\n');
    }
    block
}

fn starts_handoff_section(line: &str) -> bool {
    if line.trim_start().starts_with('#') {
        return true;
    }
    if starts_unrelated_list_section(line) {
        return true;
    }

    let line = metadata_line(line);
    let starts_known_section = [
        "codexy orchestration contract",
        "duplicate/no-active-work state",
        "parent/child ownership boundary",
        "stop condition",
        "authoritative stop condition",
        "next action",
    ]
    .iter()
    .any(|section| line.to_ascii_lowercase().starts_with(section));

    starts_known_section || starts_unbulleted_section_label(line)
}

fn starts_unrelated_list_section(line: &str) -> bool {
    let line = line.trim();
    if !line.starts_with(['-', '*']) {
        return false;
    }

    let line = metadata_line(line);
    if is_git_preflight_line(line) || starts_with_preflight_command(line) {
        return false;
    }
    line.contains(':') || is_plain_list_section_heading(line)
}

fn is_plain_list_section_heading(line: &str) -> bool {
    !line.is_empty()
        && line
            .chars()
            .all(|character| character.is_ascii_alphabetic() || character.is_ascii_whitespace())
}

fn starts_unbulleted_section_label(line: &str) -> bool {
    let line = line.trim();
    let Some((label, _)) = line.split_once(':') else {
        return false;
    };
    let label = label.trim();
    !label.is_empty()
        && !line.starts_with(['-', '*'])
        && !is_git_preflight_line(line)
        && !starts_with_preflight_command(line)
        && !is_block_local_preflight_negation(label)
        && label.chars().all(is_section_label_character)
}

fn is_section_label_character(character: char) -> bool {
    character.is_ascii_alphabetic() || character.is_ascii_whitespace() || character == '-'
}

fn is_unchecked_checklist_item(line: &str) -> bool {
    line.trim()
        .trim_start_matches(['-', '*'])
        .trim_start()
        .starts_with("[ ]")
}

fn starts_with_preflight_command(line: &str) -> bool {
    REQUIRED_PREFLIGHT_COMMANDS
        .iter()
        .any(|phrase| line.to_ascii_lowercase().starts_with(phrase))
}

fn metadata_line(line: &str) -> &str {
    let line = line.trim().trim_start_matches(['-', '*']).trim_start();
    let line = line
        .strip_prefix("[x]")
        .or_else(|| line.strip_prefix("[X]"))
        .or_else(|| line.strip_prefix("[ ]"))
        .unwrap_or(line)
        .trim_start();
    line.trim_start_matches('#').trim_start()
}

fn is_git_preflight_line(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    line.contains("git graph/log preflight") || line.contains("git preflight")
}

fn has_negated_evidence(text: &str) -> bool {
    pre_log_output_lines(text).any(|line| {
        let line = line.to_ascii_lowercase();
        has_negation_phrase(&line)
            && (refers_to_git_preflight(&line) || is_block_local_preflight_negation(&line))
    })
}

fn has_negation_phrase(line: &str) -> bool {
    has_any(
        line,
        &[
            "did not run",
            "didn't run",
            "not run",
            "not all commands were run",
            "not all commands were captured",
            "not all preflight commands were run",
            "not all preflight commands were captured",
            "not actually run",
            "not captured",
            "not checked",
            "not executed",
            "not performed",
            "no preflight command execution",
            "no preflight command capture",
            "without running",
        ],
    )
}

fn is_block_local_preflight_negation(line: &str) -> bool {
    let line = line
        .trim()
        .trim_end_matches([':', '.', ';'])
        .to_ascii_lowercase();
    [
        "not actually run",
        "commands not run",
        "commands not captured",
        "not captured",
        "not checked",
        "without running",
    ]
    .iter()
    .any(|phrase| {
        line == *phrase
            || line
                .strip_prefix(phrase)
                .is_some_and(|rest| rest.trim_start().starts_with('('))
    })
}

fn refers_to_git_preflight(line: &str) -> bool {
    is_git_preflight_line(line)
        || contains_token(line, "preflight")
        || [
            "preflight commands",
            "commands were not run",
            "commands were not captured",
            "commands were not executed",
            "commands were not performed",
            "these commands",
            "the commands",
            "pwd",
            "git status --short --branch",
            "git rev-parse head",
            "git rev-parse origin/main",
            "git log --graph",
        ]
        .iter()
        .any(|phrase| line.contains(phrase))
}

fn contains_token(text: &str, token: &str) -> bool {
    text.match_indices(token).any(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index + token.len()..].chars().next();
        before.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '-')
            && after.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '-')
    })
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}
