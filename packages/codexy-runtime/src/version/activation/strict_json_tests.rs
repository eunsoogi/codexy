use std::fs;

use anyhow::Result;

use super::fixture::assert_activation_rejected_without_mutation;
use super::{candidate_version, fixture::Fixture};

#[test]
fn activation_rejects_top_level_and_nested_duplicate_json_keys_without_mutation() -> Result<()> {
    for (label, key) in [
        (
            "top-level",
            r#""schema":"codexy-runtime-candidate-receipt/v1","#,
        ),
        (
            "nested",
            r#""repository":"https://github.com/eunsoogi/codexy","#,
        ),
    ] {
        let fixture = Fixture::new()?;
        let receipt = fs::read_to_string(&fixture.receipt)?;
        fs::write(
            &fixture.receipt,
            receipt.replacen(key, &format!("{key}{key}"), 1),
        )?;
        assert_activation_rejected_without_mutation(&fixture, candidate_version())
            .map_err(|error| anyhow::anyhow!("{label}: {error}"))?;
    }
    Ok(())
}
