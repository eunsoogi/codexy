use std::fs;

use crate::support::{
    FixtureCommand as Command, FixtureScriptBinding, bind_posix_fixture_script_launchers,
    bind_posix_fixture_shell_launchers, fixture_script_interpreter_path,
};

use crate::support;
use sha2::{Digest, Sha256};

#[path = "release_attestation_reconciliation.rs"]
mod release_attestation_reconciliation;

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
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let scripts = temp.path().join("scripts");
    fs::create_dir(&scripts)?;
    for name in ["verify-release-edit-baseline", "verify-release-attestation-set", "verify-release-attestation-total"] {
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
    let statement = r#"[{"subject":[{"name":"subject"}]}]"#;
    let fingerprint = format!("{:x}", Sha256::digest(format!("{statement}\n").as_bytes()));
    let assets = serde_json::json!([
        {"id": 2, "name": "codexy-marketplace-plugin.tar.gz", "size": 1, "digest": "sha256:marketplace"},
        {"id": 3, "name": "codexy-runtime-package.tar.gz", "size": 1, "digest": "sha256:runtime"},
        {"id": 4, "name": "runtime-release-receipt.json", "size": 1, "digest": "sha256:receipt"},
        {"id": 1, "name": "release-baseline.json", "size": 1, "digest": "sha256:baseline"}
    ]);
    let baseline = serde_json::json!({
        "schema": "codexy-release-baseline/v1",
        "release": {"id": 42, "name": "v9.9.9", "tagName": "v9.9.9", "targetCommitish": commit, "isDraft": false, "isPrerelease": false},
        "assets": [
            {"name": "codexy-marketplace-plugin.tar.gz", "size": 1, "digest": "sha256:marketplace"},
            {"name": "codexy-runtime-package.tar.gz", "size": 1, "digest": "sha256:runtime"},
            {"name": "runtime-release-receipt.json", "size": 1, "digest": "sha256:receipt"}
        ],
        "releaseReceiptSha256": "receipt",
        "attestationPolicy": {"signerWorkflow": "eunsoogi/codexy/.github/workflows/publish-version-release.yml", "sourceRef": "refs/heads/main", "sourceDigest": commit, "denySelfHostedRunners": true},
        "attestations": [
            {"name": "codexy-marketplace-plugin.tar.gz", "count": 1, "fingerprint": fingerprint},
            {"name": "codexy-runtime-package.tar.gz", "count": 1, "fingerprint": fingerprint},
            {"name": "runtime-release-receipt.json", "count": 1, "fingerprint": fingerprint}
        ]
    });
    fs::write(fixture.join("baseline.json"), serde_json::to_vec(&baseline)?)?;
    fs::write(fixture.join("state.json"), serde_json::to_vec(&serde_json::json!({
        "id": 42, "name": "v9.9.9", "tag_name": "v9.9.9", "target_commitish": commit,
        "draft": false, "prerelease": false, "assets": assets
    }))?)?;
    let event = r#"{"action":"edited","changes":{"body":{"from":"old"}},"release":{"id":42}}"#;
    fs::write(temp.path().join("event.json"), event)?;
    let bin = temp.path().join("bin"); fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    fs::write(&gh, r#"#!/bin/sh
case "$*" in
  *releases/42*) cat "$FIXTURE_DIR/state.json" ;;
  *releases/assets/1*) cat "$FIXTURE_DIR/baseline.json" ;;
  *releases/assets/*) printf x ;;
  *attestations/sha256*)
    if test "${EXTRA_ATTESTATION:-false}" = true; then
      printf '%s\n' '{"attestations":[{},{}]}'
    else
      printf '%s\n' '{"attestations":[{}]}'
    fi ;;
  *attestation*--format\ json*)
    if test "${EXTRA_ATTESTATION:-false}" = true; then
      printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"subject"}]}},{"verificationResult":{"statement":{"subject":[{"name":"subject"}]}}}]'
    else
      printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"subject"}]}}}]'
    fi ;;
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
    for name in ["verify-release-edit-baseline", "verify-release-attestation-set", "verify-release-attestation-total"] {
        bind_posix_fixture_shell_launchers(
            &scripts.join(name),
            &[("gh", "FIXTURE_GH", "FIXTURE_GH_LAUNCHER")],
        )?;
    }
    let gh_launcher = fixture_script_interpreter_path(&gh)?;
    bind_posix_fixture_script_launchers(
        &scripts.join("verify-release-edit-baseline"),
        "FIXTURE_POSIX_SHELL",
        &[
            FixtureScriptBinding {
                invocation: "scripts/verify-release-attestation-total release-baseline/release-baseline.json 1",
                child: "scripts/verify-release-attestation-total",
            },
            FixtureScriptBinding {
                invocation: "scripts/verify-release-attestation-set release-assets release-attestations.json",
                child: "scripts/verify-release-attestation-set",
            },
        ],
    )?;
    let shell_launcher = fixture_script_interpreter_path(&scripts.join("verify-release-edit-baseline"))?;
    let script = scripts.join("verify-release-edit-baseline");
    let run = |extra_attestation: bool| Command::new(&script)
        .current_dir(temp.path()).env("FIXTURE_DIR", &fixture).env("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .env_path("FIXTURE_GH", &gh)
        .env_path("FIXTURE_GH_LAUNCHER", &gh_launcher)
        .env_path("FIXTURE_POSIX_SHELL", &shell_launcher)
        .env("GITHUB_EVENT_PATH", temp.path().join("event.json")).env("EXTRA_ATTESTATION", extra_attestation.to_string())
        .output().map_err(|error| -> Box<dyn std::error::Error> { error.into() });
    let state = fs::read(fixture.join("state.json"))?;
    let baseline_bytes = fs::read(fixture.join("baseline.json"))?;
    let verified = run(false)?;
    assert!(verified.status.success(), "stdout: {} stderr: {}", String::from_utf8_lossy(&verified.stdout), String::from_utf8_lossy(&verified.stderr));
    for (name, changed_event) in [
        ("name", r#"{"action":"edited","changes":{"name":{"from":"old"}},"release":{"id":42}}"#),
        ("body and name", r#"{"action":"edited","changes":{"body":{"from":"old"},"name":{"from":"old"}},"release":{"id":42}}"#),
        ("empty", r#"{"action":"edited","changes":{},"release":{"id":42}}"#),
    ] {
        fs::write(temp.path().join("event.json"), changed_event)?;
        assert!(!run(false)?.status.success(), "accepted edited release event with {name}");
    }
    fs::write(temp.path().join("event.json"), event)?;
    let rejected_states: Vec<(&str, Box<dyn Fn(&mut serde_json::Value)>)> = vec![
        ("release id", Box::new(|state| state["id"] = serde_json::json!(43))),
        ("title", Box::new(|state| state["name"] = serde_json::json!("v9.9.8"))),
        ("tag", Box::new(|state| state["tag_name"] = serde_json::json!("v9.9.8"))),
        ("target", Box::new(|state| state["target_commitish"] = serde_json::json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"))),
        ("draft", Box::new(|state| state["draft"] = serde_json::json!(true))),
        ("prerelease", Box::new(|state| state["prerelease"] = serde_json::json!(true))),
        ("asset digest", Box::new(|state| state["assets"][0]["digest"] = serde_json::json!("sha256:changed"))),
        ("receipt digest", Box::new(|state| state["assets"][2]["digest"] = serde_json::json!("sha256:changed"))),
        ("asset removal", Box::new(|state| { state["assets"].as_array_mut().unwrap().remove(1); })),
        ("extra asset", Box::new(|state| state["assets"].as_array_mut().unwrap().push(serde_json::json!({"id": 5, "name": "unexpected", "size": 1, "digest": "sha256:extra"})))),
    ];
    for (name, mutate) in rejected_states {
        let mut tampered: serde_json::Value = serde_json::from_slice(&state)?;
        mutate(&mut tampered);
        fs::write(fixture.join("state.json"), serde_json::to_vec(&tampered)?)?;
        assert!(!run(false)?.status.success(), "{name} mutation was accepted");
    }
    fs::write(fixture.join("state.json"), &state)?;
    for (name, pointer, replacement) in [
        ("baseline receipt", "/releaseReceiptSha256", serde_json::json!("changed")),
        ("baseline signer", "/attestationPolicy/signerWorkflow", serde_json::json!("other/workflow")),
        ("baseline fingerprint", "/attestations/0/fingerprint", serde_json::json!("changed")),
    ] {
        let mut tampered: serde_json::Value = serde_json::from_slice(&baseline_bytes)?;
        *tampered.pointer_mut(pointer).ok_or("baseline field")? = replacement;
        fs::write(fixture.join("baseline.json"), serde_json::to_vec(&tampered)?)?;
        assert!(!run(false)?.status.success(), "{name} mutation was accepted");
    }
    fs::write(fixture.join("baseline.json"), &baseline_bytes)?;
    assert!(!run(true)?.status.success());
    Ok(())
}
