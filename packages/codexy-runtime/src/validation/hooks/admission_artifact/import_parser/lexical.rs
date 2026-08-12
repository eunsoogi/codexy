#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Token {
    Word(String),
    Symbol(char),
}

pub(super) fn tokens(source: &str) -> Vec<Token> {
    let values = source.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < values.len() {
        match values[index] {
            value if value.is_whitespace() => index += 1,
            '#' => {
                while index < values.len() && values[index] != '\n' {
                    index += 1;
                }
            }
            '\\' if values.get(index + 1) == Some(&'\n') => index += 2,
            '\\' if values.get(index + 1) == Some(&'\r')
                && values.get(index + 2) == Some(&'\n') =>
            {
                index += 3
            }
            quote @ ('\'' | '"') => index = skip_string(&values, index, quote),
            value if value.is_ascii_alphanumeric() || value == '_' => {
                let start = index;
                while values
                    .get(index)
                    .is_some_and(|value| value.is_ascii_alphanumeric() || *value == '_')
                {
                    index += 1;
                }
                tokens.push(Token::Word(values[start..index].iter().collect()));
            }
            value => {
                tokens.push(Token::Symbol(value));
                index += 1;
            }
        }
    }
    tokens
}

pub(super) fn dynamic(tokens: &[Token]) -> bool {
    tokens.windows(2).any(|values| {
        matches!(values, [Token::Word(name), Token::Symbol('.')] if name == "importlib")
    }) || tokens.windows(2).any(|values| {
        matches!(values, [Token::Word(name), Token::Symbol('(')] if name == "__import__" || name == "exec")
    }) || tokens.windows(4).any(|values| {
        matches!(values, [Token::Word(from), Token::Word(module), Token::Word(import), Token::Word(name)]
            if from == "from" && module == "importlib" && import == "import" && name == "import_module")
    })
}

pub(super) fn word(tokens: &[Token], index: usize) -> Option<&str> {
    match tokens.get(index) {
        Some(Token::Word(value)) => Some(value),
        _ => None,
    }
}

pub(super) fn symbol(tokens: &[Token], index: usize, expected: char) -> bool {
    matches!(tokens.get(index), Some(Token::Symbol(value)) if *value == expected)
}

fn skip_string(values: &[char], mut index: usize, quote: char) -> usize {
    let triple = values.get(index + 1) == Some(&quote) && values.get(index + 2) == Some(&quote);
    index += if triple { 3 } else { 1 };
    while index < values.len() {
        if values[index] == '\\' {
            index += 2;
            continue;
        }
        if triple && values.get(index..index + 3) == Some(&[quote, quote, quote]) {
            return index + 3;
        }
        if !triple && values[index] == quote {
            return index + 1;
        }
        index += 1;
    }
    index
}
