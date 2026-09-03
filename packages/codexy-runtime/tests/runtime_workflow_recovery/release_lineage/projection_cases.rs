use std::{fs, os::unix::fs::PermissionsExt, path::Path, process::Command};

pub(super) fn assert_projection_cases(
    projection: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for (name, kind, expected_success) in [
        ("no-delta", "no-delta", true),
        ("allowed-verifier-delta", "verifier-delta", true),
        ("allowed-reconciliation-delta", "reconciliation-delta", true),
        ("allowed-finalizer-delta", "finalizer-delta", true),
        ("allowed-smoke-delta", "smoke-delta", true),
        ("forbidden-scripts-delta", "forbidden-delta", false),
    ] {
        run_projection_case(projection, name, kind, expected_success)?;
    }
    Ok(())
}

fn run_projection_case(
    projection: &str,
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
    run_git(root, &["add", "scripts"])?;
    run_git(root, &["commit", "--quiet", "-m", "activation"])?;
    let activation = run_git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    match kind {
        "no-delta" => run_git(root, &["commit", "--quiet", "--allow-empty", "-m", "main"]).map(drop)?,
        "verifier-delta" => commit_script(root, &scripts, "verify-release-attestation-set", "activation-set\nchanged-set\n")?,
        "reconciliation-delta" => commit_script(root, &scripts, "reconcile-release-attestations", "main-reconcile\n")?,
        "finalizer-delta" => commit_script(root, &scripts, "finalize-verified-release", "main-finalizer\n")?,
        "smoke-delta" => commit_script(root, &scripts, "smoke-public-getcodexy-release.sh", "main-smoke\n")?,
        "forbidden-delta" => commit_script(root, &scripts, "unrelated-script", "forbidden\n")?,
        other => return Err(format!("unknown projection fixture: {other}").into()),
    }
    let current = run_git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    run_git(root, &["update-ref", "refs/remotes/origin/main", &current])?;
    let output = Command::new(scripts.join("project-release-verifiers.sh"))
        .current_dir(root)
        .env("GITHUB_SHA", &current)
        .env("GITHUB_REF", "refs/heads/main")
        .arg(&activation)
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
        let expected_smoke = if kind == "smoke-delta" { "main-smoke\n" } else { "activation-smoke\n" };
        assert_eq!(fs::read_to_string(scripts.join("verify-release-attestation-set"))?, expected_verifier);
        assert_eq!(fs::read_to_string(scripts.join("reconcile-release-attestations"))?, expected_reconciliation);
        assert_eq!(fs::read_to_string(scripts.join("finalize-verified-release"))?, expected_finalizer);
        assert_eq!(fs::read_to_string(scripts.join("smoke-public-getcodexy-release.sh"))?, expected_smoke);
        let smoke_metadata = fs::metadata(scripts.join("smoke-public-getcodexy-release.sh"))?;
        assert!(smoke_metadata.permissions().mode() & 0o111 != 0);
        let current_smoke = run_git(
            root,
            &["rev-parse", &format!("{current}:scripts/smoke-public-getcodexy-release.sh")],
        )?;
        assert_eq!(run_git(root, &["hash-object", "scripts/smoke-public-getcodexy-release.sh"])?, current_smoke);
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
