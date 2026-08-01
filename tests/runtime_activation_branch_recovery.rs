use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::support::{FixtureCommand, make_executable};

#[path = "runtime_activation_branch_recovery/real.rs"]
mod real;

const AUTHORIZED: [&str; 9] = [
    "Cargo.lock",
    "Cargo.toml",
    ".agents/plugins/marketplace.json",
    ".agents/plugins/release-publish-contract.json",
    ".agents/plugins/runtime-activation.json",
    "plugins/codexy/.codex-plugin/plugin.json",
    "plugins/codexy/mcp/codexy-mcp-codegraph",
    "plugins/codexy/mcp/codexy-mcp-lsp",
    "src/version/bootstrap.rs",
];
const REMOVED: [&str; 1] = ["plugins/codexy/runtime-release.json"];

#[test]
fn existing_activation_branch_authenticates_exact_derived_tree_and_pr_state()
-> Result<(), Box<dyn std::error::Error>> {
    let exact = Fixture::new(Change::Exact)?.run("OPEN")?;
    assert!(
        exact.status.success(),
        "exact activation failed: {}",
        String::from_utf8_lossy(&exact.stderr)
    );
    for change in [
        Change::WrapperDrift,
        Change::BootstrapDrift,
        Change::ReleaseContractDrift,
        Change::CargoVersionDrift,
        Change::Extra,
        Change::Missing,
    ] {
        assert!(
            !Fixture::new(change)?.run("OPEN")?.status.success(),
            "{change:?} unexpectedly passed"
        );
    }
    assert!(!Fixture::new(Change::Exact)?.run("CLOSED")?.status.success());
    assert!(!Fixture::new(Change::Exact)?.run("OPEN\nOPEN")?.status.success());
    assert!(
        !Fixture::new(Change::Exact)?
            .run_without_test_mode("OPEN")?
            .status
            .success()
    );
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum Change {
    Exact,
    WrapperDrift,
    BootstrapDrift,
    ReleaseContractDrift,
    CargoVersionDrift,
    Extra,
    Missing,
}

struct Fixture {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    bin: PathBuf,
    receipt: PathBuf,
}

impl Fixture {
    fn new(change: Change) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        let expected = temp.path().join("expected");
        let bin = temp.path().join("bin");
        fs::create_dir_all(&repo)?;
        fs::create_dir_all(&expected)?;
        fs::create_dir_all(&bin)?;
        git(&repo, &["init", "-b", "main"])?;
        git(&repo, &["config", "user.name", "test"])?;
        git(&repo, &["config", "user.email", "test@example.com"])?;
        for path in AUTHORIZED {
            write(&repo, path, format!("base:{path}\n").as_bytes())?;
            write(&expected, path, format!("derived:{path}\n").as_bytes())?;
        }
        for path in REMOVED {
            write(&repo, path, format!("base:{path}\n").as_bytes())?;
        }
        fs::create_dir_all(repo.join("scripts"))?;
        fake_sync_version(&repo.join("scripts/sync-plugin-version"))?;
        git(&repo, &["add", "."])?;
        git(&repo, &["commit", "-m", "base"])?;
        git(&repo, &["switch", "-c", "activation"])?;
        copy_tree(&expected, &repo)?;
        fs::remove_file(repo.join("plugins/codexy/runtime-release.json"))?;
        match change {
            Change::Exact => {}
            Change::WrapperDrift => write(&repo, "plugins/codexy/mcp/codexy-mcp-codegraph", b"drift\n")?,
            Change::BootstrapDrift => write(&repo, "src/version/bootstrap.rs", b"drift\n")?,
            Change::ReleaseContractDrift => write(&repo, ".agents/plugins/release-publish-contract.json", b"drift\n")?,
            Change::CargoVersionDrift => write(&repo, "Cargo.toml", b"drift\n")?,
            Change::Extra => write(&repo, "docs/extra.md", b"extra\n")?,
            Change::Missing => fs::remove_file(repo.join(".agents/plugins/runtime-activation.json"))?,
        }
        git(&repo, &["add", "-A"])?;
        git(&repo, &["commit", "-m", "activation"])?;
        fake_gh(&bin.join("gh"))?;
        fake_activator(&bin.join("activate"))?;
        let receipt = temp.path().join("receipt.json");
        fs::write(&receipt, "{}")?;
        Ok(Self {
            _temp: temp,
            repo,
            bin,
            receipt,
        })
    }

    fn run(&self, pr_state: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_test_mode(pr_state, true)
    }

    fn run_without_test_mode(
        &self,
        pr_state: &str,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        self.run_with_test_mode(pr_state, false)
    }

    fn run_with_test_mode(
        &self,
        pr_state: &str,
        test_mode: bool,
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        let mut command = FixtureCommand::new(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("scripts/verify-runtime-activation-branch"),
        );
        command.args([
            "activation",
            "main",
            "1.3.0",
            self.receipt.to_str().ok_or("receipt")?,
        ])
        .current_dir(&self.repo)
        .env(
            "PATH",
            format!("{}:{}", self.bin.display(), std::env::var("PATH")?),
        )
        .env("CODEXY_TEST_ACTIVATE_RUNTIME", self.bin.join("activate"))
        .env("EXPECTED_ROOT", self._temp.path().join("expected"))
        .env("FAKE_PR_STATE", pr_state);
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

fn git(root: &Path, arguments: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").args(arguments).current_dir(root).output()?;
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn fake_gh(path: &Path) -> std::io::Result<()> {
    executable(path, "#!/bin/sh\nprintf '%s\n' \"$FAKE_PR_STATE\"\n")
}

fn fake_activator(path: &Path) -> std::io::Result<()> {
    executable(
        path,
        r##"#!/bin/sh
set -eu
while [ "$#" -gt 0 ]; do
  case "$1" in
    --repo-root) root="$2"; shift 2 ;;
    *) shift ;;
  esac
done
for path in \
  .agents/plugins/marketplace.json \
  .agents/plugins/release-publish-contract.json \
  .agents/plugins/runtime-activation.json \
  plugins/codexy/.codex-plugin/plugin.json \
  plugins/codexy/mcp/codexy-mcp-codegraph \
  plugins/codexy/mcp/codexy-mcp-lsp \
  src/version/bootstrap.rs
do
  mkdir -p "$root/$(dirname "$path")"
  cp "$EXPECTED_ROOT/$path" "$root/$path"
done
rm -f "$root/plugins/codexy/runtime-release.json"
"##,
    )
}

fn fake_sync_version(path: &Path) -> std::io::Result<()> {
    executable(
        path,
        r##"#!/bin/sh
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
for path in Cargo.toml Cargo.lock; do
  cp "$EXPECTED_ROOT/$path" "$root/$path"
done
"##,
    )
}

fn executable(path: &Path, source: &str) -> std::io::Result<()> {
    fs::write(path, source)?;
    make_executable(path)
}
