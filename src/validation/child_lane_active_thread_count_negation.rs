pub(super) fn count_word(words: &[String], index: usize) -> Option<u64> {
    (!has_odd_count_negation(words, index))
        .then(|| match words.get(index)?.as_str() {
            "none" | "zero" => Some(0),
            word if word.chars().all(|character| character.is_ascii_digit()) => word.parse().ok(),
            _ => None,
        })
        .flatten()
}

pub(super) fn has_active_child_thread_key(words: &[String]) -> bool {
    words.iter().any(|word| word == "child")
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "thread" | "threads"))
        && has_affirmative_active_status(words)
        && (!words.iter().any(|word| {
            matches!(
                word.as_str(),
                "subagent" | "subagents" | "specialist" | "specialists"
            )
        }) || words
            .iter()
            .any(|word| matches!(word.as_str(), "exclude" | "excluding" | "excluded")))
}

fn has_affirmative_active_status(words: &[String]) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        let inherent_negations = match word.as_str() {
            "active" | "waiting" => 0,
            "inactive" => 1,
            _ => return false,
        };
        (inherent_negations + status_negations(words, index)) % 2 == 0
    })
}

fn has_odd_count_negation(words: &[String], index: usize) -> bool {
    let mut negations = 0;
    let mut position = index;
    while let Some(previous) = position.checked_sub(1) {
        let word = &words[previous];
        if matches!(word.as_str(), "not" | "no" | "non") {
            negations += 1;
        } else if !matches!(
            word.as_str(),
            "a" | "active"
                | "an"
                | "are"
                | "count"
                | "counts"
                | "codex"
                | "currently"
                | "equal"
                | "equals"
                | "exactly"
                | "inactive"
                | "is"
                | "of"
                | "reported"
                | "the"
                | "thread"
                | "threads"
                | "total"
                | "to"
                | "waiting"
                | "was"
        ) {
            break;
        }
        position = previous;
    }
    negations % 2 == 1
}

fn status_negations(words: &[String], index: usize) -> usize {
    let mut negations = 0;
    let mut position = index;
    if let Some(previous) = position
        .checked_sub(1)
        .filter(|previous| words[*previous] == "non")
    {
        negations += 1;
        position = previous;
    }
    while let Some(previous) = position.checked_sub(1) {
        if matches!(words[previous].as_str(), "not" | "no") {
            negations += 1;
            position = previous;
        } else if words[previous] == "currently" {
            position = previous;
        } else {
            break;
        }
    }
    negations
}
