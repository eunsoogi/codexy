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
pub(super) fn has_conflicting_promotion_exception(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    clauses(&normalized).into_iter().any(|clause| {
        let words = words(clause);
        words.iter().enumerate().any(|(index, word)| {
            let end = action_end(&words, index);
            (*word == "allowed" || PERMISSION_WORDS.contains(word))
                && permission_binds_to_promotion(&words, index)
                && !permission_is_negated(&words, index)
                && (!has_required_exception(&words[..end])
                    || validation_is_compromised(&words[..end]))
        })
    })
}

pub(super) fn has_temporally_narrowed_generic_default(bullet: &str) -> bool {
    let normalized = bullet.to_ascii_lowercase();
    clauses(&normalized).into_iter().any(|clause| {
        let words = words(clause);
        words.iter().enumerate().any(|(index, word)| {
            let subject = operand_subject(&words, index);
            *word == "apply"
                && positive_operand(&words, index)
                && subject.contains(&"generic")
                && subject.contains(&"child")
                && subject.contains(&"default")
                && (subject.contains(&"terra") || subject.contains(&"gpt"))
                && temporal_qualifier(&words, index)
        })
    })
}

fn clauses(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut clauses = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(*byte, b';' | b'!' | b'?')
            || *byte == b'.' && bytes.get(index + 1).is_none_or(u8::is_ascii_whitespace)
        {
            clauses.push(&text[start..index]);
            start = index + 1;
        }
    }
    clauses.push(&text[start..]);
    clauses
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
    let suffix = words[operand + 1..action_end(words, operand)]
        .iter()
        .take_while(|word| !matches!(**word, "must" | "may" | "can" | "cannot" | "apply"))
        .copied()
        .collect::<Vec<_>>();
    suffix.iter().any(|word| matches!(*word, "while" | "until"))
        || suffix.windows(2).any(|window| window == ["only", "when"])
}

fn action_end(words: &[&str], operand: usize) -> usize {
    words[operand + 1..]
        .iter()
        .enumerate()
        .find_map(|(offset, word)| {
            (*word == "while" && introduces_action(&words[operand + offset + 2..]))
                .then_some(operand + offset + 1)
        })
        .unwrap_or(words.len())
}

fn introduces_action(words: &[&str]) -> bool {
    words
        .iter()
        .any(|word| *word == "apply" || *word == "allowed" || PERMISSION_WORDS.contains(word))
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

fn has_required_exception(words: &[&str]) -> bool {
    words.windows(10).any(|window| {
        window
            == [
                "only",
                "as",
                "an",
                "explicit",
                "exception",
                "selected",
                "by",
                "complete",
                "validated",
                "measurement",
            ]
    })
}
