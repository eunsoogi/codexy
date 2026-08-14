pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut end = None;
    text.lines().map(|line| strip(line, &mut end)).collect()
}

fn strip(line: &str, end: &mut Option<&'static str>) -> Option<String> {
    let mut visible = String::new();
    let mut remainder = line;
    loop {
        if let Some(delimiter) = *end {
            if let Some(index) = remainder.find(delimiter) {
                remainder = &remainder[index + delimiter.len()..];
                *end = None;
                continue;
            }
            return Some(visible);
        }
        if let Some((index, delimiter)) = triple_opener(remainder) {
            visible.push_str(&remainder[..index]);
            remainder = &remainder[index + delimiter.len()..];
            if let Some(close) = remainder.find(delimiter) {
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
                line[offset..]
                    .find(delimiter)
                    .map(|index| (index, delimiter))
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
