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
            "gh release view v1.3.0",
            "gh release upload v1.3.0",
            "--draft",
            "gh release edit v1.3.0 --draft=false",
            "gh release download v1.3.0",
            "release asset differs from verified bytes",
            "--plugin-root \"$PWD/plugins/codexy\"",
        ],
    );
    assert!(
        !run.contains("--clobber"),
        "immutable release assets must be verified, never overwritten"
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
    assert_eq!(manifest["version"], "1.3.0");
    assert_eq!(
        manifest["supportedPlatforms"],
        json!(["darwin-arm64", "linux-x86_64", "windows-x86_64"])
    );
    assert!(!plugin.join("runtime-release.json").exists());
    assert!(!plugin.join("runtime-candidate.json").exists());
    for server in ["lsp", "codegraph"] {
        let wrapper = fs::read_to_string(plugin.join(format!("mcp/codexy-mcp-{server}")))?;
        assert!(
            wrapper.contains("getcodexy==1.3.0"),
            "final archive must project the release version into {server} wrapper"
        );
        assert!(
            !wrapper.contains("getcodexy==1.2.2"),
            "final archive must not retain the pre-release {server} wrapper pin"
        );
    }
    let runtime = plugin.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
    assert_eq!(fs::read(&runtime)?, fixture.runtime);
    let smoke = Command::new(runtime).arg("--help").output()?;
    assert!(smoke.status.success());
    assert_eq!(smoke.stdout, b"final archive runtime\n");
    for server in ["lsp", "codegraph"] {
        let runtime = plugin.join(format!("runtime/codexy-mcp-{server}-windows-x86_64.exe"));
        assert_eq!(fs::read(plugin.join(format!("mcp/codexy-mcp-{server}.exe")))?, fs::read(runtime)?);
    }
    Ok(())
}

struct FinalArchiveFixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    staged_archive: PathBuf,
    final_archive: PathBuf,
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
        fs::write(
            source.join(".codex-plugin/plugin.json"),
            b"{\"name\":\"codexy\",\"version\":\"1.3.0\"}\n",
        )?;
        fs::write(
            staged.join(".codex-plugin/plugin.json"),
            b"{\"name\":\"codexy\",\"version\":\"1.2.2\"}\n",
        )?;
        for server in ["lsp", "codegraph"] {
            let mcp = source.join("mcp");
            fs::create_dir_all(&mcp)?;
            fs::write(
                mcp.join(format!("codexy-mcp-{server}")),
                format!("#!/bin/sh\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\nexec uvx --from getcodexy==1.2.2 codexy-mcp-runtime {server} -- \"$@\"\n"),
            )?;
        }
        let runtime = b"#!/bin/sh\nprintf 'final archive runtime\\n'\n".to_vec();
        let runtime_path = staged.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
        fs::write(&runtime_path, &runtime)?;
        support::make_executable(&runtime_path)?;
        for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
            for server in ["lsp", "codegraph"] {
                let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
                let path = staged.join(format!("runtime/codexy-mcp-{server}-{platform}.{extension}"));
                if !path.exists() { fs::write(path, format!("{server}-{platform}\n"))?; }
            }
        }
        fs::create_dir_all(staged.join("mcp"))?;
        for server in ["lsp", "codegraph"] {
            fs::copy(
                staged.join(format!("runtime/codexy-mcp-{server}-windows-x86_64.exe")),
                staged.join(format!("mcp/codexy-mcp-{server}.exe")),
            )?;
        }
        let candidate = candidate(&staged)?;
        let candidate_bytes = serde_json::to_vec(&candidate)?;
        fs::write(staged.join("runtime-candidate.json"), &candidate_bytes)?;
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
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::write(
            root.join(".agents/plugins/runtime-activation.json"),
            serde_json::to_vec(&json!({
                "candidate": candidate,
                "artifact": {"sha256": staged_sha, "payloadManifestSha256": format!("{:x}", Sha256::digest(&candidate_bytes))}
            }))?,
        )?;
        Ok(Self {
            _temporary: temporary,
            final_archive: root.join("final.tar.gz"),
            root,
            staged_archive,
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
            .env("STAGING_RUN_ID", "42")
            .output()
    }
}

fn candidate(staged: &Path) -> Result<Value, std::io::Error> {
    let mut platforms = serde_json::Map::new();
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
        let mut inventory = serde_json::Map::new();
        for server in ["lsp", "codegraph"] {
            let path = format!("runtime/codexy-mcp-{server}-{platform}.{extension}");
            inventory.insert(server.to_owned(), json!({"path": path, "sha256": format!("{:x}", Sha256::digest(fs::read(staged.join(&path))?))}));
        }
        platforms.insert(platform.to_owned(), Value::Object(inventory));
    }
    Ok(json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": STAGING_COMMIT},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": platforms
    }))
}
