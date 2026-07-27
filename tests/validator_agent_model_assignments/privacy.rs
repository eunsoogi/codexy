use std::process::Command;

use crate::support;

#[test]
fn specialist_model_contract_is_not_a_public_api() -> support::TestResult {
    let output = support::public_contract_import_check()?;
    support::assert_privacy_diagnostic(&output)?;
    Ok(())
}

#[test]
fn privacy_contract_import_rejects_unrelated_cargo_failures() -> support::TestResult {
    let temp = tempfile::tempdir()?;
    let output = Command::new("cargo")
        .args(["check", "--quiet"])
        .current_dir(temp.path())
        .output()?;

    assert!(
        support::assert_privacy_diagnostic(&output).is_err(),
        "an unrelated cargo failure must not prove the specialist contract private"
    );
    Ok(())
}
