use std::{
    env,
    fs,
    path::{Path, PathBuf},
    process::Output,
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::{self, FixtureCommand as Command};

use super::workflow;

const STAGING_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ACTIVATION_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const RUNTIME_ASSET: &str = "codexy-runtime-package.tar.gz";

#[test]
fn final_publisher_materializes_and_exercises_the_public_archive()
-> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    let run = publisher.1;
    let inputs = publisher.2["on"]["workflow_dispatch"]["inputs"]
        .as_mapping()
        .ok_or("final publisher dispatch inputs")?;
    for input in ["staging_source_commit", "activation_commit", "staging_run_id"] {
        assert!(inputs.contains_key(input), "final publisher lacks {input}");
    }
    support::assert_structured_literals(
        &run,
        "final publisher lineage and archive contract",
        &[
            "STAGING_SOURCE_COMMIT",
            "ACTIVATION_COMMIT",
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main",
            "test \"$ACTIVATION_COMMIT\" = \"$(git rev-parse origin/main)\"",
            "scripts/materialize-runtime-release-archive",
            "codexy-runtime-package.tar.gz",
            "runtime-release-receipt.json",
            "scripts/inspect-release-archive public.tar.gz public-inspect/plugins/codexy",
            "gh attestation verify public-runtime.tar.gz",
        ],
    );
    Ok(())
}

#[test]
fn materializer_preserves_staged_runtime_with_space_safe_paths_without_rsync()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = FinalArchiveFixture::new()?;
    let missing_rsync = fixture.root.join("missing rsync command");
    fs::create_dir(&missing_rsync)?;
    let rsync = missing_rsync.join("rsync");
    fs::write(&rsync, "#!/bin/sh\nexit 127\n")?;
    support::make_executable(&rsync)?;
    let output = fixture.materialize(Some(missing_rsync))?;
    assert!(
        output.status.success(),
        "materializer must preserve the archive without rsync: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fixture.root.file_name().and_then(|path| path.to_str()),
        Some("final archive fixture with spaces")
    );
    let extraction = tempfile::tempdir()?;
    let extracted = extraction.path().join("extracted");
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
        let root = temporary.path().join("final archive fixture with spaces");
        fs::create_dir(&root)?;
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
        support::make_executable(&runtime_path)?;
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

    fn materialize(&self, prepend_path: Option<PathBuf>) -> Result<Output, std::io::Error> {
        let mut command = Command::new(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts/materialize-runtime-release-archive"),
        );
        if let Some(path) = prepend_path {
            let host_path = env::var_os("PATH").ok_or_else(|| std::io::Error::other("PATH"))?;
            let mut entries = vec![path];
            entries.extend(env::split_paths(&host_path));
            command.env_path_list("PATH", entries);
        }
        command
            .arg_path(&self.staged_archive)
            .arg_path(&self.final_archive)
            .current_dir(&self.root)
            .env("RELEASE_TAG", "v1.3.0")
            .env("STAGING_SOURCE_COMMIT", STAGING_COMMIT)
            .env("ACTIVATION_COMMIT", ACTIVATION_COMMIT)
            .output()
    }
}

fn candidate() -> Value {
    json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": STAGING_COMMIT},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {}
    })
}

fn release(candidate: &[u8], staged_sha: &str) -> Value {
    json!({
        "schema": "codexy-runtime-release/v1",
        "state": "candidate-proven",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": STAGING_COMMIT},
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
