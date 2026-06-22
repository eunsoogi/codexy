pub(super) fn has_git_graph_log_preflight(text: &str) -> bool {
    let lines: Vec<_> = text.lines().map(str::trim).collect();
    lines.iter().enumerate().any(|(index, line)| {
        is_git_preflight_line(line) && has_positive_evidence(line) && {
            let block = git_preflight_block(&lines, index, false);
            let evidence = git_preflight_block(&lines, index, true);
            has_all_commands(&block) && !has_negated_evidence(&evidence)
        }
    })
}

fn git_preflight_block(lines: &[&str], start: usize, commands_only: bool) -> String {
    let mut block = String::new();
    for (index, line) in lines.iter().enumerate().skip(start) {
        if index > start && starts_handoff_section(line) {
            break;
        }
        if !commands_only || index == start || contains_preflight_command(line) {
            block.push_str(line);
            block.push('\n');
        }
    }
    block
}

fn contains_preflight_command(line: &str) -> bool {
    [
        "pwd",
        "git status --short --branch",
        "git rev-parse head",
        "git rev-parse origin/main",
        "git log --graph",
    ]
    .iter()
    .any(|phrase| line.contains(phrase))
}

fn starts_handoff_section(line: &str) -> bool {
    [
        "codexy orchestration contract",
        "duplicate/no-active-work state",
        "parent/child ownership boundary",
        "stop condition",
        "authoritative stop condition",
        "next action",
    ]
    .iter()
    .any(|section| line.starts_with(section))
}

fn has_all_commands(text: &str) -> bool {
    [
        "pwd",
        "git status --short --branch",
        "git rev-parse head",
        "git rev-parse origin/main",
        "git log --graph",
    ]
    .iter()
    .all(|phrase| text.contains(phrase))
}

fn is_git_preflight_line(line: &str) -> bool {
    line.contains("git graph/log preflight") || line.contains("git preflight")
}

fn has_positive_evidence(line: &str) -> bool {
    has_any(
        line,
        &["captured", "ran", "were run", "checked", "recorded"],
    )
}

fn has_negated_evidence(line: &str) -> bool {
    has_any(
        line,
        &[
            "did not run",
            "didn't run",
            "not run",
            "not captured",
            "not checked",
            "without running",
            "missing",
            "omitted",
        ],
    )
}

fn has_any(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}
