use std::{fs, path::PathBuf, process::Command};

use super::shallow_history_fixture::{
    create_fixture as create_fixture_with_stub, index_request_count, run_smoke, smoke_events,
};
use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PRIOR_VERSION: &str = "1.6.2";

fn create_fixture(root: PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    create_fixture_with_stub(root, fake_getcodexy())
}

// Persist one selected component state so the public install and upgrade probes can verify health.
fn fake_getcodexy() -> &'static str {
    r##"#!/bin/sh
set -eu

if test "${0##*/}" = "codexy-github-install"; then
  mkdir -p "$CODEX_HOME/agents/codexy-github"
  printf '%s\n' 'name = "codexy-weaver"' >"$CODEX_HOME/agents/codexy-github/codexy-weaver.toml"
  exit 0
fi

case "${1:-}" in
install | update)
  mkdir -p "$CODEX_HOME"
  jq -n --arg version "$TARGET_VERSION" '{selection:["core","devtools","github"],versions:{core:$version,devtools:$version,github:$version}}' >"$CODEX_HOME/.codexy-public-proof.json"
  touch "$CODEX_HOME/.codexy-public-marketplace-present"
  jq -cn --arg command "$1" '{schema:"getcodexy.operation-receipt.v1",outcome:"completed",errors:[],selection_after:["core","devtools","github"]} + if $command == "update" then {command:$command} else {} end'
  ;;
status)
  printf '%s\n' '{"schema":"getcodexy.status.v1","outcome":"completed","inventory_consistency":"consistent","errors":[],"installed_components":["core","devtools","github"]}'
  ;;
doctor)
  test -f "$CODEX_HOME/agents/codexy-github/codexy-weaver.toml"
  jq -cn --arg version "$TARGET_VERSION" '{schema:"getcodexy.doctor.v1",outcome:"completed",inventory_consistency:"consistent",host_readiness:{state:"ready"},errors:[],component_health:[range(0;3)|{healthy:true,state:"healthy",observed:{plugin:{version:$version},runtime:{version:$version}}}]}'
  ;;
*)
  echo "unexpected getcodexy command" >&2
  exit 1
  ;;
esac
"##
}

#[test]
fn public_smoke_receives_prior_version_in_a_one_commit_checkout() -> TestResult {
    let temp = tempfile::tempdir()?;
    let missing_input = create_fixture(temp.path().join("missing-input"))?;
    let history = Command::new("git")
        .current_dir(&missing_input)
        .args([
            "log",
            "--format=%H",
            "--",
            "packages/getcodexy/pyproject.toml",
        ])
        .output()?;
    assert!(history.status.success());
    assert_eq!(String::from_utf8(history.stdout)?.lines().count(), 1);

    let failed = run_smoke(&missing_input, None, "ready", false, false)?;
    assert!(!failed.status.success());
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("previous package version is unavailable")
    );

    let explicit_input = create_fixture(temp.path().join("explicit-input"))?;
    let passed = run_smoke(&explicit_input, Some(PRIOR_VERSION), "ready", false, false)?;
    assert!(
        passed.status.success(),
        "one-commit public smoke failed: {}",
        String::from_utf8_lossy(&passed.stderr)
    );
    assert!(
        fs::read_to_string(explicit_input.join("public-upgrade.json"))?
            .contains("\"command\":\"update\"")
    );
    Ok(())
}

#[test]
fn public_smoke_waits_for_both_exact_simple_index_files() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = create_fixture(temp.path().join("delayed-index"))?;
    let passed = run_smoke(&root, Some(PRIOR_VERSION), "delayed", false, false)?;
    assert!(
        passed.status.success(),
        "delayed public smoke failed: {}",
        String::from_utf8_lossy(&passed.stderr)
    );

    // Two stale but valid index snapshots must be retried before installing the exact package.
    assert_eq!(index_request_count(&root)?, 3);
    let events = smoke_events(&root)?;
    let final_index_read = events
        .iter()
        .position(|event| event == "simple-index:3")
        .ok_or("third Simple Index read")?;
    let package_install = events
        .iter()
        .position(|event| event == "public-package-install")
        .ok_or("public package install")?;
    assert!(final_index_read < package_install);
    assert!(root.join("public-install.json").is_file());
    assert!(root.join("public-upgrade.json").is_file());

    let mismatch_root = create_fixture(temp.path().join("digest-mismatch"))?;
    let mismatch = run_smoke(
        &mismatch_root,
        Some(PRIOR_VERSION),
        "digest-mismatch",
        false,
        false,
    )?;
    assert!(!mismatch.status.success());
    assert!(String::from_utf8_lossy(&mismatch.stderr).contains("Simple Index SHA-256 differs"));
    Ok(())
}

#[test]
fn public_smoke_fails_closed_on_absent_or_malformed_index_metadata() -> TestResult {
    let temp = tempfile::tempdir()?;
    for (mode, failure, expected_checks) in [
        (
            "missing",
            "Simple Index did not expose both exact distributions after 18 checks",
            18,
        ),
        (
            "malformed",
            "PyPI Simple Index returned malformed metadata",
            1,
        ),
    ] {
        let root = create_fixture(temp.path().join(mode))?;
        let failed = run_smoke(&root, Some(PRIOR_VERSION), mode, false, false)?;
        assert!(!failed.status.success());
        assert!(String::from_utf8_lossy(&failed.stderr).contains(failure));
        assert_eq!(index_request_count(&root)?, expected_checks);
        let events = smoke_events(&root)?;
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "sleep:10")
                .count(),
            expected_checks - 1
        );
        assert!(!events.iter().any(|event| event == "public-package-install"));
        assert!(!root.join("public-install.json").exists());
    }
    Ok(())
}

#[test]
fn public_smoke_preserves_unrelated_pip_install_failures() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = create_fixture(temp.path().join("pip-failure"))?;
    let failed = run_smoke(&root, Some(PRIOR_VERSION), "ready", true, false)?;
    assert_eq!(failed.status.code(), Some(23));
    assert!(String::from_utf8_lossy(&failed.stderr).contains("injected unrelated pip failure"));
    let events = smoke_events(&root)?;
    let index_read = events
        .iter()
        .position(|event| event == "simple-index:1")
        .ok_or("Simple Index read")?;
    let package_install = events
        .iter()
        .position(|event| event == "public-package-install")
        .ok_or("public package install")?;
    assert!(index_read < package_install);
    assert!(!root.join("public-install.json").exists());
    Ok(())
}

#[test]
fn prepublication_local_distribution_skips_public_index_wait() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = create_fixture(temp.path().join("local-distribution"))?;
    let passed = run_smoke(&root, Some(PRIOR_VERSION), "malformed", false, true)?;
    assert!(
        passed.status.success(),
        "local-distribution smoke failed: {}",
        String::from_utf8_lossy(&passed.stderr)
    );
    let events = smoke_events(&root)?;
    assert!(
        !events
            .iter()
            .any(|event| event.starts_with("simple-index:"))
    );
    assert!(events.iter().any(|event| event == "local-package-install"));
    Ok(())
}

#[test]
fn release_workflows_resolve_and_forward_the_explicit_upgrade_version() -> TestResult {
    let publisher = document("publish-version-release.yml")?;
    let verifier = document("verify-version-release.yml")?;
    let job = "publish-release";
    let resolve = step_index(
        &publisher,
        job,
        "Resolve previous package version from full history",
    )?;
    let smoke = step_index(
        &publisher,
        job,
        "Smoke exact final package before publication",
    )?;
    assert!(resolve < smoke);
    assert_eq!(
        steps(&publisher, job)?[smoke]["env"]["UPGRADE_FROM_VERSION"],
        "${{ steps.resolve_previous_package_version.outputs.version }}"
    );
    assert_eq!(
        publisher["jobs"]["publish-release"]["outputs"]["upgrade_from_version"],
        "${{ steps.resolve_previous_package_version.outputs.version }}"
    );
    assert_eq!(
        publisher["jobs"]["verify-public-release"]["with"]["upgrade_from_version"],
        "${{ needs.publish-release.outputs.upgrade_from_version }}"
    );
    assert_eq!(
        steps(&verifier, "verify-public-release")?
            .iter()
            .find(|step| step["name"] == "Smoke public release without a token")
            .ok_or("public smoke step")?["env"]["UPGRADE_FROM_VERSION"],
        "${{ inputs.upgrade_from_version }}"
    );
    let smoke_script = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/smoke-public-getcodexy-release.sh"),
    )?;
    assert!(smoke_script.contains("previous_version=${UPGRADE_FROM_VERSION:-}"));
    assert_eq!(
        run(
            &verifier,
            "verify-public-release",
            "Smoke public release without a token"
        )?,
        "scripts/smoke-public-getcodexy-release.sh"
    );
    Ok(())
}
