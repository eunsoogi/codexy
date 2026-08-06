use super::{active, is_passive_permission, passive_permission_is_negated};

pub(super) fn authorizes(words: &[String], loc_context: bool) -> bool {
    loc_context
        && words.iter().enumerate().any(|(index, word)| {
            is_overage_marker(words, index, word)
                && (words[index + 1..]
                    .iter()
                    .position(|word| word == "without")
                    .is_some_and(|without| {
                        words[index + 1 + without..].iter().any(|word| {
                            matches!(word.as_str(), "justification" | "rationale" | "reason")
                        })
                    })
                    || has_direct_modal(words, index)
                    || active::governs_overage(words, index)
                    || has_governing_passive_permission(words, index))
                && !is_negated(words, index)
        })
}

fn is_overage_marker(words: &[String], index: usize, word: &str) -> bool {
    matches!(word, "exceed" | "exceeded" | "exceeding" | "exceeds")
        || matches!(word, "over" | "above")
            && index > 0
            && matches!(words[index - 1].as_str(), "go" | "be")
}

fn has_direct_modal(words: &[String], overage: usize) -> bool {
    overage > 0 && matches!(words[overage - 1].as_str(), "may" | "can")
        || overage >= 2
            && matches!(words[overage - 2].as_str(), "may" | "can")
            && matches!(words[overage - 1].as_str(), "go" | "be")
}

fn has_governing_passive_permission(words: &[String], overage: usize) -> bool {
    let Some(subject) = words[..overage]
        .windows(3)
        .rposition(|phrase| matches!(phrase, [governed, subject, verb] if governed == "governed" && matches!(subject.as_str(), "file" | "files") && matches!(verb.as_str(), "is" | "are")))
    else {
        return false;
    };
    words[subject + 3..overage]
        .iter()
        .rposition(|word| is_passive_permission(word))
        .is_some_and(|permission| !passive_permission_is_negated(words, subject + 3 + permission))
}

pub(super) fn is_negated(words: &[String], exceed: usize) -> bool {
    let local_start = words[..exceed]
        .iter()
        .rposition(|word| matches!(word.as_str(), "but" | "however" | "yet"))
        .map_or(0, |boundary| boundary + 1);
    let local = &words[local_start..];
    let exceed = exceed - local_start;
    let before = &local[..exceed];
    if before
        .iter()
        .rposition(|word| matches!(word.as_str(), "may" | "can"))
        .is_some_and(|modal| !before[modal + 1..].iter().any(|word| word == "not"))
    {
        return false;
    }
    let modal_negation = |part: &[String]| {
        part.windows(2).any(|pair| matches!(pair, [modal, not] if matches!(modal.as_str(), "must" | "may" | "can") && not == "not")) || part.iter().any(|word| matches!(word.as_str(), "cannot" | "never"))
    };
    modal_negation(before) || modal_negation(&local[exceed + 1..])
}
