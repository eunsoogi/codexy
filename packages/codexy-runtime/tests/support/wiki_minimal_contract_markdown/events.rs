use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use super::{CodeBlock, Document, Heading, InlineCode, Link, Text, table::TableBuilder};

struct HeadingBuilder {
    range: std::ops::Range<usize>,
    level: usize,
    text: String,
    literal: bool,
}
struct LinkBuilder {
    destination: String,
    label: String,
    literal: bool,
}

pub(super) fn parse(source: &str) -> Result<Document, String> {
    let mut document = Document {
        source: source.into(),
        length: source.len(),
        headings: Vec::new(),
        tables: Vec::new(),
        blocks: Vec::new(),
        links: Vec::new(),
        inline_code: Vec::new(),
        text: Vec::new(),
    };
    let mut heading = None;
    let mut link = None;
    let mut block = None;
    let mut html = None::<String>;
    let mut table = None;
    let mut image_depth: usize = 0;
    for (event, range) in Parser::new_ext(source, Options::ENABLE_TABLES).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(HeadingBuilder {
                    range,
                    level: level as usize,
                    text: String::new(),
                    literal: true,
                })
            }
            Event::End(TagEnd::Heading(_)) => {
                let heading = heading.take().ok_or("unbalanced heading")?;
                document.headings.push(Heading {
                    range: heading.range,
                    level: heading.level,
                    text: heading.text,
                    literal: heading.literal,
                });
            }
            Event::Start(Tag::Table(columns)) => {
                table = Some(TableBuilder::new(range.start, columns.len()))
            }
            Event::Start(Tag::TableHead) => table.as_mut().ok_or("orphan table header")?.row(),
            Event::Start(Tag::TableRow) => table.as_mut().ok_or("orphan table row")?.row(),
            Event::Start(Tag::TableCell) => table.as_mut().ok_or("orphan table cell")?.cell(),
            Event::End(TagEnd::TableCell) => table.as_mut().ok_or("orphan table cell")?.end_cell(),
            Event::End(TagEnd::TableHead) => table.as_mut().ok_or("orphan table header")?.end_row(),
            Event::End(TagEnd::TableRow) => table.as_mut().ok_or("orphan table row")?.end_row(),
            Event::End(TagEnd::Table) => document
                .tables
                .push(table.take().ok_or("unbalanced table")?.finish(range.end)),
            Event::Start(Tag::CodeBlock(kind)) => {
                block = Some(CodeBlock {
                    range,
                    info: info(kind.clone()),
                    fenced: kind.is_fenced(),
                    text: String::new(),
                })
            }
            Event::End(TagEnd::CodeBlock) => {
                let mut block = block.take().ok_or("unbalanced code block")?;
                block.range.end = range.end;
                if block.fenced && !closed_fence(source, &block.range) {
                    return Err("unbalanced fenced code block".into());
                }
                document.blocks.push(block);
            }
            Event::Start(Tag::HtmlBlock) => html = Some(String::new()),
            Event::End(TagEnd::HtmlBlock) => {
                if !html
                    .take()
                    .ok_or("unbalanced HTML block")?
                    .trim_start()
                    .starts_with("<!--")
                {
                    return Err("raw HTML blocks cannot supply contract content".into());
                }
            }
            Event::Start(Tag::Image { .. }) => {
                mark(&mut heading, &mut link, &mut table);
                image_depth += 1;
            }
            Event::End(TagEnd::Image) => image_depth = image_depth.saturating_sub(1),
            Event::Start(Tag::Link { dest_url, .. }) => {
                mark(&mut heading, &mut link, &mut table);
                if image_depth == 0 {
                    link = Some(LinkBuilder {
                        destination: dest_url.into_string(),
                        label: String::new(),
                        literal: !source[..range.start].ends_with("]("),
                    });
                }
            }
            Event::End(TagEnd::Link) => {
                if let Some(link) = link.take() {
                    document.links.push(Link {
                        label: link.label,
                        destination: link.destination,
                        literal: link.literal,
                    });
                }
            }
            Event::Text(value) => {
                if let Some(block) = &mut block {
                    block.text.push_str(&value);
                } else if let Some(html) = &mut html {
                    html.push_str(&value);
                } else if image_depth == 0 {
                    if let Some(heading) = &mut heading {
                        heading.text.push_str(&value);
                    }
                    if let Some(link) = &mut link {
                        link.label.push_str(&value);
                    }
                    if let Some(table) = &mut table {
                        table.text(&value);
                    }
                    document.text.push(Text {
                        range,
                        value: value.into_string(),
                    });
                }
            }
            Event::Code(value) => {
                if image_depth == 0 {
                    mark_inline(&mut heading, &mut link);
                    if let Some(table) = &mut table {
                        table.code(&value);
                    }
                    document.inline_code.push(InlineCode {
                        range,
                        value: value.into_string(),
                    });
                }
            }
            Event::Html(value) => {
                if let Some(html) = &mut html {
                    html.push_str(&value);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                document.text.push(Text {
                    range,
                    value: "\n".into(),
                });
                mark(&mut heading, &mut link, &mut table);
            }
            Event::InlineHtml(_)
            | Event::Rule
            | Event::TaskListMarker(_)
            | Event::FootnoteReference(_)
            | Event::InlineMath(_)
            | Event::DisplayMath(_) => mark(&mut heading, &mut link, &mut table),
            Event::Start(_) => mark(&mut heading, &mut link, &mut table),
            Event::End(_) => {}
        }
    }
    (heading.is_none() && link.is_none() && block.is_none() && html.is_none() && table.is_none())
        .then_some(document)
        .ok_or("unbalanced Markdown event stream".into())
}

fn info(kind: CodeBlockKind<'_>) -> String {
    match kind {
        CodeBlockKind::Fenced(info) => info.into_string(),
        CodeBlockKind::Indented => String::new(),
    }
}

fn mark(
    heading: &mut Option<HeadingBuilder>,
    link: &mut Option<LinkBuilder>,
    table: &mut Option<TableBuilder>,
) {
    mark_inline(heading, link);
    if let Some(table) = table {
        table.mark();
    }
}

fn mark_inline(heading: &mut Option<HeadingBuilder>, link: &mut Option<LinkBuilder>) {
    if let Some(heading) = heading {
        heading.literal = false;
    }
    if let Some(link) = link {
        link.literal = false;
    }
}

fn closed_fence(source: &str, range: &std::ops::Range<usize>) -> bool {
    let mut lines = source[range.start..].lines();
    let Some(open) = lines.next() else {
        return false;
    };
    let indentation = open.len() - open.trim_start_matches(' ').len();
    let fence = &open[indentation..];
    let Some(marker) = fence
        .chars()
        .next()
        .filter(|marker| matches!(marker, '`' | '~'))
    else {
        return false;
    };
    let length = fence
        .chars()
        .take_while(|character| *character == marker)
        .count();
    lines.any(|line| {
        let indentation = line.len() - line.trim_start_matches(' ').len();
        let candidate = &line[indentation..];
        let run = candidate
            .chars()
            .take_while(|character| *character == marker)
            .count();
        indentation <= 3
            && run >= length
            && candidate[run..]
                .bytes()
                .all(|byte| matches!(byte, b' ' | b'\t'))
    })
}
