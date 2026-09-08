use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use tempfile::tempdir;

struct CandidateFixtureSeed {
    _temporary: tempfile::TempDir,
    root: PathBuf,
}

static CANDIDATE_FIXTURE_SEED: OnceLock<Mutex<Option<CandidateFixtureSeed>>> = OnceLock::new();

pub(super) fn candidate_fixture_seed() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let seeds = CANDIDATE_FIXTURE_SEED.get_or_init(|| Mutex::new(None));
    let mut seed = seeds
        .lock()
        .map_err(|_| std::io::Error::other("candidate fixture seed lock"))?;
    if seed.is_none() {
        let temporary = tempdir()?;
        let root = temporary.path().join("repository");
        let plugin = root.join("plugins/codexy-devtools");
        fs::create_dir_all(plugin.join(".codex-plugin"))?;
        fs::create_dir_all(plugin.join("mcp"))?;
        fs::create_dir_all(root.join("staged-runtime"))?;
        fs::create_dir_all(root.join("scripts"))?;
        fs::create_dir_all(root.join("test-bin"))?;
        let contract = root.join(".agents/plugins");
        fs::create_dir_all(&contract)?;
        fs::write(
            plugin.join(".codex-plugin/plugin.json"),
            concat!(r#"{"name":"codexy-devtools","version":"1.5.1"}"#, "\n"),
        )?;
        fs::write(
            contract.join("release-publish-contract.json"),
            concat!(r#"{"bootstrap":{"candidateVersion":"1.6.0"}}"#, "\n"),
        )?;
        for server in ["lsp", "codegraph"] {
            for (platform, extension) in [
                ("darwin-arm64", "bin"),
                ("linux-x86_64", "bin"),
                ("windows-x86_64", "exe"),
            ] {
                fs::write(
                    root.join("staged-runtime")
                        .join(format!("codexy-mcp-{server}-{platform}.{extension}")),
                    format!("{server}-{platform}\n"),
                )?;
            }
        }
        for (platform, extension) in [
            ("darwin-arm64", "bin"),
            ("linux-x86_64", "bin"),
            ("windows-x86_64", "exe"),
        ] {
            fs::write(
                root.join("staged-runtime")
                    .join(format!("codexy-mcp-watcher-{platform}.{extension}")),
                format!("watcher-{platform}\n"),
            )?;
        }
        let repository = codexy_runtime::paths::repository_root();
        for name in [
            "assemble-runtime-candidate",
            "inspect-release-archive-contract.py",
            "inspect_release_archive_helpers.py",
            "inspect_release_archive_shell.py",
        ] {
            fs::copy(
                repository.join("scripts").join(name),
                root.join("scripts").join(name),
            )?;
        }
        let tar = root.join("test-bin/tar");
        fs::write(&tar, "#!/bin/sh\nexit 0\n")?;
        crate::support::make_executable(&tar)?;
        let rsync = root.join("test-bin/rsync");
        fs::write(
            &rsync,
            "#!/bin/sh\nset -eu\nsource=${8:?source}\ndestination=${9:?destination}\nmkdir -p \"$destination\"\ncp -R \"${source%/}/.\" \"$destination/\"\n",
        )?;
        crate::support::make_executable(&rsync)?;
        super::run_git(&root, &["init", "-q"])?;
        super::run_git(&root, &["config", "maintenance.auto", "false"])?;
        super::run_git(&root, &["config", "user.email", "test@example.invalid"])?;
        super::run_git(&root, &["config", "user.name", "Candidate Fixture"])?;
        super::run_git(&root, &["add", "."])?;
        super::run_git(&root, &["commit", "-qm", "fixture"])?;
        *seed = Some(CandidateFixtureSeed {
            _temporary: temporary,
            root,
        });
    }
    let seed = seed.as_ref().expect("candidate fixture seed");
    Ok(seed.root.clone())
}
