pub(super) fn normalized_policy_text(text: &str) -> String {
    let lines = text.lines().collect::<Vec<_>>();
    let mut historical_level = None;
    let mut fence = None;
    let mut visible = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let structural_line = markdown_structure(line);
        if let Some((marker, minimum)) = fence {
            if structural_line
                .and_then(fence_delimiter)
                .is_some_and(|(candidate, count, rest)| {
                    candidate == marker && count >= minimum && rest.trim().is_empty()
                })
            {
                fence = None;
            }
            index += 1;
            continue;
        }
        if let Some((marker, count, _)) = structural_line.and_then(fence_delimiter) {
            visible.push("<markdown-boundary>".to_owned());
            fence = Some((marker, count));
            index += 1;
            continue;
        }
        let setext_level = lines.get(index + 1).and_then(|next| setext_level(next));
        let heading = structural_line.and_then(atx_heading).or_else(|| {
            setext_level.and_then(|level| {
                structural_line
                    .filter(|line| !line.is_empty())
                    .map(|line| (level, line.trim()))
            })
        });
        if let Some((level, heading)) = heading {
            if historical_level.is_some_and(|historical| level <= historical) {
                historical_level = None;
            }
            if historical_level.is_none() && is_historical_heading(heading) {
                historical_level = Some(level);
            }
            visible.push(format!("<markdown-heading> {heading} </markdown-heading>"));
            index += usize::from(setext_level.is_some()) + 1;
            continue;
        }
        if structural_line.is_none() {
            visible.push("<markdown-boundary>".to_owned());
        } else if historical_level.is_none() {
            visible.push(line.to_owned());
        }
        index += 1;
    }
    visible
        .join(" ")
        .to_ascii_lowercase()
        .replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn fence_delimiter(line: &str) -> Option<(char, usize, &str)> {
    let marker = line.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let count = line
        .chars()
        .take_while(|candidate| *candidate == marker)
        .count();
    (count >= 3).then(|| (marker, count, &line[count..]))
}

fn atx_heading(line: &str) -> Option<(usize, &str)> {
    let level = line
        .chars()
        .take_while(|candidate| *candidate == '#')
        .count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = &line[level..];
    (rest.is_empty() || rest.starts_with(char::is_whitespace))
        .then(|| (level, rest.trim().trim_end_matches('#').trim_end()))
}

fn setext_level(line: &str) -> Option<usize> {
    let line = markdown_structure(line)?.trim();
    if !line.is_empty() && line.chars().all(|character| character == '=') {
        Some(1)
    } else if !line.is_empty() && line.chars().all(|character| character == '-') {
        Some(2)
    } else {
        None
    }
}

fn markdown_structure(line: &str) -> Option<&str> {
    let mut columns = 0;
    for (index, character) in line.char_indices() {
        match character {
            ' ' => columns += 1,
            '\t' => columns += 4 - columns % 4,
            _ => return (columns <= 3).then(|| &line[index..]),
        }
        if columns > 3 {
            return None;
        }
    }
    Some("")
}

fn is_historical_heading(heading: &str) -> bool {
    let heading = heading.to_ascii_lowercase();
    let mut parts = heading.splitn(2, char::is_whitespace);
    let first = parts.next().unwrap_or_default();
    let unnumbered = parts.next().filter(|_| {
        first.chars().any(|character| character.is_ascii_digit())
            && first.chars().all(|character| {
                character.is_ascii_digit() || matches!(character, '.' | '(' | ')' | ':' | '-')
            })
    });
    let title = unnumbered.unwrap_or(&heading);
    matches!(title, "history" | "historical")
        || title.starts_with("history ")
        || title.starts_with("historical ")
}
