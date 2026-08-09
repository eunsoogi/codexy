#[derive(Default)]
pub(crate) struct ActiveMarkdown {
    comment: (bool, bool),
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
        let active = self.without_comment(line);
        if raw_html_block(&active) {
            return Err("raw HTML blocks cannot supply contract content".into());
        }
        Ok(active)
    }

    fn without_comment(&mut self, line: &str) -> String {
        let indentation = line.len() - line.trim_start_matches(' ').len();
        let block = indentation <= 3 && line[indentation..].starts_with("<!--");
        let mut active = String::new();
        let mut rest = line;
        loop {
            if self.comment.0 {
                let Some(end) = rest.find("-->") else {
                    return active;
                };
                rest = &rest[end + 3..];
                let block = self.comment.1;
                self.comment = (false, false);
                if block {
                    return String::new();
                }
            } else if let Some(start) = rest.find("<!--") {
                active.push_str(&rest[..start]);
                rest = &rest[start + 4..];
                self.comment = (true, block);
            } else {
                active.push_str(rest);
                return active;
            }
        }
    }
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
