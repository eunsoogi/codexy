use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output};
use crate::support::FixtureCommand as Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt as _;

#[derive(Clone, Copy, Debug)]
pub(super) enum Scenario {
    NewPr,
    MatchingExisting,
    MismatchedIssue,
}

pub(super) struct WorkflowFixture {
    _temporary: tempfile::TempDir,
    repo: PathBuf,
    origin: PathBuf,
    baseline: String,
    state: PathBuf,
    runner: PathBuf,
    bin: PathBuf,
}

impl WorkflowFixture {
    pub(super) fn new(root: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let temporary = tempfile::tempdir()?;
        let repo = temporary.path().join("repo");
        let origin = temporary.path().join("origin.git");
        let state = temporary.path().join("state");
        let runner = temporary.path().join("runner");
        let bin = temporary.path().join("bin");
        for path in [&repo, &state, &runner, &bin] {
            fs::create_dir_all(path)?;
        }
        git(temporary.path(), &["init", "--bare", origin.to_str().ok_or("origin")?])?;
        git(&repo, &["init", "-q", "-b", "main"])?;
        git(&repo, &["config", "user.email", "codexy@example.test"])?;
        git(&repo, &["config", "user.name", "Codexy Fixture"])?;
        git(&repo, &["remote", "add", "origin", origin.to_str().ok_or("origin")?])?;
        copy_production(root, &repo, &bin)?;
        write_version_files(&repo)?;
        git(&repo, &["add", "."])?;
        git(&repo, &["commit", "-qm", "fixture main"])?;
        git(&repo, &["push", "-q", "-u", "origin", "main"])?;
        let baseline = git_stdout(&repo, &["rev-parse", "HEAD"])?.trim().to_owned();
        Ok(Self { _temporary: temporary, repo, origin, baseline, state, runner, bin })
    }

    pub(super) fn prepare(&self, scenario: Scenario) -> Result<(), Box<dyn std::error::Error>> {
        git(&self.repo, &["switch", "-q", "main"])?;
        git(&self.repo, &["reset", "--hard", &self.baseline])?;
        git(&self.repo, &["clean", "-fdx"])?;
        git(&self.repo, &["update-ref", "-d", "refs/heads/codexy/version-1.3.1"])?;
        git(&self.origin, &["update-ref", "-d", "refs/heads/codexy/version-1.3.1"])?;
        reset_directory(&self.state)?;
        reset_directory(&self.runner)?;
        fs::write(self.repo.join("packages/codexy-runtime/Cargo.toml"), "[workspace]\nresolver = \"3\"\n")?;
        let branch = "codexy/version-1.3.1";
        let mut existing = serde_json::json!([]);
        if !matches!(scenario, Scenario::NewPr) {
            git(&self.repo, &["switch", "-qc", branch])?;
            git(&self.repo, &["add", "packages/codexy-runtime/Cargo.toml"])?;
            git(&self.repo, &["commit", "-qm", "fixture version"])?;
            git(&self.repo, &["push", "-q", "-u", "origin", branch])?;
            let oid = git_stdout(&self.repo, &["rev-parse", "HEAD"])?;
            existing = serde_json::json!([{
                "number": 999,
                "headRefOid": oid.trim(),
                "headRepository": "eunsoogi/codexy",
                "headLabel": format!("eunsoogi:{branch}")
            }]);
            git(&self.repo, &["switch", "-q", "main"])?;
            fs::write(self.repo.join("packages/codexy-runtime/Cargo.toml"), "[workspace]\nresolver = \"3\"\n")?;
        }
        write_state(&self.state, scenario, existing)?;
        fs::write(self.state.join("mutation-sentinel"), b"unchanged\n")?;
        fs::write(self._temporary.path().join("summary.md"), b"")?;
        Ok(())
    }

    pub(super) fn run(&self) -> std::io::Result<Output> {
        self.run_with_issue("301")
    }

    pub(super) fn run_with_issue(&self, issue: &str) -> std::io::Result<Output> {
        let path = format!(
            "{}:{}",
            self.bin.display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new(self.repo.join("scripts/reconcile-version-pr"))
            .current_dir(&self.repo)
            .env("PATH", path)
            .env("FIXTURE_STATE", &self.state)
            .env("RUNNER_TEMP", &self.runner)
            .env("GITHUB_STEP_SUMMARY", self._temporary.path().join("summary.md"))
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("GH_TOKEN", "fixture")
            .env("ISSUE", issue)
            .env("VERSION", "1.3.1")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()
    }

    pub(super) fn artifact(&self, name: &str) -> PathBuf {
        self.runner.join("version-pr").join(name)
    }

    pub(super) fn mutation_events(&self) -> std::io::Result<Vec<String>> {
        match fs::read_to_string(self.state.join("mutations.log")) {
            Ok(log) => Ok(log.lines().map(str::to_owned).collect()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    pub(super) fn mutation_sentinel(&self) -> PathBuf {
        self.state.join("mutation-sentinel")
    }

    pub(super) fn gate_events(&self) -> std::io::Result<String> {
        fs::read_to_string(self.state.join("gates.log"))
    }
}

fn reset_directory(path: &Path) -> std::io::Result<()> {
    fs::remove_dir_all(path)?;
    fs::create_dir_all(path)
}

fn copy_production(root: &Path, repo: &Path, bin: &Path) -> std::io::Result<()> {
    for path in [
        "scripts/reconcile-version-pr",
        "scripts/canonicalize-version-pr-issue",
        "scripts/plan-version-pr-reconciliation",
        "scripts/render-version-pr-metadata",
        "scripts/version_pr_identity.py",
        "scripts/version_pr_tracks.py",
        "scripts/build-version-pr-state",
        "plugins/codexy-github/hooks/codexy-pr-title-check.sh",
        "plugins/codexy-github/hooks/codexy-pr-label-check.sh",
        "plugins/codexy-github/hooks/codexy-merge-message-check.sh",
    ] {
        copy_executable(&root.join(path), &repo.join(path))?;
    }
    fs::write(
        repo.join("plugins/codexy-github/hooks/codexy-readiness-guard.sh"),
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> \"$FIXTURE_STATE/gates.log\"\n",
    )?;
    fs::write(
        repo.join("scripts/validate-plugin-config.sh"),
        "#!/bin/sh\nset -eu\nprintf '%s\\n' \"$*\" >> \"$FIXTURE_STATE/gates.log\"\n",
    )?;
    copy_executable(
        &root.join("packages/codexy-runtime/tests/fixtures/version_pr_fake_gh.sh"),
        &bin.join("gh"),
    )?;
    for path in [
        repo.join("plugins/codexy-github/hooks/codexy-readiness-guard.sh"),
        repo.join("scripts/validate-plugin-config.sh"),
    ] {
        executable(&path)?;
    }
    Ok(())
}

fn copy_executable(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::create_dir_all(target.parent().expect("target parent"))?;
    fs::copy(source, target)?;
    executable(target)
}

fn executable(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

fn write_version_files(repo: &Path) -> std::io::Result<()> {
    for (path, text) in [
        ("packages/codexy-runtime/Cargo.toml", "[workspace]\nresolver = \"2\"\n"),
        ("packages/codexy-runtime/Cargo.lock", "# fixture\n"),
        (".agents/plugins/marketplace.json", "{}\n"),
        (".agents/plugins/release-publish-contract.json", "{}\n"),
        ("plugins/codexy/.codex-plugin/plugin.json", "{}\n"),
        ("plugins/codexy-devtools/.codex-plugin/plugin.json", "{}\n"),
        ("plugins/codexy-github/.codex-plugin/plugin.json", "{}\n"),
        ("packages/getcodexy/pyproject.toml", "[project]\nname='fixture'\n"),
    ] {
        let target = repo.join(path);
        fs::create_dir_all(target.parent().expect("version file parent"))?;
        fs::write(target, text)?;
    }
    Ok(())
}

fn write_state(state: &Path, scenario: Scenario, existing: serde_json::Value) -> std::io::Result<()> {
    let observed_issue = if matches!(scenario, Scenario::MismatchedIssue) { 302 } else { 301 };
    let values = [
        ("existing-prs.json", existing),
        ("issue.json", serde_json::json!({
            "number":301, "state":"OPEN", "url":"https://github.com/eunsoogi/codexy/issues/301",
            "labels":[{"name":"priority/medium"},{"name":"status/ready"},{"name":"type/ci"},{"name":"area/release"}],
            "milestone":{"title":"1.3.1"}, "assignees":[{"login":"eunsoogi"}]
        })),
        ("labels.json", serde_json::json!([
            {"name":"priority/medium"},{"name":"status/ready"},{"name":"status/review"},
            {"name":"type/ci"},{"name":"area/release"},{"name":"area/qa"}
        ])),
        ("observed-pr.json", serde_json::json!({
            "number":999, "headRefName":"codexy/version-1.3.1", "headRefOid":"0000000000000000000000000000000000000000",
            "body":format!("Tracks #{observed_issue}\n"), "labels":[{"name":"status/review"}],
            "closingIssuesReferences":[]
        })),
        ("review-threads.json", serde_json::json!({"pageInfo":{"hasNextPage":false},"nodes":[]})),
    ];
    for (name, value) in values {
        fs::write(state.join(name), serde_json::to_vec(&value)?)?;
    }
    fs::write(state.join("current-body.md"), format!("Tracks #{observed_issue}\n"))
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(())
}

fn git_stdout(root: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    Ok(String::from_utf8(Command::new("git").args(args).current_dir(root).output()?.stdout)?)
}
