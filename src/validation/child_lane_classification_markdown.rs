use regex::Regex;
use std::sync::OnceLock;

pub(super) fn is_in_non_rendering_block(lines: &[&str], index: usize) -> bool {
    is_inside_fenced_code_block(lines, index)
        || is_inside_html_comment(lines, index)
        || is_inside_raw_html_block(lines, index)
}

pub(super) fn is_indented_code_line(line: &str) -> bool {
    let mut columns = 0;
    for byte in line.bytes() {
        match byte {
            b' ' => columns += 1,
            b'\t' => columns += 4 - columns % 4,
            _ => break,
        }
        if columns >= 4 {
            return true;
        }
    }
    false
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
    for raw_line in lines.iter().take(index) {
        if open {
            if raw_line.contains("-->") {
                open = false;
            }
            continue;
        }
        if let Some(line) = html_block_candidate(raw_line).filter(|line| line.starts_with("<!--")) {
            open = !line.contains("-->");
        }
    }
    open
}

fn is_inside_raw_html_block(lines: &[&str], index: usize) -> bool {
    let mut end = None;
    let mut paragraph_open = false;
    for raw_line in lines.iter().take(index) {
        if let Some(marker) = end {
            if html_block_ends(marker, raw_line.trim_start()) {
                end = None;
            }
            continue;
        }
        end = html_block_candidate(raw_line).and_then(|line| raw_html_end(line, !paragraph_open));
        if end.is_some() {
            paragraph_open = false;
            continue;
        }
        paragraph_open = line_opens_or_continues_paragraph(raw_line);
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

fn raw_html_end(line: &str, allow_type_seven: bool) -> Option<HtmlEnd> {
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
    if line.starts_with("<![CDATA[") && !line.contains("]]>") {
        return Some(HtmlEnd::Marker("]]>"));
    }
    if is_declaration_start(line) && !line.contains('>') {
        return Some(HtmlEnd::Marker(">"));
    }
    if is_block_tag(&lower) || allow_type_seven && is_complete_tag_line(line) {
        return Some(HtmlEnd::Blank);
    }
    None
}

fn is_declaration_start(line: &str) -> bool {
    line.as_bytes().get(2).is_some_and(u8::is_ascii_uppercase) && line.starts_with("<!")
}

fn is_complete_tag_line(line: &str) -> bool {
    static COMPLETE_TAG: OnceLock<Regex> = OnceLock::new();
    COMPLETE_TAG
        .get_or_init(|| {
            Regex::new(
                r#"^(?:<[A-Za-z][A-Za-z0-9-]*(?:[ \t]+[A-Za-z_:][A-Za-z0-9_.:-]*(?:[ \t]*=[ \t]*(?:[^ \t"'=<>`]+|'[^']*'|"[^"]*"))?)*[ \t]*/?>|</[A-Za-z][A-Za-z0-9-]*[ \t]*>)[ \t]*$"#,
            )
            .expect("complete GFM tag regex")
        })
        .is_match(line)
}

fn starts_with_tag(line: &str, tag: &str) -> bool {
    line.strip_prefix('<')
        .and_then(|value| value.strip_prefix(tag))
        .is_some_and(|suffix| {
            suffix.is_empty() || suffix.starts_with(char::is_whitespace) || suffix.starts_with('>')
        })
}

fn is_block_tag(line: &str) -> bool {
    let line = line.strip_prefix("</").or_else(|| line.strip_prefix('<'));
    let Some(line) = line else {
        return false;
    };
    let tag_length = line
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric())
        .count();
    let tag = &line[..tag_length];
    let known = "address|article|aside|base|basefont|blockquote|body|caption|center|col|colgroup|dd|details|dialog|dir|div|dl|dt|fieldset|figcaption|figure|footer|form|frame|frameset|h1|h2|h3|h4|h5|h6|head|header|hr|html|iframe|legend|li|link|main|menu|menuitem|nav|noframes|ol|optgroup|option|p|param|section|source|summary|table|tbody|td|tfoot|th|thead|title|tr|track|ul"
        .split('|')
        .any(|candidate| candidate == tag);
    let suffix = &line[tag_length..];
    known
        && (suffix.is_empty()
            || suffix.starts_with(char::is_whitespace)
            || suffix.starts_with('>')
            || suffix.starts_with("/>"))
}

fn line_opens_or_continues_paragraph(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() || is_indented_code_line(line) {
        return false;
    }
    let candidate = html_block_candidate(line).unwrap_or(trimmed);
    if fence_candidate(line).and_then(opens_fence).is_some() {
        return false;
    }
    let atx_heading = candidate.starts_with('#')
        && candidate
            .trim_start_matches('#')
            .starts_with(char::is_whitespace);
    let setext_heading = candidate.len() > 0
        && candidate
            .chars()
            .all(|character| matches!(character, '=' | '-' | ' ' | '\t'))
        && candidate.contains(['=', '-']);
    !atx_heading && !setext_heading
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
