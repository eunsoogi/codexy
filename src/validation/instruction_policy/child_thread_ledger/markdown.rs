pub(super) fn normalized_whitespace(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let mut normalized = String::new();
    let mut index = 0;
    let mut fence = None;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        if let Some((marker, length)) = fence {
            if fence_run(line).is_some_and(|(candidate, run, rest)| {
                candidate == marker && run >= length && rest.trim().is_empty()
            }) {
                fence = None;
            }
            index += 1;
            continue;
        }
        if let Some((marker, length, _)) = fence_run(line) {
            fence = Some((marker, length));
            index += 1;
            continue;
        }
        let setext = lines
            .get(index + 1)
            .is_some_and(|next| is_setext_underline(next.trim()));
        if trimmed.starts_with('#') || setext {
            normalized.push_str(" <markdown-heading> ");
            normalized.push_str(trimmed.trim_start_matches('#').trim());
            normalized.push_str(" </markdown-heading> ");
            index += usize::from(setext);
        } else {
            normalized.push_str(line);
            normalized.push(' ');
        }
        index += 1;
    }
    normalized
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn fence_run(line: &str) -> Option<(char, usize, &str)> {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    if indent > 3 {
        return None;
    }
    let candidate = &line[indent..];
    let marker = candidate.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let length = candidate
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (length >= 3).then(|| (marker, length, &candidate[length..]))
}

fn is_setext_underline(line: &str) -> bool {
    line.len() >= 3
        && (line.chars().all(|character| character == '=')
            || line.chars().all(|character| character == '-'))
}
