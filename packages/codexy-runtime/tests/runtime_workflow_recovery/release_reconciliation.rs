use std::fs;

use crate::support;

#[path = "release_attestation_reconciliation.rs"]
mod release_attestation_reconciliation;
#[path = "release_reconciliation_assertions.rs"]
mod release_reconciliation_assertions;
#[path = "release_reconciliation/edit_baseline.rs"]
mod edit_baseline;

#[test]
fn release_reconciliation_authenticates_a_draft_before_finalization()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let publish = fs::read_to_string(root.join("scripts/publish-verified-release"))?;
    let baseline = fs::read_to_string(root.join("scripts/reconcile-release-baseline"))?;
    let attestation = fs::read_to_string(root.join("scripts/verify-release-attestation-total"))?;
    support::assert_structured_literals(
        &publish,
        "release draft reconciliation",
        &[
            "gh release create \"$RELEASE_TAG\" --verify-tag --draft --target \"$ACTIVATION_COMMIT\"",
            "scripts/reconcile-release-baseline",
            "release_assets='codexy-marketplace-plugin.tar.gz codexy-runtime-package.tar.gz runtime-release-receipt.json'",
        ],
    );
    support::assert_structured_absent_literals(
        &publish,
        "release draft reconciliation",
        &["--draft=false"],
    );
    support::assert_structured_literals(
        &baseline,
        "release baseline identity",
        &[
            "test \"$(jq -r .targetCommitish release-state.json)\" = \"$ACTIVATION_COMMIT\"",
            "existing_baseline=\"$(mktemp -d)\"",
            "BASELINE_CREATED=true",
        ],
    );
    support::assert_structured_literals(&attestation, "release baseline attestation total", &["gh api --paginate --slurp", "--source-digest \"$ACTIVATION_COMMIT\" --deny-self-hosted-runners"]);
    Ok(())
}

#[test]
fn finalization_verifies_all_attested_assets_before_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let finalizer = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/finalize-verified-release"),
    )?;
    let attestation = fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/verify-release-attestation-total"))?;
    support::assert_structured_literals(
        &finalizer,
        "attested release finalization",
        &[
            "runtime-release-receipt.json release-baseline.json",
            "final_release=\"$(mktemp -d)\"",
            "scripts/verify-release-attestation-total \"$final_release/$asset\" 1",
            "gh release edit \"$RELEASE_TAG\" --draft=false",
        ],
    );
    support::assert_structured_literals(&attestation, "release attestation total", &["gh api --paginate --slurp", "--source-digest \"$ACTIVATION_COMMIT\" --deny-self-hosted-runners"]);
    let publish = finalizer.find("gh release edit \"$RELEASE_TAG\" --draft=false").ok_or("public release")?;
    let verification = finalizer.find("scripts/verify-release-attestation-set").ok_or("attestation verification")?;
    assert!(verification < publish, "release must be authenticated before publication");
    Ok(())
}

#[test]
fn edited_release_verifier_accepts_only_a_body_change_from_an_authenticated_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    edit_baseline::verify()
}
