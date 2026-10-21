// Keep catalog drift regressions separate from the specialist inventory checks.
use super::{assert_catalog_mutation_is_detected, TestResult};

#[test]
fn disposable_catalog_mutations_are_detected() -> TestResult {
    assert_catalog_mutation_is_detected(|catalog| {
        catalog.replacen(
            "  \"codexy-advisor.toml\",\n]",
            "  \"codexy-advisor.toml\",\n  \"codexy-watcher.toml\",\n]",
            1,
        )
    })?;
    assert_catalog_mutation_is_detected(|catalog| {
        catalog.replacen(
            "  \"codexy-advisor.toml\",\n]",
            "  \"codexy-advisor.toml\",\n  \"codexy-forge.toml\",\n]",
            1,
        )
    })?;
    assert_catalog_mutation_is_detected(|catalog| {
        catalog.replacen(
            "  \"codexy-advisor.toml\",\n]",
            "  \"codexy-advisor.toml\",\n  \"codexy-watcher.toml\",\n  \"codexy-watcher.toml\",\n]",
            1,
        )
    })?;
    Ok(())
}
