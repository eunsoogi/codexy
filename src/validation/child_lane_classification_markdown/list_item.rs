pub(super) fn content(line: &str) -> Option<&str> {
    let marker_end = line.find(char::is_whitespace).unwrap_or(line.len());
    let marker = &line[..marker_end];
    let list_item = matches!(marker, "-" | "+" | "*")
        || marker.strip_suffix(['.', ')']).is_some_and(|number| {
            !number.is_empty() && number.len() <= 9 && number.chars().all(|ch| ch.is_ascii_digit())
        });
    list_item.then(|| line[marker_end..].trim_start_matches([' ', '\t']))
}
