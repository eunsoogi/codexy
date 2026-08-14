use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde_yaml::Value;

use crate::support;

pub(super) const SOURCE_BINDINGS: [(&str, &str); 1] = [(
    "scripts/verify-runtime-release-source-binding",
    "CODEXY_FIXTURE_SOURCE_BINDING",
)];

pub(super) fn source_binding_path(root: &Path) -> PathBuf {
    root.join("scripts/verify-runtime-release-source-binding")
}

pub(super) fn install_source_binding_verifier(root: &Path) -> io::Result<()> {
    let verifier = root.join("scripts/verify-runtime-release-source-binding");
    let source = codexy_runtime::paths::repository_root()
        .join("scripts/verify-runtime-release-source-binding");
    fs::create_dir_all(verifier.parent().expect("verifier parent"))?;
    fs::copy(source, &verifier)?;
    support::make_executable(&verifier)
}

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

#[test]
fn release_step_runs_the_extracted_verifier_through_sh() -> Result<(), Box<dyn std::error::Error>> {
    assert!(release_step()?.contains("sh scripts/verify-runtime-release-source-binding"));
    Ok(())
}
