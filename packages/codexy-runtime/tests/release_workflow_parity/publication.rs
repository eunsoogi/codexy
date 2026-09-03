use std::fs;

use super::*;
#[path = "publication/public_artifact_proof.rs"] mod public_artifact_proof;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn publication_phases_are_separate_and_explicitly_gated() -> TestResult {
    let bootstrap = document("bootstrap-package.yml")?;
    let staging = document("runtime-candidate.yml")?;
    let root = codexy_runtime::paths::repository_root();
    let staging_text = fs::read_to_string(root.join(".github/workflows/runtime-candidate.yml"))?;
    let activation = document("runtime-activation.yml")?;
    let publisher = document("publish-version-release.yml")?;
    for (workflow, has_pull_request) in [
        (&bootstrap, false),
        (&staging, false),
        (&activation, false),
        (&publisher, false),
    ] {
        assert_expected_triggers(workflow, has_pull_request)?;
    }
    for removed in [
        "a02-clean-runner",
        "eunsoogi/787-official-no-node-runner",
        "a02-no-node-",
        ".github/scripts/a02-clean-runner.sh",
    ] {
        assert!(!staging_text.contains(removed), "historical A02 reference remains: {removed}");
    }
    assert!(!root.join(".github/scripts/a02-clean-runner.sh").exists());
    let bootstrap_guard = run(
        &bootstrap,
        "publish-bootstrap",
        "Reject bootstrap-first PyPI publication",
    )?;
    assert!(bootstrap_guard.contains("exit 1"));
    assert!(!bootstrap_guard.contains("pypa/gh-action-pypi-publish"));
    let staging_assembly = run(
        &staging,
        "stage-runtime",
        "Assemble canonical staged archive and receipt",
    )?;
    assert!(staging_assembly.contains("scripts/assemble-runtime-candidate"));
    let staging_assembly = script("assemble-runtime-candidate")?;
    assert!(
        staging_assembly.contains("rsync -a") && staging_assembly.contains("--exclude runtime")
    );
    let copied = lines(&staging_assembly)
        .position(|line| {
            line.contains("cp") && line.contains("staged-runtime") && line.contains("$root/runtime")
        })
        .ok_or("staging copy")?;
    let executable = lines(&staging_assembly)
        .position(|line| {
            line.contains("chmod")
                && line.contains("$root/runtime/codexy-mcp-")
                && line.contains("${server}-${platform}")
        })
        .ok_or("staging mode")?;
    assert!(copied < executable);
    let proof = step_index(
        &activation,
        "open-activation-pr",
        "Build local candidate bootstrap and prove authenticated staging identity",
    )?;
    let apply = step_index(
        &activation,
        "open-activation-pr",
        "Apply verified activation and version-selection contract",
    )?;
    let pr = step_index(
        &activation,
        "open-activation-pr",
        "Create exactly one activation pull request",
    )?;
    assert!(proof < apply && apply < pr);
    assert!(
        run(
            &activation,
            "open-activation-pr",
            "Apply verified activation and version-selection contract"
        )?
        .contains("scripts/sync-plugin-version.sh --version \"$BOOTSTRAP_VERSION\"")
    );
    let activation_proof = run(
        &activation,
        "open-activation-pr",
        "Build local candidate bootstrap and prove authenticated staging identity",
    )?;
    assert!(activation_proof.contains("scripts/download-runtime-staging-artifact staging"));
    assert!(super::command_present(
        activation_proof,
        &["gh", "attestation", "verify"]
    ));
    let activation_pr = run(
        &activation,
        "open-activation-pr",
        "Create exactly one activation pull request",
    )?;
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line
                .split_ascii_whitespace()
                .any(|word| word == "plugins/codexy-devtools")
    }));
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line
                .split_ascii_whitespace()
                .any(|word| word == ".agents/plugins")
    }));
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line.split_ascii_whitespace().any(|word| {
                word == "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json"
            })
    }));
    assert!(lines(activation_pr).any(|line| {
        line.starts_with("git add ")
            && line
                .split_ascii_whitespace()
                .any(|word| word == "packages/getcodexy/uv.lock")
    }));
    crate::support::assert_structured_literals(
        activation_pr,
        "activation pull request metadata",
        &[
            "--title \"feat(runtime): activate v${BOOTSTRAP_VERSION}\"",
            "version ${BOOTSTRAP_VERSION}",
        ],
    );
    crate::support::assert_structured_absent_literals(
        activation_pr,
        "activation pull request metadata",
        &["Fixes #"],
    );
    let release = run(
        &publisher,
        "publish-release",
        "Create and verify the only public version release",
    )?;
    assert!(release.contains("scripts/publish-verified-release"));
    Ok(())
}
#[test]
fn final_package_publication_is_bound_after_public_release_and_payload_gated() -> TestResult {
    let publisher = document("publish-version-release.yml")?;
    let verifier = document("verify-version-release.yml")?;
    let job = "publish-release";
    let finalize = step_index(&publisher, job, "Finalize authenticated public release")?;
    let package = step_index(
        &publisher,
        job,
        "Build and verify exact final getcodexy package",
    )?;
    let publish = steps(&publisher, job)?
        .iter()
        .position(|step| step["name"] == "Publish exact final getcodexy package")
        .ok_or("package publication")?;
    assert!(finalize < package && package < publish);

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
        "plugin list --json",
        "getcodexy status --json",
        "getcodexy doctor --json",
        "component_health",
    ] {
        assert!(public.contains(required), "missing public install proof: {required}");
    }
    Ok(())
}
