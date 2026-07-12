pub(super) fn has_weakened_marker_prefix(prefix: &str) -> bool {
    let words = prefix
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let Some(verb_index) = words
        .iter()
        .rposition(|word| matches!(*word, "reference" | "record"))
    else {
        return false;
    };
    if words[..verb_index].last() != Some(&"must") {
        return false;
    }
    let suffix = &words[verb_index + 1..];
    let Some(first) = suffix.first() else {
        return false;
    };
    matches!(*first, "optional" | "optionally" | "waived" | "waiver")
        || matches!(*first, "if" | "when" | "where" | "unless" | "provided")
        || matches!(
            suffix,
            ["only", "if" | "when" | "where" | "unless" | "provided", ..]
        )
}

pub(super) fn has_quoted_marker_prefix(prefix: &str, suffix: &str) -> bool {
    let prefix = prefix.trim_end();
    has_unclosed_straight_quote(prefix, suffix, '"')
        || has_unclosed_straight_quote(prefix, suffix, '\'')
        || has_unclosed_straight_quote(prefix, suffix, '`')
        || has_unclosed_quote(prefix, '“', '”')
        || has_unclosed_quote(prefix, '‘', '’')
}

fn has_unclosed_straight_quote(prefix: &str, suffix: &str, quote: char) -> bool {
    let chars = prefix.chars().collect::<Vec<_>>();
    chars
        .iter()
        .enumerate()
        .fold(false, |open, (index, character)| {
            let contraction = quote == '\''
                && index > 0
                && chars[index - 1].is_alphanumeric()
                && ((!open && chars[index - 1] == 's')
                    || (open
                        && chars[index - 1] == 's'
                        && (chars[index + 1..]
                            .iter()
                            .all(|character| character.is_whitespace())
                            || suffix.contains(quote)))
                    || chars
                        .get(index + 1)
                        .is_some_and(|next| next.is_alphanumeric()));
            if *character == quote && !contraction {
                !open
            } else {
                open
            }
        })
}

fn has_unclosed_quote(prefix: &str, opening: char, closing: char) -> bool {
    let mut open = false;
    for character in prefix.chars() {
        if character == opening {
            open = true;
        } else if character == closing {
            open = false;
        }
    }
    open
}
