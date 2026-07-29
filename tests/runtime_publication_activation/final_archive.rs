use std::{
    fs,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::workflow;

const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const RUNTIME_ASSET: &str = "codexy-runtime-package.tar.gz";

#[test]
fn final_publisher_materializes_and_exercises_the_public_archive()
-> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    let run = publisher.1;
    for required in [
        "git checkout --detach \"$SOURCE_COMMIT\"",
        "scripts/materialize-runtime-release-archive",
        "codexy-runtime-package.tar.gz",
        "runtime-release-receipt.json",
        "scripts/inspect-release-archive public.tar.gz public-inspect/plugins/codexy",
        "gh attestation verify public-runtime.tar.gz",
    ] {
        assert!(run.contains(required), "final publisher lacks {required}");
    }
    Ok(())
}

#[test]
fn materializer_preserves_staged_runtime_and_activates_metadata()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let output = Command::new(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("scripts/materialize-runtime-release-archive"),
    )
    .args([&fixture.staged_archive, &fixture.final_archive])
    .current_dir(&fixture.root)
    .env("RELEASE_TAG", "v1.3.0")
    .env("SOURCE_COMMIT", COMMIT)
    .output()?;
    assert!(
        output.status.success(),
        "materializer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let extracted = fixture.root.join("extracted");
    fs::create_dir(&extracted)?;
    assert!(
        Command::new("tar")
            .args(["-xzf"])
            .arg(&fixture.final_archive)
            .arg("-C")
            .arg(&extracted)
            .status()?
            .success()
    );
    let plugin = extracted.join("plugins/codexy");
    let manifest: Value =
        serde_json::from_slice(&fs::read(plugin.join(".codex-plugin/plugin.json"))?)?;
    let release: Value =
        serde_json::from_slice(&fs::read(plugin.join("runtime-release.json"))?)?;
    assert_eq!(manifest["version"], "1.3.0");
    assert_eq!(release["artifact"]["tag"], "v1.3.0");
    assert_eq!(
        release["artifact"]["url"],
        format!(
            "https://github.com/eunsoogi/codexy/releases/download/v1.3.0/{RUNTIME_ASSET}"
        )
    );
    assert_eq!(
        fs::read(plugin.join("runtime-candidate.json"))?,
        fixture.candidate
    );
    let runtime = plugin.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
    assert_eq!(fs::read(&runtime)?, fixture.runtime);
    let smoke = Command::new(runtime).arg("--help").output()?;
    assert!(smoke.status.success());
    assert_eq!(smoke.stdout, b"final archive runtime\n");
    Ok(())
}

struct FinalArchiveFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    staged_archive: PathBuf,
    final_archive: PathBuf,
    candidate: Vec<u8>,
    runtime: Vec<u8>,
}

impl FinalArchiveFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().to_path_buf();
        let source = root.join("plugins/codexy");
        let staged = root.join("staged/plugins/codexy");
        for plugin in [&source, &staged] {
            fs::create_dir_all(plugin.join(".codex-plugin"))?;
            fs::create_dir_all(plugin.join("runtime"))?;
        }
        let candidate = serde_json::to_vec(&candidate())?;
        fs::write(source.join("runtime-candidate.json"), &candidate)?;
        fs::write(staged.join("runtime-candidate.json"), &candidate)?;
        fs::write(
            source.join(".codex-plugin/plugin.json"),
            b"{\"name\":\"codexy\",\"version\":\"1.3.0\"}\n",
        )?;
        fs::write(
            staged.join(".codex-plugin/plugin.json"),
            b"{\"name\":\"codexy\",\"version\":\"1.2.2\"}\n",
        )?;
        let runtime = b"#!/bin/sh\nprintf 'final archive runtime\\n'\n".to_vec();
        let runtime_path = staged.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
        fs::write(&runtime_path, &runtime)?;
        fs::set_permissions(&runtime_path, fs::Permissions::from_mode(0o755))?;
        let staged_archive = root.join("staging.tar.gz");
        assert!(
            Command::new("tar")
                .env("COPYFILE_DISABLE", "1")
                .args(["-C"])
                .arg(root.join("staged"))
                .args(["-czf"])
                .arg(&staged_archive)
                .arg("plugins/codexy")
                .status()?
                .success()
        );
        let staged_sha = format!("{:x}", Sha256::digest(fs::read(&staged_archive)?));
        fs::write(
            source.join("runtime-release.json"),
            serde_json::to_vec_pretty(&release(&candidate, &staged_sha))?,
        )?;
        Ok(Self {
            _temporary: temporary,
            final_archive: root.join("final.tar.gz"),
            root,
            staged_archive,
            candidate,
            runtime,
        })
    }
}

fn candidate() -> Value {
    json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": COMMIT},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {}
    })
}

fn release(candidate: &[u8], staged_sha: &str) -> Value {
    json!({
        "schema": "codexy-runtime-release/v1",
        "state": "candidate-proven",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": COMMIT},
        "artifact": {
            "tag": "v1.3.0",
            "url": format!("https://github.com/eunsoogi/codexy/releases/download/v1.3.0/{RUNTIME_ASSET}"),
            "sha256": staged_sha,
            "payloadManifestSha256": format!("{:x}", Sha256::digest(candidate))
        },
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {}
    })
}
