use super::receipt::receipt;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

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
            &[
                "ls-remote",
                "origin",
                &format!("refs/heads/{branch}"),
            ],
        )?
        .split_whitespace()
        .next()
        .ok_or("remote head")?
        .to_owned())
    }

    pub(crate) fn select_branch(
        &self,
        receipt: &Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        success(
            Command::new(self.repo.join("scripts/select-runtime-activation-branch.sh"))
                .args([&self.version, receipt.to_str().ok_or("receipt path")?])
                .env(
                    "PATH",
                    format!("{}:{}", self.bin.display(), std::env::var("PATH")?),
                )
                .env("GH_TOKEN", "fixture-github-token")
                .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
                .env("CODEXY_FIXTURE_STEP", "direct-selector")
                .env("PR_STATE_FILE", self.root.path().join("pr-state"))
                .env("OPEN_ACTIVATION_BRANCH", &self.open_branch)
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

const GH: &str = r#"#!/bin/sh
set -eu
test "${GH_TOKEN:-}" = fixture-github-token || { echo "verifier GitHub query requires GH_TOKEN: $CODEXY_FIXTURE_STEP" >&2; exit 4; }
case "$*" in
  'pr list '*'--json number,headRefName'*)
    if test "$(cat "$PR_STATE_FILE")" = 1; then
      printf '[{"number":1,"headRefName":"%s"}]\n' "$OPEN_ACTIVATION_BRANCH"
    else
      printf '[]\n'
    fi
    ;;
  'pr list '*'--json state '*) if test "$(cat "$PR_STATE_FILE")" = 1; then printf '%s\n' OPEN; fi ;;
  'pr list '*'--json number '*) cat "$PR_STATE_FILE" ;;
  'pr list '*'--json headRefOid '*)
    attempt=0
    test ! -f "$PR_READBACK_ATTEMPT" || attempt=$(cat "$PR_READBACK_ATTEMPT")
    attempt=$((attempt + 1)); printf '%s' "$attempt" > "$PR_READBACK_ATTEMPT"
    head=$(git rev-parse HEAD); count=$(cat "$PR_STATE_FILE")
    case "$PR_READBACK_MODE" in
      readback-delay) if test "$attempt" -lt 3; then head=$(git rev-parse HEAD^); fi ;;
      readback-wrong) head=$(git rev-parse HEAD^) ;;
      readback-missing) head=missing; count=0 ;;
      readback-duplicate) count=2 ;;
    esac
    case "$*" in *'@tsv'*) printf '%s\t%s\n' "$count" "$head" ;; *) printf '%s\n' "$head" ;; esac ;;
  'pr create '*) test "$(cat "$PR_STATE_FILE")" = 0; printf 1 > "$PR_STATE_FILE" ;;
  *) echo "unexpected GitHub mutation: $*" >&2; exit 98 ;;
esac
"#;

const CARGO: &str = r#"#!/bin/sh
set -eu
case "$*" in *'--bin codexy-sync-version -- '*) ;; *) exit 99 ;; esac
while test "$1" != --; do shift; done
shift
exec "$CODEXY_TEST_SYNC_VERSION_BINARY" "$@"
"#;
