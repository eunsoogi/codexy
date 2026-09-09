use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use crate::support::{
    FixtureCommand, make_executable, write_posix_fixture_command,
    write_posix_fixture_shell_runner,
};

const CANDIDATE_VERSION: &str = "1.7.0";
const SYNC_PATHS: &[&str] = &["README.md", "README.ko.md"];
const STATIC_PATHS: &[&str] = &[
    ".agents/plugins/marketplace.json",
    ".agents/plugins/release-publish-contract.json",
    ".agents/plugins/runtime-activation.json",
    "packages/codexy-runtime/Cargo.lock",
    "packages/codexy-runtime/Cargo.toml",
    "packages/codexy-runtime/src/version/bootstrap.rs",
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
    "plugins/codexy-devtools/.codex-plugin/plugin.json",
    "plugins/codexy-devtools/mcp/codexy-mcp-devtools",
    "plugins/codexy-devtools/runtime-release.json",
    "plugins/codexy-github/.codex-plugin/plugin.json",
    "plugins/codexy/.codex-plugin/plugin.json",
];
const WATCHER_PATHS: &[&str] = &[
    "plugins/codexy/mcp/codexy-mcp-watcher.cmd",
    "plugins/codexy/mcp/codexy-mcp-watcher.sh",
];
const PRESERVED_PATHS: &[&str] = &[
    "plugins/codexy-devtools/mcp/codexy-mcp-codegraph",
    "plugins/codexy-devtools/mcp/codexy-mcp-lsp",
];

#[derive(Clone, Copy)]
pub(super) enum WatcherCase {
    Expected,
    Missing,
    Tampered,
    Unexpected,
}

pub(super) fn run_case(
    base_version: &str,
    watcher_case: WatcherCase,
) -> Result<Output, Box<dyn std::error::Error>> {
    run_case_with_state(base_version, watcher_case, false)
}

pub(super) fn run_staged_case(
    base_version: &str,
    watcher_case: WatcherCase,
) -> Result<Output, Box<dyn std::error::Error>> {
    run_case_with_state(base_version, watcher_case, true)
}

fn run_case_with_state(
    base_version: &str,
    watcher_case: WatcherCase,
    staged: bool,
) -> Result<Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let repo = temp.path().join("repo with spaces");
    let expected = temp.path().join("expected");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&repo)?;
    fs::create_dir_all(&expected)?;
    fs::create_dir_all(&bin)?;
    initialize_repository(&repo, &expected, base_version, watcher_case)?;
    if staged {
        git(&repo, &["reset", "--soft", "main"])?;
    }

    let receipt = temp.path().join("receipt.json");
    fs::write(&receipt, b"{}")?;
    let verifier = codexy_runtime::paths::repository_root()
        .join("scripts/verify-runtime-activation-branch");
    let runner = bin.join("verify.sh");
    let gh = bin.join("gh");
    let activator = bin.join("activate");
    let sync = bin.join("sync");
    write_posix_fixture_shell_runner(&runner, "CODEXY_FIXTURE_TARGET", &[("gh", "CODEXY_FIXTURE_GH")])?;
    write_posix_fixture_command(&gh, "#!/bin/sh\nprintf '%s\\n' OPEN\n")?;
    write_posix_fixture_command(&activator, &copy_fixture_script(false))?;
    write_posix_fixture_command(&sync, &copy_fixture_script(true))?;

    let mut command = FixtureCommand::new(&runner);
    command
        .args([
            "activation",
            "main",
            CANDIDATE_VERSION,
            receipt.to_str().ok_or("receipt path")?,
        ])
        .current_dir(&repo)
        .env_path("CODEXY_FIXTURE_TARGET", verifier)
        .env_path("CODEXY_FIXTURE_GH", &gh)
        .env_path("CODEXY_TEST_ACTIVATE_RUNTIME", &activator)
        .env_path("CODEXY_TEST_SYNC_VERSION_BINARY", &sync)
        .env_path("EXPECTED_ROOT", &expected)
        .env("CODEXY_TEST_MODE", "1");
    Ok(command.output()?)
}

fn initialize_repository(
    repo: &Path,
    expected: &Path,
    base_version: &str,
    watcher_case: WatcherCase,
) -> Result<(), Box<dyn std::error::Error>> {
    git(repo, &["init", "-b", "main"])?;
    git(repo, &["config", "core.autocrlf", "false"])?;
    git(repo, &["config", "user.name", "test"])?;
    git(repo, &["config", "user.email", "test@example.com"])?;
    for relative in STATIC_PATHS {
        write(repo, relative, format!("base:{relative}\n"))?;
        write(expected, relative, format!("derived:{relative}\n"))?;
    }
    for relative in SYNC_PATHS {
        write(repo, relative, format!("base:{relative}\n"))?;
        write(expected, relative, format!("derived:{relative}\n"))?;
    }
    for relative in ["packages/getcodexy/pyproject.toml", "packages/getcodexy/uv.lock"] {
        write(repo, relative, format!("version = \"{base_version}\"\n"))?;
        write(expected, relative, format!("version = \"{CANDIDATE_VERSION}\"\n"))?;
    }
    if base_version == CANDIDATE_VERSION {
        for relative in ["packages/getcodexy/pyproject.toml", "packages/getcodexy/uv.lock"] {
            write(expected, relative, format!("version = \"{base_version}\"\n"))?;
        }
    }
    for relative in PRESERVED_PATHS {
        write_executable(repo, relative, format!("base:{relative}\n"))?;
        write_executable(expected, relative, format!("base:{relative}\n"))?;
    }
    if !matches!(watcher_case, WatcherCase::Missing) {
        for relative in WATCHER_PATHS {
            write_executable(repo, relative, format!("base:{relative}\n"))?;
            write_executable(expected, relative, format!("derived:{relative}\n"))?;
        }
    }
    write_executable(repo, "scripts/activate-runtime-contract.sh", "#!/bin/sh\n")?;
    write_executable(repo, "scripts/sync-plugin-version.sh", "#!/bin/sh\n")?;
    git(repo, &["add", "."])?;
    git(
        repo,
        &[
            "add",
            "-f",
            "--",
            "packages/getcodexy/pyproject.toml",
            "packages/getcodexy/uv.lock",
        ],
    )?;
    git(repo, &["commit", "-m", "base"])?;
    git(repo, &["switch", "-c", "activation"])?;
    for relative in STATIC_PATHS
        .iter()
        .chain(["packages/getcodexy/pyproject.toml", "packages/getcodexy/uv.lock"].iter())
        .chain(SYNC_PATHS.iter())
        .chain(WATCHER_PATHS.iter())
    {
        if expected.join(relative).is_file() {
            fs::copy(expected.join(relative), repo.join(relative))?;
        }
    }
    match watcher_case {
        WatcherCase::Tampered => write(repo, WATCHER_PATHS[1], "tampered\n")?,
        WatcherCase::Unexpected => write(repo, "docs/unexpected.txt", "unexpected\n")?,
        WatcherCase::Expected | WatcherCase::Missing => {}
    }
    git(repo, &["add", "."])?;
    git(
        repo,
        &[
            "add",
            "-f",
            "--",
            "packages/getcodexy/pyproject.toml",
            "packages/getcodexy/uv.lock",
        ],
    )?;
    git(repo, &["commit", "-m", "activation"])?;
    Ok(())
}

fn copy_fixture_script(sync: bool) -> String {
    let mut source = String::from("#!/bin/sh\nset -eu\n");
    if sync {
        source.push_str("root=\"${CODEXY_REPO_ROOT:?}\"\n");
    } else {
        source.push_str("root=\nwhile [ \"$#\" -gt 0 ]; do\n  case \"$1\" in\n    --repo-root) root=\"$2\"; shift 2 ;;\n    *) shift ;;\n  esac\ndone\ntest -n \"$root\"\n");
    }
    for relative in STATIC_PATHS
        .iter()
        .chain(["packages/getcodexy/pyproject.toml", "packages/getcodexy/uv.lock"].iter())
        .chain(SYNC_PATHS.iter())
        .chain(WATCHER_PATHS.iter())
    {
        source.push_str(&format!(
            "if test -f \"$EXPECTED_ROOT/{relative}\"; then mkdir -p \"$root/$(dirname '{relative}')\"; cp \"$EXPECTED_ROOT/{relative}\" \"$root/{relative}\"; fi\n"
        ));
    }
    source
}

fn write(root: &Path, relative: &str, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn write_executable(
    root: &Path,
    relative: &str,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    let path = root.join(relative);
    write(root, relative, contents)?;
    make_executable(&path)
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned().into())
    }
}
