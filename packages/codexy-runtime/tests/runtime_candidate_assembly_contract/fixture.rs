use std::{
    fs,
    path::Path,
    process::Command,
};

use tempfile::tempdir;

mod fixture_seed;

type FixtureResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct CandidateFixture {
    temp: tempfile::TempDir,
    source_commit: String,
}

impl CandidateFixture {
    pub(super) const TARGET_VERSION: &str = "1.6.0";

    pub(super) fn new(wrapper: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_dispatcher(wrapper, true)
    }

    pub(super) fn new_without_dispatcher(wrapper: &str) -> FixtureResult<Self> {
        Self::new_with_dispatcher(wrapper, false)
    }

    fn new_with_dispatcher(wrapper: &str, include_dispatcher: bool) -> FixtureResult<Self> {
        let temp = tempdir()?;
        let root = temp.path();
        let seed_root = fixture_seed::candidate_fixture_seed()?;
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
        let source_commit = String::from_utf8(run_git(root, &["rev-parse", "HEAD"])?)?;
        Ok(Self {
            temp,
            source_commit: source_commit.trim().into(),
        })
    }

    pub(super) fn assemble(&self) -> std::process::Output {
        self.assemble_with_target(Some(Self::TARGET_VERSION))
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
            command.env("EXACT_PR_NUMBER", "7");
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

fn run_git(root: &Path, arguments: &[&str]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(arguments).current_dir(root).output()?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    Err(format!(
        "git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
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
