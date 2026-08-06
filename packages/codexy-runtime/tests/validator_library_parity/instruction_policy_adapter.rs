use super::{copy_plugin_fixture, normalized_fixture_stderr};
use crate::support;
use std::path::Path;

#[test]
fn single_surface_instruction_policy_adapter_matches_the_manifest_fixture()
-> Result<(), Box<dyn std::error::Error>> {
    let relative = Path::new("skills/proof-driven-completion/SKILL.md");
    let canonical = codexy_runtime::paths::repository_root()
        .join("plugins/codexy")
        .join(relative);
    let original = std::fs::read_to_string(&canonical)?;
    let fixture = support::instruction_policy_fixture(relative)?;
    let (_temp, manifest_root) = copy_plugin_fixture(&[relative])?;
    let manifest_path = manifest_root.join(relative);

    assert_matching_policy_results(
        fixture.path(),
        &manifest_root,
        &manifest_path,
        true,
        "canonical policy",
    )?;

    let replacement = original.replace("MUST NOT accept", "do not accept");
    std::fs::write(fixture.path(), &replacement)?;
    std::fs::write(&manifest_path, replacement)?;
    assert_matching_policy_results(
        fixture.path(),
        &manifest_root,
        &manifest_path,
        false,
        "rejected policy",
    )?;
    assert_eq!(std::fs::read_to_string(canonical)?, original);
    Ok(())
}

fn assert_matching_policy_results(
    single_path: &Path,
    manifest_root: &Path,
    manifest_path: &Path,
    expected_success: bool,
    context: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let single_surface = support::validator_instruction_policy_file(single_path)?;
    let manifest_fixture = support::validator_instruction_policy(manifest_root)?;
    assert_eq!(single_surface.status.success(), expected_success, "{context}");
    assert_eq!(single_surface.status.code(), manifest_fixture.status.code(), "{context}");
    assert_eq!(
        normalized_fixture_stderr(&single_surface, single_path),
        normalized_fixture_stderr(&manifest_fixture, manifest_path),
        "{context}",
    );
    Ok(())
}
