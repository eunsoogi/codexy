type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::fs;

use crate::support::{
    wiki_core_article::{CanonicalDate, ProvenanceState, assess_article},
};

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
