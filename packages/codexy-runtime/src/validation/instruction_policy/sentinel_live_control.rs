use super::sentinel_scope_policy::{has_positive_permission, split_modal_and_clause, words};

pub(super) fn detects(text: &str) -> bool {
    visible_text(text)
        .to_ascii_lowercase()
        .split(['.', '!', '?'])
        .flat_map(|sentence| sentence.split(" but "))
        .flat_map(split_modal_and_clause)
        .any(|clause| {
            let clause_words = words(clause);
            clause.contains("live sentinel")
                && !historical_or_terminal(clause)
                && clause_words.iter().enumerate().any(|(index, word)| {
                    matches_control(&clause_words, word)
                        && has_positive_permission(&clause_words, index)
                        && !has_prohibition(&clause_words, index)
                })
        })
}

fn visible_text(text: &str) -> String {
    let mut fenced = false;
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("```") {
                fenced = !fenced;
                return None;
            }
            if fenced || line.starts_with("sentinel_") {
                return None;
            }
            (!line.is_empty()).then_some(line)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn historical_or_terminal(sentence: &str) -> bool {
    "historic|former|previous"
        .split('|')
        .any(|marker| sentence.contains(marker))
        || sentence.contains("archived result")
            && "after terminal pass|after terminal block|after terminal unobservable"
                .split('|')
                .any(|marker| sentence.contains(marker))
}

fn matches_control(words: &[&str], word: &str) -> bool {
    [
        "message",
        "interrupt",
        "replace",
        "duplicate",
        "follow",
        "follow-up",
    ]
    .contains(&word)
        || word.starts_with("poll")
        || word == "send" && words.contains(&"terminal-status")
}

fn has_prohibition(words: &[&str], action_index: usize) -> bool {
    let context = words[..action_index]
        .rsplit(|word| *word == "but")
        .next()
        .unwrap();
    context
        .windows(2)
        .any(|pair| matches!(pair[0], "must" | "may" | "should") && pair[1] == "not")
        || context
            .iter()
            .any(|word| matches!(*word, "never" | "refrain" | "neither"))
}
