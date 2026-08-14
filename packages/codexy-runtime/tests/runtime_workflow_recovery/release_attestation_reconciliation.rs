use std::fs;

use crate::support::{
    ReleaseFixtureCommand, bind_posix_fixture_shell_launchers, fixture_script_interpreter_path,
};

#[test]
fn attestation_reconciliation_models_paginated_slurp_and_rerun_state()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let temp = tempfile::tempdir()?;
    let scripts = temp.path().join("scripts");
    let artifacts = temp.path().join("dist");
    let bin = temp.path().join("bin");
    fs::create_dir_all(&scripts)?;
    fs::create_dir_all(&artifacts)?;
    fs::create_dir_all(&bin)?;
    let script = scripts.join("reconcile-release-attestations");
    fs::copy(root.join("scripts/reconcile-release-attestations"), &script)?;
    crate::support::make_executable(&script)?;
    let artifact = artifacts.join("release-baseline.json");
    fs::write(&artifact, b"baseline")?;
    let gh = bin.join("gh");
    fs::write(&gh, r#"#!/bin/sh
case "$*" in
  *'api --include'*attestations*)
    case "${ATTESTATION_STATE:?}" in absent) printf '%s\n' 'HTTP/2 404 Not Found' ;; *) printf '%s\n' 'HTTP/2 200 OK' ;; esac ;;
  *'api --paginate --slurp'*attestations*)
    printf '%s\n' '[{"attestations":[{}]},{"attestations":[{}]}]' ;;
  *'attestation verify'*)
    case "${ATTESTATION_STATE:?}" in mismatch) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"}]}}}]' ;; *) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"}]}}},{"verificationResult":{"statement":{"subject":[{"name":"two"}]}}}]' ;; esac ;;
  *) exit 1 ;;
esac
"#)?;
    crate::support::make_executable(&gh)?;
    bind_posix_fixture_shell_launchers(&script, &[("gh", "FIXTURE_GH", "FIXTURE_GH_LAUNCHER")])?;
    let launcher = fixture_script_interpreter_path(&gh)?;
    let environment = temp.path().join("release.env");
    let run = |state: &str| ReleaseFixtureCommand::new(&script)
        .arg_path(&artifacts).args(["ATTEST_ORIGINAL", "release-baseline.json"])
        .current_dir(temp.path()).scalar("GITHUB_REPOSITORY", "eunsoogi/codexy")
        .scalar("ACTIVATION_COMMIT", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .path("GITHUB_ENV", &environment).scalar("ATTESTATION_STATE", state)
        .path("FIXTURE_GH", &gh).path("FIXTURE_GH_LAUNCHER", &launcher).output();
    let absent = run("absent")?;
    ReleaseFixtureCommand::assert_success("reconcile-release-attestations absent", &absent);
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=true\n");
    fs::write(&environment, "")?;
    let existing = run("existing")?;
    ReleaseFixtureCommand::assert_success("reconcile-release-attestations existing", &existing);
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=false\n");
    fs::write(&environment, "")?;
    assert!(!run("mismatch")?.status.success());
    assert_eq!(fs::read_to_string(&environment)?, "");
    Ok(())
}
