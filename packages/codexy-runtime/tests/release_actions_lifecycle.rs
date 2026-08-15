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
fn release_script_fixtures_declare_their_known_windows_shell_children()
-> Result<(), Box<dyn std::error::Error>> {
    let tests = codexy_runtime::paths::runtime_package_root().join("tests");
    let materializer = fs::read_to_string(
        tests.join("release_publication_recovery/fixture_materialization.rs"),
    )?;
    let edit_baseline = fs::read_to_string(
        tests.join("runtime_workflow_recovery/release_reconciliation/edit_baseline.rs"),
    )?;
    for (fixture, invocation) in [
        (&materializer, "scripts/generate-release-changelog"),
        (&materializer, "scripts/reconcile-release-baseline"),
        (&materializer, "scripts/verify-release-attestation-total"),
        (&edit_baseline, "scripts/verify-release-attestation-total"),
    ] {
        assert!(fixture.contains("FixtureScriptBinding"));
        assert!(fixture.contains(invocation), "missing typed fixture invocation: {invocation}");
    }
    Ok(())
}

#[test]
fn release_fixture_shell_boundaries_preserve_each_process_path_authority()
-> Result<(), Box<dyn std::error::Error>> {
    let tests = codexy_runtime::paths::runtime_package_root().join("tests");
    let recovery = fs::read_to_string(tests.join("release_publication_recovery/fixture.rs"))?;
    let edit_baseline = fs::read_to_string(
        tests.join("runtime_workflow_recovery/release_reconciliation/edit_baseline.rs"),
    )?;
    let materialization = fs::read_to_string(
        tests.join("release_publication_recovery/fixture_materialization.rs"),
    )?;
    let settings = fs::read_to_string(tests.join("release_settings_admission.rs"))?;
    let attestation = fs::read_to_string(
        tests.join("runtime_workflow_recovery/release_attestation_reconciliation.rs"),
    )?;
    let bindings = fs::read_to_string(tests.join("support/fixture_command_bindings.rs"))?;
    let github_adapter = fs::read_to_string(tests.join("support/fixture_github_argv_adapter.rs"))?;
    let command = fs::read_to_string(tests.join("support/release_fixture_command.rs"))?;
    for (fixture, input) in [(&recovery, "GITHUB_ENV"), (&edit_baseline, "GITHUB_EVENT_PATH")] {
        assert!(
            fixture.contains(&format!(".path(\"{input}\"")),
            "POSIX shell input must use its projected path: {input}"
        );
    }
    for (fixture, input) in [
        (&recovery, "FIXTURE_GH"),
        (&recovery, "FIXTURE_GH_LAUNCHER"),
        (&recovery, "FIXTURE_GH_CYGPATH"),
        (&recovery, "FIXTURE_GH_STATE_ROOT"),
    ] {
        assert!(fixture.contains(&format!(".payload_path(\"{input}\"")), "native payload input must retain its host path: {input}");
    }
    for input in ["FIXTURE_DIR"] {
        assert!(edit_baseline.contains(&format!(".path(\"{input}\"")), "POSIX payload input must use its projected path: {input}");
    }
    for (fixture, input) in [
        (&edit_baseline, "FIXTURE_GH"),
        (&edit_baseline, "FIXTURE_GH_LAUNCHER"),
        (&edit_baseline, "FIXTURE_GH_CYGPATH"),
        (&attestation, "FIXTURE_GH"),
        (&attestation, "FIXTURE_GH_LAUNCHER"),
        (&attestation, "FIXTURE_GH_CYGPATH"),
    ] {
        assert!(fixture.contains(&format!(".payload_path(\"{input}\"")), "native payload input must retain its host path: {input}");
    }
    for input in ["FIXTURE_GH", "FIXTURE_GH_LAUNCHER", "FIXTURE_GH_CYGPATH"] {
        assert!(settings.contains(&format!(".env_native_path(\"{input}\"")), "settings native input must retain its host path: {input}");
    }
    for (fixture, name) in [
        (&materialization, "publisher"),
        (&edit_baseline, "edit verifier"),
        (&settings, "settings verifier"),
        (&attestation, "attestation reconciler"),
    ] {
        assert!(
            fixture.contains("FixtureArgumentDomain::GitHubApi"),
            "the {name} mock must preserve logical GitHub API arguments"
        );
        assert!(
            fixture.contains("adapter_launcher_environment"),
            "the {name} mock must declare the native GitHub argv adapter"
        );
    }
    assert!(github_adapter.contains("native_arguments"), "GitHub fixture arguments must cross a typed adapter");
    assert!(github_adapter.contains("FIXTURE_GH_CYGPATH"), "the typed adapter must receive its native filesystem converter explicitly");
    assert!(github_adapter.contains("split(b'\\0')"), "GitHub argv transport must preserve multiline fields");
    assert!(github_adapter.contains("missing typed launch transport"), "native payload and launcher must cross the typed transport");
    assert!(github_adapter.contains("posix_payload_path"), "the adapter must project only POSIX payload paths");
    assert!(github_adapter.contains("subprocess.run([launcher, payload, *arguments]"), "the adapter must launch decoded native executables as typed argv");
    assert!(!github_adapter.contains("os.execv"), "the adapter must not reparse the native launcher through execv");
    assert!(bindings.contains("printf '%s\\\\0'"), "GitHub argv transport must be per invocation");
    assert!(!bindings.contains("fixture_argv_transport"), "GitHub argv transport must not reuse a fixture file");
    assert!(!bindings.contains("MSYS2_ARG_CONV_EXCL"), "GitHub fixture arguments must not rely on environment pattern suppression");
    assert!(command.contains("payload_path"), "release fixture commands must distinguish native payload paths");
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
