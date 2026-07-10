use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const MANAGED: &[u8] = b"# CODEXY MANAGED AGENT\nname = \"codexy-sentinel\"\n";

#[cfg(unix)]
#[test]
fn registration_rejects_symlinked_ancestor_without_outside_writes() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let plugin = fixture(temp.path())?;
    let outside = temp.path().join("outside");
    std::fs::create_dir_all(outside.join("nested"))?;
    symlink(&outside, temp.path().join("selected"))?;

    let output = run(&plugin, &temp.path().join("selected/nested/.codex"), &[])?;

    assert!(!output.status.success(), "stdout:\n{}", stdout(&output));
    assert!(
        !outside.join("nested/.codex").exists(),
        "wrote through ancestor link"
    );
    Ok(())
}

#[test]
fn failed_install_removes_every_directory_it_created() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin = fixture(temp.path())?;
    let parent = temp.path().join("new-parent");
    let output = command(&plugin, &parent.join(".codex"))
        .env("CODEXY_AGENT_REGISTRATION_FAIL_AFTER", "1")
        .output()?;

    assert!(!output.status.success(), "failure injection was ignored");
    assert!(!parent.exists(), "failed transaction left {parent:?}");
    Ok(())
}

#[test]
fn identical_write_revalidates_the_planned_file() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin = fixture(temp.path())?;
    let path = temp.path().join("role.toml");
    std::fs::write(&path, MANAGED)?;
    let output = python(
        &plugin,
        &path,
        r#"
from agent_registration_fs import Transaction, snapshot
p = Path(sys.argv[2]); planned = snapshot(p); p.write_bytes(b"user-owned\n")
try: Transaction().write(p, planned.data, planned)
except RuntimeError: pass
else: raise AssertionError("identical write accepted a raced replacement")
assert p.read_bytes() == b"user-owned\n"
"#,
    )?;
    assert_python_success(output)
}

#[test]
fn rollback_refuses_to_delete_a_concurrent_user_replacement() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin = fixture(temp.path())?;
    let path = temp.path().join("role.toml");
    let output = python(
        &plugin,
        &path,
        r##"
from agent_registration_fs import FileState, Transaction
p = Path(sys.argv[2]); tx = Transaction(); tx.write(p, b"# CODEXY MANAGED AGENT\n", FileState(None))
p.write_bytes(b"user-owned\n")
try: tx.rollback()
except RuntimeError: pass
else: raise AssertionError("rollback accepted a raced replacement")
assert p.read_bytes() == b"user-owned\n"
"##,
    )?;
    assert_python_success(output)
}

#[test]
fn config_rewrite_refuses_a_concurrent_user_edit() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin = fixture(temp.path())?;
    let home = temp.path().join("home");
    std::fs::create_dir(&home)?;
    let output = python(
        &plugin,
        &home,
        r#"
from agent_registration_fs import Transaction
from agent_registration_lifecycle import RegistrationStore
home = Path(sys.argv[2]); config = home / "config.toml"; original = 'model = "old"\n'
config.write_text('user_concurrent = true\n')
try: RegistrationStore(home, "config.toml")._rewrite_config(Transaction(), original, original)
except RuntimeError: pass
else: raise AssertionError("concurrent config edit was overwritten")
assert config.read_text() == 'user_concurrent = true\n'
assert not list(home.glob("*.codexy-backup-*"))
"#,
    )?;
    assert_python_success(output)
}

#[test]
fn no_op_uninstall_still_honors_the_registration_lock() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin = fixture(temp.path())?;
    let home = temp.path().join("home");
    std::fs::create_dir(&home)?;
    let lock = home.join(".codexy-agent-registration.lock");
    std::fs::write(&lock, b"owner\n")?;

    let output = run(&plugin, &home, &["--uninstall"])?;

    assert!(!output.status.success(), "uninstall bypassed lock");
    assert_eq!(std::fs::read(lock)?, b"owner\n");
    Ok(())
}

#[test]
fn diagnostics_confirm_v2_only_from_the_real_table() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin = fixture(temp.path())?;
    let home = temp.path().join("home");
    assert!(run(&plugin, &home, &[])?.status.success());
    assert_unconfirmed(&run(&plugin, &home, &["--diagnose"])?);
    std::fs::write(
        home.join("config.toml"),
        "note = '''\n[features.multi_agent_v2]\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n'''\n",
    )?;
    assert_unconfirmed(&run(&plugin, &home, &["--diagnose"])?);
    for header in ["[[profiles]]", "[[\"profiles]\"]]", "[\"profiles]\"]"] {
        std::fs::write(
            home.join("config.toml"),
            format!(
                "[features.multi_agent_v2]\n{header}\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n"
            ),
        )?;
        let output = run(&plugin, &home, &["--diagnose"])?;
        assert!(stdout(&output).contains(
            "B tool-schema: CONFIGURED (namespace=default/unobserved, agent_type-visible=unconfirmed)"
        ));
    }
    for config in [
        "[features.multi_agent_v2]\nprobe = [\n  [\"nested\"],\n]\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n",
        "[\"features\".\"multi_agent_v2\"]\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n",
        "[features.multi_agent_v2]\nprobe = [\"\"\"\nnested\n\"\"\"]\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n",
        "[features.multi_agent_v2]\nprobe = [\"\"\"\nnested\n\"\"\"] # \"\"\"\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n",
        "[features.multi_agent_v2]\nprobe = [\"\"\"nested\"\"\"]\ntool_namespace = \"agents\"\nhide_spawn_agent_metadata = false\n",
    ] {
        std::fs::write(home.join("config.toml"), config)?;
        let output = run(&plugin, &home, &["--diagnose"])?;
        assert!(
            stdout(&output)
                .contains("B tool-schema: CONFIGURED (namespace=agents, agent_type-visible=true)")
        );
    }
    Ok(())
}

fn fixture(root: &Path) -> std::io::Result<PathBuf> {
    let plugin = root.join("plugin");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin,
    )?;
    Ok(plugin)
}

fn command(plugin: &Path, home: &Path) -> Command {
    let mut command =
        Command::new(plugin.join("skills/codex-orchestration/scripts/register-codexy-agents"));
    command
        .args(["--plugin-root"])
        .arg(plugin)
        .args(["--codex-home"])
        .arg(home);
    command
}

fn run(plugin: &Path, home: &Path, extra: &[&str]) -> std::io::Result<Output> {
    command(plugin, home).args(extra).output()
}

fn python(plugin: &Path, argument: &Path, body: &str) -> std::io::Result<Output> {
    let scripts = plugin.join("skills/codex-orchestration/scripts");
    let prelude = "import sys; sys.dont_write_bytecode=True\nfrom pathlib import Path\nsys.path.insert(0, sys.argv[1])\n";
    Command::new("python3")
        .args(["-c", &format!("{prelude}{body}"), scripts.to_str().unwrap()])
        .arg(argument)
        .output()
}

fn assert_python_success(output: Output) -> TestResult {
    assert!(
        output.status.success(),
        "python stderr:\n{}",
        stderr(&output)
    );
    Ok(())
}

fn assert_unconfirmed(output: &Output) {
    assert!(output.status.success(), "stderr:\n{}", stderr(output));
    assert!(
        stdout(output).contains("B tool-schema: UNCONFIRMED"),
        "stdout:\n{}",
        stdout(output)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
