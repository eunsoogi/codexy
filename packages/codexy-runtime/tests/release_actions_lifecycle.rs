use std::fs;

use serde_yaml::Value;

#[test]
fn release_lifecycle_derives_every_public_identity_from_an_admitted_target_version()
-> Result<(), Box<dyn std::error::Error>> {
    let publisher = workflow("publish-version-release.yml")?;
    let inputs = publisher["on"]["workflow_dispatch"]["inputs"]
        .as_mapping()
        .ok_or("publisher inputs")?;
    assert!(inputs.contains_key("target_version"));
    assert_eq!(publisher["concurrency"]["cancel-in-progress"], false);
    assert_eq!(publisher["concurrency"]["group"], "codexy-release-${{ inputs.target_version }}");

    let job = publisher["jobs"]["publish-release"]
        .as_mapping()
        .ok_or("version-neutral publisher job")?;
    let admission = named_run(job, "Validate target version and protected release settings")?;
    for required in [
        "scripts/validate-release-lifecycle-contract",
        "scripts/verify-release-settings",
    ] {
        assert!(admission.contains(required), "missing publisher admission: {required}");
    }
    assert_eq!(job["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == "Validate target version and protected release settings")).and_then(|step| step["env"]["TARGET_VERSION"].as_str()), Some("${{ inputs.target_version }}"));
    let materialize = named_run(job, "Materialize and exercise activated final artifacts")?;
    assert!(materialize.contains("RELEASE_TAG=\"$RELEASE_TAG\""));
    assert!(materialize.contains("release: {tag: $releaseTag}"));
    let release = named_run(job, "Create and verify the only public version release")?;
    assert_eq!(release, "scripts/publish-verified-release");
    let release_script = fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"))?;
    for required in ["tag_ref=\"refs/tags/$RELEASE_TAG\"", "gh release view \"$RELEASE_TAG\"", "scripts/reconcile-release-baseline"] {
        assert!(release_script.contains(required), "missing version-neutral release operation: {required}");
    }
    let public = publisher["jobs"]["verify-public-release"]
        .as_mapping()
        .ok_or("version-neutral public verifier")?;
    let checkout = public["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["uses"].as_str().is_some()))
        .ok_or("public verifier checkout")?;
    assert_eq!(checkout["with"]["ref"], "${{ inputs.activation_commit }}");
    let download = named_run(public, "Download and verify reconciled public release without a token")?;
    for required in [
        "releases/download/$RELEASE_TAG",
        "= \"$RELEASE_TAG\"",
    ] {
        assert!(download.contains(required), "missing future-version public check: {required}");
    }
    assert!(download.contains("test \"$(git rev-parse HEAD)\" = \"$ACTIVATION_COMMIT\""));
    let smoke = named_run(public, "Smoke public release without a token")?;
    assert!(smoke.contains("getcodexy==${TARGET_VERSION}"));
    assert_eq!(public["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == "Download and verify reconciled public release without a token")).and_then(|step| step["env"]["RELEASE_TAG"].as_str()), Some("v${{ inputs.target_version }}"));

    let activation = workflow("runtime-activation.yml")?;
    assert_eq!(activation["concurrency"]["cancel-in-progress"], false);
    assert_eq!(activation["concurrency"]["group"], "codexy-runtime-activation-${{ inputs.bootstrap_version }}");
    let activation_run = named_run(
        activation["jobs"]["open-activation-pr"].as_mapping().ok_or("activation job")?,
        "Create exactly one activation pull request",
    )?;
    assert!(activation_run.contains("version ${BOOTSTRAP_VERSION}"));
    assert!(!activation_run.contains("Fixes #"));
    Ok(())
}

#[test]
fn release_edit_verifier_allows_only_body_changes_and_rechecks_actions_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    let verifier = workflow("verify-release-edit.yml")?;
    assert_eq!(verifier["on"]["release"]["types"], serde_yaml::to_value(["edited"])?);
    let run = named_run(
        verifier["jobs"]["verify-release-edit"].as_mapping().ok_or("release edit job")?,
        "Reject protected release mutation against Actions baseline",
    )?;
    for required in ["scripts/verify-release-edit-baseline"] {
        assert!(run.contains(required), "missing release edit check: {required}");
    }
    let step = verifier["jobs"]["verify-release-edit"]["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == "Reject protected release mutation against Actions baseline")).ok_or("release edit step")?;
    assert_eq!(step["env"]["GITHUB_EVENT_PATH"], "${{ github.event_path }}");
    assert_eq!(step["env"]["GH_TOKEN"], "${{ github.token }}");
    Ok(())
}

#[test]
fn bootstrap_publication_uses_minimal_build_dependencies_and_protected_pypi_environment()
-> Result<(), Box<dyn std::error::Error>> {
    let bootstrap = workflow("bootstrap-package.yml")?;
    assert_eq!(bootstrap["concurrency"]["cancel-in-progress"], false);
    assert_eq!(bootstrap["concurrency"]["group"], "codexy-bootstrap-${{ inputs.bootstrap_version }}");
    let job = bootstrap["jobs"]["publish-bootstrap"].as_mapping().ok_or("bootstrap job")?;
    assert_eq!(job["environment"]["name"], "pypi");
    let admission = named_run(job, "Admit current protected-main bootstrap source")?;
    assert!(admission.contains("scripts/verify-release-settings --require-pypi"));
    let build = named_run(job, "Build and publish bootstrap package")?;
    assert!(build.contains("python -m pip install --disable-pip-version-check build"));
    assert!(build.contains("python -m build --outdir dist packages/getcodexy"));
    for forbidden in ["--require-hashes", "release-build.txt", "--no-isolation"] {
        assert!(!build.contains(forbidden), "bootstrap build retains {forbidden}");
    }
    let publish = job["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["uses"].as_str().is_some_and(|value| value.starts_with("pypa/gh-action-pypi-publish@"))))
        .and_then(|step| step["uses"].as_str())
        .ok_or("pypi action")?;
    assert_eq!(publish, "pypa/gh-action-pypi-publish@release/v1");
    Ok(())
}

#[test]
fn release_script_fixtures_use_the_shared_cross_platform_command_dispatcher()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::runtime_package_root().join("tests");
    for path in [
        root.join("release_publication_recovery.rs"),
        root.join("release_settings_admission.rs"),
        root.join("runtime_workflow_recovery/release_reconciliation.rs"),
    ] {
        let source = fs::read_to_string(&path)?;
        assert!(
            source.contains("FixtureCommand as Command"),
            "{} must dispatch shell fixtures through FixtureCommand",
            path.display()
        );
        assert!(
            source.contains("bind_posix_fixture_shell_launchers")
                && source.contains(".env_path(\"FIXTURE_GH\"")
                && source.contains(".env_path(\"FIXTURE_GH_LAUNCHER\"")
                && !source.contains(".env(\"PATH\", format!(\"{}:{}\""),
            "{} must bind its copied shell fixture instead of relying on Windows PATH precedence",
            path.display()
        );
    }
    Ok(())
}

fn workflow(name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root().join(".github/workflows").join(name);
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}

fn named_run<'a>(
    job: &'a serde_yaml::Mapping,
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .and_then(|steps| steps.iter().find(|step| step["name"] == name))
        .and_then(|step| step["run"].as_str())
        .ok_or_else(|| format!("missing {name:?} run step").into())
}
