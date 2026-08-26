use std::fs;

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, json};

use super::{activate, apply_with, canonical, prepare};

#[path = "fixture.rs"]
mod fixture;
#[path = "strict_json_tests.rs"]
mod strict_json_tests;

use fixture::{
    Fixture, assert_activation_rejected_without_mutation, candidate_version, next_patch_version,
    receipt_value, write,
};

#[test]
fn activation_promotes_the_authenticated_source_selected_runtime_pointer() -> Result<()> {
    let fixture = new_fixture()?;
    assert_eq!(
        activate(&fixture.root, candidate_version(), &fixture.receipt)?,
        6
    );
    let release: Value = serde_json::from_slice(&fs::read(
        fixture.path("plugins/codexy-devtools/runtime-release.json"),
    )?)?;
    assert_eq!(release["state"], "source-selected");
    assert_eq!(release["source"], receipt_value()["candidate"]["source"]);
    assert_eq!(
        release["artifact"]["tag"],
        format!("v{}", candidate_version())
    );
    assert_eq!(release["artifact"]["sha256"], "f".repeat(64));
    assert_eq!(
        release["platforms"].as_object().map(|map| map.len()),
        Some(2)
    );
    assert_eq!(
        release["classes"]["devtoolsMcp"]["platforms"],
        release["platforms"]
    );
    assert!(
        !fixture
            .path("plugins/codexy-devtools/runtime-candidate.json")
            .exists()
    );
    assert_eq!(
        fs::read(fixture.path(".agents/plugins/runtime-activation.json"))?,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&canonical(receipt_value()))?
        )
        .into_bytes()
    );
    let activated: Value = serde_json::from_slice(&fs::read(
        fixture.path(".agents/plugins/runtime-activation.json"),
    )?)?;
    assert_eq!(
        activated["candidate"]["classes"]["coreHandoff"]["manifest"]["path"],
        "handoff-runtime.json"
    );
    let wrapper =
        fs::read_to_string(fixture.path("plugins/codexy-devtools/mcp/codexy-mcp-devtools"))?;
    assert!(wrapper.contains(&format!("getcodexy=={}", candidate_version())));
    assert!(!wrapper.contains(&format!("getcodexy=={}", fixture.prior_runtime_version())));
    assert!(wrapper.contains("bundled_platforms=\"darwin-arm64 linux-x86_64\""));
    let manifest: Value = serde_json::from_str(&fs::read_to_string(
        fixture.path("plugins/codexy-devtools/.codex-plugin/plugin.json"),
    )?)?;
    assert_eq!(
        manifest["supportedPlatforms"],
        json!(["darwin-arm64", "linux-x86_64"])
    );
    assert_eq!(
        fs::read_to_string(fixture.path("packages/codexy-runtime/src/version/bootstrap.rs"))?,
        format!(
            "pub(super) const VERSION: &str = \"{}\";\npub(super) const CANDIDATE_VERSION: &str = \"{}\";\n",
            candidate_version(),
            candidate_version()
        )
    );
    Ok(())
}

#[test]
fn activation_updates_the_publication_identity_without_repointing_runtime() -> Result<()> {
    let fixture = new_fixture()?;
    assert_eq!(
        activate(&fixture.root, candidate_version(), &fixture.receipt)?,
        6
    );
    let publish: Value = serde_json::from_str(&fs::read_to_string(
        fixture.path(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(publish["bootstrap"]["selectedVersion"], candidate_version());
    assert_eq!(
        publish["runtime"]["selectedTag"],
        format!("v{}", candidate_version())
    );
    assert_eq!(
        publish["runtime"]["platforms"],
        json!(["darwin-arm64", "linux-x86_64"])
    );
    assert_eq!(
        publish["package"]["platforms"],
        json!(["darwin-arm64", "linux-x86_64"])
    );
    Ok(())
}

#[test]
fn non_candidate_bootstrap_version_cannot_activate() -> Result<()> {
    let fixture = new_fixture()?;
    let non_candidate = next_patch_version(candidate_version())?;
    assert_activation_rejected_without_mutation(&fixture, &non_candidate)
}

#[test]
fn stale_selected_bootstrap_metadata_cannot_activate_and_leaves_targets_byte_identical()
-> Result<()> {
    reject_activation(candidate_version(), |fixture| {
        let stale = next_patch_version(candidate_version())?;
        write(
            &fixture.root,
            "packages/codexy-runtime/src/version/bootstrap.rs",
            format!(
                "pub(super) const VERSION: &str = \"{stale}\";\npub(super) const CANDIDATE_VERSION: &str = \"{}\";\n",
                candidate_version(),
            ),
        )
    })
}

#[test]
fn mismatched_candidate_digest_leaves_targets_byte_identical() -> Result<()> {
    reject_activation(candidate_version(), |fixture| {
        let mut receipt = receipt_value();
        receipt["artifact"]["payloadManifestSha256"] = json!("0".repeat(64));
        fs::write(&fixture.receipt, serde_json::to_vec(&receipt)?)?;
        Ok(())
    })
}

#[test]
fn mismatched_staging_run_attempt_leaves_targets_byte_identical() -> Result<()> {
    reject_activation(candidate_version(), |fixture| {
        let mut receipt = receipt_value();
        receipt["candidate"]["artifact"]["stagingRunAttempt"] = json!(2);
        fs::write(&fixture.receipt, serde_json::to_vec(&receipt)?)?;
        Ok(())
    })
}

#[test]
fn partial_core_aware_tree_is_rejected_before_mutation() -> Result<()> {
    reject_activation(candidate_version(), |fixture| {
        fs::remove_file(
            fixture.path("plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.cmd"),
        )?;
        Ok(())
    })
}

#[test]
fn candidate_without_core_handoff_class_is_rejected_before_mutation() -> Result<()> {
    reject_activation(candidate_version(), |fixture| {
        let mut receipt = receipt_value();
        receipt["candidate"]
            .as_object_mut()
            .context("candidate object")?
            .remove("classes");
        fs::write(&fixture.receipt, serde_json::to_vec(&receipt)?)?;
        Ok(())
    })
}

#[test]
fn mismatched_selected_publish_identity_leaves_targets_byte_identical() -> Result<()> {
    reject_activation(candidate_version(), |fixture| {
        let mut contract: Value = serde_json::from_str(&fs::read_to_string(
            fixture.path(".agents/plugins/release-publish-contract.json"),
        )?)?;
        contract["bootstrap"]["selectedVersion"] = json!(next_patch_version(candidate_version())?);
        write(
            &fixture.root,
            ".agents/plugins/release-publish-contract.json",
            format!("{}\n", serde_json::to_string_pretty(&contract)?),
        )
    })
}

#[test]
fn injected_staging_failure_leaves_targets_byte_identical() -> Result<()> {
    let fixture = new_fixture()?;
    let wrapper_path = fixture.path("plugins/codexy-devtools/mcp/codexy-mcp-devtools");
    let wrapper_before = fs::read(&wrapper_path)?;
    let before = fixture.tracked()?;
    let updates = prepare(&fixture.root, candidate_version(), &fixture.receipt)?;
    assert!(apply_with(&updates, |_| bail!("injected staging failure")).is_err());
    assert_eq!(fixture.tracked()?, before);
    assert_eq!(fs::read(wrapper_path)?, wrapper_before);
    Ok(())
}

fn reject_activation(version: &str, mutate: impl FnOnce(&Fixture) -> Result<()>) -> Result<()> {
    let fixture = new_fixture()?;
    mutate(&fixture)?;
    assert_activation_rejected_without_mutation(&fixture, version)
}

fn new_fixture() -> Result<Fixture> {
    let fixture = Fixture::new()?;
    write(
        &fixture.root,
        "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
        format!(
            "#!/bin/sh\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\nexec uvx --from getcodexy=={} codexy-mcp-runtime lsp -- \"$@\"\n",
            fixture.prior_runtime_version()
        ),
    )?;
    Ok(fixture)
}
