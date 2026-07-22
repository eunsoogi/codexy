use std::{collections::BTreeMap, fs, path::PathBuf};

use anyhow::{Result, bail};
use serde_json::{Value, json};

use super::{activate, apply_with, prepare};

const WRAPPERS: [&str; 2] = [
    "plugins/codexy/mcp/codexy-mcp-lsp",
    "plugins/codexy/mcp/codexy-mcp-codegraph",
];

#[test]
fn activation_writes_only_the_derived_release_and_pins() -> Result<()> {
    let fixture = Fixture::new()?;
    assert_eq!(activate(&fixture.root, "1.2.2", &fixture.receipt)?, 3);
    let release: Value = serde_json::from_str(&fs::read_to_string(fixture.release())?)?;
    assert_eq!(release["state"], "candidate-proven");
    assert_eq!(release["source"]["commit"], "a".repeat(40));
    assert_eq!(
        release["platforms"]["darwin-arm64"]["lsp"]["path"],
        "runtime/codexy-mcp-lsp-darwin-arm64.bin"
    );
    for wrapper in fixture.wrappers() {
        assert!(fs::read_to_string(wrapper)?.contains("getcodexy==1.2.2"));
    }
    Ok(())
}

#[test]
fn injected_staging_failure_leaves_targets_byte_identical() -> Result<()> {
    let fixture = Fixture::new()?;
    let before = fixture.tracked()?;
    let updates = prepare(&fixture.root, "1.2.2", &fixture.receipt)?;
    assert!(apply_with(&updates, |_| bail!("injected staging failure")).is_err());
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
        fs::create_dir_all(&mcp)?;
        fs::write(
            root.join("plugins/codexy/runtime-release.json"),
            "{\"old\":true}\n",
        )?;
        for (path, server) in WRAPPERS.into_iter().zip(["lsp", "codegraph"]) {
            fs::write(
                root.join(path),
                format!(
                    "#!/bin/sh\nexec uvx --from getcodexy==0.0.1 codexy-mcp-runtime {server} -- \"$@\"\n"
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
    fn wrappers(&self) -> impl Iterator<Item = PathBuf> + '_ {
        WRAPPERS.into_iter().map(|path| self.root.join(path))
    }
    fn tracked(&self) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
        self.wrappers()
            .chain(std::iter::once(self.release()))
            .map(|path| Ok((path.clone(), fs::read(path)?)))
            .collect()
    }
}

fn receipt_value() -> Value {
    let digest = "b".repeat(64);
    json!({
        "schema": "codexy-runtime-candidate-receipt/v1",
        "candidate": {
            "schema": "codexy-runtime-candidate/v1",
            "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": "a".repeat(40)},
            "artifact": {"tag": "runtime-candidate-1.3.0"},
            "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
            "platforms": {
                "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": digest}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
                "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}}
            }
        },
        "artifact": {"url": "https://github.com/eunsoogi/codexy/releases/download/runtime-candidate-1.3.0/codexy-marketplace-plugin.tar.gz", "sha256": "f".repeat(64), "payloadManifestSha256": "1".repeat(64)},
        "provenance": {"repositoryId": 1269350143, "workflowPath": ".github/workflows/runtime-candidate.yml", "runId": 42, "runAttempt": 1, "workflowRunUrl": "https://github.com/eunsoogi/codexy/actions/runs/42"}
    })
}
