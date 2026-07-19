use super::child_lane_owner_decision::{is_child_delegation_owner_decision, is_parent_owned_value};

pub(super) fn is_child_completion_owner(value: &str) -> bool {
    if value.starts_with("current-thread-owned") {
        return is_current_thread_owner(value);
    }
    !is_parent_owned_value(value)
        && !has_affirmative_parent_owner(value)
        && is_child_delegation_owner_decision(value)
}

fn is_current_thread_owner(value: &str) -> bool {
    let normalized = value.replace(['‘', '’'], "'");
    let value = normalized.as_str();
    value.starts_with("current-thread-owned")
        && has_implementation_lane(value)
        && !has_affirmative_parent_owner(value)
        && !has_owner_denial(value)
}

fn has_implementation_lane(value: &str) -> bool {
    [
        "implementation lane",
        "implementation owner",
        "owns implementation",
        "own implementation",
        "possède l'implémentation",
        "구현을 소유",
        "구현 소유",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn has_affirmative_parent_owner(value: &str) -> bool {
    value.split([';', ',', '—']).any(|clause| {
        let clause = without_parent_denials(clause);
        clause.contains("parent-owned")
            || clause.contains("parent implementation owner")
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
        let words = clause
            .split(|character: char| !character.is_alphanumeric())
            .collect::<Vec<_>>();
        let english = words.iter().enumerate().any(|(index, word)| {
            matches!(*word, "not" | "absent" | "never" | "neither")
                && words[index + 1..]
                    .iter()
                    .take(3)
                    .any(|word| matches!(*word, "owner" | "own" | "implementation"))
        });
        let owns_implementation = ["owner", "own", "implementation"]
            .iter()
            .any(|marker| clause.contains(marker));
        english
            || owns_implementation
                && ["isn't", "doesn't", "don't", "can't", "cannot", "won't"]
                    .iter()
                    .any(|marker| clause.contains(marker))
            || ((clause.contains("구현") || clause.contains("소유"))
                && [
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
                .any(|marker| clause.contains(marker)))
    })
}

fn without_parent_denials(clause: &str) -> String {
    [
        "not parent-owned",
        "no parent implementation edits",
        "parent ownership absent",
        "부모 소유자가 아님",
        "부모 소유자가 아니다",
        "부모 소유자가 아니며",
        "부모 소유자가 아닌",
        "부모 소유자가 아닙니다",
        "부모 소유자가 아니에요",
        "부모 소유자가 아님을",
        "부모 소유권 없음",
    ]
    .into_iter()
    .fold(clause.to_owned(), |value, marker| value.replace(marker, ""))
}
