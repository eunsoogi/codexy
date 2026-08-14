use std::{
    fs,
    path::{Path, PathBuf},
    process::Output,
};

use crate::support::{ReleaseFixtureCommand, ReleaseFixtureOutcome, fixture_script_interpreter_path};

#[path = "fixture_materialization.rs"]
mod fixture_materialization;
#[path = "fixture_native_gh.rs"]
mod fixture_native_gh;
use fixture_materialization::{bind_scripts, copy_scripts};
use fixture_native_gh::gh_fixture;

pub(crate) const ASSETS: [&str; 3] = [
    "codexy-marketplace-plugin.tar.gz",
    "codexy-runtime-package.tar.gz",
    "runtime-release-receipt.json",
];
const COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub(crate) fn make_executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

pub(crate) fn git_fixture() -> &'static str { r#"#!/bin/sh
case "$1" in fetch|merge-base) exit 0 ;; rev-parse) printf '%s\n' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ;; ls-remote) printf '%s\trefs/tags/v9.9.9\n' aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa ;; *) exit 1 ;; esac
"# }

pub(crate) struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    git_launcher: PathBuf,
    gh_launcher: PathBuf,
}

impl Fixture {
    pub(crate) fn new(
        existing: &[&str],
        published: bool,
        mismatch: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("release recovery fixture");
        for name in ["bin", "scripts", "dist", "remote"] {
            fs::create_dir_all(root.join(name))?;
        }
        for asset in ASSETS {
            let bytes = if asset == "runtime-release-receipt.json" {
                format!("{{\"source\":{{\"stagingSourceCommit\":\"{COMMIT}\",\"activationCommit\":\"{COMMIT}\"}},\"staging\":{{\"runId\":\"42\"}}}}\n")
            } else {
                format!("verified {asset}\n")
            };
            fs::write(root.join("dist").join(asset), bytes)?;
        }
        for asset in existing {
            let bytes = if mismatch {
                b"wrong\n".to_vec()
            } else {
                fs::read(root.join("dist").join(asset))?
            };
            fs::write(root.join("remote").join(asset), bytes)?;
        }
        if !existing.is_empty() {
            fs::write(root.join("exists"), "yes")?;
        }
        fs::write(root.join("draft"), if published { "false" } else { "true" })?;
        copy_scripts(&root)?;
        let git = root.join("bin/git");
        let gh = root.join("bin/gh");
        fs::write(&git, git_fixture())?;
        fs::write(&gh, gh_fixture())?;
        for path in fs::read_dir(root.join("scripts"))?.chain(fs::read_dir(root.join("bin"))?) {
            make_executable(&path?.path())?;
        }
        bind_scripts(&root)?;
        Ok(Self {
            _temp: temp,
            root,
            git_launcher: fixture_script_interpreter_path(&git)?,
            gh_launcher: fixture_script_interpreter_path(&gh)?,
        })
    }

    pub(crate) fn run_all(&self) -> Result<(), Box<dyn std::error::Error>> {
        let publish = self.run("publish-verified-release")?;
        ReleaseFixtureCommand::assert_outcome(
            "publish-verified-release",
            ReleaseFixtureOutcome::Success,
            &publish,
        );
        let finalize = self.run_with_settings(
            "finalize-verified-release",
            self.last_baseline_created()?,
            true,
        )?;
        ReleaseFixtureCommand::assert_outcome(
            "finalize-verified-release",
            ReleaseFixtureOutcome::Success,
            &finalize,
        );
        Ok(())
    }

    pub(crate) fn assert_outcome(
        operation: &str,
        expected: ReleaseFixtureOutcome,
        output: &Output,
    ) {
        ReleaseFixtureCommand::assert_outcome(operation, expected, output);
    }

    pub(crate) fn run(
        &self,
        name: &str,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_settings(name, false, true)
    }

    pub(crate) fn run_with_settings(
        &self,
        name: &str,
        baseline_created: bool,
        settings_allowed: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_policy(name, baseline_created, settings_allowed, true)
    }

    pub(crate) fn run_with_policy(
        &self,
        name: &str,
        baseline_created: bool,
        settings_allowed: bool,
        immutable: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        Ok(ReleaseFixtureCommand::new(self.root.join("scripts").join(name))
            .current_dir(&self.root)
            .path("FIXTURE_GIT", self.root.join("bin/git"))
            .path("FIXTURE_GIT_LAUNCHER", &self.git_launcher)
            .payload_path("FIXTURE_GH", self.root.join("bin/gh"))
            .payload_path("FIXTURE_GH_STATE_ROOT", &self.root)
            .path("FIXTURE_GH_LAUNCHER", &self.gh_launcher)
            .path(
                "FIXTURE_POSIX_SHELL",
                fixture_script_interpreter_path(&self.root.join("scripts/publish-verified-release"))?,
            )
            .path("FIXTURE_SCRIPT_ROOT", &self.root)
            .scalar("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .scalar("STAGING_SOURCE_COMMIT", COMMIT)
            .scalar("ACTIVATION_COMMIT", COMMIT)
            .scalar("STAGING_RUN_ID", "42")
            .scalar("RELEASE_TAG", "v9.9.9")
            .path("GITHUB_ENV", self.root.join("release.env"))
            .scalar("BASELINE_CREATED", baseline_created.to_string())
            .scalar("RELEASE_POLICY_TOKEN", "fixture-token")
            .scalar("SETTINGS_ALLOWED", settings_allowed.to_string())
            .scalar("FIXTURE_IMMUTABLE", immutable.to_string())
            .output()?)
    }

    pub(crate) fn last_baseline_created(&self) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("release.env"))?
            .lines()
            .rev()
            .find_map(|line| line.strip_prefix("BASELINE_CREATED="))
            == Some("true"))
    }
    pub(crate) fn assets(&self) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
        let names = fs::read_dir(self.root.join("remote"))?
            .map(|entry| {
                entry?
                    .file_name()
                    .into_string()
                    .map_err(|_| std::io::Error::other("asset"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ASSETS
            .into_iter()
            .chain(["release-baseline.json"])
            .filter(|name| names.iter().any(|actual| actual == name))
            .collect())
    }
    pub(crate) fn log(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("log")).unwrap_or_default())
    }
}
