use crate::support::{
    FixtureCommand as Command, bind_posix_fixture_shell_launchers, fixture_script_interpreter_path,
};
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
    let script = temp.path().join("verify-release-settings");
    fs::copy(root.join("scripts/verify-release-settings"), &script)?;
    bind_posix_fixture_shell_launchers(
        &script,
        &[("gh", "FIXTURE_GH", "FIXTURE_GH_LAUNCHER")],
    )?;
    let gh_launcher = fixture_script_interpreter_path(&gh)?;
    let run = |immutable: &str, pypi: &str, permission: &str| {
        Command::new(&script)
            .arg("--require-pypi")
            .env_path("FIXTURE_GH", &gh)
            .env_path("FIXTURE_GH_LAUNCHER", &gh_launcher)
            .env("GITHUB_REPOSITORY", "eunsoogi/codexy")
            .env("RELEASE_POLICY_TOKEN", "test-token")
            .env("FIXTURE_IMMUTABLE", immutable)
            .env("FIXTURE_PYPI", pypi)
            .env("FIXTURE_PERMISSION", permission)
            .status()
            .map(|status| status.success())
    };
    assert!(run(r#"{"enabled":true}"#, protected, "maintain")?);
    assert!(!run(r#"{"enabled":false}"#, protected, "maintain")?);
    for (name, pypi) in [
        ("protected branches", protected.replace("\"protected_branches\": true", "\"protected_branches\": false")),
        ("custom branches", protected.replace("\"custom_branch_policies\": false", "\"custom_branch_policies\": true")),
        ("admin bypass", protected.replace("\"can_admins_bypass\": false", "\"can_admins_bypass\": true")),
        ("self review", protected.replace("\"prevent_self_review\": true", "\"prevent_self_review\": false")),
        ("reviewer", protected.replace("\"reviewers\": [{\"reviewer\": {\"login\": \"maintainer\"}}]", "\"reviewers\": []")),
        ("rule types", protected.replace("{\"type\": \"branch_policy\"}", "{\"type\": \"wait_timer\"}")),
    ] {
        assert!(!run(r#"{"enabled":true}"#, &pypi, "maintain")?, "accepted weakened {name}");
    }
    assert!(!run(r#"{"enabled":true}"#, protected, "write")?);
    Ok(())
}
