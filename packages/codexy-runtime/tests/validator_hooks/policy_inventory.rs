use super::{copy, read, text, validate_all};

#[test]
fn current_packaged_inventory_matches_canonical_discovery() -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let output = validate_all(&root)?;
    assert!(output.status.success(), "{}", text(&output));
    Ok(())
}

#[test]
fn inventory_drift_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let inventory_path = root.join("hooks/policy-inventory.json");
    let mut inventory = read(&inventory_path)?;
    inventory["summary"]["uncovered"] = serde_json::json!(1);
    std::fs::write(inventory_path, serde_json::to_string_pretty(&inventory)?)?;
    let output = validate_all(&root)?;
    assert!(!output.status.success(), "{}", text(&output));
    assert!(text(&output).contains("summary must prove uncovered=0"), "{}", text(&output));
    Ok(())
}
