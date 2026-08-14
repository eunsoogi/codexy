pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut end = None;
    text.lines().map(|line| strip(line, &mut end)).collect()
}

fn strip(line: &str, end: &mut Option<&'static str>) -> Option<String> {
    if let Some(delimiter) = *end {
        if line.trim() == delimiter {
            *end = None;
        }
        return Some(String::new());
    }
    if comment_before_opener(line) {
        return Some(line.to_owned());
    }
    for (opening, closing) in [("@'", "'@"), ("@\"", "\"@")] {
        let mut offset = 0;
        while let Some(relative) = line[offset..].find(opening) {
            let index = offset + relative;
            if here_opener_allowed(&line[..index]) {
                *end = Some(closing);
                return Some(line[..index].to_owned());
            }
            offset = index + opening.len();
        }
    }
    Some(line.to_owned())
}

fn here_opener_allowed(prefix: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    for character in prefix.chars() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '`' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        }
    }
    quote.is_none()
}

fn comment_before_opener(line: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    for character in line.chars() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '`' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '#' {
            return true;
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        }
    }
    false
}
