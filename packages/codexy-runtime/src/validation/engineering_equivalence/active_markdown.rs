use std::path::{Component, Path};

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

pub(super) struct Document {
    headings: Vec<Heading>,
    links: Vec<Link>,
}

pub(super) struct Heading {
    pub(super) start: usize,
    pub(super) end: usize,
    anchor: String,
    level: HeadingLevel,
}

struct Link {
    start: usize,
    target: String,
    top_level: bool,
}

struct OpenHeading {
    start: usize,
    text: String,
    level: HeadingLevel,
}

impl Document {
    pub(super) fn parse(source: &str) -> Self {
        let mut headings = Vec::new();
        let mut links = Vec::new();
        let mut stack = Vec::new();
        let mut heading = None;
        for (event, range) in Parser::new(source).into_offset_iter() {
            match event {
                Event::Start(Tag::Heading { level, .. })
                    if matches!(level, HeadingLevel::H1 | HeadingLevel::H2) =>
                {
                    stack.push(false);
                    heading = Some(OpenHeading {
                        start: range.start,
                        text: String::new(),
                        level,
                    });
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    links.push(Link {
                        start: range.start,
                        target: dest_url.into_string(),
                        top_level: top_level(&stack),
                    });
                    stack.push(false);
                }
                Event::Start(tag) => stack.push(inactive_or_container(&tag)),
                Event::End(TagEnd::Heading(level))
                    if matches!(level, HeadingLevel::H1 | HeadingLevel::H2) =>
                {
                    let Some(_) = stack.pop() else {
                        continue;
                    };
                    let open = heading.take();
                    if top_level(&stack) {
                        if let Some(open) = open {
                            headings.push(Heading {
                                start: open.start,
                                end: source.len(),
                                anchor: fragment(&open.text),
                                level: open.level,
                            });
                        }
                    }
                }
                Event::End(_) => {
                    stack.pop();
                }
                Event::Text(value) | Event::Code(value) => {
                    if let Some(open) = &mut heading {
                        open.text.push_str(&value);
                    }
                }
                _ => {}
            }
        }
        for index in 0..headings.len().saturating_sub(1) {
            headings[index].end = headings[index + 1].start;
        }
        Self { headings, links }
    }

    pub(super) fn unique_heading(&self, anchor: &str) -> Option<&Heading> {
        let mut headings = self
            .headings
            .iter()
            .filter(|item| item.level == HeadingLevel::H2 && item.anchor == anchor);
        let heading = headings.next()?;
        headings.next().is_none().then_some(heading)
    }

    pub(super) fn exact_top_level_link(&self, heading: &Heading, target: &str) -> bool {
        let mut links = self.links.iter().filter(|link| {
            link.top_level && normalized_local_target(&link.target).as_deref() == Some(target)
        });
        let Some(link) = links.next() else {
            return false;
        };
        links.next().is_none() && (heading.start..heading.end).contains(&link.start)
    }
}

pub(super) fn top_level(stack: &[bool]) -> bool {
    stack.iter().all(|excluded| !excluded)
}

fn inactive_or_container(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::HtmlBlock
            | Tag::List(_)
            | Tag::Item
            | Tag::FootnoteDefinition(_)
            | Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
            | Tag::MetadataBlock(_)
    )
}

fn fragment(text: &str) -> String {
    let mut result = String::new();
    let mut separator = false;
    for character in text.trim().chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator && !result.is_empty() {
                result.push('-');
            }
            result.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    result
}

fn normalized_local_target(target: &str) -> Option<String> {
    let (path, fragment) = target
        .split_once('#')
        .map_or((target, None), |(path, fragment)| (path, Some(fragment)));
    if fragment.is_some() || path.contains('?') || path.is_empty() {
        return None;
    }
    let path = Path::new(path);
    if path.is_absolute()
        || path.components().any(|item| {
            matches!(
                item,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return None;
    }
    let normalized = path
        .components()
        .filter_map(|item| match item {
            Component::Normal(value) => value.to_str(),
            Component::CurDir => None,
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    (!normalized.is_empty()).then_some(normalized)
}

#[cfg(test)]
mod tests {
    use super::top_level;

    #[test]
    fn top_level_rejects_every_excluded_ancestor() {
        assert!(top_level(&[]));
        assert!(top_level(&[false, false]));
        assert!(!top_level(&[true]));
        assert!(!top_level(&[false, true]));
        assert!(!top_level(&[true, false]));
    }
}
