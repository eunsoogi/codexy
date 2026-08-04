use std::{fs, path::{Path, PathBuf}, process::Output};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::FixtureCommand as Command;

#[test]
fn materializer_binds_staging_source_to_later_activation_with_space_safe_paths()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = LifecycleFixture::new()?;
    assert_eq!(
        fixture.root.file_name().and_then(|path| path.to_str()),
        Some("work with spaces")
    );
    assert!(fixture.admits_activation(&fixture.activation_commit)?);
    let materialized = fixture.materialize(&fixture.staging_commit, &fixture.activation_commit)?;
    assert!(materialized.status.success());
    let entries = Command::new("tar")
        .args(["-tzf"])
        .arg(&fixture.final_archive)
        .output()?;
    assert!(entries.status.success());
    let entries = String::from_utf8(entries.stdout)?;
    assert!(!entries.contains("runtime-release.json"));
    assert!(!entries.contains("runtime-candidate.json"));
    let wrapper = Command::new("tar")
        .args(["-xOzf"])
        .arg(&fixture.final_archive)
        .arg("plugins/codexy/mcp/codexy-mcp-lsp")
        .output()?;
    assert!(wrapper.status.success());
    assert!(String::from_utf8(wrapper.stdout)?.contains("getcodexy==1.3.0"));
    assert!(!fixture.materialize(&"e".repeat(40), &fixture.activation_commit)?.status.success());
    assert!(!fixture.materialize(&fixture.activation_commit, &fixture.staging_commit)?.status.success());
    assert!(!fixture.materialize(&fixture.staging_commit, &"f".repeat(40))?.status.success());
    fixture.advance_protected_main()?;
    assert!(!fixture.admits_activation(&fixture.activation_commit)?);
    Ok(())
}

struct LifecycleFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    archive: PathBuf,
    final_archive: PathBuf,
    staging_commit: String,
    activation_commit: String,
}

impl LifecycleFixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let remote = temporary.path().join("protected.git");
        assert!(Command::new("git").args(["init", "--bare"]).arg(&remote).status()?.success());
        let root = temporary.path().join("work with spaces");
        fs::create_dir(&root)?;
        git(&root, &["init"])?;
        git(&root, &["branch", "-M", "main"])?;
        git(&root, &["config", "user.email", "fixture@example.test"])?;
        git(&root, &["config", "user.name", "fixture"])?;
        let remote = remote.to_string_lossy().into_owned();
        git(&root, &["remote", "add", "origin", &remote])?;
        let plugin = root.join("plugins/codexy");
        let staged = root.join("staged/plugins/codexy");
        for path in [&plugin, &staged] {
            fs::create_dir_all(path.join(".codex-plugin"))?;
            fs::create_dir_all(path.join("runtime"))?;
        }
        fs::write(plugin.join(".codex-plugin/plugin.json"), b"{\"version\":\"1.2.2\"}\n")?;
        fs::write(plugin.join("runtime-release.json"), b"{\"state\":\"legacy-public\"}\n")?;
        let mcp = plugin.join("mcp");
        fs::create_dir_all(&mcp)?;
        for server in ["lsp", "codegraph"] {
            fs::write(
                mcp.join(format!("codexy-mcp-{server}")),
                format!("#!/bin/sh\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\nexec uvx --from getcodexy==1.2.2 codexy-mcp-runtime {server} -- \"$@\"\n"),
            )?;
        }
        git(&root, &["add", "."])?;
        git(&root, &["commit", "-m", "stage source"])?;
        let staging_commit = git(&root, &["rev-parse", "HEAD"])?;
        git(&root, &["push", "-u", "origin", "main"])?;
        let candidate = candidate(&staging_commit);
        let bytes = serde_json::to_vec(&candidate)?;
        fs::write(staged.join("runtime-candidate.json"), &bytes)?;
        let runtime = staged.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
        fs::write(&runtime, b"#!/bin/sh\nexit 0\n")?;
        let archive = root.join("staging.tar.gz");
        assert!(Command::new("tar").env("COPYFILE_DISABLE", "1").args(["-C"]).arg(root.join("staged")).args(["-czf"]).arg(&archive).arg("plugins/codexy").status()?.success());
        let digest = format!("{:x}", Sha256::digest(fs::read(&archive)?));
        fs::write(plugin.join(".codex-plugin/plugin.json"), b"{\"version\":\"1.3.0\"}\n")?;
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::write(
            root.join(".agents/plugins/runtime-activation.json"),
            serde_json::to_vec(&json!({
                "candidate": candidate,
                "artifact": {"sha256": digest, "payloadManifestSha256": format!("{:x}", Sha256::digest(&bytes))}
            }))?,
        )?;
        git(&root, &["add", "."])?;
        git(&root, &["commit", "-m", "activate source"])?;
        let activation_commit = git(&root, &["rev-parse", "HEAD"])?;
        git(&root, &["push", "origin", "main"])?;
        let final_archive = root.join("final.tar.gz");
        Ok(Self { _temporary: temporary, root, archive, final_archive, staging_commit, activation_commit })
    }

    fn materialize(&self, staging: &str, activation: &str) -> Result<Output, std::io::Error> {
        Command::new("git").args(["checkout", "--detach", &self.activation_commit]).current_dir(&self.root).output()?;
        let mut command = Command::new(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/materialize-runtime-release-archive"),
        );
        command
            .arg_path(&self.archive)
            .arg_path(&self.final_archive)
            .current_dir(&self.root)
            .env("RELEASE_TAG", "v1.3.0")
            .env("STAGING_SOURCE_COMMIT", staging)
            .env("ACTIVATION_COMMIT", activation)
            .env("STAGING_RUN_ID", "42")
            .output()
    }

    fn admits_activation(&self, activation: &str) -> Result<bool, Box<dyn std::error::Error>> {
        git(&self.root, &["fetch", "--no-tags", "origin", "+refs/heads/main:refs/remotes/origin/main"])?;
        Ok(git(&self.root, &["rev-parse", "origin/main"])? == activation)
    }

    fn advance_protected_main(&self) -> Result<(), Box<dyn std::error::Error>> {
        git(&self.root, &["checkout", "main"])?;
        fs::write(self.root.join("protected-main-race"), b"advanced\n")?;
        git(&self.root, &["add", "."])?;
        git(&self.root, &["commit", "-m", "advance protected main"])?;
        git(&self.root, &["push", "origin", "main"])?;
        Ok(())
    }
}

fn git(root: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    Ok(String::from_utf8(output.stdout)?.trim().into())
}

fn candidate(commit: &str) -> Value {
    json!({"schema":"codexy-runtime-candidate/v1","source":{"repository":"https://github.com/eunsoogi/codexy","commit":commit},"artifact":{"stagingRunId":42,"stagingRunAttempt":1},"compatibility":{"bootstrapApi":1,"pluginRuntimeApi":1,"transport":"stdio-newline-v1","mcpProtocol":"2024-11-05"},"platforms":{}})
}
