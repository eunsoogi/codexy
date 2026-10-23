use std::{error::Error, path::Path};

// Candidate staging follows the source contract while the selected crate version stays older.
pub(crate) fn candidate_version(root: &Path) -> Result<String, Box<dyn Error>> {
    let contract: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join(".agents/plugins/release-publish-contract.json"),
    )?)?;
    Ok(contract["bootstrap"]["candidateVersion"]
        .as_str()
        .ok_or("candidate version")?
        .to_owned())
}
