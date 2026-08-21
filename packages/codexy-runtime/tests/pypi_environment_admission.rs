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

    let solo_maintainer = valid.replace(
        "\"prevent_self_review\":true",
        "\"prevent_self_review\":false",
    );
    assert!(run(&solo_maintainer, "admin")?.status.success());
    assert!(run(&solo_maintainer, "maintain")?.status.success());

    let multiple_reviewers = solo_maintainer.replace(
        "\"reviewers\":[{\"type\":\"User\",\"reviewer\":{\"login\":\"eunsoogi\"}}]",
        "\"reviewers\":[{\"type\":\"User\",\"reviewer\":{\"login\":\"eunsoogi\"}},{\"type\":\"User\",\"reviewer\":{\"login\":\"another-maintainer\"}}]",
    );
    assert!(run(&multiple_reviewers, "admin")?.status.success());

    let missing_self_review = valid.replace(
        "\"prevent_self_review\":true,",
        "",
    );
    assert!(!run(&missing_self_review, "admin")?.status.success());

    let invalid_self_review = valid.replace(
        "\"prevent_self_review\":true",
        "\"prevent_self_review\":\"true\"",
    );
    assert!(!run(&invalid_self_review, "admin")?.status.success());

    let empty_reviewers = valid.replace(
        "\"reviewers\":[{\"type\":\"User\",\"reviewer\":{\"login\":\"eunsoogi\"}}]",
        "\"reviewers\":[]",
    );
    assert!(!run(&empty_reviewers, "admin")?.status.success());

    let missing_reviewers = valid.replace(
        "\"prevent_self_review\":true,\"reviewers\":[{\"type\":\"User\",\"reviewer\":{\"login\":\"eunsoogi\"}}]",
        "\"prevent_self_review\":true",
    );
    assert!(!run(&missing_reviewers, "admin")?.status.success());

    let invalid_reviewer = valid.replace(
        "\"login\":\"eunsoogi\"",
        "\"login\":42",
    );
    assert!(!run(&invalid_reviewer, "admin")?.status.success());

    let empty_reviewer_login = valid.replace(
        "\"login\":\"eunsoogi\"",
        "\"login\":\"\"",
    );
    assert!(!run(&empty_reviewer_login, "admin")?.status.success());

    let duplicate_reviewers_rule = valid.replace(
        "{\"type\":\"branch_policy\"}",
        "{\"type\":\"required_reviewers\",\"prevent_self_review\":true,\"reviewers\":[{\"type\":\"User\",\"reviewer\":{\"login\":\"eunsoogi\"}}]},{\"type\":\"branch_policy\"}",
    );
    assert!(!run(&duplicate_reviewers_rule, "admin")?.status.success());

    let malformed_duplicate_reviewers_rule = valid.replace(
        "{\"type\":\"branch_policy\"}",
        "{\"type\":\"required_reviewers\",\"prevent_self_review\":\"true\",\"reviewers\":[{\"type\":\"User\",\"reviewer\":{\"login\":\"eunsoogi\"}}]},{\"type\":\"branch_policy\"}",
    );
    assert!(!run(&malformed_duplicate_reviewers_rule, "admin")?.status.success());

    let duplicate_branch_rule = valid.replace(
        "{\"type\":\"branch_policy\"}",
        "{\"type\":\"branch_policy\"},{\"type\":\"branch_policy\"}",
    );
    assert!(!run(&duplicate_branch_rule, "admin")?.status.success());

    let missing_branch_rule = valid.replace(
        ",\n        {\"type\":\"branch_policy\"}",
        "",
    );
    assert!(!run(&missing_branch_rule, "admin")?.status.success());

    let unexpected_rule = valid.replace(
        "{\"type\":\"branch_policy\"}",
        "{\"type\":\"branch_policy\"},{\"type\":\"deployment_branch_policy\"}",
    );
    assert!(!run(&unexpected_rule, "admin")?.status.success());

    let non_array_rules = r#"{
      "name":"pypi",
      "deployment_branch_policy":{"protected_branches":true,"custom_branch_policies":false},
      "can_admins_bypass":false,
      "protection_rules":{"type":"required_reviewers"}
    }"#;
    assert!(!run(non_array_rules, "admin")?.status.success());

    assert!(!run_with_second_permission(&multiple_reviewers, "admin", "read")?.status.success());

    let bypassed = valid.replace(
        "\"can_admins_bypass\":false",
        "\"can_admins_bypass\":true",
    );
    assert!(!run(&bypassed, "admin")?.status.success());

    let custom_branch_policy = valid.replace(
        "\"custom_branch_policies\":false",
        "\"custom_branch_policies\":true",
    );
    assert!(!run(&custom_branch_policy, "admin")?.status.success());

    let unprotected_branch = valid.replace(
        "\"protected_branches\":true",
        "\"protected_branches\":false",
    );
    assert!(!run(&unprotected_branch, "admin")?.status.success());

    assert!(!run(valid, "read")?.status.success());
    Ok(())
}

#[cfg(unix)]
fn run(environment: &str, permission: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    run_with_second_permission(environment, permission, permission)
}

#[cfg(unix)]
fn run_with_second_permission(
    environment: &str,
    permission: &str,
    second_permission: &str,
) -> Result<std::process::Output, Box<dyn std::error::Error>> {
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
  *collaborators/another-maintainer/permission*) printf '%s\n' "$SECOND_REVIEWER_PERMISSION" ;;
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
    .env("SECOND_REVIEWER_PERMISSION", second_permission)
    .output()?)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}
