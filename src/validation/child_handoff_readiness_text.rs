pub(super) fn has_affirmed_phrase(text: &str, phrase: &str) -> bool {
    let mut rest = text;
    let mut offset = 0;
    while let Some(index) = rest.find(phrase) {
        let start = offset + index;
        let end = start + phrase.len();
        if phrase_has_boundaries(text, start, end)
            && !is_locally_negated(&text[..start])
            && !super::codex_review_handoff::has_negative_label_value(&text[end..])
        {
            return true;
        }
        offset = end;
        rest = &text[offset..];
    }
    false
}

fn phrase_has_boundaries(text: &str, start: usize, end: usize) -> bool {
    is_boundary(text[..start].chars().next_back()) && is_boundary(text[end..].chars().next())
}

fn is_boundary(character: Option<char>) -> bool {
    character.is_none_or(|character| {
        !character.is_ascii_alphanumeric() && character != '-' && character != '_'
    })
}

fn is_locally_negated(prefix: &str) -> bool {
    let clause_start = last_clause_boundary(prefix).map_or(0, |index| index);
    prefix[clause_start..]
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .rev()
        .take(4)
        .any(|word| {
            matches!(
                word,
                "no" | "not"
                    | "never"
                    | "without"
                    | "isn't"
                    | "wasn't"
                    | "hasn't"
                    | "haven't"
                    | "aren't"
                    | "don't"
                    | "doesn't"
                    | "didn't"
                    | "won't"
                    | "can't"
                    | "cannot"
            )
        })
}

fn last_clause_boundary(text: &str) -> Option<usize> {
    text.char_indices()
        .filter(|(_, character)| matches!(character, '.' | '!' | '?' | ';' | ':' | ',' | '\n'))
        .map(|(index, character)| index + character.len_utf8())
        .last()
}
