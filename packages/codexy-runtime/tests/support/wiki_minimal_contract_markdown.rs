mod events;
mod table;

use std::{collections::BTreeMap, ops::Range};

use table::Table;

pub(crate) struct Document {
    pub(super) source: String,
    pub(super) length: usize,
    pub(super) headings: Vec<Heading>,
    pub(super) tables: Vec<Table>,
    pub(super) blocks: Vec<CodeBlock>,
    pub(super) links: Vec<Link>,
    pub(super) inline_code: Vec<InlineCode>,
    pub(super) text: Vec<Text>,
}

pub(super) struct Heading {
    pub(super) range: Range<usize>,
    pub(super) level: usize,
    pub(super) text: String,
    pub(super) literal: bool,
}

pub(super) struct CodeBlock {
    pub(super) range: Range<usize>,
    pub(super) info: String,
    pub(super) fenced: bool,
    pub(super) text: String,
}

pub(super) struct Link {
    pub(super) range: Range<usize>,
    pub(super) label: String,
    pub(super) destination: String,
    pub(super) literal: bool,
}

pub(super) struct InlineCode {
    pub(super) range: Range<usize>,
    pub(super) value: String,
}

pub(super) struct Text {
    pub(super) range: Range<usize>,
    pub(super) value: String,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ActiveKind {
    Prose,
    Inline,
}

pub(crate) struct ActiveEvent<'a> {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) kind: ActiveKind,
    pub(crate) value: &'a str,
}

#[derive(Clone)]
pub(crate) struct Scope(Range<usize>);

impl Scope {
    pub(super) fn contains(&self, offset: usize) -> bool {
        self.0.contains(&offset)
    }
}

impl Document {
    pub(crate) fn parse(source: &str) -> Result<Self, String> {
        events::parse(source)
    }

    pub(crate) fn section(&self, title: &str) -> Result<Scope, String> {
        let level = title
            .chars()
            .take_while(|character| *character == '#')
            .count();
        let name = title[level..].trim_start();
        let matches: Vec<_> = self
            .headings
            .iter()
            .enumerate()
            .filter(|(_, heading)| {
                heading.level == level && heading.literal && heading.text == name
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!("missing or duplicate section {title}"));
        }
        let (index, heading) = matches[0];
        let end = self
            .headings
            .iter()
            .skip(index + 1)
            .find(|next| next.level <= level)
            .map_or(self.length, |next| next.range.start);
        Ok(Scope(heading.range.end..end))
    }

    pub(crate) fn child(&self, parent: &Scope, title: &str) -> Result<Scope, String> {
        let section = self.section(title)?;
        parent
            .contains(section.0.start)
            .then_some(section)
            .ok_or_else(|| format!("section {title} is outside its parent"))
    }

    pub(crate) fn workflow_rows(&self, scope: &Scope) -> Result<BTreeMap<String, String>, String> {
        let tables: Vec<_> = self
            .tables
            .iter()
            .filter(|table| scope.contains(table.start))
            .collect();
        match tables.as_slice() {
            [table] => table.workflow_rows(&self.source),
            _ => Err("missing or duplicate workflow table".into()),
        }
    }

    pub(crate) fn assignments(&self, scope: &Scope) -> Result<BTreeMap<String, String>, String> {
        let blocks: Vec<_> = self
            .blocks
            .iter()
            .filter(|block| scope.contains(block.range.start))
            .collect();
        let block = match blocks.as_slice() {
            [block] => *block,
            _ => return Err("missing or malformed assignment block".into()),
        };
        if block.info != "text" {
            return Err("missing or malformed assignment block".into());
        }
        if self
            .text
            .iter()
            .any(|text| scope.contains(text.range.start) && text.value.contains(" = "))
        {
            return Err("assignment outside canonical block".into());
        }
        let mut values = BTreeMap::new();
        for line in block.text.lines().filter(|line| !line.trim().is_empty()) {
            let (key, value) = line.split_once(" = ").ok_or("malformed assignment")?;
            if key.is_empty()
                || value.is_empty()
                || values.insert(key.into(), value.into()).is_some()
            {
                return Err("duplicate or malformed assignment".into());
            }
        }
        Ok(values)
    }

    pub(crate) fn link_count(&self, label: &str, target: &str) -> usize {
        self.links
            .iter()
            .filter(|link| link.literal && link.label == label && link.destination == target)
            .count()
    }

    pub(crate) fn link_count_in_scope(&self, scope: &Scope, label: &str, target: &str) -> usize {
        self.links
            .iter()
            .filter(|link| {
                scope.contains(link.range.start)
                    && link.literal
                    && link.label == label
                    && link.destination == target
            })
            .count()
    }

    pub(crate) fn inline_code_count(&self, scope: Option<&Scope>, value: &str) -> usize {
        self.inline_code
            .iter()
            .filter(|code| {
                scope.is_none_or(|scope| scope.contains(code.range.start)) && code.value == value
            })
            .count()
    }

    pub(crate) fn active_text(&self, scope: &Scope) -> String {
        self.active_values(scope, true)
    }

    pub(crate) fn active_prose(&self, scope: &Scope) -> String {
        self.active_values(scope, false)
    }

    pub(crate) fn active_events(&self, scope: &Scope) -> Vec<ActiveEvent<'_>> {
        let mut events = self
            .text
            .iter()
            .filter(|text| scope.contains(text.range.start))
            .map(|text| ActiveEvent {
                start: text.range.start,
                end: text.range.end,
                kind: ActiveKind::Prose,
                value: &text.value,
            })
            .chain(
                self.inline_code
                    .iter()
                    .filter(|code| scope.contains(code.range.start))
                    .map(|code| ActiveEvent {
                        start: code.range.start,
                        end: code.range.end,
                        kind: ActiveKind::Inline,
                        value: &code.value,
                    }),
            )
            .collect::<Vec<_>>();
        events.sort_unstable_by_key(|event| event.start);
        events
    }

    fn active_values(&self, scope: &Scope, include_inline: bool) -> String {
        self.active_events(scope)
            .into_iter()
            .filter(|event| include_inline || event.kind == ActiveKind::Prose)
            .map(|event| event.value)
            .collect::<Vec<_>>()
            .join(" ")
    }
}
