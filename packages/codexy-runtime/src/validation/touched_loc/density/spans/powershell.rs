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
    for (opening, closing) in [("@'", "'@"), ("@\"", "\"@")] {
        if let Some(index) = line.find(opening) {
            *end = Some(closing);
            return Some(line[..index].to_owned());
        }
    }
    Some(line.to_owned())
}
