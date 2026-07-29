use std::{fs, path::Path};

use serde_yaml::Value;

#[test]
fn final_release_admits_explicit_lineage_before_publication() -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(path)?)?;
    let step = publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .and_then(|steps| steps.iter().find(|step| step["name"] == "Create and verify the only public version release"))
        .ok_or("final release step")?;
    for (name, input) in [
        ("STAGING_SOURCE_COMMIT", "staging_source_commit"),
        ("ACTIVATION_COMMIT", "activation_commit"),
        ("STAGING_RUN_ID", "staging_run_id"),
    ] {
        assert_eq!(step["env"][name], format!("${{{{ inputs.{input} }}}}"));
    }
    let release = step["run"].as_str().ok_or("final release run")?;
    let create = release.find("gh release create v1.3.0").ok_or("version release")?;
    for required in [
        "test -n \"$STAGING_SOURCE_COMMIT\"",
        "test \"$(jq -r .source.stagingSourceCommit dist/runtime-release-receipt.json)\" = \"$STAGING_SOURCE_COMMIT\"",
        "git show-ref --verify --quiet refs/tags/v1.3.0",
        "refs/tags/v1.3.0^{commit}",
        "cannot resolve existing v1.3.0 tag",
        "existing v1.3.0 tag does not match activation commit",
    ] {
        assert!(release.find(required).ok_or(required)? < create);
    }
    Ok(())
}
