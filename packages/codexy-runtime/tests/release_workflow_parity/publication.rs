use std::fs;

use super::*;
#[path = "publication/prepublish_smoke.rs"]
mod prepublish_smoke;
#[path = "publication/public_artifact_proof.rs"]
mod public_artifact_proof;
#[cfg(unix)]
#[path = "publication/shallow_history.rs"]
mod shallow_history;
#[cfg(unix)]
#[path = "publication/shallow_history_fixture.rs"]
mod shallow_history_fixture;
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
        assert!(
            !staging_text.contains(removed),
            "historical A02 reference remains: {removed}"
        );
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
    let stage = step_index(
        &activation,
        "open-activation-pr",
        "Stage and verify activation branch",
    )?;
    let pr = step_index(
        &activation,
        "open-activation-pr",
        "Create exactly one activation pull request",
    )?;
    // Prove staging identity, apply its selection contract, stage it, then open one PR.
    assert!(proof < apply && apply < stage && stage < pr);
    assert!(
        run(
            &activation,
            "open-activation-pr",
            "Apply verified activation and version-selection contract"
        )?
        .contains("--apply-from")
    );
    let activation_proof = run(
        &activation,
        "open-activation-pr",
        "Build local candidate bootstrap and prove authenticated staging identity",
    )?;
    assert!(activation_proof.contains("scripts/download-runtime-staging-artifact"));
    assert!(super::command_present(
        activation_proof,
        &["gh", "attestation", "verify"]
    ));
    let activation_stage = run(
        &activation,
        "open-activation-pr",
        "Stage and verify activation branch",
    )?;
    assert!(activation_stage.contains("git add -A -- ."));
    assert!(activation_stage.contains("scripts/verify-runtime-activation-branch"));
    let activation_pr = run(
        &activation,
        "open-activation-pr",
        "Create exactly one activation pull request",
    )?;
    assert!(activation_pr.contains("git diff --cached --quiet"));
    assert!(!activation_pr.contains("git add .agents/plugins"));
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

// Keep the explicit upgrade-version contract beside the publication workflow checks.
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
    let resolve_step = &steps(&publisher, job)?[resolve];
    assert_eq!(resolve_step["id"], "resolve_previous_package_version");
    let resolve_run = resolve_step["run"].as_str().ok_or("resolve run")?;
    for required in [
        "git log --format=%H \"$ACTIVATION_COMMIT\" -- packages/getcodexy/pyproject.toml",
        "git show \"$revision:packages/getcodexy/pyproject.toml\"",
        "printf 'version=%s\\n' \"$previous_version\" >> \"$GITHUB_OUTPUT\"",
        "previous package version is unavailable",
    ] {
        assert!(
            resolve_run.contains(required),
            "missing history gate: {required}"
        );
    }
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

    let inputs = verifier["on"]["workflow_call"]["inputs"]
        .as_mapping()
        .ok_or("verifier inputs")?;
    assert_eq!(inputs["upgrade_from_version"]["required"], true);
    assert_eq!(inputs["upgrade_from_version"]["type"], "string");
    let public_smoke = run(
        &verifier,
        "verify-public-release",
        "Smoke public release without a token",
    )?;
    assert_eq!(
        steps(&verifier, "verify-public-release")?
            .iter()
            .find(|step| step["name"] == "Smoke public release without a token")
            .ok_or("public smoke step")?["env"]["UPGRADE_FROM_VERSION"],
        "${{ inputs.upgrade_from_version }}"
    );
    assert_eq!(public_smoke, "scripts/smoke-public-getcodexy-release.sh");
    let smoke_script = fs::read_to_string(
        codexy_runtime::paths::repository_root().join("scripts/smoke-public-getcodexy-release.sh"),
    )?;
    assert!(smoke_script.contains("previous_version=${UPGRADE_FROM_VERSION:-}"));
    assert!(!smoke_script.contains("git log --format=%H"));
    assert!(!smoke_script.contains("git show \"$revision:packages/getcodexy/pyproject.toml\""));
    Ok(())
}
