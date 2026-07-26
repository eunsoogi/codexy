pub(super) struct Token {
    pub(super) text: String,
    starts_uppercase: bool,
}

pub(super) fn tokens(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut starts_uppercase = false;
    for character in input.chars() {
        if character.is_ascii_alphanumeric() || character == '@' {
            if word.is_empty() {
                starts_uppercase = character.is_uppercase();
            }
            word.push(character.to_ascii_lowercase());
        } else {
            finish(&mut tokens, &mut word, &mut starts_uppercase);
            if character == ',' {
                tokens.push(Token {
                    text: "comma".into(),
                    starts_uppercase: false,
                });
            }
        }
    }
    finish(&mut tokens, &mut word, &mut starts_uppercase);
    tokens
}

fn finish(tokens: &mut Vec<Token>, word: &mut String, starts_uppercase: &mut bool) {
    if !word.is_empty() {
        tokens.push(Token {
            text: std::mem::take(word),
            starts_uppercase: *starts_uppercase,
        });
        *starts_uppercase = false;
    }
}

pub(super) fn clause_boundary(tokens: &[Token]) -> Option<usize> {
    tokens
        .iter()
        .position(|token| token.text == "comma")
        .or_else(|| {
            tokens.iter().enumerate().find_map(|(index, token)| {
                (matches!(token.text.as_str(), "and" | "or" | "but" | "then")
                    && tokens
                        .get(index + 1)
                        .is_some_and(|next| next.starts_uppercase))
                .then_some(index)
            })
        })
}
