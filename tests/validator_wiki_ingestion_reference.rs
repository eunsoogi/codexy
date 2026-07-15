type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn wiki_skill_requires_reading_the_ingestion_facade() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let skill = std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/SKILL.md"))?;

    assert!(
        skill.contains("`ingest` and `ingest-collection` → MUST read `references/ingestion.md`")
    );
    Ok(())
}

#[test]
fn wiki_ingestion_facade_requires_relevant_split_reference_reads() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let ingestion =
        std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/ingestion.md"))?;
    let normalized = ingestion.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(normalized.contains(
        "Before any `ingest` or `ingest-collection` action, MUST read \
         [Overview](ingestion/overview.md) before acting."
    ));
    assert!(normalized.contains(
        "For Wayback CDX snapshot ingestion, MUST also read \
         [Wayback CDX Snapshots](ingestion/wayback-cdx-snapshots.md) before acting."
    ));
    Ok(())
}

#[test]
fn wiki_dataset_facade_requires_relevant_split_reference_reads() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let datasets =
        std::fs::read_to_string(root.join("plugins/codexy/skills/wiki/references/datasets.md"))?;
    let normalized = datasets.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(normalized.contains(
        "Before any `dataset` action, MUST read [Boundary](datasets/boundary.md) \
         and [Index Format](datasets/index-format.md) before acting."
    ));
    Ok(())
}
