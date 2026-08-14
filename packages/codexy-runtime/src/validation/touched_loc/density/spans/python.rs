pub(super) fn lines(text: &str) -> Vec<Option<String>> {
    let mut end = None;
    text.lines().map(|line| strip(line, &mut end)).collect()
}

fn strip(line: &str, end: &mut Option<&'static str>) -> Option<String> {
    if let Some(delimiter) = *end {
        if let Some(index) = line.find(delimiter) {
            *end = None;
            return Some(line[index + delimiter.len()..].to_owned());
        }
        return Some(String::new());
    }
    if let Some((index, delimiter)) = triple_opener(line) {
        let tail = &line[index + delimiter.len()..];
        if let Some(close) = tail.find(delimiter) {
            return Some(format!(
                "{}{}",
                &line[..index],
                &tail[close + delimiter.len()..]
            ));
        }
        *end = Some(delimiter);
        return Some(line[..index].to_owned());
    }
    Some(line.to_owned())
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
