use std::{fs, path::Path, process::Command};

#[cfg(unix)]
#[test]
fn pypi_environment_admission_accepts_the_protected_shape_and_rejects_drift()
-> Result<(), Box<dyn std::error::Error>> {
    let valid = r#"{
      "name":"pypi",
      "deployment_branch_policy":{"protected_branches":true,"custom_branch_policies":false},
      "can_admins_bypass":false,
      "protection_rules":[
        {"type":"required_reviewers","prevent_self_review":true,"reviewers":[{"type":"User","reviewer":{"login":"eunsoogi"}}]},
        {"type":"branch_policy"}
      ]
    }"#;
    assert!(run(valid, "admin")?.status.success());
    assert!(run(valid, "maintain")?.status.success());

    let drifted = valid.replace(
        "\"can_admins_bypass\":false",
        "\"can_admins_bypass\":true",
    );
    assert!(!run(&drifted, "admin")?.status.success());
    assert!(!run(valid, "read")?.status.success());
    Ok(())
}

#[cfg(unix)]
fn run(environment: &str, permission: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    fs::create_dir(&bin)?;
    fs::write(temp.path().join("environment.json"), environment)?;
    let gh = bin.join("gh");
    fs::write(
        &gh,
        r#"#!/bin/sh
case "$*" in
  *environments/pypi*) cat "$POLICY_FIXTURE" ;;
  *collaborators/*/permission*) printf '%s\n' "$REVIEWER_PERMISSION" ;;
  *) exit 1 ;;
esac
"#,
    )?;
    make_executable(&gh)?;
    let path = format!("{}:{}", bin.display(), std::env::var("PATH")?);
    Ok(Command::new(
        codexy_runtime::paths::repository_root().join("scripts/admit-pypi-environment"),
    )
    .env("PATH", path)
    .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
    .env("GH_TOKEN", "github.token")
    .env("POLICY_FIXTURE", temp.path().join("environment.json"))
    .env("REVIEWER_PERMISSION", permission)
    .output()?)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}
