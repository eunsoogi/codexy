use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use crate::support::{
    FixtureCommand, write_posix_fixture_command, write_posix_fixture_shell_runner,
};
use serde_json::Value;

use super::{command, future_build::Binaries, real_fixture_seed, receipt::receipt_value};

pub(super) struct Fixture {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    receipt: PathBuf,
    activator: PathBuf,
    gh: PathBuf,
    verifier_runner: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let prepared = real_fixture_seed::materialize()?;
        let temp = prepared.temp;
        let repo = prepared.repo;
        let receipt = temp.path().join("receipt.json");
        fs::write(&receipt, serde_json::to_vec(&receipt_value())?)?;
        let activator = temp.path().join("activate-runtime");
        write_posix_fixture_command(&activator, ACTIVATOR_BOOTSTRAP)?;
        let gh = temp.path().join("gh");
        write_posix_fixture_command(&gh, "#!/bin/sh\nprintf '%s\\n' OPEN\n")?;
        let verifier_runner = temp.path().join("verify-runtime-activation-branch");
        write_posix_fixture_shell_runner(
            &verifier_runner,
            "CODEXY_FUTURE_VERIFIER",
            &[("gh", "CODEXY_FUTURE_GH")],
        )?;
        Ok(Self {
            _temp: temp,
            repo,
            receipt,
            activator,
            gh,
            verifier_runner,
        })
    }

    pub(super) fn initial_selected(&self) -> Result<String, Box<dyn std::error::Error>> {
        let contract: Value = serde_json::from_slice(&fs::read(
            self.repo.join(".agents/plugins/release-publish-contract.json"),
        )?)?;
        contract["bootstrap"]["selectedVersion"]
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| "initial selected bootstrap version".into())
    }

    pub(super) fn canonicalize_selected(
        &self,
        sync: &Path,
        selected: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        normalize_core_platforms(&self.repo)?;
        run_binary(sync, &self.repo, &["--version", selected])
    }

    pub(super) fn prepare_candidate(
        &self,
        sync: &Path,
        version: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        run_binary(sync, &self.repo, &["--prepare-candidate", version])
    }

    pub(super) fn apply_transition(
        &self,
        binaries: &Binaries,
        version: &str,
        branch: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        git(&self.repo, &["switch", "-c", branch])?;
        run_binary(
            &binaries.activator,
            &self.repo,
            &[
                "--repo-root",
                self.repo.to_str().ok_or("repository path")?,
                "--bootstrap-version",
                version,
                "--candidate-receipt",
                self.receipt.to_str().ok_or("receipt path")?,
            ],
        )?;
        run_binary(&binaries.sync, &self.repo, &["--version", version])?;
        self.commit(&format!("activation {version}"))
    }

    pub(super) fn verify_twice(
        &self,
        binaries: &Binaries,
        base: &str,
        branch: &str,
        version: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for repeat in 1..=2 {
            let output = self.verify(binaries, base, branch, version)?;
            if !output.status.success() {
                return Err(format!(
                    "future {version} verifier repeat {repeat} failed\nstdout:\n{}\nstderr:\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr),
                )
                .into());
            }
        }
        Ok(())
    }

    fn verify(
        &self,
        binaries: &Binaries,
        base: &str,
        branch: &str,
        version: &str,
    ) -> Result<Output, Box<dyn std::error::Error>> {
        let mut command = FixtureCommand::new(&self.verifier_runner);
        command
            .args([branch, base, version])
            .arg(&self.receipt)
            .current_dir(&self.repo)
            .env("CODEXY_TEST_MODE", "1")
            .envs([
                ("GIT_CONFIG_COUNT", "2"),
                ("GIT_CONFIG_KEY_0", "maintenance.auto"),
                ("GIT_CONFIG_VALUE_0", "false"),
                ("GIT_CONFIG_KEY_1", "gc.auto"),
                ("GIT_CONFIG_VALUE_1", "0"),
            ])
            .env_path("CODEXY_TEST_ACTIVATE_RUNTIME", &self.activator)
            .env_path("CODEXY_FUTURE_ACTIVATOR_BINARY", &binaries.activator)
            .env_path("CODEXY_FUTURE_VERIFIER", self.repo.join("scripts/verify-runtime-activation-branch"))
            .env_path("CODEXY_FUTURE_GH", &self.gh)
            .env_path("CODEXY_TEST_ACTIVATE_RUNTIME_BINARY", &binaries.activator)
            .env_path("CODEXY_TEST_SYNC_VERSION_BINARY", &binaries.sync);
        Ok(command.output()?)
    }

    pub(super) fn commit(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        git(&self.repo, &["add", "-A", "--", "."])?;
        git(&self.repo, &["commit", "-m", message])
    }

    pub(super) fn branch_from(
        &self,
        source: &str,
        branch: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        git(&self.repo, &["switch", "-c", branch, source])
    }
}

const ACTIVATOR_BOOTSTRAP: &str = r#"#!/bin/sh
set -eu
root=
previous=
for argument in "$@"; do
    if [ "$previous" = --repo-root ]; then root="$argument"; break; fi
    previous="$argument"
done
test -n "$root"
if ! test -d "$root/.git"; then
    git -C "$root" init -b main >/dev/null
    git -C "$root" config user.name future-test
    git -C "$root" config user.email future-test@example.com
    git -C "$root" add -A -- .
    git -C "$root" commit -m future-base >/dev/null
fi
exec "$CODEXY_FUTURE_ACTIVATOR_BINARY" "$@"
"#;

fn run_binary(
    binary: &Path,
    repo: &Path,
    args: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(binary)
        .args(args)
        .current_dir(repo)
        .env("CODEXY_REPO_ROOT", repo)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{} failed\nstdout:\n{}\nstderr:\n{}",
            binary.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        )
        .into())
    }
}

fn normalize_core_platforms(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let marketplace: Value = serde_json::from_slice(&fs::read(
        repo.join(".agents/plugins/marketplace.json"),
    )?)?;
    let platforms = marketplace["plugins"]
        .as_array()
        .and_then(|plugins| plugins.iter().find(|plugin| plugin["name"] == "codexy"))
        .and_then(|plugin| plugin.get("supportedPlatforms"))
        .cloned()
        .ok_or("marketplace core supportedPlatforms")?;
    let path = repo.join("plugins/codexy/.codex-plugin/plugin.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&path)?)?;
    manifest["supportedPlatforms"] = platforms;
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    command::run(&mut command)
}
