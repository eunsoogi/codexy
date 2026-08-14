use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Output,
};

use crate::support::{
    self, FixtureCommand as Command, write_posix_fixture_command,
    write_posix_fixture_shell_runner_with_scrub,
};
use super::fixture_scripts::{gh_fixture, git_fixture, jq_fixture, remote_state};

const STAGING: &str = "0123456789abcdef0123456789abcdef01234567";
const ACTIVATION: &str = "89abcdef0123456789abcdef0123456789abcdef";

fn contextual_error(stage: &str, path: &Path, details: &str, error: io::Error) -> io::Error {
    let raw_os_error = error.raw_os_error();
    let message = format!(
        "{stage}: path={} {details} raw_os_error={raw_os_error:?}: {error}",
        path.display()
    );
    io::Error::new(error.kind(), message)
}
macro_rules! fixture_io {
    ($stage:expr, $path:expr, $result:expr) => {
        $result.map_err(|error| contextual_error($stage, $path, "", error))?
    };
}
fn fixture_output(command: &mut Command, path: &Path, cwd: &Path) -> io::Result<Output> {
    let program = command.get_program().to_string_lossy().into_owned();
    let argv = std::iter::once(program.clone())
        .chain(command.get_args().map(|arg| arg.to_string_lossy().into_owned()))
        .collect::<Vec<_>>()
        .join(" ");
    let details = format!("executable={program} cwd={} argv=[{argv}]", cwd.display());
    command
        .output()
        .map_err(|error| contextual_error("spawn/output fixture command", path, &details, error))
}

pub(super) fn assert_fixture_error_context(
    path: &Path,
    cwd: &Path,
    raw_os_error: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new(path);
    command.current_dir(cwd);
    let text = fixture_output(&mut command, path, cwd)
        .expect_err("fixture error")
        .to_string();
    let (prefix, fields) = text.split_once("path=").ok_or("path field")?;
    assert_eq!(prefix, "spawn/output fixture command: ");
    let (actual_path, fields) = fields.split_once(" executable=").ok_or("executable field")?;
    assert_eq!(actual_path, path.to_str().ok_or("fixture path")?);
    let (actual_executable, fields) = fields.split_once(" cwd=").ok_or("cwd field")?;
    let (actual_cwd, fields) = fields.split_once(" argv=[").ok_or("argv field")?;
    let (actual_argv, fields) = fields.split_once("] raw_os_error=").ok_or("raw error field")?;
    let (actual_raw, _) = fields.split_once(": ").ok_or("error detail")?;
    assert_eq!(actual_cwd, cwd.to_str().ok_or("cwd path")?);
    assert_eq!(actual_argv, actual_executable);
    if raw_os_error {
        assert_ne!(actual_raw, "None");
    }
    Ok(())
}
use super::fixture_state::RemoteTag;

pub(super) struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    script: PathBuf,
    runner: PathBuf,
    calls: PathBuf,
    pushes: PathBuf,
    api_calls: PathBuf,
    merge_base_calls: PathBuf,
}

impl Fixture {
    pub(super) fn new(state: RemoteTag) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = fixture_io!(
            "create fixture tempdir",
            Path::new("<tempdir>"),
            tempfile::tempdir()
        );
        let root = temp.path().join("release tag fixture with spaces");
        let bin = root.join("bin");
        fixture_io!(
            "create fixture dist directory",
            &root.join("dist"),
            fs::create_dir_all(root.join("dist"))
        );
        fixture_io!("create fixture bin directory", &bin, fs::create_dir(&bin));
        fixture_io!(
            "write fixture release receipt",
            &root.join("dist/runtime-release-receipt.json"),
            fs::write(root.join("dist/runtime-release-receipt.json"), "{}")
        );
        fixture_io!(
            "install release source-binding verifier",
            &root,
            install_source_binding_verifier(&root)
        );
        for (name, body) in [
            ("git", git_fixture()),
            ("jq", jq_fixture()),
            ("gh", gh_fixture()),
        ] {
            let path = bin.join(name);
            fixture_io!(
                &format!("write fixture command {name}"),
                &path,
                write_posix_fixture_command(&path, body)
            );
        }
        let script = root.join("release-step.sh");
        fixture_io!(
            "write release-step fixture",
            &script,
            fs::write(&script, format!("#!/bin/sh\nset -e\n{}", release_step()?))
        );
        fixture_io!(
            "chmod release-step fixture",
            &script,
            support::make_executable(&script)
        );
        let runner = root.join("bound-release-step.sh");
        fixture_io!(
            "write bound runner; sh -n validation and chmod",
            &runner,
            write_posix_fixture_shell_runner_with_scrub(
                &runner,
                "CODEXY_FIXTURE_RELEASE_STEP",
                &[
                    ("git", "CODEXY_FIXTURE_GIT"),
                    ("jq", "CODEXY_FIXTURE_JQ"),
                    ("gh", "CODEXY_FIXTURE_GH"),
                ],
                &[
                    "GIT_DIR",
                    "GIT_WORK_TREE",
                    "GIT_INDEX_FILE",
                    "GIT_COMMON_DIR",
                    "GH_CONFIG_DIR",
                    "GH_HOST",
                    "GH_ENTERPRISE_TOKEN",
                    "GH_TOKEN",
                    "GITHUB_TOKEN",
                ],
                &[("GH_TOKEN", "CODEXY_FIXTURE_GH_TOKEN")],
            )
        );
        fixture_io!(
            "write remote state",
            &root.join("remote-state"),
            fs::write(root.join("remote-state"), remote_state(state))
        );
        fixture_io!(
            "write remote query state",
            &root.join("remote-queries"),
            fs::write(root.join("remote-queries"), "0")
        );
        let calls = root.join("release-calls");
        let pushes = root.join("git-push-calls");
        let api_calls = root.join("api-calls");
        let merge_base_calls = root.join("merge-base-calls");
        Ok(Self {
            _temp: temp,
            root,
            script,
            runner,
            calls,
            pushes,
            api_calls,
            merge_base_calls,
        })
    }

    pub(super) fn run(&self) -> Result<Output, Box<dyn std::error::Error>> {
        self.run_with_inherited_state(&[])
    }

    pub(super) fn run_with_inherited_state(
        &self,
        inherited: &[(&str, &str)],
    ) -> Result<Output, Box<dyn std::error::Error>> {
        let mut command = Command::new(&self.runner);
        command.current_dir(&self.root);
        let mut command_path = vec![self.root.join("bin")];
        command_path.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        command.env_path_list("PATH", command_path);
        for (key, value) in inherited {
            command.env(key, value);
        }
        command
            .env_path("CODEXY_FIXTURE_RELEASE_STEP", &self.script)
            .env_path("CODEXY_FIXTURE_GIT", self.root.join("bin/git"))
            .env_path("CODEXY_FIXTURE_JQ", self.root.join("bin/jq"))
            .env_path("CODEXY_FIXTURE_GH", self.root.join("bin/gh"))
            .env_path("REMOTE_STATE", self.root.join("remote-state"))
            .env_path("REMOTE_QUERIES", self.root.join("remote-queries"))
            .env_path("FETCHED_STATE", self.root.join("fetched-state"))
            .env_path("RELEASE_CALLS", &self.calls)
            .env_path("GIT_PUSH_CALLS", &self.pushes)
            .env_path("API_CALLS", &self.api_calls)
            .env_path("MERGE_BASE_CALLS", &self.merge_base_calls)
            .env_path(
                "CODEXY_FIXTURE_COMMAND_TRACE",
                self.root.join("command-trace"),
            )
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("CODEXY_FIXTURE_GH_TOKEN", "fixture-token")
            .env("STAGING_SOURCE_COMMIT", STAGING)
            .env("ACTIVATION_COMMIT", ACTIVATION)
            .env("STAGING_RUN_ID", "42");
        Ok(fixture_output(&mut command, &self.runner, &self.root)?)
    }

    pub(super) fn release_calls(&self) -> Result<usize, Box<dyn std::error::Error>> {
        lines(&self.calls)
    }
    pub(super) fn git_push_calls(&self) -> Result<usize, Box<dyn std::error::Error>> {
        lines(&self.pushes)
    }
    pub(super) fn api_calls(&self) -> Result<usize, Box<dyn std::error::Error>> {
        lines(&self.api_calls)
    }
    pub(super) fn command_calls(&self, name: &str) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("command-trace"))?
            .lines()
            .filter(|line| *line == name)
            .count())
    }

    pub(super) fn remote_state(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.root.join("remote-state"))?
            .trim()
            .to_owned())
    }
}

use super::fixture_support::{install_source_binding_verifier, lines, release_step};
