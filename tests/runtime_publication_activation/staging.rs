use crate::support;

use super::{has_dispatch, workflow};

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
    for forbidden in [
        "gh release create",
        "gh release view",
        "git tag -a",
        "git push origin $CANDIDATE_TAG",
        "releases/download/$CANDIDATE_TAG",
    ] {
        assert!(
            !staging.1.contains(forbidden),
            "runtime staging must not depend on a non-version release or tag: {forbidden}"
        );
    }
    Ok(())
}

#[test]
fn final_publisher_is_version_only() -> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    assert!(has_dispatch(&publisher.2), "final publisher needs workflow_dispatch");
    support::assert_structured_literals(
        &publisher.1,
        "version-only final publisher",
        &[
            "v1.3.0",
            "gh release create",
            "attest-build-provenance",
            "curl --fail",
            "codexy-marketplace-plugin.tar.gz",
        ],
    );
    assert!(
        !publisher.1.contains("runtime-candidate-"),
        "final publisher must never create a candidate release or tag"
    );
    Ok(())
}
