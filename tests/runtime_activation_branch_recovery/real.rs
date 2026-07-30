use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use crate::support::{
    FixtureCommand, write_posix_fixture_command,
};

mod metadata;
mod shell_runner;

#[test]
fn real_base_activator_authenticates_retry_and_metadata_matrix()
-> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new()?;
    assert_result(fixture.verify("main", "1.3.0")?, true, "exact retry");
    assert_eq!(
        fixture.cargo_invocations()?,
        0,
        "the verifier must use the injected prebuilt sync binary instead of cargo",
    );
    assert_eq!(
        fixture.external_activation_process_invocations(),
        1,
        "real matrix must retain only the successful verifier activation process",
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
    command_trace: PathBuf,
    runner: PathBuf,
    external_activation_process_invocations: Cell<usize>,
}

impl Fixture {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo with spaces");
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
        metadata::restore_legacy_platform_metadata(&repo)?;
        git(&repo, &["init", "-b", "main"])?;
        git(&repo, &["config", "user.name", "test"])?;
        git(&repo, &["config", "user.email", "test@example.com"])?;
        git(&repo, &["add", "."])?;
        git(&repo, &["commit", "-m", "base"])?;
        git(&repo, &["switch", "-c", "activation"])?;
        let receipt = temp.path().join("receipt.json");
        fs::write(&receipt, serde_json::to_vec(&receipt_value())?)?;
        let external_activation_process_invocations = Cell::new(0);
        codexy_runtime::version::activation::activate(&repo, "1.3.0", &receipt)?;
        let mut sync = FixtureCommand::new(repo.join("scripts/sync-plugin-version"));
        sync.args(["--version", "1.3.0"]).current_dir(&repo);
        command(&mut sync)?;
        git(&repo, &["add", ".agents/plugins/marketplace.json", ".agents/plugins/release-publish-contract.json"])?;
        git(&repo, &["add", "plugins/codexy/.codex-plugin/plugin.json"])?;
        git(&repo, &["add", "plugins/codexy/mcp", "plugins/codexy/runtime-candidate.json"])?;
        git(&repo, &["add", "plugins/codexy/runtime-release.json", "src/version/bootstrap.rs"])?;
        git(&repo, &["add", "Cargo.toml", "Cargo.lock"])?;
        git(&repo, &["commit", "-m", "activation"])?;
        let bin = temp.path().join("bin");
        fs::create_dir(&bin)?;
        write_posix_fixture_command(&bin.join("gh"), "#!/bin/sh\nprintf 'OPEN\\n'\n")?;
        write_posix_fixture_command(&bin.join("cargo"), "#!/bin/sh\nexit 97\n")?;
        let command_trace = temp.path().join("command-trace");
        let runner = shell_runner::write_activation_verifier_runner(temp.path())?;
        Ok(Self {
            _temp: temp,
            repo,
            receipt,
            bin,
            command_trace,
            runner,
            external_activation_process_invocations,
        })
    }

    fn verify(&self, base: &str, version: &str) -> Result<Output, Box<dyn std::error::Error>> {
        let mut path = vec![self.bin.clone()];
        path.extend(std::env::split_paths(&std::env::var_os("PATH").ok_or("PATH")?));
        let mut command = FixtureCommand::new(&self.runner);
        command
            .args(["activation", base, version])
            .arg(&self.receipt)
            .current_dir(&self.repo);
        command
            .env("CODEXY_TEST_MODE", "1")
            .env_path(
                "CODEXY_FIXTURE_VERIFY_RUNTIME_ACTIVATION_BRANCH",
                self.repo.join("scripts/verify-runtime-activation-branch"),
            )
            .env_path("CODEXY_FIXTURE_GH", self.bin.join("gh"))
            .env_path("CODEXY_FIXTURE_COMMAND_TRACE", &self.command_trace)
            .env_path(
                "CODEXY_TEST_ACTIVATE_RUNTIME_BINARY",
                env!("CARGO_BIN_EXE_codexy-activate-runtime"),
            )
            .env_path(
                "CODEXY_TEST_SYNC_VERSION_BINARY",
                env!("CARGO_BIN_EXE_codexy-sync-version"),
            )
            .env_path_list("PATH", path);
        let output = command.output()?;
        self.record_external_activation_process_invocation();
        Ok(output)
    }

    fn external_activation_process_invocations(&self) -> usize {
        self.external_activation_process_invocations.get()
    }

    fn cargo_invocations(&self) -> Result<usize, Box<dyn std::error::Error>> {
        match fs::read_to_string(&self.command_trace) {
            Ok(trace) => Ok(trace.lines().filter(|command| *command == "cargo").count()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(error.into()),
        }
    }

    fn record_external_activation_process_invocation(&self) {
        self.external_activation_process_invocations
            .set(self.external_activation_process_invocations.get() + 1);
    }

}

fn receipt_value() -> Value {
    let candidate = json!({
        "schema": "codexy-runtime-candidate/v1",
        "source": {"repository": "https://github.com/eunsoogi/codexy", "commit": "a".repeat(40)},
        "artifact": {"stagingRunId": 42, "stagingRunAttempt": 1},
        "compatibility": {"bootstrapApi": 1, "pluginRuntimeApi": 1, "transport": "stdio-newline-v1", "mcpProtocol": "2024-11-05"},
        "platforms": {
            "darwin-arm64": {"lsp": {"path": "runtime/codexy-mcp-lsp-darwin-arm64.bin", "sha256": "b".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-darwin-arm64.bin", "sha256": "c".repeat(64)}},
            "linux-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-linux-x86_64.bin", "sha256": "d".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-linux-x86_64.bin", "sha256": "e".repeat(64)}},
            "windows-x86_64": {"lsp": {"path": "runtime/codexy-mcp-lsp-windows-x86_64.exe", "sha256": "9".repeat(64)}, "codegraph": {"path": "runtime/codexy-mcp-codegraph-windows-x86_64.exe", "sha256": "a".repeat(64)}}
        }
    });
    let candidate_bytes = serde_json::to_vec(&canonical(candidate.clone())).unwrap();
    json!({
        "schema": "codexy-runtime-candidate-receipt/v1", "candidate": candidate,
        "artifact": {"sha256": "f".repeat(64), "payloadManifestSha256": format!("{:x}", Sha256::digest(candidate_bytes))},
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
