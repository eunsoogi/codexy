pub(super) fn content(line: &str) -> Option<&str> {
    let marker_end = line.find(char::is_whitespace).unwrap_or(line.len());
    is_marker(&line[..marker_end]).then(|| line[marker_end..].trim_start_matches([' ', '\t']))
}

pub(super) fn continuation_indent(line: &str) -> Option<usize> {
    let marker_end = line.find(char::is_whitespace)?;
    is_marker(&line[..marker_end]).then_some(marker_end + 1)
}

fn is_marker(marker: &str) -> bool {
    matches!(marker, "-" | "+" | "*")
        || marker.strip_suffix(['.', ')']).is_some_and(|number| {
            !number.is_empty() && number.len() <= 9 && number.chars().all(|ch| ch.is_ascii_digit())
        })
}
