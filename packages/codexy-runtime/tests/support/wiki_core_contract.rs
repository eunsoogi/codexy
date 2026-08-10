use serde_yaml::Value;

use super::{
    wiki_active_token_stream::{Clause, Mode, clauses},
    wiki_minimal_contract_markdown::{Document, Scope},
};

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
    let root = document.section("## Topic root")?;
    require_clause(
        &document,
        &root,
        Mode::Must,
        &[
            "before",
            "initialization",
            "ingestion",
            "compilation",
            "querying",
            "refresh",
            "migration",
            "caller",
            "supply",
            "explicit",
            "topic",
            "root",
        ],
    )?;
    require_clause(&document, &root, Mode::Must, &["root", "absent", "request"])?;
    for action in ["search", "select", "initialize"] {
        require_clause(
            &document,
            &root,
            Mode::MustNot,
            &[action, "topic root", "implicitly"],
        )?;
    }
    require_ordered_clause(
        &document,
        &workflow,
        Mode::Must,
        &[
            "read minimal contract",
            "before freshness verification",
            "compilation",
            "query",
        ],
    )?;
    required_count(
        document.link_count_in_scope(
            &workflow,
            "Minimal Contract",
            "references/minimal-contract.md",
        ),
        "Minimal Contract link",
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
    let mapping = super::wiki_frontmatter::mapping(source).ok_or("frontmatter mapping")?;
    mapping
        .get(Value::String(key.into()))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("frontmatter string: {key}"))
}

fn required_count(count: usize, identity: &str) -> Result<(), String> {
    (count == 1)
        .then_some(())
        .ok_or_else(|| format!("missing or duplicate {identity}"))
}

fn require_clause(
    document: &Document,
    scope: &Scope,
    mode: Mode,
    terms: &[&str],
) -> Result<(), String> {
    let matches = clauses(document, scope)
        .iter()
        .filter(|clause| terms.iter().all(|term| phrase(clause, term)))
        .count();
    (matches == 1).then_some(()).ok_or_else(|| {
        format!(
            "missing, duplicate, or contradictory typed clause: {}",
            terms.join(", ")
        )
    })?;
    let clause = clauses(document, scope)
        .into_iter()
        .find(|clause| terms.iter().all(|term| phrase(clause, term)))
        .ok_or("typed clause missing after count")?;
    (clause.mode == mode)
        .then_some(())
        .ok_or_else(|| format!("contradictory typed clause: {}", terms.join(", ")))
}

fn require_ordered_clause(
    document: &Document,
    scope: &Scope,
    mode: Mode,
    terms: &[&str],
) -> Result<(), String> {
    let clauses = clauses(document, scope);
    let matches = clauses
        .iter()
        .filter(|clause| ordered_phrase(clause, terms))
        .collect::<Vec<_>>();
    (matches.len() == 1 && matches[0].mode == mode)
        .then_some(())
        .ok_or_else(|| {
            format!(
                "missing, duplicate, or contradictory ordered clause: {}",
                terms.join(", ")
            )
        })
}

fn phrase(clause: &Clause, term: &str) -> bool {
    clause.contains_phrase(term)
}

fn ordered_phrase(clause: &Clause, terms: &[&str]) -> bool {
    let mut cursor = 0;
    for term in terms {
        let words = term
            .split(|value: char| !value.is_ascii_alphanumeric())
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        let Some(index) = clause.prose[cursor..]
            .windows(words.len())
            .position(|window| window == words)
        else {
            return false;
        };
        cursor += index + words.len();
    }
    true
}
