use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Output},
    rc::Rc,
};
#[path = "fixture_matrix_commands.rs"]
mod fixture_matrix_commands;

use crate::support::{self, FixtureCommand};
use fixture_matrix_commands::{fake_activator, fake_gh, fake_sync_version};
const AUTHORIZED: [&str; 10] = [
    "packages/codexy-runtime/Cargo.lock",
    "packages/codexy-runtime/Cargo.toml",
    ".agents/plugins/marketplace.json",
    ".agents/plugins/release-publish-contract.json",
    ".agents/plugins/runtime-activation.json",
    "plugins/codexy/.codex-plugin/plugin.json",
    "plugins/codexy-devtools/.codex-plugin/plugin.json",
    "plugins/codexy-github/.codex-plugin/plugin.json",
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
    "packages/codexy-runtime/src/version/bootstrap.rs",
];
const PRESERVED: [&str; 3] = [
    "plugins/codexy-devtools/mcp/codexy-mcp-codegraph",
    "plugins/codexy-devtools/mcp/codexy-mcp-lsp",
    "plugins/codexy-devtools/runtime-release.json",
];
#[derive(Clone, Copy, Debug)]
pub(super) enum Change {
    Exact,
    WrapperDrift,
    BootstrapDrift,
    ReleaseContractDrift,
    CargoVersionDrift,
    Extra,
    Missing,
}

pub(super) struct FixtureMatrix {
    pub(super) temp: tempfile::TempDir,
    seed_repo: PathBuf,
    pub(super) expected: PathBuf,
    pub(super) bin: PathBuf,
    pub(super) receipt: PathBuf,
    git_starts: Rc<Cell<usize>>,
    pub(super) verifier_starts: Rc<Cell<usize>>,
    pub(super) batched_case_count: Rc<Cell<usize>>,
}

pub(super) struct Fixture {
    _temp: tempfile::TempDir,
    pub(super) repo: PathBuf,
    pub(super) expected: PathBuf,
    pub(super) bin: PathBuf,
    pub(super) receipt: PathBuf,
    pub(super) verifier_starts: Rc<Cell<usize>>,
}

impl FixtureMatrix {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let seed_repo = temp.path().join("seed/repo");
        let expected = temp.path().join("expected");
        let bin = temp.path().join("bin");
        let git_starts = Rc::new(Cell::new(0));
        let verifier_starts = Rc::new(Cell::new(0));
        let batched_case_count = Rc::new(Cell::new(0));
        fs::create_dir_all(&seed_repo)?;
        fs::create_dir_all(&expected)?;
        fs::create_dir_all(&bin)?;
        git(&seed_repo, &["init", "-b", "main"], &git_starts)?;
        git(&seed_repo, &["config", "user.name", "test"], &git_starts)?;
        git(&seed_repo, &["config", "user.email", "test@example.com"], &git_starts)?;
        git(&seed_repo, &["config", "core.autocrlf", "false"], &git_starts)?;
        for path in AUTHORIZED {
            write(&seed_repo, path, format!("base:{path}\n").as_bytes())?;
            write(&expected, path, format!("derived:{path}\n").as_bytes())?;
        }
        for path in PRESERVED {
            write(&seed_repo, path, format!("base:{path}\n").as_bytes())?;
            write(&expected, path, format!("base:{path}\n").as_bytes())?;
        }
        fs::create_dir_all(seed_repo.join("scripts"))?;
        fake_sync_version(&seed_repo.join("scripts/sync-plugin-version.sh"))?;
        git(&seed_repo, &["add", "."], &git_starts)?;
        git(&seed_repo, &["commit", "-m", "base"], &git_starts)?;
        git(&seed_repo, &["switch", "-c", "activation"], &git_starts)?;
        copy_tree(&expected, &seed_repo)?;
        git(&seed_repo, &["add", "-A"], &git_starts)?;
        git(&seed_repo, &["commit", "-m", "activation"], &git_starts)?;
        fake_gh(&bin.join("gh"))?;
        fake_activator(&bin.join("activate"))?;
        let receipt = temp.path().join("receipt.json");
        fs::write(&receipt, "{}")?;
        Ok(Self { temp, seed_repo, expected, bin, receipt, git_starts, verifier_starts, batched_case_count })
    }

    pub(super) fn case(
        &self,
        change: Change,
    ) -> Result<Fixture, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        support::copy_dir(&self.seed_repo, &repo)?;
        match change {
            Change::Exact => {}
            Change::WrapperDrift => write(&repo, "plugins/codexy-devtools/mcp/codexy-mcp-codegraph", b"drift\n")?,
            Change::BootstrapDrift => write(
                &repo,
                "packages/codexy-runtime/src/version/bootstrap.rs",
                b"drift\n",
            )?,
            Change::ReleaseContractDrift => write(&repo, ".agents/plugins/release-publish-contract.json", b"drift\n")?,
            Change::CargoVersionDrift => write(
                &repo,
                "packages/codexy-runtime/Cargo.toml",
                b"drift\n",
            )?,
            Change::Extra => write(&repo, "docs/extra.md", b"extra\n")?,
            Change::Missing => fs::remove_file(repo.join(".agents/plugins/runtime-activation.json"))?,
        }
        if !matches!(change, Change::Exact) {
            git(&repo, &["add", "-A"], &self.git_starts)?;
            git(&repo, &["commit", "-m", "activation"], &self.git_starts)?;
        }
        Ok(Fixture {
            _temp: temp,
            repo,
            expected: self.expected.clone(),
            bin: self.bin.clone(),
            receipt: self.receipt.clone(),
            verifier_starts: self.verifier_starts.clone(),
        })
    }

    pub(super) fn git_setup_starts(&self) -> usize { self.git_starts.get() }
    pub(super) fn verifier_starts(&self) -> usize { self.verifier_starts.get() }
    pub(super) fn batched_case_count(&self) -> usize { self.batched_case_count.get() }
}

impl Fixture {
    pub(super) fn run(&self, pr_state: &str) -> Result<Output, Box<dyn std::error::Error>> {
        self.run_with_test_mode(pr_state, true)
    }

    fn run_with_test_mode(
        &self,
        pr_state: &str,
        test_mode: bool,
    ) -> Result<Output, Box<dyn std::error::Error>> {
        self.verifier_starts.set(self.verifier_starts.get() + 1);
        let mut command = FixtureCommand::new(
            codexy_runtime::paths::repository_root()
                .join("scripts/verify-runtime-activation-branch"),
        );
        let mut path = vec![self.bin.clone()];
        path.extend(std::env::split_paths(
            &std::env::var_os("PATH").ok_or("PATH")?,
        ));
        command.args([
            "activation",
            "main",
            "1.3.0",
            self.receipt.to_str().ok_or("receipt")?,
        ])
        .current_dir(&self.repo)
        .env("CODEXY_TEST_ACTIVATE_RUNTIME", self.bin.join("activate"))
        .env("EXPECTED_ROOT", &self.expected)
        .env("FAKE_PR_STATE", pr_state)
        .env_path_list("PATH", path);
        if test_mode {
            command.env("CODEXY_TEST_MODE", "1");
        }
        Ok(command.output()?)
    }
}

fn write(root: &Path, relative: &str, bytes: &[u8]) -> std::io::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for path in AUTHORIZED {
        write(target, path, &fs::read(source.join(path))?)?;
    }
    Ok(())
}

fn git(root: &Path, arguments: &[&str], starts: &Cell<usize>) -> Result<(), Box<dyn std::error::Error>> {
    starts.set(starts.get() + 1);
    let output = StdCommand::new("git").args(arguments).current_dir(root).output()?;
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
