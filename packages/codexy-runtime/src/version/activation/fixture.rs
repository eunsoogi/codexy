use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, Result};
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::super::canonical;

const WRAPPERS: [&str; 2] = [
    "plugins/codexy-devtools/mcp/codexy-mcp-lsp",
    "plugins/codexy-devtools/mcp/codexy-mcp-codegraph",
];

pub(super) struct Fixture {
    _temp: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) receipt: PathBuf,
    prior_runtime_release: String,
    prior_runtime_version: String,
}

pub(super) fn candidate_version() -> &'static str {
    super::super::super::bootstrap::CANDIDATE_VERSION
}

fn selected_version() -> &'static str {
    super::super::super::bootstrap::VERSION
}

pub(super) fn next_patch_version(version: &str) -> Result<String> {
    let (major, remainder) = version.split_once('.').context("major version")?;
    let (minor, patch) = remainder.split_once('.').context("minor version")?;
    let patch = patch
        .parse::<u64>()?
        .checked_add(1)
        .context("patch overflow")?;
    Ok(format!("{major}.{minor}.{patch}"))
}

pub(super) fn write(root: &Path, relative: &str, contents: impl AsRef<[u8]>) -> Result<()> {
    fs::write(root.join(relative), contents)?;
    Ok(())
}

pub(super) fn assert_activation_rejected_without_mutation(
    fixture: &Fixture,
    version: &str,
) -> Result<()> {
    let before = fixture.tracked()?;
    assert!(super::super::activate(&fixture.root, version, &fixture.receipt).is_err());
    assert_eq!(fixture.tracked()?, before);
    Ok(())
}

impl Fixture {
    pub(super) fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("repo");
        let mcp = root.join("plugins/codexy-devtools/mcp");
        let prior_runtime_release = fs::read_to_string(
            crate::paths::repository_root().join("plugins/codexy-devtools/runtime-release.json"),
        )?;
        let release: Value = serde_json::from_str(&prior_runtime_release)?;
        let prior_runtime_version = release["artifact"]["tag"]
            .as_str()
            .and_then(|tag| tag.strip_prefix('v'))
            .context("prior runtime version")?
            .to_owned();
        fs::create_dir_all(root.join("packages/codexy-runtime/src/version"))?;
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::create_dir_all(root.join("plugins/codexy-devtools/.codex-plugin"))?;
        fs::create_dir_all(root.join("plugins/codexy/skills/dreaming/references"))?;
        fs::create_dir_all(root.join("plugins/codexy/skills/dreaming/scripts"))?;
        fs::create_dir_all(&mcp)?;
        write(
            &root,
            "packages/codexy-runtime/src/version/bootstrap.rs",
            format!(
                "pub(super) const VERSION: &str = \"{}\";\npub(super) const CANDIDATE_VERSION: &str = \"{}\";\n",
                selected_version(),
                candidate_version()
            ),
        )?;
        for (path, contents) in [
            (
                "plugins/codexy/skills/dreaming/references/handoff-runtime.schema.json",
                "{}",
            ),
            (
                "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.sh",
                "#!/bin/sh\n",
            ),
            (
                "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.cmd",
                "@echo off\n",
            ),
            (
                "plugins/codexy/skills/dreaming/scripts/resumable_context_capsule.py",
                "#!/usr/bin/env python3\n",
            ),
            (
                "plugins/codexy-devtools/runtime-release.json",
                prior_runtime_release.as_str(),
            ),
            (
                ".agents/plugins/release-publish-contract.json",
                &serde_json::to_string(&json!({
                    "bootstrap": {"selectedVersion": selected_version()},
                    "runtime": {
                        "selectedTag": release["artifact"]["tag"],
                        "platforms": ["darwin-arm64", "linux-x86_64"]
                    },
                    "package": {"platforms": ["darwin-arm64", "linux-x86_64"]}
                }))?,
            ),
            (
                "plugins/codexy-devtools/.codex-plugin/plugin.json",
                r#"{"supportedPlatforms":["darwin-arm64","linux-x86_64"]}"#,
            ),
            (
                ".agents/plugins/marketplace.json",
                r#"{"plugins":[{"supportedPlatforms":["darwin-arm64","linux-x86_64"]}]}"#,
            ),
        ] {
            write(&root, path, contents)?;
        }
        for (path, server) in WRAPPERS.into_iter().zip(["lsp", "codegraph"]) {
            write(
                &root,
                path,
                format!(
                    "#!/bin/sh\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\nexec uvx --from getcodexy=={prior_runtime_version} codexy-mcp-runtime {server} -- \"$@\"\n"
                ),
            )?;
        }
        let receipt = root.join("receipt.json");
        fs::write(&receipt, serde_json::to_string(&receipt_value())?)?;
        Ok(Self {
            _temp: temp,
            root,
            receipt,
            prior_runtime_release,
            prior_runtime_version,
        })
    }

    pub(super) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub(super) fn wrappers(&self) -> impl Iterator<Item = PathBuf> + '_ {
        WRAPPERS.into_iter().map(|path| self.root.join(path))
    }

    pub(super) fn prior_runtime_release(&self) -> &str {
        &self.prior_runtime_release
    }

    pub(super) fn prior_runtime_version(&self) -> &str {
        &self.prior_runtime_version
    }

    pub(super) fn tracked(&self) -> Result<BTreeMap<PathBuf, Option<Vec<u8>>>> {
        self.wrappers()
            .chain(
                [
                    "plugins/codexy-devtools/runtime-release.json",
                    ".agents/plugins/release-publish-contract.json",
                    "plugins/codexy-devtools/runtime-candidate.json",
                    ".agents/plugins/runtime-activation.json",
                    "packages/codexy-runtime/src/version/bootstrap.rs",
                    "plugins/codexy-devtools/.codex-plugin/plugin.json",
                ]
                .into_iter()
                .map(|path| self.path(path)),
            )
            .chain(std::iter::once(
                self.path(".agents/plugins/marketplace.json"),
            ))
            .map(|path| Ok((path.clone(), fs::read(path).ok())))
            .collect()
    }
}

pub(super) fn receipt_value() -> Value {
    let digest = "b".repeat(64);
    let candidate = json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": "a".repeat(40), "tree": "1".repeat(40)},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {
            "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": digest}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
            "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}},
            "windows-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-windows-x86_64.exe", "sha256": "9".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-windows-x86_64.exe", "sha256": "a".repeat(64)}}
        },
        "classes": core_classes(),
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

fn core_classes() -> Value {
    json!({
        "devtoolsMcp": {"platforms": {
            "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": "b".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
            "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}},
            "windows-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-windows-x86_64.exe", "sha256": "9".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-windows-x86_64.exe", "sha256": "a".repeat(64)}}
        }},
        "coreHandoff": {
            "manifest": {"path": "handoff-runtime.json", "sha256": "2".repeat(64)},
            "platforms": {
                "darwin-arm64": {"path": "runtime/codexy-handoff-validate-darwin-arm64.bin", "sha256": "3".repeat(64), "kind": "mach-o"},
                "linux-x86_64": {"path": "runtime/codexy-handoff-validate-linux-x86_64.bin", "sha256": "4".repeat(64), "kind": "elf"},
                "windows-x86_64": {"path": "runtime/codexy-handoff-validate-windows-x86_64.exe", "sha256": "5".repeat(64), "kind": "pe"}
            }
        }
    })
}
