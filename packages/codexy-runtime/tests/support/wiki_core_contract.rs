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
    require_clause(
        &document,
        &root,
        Mode::MustNot,
        &[
            "search",
            "select",
            "initialize",
            "topic",
            "root",
            "implicitly",
        ],
    )?;
    require_clause(
        &document,
        &workflow,
        Mode::Must,
        &[
            "before",
            "freshness",
            "verification",
            "compilation",
            "query",
            "read",
            "minimal",
            "contract",
        ],
    )?;
    required_count(
        document.link_count("Minimal Contract", "references/minimal-contract.md"),
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
        .filter(|clause| clause.mode == mode && terms.iter().all(|term| phrase(clause, term)))
        .count();
    required_count(matches, &format!("typed clause: {}", terms.join(", ")))
}

fn phrase(clause: &Clause, term: &str) -> bool {
    let terms = term
        .split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    clause
        .prose
        .windows(terms.len())
        .any(|window| window == terms)
}
