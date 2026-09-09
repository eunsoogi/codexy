use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

pub(super) fn assert_projection_cases(
    projection: &str,
    workflow_steps: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, kind, expected_success) in [
        ("no-delta", "no-delta", true),
        ("allowed-verifier-delta", "verifier-delta", true),
        ("allowed-reconciliation-delta", "reconciliation-delta", true),
        ("allowed-finalizer-delta", "finalizer-delta", true),
        ("allowed-smoke-delta", "smoke-delta", true),
        ("allowed-host-delta", "host-delta", true),
        ("allowed-smoke-and-host-delta", "smoke-host-delta", true),
        ("non-executable-host", "host-mode-delta", false),
        ("forbidden-scripts-delta", "forbidden-delta", false),
    ] {
        for step in workflow_steps {
            let invocation = step.lines().filter(|line| line.contains("project-release-verifiers")).collect::<Vec<_>>().join("\n");
            run_projection_case(projection, &invocation, name, kind, expected_success)?;
        }
    }
    Ok(())
}

fn run_projection_case(
    projection: &str,
    invocation: &str,
    name: &str,
    kind: &str,
    expected_success: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path();
    run_git(root, &["init", "--quiet", "--initial-branch=main"])?;
    run_git(root, &["config", "user.email", "codexy-test@example.invalid"])?;
    run_git(root, &["config", "user.name", "codexy-test"])?;
    let scripts = root.join("scripts");
    fs::create_dir(&scripts)?;
    write_executable(&scripts.join("project-release-verifiers.sh"), projection)?;
    write_executable(&scripts.join("reconcile-release-attestations"), "activation-reconcile\n")?;
    write_executable(&scripts.join("verify-release-attestation-set"), "activation-set\n")?;
    write_executable(&scripts.join("finalize-verified-release"), "activation-finalizer\n")?;
    write_executable(
        &scripts.join("smoke-public-getcodexy-release.sh"),
        "activation-smoke\n",
    )?;
    write_executable(&scripts.join("fake_public_codex_host.py"), "activation-host\n")?;
    run_git(root, &["add", "scripts"])?;
    run_git(root, &["commit", "--quiet", "-m", "activation"])?;
    let activation = run_git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    match kind {
        "no-delta" => run_git(root, &["commit", "--quiet", "--allow-empty", "-m", "main"]).map(drop)?,
        "verifier-delta" => commit_script(root, &scripts, "verify-release-attestation-set", "activation-set\nchanged-set\n")?,
        "reconciliation-delta" => commit_script(root, &scripts, "reconcile-release-attestations", "main-reconcile\n")?,
        "finalizer-delta" => commit_script(root, &scripts, "finalize-verified-release", "main-finalizer\n")?,
        "smoke-delta" => commit_script(root, &scripts, "smoke-public-getcodexy-release.sh", "main-smoke\n")?,
        "host-delta" => commit_script(root, &scripts, "fake_public_codex_host.py", "main-host\n")?,
        "smoke-host-delta" => {
            commit_script(root, &scripts, "smoke-public-getcodexy-release.sh", "main-smoke\n")?;
            commit_script(root, &scripts, "fake_public_codex_host.py", "main-host\n")?;
        }
        "host-mode-delta" => {
            fs::set_permissions(scripts.join("fake_public_codex_host.py"), fs::Permissions::from_mode(0o644))?;
            commit_script(root, &scripts, "fake_public_codex_host.py", "main-host\n")?;
        }
        "forbidden-delta" => commit_script(root, &scripts, "unrelated-script", "forbidden\n")?,
        other => return Err(format!("unknown projection fixture: {other}").into()),
    }
    let current = run_git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    run_git(root, &["update-ref", "refs/remotes/origin/main", &current])?;
    let runner_temp = tempfile::tempdir()?;
    run_git(root, &["checkout", "--detach", &activation])?;
    let output = Command::new("sh")
        .arg("-eu")
        .arg("-c")
        .arg(invocation)
        .current_dir(root)
        .env("GITHUB_SHA", &current)
        .env("GITHUB_REF", "refs/heads/main")
        .env("ACTIVATION_COMMIT", &activation)
        .env("RUNNER_TEMP", runner_temp.path())
        .output()?;
    assert_eq!(
        output.status.success(),
        expected_success,
        "{name} projection case had unexpected status: {}",
        String::from_utf8_lossy(&output.stderr),
    );
    if expected_success {
        assert_eq!(run_git(root, &["rev-parse", "HEAD"])?.trim(), activation);
        let expected_verifier = if kind == "verifier-delta" { "activation-set\nchanged-set\n" } else { "activation-set\n" };
        let expected_reconciliation = if kind == "reconciliation-delta" { "main-reconcile\n" } else { "activation-reconcile\n" };
        let expected_finalizer = if kind == "finalizer-delta" { "main-finalizer\n" } else { "activation-finalizer\n" };
        let expected_smoke = if matches!(kind, "smoke-delta" | "smoke-host-delta") { "main-smoke\n" } else { "activation-smoke\n" };
        assert_eq!(fs::read_to_string(scripts.join("verify-release-attestation-set"))?, expected_verifier);
        assert_eq!(fs::read_to_string(scripts.join("reconcile-release-attestations"))?, expected_reconciliation);
        assert_eq!(fs::read_to_string(scripts.join("finalize-verified-release"))?, expected_finalizer);
        assert_eq!(fs::read_to_string(scripts.join("smoke-public-getcodexy-release.sh"))?, expected_smoke);
        assert!(!Command::new("git").current_dir(root).args(["symbolic-ref", "-q", "HEAD"]).status()?.success());
        let expected_host = if matches!(kind, "host-delta" | "smoke-host-delta") { "main-host\n" } else { "activation-host\n" };
        assert_eq!(fs::read_to_string(scripts.join("fake_public_codex_host.py"))?, expected_host);
        for path in ["scripts/smoke-public-getcodexy-release.sh", "scripts/fake_public_codex_host.py"] {
            assert!(fs::metadata(root.join(path))?.permissions().mode() & 0o111 != 0);
            assert_eq!(run_git(root, &["hash-object", path])?, run_git(root, &["rev-parse", &format!("{current}:{path}")])?);
        }
    }
    Ok(())
}

fn commit_script(root: &Path, scripts: &Path, name: &str, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(scripts.join(name), contents)?;
    run_git(root, &["add", &format!("scripts/{name}")])?;
    run_git(root, &["commit", "--quiet", "-m", "main"])?;
    Ok(())
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(cwd).args(args).output()?;
    if !output.status.success() {
        return Err(format!("git {args:?} failed: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn write_executable(path: &Path, contents: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, contents)?;
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
