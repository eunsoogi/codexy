use crate::support;

use super::{has_dispatch, workflow};
use super::super::structured_contract_artifacts::TextShape;

#[test]
fn runtime_staging_uses_authenticated_actions_artifacts_not_candidate_releases()
-> Result<(), Box<dyn std::error::Error>> {
    let staging = workflow("runtime-candidate.yml")?;
    support::assert_structured_literals(
        &staging.1,
        "authenticated runtime staging",
        &[
            "actions/upload-artifact",
            "actions/download-artifact",
            "attest-build-provenance",
            "SOURCE_COMMIT",
            "sha256",
        ],
    );
    TextShape::new(&staging.1).assert_absent_concepts(
        "runtime staging has no public candidate release or tag",
        &[
            "gh release create",
            "gh release view",
            "git tag -a",
            "git push origin $CANDIDATE_TAG",
            "releases/download/$CANDIDATE_TAG",
        ],
    );
    Ok(())
}

#[test]
fn windows_runtime_candidate_provisions_uv_before_installed_contract()
-> Result<(), Box<dyn std::error::Error>> {
    let staging = workflow("runtime-candidate.yml")?;
    let setup = staging
        .1
        .find("- uses: astral-sh/setup-uv@v7")
        .ok_or("Windows runtime candidate must provision uv")?;
    let contract = staging
        .1
        .find("- name: Prove native handoff bridge and installed CMD adversarial contract")
        .ok_or("Windows installed adversarial contract step missing")?;
    let setup_block = &staging.1[setup..contract];
    assert!(
        setup_block.contains("if: matrix.platform == 'windows-x86_64'"),
        "uv provisioning must be scoped to the Windows candidate"
    );
    assert!(setup < contract, "uv must be provisioned before the Windows uv run");
    Ok(())
}

#[test]
fn final_publisher_is_version_only() -> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    let verifier = workflow("verify-version-release.yml")?;
    assert!(has_dispatch(&publisher.2), "final publisher needs workflow_dispatch");
    let lifecycle = format!(
        "{}\n{}\n{}",
        publisher.1,
        verifier.1,
        std::fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"))?,
    );
    support::assert_structured_literals(
        &lifecycle,
        "version-only final publisher",
        &[
            "target_version",
            "RELEASE_TAG",
            "gh api --method POST",
            "attest-build-provenance",
            "curl --fail",
            "codexy-marketplace-plugin.tar.gz",
        ],
    );
    TextShape::new(&publisher.1).assert_absent_concepts(
        "final publisher has no candidate release or tag",
        &[
            "gh release create runtime-candidate-",
            "gh release view runtime-candidate-",
            "git tag -a runtime-candidate-",
            "git push origin runtime-candidate-",
            "releases/download/runtime-candidate-",
        ],
    );
    Ok(())
}
