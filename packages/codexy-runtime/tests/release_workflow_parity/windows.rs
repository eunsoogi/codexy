use super::{document, run};

#[test]
// Windows must consume the selected-release verifier output instead of introducing a separate download path.
fn windows_package_lifecycle_uses_the_selected_public_release_helper()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let workflow = document("python-package.yml")?;
    let lifecycle = run(
        &workflow,
        "github-activation-windows",
        "Run native component lifecycle tests",
    )?;
    for required in [
        "& ./.github/scripts/verify-public-marketplace-baselines.ps1",
        "if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }",
    ] {
        assert!(lifecycle.contains(required), "lifecycle binding missing {required}");
    }
    assert!(!lifecycle.contains("$release.watcher_binary"));
    assert!(!lifecycle.contains("CODEXY_TEST_WATCHER_BINARY"));
    let runtime = run(
        &workflow,
        "github-activation-windows",
        "Verify candidate wheel runtime handoff on native Windows",
    )?;
    assert!(runtime.contains(
        "$runtimeDir = Join-Path $env:CODEXY_PUBLIC_BUNDLE_ROOT \"plugins/codexy/runtime\""
    ));
    for forbidden in [
        "gh release",
        "Invoke-WebRequest",
        "Invoke-RestMethod",
        "Start-BitsTransfer",
        "releases/download/",
    ] {
        assert!(!lifecycle.contains(forbidden), "lifecycle contains {forbidden}");
    }
    let workflow_text = std::fs::read_to_string(
        root.join(".github/workflows/python-package.yml"),
    )?;
    for required in ["BASE_SHA:", "PRIOR_PUBLIC_VERSION:"] {
        assert!(workflow_text.contains(required), "workflow helper wiring missing {required}");
    }
    let helper = std::fs::read_to_string(root.join("scripts/verify_public_marketplace_bundle.py"))?;
    for required in [
        "from public_marketplace_bundle_support import",
        "CONTRACT,",
        "LEGACY_UPGRADE_VERSION,",
        "release_tag = current_tag",
        "\"gh\",",
        "\"release\",",
        "\"download\",",
        "runtime-release-receipt.json",
        "--legacy-upgrade",
        "legacy_watcher_binary",
        "extracted_root",
    ] {
        assert!(helper.contains(required), "public release helper missing {required}");
    }
    let support = std::fs::read_to_string(
        root.join("scripts/public_marketplace_bundle_support.py"),
    )?;
    for required in [
        "CONTRACT = Path(\".agents/plugins/release-publish-contract.json\")",
        "runtime-release-receipt/v2",
        "manifestSha256",
        "LEGACY_UPGRADE_VERSION",
        "watcher.exists()",
        "tarfile.open",
        "archive.extract",
    ] {
        assert!(support.contains(required), "public release support missing {required}");
    }
    let baselines = std::fs::read_to_string(
        root.join(".github/scripts/verify-public-marketplace-baselines.ps1"),
    )?;
    for required in [
        "python scripts/verify_public_marketplace_bundle.py --output-dir $bundleRoot",
        "--legacy-upgrade --output-dir $legacyRoot",
        "$legacyRelease.legacy_watcher_binary",
        "$env:CODEXY_TEST_LEGACY_MARKETPLACE_ROOT",
    ] {
        assert!(baselines.contains(required), "baseline verifier missing {required}");
    }
    Ok(())
}
