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

struct FixtureSeed {
    _temp: tempfile::TempDir,
    repo: PathBuf,
    candidate: String,
}

impl FixtureSeed {
    fn create() -> Result<Self, Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let prepared = temp.path().join("prepared");
        fs::create_dir(&prepared)?;
        let candidate = prepare(&prepared, temp.path())?;
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
    git(repo, &["init", "-b", "main"])?;
    configure_repository(repo)?;
    git(repo, &["add", "."])?;
    git(repo, &["commit", "-m", "base"])
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

fn prepare(repo: &Path, temp_root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let historical_archive = temp_root.join("historical.tar");
    let pre_activation_revision = metadata::pre_activation_revision()?;
    let mut archive_historical = Command::new("git");
    archive_historical
        .args(["archive", "--format=tar", &pre_activation_revision])
        .arg("-o")
        .arg(&historical_archive)
        .current_dir(codexy_runtime::paths::repository_root());
    run(&mut archive_historical)?;
    let mut extract_historical = Command::new("tar");
    extract_historical
        .arg("-xf")
        .arg(&historical_archive)
        .arg("-C")
        .arg(repo);
    run(&mut extract_historical)?;
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

fn run(process: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    command::run(process)
}
