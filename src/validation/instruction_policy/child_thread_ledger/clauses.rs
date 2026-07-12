pub(super) fn has_false_requirement(text: &str) -> bool {
    text.match_indices("requirement").any(|(index, _)| {
        text[index + "requirement".len()..]
            .trim_start_matches(|character: char| {
                character == '#' || character.is_ascii_digit() || character.is_whitespace()
            })
            .starts_with(": false")
    })
}

pub(super) fn has_mutating_permission(after: &str) -> bool {
    after.split('.').any(has_live_sentinel_mutation)
}

fn has_live_sentinel_mutation(sentence: &str) -> bool {
    let words = sentence
        .split_whitespace()
        .map(|word| word.trim_matches(|character: char| !character.is_ascii_alphabetic()))
        .collect::<Vec<_>>();
    words
        .iter()
        .enumerate()
        .filter(|(_, word)| matches!(**word, "may" | "can" | "could" | "might"))
        .any(|(index, _)| {
            let following = &words[index + 1..];
            for (offset, word) in following.iter().enumerate() {
                if matches!(*word, "may" | "can" | "could" | "might") {
                    break;
                }
                if is_mutating_action(word, &following[offset + 1..])
                    && targets_live_sentinel(&words, word, &following[offset + 1..])
                {
                    if !is_negated_action(&following[..offset]) {
                        return true;
                    }
                }
            }
            false
        })
}

fn targets_live_sentinel(words: &[&str], action: &str, following: &[&str]) -> bool {
    words.windows(2).any(|words| words == ["live", "sentinel"])
        || matches!(action, "send" | "issue")
            && following
                .windows(2)
                .any(|words| matches!(words, ["status", "request"] | ["follow", "up"]))
        || action == "declare" && following.contains(&"unavailable")
}

fn is_negated_action(preceding: &[&str]) -> bool {
    let last_negation = preceding
        .iter()
        .rposition(|word| matches!(*word, "not" | "never"));
    let last_contrast = preceding
        .iter()
        .rposition(|word| matches!(*word, "but" | "however"));
    last_negation.is_some_and(|negation| last_contrast.is_none_or(|contrast| negation > contrast))
}

fn is_mutating_action(word: &str, following: &[&str]) -> bool {
    matches!(
        word,
        "interrupt"
            | "poll"
            | "message"
            | "mutate"
            | "declare"
            | "replace"
            | "cancel"
            | "terminate"
    ) || matches!(word, "send" | "issue")
        && following
            .iter()
            .any(|word| matches!(*word, "message" | "request" | "prompt"))
}
