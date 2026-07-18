use std::path::Path;
use std::process::Command;

mod support;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn bootstrap_and_registrar_reject_abbreviated_options() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let scripts = plugin_root.join("skills/codex-orchestration/scripts");
    let codex_home = temp.path().join("home/.codex");

    for arguments in [
        vec!["--plugin-r", path(&plugin_root)?, "--dry"],
        vec!["--unin", "--dry"],
    ] {
        let output = Command::new(scripts.join("bootstrap-codexy-agents"))
            .args(arguments)
            .env("CODEX_HOME", &codex_home)
            .output()?;
        assert!(
            !output.status.success(),
            "bootstrap accepted abbreviated options"
        );
        assert!(stderr(&output).contains("unsupported option"));
        assert!(!codex_home.exists(), "rejected options mutated CODEX_HOME");
    }

    let output = Command::new(scripts.join("register-codexy-agents"))
        .args(["--plugin-r", path(&plugin_root)?, "--dry"])
        .env("CODEX_HOME", &codex_home)
        .output()?;
    assert!(
        !output.status.success(),
        "registrar accepted abbreviated options"
    );
    assert!(stderr(&output).contains("unrecognized arguments"));
    assert!(!codex_home.exists(), "rejected options mutated CODEX_HOME");
    Ok(())
}

#[cfg(unix)]
#[test]
fn bootstrap_rejects_symlink_invocation_before_running_a_sibling() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");
    let attack_dir = temp.path().join("attack");
    std::fs::create_dir(&attack_dir)?;
    let linked_bootstrap = attack_dir.join("bootstrap-codexy-agents");
    symlink(&bootstrap, &linked_bootstrap)?;
    let marker = temp.path().join("attacker-ran");
    let attacker = attack_dir.join("register-codexy-agents");
    std::fs::write(
        &attacker,
        format!("#!/bin/sh\ntouch '{}'\n", path(&marker)?),
    )?;
    executable(&attacker)?;

    let output = Command::new(&linked_bootstrap).output()?;
    assert!(
        !output.status.success(),
        "bootstrap accepted symlink invocation"
    );
    assert!(stderr(&output).contains("symbolic link"));
    assert!(
        !marker.exists(),
        "bootstrap executed an untrusted sibling registrar"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn bootstrap_rejects_a_symlinked_registrar() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let scripts = plugin_root.join("skills/codex-orchestration/scripts");
    let registrar = scripts.join("register-codexy-agents");
    std::fs::remove_file(&registrar)?;
    let marker = temp.path().join("attacker-ran");
    let attacker = temp.path().join("attacker-registrar");
    std::fs::write(
        &attacker,
        format!("#!/bin/sh\ntouch '{}'\n", path(&marker)?),
    )?;
    executable(&attacker)?;
    symlink(&attacker, &registrar)?;

    let output = Command::new(scripts.join("bootstrap-codexy-agents")).output()?;
    assert!(
        !output.status.success(),
        "bootstrap accepted a symlinked registrar"
    );
    assert!(stderr(&output).contains("regular executable"));
    assert!(!marker.exists(), "bootstrap executed a symlinked registrar");
    Ok(())
}

#[cfg(unix)]
#[test]
fn validator_rejects_a_symlinked_bootstrap_entrypoint() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let bootstrap = plugin_root.join("skills/codex-orchestration/scripts/bootstrap-codexy-agents");
    let target = temp.path().join("outside-bootstrap");
    std::fs::rename(&bootstrap, &target)?;
    symlink(&target, &bootstrap)?;

    let output = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", path(&plugin_root)?, "--check-roles"])
        .output()?;
    assert!(
        !output.status.success(),
        "validator accepted a symlinked bootstrap"
    );
    assert!(stderr(&output).contains("must not be a symbolic link"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn plugin_root_bootstrap_and_validator_reject_symlink_entrypoints() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let root_bootstrap = plugin_root.join("bootstrap-codexy-agents");
    let linked = temp.path().join("linked-bootstrap");
    symlink(&root_bootstrap, &linked)?;
    let invocation = Command::new(&linked).output()?;
    assert!(
        !invocation.status.success(),
        "root bootstrap accepted a symlink invocation"
    );
    assert!(stderr(&invocation).contains("symbolic link"));

    let target = temp.path().join("outside-root-bootstrap");
    std::fs::rename(&root_bootstrap, &target)?;
    symlink(&target, &root_bootstrap)?;
    let validation = Command::new(env!("CARGO_BIN_EXE_codexy-validate"))
        .args(["--plugin-root", path(&plugin_root)?, "--check-roles"])
        .output()?;
    assert!(
        !validation.status.success(),
        "validator accepted a symlinked root bootstrap"
    );
    assert!(stderr(&validation).contains("must not be a symbolic link"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn bootstrap_checker_and_hook_ignore_hostile_path_dirname() -> TestResult {
    let temp = tempfile::tempdir()?;
    let plugin_root = installed_plugin(temp.path())?;
    let codex_home = temp.path().join("home/.codex");
    let fake_bin = temp.path().join("fake-bin");
    std::fs::create_dir_all(&fake_bin)?;
    let marker = temp.path().join("hostile-dirname-executed");
    let fake_dirname = fake_bin.join("dirname");
    std::fs::write(
        &fake_dirname,
        format!(
            "#!/bin/sh\nprintf ran > '{}'\nprintf '/tmp'\n",
            path(&marker)?
        ),
    )?;
    executable(&fake_dirname)?;
    let hostile_path = format!(
        "{}:{}",
        path(&fake_bin)?,
        std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin:/usr/sbin:/sbin".to_string())
    );

    let bootstrap = Command::new(plugin_root.join("bootstrap-codexy-agents"))
        .args(["--codex-home", path(&codex_home)?])
        .env("PATH", &hostile_path)
        .output()?;
    assert!(
        bootstrap.status.success(),
        "stderr:\n{}",
        stderr(&bootstrap)
    );

    let checker = Command::new(plugin_root.join("check-codexy-agents"))
        .env("CODEX_HOME", &codex_home)
        .env("PATH", &hostile_path)
        .output()?;
    assert!(checker.status.success(), "stderr:\n{}", stderr(&checker));

    let hook = Command::new(plugin_root.join("hooks/codexy-routing-context.sh"))
        .arg("SessionStart")
        .env("CODEX_HOME", &codex_home)
        .env("PATH", hostile_path)
        .output()?;
    assert!(hook.status.success(), "stderr:\n{}", stderr(&hook));
    assert!(!marker.exists(), "hostile PATH dirname was executed");
    Ok(())
}

fn installed_plugin(temp: &Path) -> TestResult<std::path::PathBuf> {
    let plugin_root = temp.join("installed-codexy");
    support::copy_dir(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("plugins/codexy"),
        &plugin_root,
    )?;
    Ok(plugin_root)
}

#[cfg(unix)]
fn executable(path: &Path) -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

fn path(path: &Path) -> Result<&str, Box<dyn std::error::Error>> {
    Ok(path.to_str().ok_or("path must be UTF-8")?)
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
