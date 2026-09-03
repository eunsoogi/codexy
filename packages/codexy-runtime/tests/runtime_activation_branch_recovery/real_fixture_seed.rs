use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use super::{command, metadata, real_source_pointer};

pub(super) struct MaterializedFixture {
    pub(super) temp: tempfile::TempDir,
    pub(super) repo: PathBuf,
    pub(super) candidate: String,
}

const PREPARED_PATHS: [&str; 19] = [
    ".agents/plugins",
    ".gitattributes",
    ".github/workflows/plugin-runtime-binaries.yml",
    "README.md",
    "README.ko.md",
    "docs/getcodexy-component-installation.md",
    "packages/codexy-runtime",
    "packages/getcodexy/contracts/component-installation-contract.json",
    "packages/getcodexy/pyproject.toml",
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json",
    "packages/getcodexy/tests/fixtures/component-installation-cases.json",
    "packages/getcodexy/uv.lock",
    "plugins/codexy",
    "plugins/codexy-devtools",
    "plugins/codexy-github",
    "scripts/activate-runtime-contract.sh",
    "scripts/download-selected-runtime-package.sh",
    "scripts/sync-plugin-version.sh",
    "scripts/verify-runtime-activation-branch",
];

struct FixtureSeed {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    candidate: String,
}

impl FixtureSeed {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let prepared = temp.path().join("prepared");
        let candidate = prepare(&prepared)?;
        initialize_repository(&prepared)?;
        Ok(Self {
            _temp: temp,
            repo: prepared,
            candidate,
        })
    }

    fn materialize(&self) -> Result<MaterializedFixture, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo with spaces");
        let mut clone = Command::new("git");
        clone
            .args(["-c", "core.autocrlf=false", "clone", "--shared"])
            .arg(&self.repo)
            .arg(&repo);
        run(&mut clone)?;
        configure_repository(&repo)?;
        Ok(MaterializedFixture {
            temp,
            repo,
            candidate: self.candidate.clone(),
        })
    }
}

fn initialize_repository(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    configure_repository(repo)?;
    let mut add = Command::new("git");
    add.args(["add", "-A", "--"])
        .args(PREPARED_PATHS)
        .current_dir(repo);
    run(&mut add)?;
    let tree = git_output(repo, &["write-tree"])?;
    let commit = git_output(repo, &["commit-tree", &tree, "-m", "base"])?;
    git(repo, &["reset", "--hard", &commit])?;
    git(repo, &["switch", "-C", "main"])
}

fn configure_repository(repo: &Path) -> Result<(), Box<dyn std::error::Error>> {
    git(repo, &["config", "user.name", "test"])?;
    git(repo, &["config", "user.email", "test@example.com"])
}

pub(super) fn materialize() -> Result<MaterializedFixture, Box<dyn std::error::Error>> {
    static SEED: OnceLock<Result<FixtureSeed, String>> = OnceLock::new();
    match SEED.get_or_init(|| FixtureSeed::create().map_err(|error| error.to_string())) {
        Ok(seed) => seed.materialize(),
        Err(error) => Err(error.clone().into()),
    }
}

fn prepare(repo: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let pre_activation_revision = metadata::pre_activation_revision()?;
    let mut clone = Command::new("git");
    clone
        .args(["-c", "core.autocrlf=false", "clone", "--shared", "--no-tags", "--no-checkout"])
        .arg(codexy_runtime::paths::repository_root())
        .arg(repo);
    run(&mut clone)?;
    git(repo, &["checkout", "--detach", &pre_activation_revision])?;
    for relative in [
        "packages/codexy-runtime/schemas/handoff-runtime.schema.json",
        "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.sh",
        "plugins/codexy/skills/dreaming/scripts/resumable-context-capsule.cmd",
        "plugins/codexy/skills/dreaming/scripts/resumable_context_capsule.py",
    ] {
        let path = repo.join(relative);
        if path.is_file() {
            fs::remove_file(path)?;
        }
    }
    let runtime = repo.join("packages/codexy-runtime");
    fs::create_dir_all(runtime.join("src/version"))?;
    let suite = runtime.join("tests/suites/all.rs");
    fs::create_dir_all(suite.parent().ok_or("suite parent")?)?;
    fs::copy(
        codexy_runtime::paths::runtime_package_root().join("tests/suites/all.rs"),
        suite,
    )?;
    for relative in ["Cargo.toml", "Cargo.lock"] {
        let path = repo.join(relative);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    let candidate = metadata::current_candidate_version()?;
    metadata::synchronize_current_plugin_validation_inputs(repo)?;
    let marketplace_command = "codex plugin marketplace add eunsoogi/codexy";
    let pin = format!("{marketplace_command} --ref v{candidate}");
    for relative in ["README.md", "README.ko.md"] {
        let path = repo.join(relative);
        let text = fs::read_to_string(&path)?;
        fs::write(path, text.replacen(marketplace_command, &pin, 1))?;
    }
    real_source_pointer::restore_pre_activation_runtime_inputs(repo, &pre_activation_revision)?;
    metadata::make_uv_lock_stale(repo)?;
    let workflow = ".github/workflows/plugin-runtime-binaries.yml";
    let workflow_target = repo.join(workflow);
    fs::create_dir_all(workflow_target.parent().ok_or("workflow parent")?)?;
    fs::copy(
        codexy_runtime::paths::repository_root().join(workflow),
        workflow_target,
    )?;
    for relative in [
        "scripts/activate-runtime-contract.sh",
        "scripts/sync-plugin-version.sh",
        "scripts/verify-runtime-activation-branch",
    ] {
        fs::copy(
            codexy_runtime::paths::repository_root().join(relative),
            repo.join(relative),
        )?;
    }
    Ok(candidate)
}

fn git(root: &Path, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = Command::new("git");
    command.args(args).current_dir(root);
    run(&mut command)
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned().into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn run(process: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    command::run(process)
}
