pub(super) fn is_in_non_rendering_block(lines: &[&str], index: usize) -> bool {
    is_inside_fenced_code_block(lines, index)
        || is_inside_html_comment(lines, index)
        || is_inside_raw_html_block(lines, index)
}

pub(super) fn is_indented_code_line(line: &str) -> bool {
    line.starts_with('\t')
        || line
            .as_bytes()
            .iter()
            .take_while(|byte| **byte == b' ')
            .count()
            >= 4
}

fn is_inside_fenced_code_block(lines: &[&str], index: usize) -> bool {
    let mut open = None;
    for line in lines.iter().take(index) {
        let Some(candidate) = fence_candidate(line) else {
            continue;
        };
        match open {
            Some((marker, length)) if closes_fence(candidate, marker, length) => open = None,
            None => open = opens_fence(candidate),
            _ => {}
        }
    }
    open.is_some()
}

fn is_inside_html_comment(lines: &[&str], index: usize) -> bool {
    let mut open = false;
    for line in lines.iter().take(index) {
        let mut remaining = *line;
        loop {
            if open {
                let Some(end) = remaining.find("-->") else {
                    break;
                };
                open = false;
                remaining = &remaining[end + 3..];
            } else {
                let Some(start) = remaining.find("<!--") else {
                    break;
                };
                open = true;
                remaining = &remaining[start + 4..];
            }
        }
    }
    open
}

fn is_inside_raw_html_block(lines: &[&str], index: usize) -> bool {
    let mut end: Option<&'static str> = None;
    for line in lines.iter().take(index).map(|line| line.trim_start()) {
        if let Some(marker) = end {
            if marker.is_empty() && line.is_empty() || !marker.is_empty() && line.contains(marker) {
                end = None;
            }
            continue;
        }
        end = raw_html_end(line);
    }
    end.is_some()
}

fn raw_html_end(line: &str) -> Option<&'static str> {
    let lower = line.to_ascii_lowercase();
    for (tag, end) in [
        ("pre", "</pre>"),
        ("script", "</script>"),
        ("style", "</style>"),
        ("textarea", "</textarea>"),
    ] {
        if starts_with_tag(&lower, tag) && !lower.contains(end) {
            return Some(end);
        }
    }
    if lower.starts_with("<?") && !lower.contains("?>") {
        return Some("?>");
    }
    if lower.starts_with("<![cdata[") && !lower.contains("]]>") {
        return Some("]]>");
    }
    is_block_tag(&lower).then_some("")
}

fn starts_with_tag(line: &str, tag: &str) -> bool {
    line.strip_prefix('<')
        .and_then(|value| value.strip_prefix(tag))
        .is_some_and(|suffix| {
            suffix.is_empty()
                || suffix.starts_with(char::is_whitespace)
                || matches!(suffix.chars().next(), Some('>' | '/'))
        })
}

fn is_block_tag(line: &str) -> bool {
    let line = line.strip_prefix("</").or_else(|| line.strip_prefix('<'));
    let Some(line) = line else {
        return false;
    };
    let tag = line
        .split(|character: char| character.is_whitespace() || matches!(character, '>' | '/'))
        .next()
        .unwrap_or_default();
    "address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h1|h2|h3|h4|h5|h6|head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|nav|noframes|ol|optgroup|option|p|param|search|section|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul"
        .split('|')
        .any(|candidate| candidate == tag)
}

fn fence_candidate(line: &str) -> Option<&str> {
    let spaces = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    (spaces <= 3 && !line.starts_with('\t')).then(|| &line[spaces..])
}

fn opens_fence(line: &str) -> Option<(u8, usize)> {
    let marker = *line.as_bytes().first()?;
    if !matches!(marker, b'`' | b'~') {
        return None;
    }
    let length = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    (length >= 3 && (marker != b'`' || !line[length..].contains('`'))).then_some((marker, length))
}

fn closes_fence(line: &str, marker: u8, minimum: usize) -> bool {
    let length = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    length >= minimum && line[length..].trim().is_empty()
}
