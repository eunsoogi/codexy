use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Mutex, OnceLock},
};

use tempfile::tempdir;

struct CandidateFixtureSeed {
    _temporary: tempfile::TempDir,
    root: PathBuf,
}

static CANDIDATE_FIXTURE_SEED: OnceLock<Mutex<Option<CandidateFixtureSeed>>> = OnceLock::new();

pub(super) struct CandidateFixture {
    temp: tempfile::TempDir,
    source_commit: String,
}

impl CandidateFixture {
    pub(super) fn new(wrapper: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_dispatcher(wrapper, true)
    }

    pub(super) fn new_without_dispatcher(
        wrapper: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_dispatcher(wrapper, false)
    }

    fn new_with_dispatcher(
        wrapper: &str,
        include_dispatcher: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path();
        let seed_root = candidate_fixture_seed()?;
        crate::support::copy_dir(seed_root, root)?;
        let plugin = root.join("plugins/codexy-devtools");
        fs::write(plugin.join("mcp/codexy-mcp-devtools"), wrapper)?;
        if include_dispatcher {
            fs::write(
                root.join("staged-runtime/codexy-mcp-devtools-windows-x86_64.exe"),
                "dispatcher-windows\n",
            )?;
        }
        run_git(root, &["add", "plugins/codexy-devtools/mcp"])?;
        run_git(root, &["commit", "-qm", "fixture wrapper"])?;
        let source_commit = String::from_utf8(run_git(root, &["rev-parse", "HEAD"])?)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            temp,
            source_commit: source_commit.trim().into(),
        })
    }

    pub(super) fn assemble(&self) -> std::process::Output {
        self.assemble_with_target(Some("1.6.0"))
    }

    pub(super) fn assemble_with_target(
        &self,
        target_version: Option<&str>,
    ) -> std::process::Output {
        let mut command = Command::new("sh");
        command
            .arg("scripts/assemble-runtime-candidate")
            .current_dir(self.root())
            .env("SOURCE_COMMIT", &self.source_commit)
            .env("STAGING_RUN_ID", "1")
            .env("STAGING_RUN_ATTEMPT", "1")
            .env("GITHUB_SERVER_URL", "https://github.invalid")
            .env("GITHUB_REPOSITORY", "example/codexy")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.root().join("test-bin").display(),
                    std::env::var("PATH").expect("PATH")
                ),
            );
        if let Some(target_version) = target_version {
            command.env("TARGET_VERSION", target_version);
        } else {
            command.env_remove("TARGET_VERSION");
        }
        command.output().expect("candidate assembly starts")
    }

    pub(super) fn root(&self) -> &Path {
        self.temp.path()
    }

    pub(super) fn enable_core_runtime(&self) -> Result<(), Box<dyn std::error::Error>> {
        for (platform, extension) in [
            ("darwin-arm64", "bin"),
            ("linux-x86_64", "bin"),
            ("windows-x86_64", "exe"),
        ] {
            let path = self
                .root()
                .join("staged-runtime")
                .join(format!("codexy-handoff-validate-{platform}.{extension}"));
            fs::write(path, format!("handoff-{platform}\n"))?;
        }
        let repository = codexy_runtime::paths::repository_root();
        for source in [
            "packages/codexy-runtime/schemas/handoff-runtime.schema.json",
            "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.sh",
            "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.cmd",
            "plugins/codexy/skills/dreaming/scripts/resumable_context_capsule.py",
            "scripts/handoff_runtime_contract.py",
        ] {
            let target = self.root().join(source);
            fs::create_dir_all(target.parent().ok_or("core source parent")?)?;
            fs::copy(repository.join(source), target)?;
        }
        Ok(())
    }
}

fn candidate_fixture_seed() -> Result<PathBuf, Box<dyn std::error::Error>> {
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
        for (path, contents) in [
            (plugin.join(".codex-plugin/plugin.json"), r#"{"name":"codexy-devtools","version":"1.5.1"}"#),
            (contract.join("release-publish-contract.json"), r#"{"bootstrap":{"candidateVersion":"1.6.0"}}"#),
        ] {
            fs::write(path, format!("{contents}\n"))?;
        }
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
        fs::copy(
            codexy_runtime::paths::repository_root().join("scripts/assemble-runtime-candidate"),
            root.join("scripts/assemble-runtime-candidate"),
        )?;
        fs::copy(
            codexy_runtime::paths::repository_root().join("scripts/inspect-release-archive-contract.py"),
            root.join("scripts/inspect-release-archive-contract.py"),
        )?;
        let tar = root.join("test-bin/tar");
        fs::write(&tar, "#!/bin/sh\nexit 0\n")?;
        crate::support::make_executable(&tar)?;
        let rsync = root.join("test-bin/rsync");
        fs::write(
            &rsync,
            "#!/bin/sh\nset -eu\nsource=${8:?source}\ndestination=${9:?destination}\nmkdir -p \"$destination\"\ncp -R \"${source%/}/.\" \"$destination/\"\n",
        )?;
        crate::support::make_executable(&rsync)?;
        run_git(&root, &["init", "-q"])?;
        run_git(&root, &["config", "maintenance.auto", "false"])?;
        run_git(&root, &["config", "user.email", "test@example.invalid"])?;
        run_git(&root, &["config", "user.name", "Candidate Fixture"])?;
        run_git(&root, &["add", "."])?;
        run_git(&root, &["commit", "-qm", "fixture"])?;
        *seed = Some(CandidateFixtureSeed {
            _temporary: temporary,
            root,
        })
    }
    let seed = seed.as_ref().expect("candidate fixture seed");
    Ok(seed.root.clone())
}

fn run_git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(arguments).current_dir(root).output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!(
            "git {arguments:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::CandidateFixture;

    #[test]
    fn candidate_fixture_copies_do_not_share_seed_files() -> Result<(), Box<dyn std::error::Error>> {
        let first = CandidateFixture::new("first wrapper\n")?;
        let second = CandidateFixture::new("second wrapper\n")?;
        for fixture in [&first, &second] {
            let config = std::process::Command::new("git")
                .args(["config", "--get", "maintenance.auto"])
                .current_dir(fixture.root())
                .output()?;
            assert!(
                config.status.success(),
                "candidate fixture must copy its Git maintenance configuration: {}",
                String::from_utf8_lossy(&config.stderr)
            );
            assert_eq!(config.stdout, b"false\n");
        }
        std::fs::write(
            first.root().join("scripts/assemble-runtime-candidate"),
            "mutated script\n",
        )?;
        std::fs::write(
            first.root().join("plugins/codexy-devtools/mcp/codexy-mcp-devtools"),
            "mutated wrapper\n",
        )?;

        assert_ne!(
            std::fs::read(second.root().join("scripts/assemble-runtime-candidate"))?,
            b"mutated script\n"
        );
        assert_eq!(
            std::fs::read(second.root().join("plugins/codexy-devtools/mcp/codexy-mcp-devtools"))?,
            b"second wrapper\n"
        );
        assert_eq!(
            super::run_git(
                second.root(),
                &["show", "HEAD:plugins/codexy-devtools/mcp/codexy-mcp-devtools"],
            )?,
            b"second wrapper\n"
        );
        Ok(())
    }
}
