use std::fs;

use serde_yaml::Value;

use crate::support;

#[path = "runtime_workflow_recovery/release_lineage.rs"]
mod release_lineage;
#[path = "runtime_workflow_recovery/release_reconciliation.rs"]
mod release_reconciliation;
#[path = "runtime_workflow_recovery/release_tag_admission.rs"]
mod release_tag_admission;
#[path = "runtime_workflow_recovery/durable_selection.rs"]
mod durable_selection;
#[path = "runtime_workflow_recovery/durable_selection_behavior.rs"]
mod durable_selection_behavior;
#[path = "runtime_workflow_recovery/legacy_selected_source.rs"]
mod legacy_selected_source;
#[path = "runtime_workflow_recovery/legacy_public_assembly.rs"]
mod legacy_public_assembly;

#[test]
fn activation_requires_clean_bootstrap_entrypoint_and_successful_staging_run()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let proof = run(
        &activation,
        "open-activation-pr",
        "Prove public bootstrap and authenticated staging identity",
    )?;
    support::assert_structured_literals(
        proof,
        "activation bootstrap and staging workflow proof",
        &[
            "python -m venv public-bootstrap",
            "getcodexy==${BOOTSTRAP_VERSION}",
            "public-bootstrap/bin/codexy-mcp-runtime --help",
            "scripts/download-runtime-staging-artifact staging",
        ],
    );
    let download = script("download-runtime-staging-artifact")?;
    support::assert_structured_literals(
        &download,
        "authenticated staging downloader",
        &[".status \"$run\")\" = completed", ".conclusion \"$run\")\" = success"],
    );
    Ok(())
}

#[test]
fn staging_publication_uses_expiring_authenticated_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let steps = candidate["jobs"]["stage-runtime"]["steps"]
        .as_sequence()
        .ok_or("staging steps")?;
    let (_, publish) = named_step(steps, "Upload authenticated staging bundle")?;
    assert_eq!(publish["uses"], "actions/upload-artifact@v7");
    assert_eq!(publish["with"]["name"], "runtime-staging-${{ github.run_id }}-${{ github.run_attempt }}");
    assert_eq!(publish["with"]["retention-days"], 14);
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
    assert_eq!(assembly, "scripts/assemble-runtime-candidate");
    let assembly = script("assemble-runtime-candidate")?;
    support::assert_structured_literals(
        &assembly,
        "reproducible candidate archive",
        &["tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner -C dist/candidate -czf dist/codexy-marketplace-plugin.tar.gz plugins/codexy"],
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
        "Prove public bootstrap and authenticated staging identity",
    )?;
    assert!(proof.lines().any(|line| line.trim() == "scripts/download-runtime-staging-artifact staging"));
    let download = script("download-runtime-staging-artifact")?;
    support::assert_structured_literals(
        &download,
        "activation staging success binding",
        &["runtime-staging-artifacts.json", "actions/artifacts/$artifact_id/zip", ".expired == false"],
    );
    Ok(())
}

#[test]
fn activation_pr_creation_reuses_an_existing_verified_staging_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let activation = workflow("runtime-activation.yml")?;
    let branch = run(&activation, "open-activation-pr", "Prepare one version-selection branch")?;
    support::assert_structured_literals(
        branch,
        "resumable activation pull request",
        &[
            "git ls-remote --exit-code --heads origin \"$branch\"",
            "scripts/verify-runtime-activation-branch \"$branch\" origin/main \"$BOOTSTRAP_VERSION\" \"$GITHUB_WORKSPACE/staging/runtime-staging-receipt.json\"",
            "codexy/runtime-activation-v${BOOTSTRAP_VERSION}",
        ],
    );
    let creation = run(&activation, "open-activation-pr", "Create exactly one activation pull request")?;
    support::assert_structured_literals(creation, "activation PR reuse", &["gh pr list --head \"$branch\" --state open", "activation branch differs from verified contract"]);
    Ok(())
}

#[test]
fn candidate_builds_run_platform_local_lsp_and_codegraph_protocol_smokes()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let steps = candidate["jobs"]["build-runtime"]["steps"]
        .as_sequence()
        .ok_or("build-runtime steps")?;
    let smoke = named_step(steps, "Smoke platform-local MCP protocols")?;
    let package = step_index(steps, "Package declared platform binaries")?;
    assert!(smoke.0 < package, "protocol smoke must precede packaging");
    let script = smoke.1["run"].as_str().ok_or("smoke run")?;
    support::assert_structured_literals(
        script,
        "platform-local MCP protocol smokes",
        &[
            "codexy-mcp-lsp",
            "codexy-mcp-codegraph",
            "\"method\": \"initialize\"",
            "\"protocolVersion\": \"2024-11-05\"",
            "\"name\": \"lsp_status\"",
            "\"name\": \"codegraph_overview\"",
        ],
    );
    Ok(())
}

#[test]
fn candidate_keeps_windows_native_until_verified_activation()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let matrix = candidate["jobs"]["build-runtime"]["strategy"]["matrix"]["include"]
        .as_sequence()
        .ok_or("candidate build matrix")?;
    assert!(matrix.iter().any(|entry| {
        entry["platform"] == "windows-x86_64" && entry["runner"] == "windows-latest"
    }));
    let steps = candidate["jobs"]["build-runtime"]["steps"]
        .as_sequence()
        .ok_or("candidate build steps")?;
    let (_, native) = named_step(steps, "Smoke native Windows MCP protocols")?;
    assert_eq!(native["shell"], "pwsh");
    support::assert_structured_literals(
        native["run"].as_str().ok_or("native Windows smoke")?,
        "native Windows candidate proof",
        &["ProcessStartInfo", "codexy-mcp-lsp.exe", "codexy-mcp-codegraph.exe", "tools/call"],
    );
    let assembly = run(&candidate, "stage-runtime", "Assemble canonical staged archive and receipt")?;
    assert_eq!(assembly, "scripts/assemble-runtime-candidate");
    let assembly = script("assemble-runtime-candidate")?;
    support::assert_structured_literals(
        &assembly,
        "candidate-only Windows activation staging",
        &[
            "windows-x86_64",
            "extension = \"exe\" if platform == \"windows-x86_64\" else \"bin\"",
            "manifest[\"supportedPlatforms\"] = [\"darwin-arm64\", \"linux-x86_64\", \"windows-x86_64\"]",
            "codexy-mcp-devtools-windows-x86_64.exe",
            "codexy-mcp-devtools.exe",
        ],
    );

    let selected = workflow("plugin-runtime-binaries.yml")?;
    let windows = run(
        &selected,
        "verify-windows-selected-candidate",
        "Verify immutable native Windows candidate bytes",
    )?;
    support::assert_structured_literals(
        windows,
        "selected Windows runtime truth boundary",
        &[
            "legacy-public baseline intentionally has no selected Windows candidate",
            "candidate-proven",
            "tar.exe",
            "codexy-mcp-$server-windows-x86_64.exe",
        ],
    );
    Ok(())
}

fn workflow(name: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let path = codexy_runtime::paths::repository_root().join(".github/workflows").join(name);
    Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
}

fn script(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(codexy_runtime::paths::repository_root().join("scripts").join(name))?)
}

fn run<'a>(
    value: &'a Value,
    job: &str,
    name: &str,
) -> Result<&'a str, Box<dyn std::error::Error>> {
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
