use super::{copy, read, text, validate};
use super::structured_contract_artifacts::TextShape;

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
fn inventory_uses_a_version_independent_capability_contract() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let inventory = read(&root.join("hooks/policy-inventory.json"))?;

    assert!(
        inventory.get("version").is_none(),
        "inventory must not store a Codex version field"
    );
    assert!(
        root.join("hooks/capability-contract.json").is_file(),
        "inventory requires a checked-in capability contract"
    );

    let output = validate(&root)?;
    assert!(output.status.success(), "{}", text(&output));
    Ok(())
}

#[test]
fn inventory_rejects_legacy_version_storage() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let path = root.join("hooks/policy-inventory.json");
    let mut inventory = read(&path)?;
    inventory["version"] = serde_json::json!("ambient-cli-version");
    std::fs::write(path, serde_json::to_vec(&inventory)?)?;

    assert_rejected(&root, "must match policy inventory schema")
}

#[test]
fn capability_contract_rejects_missing_extra_schema_and_tampered_entries(
) -> Result<(), Box<dyn std::error::Error>> {
    for mutation in ["missing", "extra", "schema", "tampered"] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let path = root.join("hooks/capability-contract.json");
        let mut contract = read(&path)?;
        match mutation {
            "missing" => {
                contract["capabilities"].as_array_mut().ok_or("capabilities")?.pop();
            }
            "extra" => {
                let extra = contract["capabilities"][0].clone();
                contract["capabilities"]
                    .as_array_mut()
                    .ok_or("capabilities")?
                    .push(extra);
            }
            "schema" => contract["capabilities"][1]["schema"] = serde_json::json!("wrong-schema"),
            "tampered" => contract["capabilities"][1]["contentDigest"] = serde_json::json!("0000000000000000"),
            _ => unreachable!(),
        }
        std::fs::write(path, serde_json::to_vec(&contract)?)?;
        assert_rejected(&root, "capability")?;
    }
    Ok(())
}

#[test]
fn capability_contract_rejects_a_tampered_aggregate_digest(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let root = copy(temp.path())?;
    let path = root.join("hooks/capability-contract.json");
    let mut contract = read(&path)?;
    contract["contentDigest"] = serde_json::json!("0000000000000000");
    std::fs::write(path, serde_json::to_vec(&contract)?)?;

    assert_rejected(&root, "content digest")
}

#[test]
fn inventory_generator_and_validator_reject_version_or_ambient_discovery(
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = codexy_runtime::paths::repository_root();
    let generator = std::fs::read_to_string(repository.join("scripts/generate-hook-policy-inventory"))?;
    let validator = std::fs::read_to_string(
        codexy_runtime::paths::runtime_package_root().join("src/validation/hooks/policy_inventory.rs"),
    )?;

    TextShape::new(&generator).assert_absent_concepts(
        "hook.inventory.generator.no-version-or-ambient-discovery",
        &[
            "subprocess",
            "os.environ",
            "shutil.which",
            "codex --version",
            "supportedCodexBuild",
            "version",
        ],
    );
    TextShape::new(&validator).assert_absent_concepts(
        "hook.inventory.validator.no-version-state",
        &["version"],
    );
    Ok(())
}

#[test]
fn suite_registry_requires_the_actual_admission_runtime_suite(
) -> Result<(), Box<dyn std::error::Error>> {
    for mapping in ["missing", "../tests/suites/all.rs", "tests/validator_hooks.rs", "tests/suites/archive.rs"] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let path = root.join("hooks/policy-inventory.json");
        let mut inventory = read(&path)?;
        if mapping == "missing" {
            inventory["testSuites"].as_object_mut().ok_or("test suites")?.remove("admission");
        } else {
            inventory["testSuites"]["admission"] = serde_json::json!(mapping);
        }
        std::fs::write(path, serde_json::to_vec(&inventory)?)?;
        assert_rejected(&root, "actual admission runtime suite")?;
    }
    Ok(())
}

#[test]
fn suite_registry_rejects_missing_or_non_regular_runtime_targets(
) -> Result<(), Box<dyn std::error::Error>> {
    for target_kind in ["missing", "directory"] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let target = temp.path().join("packages/codexy-runtime/tests/suites/all.rs");
        std::fs::remove_file(&target)?;
        if target_kind == "directory" {
            std::fs::create_dir(&target)?;
        }
        assert_rejected(&root, "actual admission runtime suite")?;
    }
    #[cfg(unix)]
    {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let target = temp.path().join("packages/codexy-runtime/tests/suites/all.rs");
        std::fs::remove_file(&target)?;
        let alternate = temp.path().join("alternate-suite.rs");
        std::fs::write(&alternate, "// alternate suite\n")?;
        std::os::unix::fs::symlink(alternate, target)?;
        assert_rejected(&root, "actual admission runtime suite")?;
    }
    Ok(())
}

#[test]
fn enforcement_rejects_nonpreventive_events_and_unknown_inputs(
) -> Result<(), Box<dyn std::error::Error>> {
    for case in ["session-end", "unknown-input"] {
        let temp = tempfile::tempdir()?;
        let root = copy(temp.path())?;
        let path = root.join("hooks/policy-inventory.json");
        let mut inventory = read(&path)?;
        let rules = inventory["rules"].as_array_mut().ok_or("rules")?;
        let rule = if case == "session-end" {
            rules
                .iter_mut()
                .find(|rule| rule["decision"] == "reviewed-exception")
                .ok_or("reviewed exception")?
        } else {
            rules
                .iter_mut()
                .find(|rule| rule["decision"] == "enforced")
                .ok_or("enforced rule")?
        };
        rule["decision"] = serde_json::json!("enforced");
        rule["event"] = serde_json::json!("PreToolUse");
        rule["input"] = serde_json::json!("unsupported-input");
        if case == "session-end" {
            rule["event"] = serde_json::json!("SessionEnd");
            rule["input"] = serde_json::json!("session-end");
            let object = rule.as_object_mut().ok_or("rule")?;
            object.remove("unavailableEvent");
            object.remove("unavailableInput");
            object.remove("rationale");
        }
        std::fs::write(path, serde_json::to_vec(&inventory)?)?;
        assert_rejected(&root, "overclaims preventive enforcement")?;
    }
    Ok(())
}

#[test]
fn session_end_is_audited_nonpreventive_without_a_handler(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = codexy_runtime::paths::repository_root().join("plugins/codexy");
    let contract = read(&root.join("hooks/capability-contract.json"))?;
    let session_end = contract["capabilities"]
        .as_array()
        .ok_or("capabilities")?
        .iter()
        .find(|capability| capability["event"] == "SessionEnd")
        .ok_or("SessionEnd")?;
    assert_eq!(session_end["preventive"], serde_json::json!(false));
    let hooks = read(&root.join("hooks/hooks.json"))?;
    assert!(hooks["hooks"].get("SessionEnd").is_none());
    Ok(())
}
