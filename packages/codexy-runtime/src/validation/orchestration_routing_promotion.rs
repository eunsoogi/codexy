const PERMISSION_WORDS: &[&str] = &[
    "allow",
    "allowed",
    "may",
    "can",
    "permit",
    "permitted",
    "proceed",
    "select",
];
const REQUIRED_EXCEPTION: &str =
    "only as an explicit exception selected by complete validated measurement";

pub(super) fn has_conflicting_promotion_exception(bullet: &str) -> bool {
    clauses(bullet)
        .filter(|clause| clause.contains("promotion") && clause.contains("terra/high"))
        .any(|clause| positive_permission(&clause) && !clause.contains(REQUIRED_EXCEPTION))
}

pub(super) fn has_temporally_narrowed_generic_default(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    normalized.contains("generic")
        && normalized.contains("child")
        && normalized.contains("terra/high")
        && normalized.contains("default")
        && normalized.contains("#549")
        && ["while", "until", "only when"]
            .iter()
            .any(|word| normalized.contains(word))
        && positive_permission(&normalized)
}

fn clauses(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split([';', '.'])
        .map(|clause| clause.trim().to_ascii_lowercase())
        .filter(|clause| !clause.is_empty())
}

fn positive_permission(clause: &str) -> bool {
    let words = clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    words.iter().enumerate().any(|(index, word)| {
        PERMISSION_WORDS.contains(word)
            && words.get(index + 1).is_none_or(|next| *next != "not")
            && !words[..index]
                .iter()
                .rev()
                .take(2)
                .any(|prior| matches!(*prior, "not" | "never"))
    })
}
