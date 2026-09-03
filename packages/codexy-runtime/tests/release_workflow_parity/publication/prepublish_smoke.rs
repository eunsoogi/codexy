use std::fs;

use sha2::{Digest as _, Sha256};
use super::*;

const RAW_DIGEST_ASSIGNMENT: &str =
    r#"runtime_package_sha256="$(sha256sum dist/codexy-runtime-package.tar.gz | awk '{print $1}')""#;
const RAW_DIGEST_RECEIPT_CHECK: &str =
    r#"test "$runtime_package_sha256" = "$(jq -er .artifact.sha256 staging/runtime-staging-receipt.json)""#;
const RAW_DIGEST_OUTPUT: &str =
    r#"printf 'runtime_package_sha256=%s\n' "$runtime_package_sha256" >> "$GITHUB_OUTPUT""#;

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
    let materialize = step_index(
        &publisher,
        job,
        "Materialize and exercise activated final artifacts",
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
    assert!(
        materialize < smoke
            && package < smoke
            && smoke < release
            && release < finalize
            && finalize < publish
    );
    let materialize_step = &steps(&publisher, job)?[materialize];
    assert_eq!(materialize_step["id"], "materialize_final_artifacts");
    let materialize_run = materialize_step["run"].as_str().ok_or("materialization run")?;
    for required in [
        "cp staging/codexy-marketplace-plugin.tar.gz dist/codexy-runtime-package.tar.gz",
        RAW_DIGEST_ASSIGNMENT,
        RAW_DIGEST_RECEIPT_CHECK,
        RAW_DIGEST_OUTPUT,
    ] {
        assert!(
            materialize_run.contains(required),
            "missing raw runtime digest proof: {required}"
        );
    }

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
        prepublish_step["env"]["CODEXY_RUNTIME_PACKAGE_SHA256"],
        "${{ steps.materialize_final_artifacts.outputs.runtime_package_sha256 }}"
    );
    assert!(!materialize_run.contains("hashFiles"));
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

#[test]
fn raw_digest_output_matches_materialized_archive_bytes() -> TestResult {
    let publisher = document("publish-version-release.yml")?;
    let materialize = run(
        &publisher,
        "publish-release",
        "Materialize and exercise activated final artifacts",
    )?;
    for required in [
        RAW_DIGEST_ASSIGNMENT,
        RAW_DIGEST_RECEIPT_CHECK,
        RAW_DIGEST_OUTPUT,
    ] {
        assert!(materialize.contains(required));
    }

    let archive = b"materialized runtime archive bytes\n";
    let raw_digest = format!("{:x}", Sha256::digest(archive));
    assert_eq!(raw_digest.len(), 64);
    assert_eq!(raw_digest, raw_digest.to_ascii_lowercase());

    let aggregate_digest = format!("{:x}", Sha256::digest(raw_digest.as_bytes()));
    assert_ne!(
        raw_digest, aggregate_digest,
        "a digest of a per-file digest is not the archive-byte digest"
    );

    let root = tempfile::tempdir()?;
    let staging = root.path().join("staging");
    let dist = root.path().join("dist");
    fs::create_dir_all(&staging)?;
    fs::create_dir_all(&dist)?;
    fs::write(staging.join("codexy-marketplace-plugin.tar.gz"), archive)?;
    fs::write(
        staging.join("runtime-staging-receipt.json"),
        format!(r#"{{"artifact":{{"sha256":"{raw_digest}"}}}}"#),
    )?;
    let github_output = root.path().join("github-output");

    let materialization_lines = [
        "cp staging/codexy-marketplace-plugin.tar.gz dist/codexy-runtime-package.tar.gz",
        RAW_DIGEST_ASSIGNMENT,
        RAW_DIGEST_RECEIPT_CHECK,
        RAW_DIGEST_OUTPUT,
    ];
    let script = format!(
        "set -eu\nreceipt=staging/runtime-staging-receipt.json\n{}\n",
        materialization_lines.join("\n")
    );
    let result = std::process::Command::new("sh")
        .args(["-c", script.as_str()])
        .current_dir(root.path())
        .env("GITHUB_OUTPUT", &github_output)
        .output()?;
    assert!(
        result.status.success(),
        "materialization digest commands failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read_to_string(&github_output)?,
        format!("runtime_package_sha256={raw_digest}\n")
    );
    assert_eq!(
        fs::read(root.path().join("dist/codexy-runtime-package.tar.gz"))?,
        archive
    );
    Ok(())
}
