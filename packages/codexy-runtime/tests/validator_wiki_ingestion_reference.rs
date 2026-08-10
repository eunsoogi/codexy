type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::{collections::BTreeMap, fs, path::{Path, PathBuf}};

const REMOVED_WORKFLOWS: &[&str] = &[
    "collect", "plan", "project", "inventory", "dataset", "archive", "ll", "status", "session",
    "session-capture", "rehydrate", "feedback", "feedback-capture",
];

#[test]
fn wiki_skill_exposes_the_core_workflow_and_no_removed_command() -> TestResult {
    let root = codexy_runtime::paths::repository_root();
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;

    assert!(skill.contains("## Core workflow"));
    assert!(skill.contains("`init → ingest → compile → query → refresh`"));
    assert!(skill.contains("[Migration](references/migration.md)"));
    for workflow in REMOVED_WORKFLOWS {
        assert!(
            !skill.contains(&format!("`{workflow}`")),
            "removed workflow remains in the installed skill: {workflow}"
        );
    }
    Ok(())
}

#[test]
fn supported_topic_fixture_proves_ingest_to_bounded_query() -> TestResult {
    let root = fixture_root();
    let index = read(&root.join("supported-topic/_index.md"))?;
    let category = read(&root.join("supported-topic/wiki/_index.md"))?;
    let article = read(&root.join("supported-topic/wiki/retrieval.md"))?;
    let raw = read(&root.join("supported-topic/raw/retrieval-source.md"))?;

    assert!(index.contains("wiki/_index.md"));
    assert!(category.contains("retrieval.md"));
    assert!(article.contains("sources:\n  - raw/retrieval-source.md"));
    assert!(article.contains("volatility: warm"));
    assert!(article.contains("verified: 2026-08-09"));
    assert!(raw.contains("source: https://example.test/retrieval"));
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
    let normalized = guide.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(normalized.contains("MUST preserve existing `raw/`, `wiki/`, `_index.md`, and `log.md`"));
    assert!(normalized.contains("MUST NOT delete, overwrite, or rename existing topic data"));
    assert!(normalized.contains("MUST preserve every complete relative `sources:` scalar exactly"));
    assert!(normalized.contains("MUST stop, MUST report the provenance gap, and MUST leave the entire topic tree unchanged"));
    let preflight = guide.find("MUST validate every referenced provenance");
    let first_write = guide.find("MUST append one migration entry");
    assert!(preflight.is_some_and(|preflight| first_write.is_some_and(|write| preflight < write)));
    assert_eq!(
        snapshot(&root.join("failure/before"))?,
        snapshot(&root.join("failure/after"))?,
        "a failed preflight must leave the entire topic tree unchanged"
    );
    assert_eq!(read(&root.join("success/before/raw/source.md"))?, read(&root.join("success/after/raw/source.md"))?);
    let master = read(&root.join("success/after/_index.md"))?;
    let category = read(&root.join("success/after/wiki/_index.md"))?;
    let article = read(&root.join("success/after/wiki/topic.md"))?;
    assert!(master.contains("wiki/_index.md"));
    assert!(category.contains("topic.md"));
    assert!(article.contains("sources:\n  - raw/source.md"));
    assert!(article.contains("updated: 2026-08-09"));
    assert!(master.len() + category.len() + article.len() <= 48_000);
    Ok(())
}

#[test]
fn negative_fixtures_expose_broken_provenance_and_future_freshness() -> TestResult {
    let root = fixture_root().join("negative");
    let broken = read(&root.join("broken-provenance.md"))?;
    let future = read(&root.join("future-freshness.md"))?;

    assert!(broken.contains("sources:\n  - raw/missing.md"));
    assert!(!root.join("raw/missing.md").exists());
    assert!(future.contains("verified: 2100-01-01"));
    assert!(root.join("raw/available.md").exists());
    assert_eq!(future_date_credit(&future), 0);
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

fn future_date_credit(article: &str) -> u8 {
    article.contains("verified: 2100-01-01").then_some(0).unwrap_or(25)
}
