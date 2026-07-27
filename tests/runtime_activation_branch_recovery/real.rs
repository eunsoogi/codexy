use std::{
    fs,
    os::unix::fs::PermissionsExt as _,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

#[test]
fn real_base_activator_authenticates_retry_and_metadata_matrix()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    assert_result(fixture.verify("main", "1.3.0")?, true, "exact retry");
    assert_result(
        fixture.activate(&fixture.repo, "1.4.0")?,
        false,
        "wrong candidate metadata",
    );
    fixture.add_wrong_base()?;
    let wrong_base = fixture.archive_ref("wrong-base")?;
    assert_result(
        fixture.activate(&wrong_base, "1.3.0")?,
        false,
        "wrong base metadata",
    );
    fixture.add_wrapper_drift()?;
    assert_result(
        fixture.verify("main", "1.3.0")?,
        false,
        "same-path drift",
    );
    Ok(())
}

fn assert_result(output: Output, success: bool, case: &str) {
    assert_eq!(
        output.status.success(),
        success,
        "unexpected {case} result\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

struct Fixture {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    receipt: PathBuf,
    bin: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let archive = temp.path().join("repo.tar");
        fs::create_dir(&repo)?;
        command(
            Command::new("git")
                .args(["archive", "--format=tar", "HEAD"])
                .arg("-o")
                .arg(&archive)
                .current_dir(env!("CARGO_MANIFEST_DIR")),
        )?;
        command(
            Command::new("tar")
                .arg("-xf")
                .arg(&archive)
                .arg("-C")
                .arg(&repo),
        )?;
        for relative in [
            "scripts/activate-runtime-contract",
            "scripts/verify-runtime-activation-branch",
        ] {
            fs::copy(
                Path::new(env!("CARGO_MANIFEST_DIR")).join(relative),
                repo.join(relative),
            )?;
        }
        git(&repo, &["init", "-b", "main"])?;
        git(&repo, &["config", "user.name", "test"])?;
        git(&repo, &["config", "user.email", "test@example.com"])?;
        git(&repo, &["add", "."])?;
        git(&repo, &["commit", "-m", "base"])?;
        git(&repo, &["switch", "-c", "activation"])?;
        let receipt = temp.path().join("receipt.json");
        fs::write(&receipt, serde_json::to_vec(&receipt_value())?)?;
        command(
            Command::new(env!("CARGO_BIN_EXE_codexy-activate-runtime"))
                .args(["--repo-root", repo.to_str().ok_or("repo")?])
                .args(["--bootstrap-version", "1.3.0"])
                .args(["--candidate-receipt", receipt.to_str().ok_or("receipt")?]),
        )?;
        git(&repo, &["add", ".agents/plugins/release-publish-contract.json"])?;
        git(&repo, &["add", "plugins/codexy/mcp", "plugins/codexy/runtime-candidate.json"])?;
        git(&repo, &["add", "plugins/codexy/runtime-release.json", "src/version/bootstrap.rs"])?;
        git(&repo, &["commit", "-m", "activation"])?;
        let bin = temp.path().join("bin");
        fs::create_dir(&bin)?;
        executable(&bin.join("gh"), "#!/bin/sh\nprintf 'OPEN\\n'\n")?;
        Ok(Self {
            _temp: temp,
            repo,
            receipt,
            bin,
        })
    }

    fn verify(&self, base: &str, version: &str) -> Result<Output, Box<dyn std::error::Error>> {
        Ok(Command::new(self.repo.join("scripts/verify-runtime-activation-branch"))
            .args(["activation", base, version])
            .arg(&self.receipt)
            .current_dir(&self.repo)
            .env(
                "PATH",
                format!("{}:{}", self.bin.display(), std::env::var("PATH")?),
            )
            .env("CODEXY_TEST_MODE", "1")
            .env(
                "CODEXY_TEST_ACTIVATE_RUNTIME_BINARY",
                env!("CARGO_BIN_EXE_codexy-activate-runtime"),
            )
            .output()?)
    }

    fn activate(&self, root: &Path, version: &str) -> Result<Output, Box<dyn std::error::Error>> {
        Ok(Command::new(env!("CARGO_BIN_EXE_codexy-activate-runtime"))
            .args(["--repo-root", root.to_str().ok_or("root")?])
            .args(["--bootstrap-version", version])
            .arg("--candidate-receipt")
            .arg(&self.receipt)
            .output()?)
    }

    fn archive_ref(&self, reference: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let archive = self._temp.path().join(format!("{reference}.tar"));
        let root = self._temp.path().join(format!("{reference}-root"));
        fs::create_dir(&root)?;
        command(
            Command::new("git")
                .args(["archive", "--format=tar", reference])
                .arg("-o")
                .arg(&archive)
                .current_dir(&self.repo),
        )?;
        command(
            Command::new("tar")
                .arg("-xf")
                .arg(&archive)
                .arg("-C")
                .arg(&root),
        )?;
        Ok(root)
    }

    fn add_wrong_base(&self) -> Result<(), Box<dyn std::error::Error>> {
        git(&self.repo, &["switch", "-c", "wrong-base", "main"])?;
        let path = self.repo.join("src/version/bootstrap.rs");
        let source = fs::read_to_string(&path)?;
        fs::write(&path, source.replace("VERSION: &str = \"1.2.2\"", "VERSION: &str = \"1.1.0\""))?;
        git(&self.repo, &["add", "src/version/bootstrap.rs"])?;
        git(&self.repo, &["commit", "-m", "wrong base"])?;
        git(&self.repo, &["switch", "activation"])?;
        Ok(())
    }

    fn add_wrapper_drift(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = self.repo.join("plugins/codexy/mcp/codexy-mcp-lsp");
        let mut source = fs::read_to_string(&path)?;
        source.push_str("# drift\n");
        fs::write(&path, source)?;
        git(&self.repo, &["add", "plugins/codexy/mcp/codexy-mcp-lsp"])?;
        git(&self.repo, &["commit", "-m", "drift"])?;
        Ok(())
    }
}

fn receipt_value() -> Value {
    let candidate = json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": "a".repeat(40)},
        "artifact": {"tag": "runtime-candidate-1.3.0"},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {
            "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": "b".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
            "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}}
        }
    });
    let candidate_bytes = serde_json::to_vec(&canonical(candidate.clone())).unwrap();
    json!({
        "schema": "codexy-runtime-candidate-receipt/v1", "candidate": candidate,
        "artifact": {"url": "https://github.com/eunsoogi/codexy/releases/download/runtime-candidate-1.3.0/codexy-marketplace-plugin.tar.gz", "sha256": "f".repeat(64), "payloadManifestSha256": format!("{:x}", Sha256::digest(candidate_bytes))},
        "provenance": {"repositoryId": 1269350143, "workflowPath": ".github/workflows/runtime-candidate.yml", "runId": 42, "runAttempt": 1, "workflowRunUrl": "https://github.com/eunsoogi/codexy/actions/runs/42"}
    })
}

fn canonical(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonical(value)))
                    .collect(),
            )
        }
        Value::Array(values) => Value::Array(values.into_iter().map(canonical).collect()),
        other => other,
    }
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    command(Command::new("git").args(args).current_dir(root))
}

fn command(command: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let output = command.output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned().into())
    }
}

fn executable(path: &Path, source: &str) -> std::io::Result<()> {
    fs::write(path, source)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}
