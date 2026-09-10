use std::fs;

use serde_yaml::Value;

use crate::support;

#[cfg(unix)]
#[path = "runtime_workflow_recovery/activation_generation_behavior.rs"]
mod activation_generation_behavior;
#[path = "runtime_workflow_recovery/activation_generation_contract.rs"]
mod activation_generation_contract;
#[cfg(unix)]
#[path = "runtime_workflow_recovery/activation_retry_behavior.rs"]
mod activation_retry_behavior;
#[path = "runtime_workflow_recovery/ci_dispatch.rs"]
mod ci_dispatch;
#[cfg(unix)]
#[path = "runtime_workflow_recovery/ci_dispatch_behavior.rs"]
mod ci_dispatch_behavior;
#[path = "runtime_workflow_recovery/durable_selection.rs"]
mod durable_selection;
#[path = "runtime_workflow_recovery/durable_selection_behavior.rs"]
mod durable_selection_behavior;
#[path = "runtime_workflow_recovery/exact_pr_head_admission.rs"]
mod exact_pr_head_admission;
#[path = "runtime_workflow_recovery/legacy_public_assembly.rs"]
mod legacy_public_assembly;
#[path = "runtime_workflow_recovery/legacy_selected_source.rs"]
mod legacy_selected_source;
#[path = "runtime_workflow_recovery/release_lineage.rs"]
mod release_lineage;
#[path = "runtime_workflow_recovery/release_reconciliation.rs"]
mod release_reconciliation;
#[path = "runtime_workflow_recovery/release_tag_admission.rs"]
mod release_tag_admission;
#[path = "runtime_workflow_recovery/windows_smoke.rs"]
mod windows_smoke;
#[path = "runtime_workflow_recovery/workflow_behavior.rs"]
mod workflow_behavior;

#[test]
fn activation_requires_clean_bootstrap_entrypoint_and_successful_staging_run()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let proof = run(
        &activation,
        "open-activation-pr",
        "Build local candidate bootstrap and prove authenticated staging identity",
    )?;
    support::assert_structured_literals(
        proof,
        "activation bootstrap and staging workflow proof",
        &[
            "git fetch --no-tags origin +refs/heads/main:refs/remotes/origin/main",
            "python -m build --outdir \"$RUNNER_TEMP/local-bootstrap-dist\" packages/getcodexy",
            "getcodexy==${BOOTSTRAP_VERSION}",
            "local-bootstrap/bin/codexy-mcp-runtime",
            "--help",
            "scripts/download-runtime-staging-artifact",
        ],
    );
    let download = script("download-runtime-staging-artifact")?;
    support::assert_structured_literals(
        &download,
        "authenticated staging downloader",
        &[
            ".status \"$run\")\" = completed",
            ".conclusion \"$run\")\" = success",
        ],
    );
    Ok(())
}

#[test]
fn staging_publication_records_a_reproducible_success_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let assembly = run(
        &candidate,
        "stage-runtime",
        "Assemble canonical staged archive and receipt",
    )?;
    assert!(assembly.contains("scripts/assemble-runtime-candidate"));
    let assembly = script("assemble-runtime-candidate")?;
    support::assert_structured_literals(
        &assembly,
        "reproducible candidate archive",
        &[
            "tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -C dist/candidate -czf dist/codexy-marketplace-plugin.tar.gz plugins/codexy",
        ],
    );
    let publish = run(
        &candidate,
        "stage-runtime",
        "Verify staged archive and receipt digests",
    )?;
    support::assert_structured_literals(
        publish,
        "staging success binding",
        &["sha256sum", "runtime-staging-receipt.json"],
    );
    Ok(())
}

#[test]
fn activation_requires_a_successful_authenticated_staging_binding()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let proof = run(
        &activation,
        "open-activation-pr",
        "Build local candidate bootstrap and prove authenticated staging identity",
    )?;
    assert!(proof.contains("scripts/download-runtime-staging-artifact"));
    let download = script("download-runtime-staging-artifact")?;
    support::assert_structured_literals(
        &download,
        "activation staging success binding",
        &[
            "runtime-staging-artifacts.json",
            "actions/artifacts/$artifact_id/zip",
            ".expired == false",
        ],
    );
    Ok(())
}

#[test]
fn activation_pr_creation_reuses_an_existing_verified_staging_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let branch = run(
        &activation,
        "open-activation-pr",
        "Prepare one version-selection branch",
    )?;
    support::assert_structured_literals(
        branch,
        "resumable activation pull request",
        &[
            "git ls-remote --exit-code --heads origin \"$branch\"",
            "\"$trusted/scripts/verify-runtime-activation-branch\" \"$branch\" \"$existing_base\" \"$BOOTSTRAP_VERSION\" \"$RUNNER_TEMP/codexy-runtime-staging/runtime-staging-receipt.json\"",
            "select-runtime-activation-branch.sh",
            "\"$BOOTSTRAP_VERSION\" \"$RUNNER_TEMP/codexy-runtime-staging/runtime-staging-receipt.json\"",
        ],
    );
    let creation = run(
        &activation,
        "open-activation-pr",
        "Create exactly one activation pull request",
    )?;
    support::assert_structured_literals(
        creation,
        "activation PR reuse",
        &[
            "\"$RUNNER_TEMP/codexy-runtime-contract/scripts/select-runtime-activation-branch.sh\" --open-pr \"$GITHUB_REPOSITORY\" \"$branch\"",
            "git rev-parse -q --verify MERGE_HEAD",
        ],
    );
    Ok(())
}

#[test]
fn activation_new_and_retry_paths_share_post_transform_tree_verification()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let job = activation["jobs"]["open-activation-pr"]
        .as_mapping()
        .ok_or("activation job")?;
    let steps = job["steps"].as_sequence().ok_or("activation steps")?;
    let prepare = step_index(steps, "Prepare one version-selection branch")?;
    let apply = step_index(
        steps,
        "Apply verified activation and version-selection contract",
    )?;
    let stage = step_index(steps, "Stage and verify activation branch")?;
    let create = step_index(steps, "Create exactly one activation pull request")?;
    assert!(prepare < apply && apply < stage && stage < create);

    let prepare_run = steps[prepare]["run"].as_str().ok_or("prepare run")?;
    assert!(prepare_run.contains("state_file=\"$RUNNER_TEMP/codexy-runtime-activation-state\""));
    assert!(prepare_run.contains("printf '%s\\n' existing > \"$state_file\""));
    assert!(prepare_run.contains("printf '%s\\n' new > \"$state_file\""));

    let stage_run = steps[stage]["run"].as_str().ok_or("stage run")?;
    assert!(stage_run.contains("git add -A -- ."));
    assert!(stage_run.contains("\"$RUNNER_TEMP/codexy-runtime-contract/scripts/verify-runtime-activation-branch\" \"$branch\" \"$GITHUB_SHA\" \"$BOOTSTRAP_VERSION\" \"$receipt\""));
    assert!(!stage_run.contains("git add .agents/plugins"));

    let create_run = steps[create]["run"].as_str().ok_or("create run")?;
    assert!(create_run.contains("state_file=\"$RUNNER_TEMP/codexy-runtime-activation-state\""));
    assert!(!create_run.contains("git add .agents/plugins"));
    assert!(create_run.contains("git diff --cached --quiet"));
    assert!(create_run.contains("git commit -m \"feat(runtime): activate v${BOOTSTRAP_VERSION}\""));
    assert!(create_run.contains("git push origin \"$branch\""));
    assert!(create_run.contains("head_sha=\"$(git rev-parse HEAD)\""));
    assert!(create_run.contains("git ls-remote --exit-code origin \"refs/heads/$branch\""));
    assert!(create_run.contains("--open-pr"));
    assert!(!create_run.contains("--allow-empty"));
    assert!(!create_run.contains("git commit --amend"));
    Ok(())
}

fn workflow(name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root()
        .join(".github/workflows")
        .join(name);
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}

fn script(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(
        codexy_runtime::paths::repository_root()
            .join("scripts")
            .join(name),
    )?)
}

fn run<'a>(value: &'a Value, job: &str, name: &str) -> Result<&'a str, Box<dyn std::error::Error>> {
    value["jobs"][job]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == name))
        .and_then(|step| step["run"].as_str())
        .ok_or_else(|| format!("missing run step {name:?}").into())
}

fn named_step<'a>(
    steps: &'a [Value],
    name: &str,
) -> Result<(usize, &'a Value), Box<dyn std::error::Error>> {
    steps
        .iter()
        .enumerate()
        .find(|(_, step)| step["name"] == name)
        .ok_or_else(|| format!("missing step {name:?}").into())
}

fn step_index(steps: &[Value], name: &str) -> Result<usize, Box<dyn std::error::Error>> {
    named_step(steps, name).map(|(index, _)| index)
}
