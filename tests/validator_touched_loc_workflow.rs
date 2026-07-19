#[test]
fn touched_loc_workflow_runs_for_all_pull_requests() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflow = std::fs::read_to_string(root.join(".github/workflows/touched-loc-gate.yml"))?;

    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("pull_request:").is_some());
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(
        workflow.find("paths:").is_none(),
        "touched LOC gate must not use a narrow paths filter"
    );
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("fetch-depth: 0").is_some());
    // structured-contract: non-contract substring rationale: verifies generated GitHub Actions source text
    assert!(workflow.find("--check-touched-loc").is_some());
    Ok(())
}
