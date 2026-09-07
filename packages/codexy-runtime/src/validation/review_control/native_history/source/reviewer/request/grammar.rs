pub(super) fn starts_with_example_wrapper(text: &str) -> bool {
    let mut text = text.trim_start();
    text = text.strip_prefix("- ").unwrap_or(text).trim_start();
    loop {
        if matches!(
            text.chars().next(),
            Some('`' | '\'' | '"' | '“' | '”' | '‘' | '’' | '>')
        ) {
            return true;
        }
        let Some(rest) = ["**", "__", "*", "_"]
            .iter()
            .find_map(|marker| text.strip_prefix(marker))
        else {
            return false;
        };
        text = rest.trim_start();
    }
}

pub(super) fn contains_prefix_negation(text: &str) -> bool {
    let Some(review_start) = first_word_start(text, "review") else {
        return false;
    };
    let prefix = &text[..review_start + "review".len()];
    let clause = prefix.rsplit_once(';').map_or(prefix, |(_, clause)| clause);
    let words = clause
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let action_index = words
        .iter()
        .rposition(|word| is_action(word) && *word != "review")
        .or_else(|| words.iter().rposition(|word| *word == "review"));
    let Some(action_index) = action_index else {
        return false;
    };
    directly_negated(&words[..action_index])
}

fn first_word_start(text: &str, target: &str) -> Option<usize> {
    let mut start = None;
    for (index, character) in text
        .char_indices()
        .chain(std::iter::once((text.len(), '\0')))
    {
        if character.is_ascii_alphanumeric() {
            start.get_or_insert(index);
        } else if let Some(start) = start.take()
            && text[start..index].eq_ignore_ascii_case(target)
        {
            return Some(start);
        }
    }
    None
}

fn is_action(word: &str) -> bool {
    matches!(
        word,
        "perform"
            | "run"
            | "conduct"
            | "complete"
            | "start"
            | "begin"
            | "execute"
            | "authorize"
            | "review"
            | "select"
            | "use"
    )
}

fn directly_negated(words: &[&str]) -> bool {
    words.last().is_some_and(|word| {
        matches!(
            *word,
            "not" | "without" | "never" | "cannot" | "no" | "neither"
        )
    }) || words.windows(2).last().is_some_and(|pair| {
        matches!(
            pair,
            ["do", "not"]
                | ["does", "not"]
                | ["did", "not"]
                | ["must", "not"]
                | ["should", "not"]
                | ["will", "not"]
                | ["don", "t"]
                | ["doesn", "t"]
                | ["didn", "t"]
                | ["won", "t"]
        )
    })
}

pub(super) fn contains_post_review_negation(words: &[&str]) -> bool {
    matches!(
        words,
        ["not" | "never" | "no"]
            | ["do" | "does" | "did" | "must" | "should" | "will", "not"]
            | ["don" | "doesn" | "didn" | "won", "t"]
    )
}
