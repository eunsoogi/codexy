use super::git_preflight_lines::is_git_log_graph_output_line;

pub(super) const REQUIRED_PREFLIGHT_COMMANDS: &[&str] = &[
    "pwd",
    "git status --short --branch",
    "git rev-parse head",
    "git rev-parse origin/main",
    "git log --graph",
];

pub(super) fn has_all_commands(text: &str) -> bool {
    let text = pre_log_output_lines(text)
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    REQUIRED_PREFLIGHT_COMMANDS.iter().all(|phrase| {
        if matches!(*phrase, "git rev-parse head" | "git rev-parse origin/main") {
            contains_command_target(&text, phrase)
        } else {
            text.contains(phrase)
        }
    })
}

pub(super) fn has_executed_evidence(text: &str) -> bool {
    let lines: Vec<_> = pre_log_output_lines(text).collect();
    if lines.iter().copied().any(has_planned_execution_evidence) {
        return false;
    }
    if lines.iter().any(|line| line.trim_start().starts_with("$ ")) {
        return REQUIRED_PREFLIGHT_COMMANDS
            .iter()
            .all(|phrase| has_executed_command(&lines, phrase));
    }
    let text = lines.join("\n").to_ascii_lowercase();
    if has_negated_execution_evidence(&text) {
        return false;
    }
    has_any(&text, &["captured", "were run", "checked", "recorded"])
}

fn has_executed_command(lines: &[&str], phrase: &str) -> bool {
    lines.iter().any(|line| {
        let command = line
            .trim_start()
            .strip_prefix("$ ")
            .unwrap_or("")
            .to_ascii_lowercase();
        command_starts_with_phrase(&command, phrase)
    })
}

fn command_starts_with_phrase(command: &str, phrase: &str) -> bool {
    command
        .strip_prefix(phrase)
        .is_some_and(|rest| rest.chars().next().is_none_or(char::is_whitespace))
}

fn contains_command_target(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(index, _)| {
        text[index + phrase.len()..]
            .chars()
            .next()
            .is_none_or(|character| {
                character.is_ascii_whitespace() || matches!(character, ',' | '.' | ';')
            })
    })
}

pub(super) fn pre_log_output_lines(text: &str) -> impl Iterator<Item = &str> {
    let mut saw_git_log_command = false;
    text.lines().filter(move |line| {
        if saw_git_log_command && is_git_log_graph_output_line(line) {
            return false;
        }
        if line.to_ascii_lowercase().contains("git log --graph") {
            saw_git_log_command = true;
        }
        true
    })
}

fn has_planned_execution_evidence(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    has_any(
        &line,
        &[
            "to be checked",
            "to be captured",
            "to be recorded",
            "to be run",
            "should be checked",
            "should be captured",
            "should be recorded",
            "should be run",
            "will be checked",
            "will be captured",
            "will be recorded",
            "will be recorded/captured",
            "will be run",
        ],
    )
}

fn has_negated_execution_evidence(text: &str) -> bool {
    text.lines().any(|line| {
        line.split([';', ',', '.']).any(|clause| {
            refers_to_git_preflight_evidence(clause)
                && has_ordered_pair(
                    clause,
                    &["no ", "not "],
                    &[
                        "captured",
                        "checked",
                        "executed",
                        "performed",
                        "recorded",
                        "run",
                    ],
                )
        })
    })
}

fn refers_to_git_preflight_evidence(clause: &str) -> bool {
    contains_token(clause, "git") || contains_token(clause, "preflight")
}

fn contains_token(text: &str, token: &str) -> bool {
    text.match_indices(token).any(|(index, _)| {
        let before = text[..index].chars().next_back();
        let after = text[index + token.len()..].chars().next();
        before.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '-')
            && after.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '-')
    })
}

fn has_ordered_pair(text: &str, before_phrases: &[&str], after_phrases: &[&str]) -> bool {
    before_phrases.iter().any(|before| {
        text.find(before).is_some_and(|index| {
            after_phrases
                .iter()
                .any(|after| text[index + before.len()..].contains(after))
        })
    })
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}
