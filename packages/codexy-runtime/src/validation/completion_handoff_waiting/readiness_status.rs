const READINESS_WORDS: [&str; 2] = ["pr", "merge"];
const STATUS_WORDS: [&str; 3] = ["blocker", "blockers", "status"];
const PENDING_WORDS: [&str; 3] = ["blocked", "unresolved", "waiting"];

pub(super) fn is_neutral_heading(fragment: &str) -> bool {
    let Some((heading, value)) = fragment.trim().split_once(':') else {
        return false;
    };
    let heading_words = words(heading);
    is_readiness_heading(&heading_words) && is_pending_status(&words(value))
}

fn is_readiness_heading(words: &[&str]) -> bool {
    let Some(readiness) = words.iter().position(|word| *word == "readiness") else {
        return false;
    };
    let owner = readiness.checked_sub(1).and_then(|index| words.get(index));
    matches!(owner, Some(owner) if READINESS_WORDS.contains(owner))
        && (words.len() == readiness + 1
            || words[readiness + 1..]
                .iter()
                .any(|word| STATUS_WORDS.contains(word)))
}

fn is_pending_status(words: &[&str]) -> bool {
    PENDING_WORDS.iter().any(|pending| words.contains(pending))
        || has_word_pair(words, "not", "ready")
        || has_word_pair(words, "not", "complete")
}

fn has_word_pair(words: &[&str], first: &str, second: &str) -> bool {
    words
        .windows(2)
        .any(|pair| pair[0] == first && pair[1] == second)
        || words
            .windows(3)
            .any(|pair| pair[0] == first && pair[2] == second)
}

fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect()
}
