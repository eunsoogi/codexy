pub(super) fn content(line: &str) -> Option<&str> {
    let marker_end = line.find(char::is_whitespace).unwrap_or(line.len());
    is_marker(&line[..marker_end]).then(|| line[marker_end..].trim_start_matches([' ', '\t']))
}

pub(super) fn continuation_indent(line: &str) -> Option<usize> {
    let leading = line.bytes().take_while(|byte| *byte == b' ').count();
    let line = &line[leading..];
    let marker_end = line.find(char::is_whitespace).unwrap_or(line.len());
    is_marker(&line[..marker_end]).then(|| {
        let padding = line[marker_end..]
            .bytes()
            .take_while(|byte| *byte == b' ')
            .count();
        leading
            + marker_end
            + if (1..=4).contains(&padding) {
                padding
            } else {
                1
            }
    })
}

fn is_marker(marker: &str) -> bool {
    matches!(marker, "-" | "+" | "*")
        || marker.strip_suffix(['.', ')']).is_some_and(|number| {
            !number.is_empty() && number.len() <= 9 && number.chars().all(|ch| ch.is_ascii_digit())
        })
}
