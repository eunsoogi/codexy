//! Ensures the specialist contract stays private and that only its own
//! diagnostic, not an unrelated Cargo failure, proves the boundary.

use std::path::Path;
use std::process::Command;

use crate::support;

#[test]
fn specialist_model_contract_is_not_a_public_api() -> support::TestResult {
    let output = support::public_contract_import_check()?;
    support::assert_privacy_diagnostic(&output)?;
    Ok(())
}

#[test]
fn privacy_contract_check_reuses_the_running_test_target() {
    // Windows may place this generated suite below build/.../out instead of deps.
    let executable = std::env::current_exe().expect("test executable");
    let profile = support::cargo_test_profile_dir(&executable).expect("Cargo profile directory");
    assert_eq!(
        support::public_contract_target_dir(),
        profile.parent().expect("target directory")
    );
}

#[test]
fn privacy_target_resolution_supports_cargo_suite_output_layouts() {
    let target = Path::new("custom-target");
    let dependency_runner = target.join("debug").join("deps").join("suite_agent.exe");
    let generated_runner = target
        .join("debug")
        .join("build")
        .join("codexy-runtime")
        .join("hash")
        .join("out")
        .join("suite_agent.exe");

    assert_eq!(
        support::cargo_test_profile_dir(&dependency_runner),
        Some(target.join("debug"))
    );
    assert_eq!(
        support::cargo_test_profile_dir(&generated_runner),
        Some(target.join("debug"))
    );
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
