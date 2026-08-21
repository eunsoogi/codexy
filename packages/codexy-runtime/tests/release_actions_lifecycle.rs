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
    assert_eq!(
        named_run(job, "Admit protected PyPI environment")?,
        "scripts/admit-pypi-environment",
    );
    let admission = named_run(job, "Validate target version and release lifecycle contract")?;
    for required in ["scripts/validate-release-lifecycle-contract"] {
        assert!(admission.contains(required), "missing publisher admission: {required}");
    }
    assert_eq!(job["steps"].as_sequence().and_then(|steps| steps.iter().find(|step| step["name"] == "Validate target version and release lifecycle contract")).and_then(|step| step["env"]["TARGET_VERSION"].as_str()), Some("${{ inputs.target_version }}"));
    let materialize = named_run(job, "Materialize and exercise activated final artifacts")?;
    assert!(materialize.contains("RELEASE_TAG=\"$RELEASE_TAG\""));
    assert!(materialize.contains("scripts/create_release_train_receipt.py"));
    let receipt = fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/create_release_train_receipt.py"))?;
    assert!(receipt.contains("codexy-runtime-release-receipt/v2"));
    assert!(receipt.contains("codexy-marketplace-bundle.tar.gz"));
    let release = named_run(job, "Create and verify the only public version release")?;
    assert_eq!(release, "scripts/publish-verified-release");
    let release_script = fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts/publish-verified-release"))?;
    for required in [
        "tag_ref=\"refs/tags/$RELEASE_TAG\"",
        "gh release view \"$RELEASE_TAG\"",
        "release_id=\"$(release_id_for_tag)\"",
        "RELEASE_ID=\"$release_id\" scripts/reconcile-release-baseline",
        "scripts/reconcile-release-baseline",
    ] {
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
fn release_publication_paths_do_not_depend_on_a_policy_pat_preflight()
-> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root();
    let bootstrap = workflow("bootstrap-package.yml")?;
    let publisher = workflow("publish-version-release.yml")?;
    let finalizer = fs::read_to_string(root.join("scripts/finalize-verified-release"))?;
    let environment_admission = fs::read_to_string(root.join("scripts/admit-pypi-environment"))?;

    for (name, source) in [
        (
            "bootstrap workflow",
            fs::read_to_string(root.join(".github/workflows/bootstrap-package.yml"))?,
        ),
        (
            "version release workflow",
            fs::read_to_string(root.join(".github/workflows/publish-version-release.yml"))?,
        ),
        ("release finalizer", finalizer.clone()),
        ("PyPI environment admission", environment_admission),
    ] {
        assert!(!source.contains("CODEXY_RELEASE_POLICY_TOKEN"), "{name} retains the policy PAT secret");
        assert!(!source.contains("RELEASE_POLICY_TOKEN"), "{name} retains the policy PAT variable");
        assert!(!source.contains("verify-release-settings"), "{name} retains the duplicate policy preflight");
    }
    assert!(!root.join("scripts/verify-release-settings").exists());

    assert_eq!(bootstrap["jobs"]["publish-bootstrap"]["environment"]["name"], "pypi");
    assert_eq!(bootstrap["jobs"]["publish-bootstrap"]["permissions"]["actions"], "read");
    assert_eq!(bootstrap["jobs"]["publish-bootstrap"]["permissions"]["id-token"], "write");
    assert_eq!(publisher["permissions"]["id-token"], "write");
    assert_eq!(publisher["permissions"]["attestations"], "write");
    assert_eq!(publisher["jobs"]["publish-release"]["environment"]["name"], "pypi");
    assert_eq!(
        named_run(
            publisher["jobs"]["publish-release"].as_mapping().ok_or("publisher job")?,
            "Finalize authenticated public release",
        )?,
        "scripts/finalize-verified-release",
    );
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
    assert!(admission.contains("git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main"));
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
fn release_workflows_use_supported_action_version_tags() -> Result<(), Box<dyn std::error::Error>> {
    for name in [
        "bootstrap-package.yml",
        "runtime-candidate.yml",
        "runtime-activation.yml",
        "plugin-runtime-binaries.yml",
        "plugin-version-bump.yml",
        "publish-version-release.yml",
        "verify-release-edit.yml",
    ] {
        let document = workflow(name)?;
        for job in document["jobs"].as_mapping().ok_or("workflow jobs")?.values() {
            for step in job["steps"].as_sequence().ok_or("workflow steps")? {
                let Some(uses) = step["uses"].as_str() else { continue };
                assert!(
                    uses.rsplit_once('@').is_some_and(|(_, tag)| !tag.is_empty() && !tag.bytes().all(|byte| byte.is_ascii_hexdigit())),
                    "workflow action must use a supported version tag: {uses}"
                );
            }
        }
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
