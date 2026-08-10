use serde_yaml::Value;

use super::wiki_minimal_contract_markdown::Document;

pub(crate) fn validate_core_skill(source: &str, removed: &[&str]) -> Result<(), String> {
    let document = Document::parse(source)?;
    let workflow = document.section("## Core workflow")?;
    required_count(
        document.inline_code_count(Some(&workflow), "init → ingest → compile → query → refresh"),
        "core command inventory",
    )?;
    required_count(
        document.link_count("Migration", "references/migration.md"),
        "migration link",
    )?;
    for command in removed {
        if document.inline_code_count(None, command) != 0 {
            return Err(format!("removed command remains active: {command}"));
        }
    }
    Ok(())
}

pub(crate) fn markdown_link_count(
    source: &str,
    label: &str,
    target: &str,
) -> Result<usize, String> {
    Ok(Document::parse(source)?.link_count(label, target))
}

pub(crate) fn frontmatter_string(source: &str, key: &str) -> Result<String, String> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let (opening, remainder) = source.split_once('\n').ok_or("frontmatter opening")?;
    if opening.trim_end_matches('\r') != "---" {
        return Err("frontmatter opening".into());
    }
    let mut end = 0;
    for line in remainder.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            break;
        }
        end += line.len();
    }
    let yaml = (end < remainder.len())
        .then_some(&remainder[..end])
        .ok_or("frontmatter closing")?;
    let Value::Mapping(mapping) =
        serde_yaml::from_str::<Value>(yaml).map_err(|error| error.to_string())?
    else {
        return Err("frontmatter mapping".into());
    };
    mapping
        .get(Value::String(key.into()))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("frontmatter string: {key}"))
}

pub(crate) fn validate_migration_rules(source: &str) -> Result<(), String> {
    let document = Document::parse(source)?;
    let scope_section = document.section("## Scope")?;
    let procedure_section = document.section("## Procedure")?;
    let scope = RuleRecords::parse(&document.active_prose(&scope_section));
    let procedure = RuleRecords::parse(&document.active_prose(&procedure_section));
    scope.require(Mode::Must, &["preserve", "existing"])?;
    RuleRecords::parse(&document.active_text(&scope_section))
        .require_active(&["raw", "wiki", "index", "log"])?;
    required_count(
        document.inline_code_count(Some(&procedure_section), "sources:"),
        "source scalar identity",
    )?;
    scope.require(
        Mode::MustNot,
        &["delete", "overwrite", "rename", "existing topic data"],
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
        ),
        (
            Mode::Must,
            &["preserve", "complete relative", "scalar", "exactly"],
        ),
        (Mode::Must, &["stop"]),
        (Mode::Must, &["report", "provenance gap"]),
        (Mode::Must, &["leave", "entire topic tree unchanged"]),
        (Mode::Must, &["append", "migration entry"]),
    ])?;
    scope.reject_qualifiers()?;
    procedure.reject_qualifiers()
}

fn required_count(count: usize, identity: &str) -> Result<(), String> {
    (count == 1)
        .then_some(())
        .ok_or_else(|| format!("missing or duplicate {identity}"))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Must,
    MustNot,
}

struct RuleRecords {
    source: String,
    records: Vec<(Mode, String)>,
}

impl RuleRecords {
    fn new(source: &str) -> Self {
        let source = normalize(source);
        let mut records = Vec::new();
        for sentence in source.split('.') {
            let mut remainder = sentence.trim();
            while let Some(index) = remainder.find("must ") {
                Self::push(&mut records, &remainder[index + "must ".len()..]);
                remainder = &remainder[index + "must ".len()..];
            }
        }
        Self { source, records }
    }

    fn parse(source: &str) -> Self {
        Self::new(source)
    }

    fn require(&self, mode: Mode, terms: &[&str]) -> Result<(), String> {
        self.unique(mode, terms)
    }

    fn require_ordered(&self, expected: &[(Mode, &[&str])]) -> Result<(), String> {
        let mut from = 0;
        for (mode, terms) in expected {
            self.unique(*mode, terms)?;
            let Some(index) = self.records[from..].iter().position(|(found, tail)| {
                *found == *mode && terms.iter().all(|term| phrase(tail, &normalize(term)))
            }) else {
                return Err("missing or reordered normative record".into());
            };
            from += index + 1;
        }
        Ok(())
    }

    fn reject_qualifiers(&self) -> Result<(), String> {
        let forbidden = [
            "except",
            "unless",
            "baseline",
            "allowlist",
            "compatibility",
            "alias",
            "external",
            "restore",
        ];
        forbidden
            .iter()
            .all(|term| !phrase(&self.source, term))
            .then_some(())
            .ok_or("qualified normative route".into())
    }

    fn push(records: &mut Vec<(Mode, String)>, tail: &str) {
        let (mode, tail) = tail
            .strip_prefix("not ")
            .map_or((Mode::Must, tail), |tail| (Mode::MustNot, tail));
        let identity = tail.split("must ").next().unwrap_or_default().trim();
        if !identity.is_empty() {
            records.push((mode, identity.into()));
        }
    }

    fn unique(&self, mode: Mode, terms: &[&str]) -> Result<(), String> {
        let matches = self
            .records
            .iter()
            .filter(|(_, clause)| terms.iter().all(|term| phrase(clause, &normalize(term))))
            .collect::<Vec<_>>();
        (matches.len() == 1 && matches[0].0 == mode)
            .then_some(())
            .ok_or_else(|| {
                format!(
                    "missing, duplicate, or contradictory normative record: {}; records={:?}",
                    terms.join(", "),
                    self.records
                )
            })
    }

    fn require_active(&self, terms: &[&str]) -> Result<(), String> {
        terms
            .iter()
            .all(|term| phrase(&self.source, &normalize(term)))
            .then_some(())
            .ok_or_else(|| format!("missing active identity: {}", terms.join(", ")))
    }
}

fn normalize(source: &str) -> String {
    source
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else if character == '.' {
                '.'
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn phrase(text: &str, concept: &str) -> bool {
    text.match_indices(concept)
        .any(|(index, _)| phrase_at(text, index, concept))
}

fn phrase_at(text: &str, index: usize, concept: &str) -> bool {
    let before = text[..index].chars().next_back();
    let after = text[index + concept.len()..].chars().next();
    before.is_none_or(is_separator) && after.is_none_or(is_separator)
}

fn is_separator(character: char) -> bool {
    character.is_whitespace() || character == '.'
}
