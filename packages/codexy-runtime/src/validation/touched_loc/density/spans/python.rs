pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut end = None;
    text.lines().map(|line| strip(line, &mut end)).collect()
}

fn strip(line: &str, end: &mut Option<&'static str>) -> Option<String> {
    let mut visible = String::new();
    let mut remainder = line;
    loop {
        if let Some(delimiter) = *end {
            if let Some(index) = unescaped_delimiter(remainder, delimiter) {
                remainder = &remainder[index + delimiter.len()..];
                *end = None;
                continue;
            }
            return Some(visible);
        }
        if let Some((index, delimiter)) = triple_opener(remainder) {
            visible.push_str(&remainder[..index]);
            remainder = &remainder[index + delimiter.len()..];
            if let Some(close) = unescaped_delimiter(remainder, delimiter) {
                remainder = &remainder[close + delimiter.len()..];
                continue;
            }
            *end = Some(delimiter);
            return Some(visible);
        }
        visible.push_str(remainder);
        return Some(visible);
    }
}

fn triple_opener(line: &str) -> Option<(usize, &'static str)> {
    let mut offset = 0;
    while offset < line.len() {
        let (relative, delimiter) = ["\"\"\"", "'''"]
            .into_iter()
            .filter_map(|delimiter| {
                unescaped_delimiter(&line[offset..], delimiter).map(|index| (index, delimiter))
            })
            .min_by_key(|(index, _)| *index)?;
        let index = offset + relative;
        if triple_opener_allowed(&line[..index]) {
            return Some((index, delimiter));
        }
        offset = index + delimiter.len();
    }
    None
}

fn unescaped_delimiter(text: &str, delimiter: &str) -> Option<usize> {
    let mut offset = 0;
    while let Some(relative) = text[offset..].find(delimiter) {
        let index = offset + relative;
        if !escaped_at(text, index) {
            return Some(index);
        }
        offset = index + delimiter.len();
    }
    None
}

fn escaped_at(text: &str, index: usize) -> bool {
    text[..index]
        .chars()
        .rev()
        .take_while(|character| *character == '\\')
        .count()
        % 2
        == 1
}

fn triple_opener_allowed(prefix: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    for character in prefix.chars() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if character == '#' {
            return false;
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        }
    }
    quote.is_none()
}
