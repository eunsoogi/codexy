#[derive(Clone, Copy)]
pub(super) struct OpenBlock {
    kind: MarkdownBlock,
    list_continuation: Option<usize>,
}

impl OpenBlock {
    pub(super) fn new(kind: MarkdownBlock, list_continuation: Option<usize>) -> Self {
        Self {
            kind,
            list_continuation,
        }
    }

    pub(super) fn closes(self, candidate: Option<&str>, raw_line: &str) -> bool {
        match self.kind {
            MarkdownBlock::Fence(marker, length) => {
                candidate.is_some_and(|line| closes_fence(line, marker, length))
            }
            MarkdownBlock::Comment => raw_line.contains("-->"),
            MarkdownBlock::Html(end) => html_block_ends(end, raw_line.trim_start()),
        }
    }

    pub(super) fn ends_with_list_item(self, raw_line: &str) -> bool {
        self.list_continuation
            .is_some_and(|indent| !raw_line.trim().is_empty() && leading_indent(raw_line) < indent)
    }
}

#[derive(Clone, Copy)]
pub(super) enum MarkdownBlock {
    Fence(u8, usize),
    Comment,
    Html(HtmlEnd),
}

#[derive(Clone, Copy)]
pub(super) enum HtmlEnd {
    TypeOne(&'static str),
    Marker(&'static str),
    Blank,
}

fn html_block_ends(end: HtmlEnd, line: &str) -> bool {
    match end {
        HtmlEnd::TypeOne(tag) => line.to_ascii_lowercase().contains(&format!("</{tag}>")),
        HtmlEnd::Marker(marker) => line.contains(marker),
        HtmlEnd::Blank => line.is_empty(),
    }
}

fn closes_fence(line: &str, marker: u8, minimum: usize) -> bool {
    let length = line
        .as_bytes()
        .iter()
        .take_while(|byte| **byte == marker)
        .count();
    length >= minimum && line[length..].trim().is_empty()
}

fn leading_indent(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}
