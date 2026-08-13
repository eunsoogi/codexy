use std::fs;

use anyhow::Result;

use super::{Fixture, activate, assert_activation_rejected_without_mutation};

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
        assert_activation_rejected_without_mutation(&fixture, "1.3.0")
            .map_err(|error| anyhow::anyhow!("{label}: {error}"))?;
    }
    Ok(())
}

#[test]
fn activation_rejects_an_out_of_range_semver_before_mutation() -> Result<()> {
    let fixture = Fixture::new()?;
    let before = fixture.tracked()?;
    let error = activate(&fixture.root, "2147483648.0.0", &fixture.receipt).unwrap_err();
    assert!(error.to_string().contains("version must be semver-like"));
    assert_eq!(fixture.tracked()?, before);
    Ok(())
}
