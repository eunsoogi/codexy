use std::fs;

use crate::support;

#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use sha2::{Digest, Sha256};

#[cfg(unix)]
#[path = "release_reconciliation/release_attestation_reconciliation.rs"]
mod release_attestation_reconciliation;

#[test]
fn release_reconciliation_authenticates_a_draft_before_finalization()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let publish = fs::read_to_string(root.join("scripts/publish-verified-release"))?;
    let baseline = fs::read_to_string(root.join("scripts/reconcile-release-baseline"))?;
    let attestation_set = fs::read_to_string(root.join("scripts/verify-release-attestation-set"))?;
    support::assert_structured_literals(
        &publish,
        "release draft reconciliation",
        &[
            "release_create_response=\"$(gh api --method POST",
            "repos/$GITHUB_REPOSITORY/releases\" -f \"tag_name=$RELEASE_TAG\"",
            "jq -er .id",
            "uploadUrl: .upload_url",
            "\"$upload_url?name=$asset\"",
            "scripts/reconcile-release-baseline",
            "release_assets='codexy-marketplace-plugin.tar.gz codexy-marketplace-bundle.tar.gz codexy-runtime-package.tar.gz runtime-release-receipt.json'",
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
            "attestation_policies()",
            "runtime-candidate.yml",
            "RELEASE_ID",
            "releases/$RELEASE_ID",
            "releases/assets/$asset_id",
            "BASELINE_CREATED=true",
        ],
    );
    support::assert_structured_literals(&attestation_set, "per-subject attestation verification", &["gh attestation verify", "runtime-candidate.yml", "source_digest=\"$STAGING_SOURCE_COMMIT\"", "release-baseline.json"]);
    Ok(())
}

#[test]
fn finalization_verifies_all_attested_assets_before_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let finalizer = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/finalize-verified-release"),
    )?;
    support::assert_structured_literals(
        &finalizer,
        "attested release finalization",
        &[
            "runtime-release-receipt.json release-baseline.json",
            "final_release=\"$(mktemp -d)\"",
            "scripts/verify-release-attestation-set \"$final_release\" final-baseline-attestation.json baseline",
            "gh api --method PATCH",
            "releases/$RELEASE_ID\" -F draft=false",
        ],
    );
    support::assert_structured_absent_literals(
        &finalizer,
        "draft release operations must use numeric identity",
        &[
            "gh release upload \"$RELEASE_TAG\"",
            "gh release download \"$RELEASE_TAG\"",
            "releases/tags/$RELEASE_TAG",
            "gh release edit \"$RELEASE_TAG\" --draft=false",
        ],
    );
    let publish = finalizer.find("gh api --method PATCH").ok_or("public release")?;
    let verification = finalizer.find("scripts/verify-release-attestation-set").ok_or("attestation verification")?;
    assert!(verification < publish, "release must be authenticated before publication");
    Ok(())
}

#[cfg(unix)]
#[test]
fn edited_release_verifier_accepts_only_a_body_change_from_an_authenticated_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let scripts = temp.path().join("scripts");
    fs::create_dir(&scripts)?;
    for name in ["verify-release-edit-baseline", "verify-release-attestation-set"] {
        let destination = scripts.join(name);
        fs::copy(root.join("scripts").join(name), &destination)?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&destination)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&destination, permissions)?;
        }
    }
    let fixture = temp.path().join("fixture");
    fs::create_dir(&fixture)?;
    let commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let staging_commit = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let statement = r#"[{"subject":[{"name":"subject"}]}]"#;
    let fingerprint = format!("{:x}", Sha256::digest(format!("{statement}\n").as_bytes()));
    let runtime_statement = r#"[{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"}]}]"#;
    let runtime_fingerprint = format!("{:x}", Sha256::digest(format!("{runtime_statement}\n").as_bytes()));
    let assets = serde_json::json!([
        {"id": 2, "name": "codexy-marketplace-plugin.tar.gz", "size": 1, "digest": "sha256:marketplace"},
        {"id": 3, "name": "codexy-marketplace-bundle.tar.gz", "size": 1, "digest": "sha256:bundle"},
        {"id": 4, "name": "codexy-runtime-package.tar.gz", "size": 1, "digest": "sha256:runtime"},
        {"id": 5, "name": "runtime-release-receipt.json", "size": 1, "digest": "sha256:receipt"},
        {"id": 1, "name": "release-baseline.json", "size": 1, "digest": "sha256:baseline"}
    ]);
    let baseline = serde_json::json!({
        "schema": "codexy-release-baseline/v2",
        "release": {"id": 42, "name": "v9.9.9", "tagName": "v9.9.9", "targetCommitish": commit, "isDraft": false, "isPrerelease": false},
        "assets": [
            {"name": "codexy-marketplace-bundle.tar.gz", "size": 1, "digest": "sha256:bundle"},
            {"name": "codexy-marketplace-plugin.tar.gz", "size": 1, "digest": "sha256:marketplace"},
            {"name": "codexy-runtime-package.tar.gz", "size": 1, "digest": "sha256:runtime"},
            {"name": "runtime-release-receipt.json", "size": 1, "digest": "sha256:receipt"}
        ],
        "releaseReceiptSha256": "receipt",
        "attestationPolicies": {
            "codexy-marketplace-plugin.tar.gz": {"signerWorkflow": "eunsoogi/codexy/.github/workflows/publish-version-release.yml", "sourceRef": "refs/heads/main", "sourceDigest": commit, "denySelfHostedRunners": true},
            "codexy-marketplace-bundle.tar.gz": {"signerWorkflow": "eunsoogi/codexy/.github/workflows/publish-version-release.yml", "sourceRef": "refs/heads/main", "sourceDigest": commit, "denySelfHostedRunners": true},
            "codexy-runtime-package.tar.gz": {"signerWorkflow": "eunsoogi/codexy/.github/workflows/runtime-candidate.yml", "sourceRef": "refs/heads/main", "sourceDigest": staging_commit, "denySelfHostedRunners": true},
            "runtime-release-receipt.json": {"signerWorkflow": "eunsoogi/codexy/.github/workflows/publish-version-release.yml", "sourceRef": "refs/heads/main", "sourceDigest": commit, "denySelfHostedRunners": true}
        },
        "attestations": [
            {"name": "codexy-marketplace-bundle.tar.gz", "count": 1, "fingerprint": fingerprint},
            {"name": "codexy-marketplace-plugin.tar.gz", "count": 1, "fingerprint": fingerprint},
            {"name": "codexy-runtime-package.tar.gz", "count": 1, "fingerprint": runtime_fingerprint},
            {"name": "runtime-release-receipt.json", "count": 1, "fingerprint": fingerprint}
        ]
    });
    fs::write(fixture.join("baseline.json"), serde_json::to_vec(&baseline)?)?;
    fs::write(fixture.join("state.json"), serde_json::to_vec(&serde_json::json!({
        "id": 42, "name": "v9.9.9", "tag_name": "v9.9.9", "target_commitish": commit,
        "draft": false, "prerelease": false, "assets": assets
    }))?)?;
    fs::write(temp.path().join("event.json"), r#"{"action":"edited","changes":{"body":{"from":"old"}},"release":{"id":42}}"#)?;
    let bin = temp.path().join("bin"); fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    fs::write(&gh, r#"#!/bin/sh
case "$*" in
  *releases/42*) cat "$FIXTURE_DIR/state.json" ;;
  *releases/assets/1*) cat "$FIXTURE_DIR/baseline.json" ;;
  *releases/assets/5*) printf '%s\n' '{"source":{"stagingSourceCommit":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}}' ;;
  *releases/assets/*) printf x ;;
  *attestations/sha256*)
    case "${ATTESTATION_STATE:?}" in
      release|extra) printf '%s\n' '{"attestations":[{},{}]}' ;;
      *) printf '%s\n' '{"attestations":[{}]}' ;;
    esac ;;
  *attestation*codexy-runtime-package.tar.gz*--format\ json*)
    case "${ATTESTATION_STATE:?}" in
      extra) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"}]} }},{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"}]}}}]' ;;
      *) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"codexy-marketplace-plugin.tar.gz"},{"name":"runtime-staging-receipt.json"}]}}}]' ;;
    esac ;;
  *attestation*--format\ json*)
    case "${ATTESTATION_STATE:?}" in
      extra) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"subject"}]}},{"verificationResult":{"statement":{"subject":[{"name":"subject"}]}}}]' ;;
      many-unrelated)
        # The policy match is after the default 30-result window.
        case "$*" in
          *"--limit 1000"*) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"subject"}]}}}]' ;;
          *) printf '%s\n' '[]' ;;
        esac ;;
      *) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"subject"}]}}}]' ;;
    esac ;;
  *attestation*) exit 0 ;;
  *) exit 1 ;;
esac
"#)?;
    #[cfg(unix)] {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&gh)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&gh, permissions)?;
    }
    let run = |attestation_state: &str| Command::new(scripts.join("verify-release-edit-baseline"))
        .current_dir(temp.path()).env("FIXTURE_DIR", &fixture).env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .env("GITHUB_EVENT_PATH", temp.path().join("event.json")).env("ATTESTATION_STATE", attestation_state).env("PATH", format!("{}:{}", bin.display(), std::env::var("PATH")?))
        .output().map_err(|error| -> Box<dyn std::error::Error> { error.into() });
    let state = fs::read(fixture.join("state.json"))?;
    let baseline_bytes = fs::read(fixture.join("baseline.json"))?;
    let verified = run("single")?;
    assert!(verified.status.success(), "stdout: {} stderr: {}", String::from_utf8_lossy(&verified.stdout), String::from_utf8_lossy(&verified.stderr));
    let rejected_states: Vec<(&str, Box<dyn Fn(&mut serde_json::Value)>)> = vec![
        ("release id", Box::new(|state| state["id"] = serde_json::json!(43))),
        ("title", Box::new(|state| state["name"] = serde_json::json!("v9.9.8"))),
        ("tag", Box::new(|state| state["tag_name"] = serde_json::json!("v9.9.8"))),
        ("target", Box::new(|state| state["target_commitish"] = serde_json::json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"))),
        ("draft", Box::new(|state| state["draft"] = serde_json::json!(true))),
        ("prerelease", Box::new(|state| state["prerelease"] = serde_json::json!(true))),
        ("asset digest", Box::new(|state| state["assets"][0]["digest"] = serde_json::json!("sha256:changed"))),
        ("receipt digest", Box::new(|state| state["assets"][3]["digest"] = serde_json::json!("sha256:changed"))),
        ("asset removal", Box::new(|state| { state["assets"].as_array_mut().unwrap().remove(1); })),
        ("extra asset", Box::new(|state| state["assets"].as_array_mut().unwrap().push(serde_json::json!({"id": 5, "name": "unexpected", "size": 1, "digest": "sha256:extra"})))),
    ];
    for (name, mutate) in rejected_states {
        let mut tampered: serde_json::Value = serde_json::from_slice(&state)?;
        mutate(&mut tampered);
        fs::write(fixture.join("state.json"), serde_json::to_vec(&tampered)?)?;
        assert!(!run("single")?.status.success(), "{name} mutation was accepted");
    }
    fs::write(fixture.join("state.json"), &state)?;
    for (name, pointer, replacement) in [
        ("baseline receipt", "/releaseReceiptSha256", serde_json::json!("changed")),
        ("baseline signer", "/attestationPolicies/codexy-marketplace-plugin.tar.gz/signerWorkflow", serde_json::json!("other/workflow")),
        ("baseline fingerprint", "/attestations/0/fingerprint", serde_json::json!("changed")),
    ] {
        let mut tampered: serde_json::Value = serde_json::from_slice(&baseline_bytes)?;
        *tampered.pointer_mut(pointer).ok_or("baseline field")? = replacement;
        fs::write(fixture.join("baseline.json"), serde_json::to_vec(&tampered)?)?;
        assert!(!run("single")?.status.success(), "{name} mutation was accepted");
    }
    fs::write(fixture.join("baseline.json"), &baseline_bytes)?;
    assert!(run("many-unrelated")?.status.success());
    assert!(!run("extra")?.status.success());
    Ok(())
}
