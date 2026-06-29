use super::git_preflight_lines::is_git_log_graph_output_line;

pub(super) const REQUIRED_PREFLIGHT_COMMANDS: &[&str] = &[
    "pwd",
    "git status --short --branch",
    "git rev-parse head",
    "git rev-parse origin/main",
    "git log --graph",
];

pub(super) fn has_all_commands(text: &str) -> bool {
    let text = text.to_ascii_lowercase();
    REQUIRED_PREFLIGHT_COMMANDS.iter().all(|phrase| {
        if matches!(*phrase, "git rev-parse head" | "git rev-parse origin/main") {
            contains_command_target(&text, phrase)
        } else {
            text.contains(phrase)
        }
    })
}

pub(super) fn has_executed_evidence(text: &str) -> bool {
    if pre_log_output_lines(text).any(has_planned_execution_evidence) {
        return false;
    }
    let text = text.to_ascii_lowercase();
    has_any(&text, &["captured", "were run", "checked", "recorded"])
        || text.lines().any(|line| line.starts_with("$ "))
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

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}
