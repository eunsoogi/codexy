use crate::support::{
    FixtureArgumentDomain, FixtureCommand as Command, bind_posix_fixture_shell_launchers,
    fixture_github_cygpath_path, fixture_script_interpreter_path,
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
repo=eunsoogi/codexy
header='X-GitHub-Api-Version: 2026-03-10'
test "${CODEXY_FIXTURE_GH_TRANSPORT:-}" = 1 || exit 2
case "$*" in
  "api -H $header repos/$repo/immutable-releases") printf '%s\n' "$FIXTURE_IMMUTABLE" ;;
  "api -H $header repos/$repo/environments/pypi") printf '%s\n' "$FIXTURE_PYPI" ;;
  "api -H $header repos/$repo/collaborators/maintainer/permission --jq .permission") printf '%s\n' "$FIXTURE_PERMISSION" ;;
  *) exit 2 ;;
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
        &[("gh", "FIXTURE_GH", "FIXTURE_GH_LAUNCHER", FixtureArgumentDomain::GitHubApi {
            adapter_launcher_environment: "FIXTURE_GH_ADAPTER_LAUNCHER",
        })],
    )?;
    let gh_launcher = fixture_script_interpreter_path(&gh)?;
    let gh_adapter = crate::support::fixture_github_argv_adapter_path(&script);
    let gh_adapter_launcher = fixture_script_interpreter_path(&gh_adapter)?;
    let cygpath = fixture_github_cygpath_path(temp.path())?;
    let run = |repository: &str, immutable: &str, pypi: &str, permission: &str| {
        Command::new(&script)
            .arg("--require-pypi")
            .env_native_path("FIXTURE_GH", &gh)
            .env_native_path("FIXTURE_GH_LAUNCHER", &gh_launcher)
            .env_native_path("FIXTURE_GH_CYGPATH", &cygpath)
            .env_path("FIXTURE_GH_ADAPTER_LAUNCHER", &gh_adapter_launcher)
            .env("GITHUB_REPOSITORY", repository)
            .env("RELEASE_POLICY_TOKEN", "test-token")
            .env("FIXTURE_IMMUTABLE", immutable)
            .env("FIXTURE_PYPI", pypi)
            .env("FIXTURE_PERMISSION", permission)
            .status()
            .map(|status| status.success())
    };
    assert!(run("eunsoogi/codexy", r#"{"enabled":true}"#, protected, "maintain")?);
    assert!(!run("eunsoogi/codexy", r#"{"enabled":false}"#, protected, "maintain")?);
    for (name, pypi) in [
        ("protected branches", protected.replace("\"protected_branches\": true", "\"protected_branches\": false")),
        ("custom branches", protected.replace("\"custom_branch_policies\": false", "\"custom_branch_policies\": true")),
        ("admin bypass", protected.replace("\"can_admins_bypass\": false", "\"can_admins_bypass\": true")),
        ("self review", protected.replace("\"prevent_self_review\": true", "\"prevent_self_review\": false")),
        ("reviewer", protected.replace("\"reviewers\": [{\"reviewer\": {\"login\": \"maintainer\"}}]", "\"reviewers\": []")),
        ("rule types", protected.replace("{\"type\": \"branch_policy\"}", "{\"type\": \"wait_timer\"}")),
    ] {
        assert!(!run("eunsoogi/codexy", r#"{"enabled":true}"#, &pypi, "maintain")?, "accepted weakened {name}");
    }
    assert!(!run("eunsoogi/codexy", r#"{"enabled":true}"#, protected, "write")?);
    assert!(
        !run("/d/workspace/eunsoogi/codexy", r#"{"enabled":true}"#, protected, "maintain")?,
        "accepted a converted logical repository"
    );
    Ok(())
}
