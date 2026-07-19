use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};

pub(super) fn is_child_completion_owner(value: &str) -> bool {
    if value.starts_with("current-thread-owned") {
        return is_current_thread_owner(value);
    }
    !is_parent_owned_value(value) && is_child_delegation_owner_decision(value)
}

fn is_current_thread_owner(value: &str) -> bool {
    let normalized = value.replace(['‘', '’'], "'");
    let value = normalized.as_str();
    value.starts_with("current-thread-owned")
        && (value.contains("implementation lane")
            || value.contains("child implementation")
            || value.contains("구현"))
        && !has_affirmative_parent_owner(value)
        && !has_owner_denial(value)
}

fn has_affirmative_parent_owner(value: &str) -> bool {
    value.split([';', ',', '—']).any(|clause| {
        (clause.contains("parent-owned") && !clause.contains("not parent-owned"))
            || (clause.contains("부모 소유자")
                && ![
                    "아님",
                    "아니다",
                    "아니며",
                    "아닌",
                    "아닙니다",
                    "아니에요",
                    "않음",
                    "않다",
                ]
                .iter()
                .any(|marker| clause.contains(marker)))
    })
}

fn has_owner_denial(value: &str) -> bool {
    value.split([';', ',', '—']).any(|clause| {
        let clause = without_parent_denials(clause);
        clause
            .split(|character: char| !character.is_alphanumeric())
            .any(|word| {
                matches!(
                    word,
                    "not" | "no" | "without" | "absent" | "never" | "neither"
                )
            })
            || ["isn't", "doesn't", "don't", "can't", "cannot", "won't"]
                .iter()
                .any(|marker| clause.contains(marker))
            || [
                "아님",
                "아니다",
                "아닌",
                "아닙니다",
                "아니에요",
                "않음",
                "않다",
                "없음",
                "없다",
                "소유하지",
            ]
            .iter()
            .any(|marker| clause.contains(marker))
    })
}

fn without_parent_denials(clause: &str) -> String {
    [
        "not parent-owned",
        "no parent implementation edits",
        "부모 소유자가 아님",
        "부모 소유자가 아니다",
        "부모 소유자가 아니며",
        "부모 소유자가 아닌",
        "부모 소유자가 아닙니다",
        "부모 소유자가 아니에요",
        "부모 소유자가 아님을",
    ]
    .into_iter()
    .fold(clause.to_owned(), |value, marker| value.replace(marker, ""))
}
