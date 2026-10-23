use std::fs;

use super::{document, steps};
#[path = "candidate_identity.rs"]
mod candidate_identity;

// Keep candidate receipts attributable to the exact PR head and Actions attempt being inspected.
#[test]
fn candidate_artifact_binds_source_workflow_and_attempt_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = document("runtime-candidate.yml")?;
    let inputs = &candidate["on"]["workflow_dispatch"]["inputs"];
    assert_eq!(inputs["source_commit"]["required"], true);
    assert_eq!(inputs["exact_pr_number"]["default"], "");

    let stage_steps = steps(&candidate, "stage-runtime")?;
    let assembly = stage_steps
        .iter()
        .find(|step| step["name"] == "Assemble canonical staged archive and receipt")
        .ok_or("candidate assembly step")?;
    assert_eq!(assembly["run"], "scripts/assemble-runtime-candidate");
    for (key, expected) in [
        ("SOURCE_COMMIT", "${{ inputs.source_commit }}"),
        ("STAGING_RUN_ID", "${{ github.run_id }}"),
        ("STAGING_RUN_ATTEMPT", "${{ github.run_attempt }}"),
        ("WORKFLOW_SHA", "${{ github.workflow_sha }}"),
        ("EXACT_PR_NUMBER", "${{ inputs.exact_pr_number }}"),
    ] {
        assert_eq!(assembly["env"][key], expected, "assembly identity {key}");
    }

    let upload = stage_steps
        .iter()
        .find(|step| step["name"] == "Upload isolated PR-head candidate bundle")
        .ok_or("PR-head artifact upload")?;
    assert_eq!(upload["with"]["name"], "runtime-pr-head-${{ github.run_id }}-${{ github.run_attempt }}");
    assert_eq!(upload["with"]["retention-days"], 14);
    let paths = upload["with"]["path"].as_str().ok_or("PR-head artifact paths")?;
    for path in [
        "dist/codexy-marketplace-plugin.tar.gz",
        "dist/runtime-staging-receipt.json",
        "dist/runtime-staging-provenance.json",
    ] {
        assert!(paths.contains(path), "PR-head artifact omits {path}");
    }
    let artifact_name = assembly["env"]["ARTIFACT_NAME"]
        .as_str()
        .ok_or("attempt-scoped provenance name")?;
    assert!(artifact_name.contains("runtime-pr-head-{0}-{1}"));
    assert!(artifact_name.contains("github.run_id") && artifact_name.contains("github.run_attempt"));
    Ok(())
}

// Keep installed-host candidate artifacts tied to the activated source and forbid publication rights.
#[test]
fn release_candidate_bundle_binds_inputs_and_stays_non_publishing()
-> Result<(), Box<dyn std::error::Error>> {
    let workflow = document("prepare-version-candidate.yml")?;
    let inputs = &workflow["on"]["workflow_dispatch"]["inputs"];
    for input in [
        "target_version",
        "staging_source_commit",
        "activation_commit",
        "staging_run_id",
    ] {
        assert_eq!(inputs[input]["required"], true, "required input {input}");
    }

    for permissions in [&workflow["permissions"], &workflow["jobs"]["prepare-candidate"]["permissions"]] {
        assert_eq!(permissions["actions"], "read");
        assert_eq!(permissions["contents"], "read");
        assert!(permissions.get("id-token").is_none());
        assert!(permissions.get("attestations").is_none());
    }

    let candidate_steps = steps(&workflow, "prepare-candidate")?;
    let admission = candidate_steps
        .iter()
        .find(|step| step["name"] == "Verify selected protected-main source and activation record")
        .ok_or("source admission step")?["run"]
        .as_str()
        .ok_or("source admission command")?;
    for required in [
        "refs/heads/main",
        "git merge-base --is-ancestor \"$ACTIVATION_COMMIT\" \"$DISPATCH_SHA\"",
        "git merge-base --is-ancestor \"$DISPATCH_SHA\" origin/main",
        "runtime-activation.json",
        ".candidate.artifact.stagingRunId",
    ] {
        assert!(admission.contains(required), "source admission omits {required}");
    }

    let download = candidate_steps
        .iter()
        .find(|step| step["name"] == "Download authenticated selected runtime artifact")
        .ok_or("authenticated staging download")?;
    assert_eq!(download["env"]["GH_TOKEN"], "${{ github.token }}");
    let download_run = download["run"].as_str().ok_or("download command")?;
    for required in [
        "scripts/download-runtime-staging-artifact staging",
        "runtime-staging-artifacts.json",
        "sha256sum staging.zip",
    ] {
        assert!(download_run.contains(required), "download omits {required}");
    }

    let identity = candidate_steps
        .iter()
        .position(|step| step["name"] == "Verify selected staging receipt against activation and package bytes")
        .ok_or("selected staging identity check")?;
    let materialize_index = candidate_steps
        .iter()
        .position(|step| step["name"] == "Materialize and inspect candidate marketplace archives")
        .ok_or("candidate archive step")?;
    assert!(
        identity < materialize_index,
        "receipt identity must be checked before packaging"
    );
    let identity_step = &candidate_steps[identity];
    assert_eq!(identity_step["env"]["WORKFLOW_SHA"], "${{ github.workflow_sha }}");
    let identity_run = identity_step["run"].as_str().ok_or("selected staging verifier command")?;
    let projection_commands = [
        "verifier=\"$RUNNER_TEMP/verify_selected_staging_identity.py\"",
        "git show \"$WORKFLOW_SHA:scripts/verify_selected_staging_identity.py\" > \"$verifier\"",
        "chmod 755 \"$verifier\"",
        "git hash-object \"$verifier\"",
        "git rev-parse \"$WORKFLOW_SHA:scripts/verify_selected_staging_identity.py\"",
        "\"$verifier\" staging/runtime-staging-receipt.json staging/runtime-staging-run.json staging/codexy-marketplace-plugin.tar.gz",
    ];
    let mut previous_position = 0;
    for required in projection_commands {
        let position = identity_run
            .find(required)
            .ok_or_else(|| format!("candidate verifier omits {required}"))?;
        assert!(position >= previous_position, "candidate verifier source is not checked before execution");
        previous_position = position;
    }

    let materialize = candidate_steps
        .iter()
        .find(|step| step["name"] == "Materialize and inspect candidate marketplace archives")
        .ok_or("candidate archive step")?["run"]
        .as_str()
        .ok_or("candidate archive command")?;
    for required in [
        "scripts/materialize-runtime-release-archive",
        "scripts/assemble-release-train-archive.sh",
        "scripts/inspect_release_train_archive.py",
        "scripts/create_release_train_receipt.py",
    ] {
        assert!(materialize.contains(required), "candidate archive omits {required}");
    }

    let build = candidate_steps
        .iter()
        .find(|step| step["name"] == "Build and verify getcodexy from the frozen activation commit")
        .ok_or("candidate package build")?["run"]
        .as_str()
        .ok_or("candidate package command")?;
    assert!(build.contains("python -m build --outdir dist/getcodexy-dist packages/getcodexy"));
    assert!(build.contains("scripts/verify_getcodexy_package_artifact.py"));

    let receipt = candidate_steps
        .iter()
        .find(|step| step["name"] == "Bind selected inputs and this workflow attempt to the candidate receipt")
        .ok_or("candidate provenance receipt")?["run"]
        .as_str()
        .ok_or("candidate receipt command")?;
    for required in [
        "GITHUB_RUN_ID",
        "GITHUB_RUN_ATTEMPT",
        "WORKFLOW_SHA",
        "artifactId",
        "artifactDigest",
        "bundleArtifact",
    ] {
        assert!(receipt.contains(required), "candidate receipt omits {required}");
    }

    let upload = candidate_steps
        .iter()
        .find(|step| step["uses"] == "actions/upload-artifact@v7")
        .ok_or("candidate artifact upload")?;
    assert_eq!(
        upload["with"]["name"],
        "codexy-${{ inputs.target_version }}-candidate-${{ github.run_id }}-${{ github.run_attempt }}"
    );
    let paths = upload["with"]["path"].as_str().ok_or("candidate artifact paths")?;
    for required in [
        "dist/codexy-marketplace-plugin.tar.gz",
        "dist/codexy-marketplace-bundle.tar.gz",
        "dist/codexy-runtime-package.tar.gz",
        "dist/runtime-release-receipt.json",
        "dist/codexy-marketplace-candidate-receipt.json",
        "dist/getcodexy-dist/*",
    ] {
        assert!(paths.contains(required), "candidate artifact omits {required}");
    }

    let source = fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join(".github/workflows/prepare-version-candidate.yml"),
    )?;
    for forbidden in [
        "contents: write",
        "id-token: write",
        "attestations: write",
        "environment: pypi",
        "pypa/gh-action-pypi-publish",
        "scripts/publish-verified-release",
        "gh release create",
        "git push ",
        "git tag ",
    ] {
        assert!(!source.contains(forbidden), "candidate workflow contains {forbidden}");
    }
    Ok(())
}
