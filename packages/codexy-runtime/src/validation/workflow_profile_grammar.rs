const STRICT_SIGNALS: [&str; 8] = [
    "destructive",
    "security",
    "secret",
    "secrets",
    "permission",
    "permissions",
    "release",
    "publication",
];

#[derive(Debug)]
struct Token {
    text: String,
    joined_to_previous: bool,
}

pub(super) fn value_has_strict_signal(value: &str) -> bool {
    category_clauses(value).iter().any(|tokens| {
        tokens
            .iter()
            .enumerate()
            .any(|(index, _)| signal_at(tokens, index) && !category_negated(tokens, index))
    })
}

fn category_clauses(value: &str) -> Vec<Vec<Token>> {
    let characters = value.chars().collect::<Vec<_>>();
    let mut clauses = vec![Vec::new()];
    let mut token = String::new();
    let mut joined_to_previous = false;
    for (index, character) in characters.iter().copied().enumerate() {
        if character.is_ascii_alphanumeric() {
            token.push(character);
            continue;
        }
        finish_token(&mut clauses, &mut token, &mut joined_to_previous);
        if compound_hyphen(&characters, index) {
            joined_to_previous = true;
        } else if clause_delimiter(character) {
            start_clause(&mut clauses);
        }
    }
    finish_token(&mut clauses, &mut token, &mut joined_to_previous);
    clauses
}

fn compound_hyphen(characters: &[char], index: usize) -> bool {
    matches!(characters[index], '-' | '‐' | '‑')
        && index > 0
        && index + 1 < characters.len()
        && characters[index - 1].is_ascii_alphanumeric()
        && characters[index + 1].is_ascii_alphanumeric()
}

fn clause_delimiter(character: char) -> bool {
    matches!(
        character,
        ',' | ';' | ':' | '.' | '!' | '?' | '-' | '‐' | '‑' | '‒' | '–' | '—' | '―' | '−'
    )
}

fn finish_token(clauses: &mut Vec<Vec<Token>>, token: &mut String, joined_to_previous: &mut bool) {
    if token.is_empty() {
        return;
    }
    if !*joined_to_previous && matches!(token.as_str(), "but" | "yet" | "however") {
        start_clause(clauses);
    } else {
        let Some(clause) = clauses.last_mut() else {
            return;
        };
        clause.push(Token {
            text: std::mem::take(token),
            joined_to_previous: std::mem::take(joined_to_previous),
        });
        return;
    }
    token.clear();
    *joined_to_previous = false;
}

fn start_clause(clauses: &mut Vec<Vec<Token>>) {
    if clauses.last().is_some_and(|clause| !clause.is_empty()) {
        clauses.push(Vec::new());
    }
}

fn signal_at(tokens: &[Token], index: usize) -> bool {
    signal_end(tokens, index).is_some()
}

fn signal_end(tokens: &[Token], index: usize) -> Option<usize> {
    STRICT_SIGNALS
        .contains(&tokens[index].text.as_str())
        .then_some(index)
        .or_else(|| compound_signal(tokens, index).map(|width| index + width - 1))
}

fn compound_signal(tokens: &[Token], index: usize) -> Option<usize> {
    let token = tokens[index].text.as_str();
    let next = tokens.get(index + 1).map(|token| token.text.as_str());
    matches!(
        (token, next),
        ("high", Some("risk")) | ("multi", Some("lane")) | ("merge", Some("sensitive"))
    )
    .then_some(2)
    .or_else(|| {
        matches!(
            (
                token,
                next,
                tokens.get(index + 2).map(|token| token.text.as_str()),
                tokens.get(index + 3).map(|token| token.text.as_str()),
            ),
            ("high", Some("consequence"), Some("external"), Some("state"))
        )
        .then_some(4)
    })
}

fn category_negated(tokens: &[Token], index: usize) -> bool {
    free_suffix(tokens, index)
        || postfix_negation(tokens, index)
        || direct_prefix_negation(tokens, index)
        || coordinated_prefix_negation(tokens, index)
}

fn free_suffix(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index + 1)
        .is_some_and(|token| token.joined_to_previous && token.text == "free")
}

fn postfix_negation(tokens: &[Token], index: usize) -> bool {
    let mut cursor = signal_end(tokens, index).map_or(index + 1, |end| end + 1);
    while tokens
        .get(cursor)
        .is_some_and(|token| token.joined_to_previous)
    {
        cursor += 1;
    }
    matches!(
        (
            tokens.get(cursor).map(|token| token.text.as_str()),
            tokens.get(cursor + 1).map(|token| token.text.as_str()),
            tokens.get(cursor + 2).map(|token| token.text.as_str()),
        ),
        (
            Some("is" | "are" | "was" | "were"),
            Some("not"),
            Some("involved")
        )
    )
}

fn direct_prefix_negation(tokens: &[Token], index: usize) -> bool {
    let cursor = skip_articles(tokens, index);
    let Some(prefix) = cursor.checked_sub(1) else {
        return false;
    };
    if tokens[prefix].text == "only" && prefix > 0 && tokens[prefix - 1].text == "not" {
        return false;
    }
    matches!(
        tokens[prefix].text.as_str(),
        "no" | "not" | "without" | "non"
    )
}

fn coordinated_prefix_negation(tokens: &[Token], index: usize) -> bool {
    let cursor = skip_articles(tokens, index);
    let Some(coordinator) = cursor.checked_sub(1) else {
        return false;
    };
    if !matches!(tokens[coordinator].text.as_str(), "or" | "and") {
        return false;
    }
    let Some(candidate_end) = coordinator.checked_sub(1) else {
        return false;
    };
    let Some(candidate) = signal_start(tokens, candidate_end) else {
        return false;
    };
    signal_at(tokens, candidate)
        && (coordinating_prefix_negation(tokens, candidate)
            || coordinated_prefix_negation(tokens, candidate))
}

fn signal_start(tokens: &[Token], end: usize) -> Option<usize> {
    (0..=end)
        .rev()
        .find(|start| signal_end(tokens, *start) == Some(end))
}

fn coordinating_prefix_negation(tokens: &[Token], index: usize) -> bool {
    let cursor = skip_articles(tokens, index);
    cursor
        .checked_sub(1)
        .is_some_and(|prefix| matches!(tokens[prefix].text.as_str(), "no" | "not" | "without"))
}

fn skip_articles(tokens: &[Token], mut cursor: usize) -> usize {
    while cursor > 0 && matches!(tokens[cursor - 1].text.as_str(), "a" | "an" | "the" | "any") {
        cursor -= 1;
    }
    cursor
}
