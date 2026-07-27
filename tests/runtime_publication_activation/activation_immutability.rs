use std::{fs, path::Path};

use crate::support::FixtureCommand;

use super::activation_bytes;

#[test]
fn invalid_activation_is_byte_identical() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let gate = root.join("scripts/activate-runtime-contract");
    assert!(
        gate.is_file(),
        "missing activation gate entrypoint: {}",
        gate.display()
    );
    let before = activation_bytes(root)?;
    let temp = tempfile::tempdir()?;
    let receipt = temp.path().join("invalid-receipt.json");
    fs::write(&receipt, r#"{"schema":"invalid"}"#)?;
    let output = FixtureCommand::new(&gate)
        .args([
            "--repo-root",
            root.to_str().ok_or("non-UTF-8 repository root")?,
            "--bootstrap-version",
            "1.2.2",
            "--candidate-receipt",
            receipt.to_str().ok_or("non-UTF-8 receipt path")?,
        ])
        .env("CODEXY_TEST_MODE", "1")
        .env(
            "CODEXY_TEST_ACTIVATE_RUNTIME_BINARY",
            env!("CARGO_BIN_EXE_codexy-activate-runtime"),
        )
        .output()?;
    assert!(
        !output.status.success(),
        "activation accepted an invalid candidate receipt"
    );
    assert_eq!(
        activation_bytes(root)?,
        before,
        "failed activation mutated a public pointer"
    );
    Ok(())
}
