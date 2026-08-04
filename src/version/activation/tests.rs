use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Result, bail};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::{activate, apply_with, canonical, prepare};

const WRAPPERS: [&str; 2] = [
    "plugins/codexy/mcp/codexy-mcp-lsp",
    "plugins/codexy/mcp/codexy-mcp-codegraph",
];

#[test]
fn activation_preserves_the_prior_public_runtime_until_final_release() -> Result<()> {
    let fixture = Fixture::new()?;
    assert_eq!(activate(&fixture.root, "1.3.0", &fixture.receipt)?, 4);
    assert_eq!(
        fs::read_to_string(fixture.release())?,
        r#"{"artifact":{"tag":"v1.2.2"}}"#
    );
    assert!(!fixture.candidate().exists());
    assert_eq!(
        fs::read(fixture.record())?,
        serde_json::to_vec(&canonical(receipt_value()))?
    );
    for wrapper in fixture.wrappers() {
        let wrapper = fs::read_to_string(wrapper)?;
        assert!(wrapper.contains("getcodexy==0.0.1"));
        assert!(wrapper.contains("bundled_platforms=\"darwin-arm64 linux-x86_64\""));
    }
    let manifest: Value = serde_json::from_str(&fs::read_to_string(fixture.manifest())?)?;
    assert_eq!(
        manifest["supportedPlatforms"],
        json!(["darwin-arm64", "linux-x86_64"])
    );
    assert_eq!(
        fs::read_to_string(fixture.bootstrap())?,
        "pub(super) const VERSION: &str = \"1.3.0\";\npub(super) const CANDIDATE_VERSION: &str = \"1.3.0\";\n"
    );
    Ok(())
}

#[test]
fn activation_updates_the_publication_identity_without_repointing_runtime() -> Result<()> {
    let fixture = Fixture::new()?;
    assert_eq!(activate(&fixture.root, "1.3.0", &fixture.receipt)?, 4);
    let publish: Value = serde_json::from_str(&fs::read_to_string(fixture.publish())?)?;
    assert_eq!(publish["bootstrap"]["selectedVersion"], "1.3.0");
    assert_eq!(publish["runtime"]["selectedTag"], "v1.3.0");
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
fn selected_bootstrap_cannot_activate_a_candidate() -> Result<()> {
    let fixture = Fixture::new()?;
    assert_activation_rejected_without_mutation(&fixture, "1.2.2")
}

#[test]
fn stale_selected_bootstrap_metadata_cannot_activate_and_leaves_targets_byte_identical()
-> Result<()> {
    let fixture = Fixture::new()?;
    fs::write(
        fixture.bootstrap(),
        "pub(super) const VERSION: &str = \"1.1.0\";\npub(super) const CANDIDATE_VERSION: &str = \"1.3.0\";\n",
    )?;
    assert_activation_rejected_without_mutation(&fixture, "1.3.0")
}

#[test]
fn mismatched_candidate_digest_leaves_targets_byte_identical() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut receipt = receipt_value();
    receipt["artifact"]["payloadManifestSha256"] = json!("0".repeat(64));
    fs::write(&fixture.receipt, serde_json::to_vec(&receipt)?)?;
    assert_activation_rejected_without_mutation(&fixture, "1.3.0")
}

#[test]
fn mismatched_staging_run_attempt_leaves_targets_byte_identical() -> Result<()> {
    let fixture = Fixture::new()?;
    let mut receipt = receipt_value();
    receipt["candidate"]["artifact"]["stagingRunAttempt"] = json!(2);
    fs::write(&fixture.receipt, serde_json::to_vec(&receipt)?)?;
    assert_activation_rejected_without_mutation(&fixture, "1.3.0")
}

#[test]
fn mismatched_selected_publish_identity_leaves_targets_byte_identical() -> Result<()> {
    let fixture = Fixture::new()?;
    fs::write(
        fixture.publish(),
        r#"{"bootstrap":{"selectedVersion":"1.2.1"},"runtime":{"selectedTag":"v1.2.2"}}"#,
    )?;
    assert_activation_rejected_without_mutation(&fixture, "1.3.0")
}

#[test]
fn injected_staging_failure_leaves_targets_byte_identical() -> Result<()> {
    let fixture = Fixture::new()?;
    let before = fixture.tracked()?;
    let updates = prepare(&fixture.root, "1.3.0", &fixture.receipt)?;
    assert!(apply_with(&updates, |_| bail!("injected staging failure")).is_err());
    assert_eq!(fixture.tracked()?, before);
    Ok(())
}

fn assert_activation_rejected_without_mutation(fixture: &Fixture, version: &str) -> Result<()> {
    let before = fixture.tracked()?;
    assert!(activate(&fixture.root, version, &fixture.receipt).is_err());
    assert_eq!(fixture.tracked()?, before);
    Ok(())
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    receipt: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("repo");
        let mcp = root.join("plugins/codexy/mcp");
        fs::create_dir_all(root.join("src/version"))?;
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::create_dir_all(root.join("plugins/codexy/.codex-plugin"))?;
        fs::create_dir_all(&mcp)?;
        fs::write(
            root.join("src/version/bootstrap.rs"),
            "pub(super) const VERSION: &str = \"1.3.0\";\npub(super) const CANDIDATE_VERSION: &str = \"1.3.0\";\n",
        )?;
        fs::write(
            root.join("plugins/codexy/runtime-release.json"),
            r#"{"artifact":{"tag":"v1.2.2"}}"#,
        )?;
        fs::write(
            root.join(".agents/plugins/release-publish-contract.json"),
            r#"{"bootstrap":{"selectedVersion":"1.3.0"},"runtime":{"selectedTag":"v1.2.2","platforms":["darwin-arm64","linux-x86_64"]},"package":{"platforms":["darwin-arm64","linux-x86_64"]}}"#,
        )?;
        fs::write(
            root.join("plugins/codexy/.codex-plugin/plugin.json"),
            r#"{"supportedPlatforms":["darwin-arm64","linux-x86_64"]}"#,
        )?;
        fs::write(
            root.join(".agents/plugins/marketplace.json"),
            r#"{"plugins":[{"supportedPlatforms":["darwin-arm64","linux-x86_64"]}]}"#,
        )?;
        for (path, server) in WRAPPERS.into_iter().zip(["lsp", "codegraph"]) {
            fs::write(
                root.join(path),
                format!(
                    "#!/bin/sh\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\nexec uvx --from getcodexy==0.0.1 codexy-mcp-runtime {server} -- \"$@\"\n"
                ),
            )?;
        }
        let receipt = root.join("receipt.json");
        fs::write(&receipt, serde_json::to_string(&receipt_value())?)?;
        Ok(Self {
            _temp: temp,
            root,
            receipt,
        })
    }

    fn release(&self) -> PathBuf {
        self.root.join("plugins/codexy/runtime-release.json")
    }
    fn publish(&self) -> PathBuf {
        self.root
            .join(".agents/plugins/release-publish-contract.json")
    }
    fn candidate(&self) -> PathBuf {
        self.root.join("plugins/codexy/runtime-candidate.json")
    }
    fn record(&self) -> PathBuf {
        self.root.join(".agents/plugins/runtime-activation.json")
    }
    fn bootstrap(&self) -> PathBuf {
        self.root.join("src/version/bootstrap.rs")
    }
    fn manifest(&self) -> PathBuf {
        self.root.join("plugins/codexy/.codex-plugin/plugin.json")
    }
    fn wrappers(&self) -> impl Iterator<Item = PathBuf> + '_ {
        WRAPPERS.into_iter().map(|path| self.root.join(path))
    }
    fn tracked(&self) -> Result<BTreeMap<PathBuf, Option<Vec<u8>>>> {
        self.wrappers()
            .chain(std::iter::once(self.release()))
            .chain(std::iter::once(self.publish()))
            .chain(std::iter::once(self.candidate()))
            .chain(std::iter::once(self.record()))
            .chain(std::iter::once(self.bootstrap()))
            .chain(std::iter::once(self.manifest()))
            .chain(std::iter::once(
                self.root.join(".agents/plugins/marketplace.json"),
            ))
            .map(|path| Ok((path.clone(), fs::read(path).ok())))
            .collect()
    }
}

fn receipt_value() -> Value {
    let digest = "b".repeat(64);
    let candidate = json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": "a".repeat(40)},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {
            "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": digest}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
            "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}},
            "windows-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-windows-x86_64.exe", "sha256": "9".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-windows-x86_64.exe", "sha256": "a".repeat(64)}}
        }
    });
    let payload_sha = format!(
        "{:x}",
        Sha256::digest(canonical(candidate.clone()).to_string().into_bytes())
    );
    json!({
        "schema": "codexy-runtime-candidate-receipt/v1",
        "candidate": candidate,
        "artifact": {"sha256": "f".repeat(64), "payloadManifestSha256": payload_sha},
        "provenance": {"repositoryId": 1_269_350_143, "workflowPath": ".github/workflows/runtime-candidate.yml", "runId": 42, "runAttempt": 1, "workflowRunUrl": "https://github.com/eunsoogi/codexy/actions/runs/42"}
    })
}
