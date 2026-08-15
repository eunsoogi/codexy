use std::fs;

use crate::support::{
    FixtureArgumentDomain, FixtureScriptBinding, ReleaseFixtureCommand, ReleaseFixtureOutcome,
    bind_posix_fixture_script_launchers, bind_posix_fixture_shell_launchers,
    fixture_github_argv_adapter_path, fixture_script_interpreter_path,
};
use sha2::{Digest, Sha256};

use super::release_reconciliation_assertions::assert_rejected;

pub(super) fn verify() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let scripts = temp.path().join("scripts");
    fs::create_dir(&scripts)?;
    for name in [
        "verify-release-edit-baseline",
        "verify-release-attestation-set",
        "verify-release-attestation-total",
    ] {
        let destination = scripts.join(name);
        fs::copy(root.join("scripts").join(name), &destination)?;
        crate::support::make_executable(&destination)?;
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
    let bin = temp.path().join("bin");
    fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    fs::write(&gh, r#"#!/bin/sh
test "${CODEXY_FIXTURE_GH_TRANSPORT:-}" = 1 || exit 2
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
    crate::support::make_executable(&gh)?;
    for name in [
        "verify-release-edit-baseline",
        "verify-release-attestation-set",
        "verify-release-attestation-total",
    ] {
        bind_posix_fixture_shell_launchers(
            &scripts.join(name),
            &[("gh", "FIXTURE_GH", "FIXTURE_GH_LAUNCHER", FixtureArgumentDomain::GitHubApi {
                adapter_launcher_environment: "FIXTURE_GH_ADAPTER_LAUNCHER",
            })],
        )?;
    }
    let gh_launcher = fixture_script_interpreter_path(&gh)?;
    let gh_adapter = fixture_github_argv_adapter_path(
        &scripts.join("verify-release-edit-baseline"),
    );
    let gh_adapter_launcher = fixture_script_interpreter_path(&gh_adapter)?;
    bind_posix_fixture_script_launchers(
        &scripts.join("verify-release-edit-baseline"),
        "FIXTURE_POSIX_SHELL",
        "FIXTURE_SCRIPT_ROOT",
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
    let run = |extra_attestation: bool| ReleaseFixtureCommand::new(&script)
        .current_dir(temp.path()).path("FIXTURE_DIR", &fixture).scalar("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .payload_path("FIXTURE_GH", &gh)
        .payload_path("FIXTURE_GH_LAUNCHER", &gh_launcher)
        .path("FIXTURE_GH_ADAPTER_LAUNCHER", &gh_adapter_launcher)
        .path("FIXTURE_POSIX_SHELL", &shell_launcher)
        .path("FIXTURE_SCRIPT_ROOT", temp.path())
        .path("GITHUB_EVENT_PATH", temp.path().join("event.json"))
        .scalar("EXTRA_ATTESTATION", extra_attestation.to_string())
        .output().map_err(|error| -> Box<dyn std::error::Error> { error.into() });
    let state = fs::read(fixture.join("state.json"))?;
    let baseline_bytes = fs::read(fixture.join("baseline.json"))?;
    let verified = run(false)?;
    ReleaseFixtureCommand::assert_outcome(
        "verify-release-edit-baseline",
        ReleaseFixtureOutcome::Success,
        &verified,
    );
    for (name, changed_event) in [
        ("name", r#"{"action":"edited","changes":{"name":{"from":"old"}},"release":{"id":42}}"#),
        ("body and name", r#"{"action":"edited","changes":{"body":{"from":"old"},"name":{"from":"old"}},"release":{"id":42}}"#),
        ("empty", r#"{"action":"edited","changes":{},"release":{"id":42}}"#),
    ] {
        fs::write(temp.path().join("event.json"), changed_event)?;
        assert_rejected(&format!("verify-release-edit-baseline changed {name}"), &run(false)?);
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
        assert_rejected(&format!("verify-release-edit-baseline release {name}"), &run(false)?);
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
        assert_rejected(&format!("verify-release-edit-baseline baseline {name}"), &run(false)?);
    }
    fs::write(fixture.join("baseline.json"), &baseline_bytes)?;
    assert_rejected("verify-release-edit-baseline extra attestation", &run(true)?);
    Ok(())
}
