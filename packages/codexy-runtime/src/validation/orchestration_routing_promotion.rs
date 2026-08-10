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
    let normalized = bullet.to_ascii_lowercase();
    let words = words(&normalized);
    words.iter().enumerate().any(|(index, word)| {
        (*word == "allowed" || PERMISSION_WORDS.contains(word))
            && permission_binds_to_promotion(&words, index)
            && !permission_is_negated(&words, index)
            && (!normalized.contains(REQUIRED_EXCEPTION) || validation_is_compromised(&words))
    })
}

pub(super) fn has_temporally_narrowed_generic_default(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    let words = words(&normalized);
    words.iter().enumerate().any(|(index, word)| {
        *word == "apply"
            && positive_operand(&words, index)
            && operand_subject(&words, index).contains(&"generic")
            && operand_subject(&words, index).contains(&"child")
            && operand_subject(&words, index).contains(&"default")
            && (operand_subject(&words, index).contains(&"terra")
                || operand_subject(&words, index).contains(&"gpt"))
            && temporal_qualifier(&words, index)
    })
}

fn permission_binds_to_promotion(words: &[&str], operand: usize) -> bool {
    let subject = operand_subject(words, operand);
    subject.contains(&"promotion") && subject.contains(&"terra") && subject.contains(&"high")
}

fn operand_subject<'a>(words: &'a [&'a str], operand: usize) -> &'a [&'a str] {
    let start = words[..operand]
        .iter()
        .rposition(|word| *word == "while")
        .map_or(0, |index| index + 1);
    &words[start..operand]
}

fn temporal_qualifier(words: &[&str], operand: usize) -> bool {
    let suffix = words[operand + 1..]
        .iter()
        .take_while(|word| !matches!(**word, "must" | "may" | "can" | "cannot" | "apply"))
        .copied()
        .collect::<Vec<_>>();
    suffix.iter().any(|word| matches!(*word, "while" | "until"))
        || suffix.windows(2).any(|window| window == ["only", "when"])
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

fn words(clause: &str) -> Vec<&str> {
    clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}

fn permission_is_negated(words: &[&str], operand: usize) -> bool {
    words[operand.saturating_sub(3)..operand]
        .iter()
        .any(|word| matches!(*word, "not" | "never" | "cannot"))
        || words[operand + 1..]
            .first()
            .is_some_and(|word| matches!(*word, "not" | "never"))
}

fn validation_is_compromised(words: &[&str]) -> bool {
    words.iter().any(|word| *word == "without")
        || words.windows(3).any(|window| {
            window == ["before", "complete", "validated"] || window == ["prior", "to", "complete"]
        })
}
