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
        .into_iter()
        .filter(|clause| clause.contains("promotion") && clause.contains("terra/high"))
        .any(|clause| {
            positive_permission(&clause)
                && (!clause.contains(REQUIRED_EXCEPTION) || validation_is_preceded(&words(&clause)))
        })
}

pub(super) fn has_temporally_narrowed_generic_default(bullet: &str) -> bool {
    clauses(bullet).into_iter().any(|clause| {
        clause.contains("generic")
            && clause.contains("child")
            && clause.contains("terra/high")
            && clause.contains("default")
            && clause.contains("#549")
            && ["while", "until", "only when"]
                .iter()
                .any(|word| clause.contains(word))
            && affirmative_modal(&words(&clause))
    })
}

fn clauses(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .replace(", but ", ";")
        .replace(", while ", ";")
        .split([';', '.'])
        .map(|clause| clause.trim().to_ascii_lowercase())
        .filter(|clause| !clause.is_empty())
        .collect()
}

fn positive_permission(clause: &str) -> bool {
    let words = words(clause);
    words
        .iter()
        .enumerate()
        .any(|(index, word)| PERMISSION_WORDS.contains(word) && positive_operand(&words, index))
}

fn words(clause: &str) -> Vec<&str> {
    clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn affirmative_modal(words: &[&str]) -> bool {
    words
        .iter()
        .position(|word| *word == "apply")
        .is_some_and(|index| positive_operand(words, index))
}

fn positive_operand(words: &[&str], operand: usize) -> bool {
    let start = operand.saturating_sub(3);
    let Some(modal) = (start..operand)
        .rev()
        .find(|index| matches!(words[*index], "must" | "may" | "can" | "cannot"))
    else {
        return false;
    };
    words[modal] != "cannot"
        && !words[modal + 1..=operand]
            .iter()
            .any(|word| matches!(*word, "not" | "never"))
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
