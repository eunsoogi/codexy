use super::child_lane_ownership_phrases::{
    field_value, has_absent_actor_phrase, has_absent_authored_phrase, has_absent_field_value,
    metadata_key, trimmed_value,
};

pub(super) fn line_has_parent_implementation_setup(line: &str) -> bool {
    if setup_field_value(line).is_some_and(|(key, value)| {
        (has_parent_context(key) || has_present_parent_context(value))
            && !has_absent_setup_field_value(value)
    }) {
        return true;
    }
    let line = trimmed_value(line);
    [
        "parent-created draft worktree",
        "parent-created implementation worktree",
        "parent-created implementation branch",
        "parent created draft worktree",
        "parent created implementation worktree",
        "parent created implementation branch",
        "orchestrator-created draft worktree",
        "orchestrator-created implementation worktree",
        "orchestrator-created implementation branch",
        "orchestrator created draft worktree",
        "orchestrator created implementation worktree",
        "orchestrator created implementation branch",
    ]
    .into_iter()
    .any(|marker| line.contains(marker) && !has_absent_setup_marker(line, marker))
}

fn setup_field_value<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
    line.split_once(':').and_then(|(key, value)| {
        let key = metadata_key(key);
        [
            "parent implementation setup",
            "orchestrator implementation setup",
            "implementation-surface reads",
            "implementation surface reads",
        ]
        .into_iter()
        .any(|field| key.contains(field))
        .then_some((key, value.trim()))
    })
}

fn has_parent_context(value: &str) -> bool {
    value.contains("parent") || value.contains("orchestrator")
}

fn has_present_parent_context(value: &str) -> bool {
    value.split([';', ',']).any(|clause| {
        has_present_actor_read_phrase(clause, "parent")
            || has_present_actor_read_phrase(clause, "orchestrator")
    })
}

fn has_present_actor_read_phrase(clause: &str, actor: &str) -> bool {
    (has_actor_read_action(clause, actor, "read") || has_actor_read_action(clause, actor, "reads"))
        && !has_absent_actor_read_phrase(clause, actor)
}

fn has_actor_read_action(clause: &str, actor: &str, action: &str) -> bool {
    let marker = format!("{actor} {action}");
    clause.match_indices(&marker).any(|(index, _)| {
        let prefix = &clause[..index];
        let suffix = &clause[index + marker.len()..];
        is_action_boundary(prefix.chars().next_back()) && is_action_boundary(suffix.chars().next())
    })
}

fn is_action_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        character.is_ascii_whitespace() || matches!(character, '.' | ',' | ';' | ':')
    })
}

fn has_absent_actor_read_phrase(clause: &str, actor: &str) -> bool {
    has_absent_actor_phrase(clause, actor, "read")
        || has_absent_actor_phrase(clause, actor, "reads")
}

fn has_absent_setup_field_value(value: &str) -> bool {
    has_absent_field_value(value, "implementation setup")
        || has_absent_actor_phrase(value, "parent", "implementation setup")
        || has_absent_actor_phrase(value, "orchestrator", "implementation setup")
}

fn has_absent_setup_marker(line: &str, marker: &str) -> bool {
    field_value(line, marker).is_some_and(|value| has_absent_field_value(value, marker))
        || has_absent_authored_phrase(line, marker)
}
