pub(super) struct ActiveLine {
    pub(super) source_line: usize,
    pub(super) text: String,
}

pub(super) fn active_lines(evidence: &str) -> Vec<ActiveLine> {
    let text = evidence.to_ascii_lowercase();
    let mut lines = Vec::new();
    let mut start = 0;
    for (source_line, fragment) in text.split_inclusive('\n').enumerate() {
        if super::sentinel_handoff::active_result_line(&text, start) {
            if let Some(text) = normalize(fragment.trim()) {
                lines.push(ActiveLine { source_line, text });
            }
        }
        start += fragment.len();
    }
    lines
}

fn normalize(line: &str) -> Option<String> {
    let mut line = line.trim();
    loop {
        let next = strip_container(line);
        if next != line {
            line = next;
            continue;
        }
        if line.strip_prefix("[ ]").is_some() {
            return None;
        }
        if let Some(next) = line
            .strip_prefix("[x]")
            .or_else(|| line.strip_prefix("[X]"))
        {
            line = next.trim_start();
            continue;
        }
        return Some(line.to_owned());
    }
}

fn strip_container(line: &str) -> &str {
    line.strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "))
        .or_else(|| strip_ordered(line))
        .map(str::trim_start)
        .unwrap_or(line)
}

fn strip_ordered(line: &str) -> Option<&str> {
    let digits = line.trim_start_matches(|character: char| character.is_ascii_digit());
    (digits.len() != line.len())
        .then_some(digits)
        .and_then(|tail| tail.strip_prefix(". ").or_else(|| tail.strip_prefix(") ")))
}
