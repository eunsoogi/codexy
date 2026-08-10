type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};

use crate::support::wiki_core_article::{
    ArticleFinding, CanonicalDate, FreshnessState, ProvenanceState, assess_article,
};
use crate::support::wiki_core_contract::{
    frontmatter_string, markdown_link_count, validate_core_skill, validate_migration_rules,
};

const REMOVED_WORKFLOWS: &[&str] = &[
    "collect", "plan", "project", "inventory", "dataset", "archive", "ll", "status", "session",
    "session-capture", "rehydrate", "feedback", "feedback-capture",
];

#[test]
fn wiki_skill_exposes_the_core_workflow_and_no_removed_command() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;

    validate_core_skill(&skill, REMOVED_WORKFLOWS).map_err(std::io::Error::other)?;
    Ok(())
}

#[test]
fn supported_topic_fixture_proves_ingest_to_bounded_query() -> TestResult {
    let root = fixture_root();
    let index = read(&root.join("supported-topic/_index.md"))?;
    let category = read(&root.join("supported-topic/wiki/_index.md"))?;
    let article = read(&root.join("supported-topic/wiki/retrieval.md"))?;
    let raw = read(&root.join("supported-topic/raw/retrieval-source.md"))?;

    assert_eq!(markdown_link_count(&index, "Wiki articles", "wiki/_index.md")?, 1);
    assert_eq!(markdown_link_count(&category, "Retrieval", "retrieval.md")?, 1);
    assert_eq!(frontmatter_string(&article, "volatility")?, "warm");
    assert_eq!(frontmatter_string(&article, "verified")?, "2026-08-09");
    assert_eq!(frontmatter_string(&raw, "source")?, "https://example.test/retrieval");
    let assessment = assess_article(&article, &root.join("supported-topic"), evaluation_day()?);
    assert_eq!(assessment.freshness_credit, 25);
    assert!(matches!(assessment.freshness, FreshnessState::Valid(_)));
    assert_eq!(assessment.provenance, ProvenanceState::Complete);
    assert!(index.len() <= 4_000);
    assert!(category.len() <= 4_000);
    assert!(article.len() <= 4_000);
    assert!(index.len() + category.len() + article.len() <= 48_000);
    Ok(())
}

#[test]
fn migration_fixture_preserves_raw_history_and_adds_only_derived_metadata() -> TestResult {
    let root = fixture_root().join("migration");
    let guide = read(&codexy_runtime::paths::repository_root().join(
        "plugins/codexy/skills/wiki/references/migration.md",
    ))?;
    validate_migration_rules(&guide).map_err(std::io::Error::other)?;
    assert_eq!(
        snapshot(&root.join("failure/before"))?,
        snapshot(&root.join("failure/after"))?,
        "a failed preflight must leave the entire topic tree unchanged"
    );
    assert_eq!(
        snapshot(&root.join("invalid-freshness/before"))?,
        snapshot(&root.join("invalid-freshness/after"))?,
        "invalid freshness must leave the entire topic tree unchanged"
    );
    assert_eq!(
        snapshot(&root.join("missing-freshness/before"))?,
        snapshot(&root.join("missing-freshness/after"))?,
        "missing freshness must leave the entire topic tree unchanged"
    );
    let failed = assess_article(
        &read(&root.join("failure/before/wiki/topic.md"))?,
        &root.join("failure/before"),
        evaluation_day()?,
    );
    assert_eq!(failed.provenance, ProvenanceState::Broken);
    assert!(failed.blocks_migration());
    let invalid = assess_article(
        &read(&root.join("invalid-freshness/before/wiki/topic.md"))?,
        &root.join("invalid-freshness/before"),
        evaluation_day()?,
    );
    assert_eq!(invalid.freshness, FreshnessState::Malformed);
    assert!(invalid.blocks_migration());
    let missing = assess_article(
        &read(&root.join("missing-freshness/before/wiki/topic.md"))?,
        &root.join("missing-freshness/before"),
        evaluation_day()?,
    );
    assert_eq!(missing.freshness, FreshnessState::Missing);
    assert!(missing.blocks_migration());
    assert_eq!(read(&root.join("success/before/raw/source.md"))?, read(&root.join("success/after/raw/source.md"))?);
    let master = read(&root.join("success/after/_index.md"))?;
    let category = read(&root.join("success/after/wiki/_index.md"))?;
    let article = read(&root.join("success/after/wiki/topic.md"))?;
    assert_eq!(markdown_link_count(&master, "Articles", "wiki/_index.md")?, 1);
    assert_eq!(markdown_link_count(&category, "Legacy topic", "topic.md")?, 1);
    assert_eq!(frontmatter_string(&article, "updated")?, "2026-08-09");
    let success = assess_article(&article, &root.join("success/after"), evaluation_day()?);
    assert!(!success.blocks_migration());
    assert!(master.len() + category.len() + article.len() <= 48_000);
    Ok(())
}

#[test]
fn negative_fixtures_expose_broken_provenance_and_future_freshness() -> TestResult {
    let root = fixture_root().join("negative");
    let broken = assess_article(
        &read(&root.join("broken-provenance.md"))?,
        &root,
        evaluation_day()?,
    );
    let future = read(&root.join("future-freshness.md"))?;
    let valid = read(&root.join("valid-freshness.md"))?;
    let malformed = read(&root.join("malformed-freshness.md"))?;
    let missing = "---\ntitle: Missing freshness\nsources:\n  - raw/available.md\n---\n\nverified: 2026-08-09";

    assert!(!root.join("raw/missing.md").exists());
    assert!(root.join("raw/available.md").exists());
    let evaluation = evaluation_day()?;
    let valid = assess_article(&valid, &root, evaluation);
    let future = assess_article(&future, &root, evaluation);
    let malformed = assess_article(&malformed, &root, evaluation);
    let missing = assess_article(missing, &root, evaluation);
    assert_eq!(broken.provenance, ProvenanceState::Broken);
    assert!(broken.blocks_migration());
    assert_eq!(valid.freshness_credit, 25);
    assert!(matches!(valid.freshness, FreshnessState::Valid(_)));
    assert_eq!(future.freshness_credit, 0);
    assert_eq!(future.freshness, FreshnessState::Future);
    assert!(!future.blocks_migration());
    assert_eq!(malformed.freshness, FreshnessState::Malformed);
    assert!(malformed.blocks_migration());
    assert_eq!(missing.freshness, FreshnessState::Missing);
    assert!(missing.blocks_migration());
    Ok(())
}

#[test]
fn shared_assessment_accepts_only_structured_frontmatter_inputs() -> TestResult {
    let root = fixture_root().join("negative");
    let evaluation = evaluation_day()?;
    let assessment = |frontmatter: &str, body: &str| {
        assess_article(&format!("---\n{frontmatter}\n---\n{body}"), &root, evaluation)
    };
    for (date, valid) in [
        ("2024-02-29", true), ("2000-02-29", true), ("2023-02-29", false),
        ("1900-02-29", false), ("2026-04-30", true), ("2026-04-31", false),
        ("2026-00-01", false), ("2026-13-01", false),
    ] {
        let result = assessment(&format!("verified: {date}\nsources:\n  - raw/available.md"), "");
        assert_eq!(matches!(result.freshness, FreshnessState::Valid(_)), valid, "{date}");
    }
    let today = assessment("verified: 2026-08-10\nsources:\n  - raw/available.md", "");
    let future = assessment("verified: 2100-01-01\nsources:\n  - raw/available.md", "");
    assert_eq!(today.freshness_credit, 25);
    assert_eq!(future.freshness, FreshnessState::Future);
    assert_eq!(future.findings, vec![ArticleFinding::FutureFreshness]);
    for (name, fields, provenance) in [
        ("missing sources", "verified: 2026-08-09", ProvenanceState::Missing),
        ("empty sources", "verified: 2026-08-09\nsources: []", ProvenanceState::Malformed),
        ("non-sequence sources", "verified: 2026-08-09\nsources: raw/available.md", ProvenanceState::Malformed),
        ("non-string source", "verified: 2026-08-09\nsources:\n  - 7", ProvenanceState::Malformed),
        ("missing file", "verified: 2026-08-09\nsources:\n  - raw/missing.md", ProvenanceState::Broken),
        ("absolute path", "verified: 2026-08-09\nsources:\n  - /etc/passwd", ProvenanceState::Malformed),
        ("root escape", "verified: 2026-08-09\nsources:\n  - ../escape.md", ProvenanceState::Malformed),
    ] {
        let result = assessment(fields, "");
        assert!(matches!(result.freshness, FreshnessState::Valid(_)), "{name}");
        assert_eq!(result.provenance, provenance, "{name}");
        assert!(result.blocks_migration(), "{name}");
    }
    let missing_verified = assessment("sources:\n  - raw/available.md", "");
    let non_string_date = assessment("verified: 7\nsources:\n  - raw/available.md", "");
    assert_eq!(missing_verified.freshness, FreshnessState::Missing);
    assert_eq!(non_string_date.freshness, FreshnessState::Malformed);
    assert!(missing_verified.blocks_migration() && non_string_date.blocks_migration());
    for fields in ["[]", "verified: [\nsources:\n  - raw/available.md", "verified: 2026-08-09\nverified: 2026-08-09\nsources:\n  - raw/available.md", "verified: 2026-08-09\nsources:\n  - raw/available.md\nsources:\n  - raw/available.md"] {
        let result = assessment(fields, "");
        assert_eq!(result.freshness, FreshnessState::Malformed);
        assert_eq!(result.provenance, ProvenanceState::Malformed);
    }
    let spaces = assess_article("\u{feff}---\nverified: 2026-08-09\nsources:\n  - raw/with spaces.md\n---", &root, evaluation);
    assert_eq!(spaces.source_scalars, ["raw/with spaces.md"]);
    assert_eq!(spaces.provenance, ProvenanceState::Complete);
    let body_only = assess_article("verified: 2026-08-09\nsources:\n  - raw/available.md", &root, evaluation_day()?);
    assert_eq!(body_only.freshness, FreshnessState::Missing);
    assert_eq!(body_only.provenance, ProvenanceState::Missing);
    let source_decoy = assessment("verified: 2026-08-09", "sources:\n  - raw/available.md");
    assert_eq!(source_decoy.provenance, ProvenanceState::Missing);
    Ok(())
}

fn fixture_root() -> PathBuf {
    codexy_runtime::paths::repository_root().join("packages/codexy-runtime/tests/fixtures/wiki-core")
}

fn read(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

fn snapshot(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, std::io::Error> {
    let mut files = BTreeMap::new();
    collect_snapshot(root, root, &mut files)?;
    Ok(files)
}

fn collect_snapshot(
    root: &Path,
    path: &Path,
    files: &mut BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_snapshot(root, &path, files)?;
        } else {
            files.insert(path.strip_prefix(root).map_err(std::io::Error::other)?.into(), fs::read(&path)?);
        }
    }
    Ok(())
}

fn evaluation_day() -> Result<CanonicalDate, Box<dyn std::error::Error>> {
    CanonicalDate::parse("2026-08-10").ok_or_else(|| "invalid evaluation day".into())
}
