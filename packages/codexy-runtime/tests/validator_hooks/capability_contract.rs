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
    contract["concerns"][0]["inputContract"] = serde_json::json!("wrong-schema");
    std::fs::write(path, serde_json::to_vec(&contract)?)?;
    assert_rejected(&root, "missing, extra, stale, or tampered concern")
}

#[test]
fn validator_rejects_tampered_retained_capability_content_digests(
) -> Result<(), Box<dyn std::error::Error>> {
    for (path, expected) in [
        ("concerns.0.contentDigest", "missing, extra, stale, or tampered concern"),
        ("contentDigest", "content digest does not bind its exact concerns"),
    ] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let contract_path = root.join("hooks/capability-contract.json");
        let mut contract = read(&contract_path)?;
        if path == "contentDigest" {
            contract[path] = serde_json::json!("0000000000000000");
        } else {
            contract["concerns"][0]["contentDigest"] = serde_json::json!("0000000000000000");
        }
        std::fs::write(contract_path, serde_json::to_vec(&contract)?)?;
        assert_rejected(&root, expected)?;
    }
    Ok(())
}

#[test]
fn validator_rejects_missing_reordered_or_cross_platform_concern_bindings(
) -> Result<(), Box<dyn std::error::Error>> {
    for (mutation, expected) in [
        ("missing", "must be a non-empty matcher group array"),
        ("windows", "concern"),
    ] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let path = root.join("hooks/hooks.json");
        let mut hooks = read(&path)?;
        let groups = hooks["hooks"]["PreToolUse"]
            .as_array_mut()
            .ok_or("groups")?;
        match mutation {
            "missing" => {
                groups.pop();
            }
            "windows" => {
                groups[0]["hooks"][0]["commandWindows"] = serde_json::json!(
                    "\"${PLUGIN_ROOT}/hooks/codexy-destructive-command.cmd\" PreToolUse"
                );
            }
            _ => unreachable!(),
        }
        std::fs::write(path, serde_json::to_vec(&hooks)?)?;
        assert_rejected(&root, expected)?;
    }
    Ok(())
}
