use super::child_lane_ownership_phrases::{has_absent_field_value, trimmed_value};

pub(super) fn is_child_delegation_owner_decision(value: &str) -> bool {
    let value = trimmed_value(value);
    is_affirmative_child_owned_value(value)
        || is_current_thread_child_implementation(value)
        || (!has_negated_child_routing_requirement(value)
            && has_child_delegation(value)
            && has_routing_only_parent_context(value))
}

fn is_current_thread_child_implementation(value: &str) -> bool {
    has_affirmative_owner_token(value, "current-thread-owned")
        && (value.contains("implementation lane")
            || value.contains("child implementation")
            || value.contains("현재 작업이 구현을 소유함"))
}

pub(super) fn is_affirmative_child_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    has_affirmative_owner_token(value, "child-owned")
        && !has_absent_field_value(value, "child-owned")
}

pub(super) fn is_parent_owned_value(value: &str) -> bool {
    let value = trimmed_value(value);
    value.starts_with("parent-owned") && has_affirmative_owner_token(value, "parent-owned")
}

pub(super) fn is_supported_owner_decision(value: &str) -> bool {
    let value = trimmed_value(value);
    [
        is_child_delegation_owner_decision(value),
        is_parent_owned_value(value),
        is_external_human_owned_value(value),
    ]
    .into_iter()
    .filter(|supported| *supported)
    .count()
        == 1
}

fn is_external_human_owned_value(value: &str) -> bool {
    has_affirmative_owner_token(value, "external/human-owned")
}

fn has_affirmative_owner_token(value: &str, owner: &str) -> bool {
    has_owner_token(value, owner) && !has_negated_owner_token(value, owner)
}

fn has_owner_token(value: &str, owner: &str) -> bool {
    value
        .match_indices(owner)
        .any(|(index, _)| is_owner_token_at(value, owner, index))
}

fn has_negated_owner_token(value: &str, owner: &str) -> bool {
    value.match_indices(owner).any(|(index, _)| {
        is_owner_token_at(value, owner, index)
            && (has_english_negation_before(value, index)
                || (owner == "current-thread-owned"
                    && value.contains("현재 작업이 구현을 소유하지 않음")))
    })
}

fn is_owner_token_at(value: &str, owner: &str, index: usize) -> bool {
    let boundary = |byte: u8| !byte.is_ascii_alphanumeric() && byte != b'-';
    (index == 0 || boundary(value.as_bytes()[index - 1]))
        && value
            .as_bytes()
            .get(index + owner.len())
            .is_none_or(|byte| boundary(*byte))
}

fn has_english_negation_before(value: &str, index: usize) -> bool {
    value[..index]
        .rsplit(|character| matches!(character, ',' | ';' | '.'))
        .next()
        .is_some_and(|clause| {
            clause
                .split(|character: char| !character.is_ascii_alphabetic())
                .any(|word| matches!(word, "not" | "no" | "without"))
        })
}

fn has_child_delegation(value: &str) -> bool {
    (value.contains("child delegation")
        || value.contains("child-lane setup")
        || value.contains("child routing")
        || value.contains("child thread/worktree owner")
        || value.contains("thread/worktree tool discovery")
        || value.contains("thread tool discovery")
        || value.contains("worktree tool discovery"))
        && !value.contains("without child delegation")
}

fn has_routing_only_parent_context(value: &str) -> bool {
    value.contains("routing-only")
        || value.contains("coordination-only")
        || value.contains("delegation-only")
        || value.contains("child routing required")
        || value.contains("owner required")
        || value.contains("tool discovery only")
        || value.contains("tool-discovery-only")
}

fn has_negated_child_routing_requirement(value: &str) -> bool {
    let value = value.replace("child-routing", "child routing");
    [
        "no child routing required",
        "child routing not required",
        "no child routing is required",
        "child routing is not required",
        "without child routing",
    ]
    .into_iter()
    .any(|marker| value.contains(marker))
}
