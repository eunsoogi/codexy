use std::fs;

use crate::support::{
    FixtureArgumentDomain, ReleaseFixtureCommand, ReleaseFixtureOutcome,
    bind_posix_fixture_shell_launchers, fixture_script_interpreter_path,
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
repo=eunsoogi/codexy
header='X-GitHub-Api-Version: 2026-03-10'
route="repos/$repo/attestations/sha256:8ba8496a2525ae171ffd104d632dede6ef418d9b95962a9d88e2fcdbc8d48d24?per_page=100"
case "$1" in
  api)
    case "$2" in
      --include)
        test "$3" = -H && test "$4" = "$header" && test "$5" = "$route" || exit 2
        case "${ATTESTATION_STATE:?}" in absent) printf '%s\n' 'HTTP/2 404 Not Found' ;; *) printf '%s\n' 'HTTP/2 200 OK' ;; esac ;;
      --paginate)
        test "$3" = --slurp && test "$4" = -H && test "$5" = "$header" && test "$6" = "$route" || exit 2
        printf '%s\n' '[{"attestations":[{}]},{"attestations":[{}]}]' ;;
      *) exit 2 ;;
    esac ;;
  attestation)
    test "$2" = verify && test "$4" = --repo && test "$5" = "$repo" || exit 2
    case "${ATTESTATION_STATE:?}" in mismatch) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"}]}}}]' ;; *) printf '%s\n' '[{"verificationResult":{"statement":{"subject":[{"name":"one"}]}}},{"verificationResult":{"statement":{"subject":[{"name":"two"}]}}}]' ;; esac ;;
  *) exit 2 ;;
esac
"#)?;
    crate::support::make_executable(&gh)?;
    bind_posix_fixture_shell_launchers(&script, &[("gh", "FIXTURE_GH", "FIXTURE_GH_LAUNCHER", FixtureArgumentDomain::GitHubApi)])?;
    let launcher = fixture_script_interpreter_path(&gh)?;
    let environment = temp.path().join("release.env");
    let run = |repository: &str, state: &str| ReleaseFixtureCommand::new(&script)
        .arg_path(&artifacts).args(["ATTEST_ORIGINAL", "release-baseline.json"])
        .current_dir(temp.path()).scalar("GITHUB_REPOSITORY", repository)
        .scalar("ACTIVATION_COMMIT", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .path("GITHUB_ENV", &environment).scalar("ATTESTATION_STATE", state)
        .path("FIXTURE_GH", &gh).path("FIXTURE_GH_LAUNCHER", &launcher).output();
    let absent = run("eunsoogi/codexy", "absent")?;
    ReleaseFixtureCommand::assert_outcome(
        "reconcile-release-attestations absent",
        ReleaseFixtureOutcome::Success,
        &absent,
    );
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=true\n");
    fs::write(&environment, "")?;
    let existing = run("eunsoogi/codexy", "existing")?;
    ReleaseFixtureCommand::assert_outcome(
        "reconcile-release-attestations existing",
        ReleaseFixtureOutcome::Success,
        &existing,
    );
    assert_eq!(fs::read_to_string(&environment)?, "ATTEST_ORIGINAL=false\n");
    fs::write(&environment, "")?;
    let mismatch = run("eunsoogi/codexy", "mismatch")?;
    ReleaseFixtureCommand::assert_outcome(
        "reconcile-release-attestations mismatch",
        ReleaseFixtureOutcome::Failure,
        &mismatch,
    );
    assert_eq!(fs::read_to_string(&environment)?, "");
    let converted = run("/d/workspace/eunsoogi/codexy", "existing")?;
    ReleaseFixtureCommand::assert_outcome(
        "reconcile-release-attestations converted repository",
        ReleaseFixtureOutcome::Failure,
        &converted,
    );
    Ok(())
}
