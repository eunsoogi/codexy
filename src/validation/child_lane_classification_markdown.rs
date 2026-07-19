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
    let mut end = None;
    for raw_line in lines.iter().take(index) {
        if let Some(marker) = end {
            if html_block_ends(marker, raw_line.trim_start()) {
                end = None;
            }
            continue;
        }
        end = html_block_candidate(raw_line).and_then(raw_html_end);
    }
    end.is_some()
}

#[derive(Clone, Copy)]
enum HtmlEnd {
    TypeOne,
    Marker(&'static str),
    Blank,
}

fn html_block_ends(end: HtmlEnd, line: &str) -> bool {
    match end {
        HtmlEnd::TypeOne => ["</pre>", "</script>", "</style>"]
            .iter()
            .any(|marker| line.to_ascii_lowercase().contains(marker)),
        HtmlEnd::Marker(marker) => line.contains(marker),
        HtmlEnd::Blank => line.is_empty(),
    }
}

fn html_block_candidate(line: &str) -> Option<&str> {
    let spaces = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    (spaces <= 3 && !line.starts_with('\t')).then(|| &line[spaces..])
}

fn raw_html_end(line: &str) -> Option<HtmlEnd> {
    let lower = line.to_ascii_lowercase();
    if ["pre", "script", "style"]
        .iter()
        .any(|tag| starts_with_tag(&lower, tag))
        && !["</pre>", "</script>", "</style>"]
            .iter()
            .any(|end| lower.contains(end))
    {
        return Some(HtmlEnd::TypeOne);
    }
    if lower.starts_with("<?") && !lower.contains("?>") {
        return Some(HtmlEnd::Marker("?>"));
    }
    if lower.starts_with("<![cdata[") && !lower.contains("]]>") {
        return Some(HtmlEnd::Marker("]]>"));
    }
    if is_declaration_start(line) && !line.contains('>') {
        return Some(HtmlEnd::Marker(">"));
    }
    (is_block_tag(&lower) || is_complete_tag_line(line)).then_some(HtmlEnd::Blank)
}

fn is_declaration_start(line: &str) -> bool {
    line.as_bytes().get(2).is_some_and(u8::is_ascii_uppercase) && line.starts_with("<!")
}

fn is_complete_tag_line(line: &str) -> bool {
    let Some(inner) = line
        .trim()
        .strip_prefix('<')
        .and_then(|line| line.strip_suffix('>'))
    else {
        return false;
    };
    let inner = inner.strip_prefix('/').unwrap_or(inner);
    let name_length = inner
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '-')
        .count();
    let Some(first) = inner.chars().next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() || name_length == 0 {
        return false;
    }
    let suffix = &inner[name_length..];
    suffix.is_empty() || suffix.starts_with(char::is_whitespace) || suffix == "/"
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
    "address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h1|h2|h3|h4|h5|h6|head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|nav|noframes|ol|optgroup|option|p|param|section|source|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul"
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
