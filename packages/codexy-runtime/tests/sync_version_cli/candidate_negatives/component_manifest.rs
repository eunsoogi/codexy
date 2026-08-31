use std::fs;

use serde_json::{Value, json};

use super::{
    TestResult, fixture_version, mutate_json, next_patch_version, prepare_candidate, reject,
    selected_fixture, shared_repository_archive,
};

const COMPONENT_MANIFEST: &str =
    "packages/getcodexy/src/codexy_runtime_tools/component-manifest.json";

#[test]
fn candidate_preparation_preserves_the_packaged_component_manifest() -> TestResult {
    let temp = tempfile::tempdir()?;
    let root = selected_fixture(shared_repository_archive()?, &temp, "component-manifest")?;
    prepare_candidate(&root)?;

    let manifest: Value = serde_json::from_str(&fs::read_to_string(root.join(COMPONENT_MANIFEST))?)?;
    let selected_version = fixture_version(&root)?;
    let candidate_version = next_patch_version(&selected_version)?;
    for field in ["components", "compatibleCombinations"] {
        for entry in manifest[field].as_array().ok_or("component manifest array")? {
            assert_eq!(
                entry["version"],
                selected_version,
                "candidate {field} changed selected identity"
            );
        }
    }
    let contract: Value = serde_json::from_str(&fs::read_to_string(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    assert_eq!(contract["bootstrap"]["selectedVersion"], selected_version);
    assert_eq!(contract["bootstrap"]["candidateVersion"], candidate_version);
    Ok(())
}

#[test]
fn candidate_check_rejects_each_component_manifest_drift() -> TestResult {
    let temp = tempfile::tempdir()?;
    for field in ["components", "compatibleCombinations"] {
        let root = selected_fixture(shared_repository_archive()?, &temp, field)?;
        prepare_candidate(&root)?;
        let candidate_version = next_patch_version(&fixture_version(&root)?)?;
        mutate_json(&root.join(COMPONENT_MANIFEST), |value| {
            value[field][0]["version"] = json!(candidate_version);
        })?;
        reject(&root, &["--check-candidate"], &format!("{field}-component-drift"))?;
    }
    Ok(())
}
