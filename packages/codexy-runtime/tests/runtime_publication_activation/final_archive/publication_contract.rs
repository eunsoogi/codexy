use std::fs;

use crate::support;

use super::super::workflow;

#[test]
fn final_publisher_materializes_and_exercises_the_public_archive()
-> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    let verifier = workflow("verify-version-release.yml")?;
    let root = codexy_runtime::paths::repository_root();
    let run = format!(
        "{}\n{}\n{}\n{}\n{}",
        publisher.1,
        verifier.1,
        fs::read_to_string(root.join("scripts/smoke-public-getcodexy-release.sh"))?,
        fs::read_to_string(root.join("scripts/publish-verified-release"))?,
        fs::read_to_string(root.join("scripts/finalize-verified-release"))?,
    );
    support::assert_structured_literals(
        &run,
        "final publisher lineage and archive contract",
        &[
            "STAGING_SOURCE_COMMIT",
            "ACTIVATION_COMMIT",
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main",
            "test \"$ACTIVATION_COMMIT\" = \"$(git rev-parse origin/main)\"",
            "scripts/materialize-runtime-release-archive",
            "scripts/assemble-release-train-archive.sh",
            "codexy-marketplace-bundle.tar.gz",
            "cp staging/codexy-marketplace-plugin.tar.gz dist/codexy-runtime-package.tar.gz",
            "runtime-release-receipt.json",
            "scripts/inspect-release-archive public.tar.gz public-inspect/plugins/codexy-devtools",
            "scripts/verify-release-attestation-set",
            "gh release view \"$RELEASE_TAG\"",
            "-F draft=true",
            "RELEASE_ID", "gh api --method POST", "uploadUrl: .upload_url", "release_upload_url", "\"$upload_url?name=$asset\"",
            "releases/assets/$asset_id", "gh api --method PATCH",
            "release asset differs from verified bytes",
            "--plugin-root \"$PWD/plugins/codexy-devtools\"",
            "jq -er .version)\" = \"$TARGET_VERSION\"",
        ],
    );
    support::assert_structured_absent_literals(
        &run,
        "immutable release asset reconciliation",
        &["--clobber", "cp dist/codexy-marketplace-plugin.tar.gz dist/codexy-runtime-package.tar.gz", "gh release upload \"$RELEASE_TAG\"", "gh release download \"$RELEASE_TAG\"", "releases/tags/$RELEASE_TAG"],
    );
    let marker = |needle: &str| run.find(needle).ok_or("publisher ordering");
    let staged_identity = marker("tar -xOzf staging/codexy-marketplace-plugin.tar.gz")?;
    let runtime_copy = marker("cp staging/codexy-marketplace-plugin.tar.gz")?;
    let public_materialization = marker("scripts/materialize-runtime-release-archive")?;
    assert!(
        staged_identity < runtime_copy && runtime_copy < public_materialization,
        "staged identity must be checked before the byte-preserving copy and public materialization"
    );
    Ok(())
}
