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
        .any(|clause| {
            positive_permission(&clause)
                && (!clause.contains(REQUIRED_EXCEPTION) || validation_is_preceded(&words(&clause)))
        })
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
        && affirmative_modal(&words(&normalized))
}

fn clauses(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split([';', '.'])
        .map(|clause| clause.trim().to_ascii_lowercase())
        .filter(|clause| !clause.is_empty())
}

fn positive_permission(clause: &str) -> bool {
    let words = words(clause);
    affirmative_modal(&words) && words.iter().any(|word| PERMISSION_WORDS.contains(word))
}

fn words(clause: &str) -> Vec<&str> {
    clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn affirmative_modal(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        matches!(*word, "must" | "may" | "can")
            && !words[index + 1..]
                .iter()
                .take(1)
                .any(|prior| matches!(*prior, "not" | "never"))
    }) && !words.iter().any(|word| *word == "cannot")
}

fn validation_is_preceded(words: &[&str]) -> bool {
    words
        .windows(3)
        .position(|window| window == ["complete", "validated", "measurement"])
        .is_some_and(|index| {
            words[..index]
                .iter()
                .any(|word| matches!(*word, "before" | "prior" | "until"))
        })
}
