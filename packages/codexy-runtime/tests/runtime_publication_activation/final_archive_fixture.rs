use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Output,
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::{self, FixtureCommand as Command};

const STAGING_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ACTIVATION_COMMIT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const RUNTIME: &[u8] = b"#!/bin/sh\nprintf 'final archive runtime\\n'\n";
pub(super) struct FinalArchiveFixture {
    _temporary: tempfile::TempDir,
    pub(super) root: PathBuf,
    pub(super) staged_archive: PathBuf,
    pub(super) public_archive: PathBuf,
    pub(super) final_archive: PathBuf,
}

impl FinalArchiveFixture {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let root = temporary.path().join("final archive fixture with spaces");
        fs::create_dir(&root)?;
        let source = root.join("plugins/codexy-devtools");
        let staged = root.join("staged/plugins/codexy-devtools");
        for plugin in [&source, &staged] {
            fs::create_dir_all(plugin.join(".codex-plugin"))?;
            fs::create_dir_all(plugin.join("runtime"))?;
        }
        for (plugin, version) in [(&source, "1.3.0"), (&staged, "1.2.2")] {
            fs::write(
                plugin.join(".codex-plugin/plugin.json"),
                format!("{{\"name\":\"codexy-devtools\",\"version\":\"{version}\"}}\n"),
            )?;
        }
        fs::create_dir_all(source.join("hooks"))?;
        fs::create_dir_all(staged.join("hooks"))?;
        fs::write(source.join("hooks/current-policy.txt"), b"current policy\n")?;
        fs::write(staged.join("hooks/current-policy.txt"), b"stale policy\n")?;
        let mcp = source.join("mcp");
        fs::create_dir_all(&mcp)?;
        fs::write(
            mcp.join("codexy-mcp-devtools"),
            "#!/bin/sh\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\nexec uvx --from getcodexy==1.2.2 codexy-mcp-runtime \"$server\" -- \"$@\"\n",
        )?;
        for server in ["lsp", "codegraph"] {
            fs::write(
                mcp.join(format!("codexy-mcp-{server}")),
                format!(
                    "#!/bin/sh\nbundled_platforms=\"darwin-arm64 linux-x86_64\"\nexec uvx --from getcodexy==1.2.2 codexy-mcp-runtime {server} -- \"$@\"\n"
                ),
            )?;
            fs::write(
                mcp.join(format!("codexy-mcp-{server}.cmd")),
                format!(
                    "@echo off\n\"%~dp0codexy-mcp-devtools.exe\" {server} %*\nexit /b %ERRORLEVEL%\n"
                ),
            )?;
        }
        let runtime_path = staged.join("runtime/codexy-mcp-lsp-darwin-arm64.bin");
        fs::write(&runtime_path, RUNTIME)?;
        support::make_executable(&runtime_path)?;
        for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
            for server in ["lsp", "codegraph"] {
                let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
                let path = staged.join(format!("runtime/codexy-mcp-{server}-{platform}.{extension}"));
                if !path.exists() {
                    fs::write(path, format!("{server}-{platform}\n"))?;
                }
            }
        }
        fs::create_dir_all(staged.join("mcp"))?;
        fs::write(staged.join("mcp/codexy-mcp-devtools.exe"), b"dispatcher-windows\n")?;
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
                .arg("plugins/codexy-devtools")
                .status()?
                .success()
        );
        let staged_sha = format!("{:x}", Sha256::digest(fs::read(&staged_archive)?));
        let public_root = root.join("public");
        fs::create_dir_all(&public_root)?;
        assert!(
            Command::new("tar")
                .args(["-xzf"])
                .arg(&staged_archive)
                .arg("-C")
                .arg(&public_root)
                .status()?
                .success()
        );
        fs::remove_file(public_root.join("plugins/codexy-devtools/runtime-candidate.json"))?;
        let public_archive = root.join("public.tar.gz");
        assert!(
            Command::new("tar")
                .env("COPYFILE_DISABLE", "1")
                .args(["-C"])
                .arg(&public_root)
                .args(["-czf"])
                .arg(&public_archive)
                .arg("plugins/codexy-devtools")
                .status()?
                .success()
        );
        super::public_receipt::write(&root, &public_archive)?;
        fs::create_dir_all(root.join(".agents/plugins"))?;
        fs::write(
            root.join(".agents/plugins/runtime-activation.json"),
            serde_json::to_vec(&json!({
                "candidate": candidate,
                "artifact": {
                    "sha256": staged_sha,
                    "payloadManifestSha256": format!("{:x}", Sha256::digest(&candidate_bytes))
                }
            }))?,
        )?;
        Ok(Self {
            _temporary: temporary,
            final_archive: root.join("final.tar.gz"),
            root,
            staged_archive,
            public_archive,
        })
    }

    pub(super) fn materialize(
        &self,
        prepend_path: Option<PathBuf>,
    ) -> Result<Output, std::io::Error> {
        self.materialize_with(
            &self.staged_archive,
            STAGING_COMMIT,
            "42",
            prepend_path,
            false,
            "v1.3.0",
        )
    }

    pub(super) fn materialize_public(&self) -> Result<Output, std::io::Error> {
        self.materialize_with(&self.public_archive, STAGING_COMMIT, "42", None, true, "v1.3.0")
    }

    pub(super) fn materialize_public_for_tag(
        &self,
        release_tag: &str,
    ) -> Result<Output, std::io::Error> {
        super::public_receipt::set_tag_for_root(&self.root, release_tag)?;
        self.materialize_with(&self.public_archive, STAGING_COMMIT, "42", None, true, release_tag)
    }

    pub(super) fn materialize_public_with(
        &self,
        archive: &Path,
        source_commit: &str,
        run_id: &str,
        prepend_path: Option<PathBuf>,
    ) -> Result<Output, std::io::Error> {
        self.materialize_with(archive, source_commit, run_id, prepend_path, true, "v1.3.0")
    }

    fn materialize_with(
        &self,
        archive: &Path,
        source_commit: &str,
        run_id: &str,
        prepend_path: Option<PathBuf>,
        public: bool,
        release_tag: &str,
    ) -> Result<Output, std::io::Error> {
        let mut command = Command::new(
            codexy_runtime::paths::repository_root()
                .join("scripts/materialize-runtime-release-archive"),
        );
        if let Some(path) = prepend_path {
            let host_path = env::var_os("PATH").ok_or_else(|| std::io::Error::other("PATH"))?;
            let mut entries = vec![path];
            entries.extend(env::split_paths(&host_path));
            command.env_path_list("PATH", entries);
        }
        match archive == self.staged_archive {
            true => command.arg_path(&self.staged_archive),
            false => command.arg_path(archive),
        };
        command
            .arg_path(&self.final_archive)
            .current_dir(&self.root)
            .env("RELEASE_TAG", release_tag)
            .env("STAGING_SOURCE_COMMIT", source_commit)
            .env("ACTIVATION_COMMIT", ACTIVATION_COMMIT)
            .env("STAGING_RUN_ID", run_id);
        if public {
            command.env("PUBLIC_RELEASE", "1");
            command.env_path(
                "PUBLIC_RELEASE_RECEIPT",
                self.root.join("public-release/runtime-release-receipt.json"),
            );
        }
        command.output()
    }

    pub(super) fn input_tree(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let output = Command::new("tar")
            .current_dir(&self.root)
            .args(["-cf", "-", "plugins/codexy-devtools", "staged/plugins/codexy-devtools"])
            .output()?;
        assert!(output.status.success(), "fixture input snapshot failed");
        Ok(output.stdout)
    }

}

fn candidate(staged: &Path) -> Result<Value, std::io::Error> {
    let mut platforms = serde_json::Map::new();
    for platform in ["darwin-arm64", "linux-x86_64", "windows-x86_64"] {
        let extension = if platform == "windows-x86_64" { "exe" } else { "bin" };
        let mut inventory = serde_json::Map::new();
        for server in ["lsp", "codegraph"] {
            let path = format!("runtime/codexy-mcp-{server}-{platform}.{extension}");
            inventory.insert(
                server.to_owned(),
                json!({
                    "path": path,
                    "sha256": format!("{:x}", Sha256::digest(fs::read(staged.join(&path))?))
                }),
            );
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
