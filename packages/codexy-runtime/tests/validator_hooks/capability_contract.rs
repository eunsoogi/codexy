use super::{copy, read, text, validate};

fn assert_rejected(
    root: &std::path::Path,
    expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let output = validate(root)?;
    assert!(!output.status.success(), "{}", text(&output));
    assert!(text(&output).contains(expected), "{}", text(&output));
    Ok(())
}

#[test]
fn validator_accepts_the_valid_retained_capability_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let output = validate(&root)?;
    assert!(output.status.success(), "{}", text(&output));
    Ok(())
}

#[test]
fn validator_rejects_a_missing_retained_capability_contract(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    std::fs::remove_file(root.join("hooks/capability-contract.json"))?;
    assert_rejected(&root, "hooks/capability-contract.json")
}

#[test]
fn validator_rejects_a_malformed_retained_capability_contract_schema(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let path = root.join("hooks/capability-contract.json");
    let mut contract = read(&path)?;
    contract["unexpected"] = serde_json::json!(true);
    std::fs::write(path, serde_json::to_vec(&contract)?)?;
    assert_rejected(&root, "must match hook capability contract schema")
}

#[test]
fn validator_rejects_an_invalid_retained_capability_entry(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let path = root.join("hooks/capability-contract.json");
    let mut contract = read(&path)?;
    contract["capabilities"][1]["schema"] = serde_json::json!("wrong-schema");
    std::fs::write(path, serde_json::to_vec(&contract)?)?;
    assert_rejected(&root, "missing, extra, stale, or tampered capability")
}

#[test]
fn validator_rejects_tampered_retained_capability_content_digests(
) -> Result<(), Box<dyn std::error::Error>> {
    for (path, expected) in [
        ("capabilities.1.contentDigest", "missing, extra, stale, or tampered capability"),
        ("contentDigest", "content digest does not bind its exact capabilities"),
    ] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let contract_path = root.join("hooks/capability-contract.json");
        let mut contract = read(&contract_path)?;
        if path == "contentDigest" {
            contract[path] = serde_json::json!("0000000000000000");
        } else {
            contract["capabilities"][1]["contentDigest"] = serde_json::json!("0000000000000000");
        }
        std::fs::write(contract_path, serde_json::to_vec(&contract)?)?;
        assert_rejected(&root, expected)?;
    }
    Ok(())
}
