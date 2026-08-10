type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::path::Path;

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
    let article = read(&root.join("supported-topic/wiki/retrieval.md"))?;
    let raw = read(&root.join("supported-topic/raw/retrieval-source.md"))?;

    assert!(index.contains("wiki/retrieval.md"));
    assert!(article.contains("sources:\n  - raw/retrieval-source.md"));
    assert!(article.contains("volatility: warm"));
    assert!(article.contains("verified: 2026-08-09"));
    assert!(raw.contains("source: https://example.test/retrieval"));
    assert!(index.len() <= 4_000);
    assert!(article.len() <= 4_000);
    assert!(index.len() + article.len() <= 48_000);
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
    assert!(normalized.contains("MUST stop and report the provenance gap"));
    assert_eq!(read(&root.join("before/raw/source.md"))?, read(&root.join("after/raw/source.md"))?);
    let article = read(&root.join("after/wiki/topic.md"))?;
    assert!(article.contains("sources:\n  - raw/source.md"));
    assert!(article.contains("updated: 2026-08-09"));
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
    Ok(())
}

fn fixture_root() -> std::path::PathBuf {
    codexy_runtime::paths::repository_root().join("packages/codexy-runtime/tests/fixtures/wiki-core")
}

fn read(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}
