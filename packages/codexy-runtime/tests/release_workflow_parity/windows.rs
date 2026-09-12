use super::{document, run};

#[test]
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
        "$verified = python scripts/verify_public_marketplace_bundle.py --output-dir $bundleRoot",
        "$release = $verified | ConvertFrom-Json",
        "$binary = $release.watcher_binary",
        "$env:CODEXY_TEST_WATCHER_BINARY = $binary",
        "\"CODEXY_TEST_WATCHER_BINARY=$binary\" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append",
    ] {
        assert!(lifecycle.contains(required), "lifecycle binding missing {required}");
    }
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
        "release_tag = current_tag",
        "\"gh\",",
        "\"release\",",
        "\"download\",",
        "runtime-release-receipt.json",
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
        "tarfile.open",
        "archive.extract",
    ] {
        assert!(support.contains(required), "public release support missing {required}");
    }
    Ok(())
}
