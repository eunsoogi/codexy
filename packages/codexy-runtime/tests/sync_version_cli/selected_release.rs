use std::fs;

use serde_json::Value;

use super::{archive_repository, run_sync};
use super::isolation::{fixture_version, next_patch_version};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selected_release_projections_fail_closed_without_mutation() -> TestResult {
    let cases: &[(&str, &str, fn(String, &str) -> String)] = &[
        (
            "contract selected version",
            ".agents/plugins/release-publish-contract.json",
            |text: String, version: &str| {
                text.replace(
                    &format!("\"selectedVersion\": \"{version}\""),
                    "\"selectedVersion\": \"1.7.0\"",
                )
            },
        ),
        (
            "contract runtime tag",
            ".agents/plugins/release-publish-contract.json",
            |text: String, version: &str| {
                text.replace(
                    &format!("\"selectedTag\": \"v{version}\""),
                    "\"selectedTag\": \"v1.7.0\"",
                )
            },
        ),
        (
            "runtime release URL",
            "plugins/codexy-devtools/runtime-release.json",
            |text: String, version: &str| {
                text.replace(
                    &format!("/v{version}/codexy-runtime-package.tar.gz"),
                    "/v1.7.0/codexy-runtime-package.tar.gz",
                )
            },
        ),
        (
            "selected bootstrap",
            "packages/codexy-runtime/src/version/bootstrap.rs",
            |text: String, version: &str| {
                text.replacen(
                    &format!("pub(super) const VERSION: &str = \"{version}\";"),
                    "pub(super) const VERSION: &str = \"1.7.0\";",
                    1,
                )
            },
        ),
        (
            "Windows workflow pin",
            ".github/workflows/python-package.yml",
            |text: String, _version: &str| {
                text.replacen(
                    "          $ErrorActionPreference = \"Stop\"\n",
                    "          $ErrorActionPreference = \"Stop\"\n          $version = \"1.7.0\"\n",
                    1,
                )
            },
        ),
        (
            "Windows lifecycle direct release download",
            ".github/workflows/python-package.yml",
            |text: String, _version: &str| {
                text.replacen(
                    "          $release = $verified | ConvertFrom-Json\n",
                    "          $release = $verified | ConvertFrom-Json\n          gh release download v1.7.0 --repo eunsoogi/codexy --dir $bundleRoot\n",
                    1,
                )
            },
        ),
        (
            "Windows lifecycle binary rebinding",
            ".github/workflows/python-package.yml",
            |text: String, _version: &str| {
                text.replacen(
                    "          $binary = $release.watcher_binary\n",
                    "          $binary = \"stale-watcher.exe\"\n",
                    1,
                )
            },
        ),
        (
            "Windows lifecycle environment rebinding",
            ".github/workflows/python-package.yml",
            |text: String, _version: &str| {
                text.replacen(
                    "          $env:CODEXY_TEST_WATCHER_BINARY = $binary\n",
                    "          $env:CODEXY_TEST_WATCHER_BINARY = $binary\n          $env:CODEXY_TEST_WATCHER_BINARY = \"stale-watcher.exe\"\n",
                    1,
                )
            },
        ),
        (
            "public bundle verifier pin",
            "scripts/public_marketplace_bundle_support.py",
            |mut text: String, _version: &str| {
                text.push_str("\nversion = \"1.7.0\"\n");
                text
            },
        ),
    ];
    for &(label, relative, mutate) in cases {
        let temporary = tempfile::tempdir()?;
        let root = archive_repository(super::shared_repository_archive()?, &temporary, label)?;
        let version = fixture_version(&root)?;
        let path = root.join(relative);
        let original = fs::read_to_string(&path)?;
        let mutated = mutate(original.clone(), &version);
        assert_ne!(mutated, original, "{label} fixture did not change");
        fs::write(&path, &mutated)?;
        let output = run_sync(&root, &["--check"])?;
        assert!(!output.status.success(), "stale {label} unexpectedly passed");
        assert_eq!(fs::read_to_string(&path)?, mutated, "{label} was mutated");
    }
    Ok(())
}

#[test]
fn candidate_keeps_the_selected_public_release_and_historical_fixtures() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let root = archive_repository(super::shared_repository_archive()?, &temporary, "candidate")?;
    let selected = fixture_version(&root)?;
    let candidate = next_patch_version(&selected)?;
    let prepared = run_sync(&root, &["--prepare-candidate", &candidate])?;
    assert!(
        prepared.status.success(),
        "candidate preparation failed: {}",
        String::from_utf8_lossy(&prepared.stderr)
    );
    let checked = run_sync(&root, &["--check-candidate"])?;
    assert!(
        checked.status.success(),
        "candidate selected-release check failed: {}",
        String::from_utf8_lossy(&checked.stderr)
    );
    let contract: Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(contract["runtime"]["selectedTag"], format!("v{selected}"));
    let historical = root.join(
        "packages/codexy-runtime/tests/runtime_workflow_recovery/ci_dispatch_fixture.rs",
    );
    let historical_text = fs::read_to_string(historical)?;
    assert!(
        historical_text.contains("HISTORICAL_RELEASE_FIXTURE"),
        "historical release fixture is not explicitly classified"
    );
    Ok(())
}
