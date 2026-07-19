use regex::Regex;
use std::sync::OnceLock;

mod block;
mod list_item;

use block::{HtmlEnd, MarkdownBlock, OpenBlock};

pub(super) fn is_in_non_rendering_block(lines: &[&str], index: usize) -> bool {
    let mut state: Option<OpenBlock> = None;
    let mut paragraph_open = false;
    for raw_line in lines.iter().take(index) {
        let candidate = html_block_candidate(raw_line);
        if let Some(open) = state {
            if open.ends_with_list_item(raw_line) {
                state = None;
            } else {
                if open.closes(candidate, raw_line) {
                    state = None;
                }
                continue;
            }
        }
        let Some(line) = candidate else {
            paragraph_open = line_opens_or_continues_paragraph(raw_line, paragraph_open);
            continue;
        };
        let list_continuation = list_item::continuation_indent(line);
        let block_line = list_item::content(line).unwrap_or(line);
        if let Some((marker, length)) = opens_fence(block_line) {
            state = Some(OpenBlock::new(
                MarkdownBlock::Fence(marker, length),
                list_continuation,
            ));
            paragraph_open = false;
            continue;
        }
        if block_line.starts_with("<!--") {
            state = (!block_line.contains("-->"))
                .then_some(OpenBlock::new(MarkdownBlock::Comment, list_continuation));
            paragraph_open = false;
            continue;
        }
        if let Some(end) = raw_html_start(block_line, !paragraph_open) {
            state = end.map(|end| OpenBlock::new(MarkdownBlock::Html(end), list_continuation));
            paragraph_open = false;
            continue;
        }
        paragraph_open = line_opens_or_continues_paragraph(raw_line, paragraph_open);
    }
    state.is_some_and(|open| {
        !lines
            .get(index)
            .is_some_and(|line| open.ends_with_list_item(line))
    })
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

fn html_block_candidate(line: &str) -> Option<&str> {
    let spaces = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    (spaces <= 3 && !line.starts_with('\t')).then(|| &line[spaces..])
}

fn raw_html_start(line: &str, allow_type_seven: bool) -> Option<Option<HtmlEnd>> {
    let lower = line.to_ascii_lowercase();
    if let Some(tag) = ["pre", "script", "style", "textarea"]
        .iter()
        .copied()
        .find(|tag| starts_with_tag(&lower, tag))
    {
        let open = !lower.contains(&format!("</{tag}>"));
        return Some(open.then_some(HtmlEnd::TypeOne(tag)));
    }
    if lower.starts_with("<?") {
        return Some((!lower.contains("?>")).then_some(HtmlEnd::Marker("?>")));
    }
    if line.starts_with("<![CDATA[") {
        return Some((!line.contains("]]>")).then_some(HtmlEnd::Marker("]]>")));
    }
    if is_declaration_start(line) {
        return Some((!line.contains('>')).then_some(HtmlEnd::Marker(">")));
    }
    if is_block_tag(&lower) || allow_type_seven && is_complete_tag_line(line) {
        return Some(Some(HtmlEnd::Blank));
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

fn line_opens_or_continues_paragraph(line: &str, paragraph_open: bool) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if is_indented_code_line(line) {
        return paragraph_open;
    }
    let candidate = html_block_candidate(line).unwrap_or(trimmed);
    if html_block_candidate(line).and_then(opens_fence).is_some() {
        return false;
    }
    if starts_container_block(candidate, paragraph_open) {
        return false;
    }
    if is_thematic_break(candidate) {
        return false;
    }
    let hashes = candidate.chars().take_while(|ch| *ch == '#').count();
    let heading_suffix = &candidate[hashes..];
    let atx_heading = (1..=6).contains(&hashes)
        && (heading_suffix.is_empty() || heading_suffix.starts_with(char::is_whitespace));
    let setext_heading = candidate.len() > 0
        && candidate
            .chars()
            .all(|character| matches!(character, '=' | '-' | ' ' | '\t'))
        && candidate.contains(['=', '-']);
    !atx_heading && !setext_heading
}

fn starts_container_block(line: &str, paragraph_open: bool) -> bool {
    if line.starts_with('>') {
        return true;
    }
    let marker_end = line.find(char::is_whitespace).unwrap_or(line.len());
    let marker = &line[..marker_end];
    let has_list_content = !line[marker_end..].trim().is_empty();
    let list_marker_is_valid = !paragraph_open || has_list_content;
    (matches!(marker, "-" | "+" | "*") && list_marker_is_valid)
        || marker.strip_suffix(['.', ')']).is_some_and(|number| {
            !number.is_empty()
                && number.len() <= 9
                && number.chars().all(|ch| ch.is_ascii_digit())
                && list_marker_is_valid
                && (!paragraph_open || number.parse() == Ok(1))
        })
}

fn is_thematic_break(line: &str) -> bool {
    let mut marker = None;
    let mut count = 0;
    for ch in line.chars().filter(|ch| !ch.is_whitespace()) {
        if !matches!(ch, '*' | '-' | '_') || marker.is_some_and(|marker| marker != ch) {
            return false;
        }
        marker = Some(ch);
        count += 1;
    }
    count >= 3
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
