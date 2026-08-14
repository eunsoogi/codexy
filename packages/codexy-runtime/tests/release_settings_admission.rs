use crate::support::FixtureCommand as Command;
use std::fs;

#[test]
fn protected_release_settings_fail_closed_for_immutable_and_pypi_policy_drift()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let bin = temp.path().join("bin");
    fs::create_dir(&bin)?;
    let gh = bin.join("gh");
    fs::write(
        &gh,
        r#"#!/bin/sh
case "$*" in
  *immutable-releases*) printf '%s\n' "$FIXTURE_IMMUTABLE" ;;
  *environments/pypi*) printf '%s\n' "$FIXTURE_PYPI" ;;
  *collaborators/*) printf '%s\n' "$FIXTURE_PERMISSION" ;;
  *) exit 1 ;;
esac
"#,
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&gh)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&gh, permissions)?;
    }
    let protected = r#"{
      "can_admins_bypass": false,
      "deployment_branch_policy": {"protected_branches": true, "custom_branch_policies": false},
      "protection_rules": [
        {"type": "branch_policy"},
        {"type": "required_reviewers", "prevent_self_review": true,
         "reviewers": [{"reviewer": {"login": "maintainer"}}]}
      ]
    }"#;
    let base_path = std::env::var("PATH")?;
    let run = |immutable: &str, pypi: &str, permission: &str| {
        Command::new(root.join("scripts/verify-release-settings"))
            .arg("--require-pypi")
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("RELEASE_POLICY_TOKEN", "test-token")
            .env("FIXTURE_IMMUTABLE", immutable)
            .env("FIXTURE_PYPI", pypi)
            .env("FIXTURE_PERMISSION", permission)
            .env("PATH", format!("{}:{base_path}", bin.display()))
            .status()
            .map(|status| status.success())
    };
    assert!(run(r#"{"enabled":true}"#, protected, "maintain")?);
    assert!(!run(r#"{"enabled":false}"#, protected, "maintain")?);
    assert!(!run(r#"{"enabled":true}"#, r#"{"protection_rules":[]}"#, "maintain")?);
    assert!(!run(r#"{"enabled":true}"#, protected, "write")?);
    Ok(())
}
