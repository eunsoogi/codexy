use std::{fs, path::Path};

use serde_json::{Value, json};


pub(super) fn select_version_advance(
    root: &Path,
    target: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let contract = root.join(".agents/plugins/release-publish-contract.json");
    let mut data: Value = serde_json::from_str(&fs::read_to_string(&contract)?)?;
    data["bootstrap"]["selectedVersion"] = json!(target);
    data["runtime"]["selectedTag"] = json!(format!("v{target}"));
    fs::write(contract, format!("{}\n", serde_json::to_string_pretty(&data)?))?;
    let candidate = super::isolation::bootstrap_candidate_version(root)?;
    fs::write(
        root.join("packages/codexy-runtime/src/version/bootstrap.rs"),
        format!(
            "pub(super) const VERSION: &str = \"{target}\";\npub(super) const CANDIDATE_VERSION: &str = \"{candidate}\";\n"
        ),
    )?;
    Ok(())
}
