use std::fs;

use super::*;

#[test]
fn final_package_is_smoked_before_public_release_and_published_afterward() -> TestResult {
    let publisher = document("publish-version-release.yml")?;
    let verifier = document("verify-version-release.yml")?;
    let job = "publish-release";
    let finalize = step_index(&publisher, job, "Finalize authenticated public release")?;
    let package = step_index(
        &publisher,
        job,
        "Build and verify exact final getcodexy package",
    )?;
    let smoke = step_index(
        &publisher,
        job,
        "Smoke exact final package before publication",
    )?;
    let release = step_index(
        &publisher,
        job,
        "Create and verify the only public version release",
    )?;
    let publish = steps(&publisher, job)?
        .iter()
        .position(|step| step["name"] == "Publish exact final getcodexy package")
        .ok_or("package publication")?;
    assert!(package < smoke && smoke < release && release < finalize && finalize < publish);

    let build = run(
        &publisher,
        job,
        "Build and verify exact final getcodexy package",
    )?;
    for required in [
        "git worktree add --detach",
        "$ACTIVATION_COMMIT",
        "scripts/validate-release-lifecycle-contract \"$TARGET_VERSION\"",
        "scripts/sync-plugin-version.sh --check",
        "scripts/verify_getcodexy_package_artifact.py",
        "python -m build",
    ] {
        assert!(build.contains(required), "missing final package gate: {required}");
    }
    let publication = &steps(&publisher, job)?[publish];
    assert_eq!(
        publication["uses"],
        "pypa/gh-action-pypi-publish@release/v1"
    );
    assert!(publication["with"]["packages-dir"].as_str().is_some());

    let prepublish_smoke = run(
        &publisher,
        job,
        "Smoke exact final package before publication",
    )?;
    assert_eq!(prepublish_smoke, "scripts/smoke-public-getcodexy-release.sh");
    let prepublish_step = &steps(&publisher, job)?[smoke];
    assert_eq!(
        prepublish_step["env"]["CODEXY_RUNTIME_PACKAGE_PATH"],
        "${{ github.workspace }}/dist/codexy-runtime-package.tar.gz"
    );
    assert_eq!(
        prepublish_step["env"]["GETCODEXY_DIST"],
        "${{ runner.temp }}/getcodexy-dist"
    );
    assert_eq!(
        prepublish_step["env"]["PUBLIC_BUNDLE_ARCHIVE"],
        "dist/codexy-marketplace-bundle.tar.gz"
    );
    assert_eq!(prepublish_step["env"]["PUBLIC_INSPECT_ROOT"], "final-inspect");

    let bootstrap = document("bootstrap-package.yml")?;
    let guard = run(
        &bootstrap,
        "publish-bootstrap",
        "Reject bootstrap-first PyPI publication",
    )?;
    assert!(guard.contains("exit 1") && guard.contains("final publisher"));
    assert!(!steps(&bootstrap, "publish-bootstrap")?
        .iter()
        .any(|step| step["uses"].as_str().is_some_and(|uses| uses.starts_with("pypa/"))));

    assert_eq!(
        publisher["jobs"]["verify-public-release"]["uses"],
        "./.github/workflows/verify-version-release.yml"
    );
    let public = run(
        &verifier,
        "verify-public-release",
        "Smoke public release without a token",
    )?;
    assert_eq!(public, "scripts/smoke-public-getcodexy-release.sh");
    let public = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/smoke-public-getcodexy-release.sh"),
    )?;
    for required in [
        "CODEX_HOME",
        "getcodexy install --json",
        "getcodexy update --json",
        "plugin list --json",
        "getcodexy status --json",
        "getcodexy doctor --json",
        "component_health",
        "GETCODEXY_DIST",
        "PUBLIC_BUNDLE_ARCHIVE",
        "PUBLIC_INSPECT_ROOT",
        "UPGRADE_FROM_VERSION",
        "FAIL_MARKETPLACE_UPGRADE",
        "installed-components.json",
    ] {
        assert!(public.contains(required), "missing public install proof: {required}");
    }
    let fake_host = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/fake_public_codex_host.py"),
    )?;
    for required in [
        r#""marketplaceSource": {"#,
        r#""sourceType": "git""#,
        r#""source": "https://github.com/eunsoogi/codexy.git""#,
        r#"state["versions"]"#,
        r#"unexpected unpinned marketplace upgrade"#,
    ] {
        assert!(
            fake_host.contains(required),
            "fake public host omits marketplace identity: {required}"
        );
    }
    Ok(())
}
