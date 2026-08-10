use super::wiki_minimal_contract_markdown::{ActiveEvent, ActiveKind, Document, Scope};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Mode {
    Must,
    MustNot,
}

pub(crate) struct Clause {
    pub(crate) mode: Mode,
    pub(crate) prose: Vec<String>,
    pub(crate) inline: Vec<String>,
}

enum Token {
    Word { value: String, spaced: bool },
    Inline(String),
    Sentence,
}

#[derive(Default)]
struct Gap {
    whitespace: bool,
    punctuation: bool,
}

pub(crate) fn clauses(document: &Document, scope: &Scope) -> Vec<Clause> {
    let tokens = tokenize(&document.active_events(scope));
    let modes = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| mode_at(&tokens, index, token))
        .collect::<Vec<_>>();
    modes
        .iter()
        .enumerate()
        .map(|(index, &(start, mode))| {
            let sentence = sentence_start(&tokens, start);
            let begin = modes[..index]
                .last()
                .is_some_and(|prior| prior.0 >= sentence)
                .then_some(start)
                .unwrap_or(sentence);
            let next = modes.get(index + 1).map_or(tokens.len(), |next| next.0);
            let end = next.min(sentence_end(&tokens, start));
            Clause {
                mode,
                prose: tokens[begin..end]
                    .iter()
                    .filter_map(word)
                    .map(str::to_ascii_lowercase)
                    .collect(),
                inline: tokens[start..end]
                    .iter()
                    .filter_map(|token| match token {
                        Token::Inline(value) => Some(value.clone()),
                        _ => None,
                    })
                    .collect(),
            }
        })
        .collect()
}

fn tokenize(events: &[ActiveEvent<'_>]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut gap = Gap::default();
    for event in events {
        match event.kind {
            ActiveKind::Prose => prose_tokens(event.value, &mut tokens, &mut gap),
            ActiveKind::Inline => {
                tokens.push(Token::Inline(event.value.into()));
                gap = Gap::default();
            }
        }
    }
    tokens
}

fn prose_tokens(source: &str, tokens: &mut Vec<Token>, gap: &mut Gap) {
    let mut word = String::new();
    for character in source.chars() {
        if character.is_ascii_alphanumeric() {
            word.push(character);
        } else {
            flush(&mut word, tokens, gap);
            if character == '.' {
                tokens.push(Token::Sentence);
                *gap = Gap::default();
            } else if character.is_whitespace() {
                gap.whitespace = true;
            } else {
                gap.punctuation = true;
            }
        }
    }
    flush(&mut word, tokens, gap);
}

fn flush(word: &mut String, tokens: &mut Vec<Token>, gap: &mut Gap) {
    if !word.is_empty() {
        tokens.push(Token::Word {
            value: std::mem::take(word),
            spaced: gap.whitespace && !gap.punctuation,
        });
        *gap = Gap::default();
    }
}

fn mode_at(tokens: &[Token], index: usize, token: &Token) -> Option<(usize, Mode)> {
    let Token::Word { value, .. } = token else {
        return None;
    };
    if value != "MUST" {
        return None;
    }
    match tokens.get(index + 1) {
        Some(Token::Word {
            value,
            spaced: true,
        }) if value == "NOT" => Some((index, Mode::MustNot)),
        _ => Some((index, Mode::Must)),
    }
}

fn sentence_start(tokens: &[Token], start: usize) -> usize {
    tokens[..start]
        .iter()
        .rposition(|token| matches!(token, Token::Sentence))
        .map_or(0, |index| index + 1)
}

fn sentence_end(tokens: &[Token], start: usize) -> usize {
    tokens[start..]
        .iter()
        .position(|token| matches!(token, Token::Sentence))
        .map_or(tokens.len(), |index| start + index)
}

fn word(token: &Token) -> Option<&str> {
    match token {
        Token::Word { value, .. } => Some(value),
        Token::Inline(_) | Token::Sentence => None,
    }
}
