use super::wiki_minimal_contract_markdown::{ActiveEvent, ActiveKind, Document, Scope};

pub(crate) fn validate_migration_rules(source: &str) -> Result<(), String> {
    let document = Document::parse(source)?;
    let scope = Rules::parse(&document, &document.section("## Scope")?);
    let procedure = Rules::parse(&document, &document.section("## Procedure")?);
    scope.require(
        Mode::Must,
        &["preserve", "existing"],
        &["raw/", "wiki/", "_index.md", "log.md"],
    )?;
    scope.require(
        Mode::MustNot,
        &["delete", "overwrite", "rename", "existing topic data"],
        &[],
    )?;
    procedure.require_ordered(&[
        (
            Mode::Must,
            &[
                "validate",
                "referenced provenance",
                "freshness input",
                "before any log",
                "derived write",
            ],
            &[],
        ),
        (
            Mode::Must,
            &["preserve", "complete relative", "scalar", "exactly"],
            &["sources:"],
        ),
        (Mode::Must, &["stop"], &[]),
        (Mode::Must, &["report", "provenance gap"], &[]),
        (Mode::Must, &["leave", "entire topic tree unchanged"], &[]),
        (Mode::Must, &["append", "migration entry"], &["log.md"]),
    ])
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Must,
    MustNot,
}

struct Clause {
    mode: Mode,
    prose: String,
    inline: Vec<String>,
}

struct Rules {
    clauses: Vec<Clause>,
}

impl Rules {
    fn parse(document: &Document, scope: &Scope) -> Self {
        let events = document.active_events(scope);
        let modes = events.iter().flat_map(modalities).collect::<Vec<_>>();
        let clauses = modes
            .iter()
            .enumerate()
            .map(|(index, &(start, mode))| {
                let next = modes.get(index + 1).map_or(usize::MAX, |next| next.0);
                let end = next.min(sentence_end(&events, start));
                let sentence = sentence_start(&events, start);
                let begin = modes[..index]
                    .last()
                    .is_some_and(|prior| prior.0 >= sentence)
                    .then_some(start)
                    .unwrap_or(sentence);
                Clause {
                    mode,
                    prose: prose_between(&events, begin, end),
                    inline: events
                        .iter()
                        .filter(|event| {
                            event.kind == ActiveKind::Inline
                                && event.start >= start
                                && event.start < end
                        })
                        .map(|event| event.value.into())
                        .collect(),
                }
            })
            .collect();
        Self { clauses }
    }

    fn require(&self, mode: Mode, terms: &[&str], inline: &[&str]) -> Result<usize, String> {
        let matches = self
            .clauses
            .iter()
            .enumerate()
            .filter(|(_, clause)| clause.matches(terms, inline))
            .collect::<Vec<_>>();
        if matches.len() != 1 || matches[0].1.mode != mode {
            return Err(format!(
                "missing, duplicate, or contradictory typed normative clause: {}",
                terms.join(", ")
            ));
        }
        matches[0].1.reject_qualifiers()?;
        Ok(matches[0].0)
    }

    fn require_ordered(&self, expected: &[(Mode, &[&str], &[&str])]) -> Result<(), String> {
        let mut previous = 0;
        for (mode, terms, inline) in expected {
            let found = self.require(*mode, terms, inline)?;
            if found < previous {
                return Err("reordered typed normative clause".into());
            }
            previous = found + 1;
        }
        Ok(())
    }
}

impl Clause {
    fn matches(&self, terms: &[&str], inline: &[&str]) -> bool {
        terms.iter().all(|term| phrase(&self.prose, term))
            && self.inline.len() == inline.len()
            && inline
                .iter()
                .all(|identity| self.inline.iter().any(|found| found == identity))
    }

    fn reject_qualifiers(&self) -> Result<(), String> {
        [
            "except",
            "unless",
            "baseline",
            "allowlist",
            "compatibility",
            "alias",
            "external",
            "restore",
        ]
        .iter()
        .all(|term| !phrase(&self.prose, term))
        .then_some(())
        .ok_or_else(|| "qualified typed normative clause".into())
    }
}

fn modalities(event: &ActiveEvent<'_>) -> Vec<(usize, Mode)> {
    if event.kind != ActiveKind::Prose {
        return Vec::new();
    }
    event
        .value
        .match_indices("MUST")
        .filter_map(|(offset, _)| {
            let tail = &event.value[offset + 4..];
            let boundary = event.value[..offset]
                .chars()
                .next_back()
                .is_none_or(|value| !value.is_ascii_alphabetic())
                && tail
                    .chars()
                    .next()
                    .is_none_or(|value| value.is_whitespace());
            boundary.then_some((
                event.start + offset,
                tail.strip_prefix(" NOT")
                    .map_or(Mode::Must, |_| Mode::MustNot),
            ))
        })
        .collect()
}

fn sentence_start(events: &[ActiveEvent<'_>], start: usize) -> usize {
    let Some(event) = events
        .iter()
        .find(|event| event.kind == ActiveKind::Prose && event.start <= start && start < event.end)
    else {
        return start;
    };
    event.value[..start - event.start]
        .rfind(['.', '\n'])
        .map_or(event.start, |offset| event.start + offset + 1)
}

fn sentence_end(events: &[ActiveEvent<'_>], start: usize) -> usize {
    events
        .iter()
        .filter(|event| event.kind == ActiveKind::Prose && event.end > start)
        .find_map(|event| {
            let from = start.saturating_sub(event.start);
            event.value[from..]
                .find('.')
                .map(|offset| event.start + from + offset + 1)
        })
        .unwrap_or(usize::MAX)
}

fn prose_between(events: &[ActiveEvent<'_>], start: usize, end: usize) -> String {
    events
        .iter()
        .filter(|event| event.kind == ActiveKind::Prose && event.end > start && event.start < end)
        .map(|event| {
            let from = start.saturating_sub(event.start);
            let to = end.saturating_sub(event.start).min(event.value.len());
            &event.value[from..to]
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn phrase(text: &str, term: &str) -> bool {
    let normalized = normalize(text);
    let term = normalize(term);
    normalized
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(term.split_whitespace().count())
        .any(|words| words.join(" ") == term)
}

fn normalize(source: &str) -> String {
    source
        .to_ascii_lowercase()
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
