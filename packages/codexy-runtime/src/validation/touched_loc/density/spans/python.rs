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
    for delimiter in ["\"\"\"", "'''"] {
        if let Some(index) = line.find(delimiter) {
            if line[..index].contains('#') || quoted_before(&line[..index]) {
                continue;
            }
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
    }
    Some(line.to_owned())
}

fn quoted_before(prefix: &str) -> bool {
    let mut quote = None;
    for character in prefix.chars() {
        quote = match quote {
            Some(delimiter) if character == delimiter => None,
            Some(delimiter) => Some(delimiter),
            None if matches!(character, '\'' | '"') => Some(character),
            None => None,
        };
    }
    quote.is_some()
}
