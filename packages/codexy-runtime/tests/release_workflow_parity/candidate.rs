use super::{document, steps};

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
