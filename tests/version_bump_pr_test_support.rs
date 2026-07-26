pub(super) fn has_trimmed_line(text: &str, expected: &str) -> bool {
    text.lines().map(str::trim).any(|line| line == expected)
}

pub(super) fn has_trimmed_line_start(text: &str, expected: &str) -> bool {
    text.lines()
        .map(str::trim)
        .any(|line| line.starts_with(expected))
}

pub(super) fn trimmed_line_position(text: &str, expected: &str) -> Option<usize> {
    text.lines()
        .map(str::trim)
        .position(|line| line.starts_with(expected))
}

pub(super) fn markdown_headings(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|line| line.starts_with("## "))
        .collect()
}

pub(super) fn markdown_section_lines<'a>(text: &'a str, heading: &str) -> Vec<&'a str> {
    text.lines()
        .skip_while(|line| *line != heading)
        .skip(1)
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| !line.is_empty())
        .collect()
}
