use std::path::Path;

use serde_json::Value;

use crate::support::{TestResult, copy_plugin_fixture};

#[test]
fn production_validator_rejects_entrypoint_heading_mutations() -> TestResult {
    for mutation in [
        HeadingMutation::Removed,
        HeadingMutation::Substituted,
        HeadingMutation::Duplicated,
        HeadingMutation::FragmentSubstituted,
    ] {
        let (_temporary, plugin_root) = copy_plugin_fixture()?;
        mutate_heading(&plugin_root, mutation)?;
        assert_rejected(&plugin_root, &format!("heading {mutation:?}"));
    }
    Ok(())
}

#[test]
fn production_validator_rejects_noncanonical_identity_file_mutations() -> TestResult {
    for mutation in [
        IdentityMutation::ParentEscape,
        IdentityMutation::Absolute,
        IdentityMutation::SiblingPath,
        IdentityMutation::SourceFileSubstitution,
    ] {
        let (_temporary, plugin_root) = copy_plugin_fixture()?;
        mutate_identity_file(&plugin_root, mutation)?;
        assert_rejected(&plugin_root, &format!("identity path {mutation:?}"));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
enum HeadingMutation {
    Removed,
    Substituted,
    Duplicated,
    FragmentSubstituted,
}

#[derive(Clone, Copy, Debug)]
enum IdentityMutation {
    ParentEscape,
    Absolute,
    SiblingPath,
    SourceFileSubstitution,
}

fn mutate_heading(plugin_root: &Path, mutation: HeadingMutation) -> TestResult {
    let skill_path = plugin_root.join("skills/engineering/SKILL.md");
    match mutation {
        HeadingMutation::Removed => {
            let skill = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, skill.replacen("## Diagnosis\n", "", 1))?;
        }
        HeadingMutation::Substituted => {
            let skill = std::fs::read_to_string(&skill_path)?;
            std::fs::write(&skill_path, skill.replacen("## Diagnosis", "## Investigate", 1))?;
        }
        HeadingMutation::Duplicated => {
            let mut skill = std::fs::read_to_string(&skill_path)?;
            skill.push_str("\n## Diagnosis\n\nDuplicate heading.\n");
            std::fs::write(&skill_path, skill)?;
        }
        HeadingMutation::FragmentSubstituted => mutate_manifest(plugin_root, |mapping| {
            mapping["entrypoint"] = Value::String("SKILL.md#specification".to_owned());
        })?,
    }
    Ok(())
}

fn mutate_identity_file(plugin_root: &Path, mutation: IdentityMutation) -> TestResult {
    let references = plugin_root.join("skills/engineering/references");
    let value = match mutation {
        IdentityMutation::ParentEscape => "../references/legacy-rule-mappings/debugging.json".to_owned(),
        IdentityMutation::Absolute => references
            .join("legacy-rule-mappings/debugging.json")
            .display()
            .to_string(),
        IdentityMutation::SiblingPath => "legacy-rule-mappings/../legacy-rule-mappings/debugging.json".to_owned(),
        IdentityMutation::SourceFileSubstitution => {
            std::fs::copy(
                references.join("legacy-rule-mappings/debugging.json"),
                references.join("identity-copy.json"),
            )?;
            "identity-copy.json".to_owned()
        }
    };
    mutate_manifest(plugin_root, |mapping| {
        mapping["identity_file"] = Value::String(value);
    })
}

fn mutate_manifest(plugin_root: &Path, mutate: impl FnOnce(&mut Value)) -> TestResult {
    let path = plugin_root.join("skills/engineering/references/legacy-rule-manifest.json");
    let mut manifest: Value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    let mapping = manifest["mappings"]
        .as_array_mut()
        .and_then(|mappings| mappings.first_mut())
        .ok_or("debugging mapping missing")?;
    mutate(mapping);
    std::fs::write(path, serde_json::to_string_pretty(&manifest)?)?;
    Ok(())
}

fn assert_rejected(plugin_root: &Path, label: &str) {
    let diagnostics = codexy_runtime::validation::engineering_equivalence_diagnostics(plugin_root);
    assert!(!diagnostics.is_empty(), "{label} must fail");
}
