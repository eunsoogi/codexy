use std::{fs, path::Path};

use serde_yaml::Value;

pub(super) fn lines(path: &Path) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(fs::read_to_string(path).unwrap_or_default().lines().count())
}

pub(super) fn release_step() -> Result<String, Box<dyn std::error::Error>> {
    let workflow = codexy_runtime::paths::repository_root()
        .join(".github/workflows/publish-version-release.yml");
    let publisher: Value = serde_yaml::from_str(&fs::read_to_string(workflow)?)?;
    let steps = publisher["jobs"]["publish-v1-3-0"]["steps"]
        .as_sequence()
        .ok_or("release steps")?;
    let source = steps
        .iter()
        .find(|step| step["name"] == "Verify selected protected-main source")
        .and_then(|step| step["run"].as_str())
        .ok_or("protected main source")?;
    let release = steps
        .iter()
        .find(|step| step["name"] == "Create and verify the only public version release")
        .and_then(|step| step["run"].as_str())
        .ok_or("final release step")?;
    Ok(format!(
        "{source}\n{}",
        release.replace("scripts/generate-release-changelog v1.3.0", "printf notes")
    ))
}
