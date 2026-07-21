use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::support;

const MANAGED: &str = "# CODEXY MANAGED AGENT\n";
const PERSONAL: &[u8] = b"name = \"personal\"\ndescription = \"keep these bytes\"\n";
#[rustfmt::skip]
const ROLES: [&str; 12] = ["codexy-architect", "codexy-auditor", "codexy-cartographer", "codexy-forge", "codexy-pathfinder", "codexy-scribe", "codexy-sculptor", "codexy-sentinel", "codexy-shipwright", "codexy-tracer", "codexy-warden", "codexy-weaver"];

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Tree = BTreeMap<PathBuf, Option<Vec<u8>>>;

#[derive(Clone, Copy, Debug)]
enum Operation {
    Install,
    Update,
    Uninstall,
}

#[test]
fn install_failure_is_transactional() -> TestResult {
    assert_failure_is_transactional(Operation::Install)
}

#[test]
fn update_failure_is_transactional() -> TestResult {
    assert_failure_is_transactional(Operation::Update)
}

#[test]
fn uninstall_failure_is_transactional() -> TestResult {
    assert_failure_is_transactional(Operation::Uninstall)
}

fn assert_failure_is_transactional(operation: Operation) -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    seed_user_state(&codex_home)?;

    if !matches!(operation, Operation::Install) {
        assert_success(run(&plugin_root, &codex_home, &[])?)?;
    }
    seed_legacy_config(&codex_home.join("config.toml"))?;
    if matches!(operation, Operation::Update) {
        let root = codex_home.join("agents/codexy");
        std::fs::write(
            root.join("codexy-sentinel.toml"),
            format!("{MANAGED}name = \"stale-sentinel\"\n"),
        )?;
        std::fs::write(
            root.join("codexy-retired.toml"),
            format!("{MANAGED}name = \"codexy-retired\"\n"),
        )?;
    }

    let before = snapshot(&codex_home)?;
    let extra = if matches!(operation, Operation::Uninstall) {
        &["--uninstall"][..]
    } else {
        &[]
    };
    let fail_after = ["14", "4", "14"][operation as usize];
    let output = registration_command(&plugin_root, &codex_home)
        .env("CODEXY_AGENT_REGISTRATION_FAIL_AFTER", fail_after)
        .args(extra)
        .output()?;

    assert!(
        !output.status.success(),
        "{operation:?} ignored injected failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        snapshot(&codex_home)?,
        before,
        "{operation:?} failure changed config, backups, agents, or user files"
    );
    Ok(())
}

#[test]
fn repeated_lifecycle_keeps_exact_roles_and_personal_file() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    seed_user_state(&codex_home)?;

    assert_idempotent(&plugin_root, &codex_home, &[], "install")?;
    let source = plugin_root.join("agents/codexy-sentinel.toml");
    let updated = format!("{}\n# updated fixture\n", std::fs::read_to_string(&source)?);
    std::fs::write(source, updated)?;
    assert_idempotent(&plugin_root, &codex_home, &[], "update")?;
    assert_idempotent(&plugin_root, &codex_home, &["--uninstall"], "uninstall")?;
    assert_success(run(&plugin_root, &codex_home, &[])?)?;

    let agents_root = codex_home.join("agents/codexy");
    assert_eq!(managed_roles(&agents_root)?, expected_roles());
    assert_eq!(std::fs::read(agents_root.join("personal.toml"))?, PERSONAL);
    Ok(())
}

#[test]
fn install_and_uninstall_dry_runs_preserve_the_complete_tree() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_fixture(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    seed_user_state(&codex_home)?;
    assert_success(run(&plugin_root, &codex_home, &[])?)?;
    seed_legacy_config(&codex_home.join("config.toml"))?;
    std::fs::write(
        codex_home.join("agents/codexy/codexy-retired.toml"),
        format!("{MANAGED}name = \"codexy-retired\"\n"),
    )?;
    let before = snapshot(&codex_home)?;

    assert_success(run(&plugin_root, &codex_home, &["--dry-run"])?)?;
    assert_eq!(
        snapshot(&codex_home)?,
        before,
        "install dry-run changed state"
    );
    assert_success(run(
        &plugin_root,
        &codex_home,
        &["--dry-run", "--uninstall"],
    )?)?;
    assert_eq!(
        snapshot(&codex_home)?,
        before,
        "uninstall dry-run mutated state"
    );
    Ok(())
}

fn seed_user_state(codex_home: &Path) -> std::io::Result<()> {
    let agents_root = codex_home.join("agents/codexy");
    std::fs::create_dir_all(&agents_root)?;
    std::fs::write(codex_home.join("config.toml"), b"model = \"gpt-5.5\"\n")?;
    std::fs::write(
        codex_home.join("config.toml.codexy-backup-existing"),
        b"original backup bytes\n",
    )?;
    std::fs::write(agents_root.join("personal.toml"), PERSONAL)
}

fn seed_legacy_config(config: &Path) -> std::io::Result<()> {
    let mut contents = std::fs::read_to_string(config)?;
    contents.push_str(
        "\n# BEGIN CODEXY MANAGED AGENTS\n[agents.codexy-sentinel]\nconfig_file = \"stale\"\n# END CODEXY MANAGED AGENTS\n",
    );
    std::fs::write(config, contents)
}

fn installed_fixture(root: &Path) -> std::io::Result<PathBuf> {
    let plugin_root = root.join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

fn registration_command(plugin_root: &Path, codex_home: &Path) -> Command {
    let mut command =
        Command::new(plugin_root.join("skills/codex-orchestration/scripts/register-codexy-agents"));
    command
        .env_remove("CODEXY_AGENT_REGISTRATION_FAIL_AFTER")
        .arg("--plugin-root")
        .arg(plugin_root)
        .arg("--codex-home")
        .arg(codex_home);
    command
}

fn run(plugin_root: &Path, codex_home: &Path, extra: &[&str]) -> std::io::Result<Output> {
    registration_command(plugin_root, codex_home)
        .args(extra)
        .output()
}

fn assert_success(output: Output) -> TestResult {
    assert!(
        output.status.success(),
        "registration failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn assert_idempotent(plugin_root: &Path, home: &Path, extra: &[&str], label: &str) -> TestResult {
    assert_success(run(plugin_root, home, extra)?)?;
    let once = snapshot(home)?;
    assert_success(run(plugin_root, home, extra)?)?;
    assert_eq!(snapshot(home)?, once, "{label} repeated with side effects");
    Ok(())
}

fn expected_roles() -> BTreeSet<String> {
    ROLES.into_iter().map(str::to_owned).collect()
}

fn managed_roles(root: &Path) -> std::io::Result<BTreeSet<String>> {
    let mut roles = BTreeSet::new();
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("toml")
            && std::fs::read_to_string(&path)?.starts_with(MANAGED)
        {
            roles.insert(path.file_stem().unwrap().to_string_lossy().into_owned());
        }
    }
    Ok(roles)
}

fn snapshot(root: &Path) -> std::io::Result<Tree> {
    let mut tree = Tree::new();
    snapshot_dir(root, root, &mut tree)?;
    Ok(tree)
}

fn snapshot_dir(base: &Path, current: &Path, tree: &mut Tree) -> std::io::Result<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path.strip_prefix(base).unwrap().to_owned();
        if entry.file_type()?.is_dir() {
            tree.insert(relative, None);
            snapshot_dir(base, &path, tree)?;
        } else {
            tree.insert(relative, Some(std::fs::read(path)?));
        }
    }
    Ok(())
}
