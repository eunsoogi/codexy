use super::{named_step, step_index, workflow};
use crate::support;

#[test]
fn staging_publication_uses_expiring_authenticated_artifacts()
-> Result<(), Box<dyn std::error::Error>> {
    let candidate = workflow("runtime-candidate.yml")?;
    let steps = candidate["jobs"]["stage-runtime"]["steps"]
        .as_sequence()
        .ok_or("staging steps")?;
    let (_, publish) = named_step(steps, "Upload authenticated staging bundle")?;
    assert_eq!(publish["uses"], "actions/upload-artifact@v7");
    assert_eq!(
        publish["with"]["name"],
        "runtime-staging-${{ github.run_id }}-${{ github.run_attempt }}"
    );
    assert_eq!(publish["with"]["retention-days"], 14);
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
