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
    }) || imports_module(tokens, "importlib") || imports_from_importlib_loader(tokens)
}

fn imports_module(tokens: &[Token], expected: &str) -> bool {
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token, Token::Word(name) if name == "import") {
            continue;
        }
        let mut index = index + 1;
        loop {
            if word(tokens, index) == Some(expected) {
                return true;
            }
            let Some(next) = module_end(tokens, index) else {
                break;
            };
            index = next;
            if word(tokens, index) == Some("as") {
                index += 2;
            }
            if !symbol(tokens, index, ',') {
                break;
            }
            index += 1;
        }
    }
    false
}

fn imports_from_importlib_loader(tokens: &[Token]) -> bool {
    tokens.windows(3).enumerate().any(|(index, values)| {
        matches!(values, [Token::Word(from), Token::Word(module), Token::Word(import)]
            if from == "from" && module == "importlib" && import == "import")
            && imported_name(tokens, index + 3, "import_module")
    })
}

fn imported_name(tokens: &[Token], mut index: usize, expected: &str) -> bool {
    if symbol(tokens, index, '(') {
        index += 1;
    }
    loop {
        if word(tokens, index) == Some(expected) || symbol(tokens, index, '*') {
            return true;
        }
        if word(tokens, index).is_none() {
            return false;
        }
        index += 1;
        if word(tokens, index) == Some("as") {
            index += 2;
        }
        if symbol(tokens, index, ',') {
            index += 1;
            continue;
        }
        return false;
    }
}

fn module_end(tokens: &[Token], mut index: usize) -> Option<usize> {
    word(tokens, index)?;
    index += 1;
    while symbol(tokens, index, '.') {
        word(tokens, index + 1)?;
        index += 2;
    }
    Some(index)
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
