use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
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
}

pub(super) fn candidate_version() -> &'static str {
    super::super::super::bootstrap::CANDIDATE_VERSION
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
        fs::create_dir_all(root.join("packages/codexy-runtime/src/version"))?;
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::create_dir_all(root.join("plugins/codexy-devtools/.codex-plugin"))?;
        fs::create_dir_all(&mcp)?;
        write(
            &root,
            "packages/codexy-runtime/src/version/bootstrap.rs",
            format!(
                "pub(super) const VERSION: &str = \"1.3.0\";\npub(super) const CANDIDATE_VERSION: &str = \"{}\";\n",
                candidate_version()
            ),
        )?;
        for (path, contents) in [
            (
                "plugins/codexy-devtools/runtime-release.json",
                r#"{"artifact":{"tag":"v1.2.2"}}"#,
            ),
            (
                ".agents/plugins/release-publish-contract.json",
                r#"{"bootstrap":{"selectedVersion":"1.3.0"},"runtime":{"selectedTag":"v1.2.2","platforms":["darwin-arm64","linux-x86_64"]},"package":{"platforms":["darwin-arm64","linux-x86_64"]}}"#,
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

    pub(super) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub(super) fn wrappers(&self) -> impl Iterator<Item = PathBuf> + '_ {
        WRAPPERS.into_iter().map(|path| self.root.join(path))
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
