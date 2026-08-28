pub(super) fn check(evidence: &str) -> Vec<String> {
    let active = super::child_lifecycle_events::active_lines(evidence);
    let lines = active
        .iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>();
    let source = lines
        .iter()
        .find_map(|line| line.strip_prefix("source thread id: "))
        .filter(|value| !value.is_empty());
    let mut errors = super::child_terminal_handoff::check(&lines, source);
    if lines.iter().any(|line| {
        line.strip_prefix("parent route: ")
            .and_then(|route| route.split([';', ',', ' ']).next())
            .is_some_and(super::child_terminal_handoff::is_local_task_target)
    }) {
        errors.push("child goal reporting must not use local agents /root routing".into());
    }
    errors
}
