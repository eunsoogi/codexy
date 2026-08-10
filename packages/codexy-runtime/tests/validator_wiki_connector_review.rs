type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::fs;

use crate::support::{
    wiki_active_token_stream::{Clause, Mode, clauses},
    wiki_core_article::{CanonicalDate, ProvenanceState, assess_article},
    wiki_minimal_contract_markdown::{Document, Scope},
};

#[test]
fn core_skill_requires_explicit_root_without_implicit_topic_selection() -> TestResult {
    let document = wiki_skill()?;
    let scope = document.section("## Topic root")?;
    require(
        &document,
        &scope,
        Mode::Must,
        &["before", "initialization", "ingestion", "compilation", "querying", "refresh", "migration", "caller", "supply", "explicit", "topic", "root"],
    )?;
    require(
        &document,
        &scope,
        Mode::MustNot,
        &["search", "select", "initialize", "topic", "root", "implicitly"],
    )?;
    Ok(())
}

#[test]
fn core_skill_loads_minimal_contract_before_applicable_core_work() -> TestResult {
    let document = wiki_skill()?;
    let scope = document.section("## Core workflow")?;
    require(
        &document,
        &scope,
        Mode::Must,
        &["before", "freshness", "verification", "compilation", "query", "read", "minimal", "contract"],
    )?;
    (document.link_count("Minimal Contract", "references/minimal-contract.md") == 1)
        .then_some(())
        .ok_or("missing or duplicate Minimal Contract link".into())
}

#[test]
fn migration_stages_and_validates_before_final_log_append() -> TestResult {
    let source = fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("plugins/codexy/skills/wiki/references/migration.md"),
    )?;
    let document = Document::parse(&source)?;
    let procedure = document.section("## Procedure")?;
    let stage = require(
        &document,
        &procedure,
        Mode::Must,
        &["stage", "all", "derived", "changes", "completion", "log", "entry"],
    )?;
    let validate = require(
        &document,
        &procedure,
        Mode::Must,
        &["validate", "staged", "derived", "changes", "completion", "log", "entry", "together"],
    )?;
    let commit = require(
        &document,
        &procedure,
        Mode::Must,
        &["atomically", "commit", "derived", "files"],
    )?;
    let append = require(
        &document,
        &procedure,
        Mode::Must,
        &["append", "one", "migration", "entry", "final", "commit", "action"],
    )?;
    require(
        &document,
        &procedure,
        Mode::Must,
        &["roll", "back", "every", "staged", "derived", "change", "leave", "topic", "unchanged"],
    )?;
    (stage < validate && validate < commit && commit < append)
        .then_some(())
        .ok_or("migration commit order".into())
}

#[test]
fn missing_raw_ingested_date_blocks_migration() -> TestResult {
    let assessment = assessment_for_raw("---\ntitle: raw\n---\nsource")?;
    assert_eq!(assessment.provenance, ProvenanceState::Complete);
    assert_eq!(assessment.raw_ingestion, crate::support::wiki_core_raw_ingestion::RawIngestionState::Missing);
    assert!(assessment.blocks_migration());
    Ok(())
}

#[test]
fn malformed_raw_ingested_date_blocks_migration() -> TestResult {
    let assessment = assessment_for_raw("---\ntitle: raw\ningested: yesterday\n---\nsource")?;
    assert_eq!(assessment.provenance, ProvenanceState::Complete);
    assert_eq!(assessment.raw_ingestion, crate::support::wiki_core_raw_ingestion::RawIngestionState::Malformed);
    assert!(assessment.blocks_migration());
    Ok(())
}

#[test]
fn raw_ingested_frontmatter_accepts_exact_yaml_document_end() -> TestResult {
    let assessment = assessment_for_raws(&[("raw/source.md", "---\ntitle: raw\ningested: 2026-08-09\n...\nsource")])?;
    assert_eq!(assessment.provenance, ProvenanceState::Complete);
    assert_eq!(assessment.raw_ingestion, crate::support::wiki_core_raw_ingestion::RawIngestionState::Complete);
    assert!(!assessment.blocks_migration());
    Ok(())
}

#[test]
fn each_raw_ingested_date_blocks_migration_independently() -> TestResult {
    for (name, raw) in [
        ("missing", "---\ntitle: raw\n---\nsource"),
        ("malformed", "---\ntitle: raw\ningested: yesterday\n---\nsource"),
    ] {
        let assessment = assessment_for_raws(&[
            ("raw/valid.md", "---\ntitle: raw\ningested: 2026-08-09\n---\nsource"),
            ("raw/invalid.md", raw),
        ])?;
        assert_eq!(assessment.provenance, ProvenanceState::Complete, "{name}");
        assert!(assessment.blocks_migration(), "{name}");
    }
    Ok(())
}

fn wiki_skill() -> Result<Document, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("plugins/codexy/skills/wiki/SKILL.md"),
    )?;
    Ok(Document::parse(&source)?)
}

fn require(
    document: &Document,
    scope: &Scope,
    mode: Mode,
    terms: &[&str],
) -> Result<usize, Box<dyn std::error::Error>> {
    let clauses = clauses(document, scope);
    let matches = clauses
        .iter()
        .enumerate()
        .filter(|(_, clause)| clause.mode == mode && terms.iter().all(|term| phrase(clause, term)))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!("missing or duplicate typed clause: {}", terms.join(", ")).into());
    }
    Ok(matches[0].0)
}

fn phrase(clause: &Clause, term: &str) -> bool {
    let terms = term
        .split(|value: char| !value.is_ascii_alphanumeric())
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    clause.prose.windows(terms.len()).any(|window| window == terms)
}

fn assessment_for_raw(raw: &str) -> Result<crate::support::wiki_core_article::ArticleAssessment, Box<dyn std::error::Error>> {
    assessment_for_raws(&[("raw/source.md", raw)])
}

fn assessment_for_raws(raws: &[(&str, &str)]) -> Result<crate::support::wiki_core_article::ArticleAssessment, Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    for (path, raw) in raws {
        let path = root.path().join(path);
        fs::create_dir_all(path.parent().ok_or("raw parent")?)?;
        fs::write(path, raw)?;
    }
    let sources = raws
        .iter()
        .map(|(path, _)| format!("  - {path}"))
        .collect::<Vec<_>>()
        .join("\n");
    let article = format!("---\nverified: 2026-08-09\nsources:\n{sources}\n---\narticle");
    let evaluation = CanonicalDate::parse("2026-08-10").ok_or("evaluation date")?;
    Ok(assess_article(&article, root.path(), evaluation))
}
