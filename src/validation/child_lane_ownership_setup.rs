use super::child_lane_ownership_phrases::{
    field_value, has_absent_actor_phrase, has_absent_authored_phrase, has_absent_field_value,
    metadata_key, trimmed_value,
};

pub(super) fn line_has_parent_implementation_setup(lines: &[&str], index: usize) -> bool {
    let line = lines[index];
    if let Some((key, value)) = setup_field_value(line) {
        return value
            .is_empty()
            .then(|| setup_continuation_has_parent_implementation_setup(lines, index, key))
            .unwrap_or_else(|| setup_value_has_parent_implementation_setup(key, value));
    }
    line_value_has_parent_implementation_setup(line)
}

fn line_value_has_parent_implementation_setup(line: &str) -> bool {
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

fn setup_value_has_parent_implementation_setup(key: &str, value: &str) -> bool {
    if let Some((key, value)) = actor_read_field_value(value) {
        return actor_read_field_has_parent_implementation_setup(key, value);
    }

    (has_parent_context(key) || has_present_parent_context(value))
        && !has_absent_setup_field_value(value)
        && !has_absent_keyed_actor_read_value(key, value)
}

fn setup_continuation_has_parent_implementation_setup(
    lines: &[&str],
    index: usize,
    key: &str,
) -> bool {
    for line in lines.iter().skip(index + 1) {
        if line.is_empty() {
            continue;
        }
        let value = continuation_value(line);
        if is_setup_continuation_boundary(value) {
            break;
        }
        if setup_value_has_parent_implementation_setup(key, value) {
            return true;
        }
    }
    false
}

fn continuation_value(value: &str) -> &str {
    value.trim_start_matches(['-', '*']).trim()
}

fn is_setup_continuation_boundary(value: &str) -> bool {
    value.split_once(':').is_some_and(|(key, _)| {
        let key = metadata_key(key);
        !key.is_empty() && !has_actor_read_phrase(key)
    })
}

fn has_actor_read_phrase(value: &str) -> bool {
    ["child", "parent", "orchestrator"]
        .into_iter()
        .any(|actor| {
            has_actor_read_action(value, actor, "read")
                || has_actor_read_action(value, actor, "reads")
        })
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

fn actor_read_field_value(value: &str) -> Option<(&str, &str)> {
    value.split_once(':').and_then(|(key, value)| {
        let key = metadata_key(key);
        has_actor_read_phrase(key).then_some((key, value.trim()))
    })
}

fn actor_read_field_has_parent_implementation_setup(key: &str, value: &str) -> bool {
    ["parent", "orchestrator"].into_iter().any(|actor| {
        has_actor_read_phrase_for(key, actor) && !value_clauses_are_absent_actor_reads(value, actor)
    }) || has_present_parent_context(value)
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

fn has_absent_keyed_actor_read_value(key: &str, value: &str) -> bool {
    ["parent", "orchestrator"].into_iter().any(|actor| {
        has_actor_read_phrase_for(key, actor) && value_clauses_are_absent_actor_reads(value, actor)
    })
}

fn value_clauses_are_absent_actor_reads(value: &str, actor: &str) -> bool {
    let mut clauses = value
        .split([';', ','])
        .map(trimmed_value)
        .filter(|clause| !clause.is_empty())
        .peekable();

    if clauses.peek().is_none() {
        return false;
    }

    clauses.all(|clause| {
        has_absent_field_value(clause, &format!("{actor} read"))
            || has_absent_field_value(clause, &format!("{actor} reads"))
            || has_absent_actor_read_phrase(clause, actor)
    })
}

fn has_actor_read_phrase_for(value: &str, actor: &str) -> bool {
    has_actor_read_action(value, actor, "read") || has_actor_read_action(value, actor, "reads")
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
