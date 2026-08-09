#[derive(Default)]
pub(crate) struct ActiveMarkdown {
    comment: (bool, bool),
    code: Option<usize>,
}

pub(crate) const TYPE6_BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

const TYPE1_BLOCK_TAGS: &[&str] = &["pre", "script", "style", "textarea"];

impl ActiveMarkdown {
    pub(crate) fn line(&mut self, line: &str, fenced: bool) -> Result<String, String> {
        if fenced {
            return Ok(line.into());
        }
        let active = self.without_comment(line)?;
        if raw_html_block(&active) {
            return Err("raw HTML blocks cannot supply contract content".into());
        }
        Ok(active)
    }

    pub(crate) fn finish(&self) -> Result<(), String> {
        self.code
            .is_none()
            .then_some(())
            .ok_or_else(|| "unbalanced inline code span".into())
    }

    fn without_comment(&mut self, line: &str) -> Result<String, String> {
        let indentation = line.len() - line.trim_start_matches(' ').len();
        let block = indentation <= 3 && line[indentation..].starts_with("<!--");
        let mut active = String::new();
        let mut rest = line;
        loop {
            if self.comment.0 {
                let Some(end) = rest.find("-->") else {
                    return Ok(active);
                };
                rest = &rest[end + 3..];
                let block = self.comment.1;
                self.comment = (false, false);
                if block {
                    return Ok(String::new());
                }
            } else if self.code.is_some() {
                let Some((start, length)) = backtick_run(rest, true) else {
                    active.push_str(rest);
                    return Ok(active);
                };
                let end = start + length;
                active.push_str(&rest[..end]);
                rest = &rest[end..];
                if self.code == Some(length) {
                    self.code = None;
                }
            } else if let Some((start, length)) = backtick_run(rest, false) {
                let comment = rest.find("<!--");
                if comment.is_some_and(|comment| comment < start) {
                    active.push_str(&rest[..comment.unwrap()]);
                    rest = &rest[comment.unwrap() + 4..];
                    self.comment = (true, block);
                    continue;
                }
                let end = start + length;
                active.push_str(&rest[..end]);
                rest = &rest[end..];
                self.code = Some(length);
            } else if let Some(start) = rest.find("<!--") {
                active.push_str(&rest[..start]);
                rest = &rest[start + 4..];
                self.comment = (true, block);
            } else {
                active.push_str(rest);
                return Ok(active);
            }
        }
    }
}

fn backtick_run(text: &str, inside_code: bool) -> Option<(usize, usize)> {
    let start = text.find('`')?;
    let length = text[start..]
        .chars()
        .take_while(|character| *character == '`')
        .count();
    let indentation = text.len() - text.trim_start_matches(' ').len();
    if !inside_code && start == indentation && opening_fence(text).is_some() {
        return None;
    }
    Some((start, length))
}

pub(crate) fn opening_fence(line: &str) -> Option<(char, usize)> {
    let indentation = line.len() - line.trim_start_matches(' ').len();
    if indentation > 3 {
        return None;
    }
    let trimmed = &line[indentation..];
    let marker = trimmed.chars().next()?;
    let length = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    let info = &trimmed[length..];
    (matches!(marker, '`' | '~') && length >= 3 && (marker == '~' || !info.contains('`')))
        .then_some((marker, length))
}

fn raw_html_block(line: &str) -> bool {
    let indentation = line.len() - line.trim_start_matches(' ').len();
    if indentation > 3 {
        return false;
    }
    let Some(rest) = line[indentation..].strip_prefix('<') else {
        return false;
    };
    if rest.starts_with("!--") {
        return false;
    }
    raw_declaration(rest) || raw_tag(rest)
}

fn raw_declaration(rest: &str) -> bool {
    matches!(rest.as_bytes().first(), Some(b'!' | b'?'))
}

fn raw_tag(rest: &str) -> bool {
    let tag = rest.strip_prefix('/').unwrap_or(rest);
    let length = tag
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '-')
        .count();
    let (name, suffix) = tag.split_at(length);
    let block = in_tags(name, TYPE1_BLOCK_TAGS) || in_tags(name, TYPE6_BLOCK_TAGS);
    let continuation = suffix
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_whitespace() || character == '>')
        || suffix.starts_with("/>");
    !name.is_empty()
        && ((block && suffix.is_empty()) || continuation)
        && (block || suffix.trim_end().ends_with('>'))
}

fn in_tags(name: &str, tags: &[&str]) -> bool {
    tags.iter().any(|tag| name.eq_ignore_ascii_case(tag))
}
