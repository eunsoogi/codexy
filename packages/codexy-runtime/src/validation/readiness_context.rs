pub(super) fn active_line(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let content = line
        .strip_prefix(['-', '*', '+'])
        .map(str::trim_start)
        .or_else(|| {
            super::child_handoff_readiness_claims::strip_ordered_list_marker(line)
                .map(str::trim_start)
        })
        .unwrap_or(line);
    (!content.starts_with("[ ]")).then_some(line)
}

pub(super) fn is_stale(segment: &str) -> bool {
    let segment = segment.trim_start_matches(|character: char| {
        character.is_whitespace() || matches!(character, '-' | '*' | '+')
    });
    let segment = super::child_handoff_readiness_claims::strip_ordered_list_marker(segment)
        .map(str::trim_start)
        .unwrap_or(segment);
    [
        "historical example",
        "previous example",
        "prior example",
        "fallback lane",
        "for example",
        "example",
    ]
    .iter()
    .any(|prefix| {
        segment.strip_prefix(prefix).is_some_and(|rest| {
            rest.chars()
                .next()
                .is_none_or(|character| !character.is_ascii_alphanumeric())
        })
    })
}

pub(super) fn current_text(text: &str) -> String {
    let lines: Vec<_> = text.lines().collect();
    let mut current = String::new();
    let mut stale_heading = false;
    let mut stale_label = false;
    for (index, raw_line) in lines.iter().enumerate() {
        let trimmed = raw_line.trim();
        if trimmed.starts_with('#') {
            stale_heading = is_stale(trimmed.trim_start_matches('#').trim_start());
            stale_label = false;
            continue;
        }
        if trimmed.is_empty() {
            stale_label = false;
            if !stale_heading {
                current.push('\n');
            }
            continue;
        }
        let current_suffix = current_lane_suffix(trimmed);
        if stale_heading || stale_label {
            let Some(suffix) = current_suffix else {
                continue;
            };
            stale_heading = false;
            stale_label = false;
            current.push_str(suffix);
            current.push('\n');
            continue;
        }
        if is_stale(trimmed) {
            if let Some(suffix) = current_suffix {
                current.push_str(suffix);
                current.push('\n');
            } else {
                stale_label = trimmed.ends_with(':');
            }
            continue;
        }
        let Some(line) = active_line(raw_line) else {
            continue;
        };
        if blocked_standalone_line(line, &lines[index + 1..]) {
            continue;
        }
        for segment in line
            .split_inclusive(['.', ';'])
            .filter(|segment| !is_stale(segment))
        {
            current.push_str(segment);
        }
        current.push('\n');
    }
    current
}

fn current_lane_suffix(line: &str) -> Option<&str> {
    ["current lane:", "current lane is "]
        .iter()
        .filter_map(|marker| {
            line.match_indices(marker)
                .map(|(index, _)| index)
                .find(|index| {
                    let prefix = line[..*index].trim_end();
                    prefix.is_empty() || prefix.ends_with(['.', ';', '!', '?'])
                })
        })
        .min()
        .map(|index| &line[index..])
}

fn blocked_standalone_line(line: &str, following: &[&str]) -> bool {
    let content = line
        .strip_prefix(['-', '*', '+'])
        .map(str::trim_start)
        .or_else(|| {
            super::child_handoff_readiness_claims::strip_ordered_list_marker(line)
                .map(str::trim_start)
        })
        .unwrap_or(line);
    let content = content
        .strip_prefix("[x]")
        .or_else(|| content.strip_prefix("[X]"))
        .unwrap_or(content)
        .trim()
        .trim_end_matches('.');
    super::child_handoff_readiness_claims::ready_label_phrases().contains(&content)
        && super::child_handoff_readiness_claims::has_next_non_claim_bullet(following)
}
