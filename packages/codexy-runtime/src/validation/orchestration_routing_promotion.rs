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
            && {
                let words = words(&clause);
                words.iter().enumerate().any(|(index, word)| {
                    *word == "apply"
                        && positive_operand(&words, index)
                        && temporal_qualifier(&words, index)
                })
            }
    })
}

fn clauses(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(", but ", ";")
        .replace(", while #549", " while #549")
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

fn temporal_qualifier(words: &[&str], operand: usize) -> bool {
    let suffix = words[operand + 1..]
        .iter()
        .take_while(|word| **word != "apply")
        .copied()
        .collect::<Vec<_>>();
    suffix.contains(&"549") && suffix.iter().any(|word| matches!(*word, "while" | "until"))
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
