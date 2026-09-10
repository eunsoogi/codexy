use super::receipt::receipt;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[path = "activation_retry_fixture_scripts.rs"]
mod scripts;
#[path = "activation_retry_setup.rs"]
mod setup;

pub(crate) struct Fixture {
    root: tempfile::TempDir,
    pub(crate) repo: PathBuf,
    pub(crate) branch: String,
    pub(crate) legacy_branch: String,
    pub(super) open_branch: String,
    version: String,
    pub(crate) main: String,
    receipt: PathBuf,
    bin: PathBuf,
    mutation: String,
}

impl Fixture {
    pub(crate) fn new(mutation: &str) -> Result<Self, Box<dyn std::error::Error>> {
        setup::prepare(mutation)
    }

    pub(crate) fn remote_head(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.remote_branch_head(&self.branch)
    }

    pub(crate) fn remote_branch_head(
        &self,
        branch: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Ok(git(
            &self.repo,
            &["ls-remote", "origin", &format!("refs/heads/{branch}")],
        )?
        .split_whitespace()
        .next()
        .ok_or("remote head")?
        .to_owned())
    }

    pub(crate) fn select_branch(
        &self,
        version: &str,
        receipt: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        success(
            Command::new(
                self.repo
                    .join("scripts/select-runtime-activation-branch.sh"),
            )
            .args([version, receipt.to_str().ok_or("receipt path")?])
            .env(
                "PATH",
                format!("{}:{}", self.bin.display(), std::env::var("PATH")?),
            )
            .env("GH_TOKEN", "fixture-github-token")
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("CODEXY_FIXTURE_STEP", "direct-selector")
            .env("PR_STATE_FILE", self.root.path().join("pr-state"))
            .env("OPEN_ACTIVATION_BRANCH", &self.open_branch)
            .env("PR_READBACK_MODE", &self.mutation)
            .output()?,
        )
    }

    pub(crate) fn pr_states(&self, branch: &str) -> Result<String, Box<dyn std::error::Error>> {
        success(
            Command::new(self.bin.join("gh"))
                .args([
                    "pr",
                    "list",
                    "--head",
                    branch,
                    "--state",
                    "all",
                    "--json",
                    "state",
                    "--jq",
                    ".[].state",
                ])
                .env("GH_TOKEN", "fixture-github-token")
                .env("CODEXY_FIXTURE_STEP", "fixture-history")
                .env("PR_READBACK_MODE", &self.mutation)
                .env("PR_STATE_FILE", self.root.path().join("pr-state"))
                .env("LEGACY_BRANCH", &self.legacy_branch)
                .env(
                    "LEGACY_PR_STATE",
                    if self.mutation == "merged-deleted" || self.mutation == "retained" {
                        "MERGED"
                    } else {
                        ""
                    },
                )
                .output()?,
        )
    }

    pub(crate) fn run(&self, attempt: &str) -> Result<Output, Box<dyn std::error::Error>> {
        self.run_with_omitted_token(attempt, None)
    }

    pub(crate) fn run_with_omitted_token(
        &self,
        attempt: &str,
        omitted_step: Option<&str>,
    ) -> Result<Output, Box<dyn std::error::Error>> {
        git(&self.repo, &["checkout", "main"])?;
        if self.mutation == "push-failure" {
            let hook = self.root.path().join("remote.git/hooks/pre-receive");
            fs::write(&hook, "#!/bin/sh\nexit 1\n")?;
            crate::support::make_executable(&hook)?;
        }
        let temporary = self.root.path().join(attempt);
        fs::create_dir(&temporary)?;
        fs::create_dir(temporary.join("codexy-runtime-staging"))?;
        fs::copy(
            &self.receipt,
            temporary.join("codexy-runtime-staging/runtime-staging-receipt.json"),
        )?;
        let workflow = super::super::workflow("runtime-activation.yml")?;
        let mut body = String::from("set -euo pipefail\n");
        for step in [
            "Prepare one version-selection branch",
            "Apply verified activation and version-selection contract",
            "Stage and verify activation branch",
            "Create exactly one activation pull request",
        ] {
            let definition = workflow["jobs"]["open-activation-pr"]["steps"]
                .as_sequence()
                .ok_or("workflow steps")?
                .iter()
                .find(|definition| definition["name"] == step)
                .ok_or("workflow step")?;
            body.push_str("(\nunset GH_TOKEN GITHUB_TOKEN\n");
            body.push_str(&format!("export CODEXY_FIXTURE_STEP='{step}'\n"));
            if definition["env"]["GH_TOKEN"] == "${{ github.token }}" && omitted_step != Some(step)
            {
                body.push_str("export GH_TOKEN=fixture-github-token\n");
            }
            body.push_str(super::super::run(&workflow, "open-activation-pr", step)?);
            body.push('\n');
            if self.mutation == "late-index" && step == "Prepare one version-selection branch" {
                body.push_str(
                    "printf tampered > retry-main-marker.txt\ngit add retry-main-marker.txt\n",
                );
            }
            body.push_str(")\n");
        }
        Ok(Command::new("bash")
            .args(["-c", &body])
            .current_dir(&self.repo)
            .env(
                "PATH",
                format!("{}:{}", self.bin.display(), std::env::var("PATH")?),
            )
            .env("RUNNER_TEMP", &temporary)
            .env("GITHUB_SHA", &self.main)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("ACTIVATION_BRANCH", &self.branch)
            .env("OPEN_ACTIVATION_BRANCH", &self.open_branch)
            .env("LEGACY_BRANCH", &self.legacy_branch)
            .env(
                "LEGACY_PR_STATE",
                if self.mutation == "merged-deleted" || self.mutation == "retained" {
                    "MERGED"
                } else {
                    ""
                },
            )
            .env("PR_READBACK_MODE", &self.mutation)
            .env("PR_READBACK_ATTEMPT", temporary.join("pr-readback-attempt"))
            .env("PR_STATE_FILE", self.root.path().join("pr-state"))
            .env("GITHUB_WORKSPACE", &self.repo)
            .env("BOOTSTRAP_VERSION", &self.version)
            .env("CODEXY_TEST_MODE", "1")
            .env(
                "CODEXY_TEST_ACTIVATE_RUNTIME_BINARY",
                env!("CARGO_BIN_EXE_codexy-activate-runtime"),
            )
            .env(
                "CODEXY_TEST_SYNC_VERSION_BINARY",
                env!("CARGO_BIN_EXE_codexy-sync-version"),
            )
            .output()?)
    }

    pub(crate) fn prepare_next_attempt(&self) -> Result<(), Box<dyn std::error::Error>> {
        git(&self.repo, &["checkout", "main"])?;
        git(&self.repo, &["branch", "-D", &self.branch])?;
        Ok(())
    }
}

pub(crate) fn git(repo: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(repo).output()?;
    success(output)
}

fn commit(repo: &Path, message: &str) -> Result<(), Box<dyn std::error::Error>> {
    git(repo, &["add", "-A"])?;
    git(repo, &["commit", "-m", message])?;
    Ok(())
}

pub(crate) fn success(output: Output) -> Result<String, Box<dyn std::error::Error>> {
    if !output.status.success() {
        return Err(format!(
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}
